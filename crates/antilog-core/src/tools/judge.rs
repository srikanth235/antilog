use antilog_types::judge::{
    AgentRun, Cents, CommitRef, ComputeSection, DataSection, Definition, Outcome, ProtectedFile,
    SealedExperiment, Trial, TrialId, TrialRecord,
};
use antilog_types::{
    Approval, ApprovalStatus, CardKind, CardPayload, ExperimentId, OrganizationId, SessionId,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::setup::{Card, ask, waiting};
use super::{Caller, Done, Ticket, ToolError, input};
use crate::code_host::CodeHost;
use crate::deps::Deps;
use crate::ids::uuid_of;
use crate::judge::measure::{MeasureError, best};
use crate::judge::receipt::{Context, issue};
use crate::judge::seal::{PlantedCheat, Proposal, run_checks};
use crate::platform::Platform;
use crate::spend::ensure_budget;
use crate::store::judge::JudgeStore;
use crate::store::{RelayStore, SessionStore};

fn word(outcome: Outcome) -> String {
    serde_json::to_value(outcome)
        .ok()
        .and_then(|v| v.as_str().map(str::to_owned))
        .unwrap_or_default()
}

const NOT_SEALED: &str = "not sealed: call seal with a definition first";

/// The latest sealed version, if any.
async fn latest<D: Deps>(
    deps: &D,
    organization: OrganizationId,
    experiment: ExperimentId,
) -> Result<Option<SealedExperiment>, ToolError> {
    let judge = deps.judge(organization);
    let mut found = None;
    for version in 1..=64 {
        match judge.sealed(experiment, version).await? {
            Some(sealed) => found = Some(sealed),
            None => break,
        }
    }
    Ok(found)
}

async fn records<D: Deps>(
    deps: &D,
    organization: OrganizationId,
    experiment: ExperimentId,
) -> Result<Vec<TrialRecord>, ToolError> {
    match latest(deps, organization, experiment).await? {
        Some(sealed) => Ok(deps
            .judge(organization)
            .trials(sealed.experiment(), sealed.version())
            .await?),
        None => Ok(vec![]),
    }
}

/// One line per trial for a session that starts from what others tried.
pub(super) async fn tried<D: Deps>(deps: &D, caller: &Caller) -> String {
    let rows = records(deps, caller.organization, caller.experiment)
        .await
        .unwrap_or_default();
    rows.iter()
        .rev()
        .take(10)
        .map(|r| format!("- {}: {}", r.trial.hypothesis, word(r.outcome())))
        .collect::<Vec<_>>()
        .join("\n")
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CheatIn {
    name: String,
    commits: Vec<CommitRef>,
    #[serde(default)]
    changed: Vec<(String, String)>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SealIn {
    definition: Definition,
    #[serde(default)]
    files: Vec<ProtectedFile>,
    #[serde(default)]
    cheats: Vec<CheatIn>,
}

#[derive(Serialize, Deserialize)]
struct Stored {
    version: u32,
    proposal: SealIn,
}

fn planted(cheats: Vec<CheatIn>) -> Vec<PlantedCheat> {
    cheats
        .into_iter()
        .map(|c| PlantedCheat {
            name: c.name,
            commits: c.commits,
            changed: c.changed,
        })
        .collect()
}

pub(super) async fn seal<D: Deps>(deps: &D, caller: &Caller, value: Value) -> Done {
    let proposal: SealIn = input(value)?;
    let version = latest(deps, caller.organization, caller.experiment)
        .await?
        .map_or(1, |s| s.version() + 1);
    let checks = run_checks(
        deps.platform(),
        &proposal.definition,
        &proposal.files,
        &planted(
            proposal
                .cheats
                .iter()
                .map(|c| CheatIn {
                    name: c.name.clone(),
                    commits: c.commits.clone(),
                    changed: c.changed.clone(),
                })
                .collect(),
        ),
    )
    .await
    .map_err(|error| {
        ToolError::new(format!(
            "the seal checks refused it: {error}. Fix the definition and call seal again"
        ))
    })?;
    let metric = &proposal.definition.metric;
    // An agent may only point at a value model a project owner already set for this metric. A
    // model that is not the project's newest for the metric needs a project owner's approval.
    let org = caller.organization;
    let project = deps
        .store()
        .experiment(org, caller.experiment)
        .await?
        .map(|row| row.project_id)
        .ok_or_else(|| ToolError::new("this experiment is not in the ledger"))?;
    let judge = deps.judge(org);
    let newest = judge
        .value_models(project)
        .await?
        .into_iter()
        .find(|row| row.model.metric == metric.name);
    if let Some(reference) = &proposal.definition.value_model {
        let found = judge
            .value_model(reference.id, reference.version)
            .await?
            .ok_or_else(|| {
                ToolError::new(
                    "no such value model: only a project owner sets one. Seal with the project's model for this metric, or with none",
                )
            })?;
        if found.project_id != project || found.model.metric != metric.name {
            return Err(ToolError::new(
                "that value model belongs to another project or metric: use this project's model for the metric, or none",
            ));
        }
    }
    let needs_owner = proposal
        .definition
        .value_model
        .as_ref()
        .map(|reference| (reference.id, reference.version))
        != newest.map(|row| (row.model.id, row.model.version));
    let worth = match &proposal.definition.value_model {
        None => "No value set".to_owned(),
        Some(reference) => judge
            .value_model(reference.id, reference.version)
            .await?
            .map_or_else(String::new, |found| {
                format!(
                    "Value: each {} is worth ${:.2} a year, set by a project owner, measured offline",
                    found.model.unit,
                    found.model.cents_per_unit / 100.0
                )
            }),
    };
    let summary = format!(
        "Seal {} (version {version}): baseline {:.2} {}, noise {:.2}, {} planted cheats rejected. {worth}",
        metric.name,
        checks.baseline(),
        metric.unit,
        checks.noise(),
        checks.cheats().len()
    );
    let approval = ask(
        deps,
        caller,
        "seal",
        Card {
            kind: CardKind::SealExperiment,
            summary,
            payload: CardPayload::SealExperiment {
                version,
                metric: metric.name.clone(),
                unit: metric.unit.clone(),
                baseline: checks.baseline(),
                noise: checks.noise(),
                cheats_rejected: u32::try_from(checks.cheats().len()).unwrap_or(u32::MAX),
                value_note: worth,
            },
            cost_cents: None,
            needs_owner,
        },
    )
    .await?;
    let body = serde_json::to_string(&Stored { version, proposal })
        .map_err(|_| ToolError::new("the proposal cannot be stored"))?;
    deps.store()
        .put_proposal(caller.organization, approval.id, &body)
        .await?;
    let answer = json!({
        "approval": approval.id,
        "status": "waiting for a person",
        "note": "A card is in front of a person. Sealing and its outcome arrive as a message.",
        "version": version,
        "baseline": checks.baseline(),
        "noise": checks.noise(),
        "cheatsRejected": checks.cheats().len(),
    });
    Ok((answer, None))
}

/// Seals what a person approved. Runs the checks again, as the judge does, and marks the experiment.
pub async fn seal_approved<D: Deps>(deps: &D, approval: &Approval) -> Result<u32, ToolError> {
    use antilog_types::{AuditEntry, AuditOutcome, ExperimentStatus};
    let approved = matches!(
        approval.status,
        ApprovalStatus::Approved | ApprovalStatus::ApprovedByPolicy
    );
    if approval.card != CardKind::SealExperiment || !approved {
        return Err(ToolError::new(
            "only an approved Seal experiment card seals",
        ));
    }
    let org = approval.organization_id;
    let body = deps
        .store()
        .proposal(org, approval.id)
        .await?
        .ok_or_else(|| ToolError::new("no proposal is stored for this card: call seal again"))?;
    let Stored { version, proposal } = serde_json::from_str(&body)
        .map_err(|_| ToolError::new("the stored proposal cannot be read: call seal again"))?;
    let sealed = crate::judge::seal::seal(
        deps.platform(),
        &deps.judge(org),
        deps.clock(),
        Proposal {
            experiment: approval.experiment_id,
            version,
            definition: proposal.definition,
            files: proposal.files,
            cheats: planted(proposal.cheats),
        },
    )
    .await
    .map_err(|error| ToolError::new(format!("{error}. Call seal again")))?;
    let entry = AuditEntry::new(
        org,
        approval.decided_by,
        "experiment.seal",
        approval.experiment_id,
        AuditOutcome::Done,
        crate::session::now_ms(deps),
    );
    let _ = deps
        .store()
        .set_experiment_status(
            org,
            approval.experiment_id,
            ExperimentStatus::Sealed,
            &entry,
        )
        .await;
    Ok(sealed.version())
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct MeasureIn {
    commits: Vec<CommitRef>,
    hypothesis: String,
    parent: Option<TrialId>,
    version: Option<u32>,
}

pub(super) async fn measure<D: Deps>(deps: &D, caller: &Caller, value: Value) -> Done {
    use crate::store::SessionStore as _;
    let m: MeasureIn = input(value)?;
    if m.commits.is_empty() {
        return Err(ToolError::new("give one commit per repo changed"));
    }
    let experiment = deps
        .store()
        .experiment(caller.organization, caller.experiment)
        .await?
        .ok_or_else(|| ToolError::new("this experiment is unknown"))?;
    ensure_budget(
        deps.store(),
        deps.clock(),
        caller.organization,
        caller.experiment,
        experiment.budget_cap_cents,
    )
    .await
    .map_err(|error| ToolError::new(error.to_string()))?;
    let sealed = latest(deps, caller.organization, caller.experiment)
        .await?
        .ok_or_else(|| ToolError::new(NOT_SEALED))?;
    let version = m.version.unwrap_or(sealed.version());
    let trial = Trial {
        id: TrialId::from_uuid(uuid_of(&format!(
            "trial:{}:{}",
            caller.session, caller.call
        ))),
        experiment: sealed.experiment(),
        version,
        session: caller.session,
        parent: m.parent,
        commits: m.commits,
        hypothesis: m.hypothesis,
        at: crate::session::now_ms(deps),
    };
    let answer = json!({
        "ticket": trial.id,
        "status": "measuring",
        "note": "The judge's outcome arrives in this session as a message. Do not poll; use history to see trials."
    });
    Ok((answer, Some(Ticket { trial })))
}

pub(super) async fn finish<D: Deps>(deps: &D, caller: &Caller, ticket: &Ticket) -> String {
    let trial = &ticket.trial;
    let id = trial.id;
    let Ok(Some(sealed)) = deps
        .judge(caller.organization)
        .sealed(trial.experiment, trial.version)
        .await
    else {
        return format!(
            "Trial {id} was not measured: version {} is not sealed. Call seal.",
            trial.version
        );
    };
    let changed = match deps.code().changed_files(&trial.commits).await {
        Ok(changed) => changed,
        Err(error) => {
            return format!(
                "Trial {id} was not measured: {error}. Check the commits are pushed, then call measure again."
            );
        }
    };
    match crate::judge::measure::measure(
        deps.platform(),
        &deps.judge(caller.organization),
        &sealed,
        trial.clone(),
        &changed,
    )
    .await
    {
        Ok(record) => {
            let mut text = describe(&sealed, &record);
            if record.outcome() == Outcome::Kept
                && issue_receipt(deps, caller, &sealed, &record).await
            {
                text.push_str(" The judge signed a receipt.");
            }
            text
        }
        Err(MeasureError::Refused(refusal)) => {
            format!(
                "Trial {id} was refused: {refusal}. Read history, then ask a person with request_budget or keep going cards if you need more."
            )
        }
        Err(MeasureError::Store(_)) => {
            format!("Trial {id} could not be recorded now. Call measure again.")
        }
    }
}

/// The judge signs the receipt of a Kept trial that is the best so far; false when it cannot.
async fn issue_receipt<D: Deps>(
    deps: &D,
    caller: &Caller,
    sealed: &SealedExperiment,
    record: &TrialRecord,
) -> bool {
    let org = caller.organization;
    let judge = deps.judge(org);
    let Ok(records) = judge.trials(sealed.experiment(), sealed.version()).await else {
        return false;
    };
    let value_model = match &sealed.definition().value_model {
        Some(reference) => match judge.value_model(reference.id, reference.version).await {
            Ok(Some(found)) => Some(found.model),
            _ => return false,
        },
        None => None,
    };
    let spend = crate::spend::spent(deps.store(), deps.clock(), org, caller.experiment)
        .await
        .map_or(Cents::ZERO, |total| Cents(total.cents));
    let agents = match deps.store().session(org, record.trial.session).await {
        Ok(Some(session)) => vec![AgentRun {
            agent: session.agent,
            platform: session.platform,
            session: session.id,
        }],
        _ => Vec::new(),
    };
    let data = &sealed.definition().data;
    let context = Context {
        value_model,
        second_opinion: None,
        spend,
        cost_of_win: spend,
        data: DataSection {
            datasets: vec![data.evaluation.clone(), data.holdout.clone()],
            skill_packs: Vec::new(),
            commits: record.trial.commits.clone(),
            credential_scopes: Vec::new(),
        },
        compute: ComputeSection {
            platform: deps.platform().name().to_owned(),
            environment: "default".to_owned(),
            judge_image: "antilog-judge".to_owned(),
            judge_digest: "unset".to_owned(),
            agents,
        },
    };
    match issue(sealed, &records, record.trial.id, context, deps.signer()) {
        Ok(receipt) => judge.put_receipt(&receipt).await.is_ok(),
        Err(_) => false,
    }
}

fn describe(sealed: &SealedExperiment, record: &TrialRecord) -> String {
    let metric = &sealed.definition().metric;
    let outcome = word(record.outcome());
    let mut text = format!(
        "Trial {} ({}): {outcome}.",
        record.trial.id, record.trial.hypothesis
    );
    if let Some(m) = record
        .finished
        .as_ref()
        .and_then(|f| f.measurement.as_ref())
    {
        text.push_str(&format!(
            " {} {:.2} {} against {:.2} for the trial it started from; noise {:.2}.",
            metric.name, m.value, metric.unit, m.parent_value, m.noise
        ));
    }
    if let Some(f) = &record.finished {
        for reason in &f.decision.reasons {
            text.push_str(&format!(" {reason}."));
        }
    }
    text
}

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct HistoryIn {
    session: Option<SessionId>,
    outcome: Option<String>,
    limit: Option<usize>,
    offset: Option<usize>,
}

pub(super) async fn history<D: Deps>(deps: &D, caller: &Caller, value: Value) -> Done {
    let h: HistoryIn = if value.is_null() {
        HistoryIn::default()
    } else {
        input(value)?
    };
    let Some(sealed) = latest(deps, caller.organization, caller.experiment).await? else {
        return Ok((json!({"trials": [], "note": NOT_SEALED}), None));
    };
    let all = deps
        .judge(caller.organization)
        .trials(sealed.experiment(), sealed.version())
        .await?;
    let win = best(&all, sealed.definition().metric.direction).map(|r| r.trial.id);
    let rows: Vec<&TrialRecord> = all
        .iter()
        .filter(|r| h.session.is_none_or(|s| r.trial.session == s))
        .filter(|r| {
            h.outcome
                .as_ref()
                .is_none_or(|o| word(r.outcome()).eq_ignore_ascii_case(o))
        })
        .collect();
    let limit = h.limit.unwrap_or(20).clamp(1, 50);
    let offset = h.offset.unwrap_or(0);
    let page: Vec<Value> = rows
        .iter()
        .skip(offset)
        .take(limit)
        .map(|r| {
            json!({
                "trial": r.trial.id,
                "session": r.trial.session,
                "parent": r.trial.parent,
                "hypothesis": r.trial.hypothesis,
                "commits": r.trial.commits,
                "outcome": word(r.outcome()),
                "value": r.finished.as_ref().and_then(|f| f.measurement.as_ref()).map(|m| m.value),
            })
        })
        .collect();
    let next = (offset + limit < rows.len()).then_some(offset + limit);
    Ok((
        json!({"version": sealed.version(), "unit": sealed.definition().metric.unit, "best": win, "trials": page, "nextOffset": next}),
        None,
    ))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DeliverIn {
    trial: TrialId,
}

pub(super) async fn deliver<D: Deps>(deps: &D, caller: &Caller, value: Value) -> Done {
    let DeliverIn { trial } = input(value)?;
    let sealed = latest(deps, caller.organization, caller.experiment)
        .await?
        .ok_or_else(|| ToolError::new(NOT_SEALED))?;
    let all = deps
        .judge(caller.organization)
        .trials(sealed.experiment(), sealed.version())
        .await?;
    let record = all
        .iter()
        .find(|r| r.trial.id == trial)
        .ok_or_else(|| ToolError::new("no such trial in this experiment: read history for ids"))?;
    if record.outcome() != Outcome::Kept {
        return Err(ToolError::new(format!(
            "trial not Kept: deliver needs a Kept trial, this one is {}",
            word(record.outcome())
        )));
    }
    match best(&all, sealed.definition().metric.direction) {
        Some(win) if win.trial.id == trial => {}
        Some(win) => {
            return Err(ToolError::new(format!(
                "this Kept trial is not the win: deliver trial {}",
                win.trial.id
            )));
        }
        None => return Err(ToolError::new("there is no Kept trial yet: call measure")),
    }
    let summary = format!(
        "Open pull requests for trial {trial}: {}",
        record.trial.hypothesis
    );
    let approval = ask(
        deps,
        caller,
        "deliver",
        Card {
            kind: CardKind::OpenPullRequest,
            summary,
            payload: CardPayload::OpenPullRequest {
                trial,
                hypothesis: record.trial.hypothesis.clone(),
                repos: {
                    let mut repos: Vec<String> = Vec::new();
                    for commit in &record.trial.commits {
                        if !repos.contains(&commit.repo) {
                            repos.push(commit.repo.clone());
                        }
                    }
                    repos
                },
            },
            cost_cents: None,
            needs_owner: false,
        },
    )
    .await?;
    Ok((waiting(&approval), None))
}
