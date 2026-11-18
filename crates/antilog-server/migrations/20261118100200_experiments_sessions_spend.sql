-- Owner: session (experiments' rows, sessions, spend) and organization (caps).
-- Spend is append-only. Caps nest: experiment budget <= project cap <= organization cap.
set local lock_timeout = '5s';

create function refuse_change() returns trigger language plpgsql as $$
begin
    raise exception '% is append-only', tg_table_name using errcode = 'AL005';
end $$;

create table experiments (
    id uuid primary key,
    organization_id uuid not null,
    project_id uuid not null,
    goal text not null check (length(goal) between 1 and 4000),
    status text not null constraint experiments_status check (status in ('not_sealed', 'sealed')),
    budget_cap_cents bigint not null check (budget_cap_cents >= 0),
    created_by uuid not null,
    created_at timestamptz not null,
    unique (organization_id, id),
    foreign key (organization_id, project_id) references projects (organization_id, id),
    foreign key (organization_id, created_by) references members (organization_id, id)
);
create index experiments_project on experiments (organization_id, project_id, created_at, id);

create table sessions (
    id uuid primary key,
    organization_id uuid not null,
    experiment_id uuid not null,
    parent_session_id uuid,
    agent text not null check (length(agent) between 1 and 200),
    platform text not null check (length(platform) between 1 and 200),
    platform_handle text,
    status text not null constraint sessions_status
        check (status in ('working', 'waiting_for_you', 'done', 'failed')),
    note text,
    cleanup text constraint sessions_cleanup check (cleanup in ('removed', 'not_removed')),
    created_at timestamptz not null,
    updated_at timestamptz not null,
    unique (organization_id, id),
    unique (organization_id, experiment_id, id),
    foreign key (organization_id, experiment_id) references experiments (organization_id, id),
    foreign key (organization_id, experiment_id, parent_session_id)
        references sessions (organization_id, experiment_id, id)
);
create index sessions_experiment on sessions (organization_id, experiment_id, created_at, id);

create table spend (
    id uuid primary key,
    organization_id uuid not null,
    session_id uuid not null,
    kind text not null constraint spend_kind check (kind in ('tokens', 'sandbox', 'judge')),
    amount_cents bigint not null check (amount_cents >= 0),
    tokens bigint not null check (tokens >= 0),
    occurred_at timestamptz not null,
    foreign key (organization_id, session_id) references sessions (organization_id, id)
);
create index spend_session on spend (organization_id, session_id, occurred_at);
create index spend_time on spend (organization_id, occurred_at);

create trigger spend_append_only before update or delete or truncate on spend
    for each statement execute function refuse_change();

create function experiments_guard() returns trigger language plpgsql as $$
declare ceiling bigint;
begin
    if tg_op = 'UPDATE' and old.status = 'sealed' and new.status <> 'sealed' then
        raise exception 'a sealed experiment stays sealed' using errcode = 'AL003';
    end if;
    select least(p.monthly_cap_cents, o.monthly_cap_cents) into ceiling
        from projects p join organizations o on o.id = p.organization_id
        where p.id = new.project_id and p.organization_id = new.organization_id;
    if new.budget_cap_cents > ceiling then
        raise exception 'the budget passes a cap above it' using errcode = 'AL002';
    end if;
    return new;
end $$;

create trigger experiments_guard before insert or update of budget_cap_cents, status on experiments
    for each row execute function experiments_guard();

create function projects_limits_guard() returns trigger language plpgsql as $$
declare ceiling bigint;
begin
    select monthly_cap_cents into ceiling from organizations where id = new.organization_id;
    if new.monthly_cap_cents > ceiling then
        raise exception 'the cap passes the organization cap' using errcode = 'AL002';
    end if;
    ceiling := least(new.monthly_cap_cents, ceiling);
    if new.default_budget_cents > ceiling
       or exists (select 1 from experiments e
                  where e.project_id = new.id and e.budget_cap_cents > ceiling) then
        raise exception 'a budget passes the cap' using errcode = 'AL002';
    end if;
    return new;
end $$;

create trigger projects_limits_guard
    before insert or update of monthly_cap_cents, default_budget_cents on projects
    for each row execute function projects_limits_guard();

create function organizations_limits_guard() returns trigger language plpgsql as $$
begin
    if exists (select 1 from projects p
               where p.organization_id = new.id
                 and (p.monthly_cap_cents > new.monthly_cap_cents
                      or p.default_budget_cents > new.monthly_cap_cents))
       or exists (select 1 from experiments e
                  where e.organization_id = new.id and e.budget_cap_cents > new.monthly_cap_cents) then
        raise exception 'the cap is below a cap or budget inside it' using errcode = 'AL002';
    end if;
    return new;
end $$;

create trigger organizations_limits_guard
    before update of monthly_cap_cents on organizations
    for each row execute function organizations_limits_guard();

create function sessions_guard() returns trigger language plpgsql as $$
begin
    if tg_op = 'INSERT' then
        if exists (select 1 from sessions s
                   where s.id = new.parent_session_id and s.parent_session_id is not null) then
            raise exception 'a spawned session cannot spawn' using errcode = 'AL004';
        end if;
    elsif old.status in ('done', 'failed') and new.status <> old.status then
        raise exception 'a % session stays so', old.status using errcode = 'AL003';
    end if;
    return new;
end $$;

create trigger sessions_guard before insert or update of status on sessions
    for each row execute function sessions_guard();
