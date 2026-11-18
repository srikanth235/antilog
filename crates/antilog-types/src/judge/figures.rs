//! Value, Return and the Report as the API serves them. Every dollar figure is derived from
//! measured wins, sealed value models and spend rows, and none is stored.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::{Direction, ExperimentId, ProjectValueModel, TrialId};
use crate::{ProjectId, SessionId, SpendKind};

/// How a dollar figure was measured, said wherever one is shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum ValueBasis {
    /// By the judge on the evaluation data, not in production.
    MeasuredOffline,
}

/// Delivered, Pending, At risk and Return for one scope and time range, in US cents a year.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ValueFigures {
    pub basis: ValueBasis,
    pub delivered_cents: i64,
    pub pending_cents: i64,
    pub at_risk_cents: i64,
    /// Everything spent in the scope and range, experiments with no win included.
    pub spent_cents: i64,
    /// Delivered over spent; `null` when nothing was spent.
    pub return_x: Option<f64>,
    /// Wins whose pull request is merged and that count in Delivered or At risk.
    pub merged_wins: u32,
    /// Wins in an open pull request.
    pub pending_wins: u32,
    /// Wins with no value model: they count in no dollar figure.
    pub unvalued_wins: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum WinState {
    /// No pull request yet.
    NotOpened,
    Pending,
    Delivered,
    /// Merged, and the watch found it regressed.
    AtRisk,
}

/// One win of one experiment version: its best Kept trial.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WinView {
    pub experiment_id: ExperimentId,
    pub version: u32,
    pub trial_id: TrialId,
    pub metric: String,
    pub unit: String,
    pub direction: Direction,
    pub baseline: f64,
    pub best: f64,
    pub state: WinState,
    /// What the whole win is worth a year; `null` with no value model.
    pub yearly_cents: Option<i64>,
    pub delivered_cents: Option<i64>,
    pub at_risk_cents: Option<i64>,
    /// When its pull request was merged.
    pub merged_at: Option<i64>,
    pub pull_request: Option<String>,
}

/// An experiment's value: the model it was sealed with, its wins and what they add up to.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExperimentValue {
    pub experiment_id: ExperimentId,
    /// The model of the newest sealed version; `null` is "No value set".
    pub value_model: Option<ProjectValueModel>,
    pub wins: Vec<WinView>,
    pub figures: ValueFigures,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct KindSpendLine {
    pub kind: SpendKind,
    pub cents: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AgentRow {
    pub agent: String,
    pub trials: u32,
    pub kept: u32,
    pub wins_delivered: u32,
    pub wins_pending: u32,
    pub pending_cents: i64,
    pub spent_cents: i64,
    pub delivered_cents: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TopWin {
    pub win: WinView,
    pub project_id: ProjectId,
    pub goal: String,
    pub agent: Option<String>,
    pub session_id: Option<SessionId>,
    /// What the experiment spent in the range.
    pub spent_cents: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StoppedExperiment {
    pub experiment_id: ExperimentId,
    pub project_id: ProjectId,
    pub goal: String,
    pub trials: u32,
    pub spent_cents: i64,
    /// True when the experiment spent all of its budget.
    pub budget_spent: bool,
}

/// The monthly Report for the organization or one project.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Report {
    /// `YYYY-MM`, in UTC.
    pub month: String,
    pub from: i64,
    pub to: i64,
    pub project_id: Option<ProjectId>,
    pub figures: ValueFigures,
    pub spend_by_kind: Vec<KindSpendLine>,
    /// Spent on experiments that have a win.
    pub spent_with_win_cents: i64,
    /// Spent on sealed experiments with no win, stopped or still going.
    pub spent_no_win_cents: i64,
    /// Spent on experiments that are not sealed yet.
    pub spent_not_sealed_cents: i64,
    pub agents: Vec<AgentRow>,
    pub top_wins: Vec<TopWin>,
    pub stopped: Vec<StoppedExperiment>,
}
