use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::id::text_enum;
use crate::{ApprovalId, AuditId, ExperimentId, MemberId, OrganizationId, SessionId, SpendId};

text_enum!(SpendKind { Tokens => "tokens", Sandbox => "sandbox", Judge => "judge" });

/// One append-only row of spend, in US cents.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Spend {
    pub id: SpendId,
    pub organization_id: OrganizationId,
    pub session_id: SessionId,
    pub kind: SpendKind,
    pub amount_cents: i64,
    pub tokens: i64,
    pub occurred_at: i64,
}

text_enum!(
    CardKind {
        SealExperiment => "seal_experiment",
        RaiseBudget => "raise_budget",
        AddCredential => "add_credential",
        AttachRepo => "attach_repo",
        ChangeEnvironment => "change_environment",
        OpenPullRequest => "open_pull_request",
        KeepGoing => "keep_going",
        ContinueOnAnotherAgent => "continue_on_another_agent",
    }
);

text_enum!(
    PolicyMode { AlwaysAsk => "always_ask", AllowUpTo => "allow_up_to", Allow => "allow" }
);

/// What policy does with one card kind. Add credential only ever asks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Policy {
    pub card: CardKind,
    pub mode: PolicyMode,
    /// Required for `allow_up_to`, in US cents.
    pub limit_cents: Option<i64>,
}

text_enum!(
    ApprovalStatus {
        Pending => "pending",
        Approved => "approved",
        Declined => "declined",
        ApprovedByPolicy => "approved_by_policy",
    }
);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Approval {
    pub id: ApprovalId,
    pub organization_id: OrganizationId,
    pub experiment_id: ExperimentId,
    pub session_id: Option<SessionId>,
    pub card: CardKind,
    /// What would happen; never a secret.
    pub summary: String,
    pub cost_cents: Option<i64>,
    pub status: ApprovalStatus,
    pub decided_by: Option<MemberId>,
    pub created_at: i64,
    pub decided_at: Option<i64>,
}

text_enum!(
    AuditOutcome {
        Done => "done",
        Denied => "denied",
        Approved => "approved",
        Declined => "declined",
        ApprovedByPolicy => "approved_by_policy",
    }
);

/// An append-only record of a write or an approval. Holds no secret.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AuditEntry {
    pub id: AuditId,
    pub organization_id: OrganizationId,
    /// `None`: Antilog itself.
    pub actor: Option<MemberId>,
    /// A registered action name, such as `project.create`.
    pub action: String,
    /// The id of what it touched.
    pub target: String,
    pub outcome: AuditOutcome,
    pub at: i64,
}

impl AuditEntry {
    pub fn new(
        organization_id: OrganizationId,
        actor: Option<MemberId>,
        action: impl Into<String>,
        target: impl ToString,
        outcome: AuditOutcome,
        at: i64,
    ) -> Self {
        Self {
            id: AuditId::new(),
            organization_id,
            actor,
            action: action.into(),
            target: target.to_string(),
            outcome,
            at,
        }
    }
}

text_enum!(Decision { Approve => "approve", Decline => "decline" });
