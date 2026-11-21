use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

macro_rules! ids {
    ($($name:ident),+ $(,)?) => {$(
        #[derive(
            Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, ToSchema,
        )]
        #[serde(transparent)]
        pub struct $name(Uuid);

        impl $name {
            pub fn new() -> Self {
                Self(Uuid::now_v7())
            }

            pub const fn from_uuid(id: Uuid) -> Self {
                Self(id)
            }

            pub const fn as_uuid(&self) -> Uuid {
                self.0
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                self.0.fmt(f)
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }
    )+};
}

ids!(
    OrganizationId,
    MemberId,
    TokenId,
    SignInId,
    SessionId,
    SpendId,
    ApprovalId,
    AuditId,
    ConnectorId,
);

macro_rules! text_enum {
    ($(#[$meta:meta])* $name:ident { $($variant:ident => $text:literal),+ $(,)? }) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
        pub enum $name {
            $(#[serde(rename = $text)] #[schema(rename = $text)] $variant),+
        }

        impl $name {
            pub const ALL: &'static [Self] = &[$(Self::$variant),+];

            pub const fn as_str(self) -> &'static str {
                match self {
                    $(Self::$variant => $text),+
                }
            }

            pub fn parse(text: &str) -> Option<Self> {
                match text {
                    $($text => Some(Self::$variant),)+
                    _ => None,
                }
            }
        }
    };
}

pub(crate) use text_enum;

#[cfg(test)]
mod text_enum_schema {
    use crate::*;
    use utoipa::PartialSchema;

    /// The OpenAPI schema must list the same strings the server sends on the wire.
    fn schema_matches_wire<T: PartialSchema + Serialize>(all: &[T]) {
        let schema = serde_json::to_value(T::schema()).expect("schema serializes");
        let listed: Vec<String> = serde_json::from_value(schema["enum"].clone())
            .expect("a text enum's schema lists its values");
        let wire: Vec<String> = all
            .iter()
            .map(|v| {
                serde_json::to_value(v)
                    .expect("serializes")
                    .as_str()
                    .expect("a string")
                    .to_owned()
            })
            .collect();
        assert_eq!(listed, wire);
    }

    #[test]
    fn every_text_enum_schema_matches_its_wire_values() {
        schema_matches_wire(ApprovalStatus::ALL);
        schema_matches_wire(AuditOutcome::ALL);
        schema_matches_wire(CardKind::ALL);
        schema_matches_wire(CostTier::ALL);
        schema_matches_wire(Decision::ALL);
        schema_matches_wire(EventKind::ALL);
        schema_matches_wire(MemberKind::ALL);
        schema_matches_wire(PolicyMode::ALL);
        schema_matches_wire(Role::ALL);
        schema_matches_wire(SessionStatus::ALL);
        schema_matches_wire(SpendKind::ALL);
    }
}
