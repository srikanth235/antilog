//! Budgets and caps: what is left to spend, and the refusal when nothing is.

use antilog_types::{AuditOutcome, ExperimentId, OrganizationId, ProjectId};
use jiff::Timestamp;

use crate::organization::{Action, Principal, Refusal, entry, member_of, permit, project_of};
use crate::store::{SpendScope, Store};

/// Epoch milliseconds of the first instant of the UTC month `now` falls in, and of the next.
pub fn month_bounds(now: i64) -> Option<(i64, i64)> {
    let zoned = Timestamp::from_millisecond(now)
        .ok()?
        .to_zoned(jiff::tz::TimeZone::UTC);
    let first = zoned.date().first_of_month();
    let start = first.to_zoned(jiff::tz::TimeZone::UTC).ok()?;
    let end = first
        .checked_add(jiff::Span::new().months(1))
        .ok()?
        .to_zoned(jiff::tz::TimeZone::UTC)
        .ok()?;
    Some((
        start.timestamp().as_millisecond(),
        end.timestamp().as_millisecond(),
    ))
}

/// What is left under each limit, in US cents; `None` is no limit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Headroom {
    pub experiment: i64,
    pub project: Option<i64>,
    pub organization: Option<i64>,
}

impl Headroom {
    pub fn remaining(&self) -> i64 {
        [Some(self.experiment), self.project, self.organization]
            .into_iter()
            .flatten()
            .min()
            .unwrap_or(self.experiment)
    }

    pub fn spent(&self) -> bool {
        self.remaining() <= 0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum Spent {
    #[error("the experiment's budget is spent")]
    Experiment,
    #[error("the project's monthly cap is reached")]
    Project,
    #[error("the organization's monthly cap is reached")]
    Organization,
}

fn left(cap: Option<i64>, spent: i64) -> Option<i64> {
    cap.map(|cap| cap.saturating_sub(spent))
}

pub async fn headroom<S: Store>(
    store: &S,
    organization: OrganizationId,
    experiment: ExperimentId,
    now: i64,
) -> Result<Headroom, Refusal> {
    let (from, to) = month_bounds(now).ok_or(Refusal::Invalid("the time is out of range"))?;
    let row = store
        .experiment(organization, experiment)
        .await?
        .ok_or(Refusal::NotFound)?;
    let project = store
        .project(organization, row.project_id)
        .await?
        .ok_or(Refusal::NotFound)?;
    let owner = store
        .organization(organization)
        .await?
        .ok_or(Refusal::NotFound)?;
    let experiment_spent = store
        .spend(
            organization,
            SpendScope::Experiment(experiment),
            0,
            now.saturating_add(1),
        )
        .await?
        .cents;
    let project_spent = store
        .spend(organization, SpendScope::Project(project.id), from, to)
        .await?
        .cents;
    let organization_spent = store
        .spend(organization, SpendScope::Organization, from, to)
        .await?
        .cents;
    Ok(Headroom {
        experiment: row.budget_cap_cents.saturating_sub(experiment_spent),
        project: left(project.monthly_cap_cents, project_spent),
        organization: left(owner.monthly_cap_cents, organization_spent),
    })
}

/// What the judge asks before `measure` and a platform asks before a session step:
/// refused when nothing is left under any limit.
pub async fn ensure_room<S: Store>(
    store: &S,
    organization: OrganizationId,
    experiment: ExperimentId,
    now: i64,
) -> Result<Headroom, Refusal> {
    let room = headroom(store, organization, experiment, now).await?;
    let blocked = if room.experiment <= 0 {
        Some(Spent::Experiment)
    } else if room.project.is_some_and(|left| left <= 0) {
        Some(Spent::Project)
    } else if room.organization.is_some_and(|left| left <= 0) {
        Some(Spent::Organization)
    } else {
        None
    };
    match blocked {
        Some(spent) => Err(Refusal::Spent(spent)),
        None => Ok(room),
    }
}

fn cents(value: Option<i64>) -> Result<(), Refusal> {
    if value.is_some_and(|value| value < 0) {
        Err(Refusal::Invalid("an amount is not negative"))
    } else {
        Ok(())
    }
}

/// Only an organization owner changes the cap and the default budget.
pub async fn set_organization_limits<S: Store>(
    store: &S,
    who: &Principal,
    now: i64,
    monthly_cap_cents: Option<i64>,
    default_budget_cents: Option<i64>,
) -> Result<(), Refusal> {
    cents(monthly_cap_cents)?;
    cents(default_budget_cents)?;
    permit(
        store,
        who,
        Action::OrganizationLimitsSet,
        None,
        who.organization,
        now,
    )
    .await?;
    store
        .set_organization_limits(
            who.organization,
            monthly_cap_cents,
            default_budget_cents,
            &entry(
                who,
                "organization.set_limits",
                who.organization,
                AuditOutcome::Done,
                now,
            ),
        )
        .await?;
    Ok(())
}

/// A project owner changes the project's cap and default budget, inside the organization's cap.
pub async fn set_project_limits<S: Store>(
    store: &S,
    who: &Principal,
    now: i64,
    project: ProjectId,
    monthly_cap_cents: Option<i64>,
    default_budget_cents: Option<i64>,
) -> Result<(), Refusal> {
    cents(monthly_cap_cents)?;
    cents(default_budget_cents)?;
    permit(
        store,
        who,
        Action::ProjectLimitsSet,
        Some(project),
        project,
        now,
    )
    .await?;
    store
        .set_project_limits(
            who.organization,
            project,
            monthly_cap_cents,
            default_budget_cents,
            &entry(who, "project.set_limits", project, AuditOutcome::Done, now),
        )
        .await?;
    Ok(())
}

/// An editor or owner of the experiment's project changes its budget, inside the project's cap.
pub async fn set_experiment_budget<S: Store>(
    store: &S,
    who: &Principal,
    now: i64,
    experiment: ExperimentId,
    budget_cap_cents: i64,
) -> Result<(), Refusal> {
    cents(Some(budget_cap_cents))?;
    let project = project_of(store, who, experiment).await?;
    permit(
        store,
        who,
        Action::ExperimentBudgetSet,
        Some(project),
        experiment,
        now,
    )
    .await?;
    store
        .set_experiment_budget(
            who.organization,
            experiment,
            budget_cap_cents,
            &entry(
                who,
                "experiment.set_budget",
                experiment,
                AuditOutcome::Done,
                now,
            ),
        )
        .await?;
    Ok(())
}

/// Spend this month against a limit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Scope {
    pub cap_cents: Option<i64>,
    pub spent_cents: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Summary {
    pub organization: Scope,
    pub projects: Vec<(ProjectId, Scope)>,
}

/// The month's spend against the organization's cap and each project's.
pub async fn summary<S: Store>(store: &S, who: &Principal, now: i64) -> Result<Summary, Refusal> {
    member_of(store, who).await?;
    let (from, to) = month_bounds(now).ok_or(Refusal::Invalid("the time is out of range"))?;
    let owner = store
        .organization(who.organization)
        .await?
        .ok_or(Refusal::NotFound)?;
    let organization = Scope {
        cap_cents: owner.monthly_cap_cents,
        spent_cents: store
            .spend(who.organization, SpendScope::Organization, from, to)
            .await?
            .cents,
    };
    let mut projects = Vec::new();
    for project in store.projects(who.organization).await? {
        let spent = store
            .spend(who.organization, SpendScope::Project(project.id), from, to)
            .await?;
        projects.push((
            project.id,
            Scope {
                cap_cents: project.monthly_cap_cents,
                spent_cents: spent.cents,
            },
        ));
    }
    Ok(Summary {
        organization,
        projects,
    })
}
