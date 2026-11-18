use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::{
    Cents, CommitRef, DatasetRef, ExperimentId, HoldoutResult, SealHash, TrialId, ValueModel,
    canonical_json, id,
};
use crate::SessionId;

id!(ReceiptId);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SecondOpinion {
    pub provider: String,
    pub flags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExperimentSection {
    pub experiment: ExperimentId,
    pub version: u32,
    pub seal: SealHash,
    pub trial: TrialId,
    pub value_model: Option<ValueModel>,
    pub baseline: f64,
    pub value: f64,
    pub repeats: u32,
    pub noise: f64,
    pub trials_tried: u32,
    pub holdout: HoldoutResult,
    pub second_opinion: Option<SecondOpinion>,
    pub spend: Cents,
    pub cost_of_win: Cents,
    pub win_return: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum CredentialHolder {
    Agent,
    Judge,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CredentialScope {
    pub holder: CredentialHolder,
    pub scope: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SkillPackRef {
    pub id: String,
    pub version: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DataSection {
    pub datasets: Vec<DatasetRef>,
    pub skill_packs: Vec<SkillPackRef>,
    pub commits: Vec<CommitRef>,
    pub credential_scopes: Vec<CredentialScope>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AgentRun {
    pub agent: String,
    pub platform: String,
    pub session: SessionId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ComputeSection {
    pub platform: String,
    pub environment: String,
    pub judge_image: String,
    pub judge_digest: String,
    pub agents: Vec<AgentRun>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReceiptBody {
    pub id: ReceiptId,
    pub experiment: ExperimentSection,
    pub data: DataSection,
    pub compute: ComputeSection,
}

impl ReceiptBody {
    pub fn bytes(&self) -> Result<Vec<u8>, serde_json::Error> {
        canonical_json(self)
    }
}

/// A body and the judge's signature over it. There is no unsigned receipt.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Receipt {
    body: ReceiptBody,
    key_id: String,
    signature: Vec<u8>,
}

impl Receipt {
    /// `sign` returns the key id and the signature over the canonical body bytes.
    pub fn sign(
        body: ReceiptBody,
        sign: impl FnOnce(&[u8]) -> (String, Vec<u8>),
    ) -> Result<Self, serde_json::Error> {
        let (key_id, signature) = sign(&body.bytes()?);
        Ok(Self {
            body,
            key_id,
            signature,
        })
    }

    pub fn body(&self) -> &ReceiptBody {
        &self.body
    }

    pub fn key_id(&self) -> &str {
        &self.key_id
    }

    pub fn signature(&self) -> &[u8] {
        &self.signature
    }
}
