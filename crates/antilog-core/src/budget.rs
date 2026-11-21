//! Budgets and caps: what is left to spend, and the refusal when nothing is.

use antilog_types::{AuditOutcome, KindSpend, OrganizationId, ProjectId, SessionId, SessionSpend};
use jiff::Timestamp;

use crate::organization::{
    Action, Principal, Refusal, entry, member_of, permit, permit_in, project_of,
};
use crate::store::{SpendScope, Store};

/// Epoch milliseconds of the first instant of the UTC month `now` falls in, and of the next.
pub fn month_bounds(now: i64) -> Option<(i64, i64)> {
    let zoned = Timestamp::from_millisecond(now)
        .ok()?
        .to_zoned(jiff::tz::TimeZone::UTC);
    let first = zoned.date().first_of_month();
    let start = first.to_zoned(jiff::tz::TimeZone::UTC).ok()?;
    let end = first
        .checked_add(jiff::Span::new().months(1))
        .ok()?
        .to_zoned(jiff::tz::TimeZone::UTC)
        .ok()?;
    Some((
        start.timestamp().as_millisecond(),
        end.timestamp().as_millisecond(),
    ))
}

/// What is left under each limit, in US cents; `None` is no limit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Headroom {
    pub session: Option<i64>,
    pub project: Option<i64>,
    pub organization: Option<i64>,
}

impl Headroom {
    /// The least left under any limit; `None` when nothing limits the session.
    pub fn remaining(&self) -> Option<i64> {
        [self.session, self.project, self.organization]
            .into_iter()
            .flatten()
            .min()
    }

    pub fn spent(&self) -> bool {
        self.remaining().is_some_and(|left| left <= 0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum Spent {
    #[error("the session's budget is spent")]
    Session,
    #[error("the project's monthly cap is reached")]
    Project,
    #[error("the organization's monthly cap is reached")]
    Organization,
}

fn left(cap: Option<i64>, spent: i64) -> Option<i64> {
    cap.map(|cap| cap.saturating_sub(spent))
}

pub async fn headroom<S: Store>(
    store: &S,
    organization: OrganizationId,
    session: SessionId,
    now: i64,
) -> Result<Headroom, Refusal> {
    let (from, to) = month_bounds(now).ok_or(Refusal::Invalid("the time is out of range"))?;
    let row = store
        .session(organization, session)
        .await?
        .ok_or(Refusal::NotFound)?;
    let project = store
        .project(organization, row.project_id)
        .await?
        .ok_or(Refusal::NotFound)?;
    let owner = store
        .organization(organization)
        .await?
        .ok_or(Refusal::NotFound)?;
    let session_spent = store
        .spend(
            organization,
            SpendScope::Session(session),
            0,
            now.saturating_add(1),
        )
        .await?
        .cents;
    let project_spent = store
        .spend(organization, SpendScope::Project(project.id), from, to)
        .await?
        .cents;
    let organization_spent = store
        .spend(organization, SpendScope::Organization, from, to)
        .await?
        .cents;
    Ok(Headroom {
        session: left(row.budget_cap_cents, session_spent),
        project: left(project.monthly_cap_cents, project_spent),
        organization: left(owner.monthly_cap_cents, organization_spent),
    })
}

/// Whether the organization's and the project's monthly caps, where they have one, still leave
/// room for a new session.
pub async fn has_room<S: Store>(
    store: &S,
    organization: OrganizationId,
    project: ProjectId,
    now: i64,
) -> Result<bool, Refusal> {
    let (from, to) = month_bounds(now).ok_or(Refusal::Invalid("the time is out of range"))?;
    let row = store
        .project(organization, project)
        .await?
        .ok_or(Refusal::NotFound)?;
    let project_spent = store
        .spend(organization, SpendScope::Project(project), from, to)
        .await?
        .cents;
    if left(row.monthly_cap_cents, project_spent).is_some_and(|left| left <= 0) {
        return Ok(false);
    }
    let owner = store
        .organization(organization)
        .await?
        .ok_or(Refusal::NotFound)?;
    let spent = store
        .spend(organization, SpendScope::Organization, from, to)
        .await?
        .cents;
    Ok(left(owner.monthly_cap_cents, spent).is_none_or(|left| left > 0))
}

/// What the model gateway asks before it forwards a call, and a session asks before it starts:
/// refused when nothing is left under any limit.
pub async fn ensure_room<S: Store>(
    store: &S,
    organization: OrganizationId,
    session: SessionId,
    now: i64,
) -> Result<Headroom, Refusal> {
    let room = headroom(store, organization, session, now).await?;
    let blocked = if room.session.is_some_and(|left| left <= 0) {
        Some(Spent::Session)
    } else if room.project.is_some_and(|left| left <= 0) {
        Some(Spent::Project)
    } else if room.organization.is_some_and(|left| left <= 0) {
        Some(Spent::Organization)
    } else {
        None
    };
    match blocked {
        Some(spent) => Err(Refusal::Spent(spent)),
        None => Ok(room),
    }
}

fn cents(value: Option<i64>) -> Result<(), Refusal> {
    if value.is_some_and(|value| value < 0) {
        Err(Refusal::Invalid("an amount is not negative"))
    } else {
        Ok(())
    }
}

/// Only an organization owner changes the cap, the default budget and how many sessions may run at
/// once (`None`: leave that as it is).
pub async fn set_organization_limits<S: Store>(
    store: &S,
    who: &Principal,
    now: i64,
    monthly_cap_cents: Option<i64>,
    default_budget_cents: Option<i64>,
    max_running_sessions: Option<u32>,
) -> Result<(), Refusal> {
    cents(monthly_cap_cents)?;
    cents(default_budget_cents)?;
    if max_running_sessions == Some(0) {
        return Err(Refusal::Invalid(
            "an organization may run at least one session",
        ));
    }
    permit(
        store,
        who,
        Action::OrganizationLimitsSet,
        who.organization,
        now,
    )
    .await?;
    store
        .set_organization_limits(
            who.organization,
            monthly_cap_cents,
            default_budget_cents,
            max_running_sessions,
            &entry(
                who,
                "organization.set_limits",
                who.organization,
                AuditOutcome::Done,
                now,
            ),
        )
        .await?;
    Ok(())
}

/// A project owner changes the project's cap and default budget, inside the organization's cap.
pub async fn set_project_limits<S: Store>(
    store: &S,
    who: &Principal,
    now: i64,
    project: ProjectId,
    monthly_cap_cents: Option<i64>,
    default_budget_cents: Option<i64>,
) -> Result<(), Refusal> {
    cents(monthly_cap_cents)?;
    cents(default_budget_cents)?;
    permit_in(store, who, Action::ProjectLimitsSet, project, project, now).await?;
    store
        .set_project_limits(
            who.organization,
            project,
            monthly_cap_cents,
            default_budget_cents,
            &entry(who, "project.set_limits", project, AuditOutcome::Done, now),
        )
        .await?;
    Ok(())
}

/// An editor or owner of the session's project changes its own cap, inside the project's cap.
pub async fn set_session_budget<S: Store>(
    store: &S,
    who: &Principal,
    now: i64,
    session: SessionId,
    budget_cap_cents: i64,
) -> Result<(), Refusal> {
    cents(Some(budget_cap_cents))?;
    store
        .session(who.organization, session)
        .await?
        .ok_or(Refusal::NotFound)?;
    let project = project_of(store, who, session).await?;
    permit_in(store, who, Action::SessionBudgetSet, project, session, now).await?;
    store
        .set_session_budget(
            who.organization,
            session,
            budget_cap_cents,
            &entry(who, "session.set_budget", session, AuditOutcome::Done, now),
        )
        .await?;
    Ok(())
}

/// Spend this month against a limit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Scope {
    pub cap_cents: Option<i64>,
    pub spent_cents: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Summary {
    pub organization: Scope,
    pub projects: Vec<(ProjectId, Scope)>,
}

/// The month's spend against the organization's cap and each project's.
pub async fn summary<S: Store>(store: &S, who: &Principal, now: i64) -> Result<Summary, Refusal> {
    member_of(store, who).await?;
    let (from, to) = month_bounds(now).ok_or(Refusal::Invalid("the time is out of range"))?;
    let owner = store
        .organization(who.organization)
        .await?
        .ok_or(Refusal::NotFound)?;
    let organization = Scope {
        cap_cents: owner.monthly_cap_cents,
        spent_cents: store
            .spend(who.organization, SpendScope::Organization, from, to)
            .await?
            .cents,
    };
    let mut projects = Vec::new();
    for project in store.projects(who.organization).await? {
        let spent = store
            .spend(who.organization, SpendScope::Project(project.id), from, to)
            .await?;
        projects.push((
            project.id,
            Scope {
                cap_cents: project.monthly_cap_cents,
                spent_cents: spent.cents,
            },
        ));
    }
    Ok(Summary {
        organization,
        projects,
    })
}

/// One session's spend since it began against its own cap, by kind. Every member reads.
pub async fn session_spend<S: Store>(
    store: &S,
    who: &Principal,
    session: SessionId,
) -> Result<SessionSpend, Refusal> {
    member_of(store, who).await?;
    let row = store
        .session(who.organization, session)
        .await?
        .ok_or(Refusal::NotFound)?;
    let lines = store.spend_lines(who.organization, session).await?;
    let mut spend = SessionSpend {
        session_id: session,
        budget_cap_cents: row.budget_cap_cents,
        spent_cents: 0,
        tokens: 0,
        by_kind: Vec::new(),
    };
    for line in lines {
        spend.spent_cents += line.cents;
        spend.tokens += line.tokens;
        match spend.by_kind.iter_mut().find(|kind| kind.kind == line.kind) {
            Some(kind) => {
                kind.cents += line.cents;
                kind.tokens += line.tokens;
            }
            None => spend.by_kind.push(KindSpend {
                kind: line.kind,
                cents: line.cents,
                tokens: line.tokens,
            }),
        }
    }
    spend.by_kind.sort_by_key(|kind| kind.kind.as_str());
    Ok(spend)
}
