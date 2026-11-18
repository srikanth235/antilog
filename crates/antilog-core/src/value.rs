//! Value models, Delivered, Pending, At risk, Return and the monthly Report.
//!
//! Every figure is derived on read from sealed value models, the judge's measurements, the
//! facts about each win's pull request and the spend rows. None is stored, and all of them are
//! "measured offline": by the judge on the evaluation data, not in production.

use antilog_types::judge::{
    AgentRow, Cents, ExperimentValue, KindSpendLine, NewValueModel, ProjectValueModel, Report,
    StoppedExperiment, TopWin, TrialId, TrialRecord, ValueBasis, ValueFigures, ValueModel,
    ValueModelId, ValueModels, WinEvent, WinEventKind, WinState, WinView,
};
use antilog_types::{
    AuditOutcome, Experiment, ExperimentId, OrganizationId, Project, ProjectId, SessionId,
};

use crate::budget::month_bounds;
use crate::judge::measure::best;
use crate::judge::value::{Delivery, WinValue, figures};
use crate::organization::{Action, Principal, Refusal, entry, member_of, permit};
use crate::store::judge::{JudgeStore, JudgeStoreError};
use crate::store::{SpendLine, SpendScope, Store};

impl From<JudgeStoreError> for Refusal {
    fn from(error: JudgeStoreError) -> Self {
        match error {
            JudgeStoreError::Conflict => Self::Conflict,
            JudgeStoreError::NotFound => Self::NotFound,
            JudgeStoreError::Unavailable(source) => Self::Unavailable(source),
        }
    }
}

/// A time range in epoch milliseconds: `from` included, `to` excluded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Range {
    pub from: i64,
    pub to: i64,
}

impl Range {
    /// Everything up to a year after `now`.
    pub fn all_time(now: i64) -> Self {
        Self {
            from: 0,
            to: now.saturating_add(366 * 24 * 3600 * 1000),
        }
    }

    pub fn new(now: i64, from: Option<i64>, to: Option<i64>) -> Result<Self, Refusal> {
        let all = Self::all_time(now);
        let range = Self {
            from: from.unwrap_or(all.from),
            to: to.unwrap_or(all.to),
        };
        if range.from < 0 || range.to <= range.from {
            return Err(Refusal::Invalid("a time range ends after it starts"));
        }
        Ok(range)
    }
}

// ---------------------------------------------------------------- value models

fn validate(new: &NewValueModel) -> Result<(), Refusal> {
    if !(1..=200).contains(&new.metric.trim().chars().count()) {
        return Err(Refusal::Invalid("a metric is 1 to 200 characters"));
    }
    if new.unit.trim().chars().count() > 40 {
        return Err(Refusal::Invalid("a unit is at most 40 characters"));
    }
    if !(new.cents_per_unit.is_finite() && new.cents_per_unit > 0.0 && new.cents_per_unit < 1.0e12)
    {
        return Err(Refusal::Invalid(
            "what a unit is worth is more than zero dollars a year",
        ));
    }
    if !(1..=500).contains(&new.source.text.trim().chars().count()) {
        return Err(Refusal::Invalid(
            "a value model names its source in 1 to 500 characters",
        ));
    }
    if new.source.link.as_ref().is_some_and(|link| {
        link.len() > 2000 || !(link.starts_with("https://") || link.starts_with("http://"))
    }) {
        return Err(Refusal::Invalid(
            "a source link is an http or https address",
        ));
    }
    Ok(())
}

async fn known_project<S: Store>(
    store: &S,
    who: &Principal,
    project: ProjectId,
) -> Result<Project, Refusal> {
    store
        .project(who.organization, project)
        .await?
        .ok_or(Refusal::NotFound)
}

/// Every member reads a project's value models: the newest version of each metric's.
pub async fn value_models<S: Store, J: JudgeStore>(
    store: &S,
    judge: &J,
    who: &Principal,
    project: ProjectId,
) -> Result<ValueModels, Refusal> {
    member_of(store, who).await?;
    known_project(store, who, project).await?;
    Ok(ValueModels {
        models: judge.value_models(project).await?,
    })
}

/// A project owner sets what one unit of a metric is worth. A change is the next version under
/// the same id: earlier versions stay, so experiments sealed on them keep their number. An agent
/// has no way here: this takes a signed-in member, checked against the project's owners.
pub async fn set_value_model<S: Store, J: JudgeStore>(
    store: &S,
    judge: &J,
    who: &Principal,
    now: i64,
    project: ProjectId,
    new: NewValueModel,
) -> Result<ProjectValueModel, Refusal> {
    validate(&new)?;
    permit(
        store,
        who,
        Action::ValueModelSet,
        Some(project),
        new.metric.as_str(),
        now,
    )
    .await?;
    let metric = new.metric.trim().to_owned();
    let current = judge
        .value_models(project)
        .await?
        .into_iter()
        .find(|row| row.model.metric == metric);
    let (id, version) = match &current {
        Some(row) => (row.model.id, row.model.version.saturating_add(1)),
        None => (ValueModelId::new(), 1),
    };
    let model = ValueModel {
        id,
        version,
        metric,
        unit: new.unit.trim().to_owned(),
        cents_per_unit: new.cents_per_unit,
        source: antilog_types::judge::ValueSource {
            text: new.source.text.trim().to_owned(),
            link: new.source.link,
        },
        set_by: who.member.as_uuid(),
    };
    judge.put_value_model(project, &model, now).await?;
    store
        .append_audit(&entry(
            who,
            "value_model.set",
            format!("{project}:{}@{}", model.id, model.version),
            AuditOutcome::Done,
            now,
        ))
        .await?;
    Ok(ProjectValueModel {
        project_id: project,
        model,
        set_at: now,
    })
}

// ---------------------------------------------------------------- wins

/// What the facts about one win's pull request say at a moment.
#[derive(Debug, Clone, PartialEq)]
pub struct Replay {
    pub delivery: Delivery,
    pub merged_at: Option<i64>,
    pub link: Option<String>,
    pub regressed_to: Option<f64>,
}

/// The state of one trial's pull request from the facts before `until`, in the order they were
/// appended. A closed pull request that never merged is not opened; a merge stays merged.
pub fn replay(events: &[WinEvent], trial: TrialId, until: i64) -> Replay {
    let mut now = Replay {
        delivery: Delivery::NotOpened,
        merged_at: None,
        link: None,
        regressed_to: None,
    };
    for event in events
        .iter()
        .filter(|event| event.trial == trial && event.at < until)
    {
        match event.kind {
            WinEventKind::Opened => {
                if now.delivery != Delivery::Merged {
                    now.delivery = Delivery::Open;
                }
                if event.link.is_some() {
                    now.link.clone_from(&event.link);
                }
            }
            WinEventKind::Merged => {
                now.delivery = Delivery::Merged;
                now.merged_at = Some(event.at);
                if event.link.is_some() {
                    now.link.clone_from(&event.link);
                }
            }
            WinEventKind::Closed => {
                if now.delivery == Delivery::Open {
                    now.delivery = Delivery::NotOpened;
                }
            }
            WinEventKind::Regressed => now.regressed_to = event.value,
            WinEventKind::Recovered => now.regressed_to = None,
        }
    }
    now
}

/// One win as the figures read it, with what the view needs besides.
#[derive(Debug, Clone)]
struct Found {
    value: WinValue,
    experiment: ExperimentId,
    version: u32,
    trial: TrialId,
    session: SessionId,
    metric: String,
    unit: String,
    replay: Replay,
}

impl Found {
    /// Counted in a range: still in an open pull request at its end, or merged inside it.
    fn counts_in(&self, range: Range) -> bool {
        match self.value.delivery {
            Delivery::NotOpened => false,
            Delivery::Open => true,
            Delivery::Merged => self.replay.merged_at.is_some_and(|at| at >= range.from),
        }
    }

    fn view(&self) -> WinView {
        let yearly = self.value.yearly();
        let lost = (self.value.delivery == Delivery::Merged)
            .then(|| self.value.lost())
            .flatten()
            .zip(yearly)
            .map(|(lost, all)| lost.min(all));
        let state = match self.value.delivery {
            Delivery::NotOpened => WinState::NotOpened,
            Delivery::Open => WinState::Pending,
            Delivery::Merged if lost.is_some_and(|lost| lost > Cents::ZERO) => WinState::AtRisk,
            Delivery::Merged => WinState::Delivered,
        };
        let merged = self.value.delivery == Delivery::Merged;
        WinView {
            experiment_id: self.experiment,
            version: self.version,
            trial_id: self.trial,
            metric: self.metric.clone(),
            unit: self.unit.clone(),
            direction: self.value.direction,
            baseline: self.value.baseline,
            best: self.value.best,
            state,
            yearly_cents: yearly.map(|cents| cents.0),
            delivered_cents: yearly
                .filter(|_| merged)
                .map(|all| all.saturating_sub(lost.unwrap_or(Cents::ZERO)).0),
            at_risk_cents: lost.filter(|_| merged).map(|cents| cents.0),
            merged_at: self.replay.merged_at,
            pull_request: self.replay.link.clone(),
        }
    }
}

/// An experiment's wins and every trial, read from the judge's tables.
struct Scan {
    wins: Vec<Found>,
    trials: Vec<TrialRecord>,
}

/// One win per sealed version: the best Kept trial whose pull request was merged, else the best
/// whose pull request is open, else the best Kept trial. A later, better trial that nobody
/// delivered does not take a merged win's place, and two merged trials of one lineage never add.
async fn scan<J: JudgeStore>(
    judge: &J,
    experiment: &Experiment,
    until: i64,
) -> Result<Scan, Refusal> {
    let events = judge.win_events(experiment.id).await?;
    let mut scan = Scan {
        wins: Vec::new(),
        trials: Vec::new(),
    };
    let mut version = 1;
    while let Some(sealed) = judge.sealed(experiment.id, version).await? {
        let definition = sealed.definition();
        let direction = definition.metric.direction;
        let records = judge.trials(experiment.id, version).await?;
        let mut candidates: Vec<(Replay, &TrialRecord)> = records
            .iter()
            .filter(|record| record.outcome() == antilog_types::judge::Outcome::Kept)
            .map(|record| (replay(&events, record.trial.id, until), record))
            .collect();
        let rank = |delivery: Delivery| match delivery {
            Delivery::Merged => 2,
            Delivery::Open => 1,
            Delivery::NotOpened => 0,
        };
        let tier = candidates
            .iter()
            .map(|(replay, _)| rank(replay.delivery))
            .max();
        if let Some(tier) = tier {
            candidates.retain(|(replay, _)| rank(replay.delivery) == tier);
            let pool: Vec<TrialRecord> = candidates
                .iter()
                .map(|(_, record)| (*record).clone())
                .collect();
            if let Some(winner) = best(&pool, direction)
                && let Some((replay, record)) = candidates
                    .into_iter()
                    .find(|(_, record)| record.trial.id == winner.trial.id)
                && let Some(measurement) = record
                    .finished
                    .as_ref()
                    .and_then(|finished| finished.measurement.as_ref())
            {
                let model = match &definition.value_model {
                    Some(reference) => judge.value_model(reference.id, reference.version).await?,
                    None => None,
                };
                scan.wins.push(Found {
                    value: WinValue {
                        direction,
                        baseline: sealed.checks().baseline(),
                        best: measurement.value,
                        value_model: model.as_ref().map(|row| row.model.clone()),
                        delivery: replay.delivery,
                        regressed_to: replay.regressed_to,
                    },
                    experiment: experiment.id,
                    version,
                    trial: record.trial.id,
                    session: record.trial.session,
                    metric: definition.metric.name.clone(),
                    unit: definition.metric.unit.clone(),
                    replay,
                });
            }
        }
        scan.trials.extend(records);
        version += 1;
    }
    Ok(scan)
}

fn to_figures(found: &[&Found], spent: i64) -> Result<ValueFigures, Refusal> {
    let wins: Vec<WinValue> = found.iter().map(|win| win.value.clone()).collect();
    let figures = figures(&wins, Cents(spent))
        .map_err(|_| Refusal::Invalid("a dollar figure is too large to add up"))?;
    let count = |delivery: Delivery| {
        u32::try_from(
            found
                .iter()
                .filter(|win| win.value.delivery == delivery)
                .count(),
        )
        .unwrap_or(u32::MAX)
    };
    Ok(ValueFigures {
        basis: ValueBasis::MeasuredOffline,
        delivered_cents: figures.delivered.0,
        pending_cents: figures.pending.0,
        at_risk_cents: figures.at_risk.0,
        spent_cents: figures.spend.0,
        return_x: figures.return_x,
        merged_wins: count(Delivery::Merged),
        pending_wins: count(Delivery::Open),
        unvalued_wins: figures.unvalued,
    })
}

async fn experiments_in<S: Store>(
    store: &S,
    organization: OrganizationId,
    project: Option<ProjectId>,
) -> Result<Vec<Experiment>, Refusal> {
    let mut all = Vec::new();
    match project {
        Some(project) => all.extend(store.experiments(organization, project).await?),
        None => {
            for project in store.projects(organization).await? {
                all.extend(store.experiments(organization, project.id).await?);
            }
        }
    }
    Ok(all)
}

async fn figures_of<S: Store, J: JudgeStore>(
    store: &S,
    judge: &J,
    organization: OrganizationId,
    scope: SpendScope,
    experiments: &[Experiment],
    range: Range,
) -> Result<ValueFigures, Refusal> {
    let mut found = Vec::new();
    for experiment in experiments {
        found.extend(scan(judge, experiment, range.to).await?.wins);
    }
    let counted: Vec<&Found> = found.iter().filter(|win| win.counts_in(range)).collect();
    let spent = store
        .spend(organization, scope, range.from, range.to)
        .await?
        .cents;
    to_figures(&counted, spent)
}

/// A project's Delivered, Pending, At risk and Return in a time range. Every member reads.
pub async fn project_value<S: Store, J: JudgeStore>(
    store: &S,
    judge: &J,
    who: &Principal,
    project: ProjectId,
    range: Range,
) -> Result<ValueFigures, Refusal> {
    member_of(store, who).await?;
    known_project(store, who, project).await?;
    let experiments = experiments_in(store, who.organization, Some(project)).await?;
    figures_of(
        store,
        judge,
        who.organization,
        SpendScope::Project(project),
        &experiments,
        range,
    )
    .await
}

/// The same for the whole organization: organization owners and editors read it.
pub async fn organization_value<S: Store, J: JudgeStore>(
    store: &S,
    judge: &J,
    who: &Principal,
    now: i64,
    range: Range,
) -> Result<ValueFigures, Refusal> {
    permit(store, who, Action::ReportRead, None, "value", now).await?;
    let experiments = experiments_in(store, who.organization, None).await?;
    figures_of(
        store,
        judge,
        who.organization,
        SpendScope::Organization,
        &experiments,
        range,
    )
    .await
}

/// One experiment's value model (of its newest sealed version), its wins and their figures.
/// With no model the experiment says "No value set" and its wins count in no dollar figure.
pub async fn experiment_value<S: Store, J: JudgeStore>(
    store: &S,
    judge: &J,
    who: &Principal,
    experiment: ExperimentId,
    range: Range,
) -> Result<ExperimentValue, Refusal> {
    member_of(store, who).await?;
    let row = store
        .experiment(who.organization, experiment)
        .await?
        .ok_or(Refusal::NotFound)?;
    let found = scan(judge, &row, range.to).await?;
    let mut value_model = None;
    let mut version = 1;
    while let Some(sealed) = judge.sealed(experiment, version).await? {
        value_model = match &sealed.definition().value_model {
            Some(reference) => judge.value_model(reference.id, reference.version).await?,
            None => None,
        };
        version += 1;
    }
    let counted: Vec<&Found> = found
        .wins
        .iter()
        .filter(|win| win.counts_in(range))
        .collect();
    let spent = store
        .spend(
            who.organization,
            SpendScope::Experiment(experiment),
            range.from,
            range.to,
        )
        .await?
        .cents;
    Ok(ExperimentValue {
        experiment_id: experiment,
        value_model,
        wins: found.wins.iter().map(Found::view).collect(),
        figures: to_figures(&counted, spent)?,
    })
}

// ---------------------------------------------------------------- facts about a win

/// The code host or the watch tells Antilog what became of a win's pull request. There is no
/// person here and no route: only Antilog itself appends these, and the append is audited as
/// Antilog's. Each fact must follow the one before it, so a win cannot be Delivered by a merge
/// nobody opened, or recover from a regression it never had.
pub async fn record_win_event<S: Store, J: JudgeStore>(
    store: &S,
    judge: &J,
    organization: OrganizationId,
    event: WinEvent,
) -> Result<(), Refusal> {
    if judge
        .sealed(event.experiment, event.version)
        .await?
        .is_none()
    {
        return Err(Refusal::NotFound);
    }
    let records = judge.trials(event.experiment, event.version).await?;
    let kept = records.iter().any(|record| {
        record.trial.id == event.trial && record.outcome() == antilog_types::judge::Outcome::Kept
    });
    if !kept {
        return Err(Refusal::Invalid("only a Kept trial's pull request counts"));
    }
    if (event.kind == WinEventKind::Regressed) != event.value.is_some()
        || event.value.is_some_and(|value| !value.is_finite())
    {
        return Err(Refusal::Invalid("only a regression carries the value read"));
    }
    let history = judge.win_events(event.experiment).await?;
    let now = replay(&history, event.trial, i64::MAX);
    let merged = now.delivery == Delivery::Merged;
    let allowed = match event.kind {
        WinEventKind::Opened => now.delivery == Delivery::NotOpened,
        WinEventKind::Merged => now.delivery == Delivery::Open,
        WinEventKind::Closed => now.delivery == Delivery::Open,
        WinEventKind::Regressed => merged,
        WinEventKind::Recovered => merged && now.regressed_to.is_some(),
    };
    if !allowed {
        return Err(Refusal::Invalid(
            "that does not follow what is known of this pull request",
        ));
    }
    judge.put_win_event(&event).await?;
    store
        .append_audit(&antilog_types::AuditEntry::new(
            organization,
            None,
            "win.record",
            format!("{}:{:?}", event.trial, event.kind),
            AuditOutcome::Done,
            event.at,
        ))
        .await?;
    Ok(())
}

// ---------------------------------------------------------------- the report

/// `YYYY-MM` to the UTC month's bounds.
pub fn month_range(month: &str) -> Result<Range, Refusal> {
    let invalid = || Refusal::Invalid("a month is written YYYY-MM");
    let (year, number) = month.split_once('-').ok_or_else(invalid)?;
    if year.len() != 4 || number.len() != 2 {
        return Err(invalid());
    }
    let year: i16 = year.parse().map_err(|_| invalid())?;
    let number: i8 = number.parse().map_err(|_| invalid())?;
    let first = jiff::civil::Date::new(year, number, 1)
        .map_err(|_| invalid())?
        .to_zoned(jiff::tz::TimeZone::UTC)
        .map_err(|_| invalid())?
        .timestamp()
        .as_millisecond();
    let (from, to) = month_bounds(first).ok_or_else(invalid)?;
    Ok(Range { from, to })
}

/// The month of an instant, written `YYYY-MM`.
pub fn month_of(at: i64) -> Option<String> {
    let date = jiff::Timestamp::from_millisecond(at)
        .ok()?
        .to_zoned(jiff::tz::TimeZone::UTC)
        .date();
    Some(format!("{:04}-{:02}", date.year(), date.month()))
}

fn total(lines: &[SpendLine], keep: impl Fn(&SpendLine) -> bool) -> i64 {
    lines
        .iter()
        .filter(|line| keep(line))
        .map(|line| line.cents)
        .sum()
}

/// The month's report for the organization or one project: spent, Delivered, Pending, At risk and
/// Return, where the spend went, the agent scoreboard, the top wins and the experiments stopped
/// with no win. The organization's is read by owners and editors, a project's by every member; it carries numbers, titles and ids, never a diff or data.
/// A trial has no time of its own, so the scoreboard counts the trials of the sessions that spent
/// in the month.
pub async fn report<S: Store, J: JudgeStore>(
    store: &S,
    judge: &J,
    who: &Principal,
    now: i64,
    month: &str,
    project: Option<ProjectId>,
) -> Result<Report, Refusal> {
    member_of(store, who).await?;
    if project.is_none() {
        permit(store, who, Action::ReportRead, None, month, now).await?;
    }
    let range = month_range(month)?;
    if let Some(project) = project {
        known_project(store, who, project).await?;
    }
    let experiments = experiments_in(store, who.organization, project).await?;
    let in_scope = |id: ExperimentId| experiments.iter().any(|experiment| experiment.id == id);
    let lines: Vec<SpendLine> = store
        .spend_lines_between(who.organization, range.from, range.to)
        .await?
        .into_iter()
        .filter(|line| in_scope(line.experiment))
        .collect();

    let mut scans = Vec::new();
    for experiment in &experiments {
        scans.push((experiment, scan(judge, experiment, range.to).await?));
    }
    let counted: Vec<&Found> = scans
        .iter()
        .flat_map(|(_, scan)| scan.wins.iter())
        .filter(|win| win.counts_in(range))
        .collect();
    let spent = total(&lines, |_| true);
    let figures = to_figures(&counted, spent)?;

    let mut kinds: Vec<KindSpendLine> = Vec::new();
    for line in &lines {
        match kinds.iter_mut().find(|kind| kind.kind == line.kind) {
            Some(kind) => kind.cents += line.cents,
            None => kinds.push(KindSpendLine {
                kind: line.kind,
                cents: line.cents,
            }),
        }
    }
    kinds.sort_by_key(|kind| kind.kind.as_str());

    let (mut with_win, mut no_win, mut not_sealed) = (0, 0, 0);
    let mut stopped = Vec::new();
    for (experiment, scan) in &scans {
        let spent_here = total(&lines, |line| line.experiment == experiment.id);
        let sessions = store.sessions(who.organization, experiment.id).await?;
        if !scan.wins.is_empty() {
            with_win += spent_here;
        } else if experiment.status == antilog_types::ExperimentStatus::Sealed {
            no_win += spent_here;
            let ended = !sessions.is_empty() && sessions.iter().all(|s| !s.status.is_active());
            if ended && spent_here > 0 {
                let all_time = store
                    .spend(
                        who.organization,
                        SpendScope::Experiment(experiment.id),
                        0,
                        range.to,
                    )
                    .await?
                    .cents;
                stopped.push(StoppedExperiment {
                    experiment_id: experiment.id,
                    project_id: experiment.project_id,
                    goal: experiment.goal.clone(),
                    trials: u32::try_from(scan.trials.len()).unwrap_or(u32::MAX),
                    spent_cents: spent_here,
                    budget_spent: all_time >= experiment.budget_cap_cents,
                });
            }
        } else {
            not_sealed += spent_here;
        }
    }
    stopped.sort_by_key(|row| std::cmp::Reverse(row.spent_cents));

    // The scoreboard: one row per agent, from the sessions that spent in the month.
    let mut agents: std::collections::BTreeMap<String, AgentRow> =
        std::collections::BTreeMap::new();
    let mut agent_of = std::collections::HashMap::new();
    for (experiment, _) in &scans {
        for session in store.sessions(who.organization, experiment.id).await? {
            agent_of.insert(session.id, session.agent);
        }
    }
    let row = |agents: &'_ mut std::collections::BTreeMap<String, AgentRow>, name: &str| {
        agents.entry(name.to_owned()).or_insert_with(|| AgentRow {
            agent: name.to_owned(),
            trials: 0,
            kept: 0,
            wins_delivered: 0,
            wins_pending: 0,
            pending_cents: 0,
            spent_cents: 0,
            delivered_cents: 0,
        });
    };
    let mut active = std::collections::HashSet::new();
    for line in &lines {
        if let Some(name) = agent_of.get(&line.session) {
            active.insert(line.session);
            row(&mut agents, name);
            if let Some(agent) = agents.get_mut(name) {
                agent.spent_cents += line.cents;
            }
        }
    }
    for (_, scan) in &scans {
        for record in &scan.trials {
            if !active.contains(&record.trial.session) {
                continue;
            }
            let Some(name) = agent_of.get(&record.trial.session) else {
                continue;
            };
            row(&mut agents, name);
            if let Some(agent) = agents.get_mut(name) {
                agent.trials += 1;
                if record.outcome() == antilog_types::judge::Outcome::Kept {
                    agent.kept += 1;
                }
            }
        }
    }
    for win in &counted {
        let Some(name) = agent_of.get(&win.session) else {
            continue;
        };
        row(&mut agents, name);
        let view = win.view();
        if let Some(agent) = agents.get_mut(name) {
            match view.state {
                WinState::Delivered | WinState::AtRisk => {
                    agent.wins_delivered += 1;
                    agent.delivered_cents += view.delivered_cents.unwrap_or(0);
                }
                WinState::Pending => {
                    agent.wins_pending += 1;
                    agent.pending_cents += view.yearly_cents.unwrap_or(0);
                }
                WinState::NotOpened => {}
            }
        }
    }
    let agents: Vec<AgentRow> = agents.into_values().collect();

    let mut top: Vec<TopWin> = counted
        .iter()
        .filter_map(|win| {
            let experiment = experiments.iter().find(|e| e.id == win.experiment)?;
            Some(TopWin {
                win: win.view(),
                project_id: experiment.project_id,
                goal: experiment.goal.clone(),
                agent: agent_of.get(&win.session).cloned(),
                session_id: Some(win.session),
                spent_cents: total(&lines, |line| line.experiment == win.experiment),
            })
        })
        .collect();
    top.sort_by_key(|win| std::cmp::Reverse(win.win.yearly_cents.unwrap_or(0)));
    top.truncate(10);

    Ok(Report {
        month: month.to_owned(),
        from: range.from,
        to: range.to,
        project_id: project,
        figures,
        spend_by_kind: kinds,
        spent_with_win_cents: with_win,
        spent_no_win_cents: no_win,
        spent_not_sealed_cents: not_sealed,
        agents,
        top_wins: top,
        stopped,
    })
}
