use antilog_types::judge::{
    Cents, ComputeSection, DataSection, ExperimentSection, HoldoutResult, Outcome, Receipt,
    ReceiptBody, ReceiptId, SealedExperiment, SecondOpinion, TrialId, TrialRecord, ValueModel,
};

use super::measure::best;
use super::value::{Delivery, WinValue, win_return};
use crate::signer::{Signature, Signer};

/// What the receipt carries that the judge does not measure itself.
#[derive(Debug, Clone)]
pub struct Context {
    pub value_model: Option<ValueModel>,
    pub second_opinion: Option<SecondOpinion>,
    pub spend: Cents,
    pub cost_of_win: Cents,
    pub data: DataSection,
    pub compute: ComputeSection,
}

#[derive(Debug, thiserror::Error)]
pub enum ReceiptError {
    #[error("the trial is not Kept")]
    NotKept,
    #[error("the trial is Kept but is not the best Kept trial, so it is not the win")]
    NotTheWin,
    #[error("the value model given is not the one the experiment was sealed with")]
    ValueModelMismatch,
    #[error("the receipt cannot be written as canonical bytes")]
    Unhashable(#[from] serde_json::Error),
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum VerifyError {
    #[error("the receipt was signed by another key")]
    UnknownSigner,
    #[error("the signature does not match the receipt")]
    BadSignature,
    #[error("the judge image digest is not the one recorded")]
    DigestMismatch,
    #[error("the receipt cannot be written as canonical bytes")]
    Unhashable,
}

/// Builds and signs the receipt of the win. Only a Kept, best trial gets one.
pub fn issue(
    sealed: &SealedExperiment,
    records: &[TrialRecord],
    trial: TrialId,
    context: Context,
    signer: &impl Signer,
) -> Result<Receipt, ReceiptError> {
    let direction = sealed.definition().metric.direction;
    let record = records
        .iter()
        .find(|r| r.trial.id == trial)
        .filter(|r| r.outcome() == Outcome::Kept)
        .ok_or(ReceiptError::NotKept)?;
    if best(records, direction).map(|r| r.trial.id) != Some(trial) {
        return Err(ReceiptError::NotTheWin);
    }
    let measurement = record
        .finished
        .as_ref()
        .and_then(|f| f.measurement.as_ref())
        .ok_or(ReceiptError::NotKept)?;
    let sealed_model = sealed.definition().value_model.as_ref();
    let same = match (&context.value_model, sealed_model) {
        (None, None) => true,
        (Some(m), Some(r)) => m.id == r.id && m.version == r.version,
        _ => false,
    };
    if !same {
        return Err(ReceiptError::ValueModelMismatch);
    }
    let baseline = sealed.checks().baseline();
    let win = WinValue {
        direction,
        baseline,
        best: measurement.value,
        value_model: context.value_model.clone(),
        delivery: Delivery::NotOpened,
        regressed_to: None,
    };
    let tried = records
        .iter()
        .filter(|r| r.finished.as_ref().is_some_and(|f| f.measurement.is_some()))
        .count();
    let body = ReceiptBody {
        id: ReceiptId::new(),
        experiment: ExperimentSection {
            experiment: sealed.experiment(),
            version: sealed.version(),
            seal: sealed.seal_hash().clone(),
            trial,
            value_model: context.value_model,
            baseline,
            value: measurement.value,
            repeats: sealed.definition().repeats,
            noise: sealed.checks().noise(),
            trials_tried: u32::try_from(tried).unwrap_or(u32::MAX),
            holdout: measurement.holdout.unwrap_or(HoldoutResult::Failed),
            second_opinion: context.second_opinion,
            spend: context.spend,
            cost_of_win: context.cost_of_win,
            win_return: win_return(&win, context.cost_of_win),
        },
        data: context.data,
        compute: context.compute,
    };
    if body.experiment.holdout != HoldoutResult::Passed {
        return Err(ReceiptError::NotKept);
    }
    Ok(Receipt::sign(body, |bytes| {
        (signer.key_id().to_owned(), signer.sign(bytes).0)
    })?)
}

/// Verify again: the signature matches, the signer is the judge, the image digest is the recorded one.
pub fn verify(
    receipt: &Receipt,
    signer: &impl Signer,
    image_digest: &str,
) -> Result<(), VerifyError> {
    if receipt.key_id() != signer.key_id() {
        return Err(VerifyError::UnknownSigner);
    }
    let bytes = receipt
        .body()
        .bytes()
        .map_err(|_| VerifyError::Unhashable)?;
    if !signer.verify(&bytes, &Signature(receipt.signature().to_vec())) {
        return Err(VerifyError::BadSignature);
    }
    if receipt.body().compute.judge_digest != image_digest {
        return Err(VerifyError::DigestMismatch);
    }
    Ok(())
}
