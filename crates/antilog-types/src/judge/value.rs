use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use super::{ExperimentId, TrialId, id};
use crate::id::text_enum;

id!(ValueModelId);

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, ToSchema,
)]
#[serde(transparent)]
pub struct Cents(pub i64);

impl Cents {
    pub const ZERO: Self = Self(0);

    pub fn checked_add(self, other: Self) -> Option<Self> {
        self.0.checked_add(other.0).map(Self)
    }

    pub fn checked_sub(self, other: Self) -> Option<Self> {
        self.0.checked_sub(other.0).map(Self)
    }

    pub fn saturating_sub(self, other: Self) -> Self {
        Self(self.0.saturating_sub(other.0))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ValueSource {
    pub text: String,
    pub link: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ValueModel {
    pub id: ValueModelId,
    pub version: u32,
    pub metric: String,
    /// What one unit of the metric is called, such as `ms`.
    pub unit: String,
    /// Cents a year that one unit of the metric is worth.
    pub cents_per_unit: f64,
    pub source: ValueSource,
    /// The project owner who set it; never an agent.
    pub set_by: Uuid,
}

impl ValueModel {
    /// Yearly value of moving the metric by `delta` units in the good direction.
    pub fn yearly(&self, delta: f64) -> Option<Cents> {
        let cents = (delta * self.cents_per_unit).round();
        if !(delta >= 0.0 && self.cents_per_unit >= 0.0 && cents.abs() < 9.0e18) {
            return None;
        }
        #[expect(
            clippy::cast_possible_truncation,
            reason = "finite and bounded below i64::MAX by the check above"
        )]
        Some(Cents(cents as i64))
    }
}

/// What a project owner sets: the model without its id, version or who set it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NewValueModel {
    pub metric: String,
    pub unit: String,
    /// Cents a year that one unit of the metric is worth; more than zero.
    pub cents_per_unit: f64,
    pub source: ValueSource,
}

/// A value model as a project holds it: one per metric, a new version on every change.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProjectValueModel {
    pub project_id: crate::ProjectId,
    pub model: ValueModel,
    pub set_at: i64,
}

/// The newest version of each metric's model in a project.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ValueModels {
    pub models: Vec<ProjectValueModel>,
}

text_enum!(
    /// What the code host or the watch told Antilog about a win's pull request.
    WinEventKind {
        Opened => "opened",
        Merged => "merged",
        Closed => "closed",
        Regressed => "regressed",
        Recovered => "recovered",
    }
);

/// One append-only fact about a win. Whether a win is Delivered, Pending or At risk is read
/// from the newest of these, never stored.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WinEvent {
    pub experiment: ExperimentId,
    pub version: u32,
    pub trial: TrialId,
    pub kind: WinEventKind,
    /// The metric value the watch read; only a regression carries one.
    pub value: Option<f64>,
    /// The pull request, when the code host gave one.
    pub link: Option<String>,
    pub at: i64,
}
