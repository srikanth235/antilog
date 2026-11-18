use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::{ExperimentId, SealHash, id};
use crate::SessionId;

id!(TrialId);

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CommitRef {
    pub repo: String,
    pub sha: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Trial {
    pub id: TrialId,
    pub experiment: ExperimentId,
    pub version: u32,
    pub session: SessionId,
    /// `None` starts from the baseline.
    pub parent: Option<TrialId>,
    pub commits: Vec<CommitRef>,
    pub hypothesis: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum Outcome {
    Measuring,
    Kept,
    WithinNoise,
    Worse,
    GuardrailFailed,
    FailedHoldout,
    Rejected,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Decision {
    pub outcome: Outcome,
    pub reasons: Vec<String>,
    /// Version of the acceptance criteria that decided it.
    pub criteria: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum Stage {
    QuickCheck,
    Full,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Stats {
    pub n: u32,
    pub mean: f64,
    pub sd: f64,
    pub confidence: f64,
    pub lower: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GuardrailReading {
    pub name: String,
    /// Paired worsening, positive when the guardrail got worse.
    pub worsening: Stats,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum HoldoutResult {
    Passed,
    Failed,
}

/// The judge's result for one trial. It holds no holdout value, only `HoldoutResult`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Measurement {
    pub trial: TrialId,
    pub experiment: ExperimentId,
    pub version: u32,
    pub seal: SealHash,
    pub stage: Stage,
    /// Which measured trial this is in its experiment version.
    pub k: u32,
    pub noise: f64,
    pub parent_value: f64,
    pub value: f64,
    pub gain: Option<Stats>,
    pub confirmation: Option<Stats>,
    pub guardrails: Vec<GuardrailReading>,
    pub holdout: Option<HoldoutResult>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Finished {
    /// `None` for a trial that never ran: Rejected or Failed. Never shown as zero.
    pub measurement: Option<Measurement>,
    pub decision: Decision,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TrialRecord {
    pub trial: Trial,
    pub finished: Option<Finished>,
}

impl TrialRecord {
    pub fn outcome(&self) -> Outcome {
        self.finished
            .as_ref()
            .map_or(Outcome::Measuring, |f| f.decision.outcome)
    }
}
