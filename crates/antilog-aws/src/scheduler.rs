//! The core's `Timer` on EventBridge Scheduler: one one-off schedule per job.
//!
//! Due jobs stay rows in Postgres; a schedule only wakes the function at that time.
//! Wake-ups are at least once, so the handler is idempotent by job id. Each schedule
//! deletes itself after it fires, and `cancel` deletes it early. A job id is one
//! schedule name, so booking the same job again moves its time instead of adding another.

use std::future::Future;
use std::sync::Arc;

use antilog_core::clock::{Clock, SystemClock};
use antilog_core::timer::{JobId, Timer, TimerError};
use aws_sdk_scheduler::error::SdkError;
use aws_sdk_scheduler::types::{
    ActionAfterCompletion, FlexibleTimeWindow, FlexibleTimeWindowMode, Target,
};
use jiff::{SignedDuration, Timestamp, tz::TimeZone};

use crate::lambda;

/// Scheduler refuses a time in the past, and a wake-up in under this long is no use anyway.
const SOONEST: SignedDuration = SignedDuration::from_secs(5);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchedulerConfig {
    /// The schedule group, created by OpenTofu so one place can disable them all.
    pub group: String,
    /// The function to wake.
    pub target_arn: String,
    /// The role Scheduler assumes to invoke it (the one role that may).
    pub role_arn: String,
}

impl SchedulerConfig {
    pub fn from_env() -> Result<Self, String> {
        let get = |name: &str| {
            std::env::var(name)
                .ok()
                .filter(|v| !v.is_empty())
                .ok_or_else(|| format!("{name} is not set"))
        };
        Ok(Self {
            group: get("ANTILOG_SCHEDULER_GROUP")?,
            target_arn: get("ANTILOG_SCHEDULER_TARGET_ARN")?,
            role_arn: get("ANTILOG_SCHEDULER_ROLE_ARN")?,
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ScheduleError {
    #[error("a schedule with that name exists")]
    Exists,
    #[error("no such schedule")]
    NotFound,
    #[error("scheduler unavailable")]
    Unavailable(#[source] Box<dyn std::error::Error + Send + Sync>),
}

/// One schedule as the adapter books it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Booking {
    pub name: String,
    /// `at(2026-11-18T10:00:00)`, in UTC.
    pub expression: String,
    pub input: String,
}

/// EventBridge Scheduler, reduced to the three calls the adapter makes.
pub trait Scheduling: Send + Sync {
    fn create(&self, booking: &Booking) -> impl Future<Output = Result<(), ScheduleError>> + Send;
    fn update(&self, booking: &Booking) -> impl Future<Output = Result<(), ScheduleError>> + Send;
    fn delete(&self, name: &str) -> impl Future<Output = Result<(), ScheduleError>> + Send;
}

pub struct SchedulerTimer<S> {
    scheduling: S,
    clock: Arc<dyn Clock>,
}

impl<S: Scheduling> SchedulerTimer<S> {
    pub fn new(scheduling: S) -> Self {
        Self::with_clock(scheduling, Arc::new(SystemClock))
    }

    pub fn with_clock(scheduling: S, clock: Arc<dyn Clock>) -> Self {
        Self { scheduling, clock }
    }
}

/// A schedule name is at most 64 characters of `[0-9a-zA-Z-_.]`.
pub fn schedule_name(job: JobId) -> String {
    format!("antilog-wake-{:032x}", job.0)
}

fn expression(at: Timestamp) -> String {
    let utc = at.to_zoned(TimeZone::UTC);
    format!("at({})", utc.strftime("%Y-%m-%dT%H:%M:%S"))
}

impl<S: Scheduling> Timer for SchedulerTimer<S> {
    async fn wake_at(&self, job: JobId, at: Timestamp) -> Result<(), TimerError> {
        let soonest = self.clock.now().checked_add(SOONEST).unwrap_or(at);
        let booking = Booking {
            name: schedule_name(job),
            expression: expression(at.max(soonest)),
            input: lambda::wake_event(&format!("{:032x}", job.0)),
        };
        match self.scheduling.create(&booking).await {
            Ok(()) => Ok(()),
            // The job was booked before: this is a reschedule, or a retry of the same booking.
            Err(ScheduleError::Exists) => {
                self.scheduling.update(&booking).await.map_err(unavailable)
            }
            Err(other) => Err(unavailable(other)),
        }
    }

    async fn cancel(&self, job: JobId) -> Result<(), TimerError> {
        match self.scheduling.delete(&schedule_name(job)).await {
            // It fired and deleted itself, or was never booked.
            Ok(()) | Err(ScheduleError::NotFound) => Ok(()),
            Err(other) => Err(unavailable(other)),
        }
    }
}

fn unavailable(error: ScheduleError) -> TimerError {
    TimerError::Unavailable(Box::new(error))
}

/// The real client.
pub struct SdkScheduling {
    client: aws_sdk_scheduler::Client,
    config: SchedulerConfig,
}

impl SdkScheduling {
    pub fn new(client: aws_sdk_scheduler::Client, config: SchedulerConfig) -> Self {
        Self { client, config }
    }

    pub async fn connect(config: SchedulerConfig) -> Self {
        let shared = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .load()
            .await;
        Self::new(aws_sdk_scheduler::Client::new(&shared), config)
    }

    fn target(&self, booking: &Booking) -> Result<Target, ScheduleError> {
        Target::builder()
            .arn(&self.config.target_arn)
            .role_arn(&self.config.role_arn)
            .input(&booking.input)
            .build()
            .map_err(|e| ScheduleError::Unavailable(Box::new(e)))
    }

    fn window() -> Result<FlexibleTimeWindow, ScheduleError> {
        FlexibleTimeWindow::builder()
            .mode(FlexibleTimeWindowMode::Off)
            .build()
            .map_err(|e| ScheduleError::Unavailable(Box::new(e)))
    }
}

impl Scheduling for SdkScheduling {
    async fn create(&self, booking: &Booking) -> Result<(), ScheduleError> {
        self.client
            .create_schedule()
            .name(&booking.name)
            .group_name(&self.config.group)
            .schedule_expression(&booking.expression)
            .schedule_expression_timezone("UTC")
            .flexible_time_window(Self::window()?)
            .target(self.target(booking)?)
            .action_after_completion(ActionAfterCompletion::Delete)
            .send()
            .await
            .map(|_| ())
            .map_err(|error| match error {
                SdkError::ServiceError(e) if e.err().is_conflict_exception() => {
                    ScheduleError::Exists
                }
                other => ScheduleError::Unavailable(Box::new(other)),
            })
    }

    async fn update(&self, booking: &Booking) -> Result<(), ScheduleError> {
        self.client
            .update_schedule()
            .name(&booking.name)
            .group_name(&self.config.group)
            .schedule_expression(&booking.expression)
            .schedule_expression_timezone("UTC")
            .flexible_time_window(Self::window()?)
            .target(self.target(booking)?)
            .action_after_completion(ActionAfterCompletion::Delete)
            .send()
            .await
            .map(|_| ())
            .map_err(|error| match error {
                SdkError::ServiceError(e) if e.err().is_resource_not_found_exception() => {
                    ScheduleError::NotFound
                }
                other => ScheduleError::Unavailable(Box::new(other)),
            })
    }

    async fn delete(&self, name: &str) -> Result<(), ScheduleError> {
        self.client
            .delete_schedule()
            .name(name)
            .group_name(&self.config.group)
            .send()
            .await
            .map(|_| ())
            .map_err(|error| match error {
                SdkError::ServiceError(e) if e.err().is_resource_not_found_exception() => {
                    ScheduleError::NotFound
                }
                other => ScheduleError::Unavailable(Box::new(other)),
            })
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::{Mutex, PoisonError};

    use super::*;

    struct At(Timestamp);
    impl Clock for At {
        fn now(&self) -> Timestamp {
            self.0
        }
    }

    #[derive(Default)]
    struct Stub {
        schedules: Mutex<HashMap<String, Booking>>,
        creates: Mutex<u32>,
        down: Mutex<bool>,
    }

    impl Stub {
        fn all(&self) -> HashMap<String, Booking> {
            self.schedules
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .clone()
        }
        fn check(&self) -> Result<(), ScheduleError> {
            if *self.down.lock().unwrap_or_else(PoisonError::into_inner) {
                return Err(ScheduleError::Unavailable("throttled".into()));
            }
            Ok(())
        }
    }

    impl Scheduling for Stub {
        async fn create(&self, booking: &Booking) -> Result<(), ScheduleError> {
            self.check()?;
            *self.creates.lock().unwrap_or_else(PoisonError::into_inner) += 1;
            let mut all = self
                .schedules
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            if all.contains_key(&booking.name) {
                return Err(ScheduleError::Exists);
            }
            all.insert(booking.name.clone(), booking.clone());
            Ok(())
        }
        async fn update(&self, booking: &Booking) -> Result<(), ScheduleError> {
            self.check()?;
            let mut all = self
                .schedules
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            match all.get_mut(&booking.name) {
                Some(slot) => {
                    *slot = booking.clone();
                    Ok(())
                }
                None => Err(ScheduleError::NotFound),
            }
        }
        async fn delete(&self, name: &str) -> Result<(), ScheduleError> {
            self.check()?;
            match self
                .schedules
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .remove(name)
            {
                Some(_) => Ok(()),
                None => Err(ScheduleError::NotFound),
            }
        }
    }

    fn timer() -> SchedulerTimer<Stub> {
        let now: Timestamp = "2026-11-18T10:00:00Z"
            .parse()
            .unwrap_or_else(|e| unreachable!("{e}"));
        SchedulerTimer::with_clock(Stub::default(), Arc::new(At(now)))
    }

    fn ts(text: &str) -> Timestamp {
        text.parse().unwrap_or_else(|e| unreachable!("{e}"))
    }

    #[tokio::test]
    async fn a_wake_up_is_one_schedule_at_the_time_in_utc() {
        let timer = timer();
        timer
            .wake_at(JobId(255), ts("2026-11-18T10:30:00Z"))
            .await
            .unwrap_or_else(|e| unreachable!("{e}"));
        let all = timer.scheduling.all();
        let booking = all
            .get(&schedule_name(JobId(255)))
            .unwrap_or_else(|| unreachable!("booked"));
        assert_eq!(
            booking.name,
            "antilog-wake-000000000000000000000000000000ff"
        );
        assert_eq!(booking.expression, "at(2026-11-18T10:30:00)");
        assert!(
            booking
                .input
                .contains("/internal/wake/000000000000000000000000000000ff")
        );
    }

    #[tokio::test]
    async fn booking_a_job_twice_moves_it_and_adds_nothing() {
        let timer = timer();
        timer
            .wake_at(JobId(1), ts("2026-11-18T10:30:00Z"))
            .await
            .unwrap_or_else(|e| unreachable!("{e}"));
        timer
            .wake_at(JobId(1), ts("2026-11-18T11:00:00Z"))
            .await
            .unwrap_or_else(|e| unreachable!("{e}"));
        timer
            .wake_at(JobId(1), ts("2026-11-18T11:00:00Z"))
            .await
            .unwrap_or_else(|e| unreachable!("{e}"));
        let all = timer.scheduling.all();
        assert_eq!(all.len(), 1);
        assert_eq!(
            all.values().next().map(|b| b.expression.as_str()),
            Some("at(2026-11-18T11:00:00)")
        );
    }

    #[tokio::test]
    async fn a_time_in_the_past_is_booked_for_just_after_now() {
        let timer = timer();
        timer
            .wake_at(JobId(2), ts("2026-11-18T09:00:00Z"))
            .await
            .unwrap_or_else(|e| unreachable!("{e}"));
        let all = timer.scheduling.all();
        assert_eq!(
            all.values().next().map(|b| b.expression.as_str()),
            Some("at(2026-11-18T10:00:05)")
        );
    }

    #[tokio::test]
    async fn cancel_deletes_and_cancelling_again_is_fine() {
        let timer = timer();
        timer
            .wake_at(JobId(3), ts("2026-11-18T10:30:00Z"))
            .await
            .unwrap_or_else(|e| unreachable!("{e}"));
        timer
            .cancel(JobId(3))
            .await
            .unwrap_or_else(|e| unreachable!("{e}"));
        assert!(timer.scheduling.all().is_empty());
        timer
            .cancel(JobId(3))
            .await
            .unwrap_or_else(|e| unreachable!("{e}"));
    }

    #[tokio::test]
    async fn an_outage_is_unavailable() {
        let timer = timer();
        *timer
            .scheduling
            .down
            .lock()
            .unwrap_or_else(PoisonError::into_inner) = true;
        assert!(matches!(
            timer.wake_at(JobId(4), ts("2026-11-18T10:30:00Z")).await,
            Err(TimerError::Unavailable(_))
        ));
        assert!(matches!(
            timer.cancel(JobId(4)).await,
            Err(TimerError::Unavailable(_))
        ));
    }
}
