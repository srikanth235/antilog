-- Owner: session. v0 is cloud sessions only (docs/V0.md, "v0 is cloud sessions only"). Experiments
-- and the judge's tables (seals, trials, receipts, value models, wins, pull requests, evaluation
-- templates) are deleted, not hidden; the last commit that has them is `daa445f`. Projects stay:
-- a session belongs to one project, with its creator, title, repos, environment, model and its
-- own optional budget. There is no data to keep: v0 has no customers, so the
-- session area's rows are cleared. Existing migrations stay as they were.
set local lock_timeout = '5s';

-- The judge's tables, children first.
alter table spend drop column trial_id;
drop table win_events;
drop table pull_requests;
drop table code_host_deliveries;
drop table receipts;
drop table trials;
drop table sealed_experiments;
drop table value_models;
drop table evaluation_templates;
drop table seal_proposals;

-- What each experiment chose from Customize, and a project's switch for a skill: every session
-- now gets every connector and the latest version of every skill.
drop table experiment_connectors;
drop table experiment_skills;
drop table project_skills_off;

-- Clear the session area. Spend and the conversation are append-only; the guard is lifted for
-- this one clearing.
alter table spend disable trigger spend_append_only;
alter table session_events disable trigger session_events_append_only;
truncate table sessions cascade;
alter table spend enable trigger spend_append_only;
alter table session_events enable trigger session_events_append_only;

-- The experiment guards go; the project and organization guards now watch sessions.
drop trigger experiments_guard on experiments;
drop function experiments_guard();
drop trigger projects_limits_guard on projects;
drop function projects_limits_guard();
drop trigger organizations_limits_guard on organizations;
drop function organizations_limits_guard();

-- Sessions move from their experiment to their project.
drop table experiments cascade;

alter table sessions
    drop column experiment_id,
    add column project_id uuid not null,
    add column title text not null check (length(title) between 1 and 200),
    add column created_by uuid not null,
    add column model text not null check (length(model) <= 200),
    add column repos text[] not null default '{}',
    add column environment text check (length(environment) <= 200),
    add column environment_version integer check (environment_version >= 1),
    add column budget_cap_cents bigint check (budget_cap_cents >= 0),
    add column follow_policy boolean not null default false,
    add constraint sessions_project foreign key (organization_id, project_id)
        references projects (organization_id, id),
    add constraint sessions_created_by foreign key (organization_id, created_by)
        references members (organization_id, id);
create index sessions_organization on sessions (organization_id, created_at desc, id desc);
create index sessions_project on sessions (organization_id, project_id, created_at desc, id desc);
create index sessions_creator on sessions (organization_id, created_by, created_at desc, id desc);

-- Caps nest: a session's budget <= its project's cap <= the organization's cap, and a cap cannot
-- be lowered under what sits inside it.
create function sessions_budget_guard() returns trigger language plpgsql
    security definer set search_path = public as $$
declare ceiling bigint;
begin
    select least(p.monthly_cap_cents, o.monthly_cap_cents) into ceiling
        from projects p join organizations o on o.id = p.organization_id
        where p.id = new.project_id and p.organization_id = new.organization_id;
    if new.budget_cap_cents > ceiling then
        raise exception 'the budget passes a cap above it' using errcode = 'AL002';
    end if;
    return new;
end $$;

create trigger sessions_budget_guard before insert or update of budget_cap_cents on sessions
    for each row execute function sessions_budget_guard();

create function projects_limits_guard() returns trigger language plpgsql
    security definer set search_path = public as $$
declare ceiling bigint;
begin
    select monthly_cap_cents into ceiling from organizations where id = new.organization_id;
    if new.monthly_cap_cents > ceiling then
        raise exception 'the cap passes the organization cap' using errcode = 'AL002';
    end if;
    ceiling := least(new.monthly_cap_cents, ceiling);
    if new.default_budget_cents > ceiling
       or exists (select 1 from sessions s
                  where s.project_id = new.id and s.budget_cap_cents > ceiling) then
        raise exception 'a budget passes the cap' using errcode = 'AL002';
    end if;
    return new;
end $$;

create trigger projects_limits_guard
    before insert or update of monthly_cap_cents, default_budget_cents on projects
    for each row execute function projects_limits_guard();

create function organizations_limits_guard() returns trigger language plpgsql
    security definer set search_path = public as $$
begin
    if exists (select 1 from projects p
               where p.organization_id = new.id
                 and (p.monthly_cap_cents > new.monthly_cap_cents
                      or p.default_budget_cents > new.monthly_cap_cents))
       or exists (select 1 from sessions s
                  where s.organization_id = new.id and s.budget_cap_cents > new.monthly_cap_cents) then
        raise exception 'the cap is below a cap or budget inside it' using errcode = 'AL002';
    end if;
    return new;
end $$;

create trigger organizations_limits_guard
    before update of monthly_cap_cents on organizations
    for each row execute function organizations_limits_guard();

-- The cards: the judge's are gone, and every card belongs to a session.
delete from policies where card in
    ('seal_experiment', 'open_pull_request', 'keep_going', 'continue_on_another_agent');
alter table policies drop constraint policies_card;
alter table policies add constraint policies_card check (card in (
    'raise_budget', 'add_credential', 'attach_repo', 'change_environment',
    'ask_first', 'use_connector_tool'));

alter table approvals drop constraint approvals_owner_asks;
drop index approvals_experiment;
alter table approvals
    drop column experiment_id,
    drop column needs_owner,
    alter column session_id set not null,
    drop constraint approvals_card,
    add constraint approvals_card check (card in (
        'raise_budget', 'add_credential', 'attach_repo', 'change_environment',
        'ask_first', 'use_connector_tool'));
create index approvals_session on approvals (organization_id, session_id, created_at, id);

-- Spend: machine time and tokens.
alter table spend drop constraint spend_kind;
alter table spend add constraint spend_kind check (kind in ('tokens', 'machine'));

-- The runner's queue: no experiment, and the judge's `Run` result is gone.
alter table machines drop column experiment_id;
alter table machine_commands drop column result;

-- The gateway holds a session token and needs the session's organization.
drop function gateway_scope(uuid);
create function gateway_scope(session uuid)
    returns table (organization_id uuid)
    language sql stable security definer set search_path = public as $$
    select organization_id from sessions where id = session
$$;
