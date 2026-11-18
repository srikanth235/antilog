use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use super::id;

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
