use antilog_types::{
    Approval, ApprovalStatus, CardKind, OrganizationId, Session, SessionId, SessionStatus,
};

use super::{STEP, SessionError, audit, book, job, now_ms};
use crate::deps::Deps;
use crate::platform::{Platform, PlatformError, PlatformEvent, SessionHandle};
use crate::spend::record_usage;
use crate::store::{
    ApprovalStore, BudgetStore, RelayStore, SessionChange, SessionStore, StoreError,
};
use crate::timer::Timer;
use crate::tools::{Caller, ToolHost, seal_approved};

const PAGES: usize = 100;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Step {
    pub status: SessionStatus,
    pub events: usize,
}

#[derive(Default)]
pub(super) struct Pulled {
    pub events: usize,
    pub idle: bool,
    pub down: bool,
    pub lost: bool,
    pub ended: Option<Option<String>>,
}

async fn pull<D: Deps, H: ToolHost>(
    deps: &D,
    host: &H,
    session: &Session,
    handle: &SessionHandle,
) -> Result<Pulled, SessionError> {
    let (org, id) = (session.organization_id, session.id);
    let mut pulled = Pulled::default();
    for _ in 0..PAGES {
        let cursor = deps.store().relay_cursor(org, id).await?;
        let page = match deps
            .platform()
            .session_events(handle, cursor.as_deref(), STEP)
            .await
        {
            Ok(page) => page,
            Err(PlatformError::Unavailable(_)) => {
                pulled.down = true;
                break;
            }
            Err(PlatformError::UnknownSession) => {
                pulled.lost = true;
                break;
            }
            Err(other) => return Err(other.into()),
        };
        if page.events.is_empty() {
            break;
        }
        for (i, event) in page.events.into_iter().enumerate() {
            pulled.events += 1;
            let seed = format!("{}:{i}", cursor.as_deref().unwrap_or(""));
            match event {
                PlatformEvent::Usage { cents, tokens } => {
                    record_usage(deps.store(), org, id, cents, tokens, now_ms(deps), &seed).await?;
                }
                PlatformEvent::ToolCall {
                    id: call,
                    name,
                    input,
                } => {
                    let caller = Caller {
                        organization: org,
                        experiment: session.experiment_id,
                        session: id,
                        actor: None,
                        call: call.clone(),
                    };
                    let reply = host.call(&caller, &name, input).await;
                    deps.platform()
                        .answer_tool(handle, &call, &reply.answer)
                        .await?;
                    if let Some(ticket) = reply.ticket {
                        let text = host.finish(&caller, &ticket).await;
                        deps.platform().send_input(handle, &text).await?;
                    }
                    pulled.idle = false;
                }
                PlatformEvent::Message { .. } => pulled.idle = false,
                PlatformEvent::Idle => pulled.idle = true,
                PlatformEvent::Ended { failed } => pulled.ended = Some(failed),
            }
        }
        if let Some(next) = page.next_cursor {
            deps.store()
                .save_relay_cursor(org, id, &next, now_ms(deps))
                .await?;
        }
    }
    Ok(pulled)
}

pub(super) async fn drain<D: Deps>(
    deps: &D,
    organization: OrganizationId,
    id: SessionId,
    handle: &SessionHandle,
) -> Result<(), SessionError> {
    let session = deps
        .store()
        .session(organization, id)
        .await?
        .ok_or(SessionError::NotFound)?;
    pull(deps, &crate::tools::NoTools, &session, handle).await?;
    Ok(())
}

/// Ends the session for good and deletes it on its platform; a delete that fails is retried.
pub(super) async fn finish<D: Deps>(
    deps: &D,
    organization: OrganizationId,
    id: SessionId,
    handle: &SessionHandle,
    status: SessionStatus,
    note: Option<String>,
    entry: antilog_types::AuditEntry,
) -> Result<Session, SessionError> {
    let removed = deps.platform().delete(handle).await.is_ok();
    let change = SessionChange {
        status: Some(status),
        note,
        cleanup: Some(if removed {
            antilog_types::Cleanup::Removed
        } else {
            antilog_types::Cleanup::NotRemoved
        }),
        ..SessionChange::default()
    };
    let session = deps
        .store()
        .update_session(organization, id, &change, now_ms(deps), Some(&entry))
        .await?;
    if removed {
        deps.timer().cancel(job(id)).await?;
    } else {
        book(deps, id, STEP).await?;
    }
    Ok(session)
}

async fn set_status<D: Deps>(
    deps: &D,
    session: &Session,
    status: SessionStatus,
    note: Option<&str>,
) -> Result<(), StoreError> {
    let change = SessionChange {
        status: Some(status),
        note: note.map(str::to_owned),
        ..SessionChange::default()
    };
    deps.store()
        .update_session(
            session.organization_id,
            session.id,
            &change,
            now_ms(deps),
            None,
        )
        .await
        .map(|_| ())
}

/// Tells the session how its approvals ended, once each, and applies what they allowed.
async fn tell<D: Deps>(
    deps: &D,
    session: &Session,
    handle: &SessionHandle,
) -> Result<bool, SessionError> {
    let (org, id) = (session.organization_id, session.id);
    let approvals = deps.store().approvals(org, session.experiment_id).await?;
    let mut told = false;
    for approval in approvals {
        let decided = approval.status != ApprovalStatus::Pending;
        if approval.session_id != Some(id)
            || !decided
            || !deps.store().mark_told(org, approval.id).await?
        {
            continue;
        }
        let text = outcome_text(deps, &approval).await;
        deps.platform().send_input(handle, &text).await?;
        told = true;
    }
    Ok(told)
}

async fn outcome_text<D: Deps>(deps: &D, approval: &Approval) -> String {
    let what = &approval.summary;
    if approval.status == ApprovalStatus::Declined {
        return format!(
            "Declined by a person: {what}. Ask what they would accept, or carry on without it."
        );
    }
    let applied = match approval.card {
        CardKind::RaiseBudget => raise_budget(deps, approval).await,
        CardKind::SealExperiment => match seal_approved(deps, approval).await {
            Ok(version) => format!("Sealed as version {version}. Measure trials with measure."),
            Err(error) => format!("Approved, but sealing failed: {error}"),
        },
        _ => "Done.".to_owned(),
    };
    format!("Approved: {what}. {applied}")
}

async fn raise_budget<D: Deps>(deps: &D, approval: &Approval) -> String {
    let org = approval.organization_id;
    let entry = audit(
        deps,
        org,
        approval.decided_by,
        "budget.raise",
        approval.experiment_id,
    );
    let result = async {
        let row = deps
            .store()
            .experiment(org, approval.experiment_id)
            .await?
            .ok_or(StoreError::NotFound)?;
        let cap = row
            .budget_cap_cents
            .saturating_add(approval.cost_cents.unwrap_or(0));
        deps.store()
            .set_experiment_budget(org, approval.experiment_id, cap, &entry)
            .await
    }
    .await;
    match result {
        Ok(()) => "The budget is raised.".to_owned(),
        Err(StoreError::Invariant(_)) => {
            "The raise passes the project's or organization's cap, so the budget is unchanged. Ask for less with request_budget.".to_owned()
        }
        Err(_) => "The budget could not be changed now; call request_budget again.".to_owned(),
    }
}

/// One relay step: pull what the agent did since the saved cursor, answer its tool calls,
/// meter its spend, tell it how its approvals ended, then book the next wake-up.
/// Wake-ups come at least once, so every part of a step is safe to run again.
pub async fn step<D: Deps, H: ToolHost>(
    deps: &D,
    host: &H,
    organization: OrganizationId,
    id: SessionId,
) -> Result<Step, SessionError> {
    let session = deps
        .store()
        .session(organization, id)
        .await?
        .ok_or(SessionError::NotFound)?;
    let Some(handle) = session.platform_handle.clone().map(SessionHandle) else {
        return Ok(Step {
            status: session.status,
            events: 0,
        });
    };
    if !session.status.is_active() {
        if session.cleanup == Some(antilog_types::Cleanup::Removed) {
            deps.timer().cancel(job(id)).await?;
        } else if deps.platform().delete(&handle).await.is_ok() {
            let change = SessionChange {
                cleanup: Some(antilog_types::Cleanup::Removed),
                ..SessionChange::default()
            };
            deps.store()
                .update_session(organization, id, &change, now_ms(deps), None)
                .await?;
            deps.timer().cancel(job(id)).await?;
        } else {
            book(deps, id, STEP).await?;
        }
        return Ok(Step {
            status: session.status,
            events: 0,
        });
    }
    let pulled = pull(deps, host, &session, &handle).await?;
    let told = tell(deps, &session, &handle).await?;
    let mut status = session.status;
    if let Some(failed) = pulled.ended {
        let (to, note) = match failed {
            None => (SessionStatus::Done, None),
            Some(why) => (SessionStatus::Failed, Some(why)),
        };
        let entry = audit(deps, organization, None, "session.end", id);
        let ended = finish(deps, organization, id, &handle, to, note, entry).await?;
        return Ok(Step {
            status: ended.status,
            events: pulled.events,
        });
    }
    if pulled.lost {
        let entry = audit(deps, organization, None, "session.end", id);
        let note = Some("The platform no longer has this session".to_owned());
        let ended = finish(
            deps,
            organization,
            id,
            &handle,
            SessionStatus::Failed,
            note,
            entry,
        )
        .await?;
        return Ok(Step {
            status: ended.status,
            events: pulled.events,
        });
    }
    if pulled.down && status == SessionStatus::Working {
        set_status(
            deps,
            &session,
            SessionStatus::WaitingForYou,
            Some("The platform is unavailable. Continue on another agent?"),
        )
        .await?;
        status = SessionStatus::WaitingForYou;
    } else if told && status == SessionStatus::WaitingForYou {
        set_status(deps, &session, SessionStatus::Working, None).await?;
        status = SessionStatus::Working;
    } else if pulled.idle && status == SessionStatus::Working {
        set_status(deps, &session, SessionStatus::WaitingForYou, None).await?;
        status = SessionStatus::WaitingForYou;
    }
    book(deps, id, STEP).await?;
    Ok(Step {
        status,
        events: pulled.events,
    })
}
