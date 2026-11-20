use antilog_types::{Approval, ApprovalStatus, CardKind, OrganizationId, SessionId, SessionStatus};

use super::{SessionError, audit, now_ms};
use crate::delivery::open_approved;
use crate::deps::Deps;
use crate::store::{
    ApprovalStore, BudgetStore, CardStore, SessionChange, SessionStore, StoreError,
};
use crate::tools::seal_approved;

/// Applies what a session's decided cards allowed, once each: seals what was approved, raises a
/// budget, opens pull requests. Returns what each one came to, in words, for the agent.
/// Safe to run again: a card already applied is skipped.
pub async fn settle<D: Deps>(
    deps: &D,
    organization: OrganizationId,
    session: SessionId,
) -> Result<Vec<String>, SessionError> {
    let row = deps
        .store()
        .session(organization, session)
        .await?
        .ok_or(SessionError::NotFound)?;
    let approvals = deps
        .store()
        .approvals(organization, row.experiment_id)
        .await?;
    let mut outcomes = Vec::new();
    for approval in approvals {
        let decided = approval.status != ApprovalStatus::Pending;
        if approval.session_id != Some(session)
            || approval.card == CardKind::AskFirst
            || !decided
            || !deps.store().mark_applied(organization, approval.id).await?
        {
            continue;
        }
        outcomes.push(outcome_text(deps, &approval).await);
    }
    if !outcomes.is_empty() && row.status == SessionStatus::WaitingForYou {
        let change = SessionChange {
            status: Some(SessionStatus::Working),
            ..SessionChange::default()
        };
        deps.store()
            .update_session(organization, session, &change, now_ms(deps), None)
            .await?;
    }
    Ok(outcomes)
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
        CardKind::OpenPullRequest => match open_approved(deps, approval).await {
            Ok(text) => text,
            Err(error) => format!("Approved, but opening the pull request failed: {error}"),
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
