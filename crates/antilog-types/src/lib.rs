#![forbid(unsafe_code)]

mod customize;
mod floor;
mod id;
mod ledger;
mod live;
mod organization;
mod runner;
mod session;

pub use customize::*;
pub use floor::*;
pub use id::*;
pub use ledger::*;
pub use live::*;
pub use organization::*;
pub use runner::*;
pub use session::*;

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum HealthStatus {
    Ok,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Health {
    pub status: HealthStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ApiError {
    pub code: String,
    pub message: String,
    pub fields: Vec<FieldError>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FieldError {
    pub field: String,
    pub message: String,
}
pub mod judge;
