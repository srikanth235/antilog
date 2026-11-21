use antilog_types::{
    Approval, ApprovalStatus, CardKind, CardPayload, Command, EventKind, OrganizationId, RepoSpec,
    SessionId, SessionStatus,
};

use super::{SessionError, audit, now_ms, record, tell};
use crate::deps::Deps;
use crate::store::{
    ApprovalStore, BudgetStore, CardStore, RunnerStore, SessionChange, SessionStore, StoreError,
};

/// Applies what a session's decided cards allowed, once each: raises a budget, attaches a repo.
/// Returns what each one came to, in words, for the agent.
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
    let approvals = deps.store().approvals(organization, session).await?;
    let mut told = Vec::new();
    for approval in approvals {
        let decided = approval.status != ApprovalStatus::Pending;
        if approval.card == CardKind::AskFirst
            || !decided
            || !deps.store().mark_applied(organization, approval.id).await?
        {
            continue;
        }
        told.push((approval.id, outcome_text(deps, &approval).await));
    }
    // Each outcome is the conversation's note from Antilog and a message to the agent, which wakes
    // it: a released machine is requeued first. Applying is guarded above, so it happens once.
    let mut status = SessionStatus::Working;
    for (id, text) in &told {
        record(
            deps,
            &row,
            format!("card:{id}"),
            EventKind::Agent,
            None,
            text,
        )
        .await?;
        status = super::wake(deps, session, now_ms(deps)).await?;
        tell(
            deps,
            organization,
            session,
            &Command::Message { text: text.clone() },
        )
        .await?;
    }
    if !told.is_empty() && row.status == SessionStatus::WaitingForYou {
        let change = SessionChange {
            status: Some(status),
            ..SessionChange::default()
        };
        deps.store()
            .update_session(organization, session, &change, now_ms(deps), None)
            .await?;
    }
    Ok(told.into_iter().map(|(_, text)| text).collect())
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
        CardKind::AttachRepo => attach_repo(deps, approval).await,
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
        approval.session_id,
    );
    let result = async {
        let row = deps
            .store()
            .session(org, approval.session_id)
            .await?
            .ok_or(StoreError::NotFound)?;
        let cap = row
            .budget_cap_cents
            .unwrap_or(0)
            .saturating_add(approval.cost_cents.unwrap_or(0));
        deps.store()
            .set_session_budget(org, approval.session_id, cap, &entry)
            .await
    }
    .await;
    match result {
        Ok(()) => "The budget is raised.".to_owned(),
        Err(StoreError::Invariant(_)) => {
            "The raise passes the organization's monthly cap, so the budget is unchanged. Ask for less with request_budget.".to_owned()
        }
        Err(_) => "The budget could not be changed now; call request_budget again.".to_owned(),
    }
}

/// The repo joins the machine's seed, so a later claim or resume clones it, and a running machine
/// is told to clone it now.
async fn attach_repo<D: Deps>(deps: &D, approval: &Approval) -> String {
    let Some(CardPayload::AttachRepo { repo, .. }) = &approval.payload else {
        return "Done.".to_owned();
    };
    let session = approval.session_id;
    let org = approval.organization_id;
    let name = repo.rsplit('/').next().unwrap_or(repo);
    let store = deps.store();
    match store.add_seed_repo(session, repo).await {
        Ok(_) => {}
        Err(StoreError::NotFound) => return "Done.".to_owned(),
        Err(_) => return "The repo could not be attached now; call attach_repo again.".to_owned(),
    }
    let running = store
        .machine(session)
        .await
        .ok()
        .flatten()
        .is_some_and(|machine| machine.state.is_held());
    if !running {
        return format!(
            "{repo} will be in the working folder as {name} when the machine next starts."
        );
    }
    let command = Command::AttachRepo {
        repo: RepoSpec {
            name: repo.clone(),
            url: String::new(),
            token: None,
        },
    };
    match store
        .push_command(org, session, &command, now_ms(deps))
        .await
    {
        Ok(_) => format!("{repo} is being cloned into the machine's working folder as {name}."),
        Err(_) => {
            format!("{repo} will be in the working folder as {name} when the machine next starts.")
        }
    }
}
