//! A win leaves as pull requests, and what the code host says of them decides what it is worth.
//!
//! Opening one is what an approved Open pull request card does. Everything after that is the
//! code host's word, received by `handle`: a merge makes the win Delivered, a close takes it
//! back to not opened. Only Antilog writes these facts; no person or agent can.

use antilog_types::judge::{
    CommitRef, Outcome, PullRequestState, TrialRecord, WinEvent, WinEventKind,
};
use antilog_types::{Approval, ApprovalStatus, CardKind, CardPayload, OrganizationId};

use crate::code_host::{CodeHost, PullRequestSpec};
use crate::deps::Deps;
use crate::organization::Refusal;
use crate::session::now_ms;
use crate::store::judge::JudgeStore;
use crate::store::{CodeHostStore, PullRequestRow};
use crate::value::record_win_event;

/// Why a pull request was not opened, worded for the agent that asked.
#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct DeliveryError(pub String);

fn error(text: impl Into<String>) -> DeliveryError {
    DeliveryError(text.into())
}

/// The last commit in each repo, in the order the repos first appear.
fn per_repo(commits: &[CommitRef]) -> Vec<CommitRef> {
    let mut out: Vec<CommitRef> = Vec::new();
    for commit in commits {
        match out.iter_mut().find(|have| have.repo == commit.repo) {
            Some(have) => have.sha.clone_from(&commit.sha),
            None => out.push(commit.clone()),
        }
    }
    out
}

fn body(record: &TrialRecord, unit: &str, baseline: f64, siblings: &[String]) -> String {
    let measured = record
        .finished
        .as_ref()
        .and_then(|finished| finished.measurement.as_ref())
        .map_or_else(String::new, |m| {
            format!(
                "Measured by the judge: {baseline:.2} {unit} before, {:.2} {unit} after (noise {:.2}). ",
                m.value, m.noise
            )
        });
    let linked = if siblings.is_empty() {
        String::new()
    } else {
        format!(" Linked pull requests in: {}.", siblings.join(", "))
    };
    format!(
        "{}\n\n{measured}The signed receipt is for trial {}.{linked}",
        record.trial.hypothesis, record.trial.id
    )
}

/// Opens one pull request per repo the trial changed, once a person approved the card, and
/// records that they are open. Returns the sentence the session is told.
pub async fn open_approved<D: Deps>(
    deps: &D,
    approval: &Approval,
) -> Result<String, DeliveryError> {
    let approved = matches!(
        approval.status,
        ApprovalStatus::Approved | ApprovalStatus::ApprovedByPolicy
    );
    let Some(CardPayload::OpenPullRequest { trial, .. }) = &approval.payload else {
        return Err(error("this card does not say which trial to open"));
    };
    if approval.card != CardKind::OpenPullRequest || !approved {
        return Err(error("only an approved Open pull request card opens one"));
    }
    let org = approval.organization_id;
    let judge = deps.judge(org);
    let record = judge
        .trial(*trial)
        .await
        .map_err(|_| error("the trial cannot be read now"))?
        .filter(|record| record.outcome() == Outcome::Kept)
        .ok_or_else(|| error("that trial is not a Kept trial of this experiment"))?;
    let existing = deps
        .store()
        .pull_requests_of(org, *trial)
        .await
        .map_err(|_| error("the pull requests cannot be read now"))?;
    if !existing.is_empty() {
        let urls: Vec<&str> = existing.iter().map(|row| row.url.as_str()).collect();
        return Ok(format!(
            "Pull requests are already open: {}.",
            urls.join(", ")
        ));
    }
    let sealed = judge
        .sealed(record.trial.experiment, record.trial.version)
        .await
        .ok()
        .flatten()
        .ok_or_else(|| error("the sealed experiment cannot be read now"))?;
    let metric = &sealed.definition().metric;
    let commits = per_repo(&record.trial.commits);
    let repos: Vec<String> = commits.iter().map(|c| c.repo.clone()).collect();
    let mut opened = Vec::new();
    let mut failure = None;
    for commit in &commits {
        let siblings: Vec<String> = repos
            .iter()
            .filter(|repo| **repo != commit.repo)
            .cloned()
            .collect();
        let spec = PullRequestSpec {
            repo: commit.repo.clone(),
            sha: commit.sha.clone(),
            title: format!("{}: {}", metric.name, record.trial.hypothesis),
            body: body(&record, &metric.unit, sealed.checks().baseline(), &siblings),
        };
        match deps.code().open_pull_request(&spec).await {
            Ok(pull_request) => opened.push(pull_request),
            Err(cause) => {
                failure = Some(cause.to_string());
                break;
            }
        }
    }
    let rows: Vec<PullRequestRow> = opened
        .iter()
        .map(|pull_request| PullRequestRow {
            organization: org,
            repo: pull_request.repo.clone(),
            number: pull_request.number,
            experiment: record.trial.experiment,
            version: record.trial.version,
            trial: *trial,
            url: pull_request.url.clone(),
            state: PullRequestState::Open,
        })
        .collect();
    if !rows.is_empty() {
        deps.store()
            .put_pull_requests(org, &rows)
            .await
            .map_err(|_| error("the pull requests were opened but could not be recorded"))?;
    }
    if let Some(cause) = failure {
        let done: Vec<&str> = opened.iter().map(|pr| pr.url.as_str()).collect();
        return Err(error(format!(
            "the code host refused ({cause}); opened so far: {}",
            if done.is_empty() {
                "none".to_owned()
            } else {
                done.join(", ")
            }
        )));
    }
    let first = rows.first().map(|row| row.url.clone());
    note(deps, org, &record, WinEventKind::Opened, first)
        .await
        .map_err(|_| error("the pull requests are open but could not be recorded as the win's"))?;
    Ok(format!(
        "Opened {}. Review comments and merges arrive as messages.",
        rows.iter()
            .map(|row| row.url.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    ))
}

async fn note<D: Deps>(
    deps: &D,
    org: OrganizationId,
    record: &TrialRecord,
    kind: WinEventKind,
    link: Option<String>,
) -> Result<(), Refusal> {
    record_win_event(
        deps.store(),
        &deps.judge(org),
        org,
        WinEvent {
            experiment: record.trial.experiment,
            version: record.trial.version,
            trial: record.trial.id,
            kind,
            value: None,
            link,
            at: now_ms(deps),
        },
    )
    .await
}

/// What the code host did to a pull request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PullRequestAction {
    Reopened,
    Closed {
        merged: bool,
    },
    /// Anything else the code host reports (edited, labelled, a review): nothing to record.
    Other,
}

/// One delivery from the code host, already authenticated and parsed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PullRequestEvent {
    /// The code host's id for this delivery: a redelivery has the same one.
    pub delivery: String,
    pub repo: String,
    pub number: u64,
    pub action: PullRequestAction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Handled {
    /// No pull request Antilog opened has this name.
    Unknown,
    /// This delivery was handled before.
    Duplicate,
    /// Not a fact about a win, or it does not follow what is known of it.
    Ignored,
    /// Merged, but a linked pull request in another repo is still open: the win is not Delivered yet.
    Waiting,
    Recorded(WinEventKind),
}

/// Applies one delivery. Safe to run again: a delivery is marked handled only after it was applied,
/// and a fact that does not follow the one before it is ignored rather than written.
pub async fn handle<D: Deps>(deps: &D, event: &PullRequestEvent) -> Result<Handled, Refusal> {
    let store = deps.store();
    let Some(pull_request) = store.pull_request(&event.repo, event.number).await? else {
        return Ok(Handled::Unknown);
    };
    let org = pull_request.organization;
    if store.delivery_seen(org, &event.delivery).await? {
        return Ok(Handled::Duplicate);
    }
    let record = deps
        .judge(org)
        .trial(pull_request.trial)
        .await?
        .ok_or(Refusal::NotFound)?;
    let siblings = store.pull_requests_of(org, pull_request.trial).await?;
    let link = siblings.first().map(|row| row.url.clone());
    let kind = match event.action {
        PullRequestAction::Other => None,
        PullRequestAction::Reopened => {
            store
                .set_pull_request_state(org, &event.repo, event.number, PullRequestState::Open)
                .await?;
            Some(WinEventKind::Opened)
        }
        PullRequestAction::Closed { merged: true } => {
            store
                .set_pull_request_state(org, &event.repo, event.number, PullRequestState::Merged)
                .await?;
            let all = siblings.iter().all(|row| {
                row.state == PullRequestState::Merged
                    || (row.repo == event.repo && row.number == event.number)
            });
            all.then_some(WinEventKind::Merged)
        }
        PullRequestAction::Closed { merged: false } => {
            store
                .set_pull_request_state(org, &event.repo, event.number, PullRequestState::Closed)
                .await?;
            Some(WinEventKind::Closed)
        }
    };
    let handled = match kind {
        None if matches!(event.action, PullRequestAction::Closed { merged: true }) => {
            Handled::Waiting
        }
        None => Handled::Ignored,
        Some(kind) => match note(deps, org, &record, kind, link).await {
            Ok(()) => Handled::Recorded(kind),
            Err(Refusal::Invalid(_)) => Handled::Ignored,
            Err(other) => return Err(other),
        },
    };
    store
        .put_delivery(org, &event.delivery, now_ms(deps))
        .await?;
    Ok(handled)
}
