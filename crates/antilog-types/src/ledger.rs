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
    /// The trial this paid for, when the meter knows it. Spend that names none is attributed to
    /// the session's next trial by time, or counts as not measured.
    #[serde(default)]
    pub trial_id: Option<crate::judge::TrialId>,
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

text_enum!(RepoAccess { Read => "read", Push => "push" });

/// What each kind of card asks, as typed fields. A card shows these, never text parsed out of
/// its summary, and an action that follows an approval reads what to do from here.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum CardPayload {
    #[serde(rename_all = "camelCase")]
    SealExperiment {
        version: u32,
        metric: String,
        unit: String,
        baseline: f64,
        noise: f64,
        cheats_rejected: u32,
        /// "No value set", or what one unit is worth, measured offline.
        value_note: String,
    },
    #[serde(rename_all = "camelCase")]
    RaiseBudget {
        /// The experiment's budget when the card was raised.
        now_cents: i64,
        /// How much more is asked.
        requested_cents: i64,
        why: String,
    },
    #[serde(rename_all = "camelCase")]
    AddCredential {
        connector: String,
        scope: String,
        why: String,
    },
    #[serde(rename_all = "camelCase")]
    AttachRepo {
        repo: String,
        access: RepoAccess,
        /// `None`: the agent pushes to a branch of its own.
        branch: Option<String>,
    },
    #[serde(rename_all = "camelCase")]
    ChangeEnvironment {
        image: Option<String>,
        setup_script_lines: Option<u32>,
        size: Option<String>,
        network: Option<String>,
    },
    #[serde(rename_all = "camelCase")]
    OpenPullRequest {
        trial: crate::judge::TrialId,
        hypothesis: String,
        /// One pull request per repo changed.
        repos: Vec<String>,
    },
    #[serde(rename_all = "camelCase")]
    KeepGoing {
        tried: u32,
    },
    ContinueOnAnotherAgent {},
}

impl CardPayload {
    /// The card kind this payload belongs to.
    pub fn card(&self) -> CardKind {
        match self {
            Self::SealExperiment { .. } => CardKind::SealExperiment,
            Self::RaiseBudget { .. } => CardKind::RaiseBudget,
            Self::AddCredential { .. } => CardKind::AddCredential,
            Self::AttachRepo { .. } => CardKind::AttachRepo,
            Self::ChangeEnvironment { .. } => CardKind::ChangeEnvironment,
            Self::OpenPullRequest { .. } => CardKind::OpenPullRequest,
            Self::KeepGoing { .. } => CardKind::KeepGoing,
            Self::ContinueOnAnotherAgent {} => CardKind::ContinueOnAnotherAgent,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Approval {
    pub id: ApprovalId,
    pub organization_id: OrganizationId,
    pub experiment_id: ExperimentId,
    pub session_id: Option<SessionId>,
    pub card: CardKind,
    /// What would happen; never a secret.
    pub summary: String,
    /// The card's own fields. `None` only for a card raised before payloads existed.
    pub payload: Option<CardPayload>,
    pub cost_cents: Option<i64>,
    /// Only a project owner may decide it, and policy never does: a changed value model.
    pub needs_owner: bool,
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
