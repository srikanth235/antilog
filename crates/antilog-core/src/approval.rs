//! Approval cards and policy: who may decide, and when policy decides for them.

use antilog_types::{
    Approval, ApprovalId, ApprovalStatus, AuditEntry, AuditOutcome, CardKind, CardPayload,
    Decision, MemberId, OrganizationId, Policy, PolicyMode, SessionId,
};

use crate::organization::{Action, Principal, Refusal, entry, member_of, permit};
use crate::store::Store;

/// A card a tool raises. `follow_policy` is the session's Mode: with Ask, nothing is pre-approved.
#[derive(Debug, Clone)]
pub struct CardRequest {
    pub organization: OrganizationId,
    pub session: SessionId,
    pub card: CardKind,
    pub summary: String,
    /// The card's typed fields; it must belong to `card`.
    pub payload: Option<CardPayload>,
    pub cost_cents: Option<i64>,
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
    if !request.follow_policy || request.card == CardKind::AddCredential {
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
        session_id: request.session,
        card: request.card,
        summary: request.summary,
        payload: request.payload,
        cost_cents: request.cost_cents,
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

/// Owners and editors of the organization decide.
pub async fn decide<S: Store>(
    store: &S,
    who: &Principal,
    now: i64,
    id: ApprovalId,
    decision: Decision,
) -> Result<Approval, Refusal> {
    store
        .approval(who.organization, id)
        .await?
        .ok_or(Refusal::NotFound)?;
    permit(store, who, Action::ApprovalDecide, id, now).await?;
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

/// The cards of one session. Every member reads.
pub async fn approvals<S: Store>(
    store: &S,
    who: &Principal,
    session: SessionId,
) -> Result<Vec<Approval>, Refusal> {
    member_of(store, who).await?;
    store
        .session(who.organization, session)
        .await?
        .ok_or(Refusal::NotFound)?;
    Ok(store.approvals(who.organization, session).await?)
}

/// Every pending card in the organization, for a member who may decide: an editor or an owner.
/// A viewer sees none. Reads write no audit row.
pub async fn pending<S: Store>(store: &S, who: &Principal) -> Result<Vec<Approval>, Refusal> {
    let member = member_of(store, who).await?;
    if member.role < Action::ApprovalDecide.minimum() {
        return Ok(Vec::new());
    }
    Ok(store.pending_approvals(who.organization).await?)
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
    permit(store, who, Action::PolicySet, policy.card.as_str(), now).await?;
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
