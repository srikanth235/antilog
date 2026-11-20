//! Approval cards and policy: who may decide, and when policy decides for them.

use antilog_types::{
    Approval, ApprovalId, ApprovalStatus, AuditEntry, AuditOutcome, CardKind, CardPayload,
    Decision, ExperimentId, MemberId, OrganizationId, Policy, PolicyMode, SessionId,
};

use crate::organization::{Action, Principal, Refusal, entry, member_of, permit, project_of};
use crate::store::Store;

/// A card a tool raises. `follow_policy` is the experiment's Mode: with Ask, nothing is pre-approved.
#[derive(Debug, Clone)]
pub struct CardRequest {
    pub organization: OrganizationId,
    pub experiment: ExperimentId,
    pub session: Option<SessionId>,
    pub card: CardKind,
    pub summary: String,
    /// The card's typed fields; it must belong to `card`.
    pub payload: Option<CardPayload>,
    pub cost_cents: Option<i64>,
    pub needs_owner: bool,
    pub follow_policy: bool,
}

/// What policy says for each card kind: the ones that were set, the rest always ask.
pub fn effective(set: &[Policy]) -> Vec<Policy> {
    CardKind::ALL
        .iter()
        .map(|card| {
            set.iter()
                .find(|policy| policy.card == *card)
                .copied()
                .unwrap_or(Policy {
                    card: *card,
                    mode: PolicyMode::AlwaysAsk,
                    limit_cents: None,
                })
        })
        .collect()
}

fn allowed(request: &CardRequest, policy: &Policy) -> bool {
    if !request.follow_policy || request.needs_owner || request.card == CardKind::AddCredential {
        return false;
    }
    match policy.mode {
        PolicyMode::AlwaysAsk => false,
        PolicyMode::Allow => true,
        PolicyMode::AllowUpTo => matches!(
            (request.cost_cents, policy.limit_cents),
            (Some(cost), Some(limit)) if cost <= limit
        ),
    }
}

/// Raises a card: pending, or approved by policy, and either way audited.
/// `actor` is the member the session works for; `None` is Antilog itself.
pub async fn raise<S: Store>(
    store: &S,
    now: i64,
    actor: Option<MemberId>,
    request: CardRequest,
) -> Result<Approval, Refusal> {
    if request.summary.chars().count() > 2000 {
        return Err(Refusal::Invalid("a summary is at most 2000 characters"));
    }
    if request
        .payload
        .as_ref()
        .is_some_and(|payload| payload.card() != request.card)
    {
        return Err(Refusal::Invalid("a card's fields belong to its kind"));
    }
    if request.cost_cents.is_some_and(|cost| cost < 0) {
        return Err(Refusal::Invalid("a cost is not negative"));
    }
    let policy = effective(&store.policies(request.organization).await?)
        .into_iter()
        .find(|policy| policy.card == request.card)
        .ok_or(Refusal::NotFound)?;
    let by_policy = allowed(&request, &policy);
    let approval = Approval {
        id: ApprovalId::new(),
        organization_id: request.organization,
        experiment_id: request.experiment,
        session_id: request.session,
        card: request.card,
        summary: request.summary,
        payload: request.payload,
        cost_cents: request.cost_cents,
        needs_owner: request.needs_owner,
        status: if by_policy {
            ApprovalStatus::ApprovedByPolicy
        } else {
            ApprovalStatus::Pending
        },
        decided_by: None,
        created_at: now,
        decided_at: by_policy.then_some(now),
    };
    let audit = AuditEntry::new(
        request.organization,
        actor,
        "approval.raise",
        approval.id,
        if by_policy {
            AuditOutcome::ApprovedByPolicy
        } else {
            AuditOutcome::Done
        },
        now,
    );
    store.request_approval(&approval, &audit).await?;
    Ok(approval)
}

/// Owners and editors of the project decide; a changed value model needs a project owner.
pub async fn decide<S: Store>(
    store: &S,
    who: &Principal,
    now: i64,
    id: ApprovalId,
    decision: Decision,
) -> Result<Approval, Refusal> {
    let approval = store
        .approval(who.organization, id)
        .await?
        .ok_or(Refusal::NotFound)?;
    let project = project_of(store, who, approval.experiment_id).await?;
    let action = if approval.needs_owner {
        Action::ApprovalDecideValueModel
    } else {
        Action::ApprovalDecide
    };
    permit(store, who, action, Some(project), id, now).await?;
    let outcome = match decision {
        Decision::Approve => AuditOutcome::Approved,
        Decision::Decline => AuditOutcome::Declined,
    };
    let decided = store
        .decide_approval(
            who.organization,
            id,
            decision,
            who.member,
            now,
            &entry(who, "approval.decide", id, outcome, now),
        )
        .await?;
    // An ask-first card answers the permission the agent is waiting on.
    if decided.card == CardKind::AskFirst {
        crate::runner::answer(store, &decided, now)
            .await
            .map_err(|_| {
                Refusal::Invalid("the answer could not reach the session; decide again")
            })?;
    }
    Ok(decided)
}

pub async fn approvals<S: Store>(
    store: &S,
    who: &Principal,
    experiment: ExperimentId,
) -> Result<Vec<Approval>, Refusal> {
    member_of(store, who).await?;
    project_of(store, who, experiment).await?;
    Ok(store.approvals(who.organization, experiment).await?)
}

/// Every pending card across the organization's experiments that this person may decide: their
/// role in the card's project is read from the one place and held against the matrix, so a viewer
/// sees none and a value-model card shows only to a project owner. Reads write no audit row.
pub async fn pending<S: Store>(store: &S, who: &Principal) -> Result<Vec<Approval>, Refusal> {
    member_of(store, who).await?;
    let mut projects = std::collections::HashMap::new();
    let mut roles = std::collections::HashMap::new();
    let mut mine = Vec::new();
    for approval in store.pending_approvals(who.organization).await? {
        let project = match projects.get(&approval.experiment_id) {
            Some(project) => *project,
            None => {
                let project = project_of(store, who, approval.experiment_id).await?;
                projects.insert(approval.experiment_id, project);
                project
            }
        };
        let role = match roles.get(&project) {
            Some(role) => *role,
            None => {
                let role = store
                    .role_in_project(who.organization, project, who.member)
                    .await?;
                roles.insert(project, role);
                role
            }
        };
        let action = if approval.needs_owner {
            Action::ApprovalDecideValueModel
        } else {
            Action::ApprovalDecide
        };
        if role.is_some_and(|role| role >= action.minimum()) {
            mine.push(approval);
        }
    }
    Ok(mine)
}

pub async fn policies<S: Store>(store: &S, who: &Principal) -> Result<Vec<Policy>, Refusal> {
    member_of(store, who).await?;
    Ok(effective(&store.policies(who.organization).await?))
}

/// Only an organization owner. Add credential only ever asks.
pub async fn set_policy<S: Store>(
    store: &S,
    who: &Principal,
    now: i64,
    policy: Policy,
) -> Result<(), Refusal> {
    match (policy.mode, policy.limit_cents) {
        (PolicyMode::AllowUpTo, Some(limit)) if limit >= 0 => {}
        (PolicyMode::AllowUpTo, _) => {
            return Err(Refusal::Invalid(
                "allow up to needs a limit of zero or more",
            ));
        }
        (_, None) => {}
        (_, Some(_)) => return Err(Refusal::Invalid("only allow up to takes a limit")),
    }
    permit(
        store,
        who,
        Action::PolicySet,
        None,
        policy.card.as_str(),
        now,
    )
    .await?;
    store
        .set_policy(
            who.organization,
            &policy,
            &entry(
                who,
                "policy.set",
                policy.card.as_str(),
                AuditOutcome::Done,
                now,
            ),
        )
        .await?;
    Ok(())
}
