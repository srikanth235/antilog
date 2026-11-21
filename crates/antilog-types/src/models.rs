//! The models a person may pick in the composer: those of the providers the organization has a key
//! for, with what the picker shows (context window, price) and the effort levels each offers.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::ModelProvider;

/// One model the organization's keys reach.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ModelInfo {
    /// `provider/model`, as a session names it.
    pub id: String,
    /// What the picker shows, such as "GLM 5.3 Flash".
    pub name: String,
    pub provider: ModelProvider,
    /// The most input tokens, when the source says.
    pub context_window: Option<u64>,
    /// US dollars per million input tokens; `None` when the source gives no price.
    pub input_price: Option<f64>,
    /// US dollars per million output tokens.
    pub output_price: Option<f64>,
    /// The effort levels the model offers, lowest first, named as OpenCode's model variants.
    /// Empty: the model has no effort setting.
    pub reasoning_levels: Vec<String>,
}

/// A session's model and effort changed from its next message on.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SessionModel {
    /// `provider/model`.
    pub model: String,
    /// One of the model's levels; left out, the model's own default.
    #[serde(default)]
    pub effort: Option<String>,
}
