-- Owner: session (the runner's queue). A session's machine: queued, claimed by a runner for a
-- lease, running, idle, released, done or failed (docs/V0-contracts.md). A runner serves every
-- organization, so these tables admit `antilog.runner = 'on'` besides the organization's own
-- setting. Also the ask-first card, which carries a permission the agent asked for.
set local lock_timeout = '5s';

create function runner_access() returns boolean language sql stable as $$
    select coalesce(current_setting('antilog.runner', true), '') = 'on'
$$;

create table machines (
    session_id uuid primary key,
    organization_id uuid not null,
    experiment_id uuid not null,
    agent text not null check (length(agent) between 1 and 200),
    state text not null constraint machines_state
        check (state in ('queued', 'claimed', 'running', 'idle', 'released', 'done', 'failed')),
    holder text check (length(holder) <= 200),
    lease_until timestamptz,
    seed text not null check (length(seed) <= 100000),
    resume boolean not null default false,
    updated_at timestamptz not null,
    foreign key (organization_id, session_id) references sessions (organization_id, id),
    check ((state in ('claimed', 'running', 'idle')) = (holder is not null))
);
create index machines_queue on machines (state, lease_until);
create index machines_holder on machines (holder) where holder is not null;

create table machine_commands (
    id uuid primary key,
    seq bigint generated always as identity,
    organization_id uuid not null,
    session_id uuid not null,
    body text not null check (length(body) <= 1000000),
    delivered boolean not null default false,
    finished boolean not null default false,
    result text check (length(result) <= 10000000),
    created_at timestamptz not null,
    foreign key (organization_id, session_id) references sessions (organization_id, id)
);
create index machine_commands_session on machine_commands (session_id, seq);

create table machine_exports (
    session_id uuid primary key,
    organization_id uuid not null,
    body text not null check (length(body) <= 100000000),
    at timestamptz not null,
    foreign key (organization_id, session_id) references sessions (organization_id, id)
);

alter table machines enable row level security;
create policy machines_scope on machines
    using (organization_id = current_organization() or runner_access())
    with check (organization_id = current_organization() or runner_access());

alter table machine_commands enable row level security;
create policy machine_commands_scope on machine_commands
    using (organization_id = current_organization() or runner_access())
    with check (organization_id = current_organization() or runner_access());

alter table machine_exports enable row level security;
create policy machine_exports_scope on machine_exports
    using (organization_id = current_organization() or runner_access())
    with check (organization_id = current_organization() or runner_access());

alter table policies drop constraint policies_card;
alter table policies add constraint policies_card check (card in (
    'seal_experiment', 'raise_budget', 'add_credential', 'attach_repo',
    'change_environment', 'open_pull_request', 'keep_going', 'continue_on_another_agent',
    'ask_first'));
alter table approvals drop constraint approvals_card;
alter table approvals add constraint approvals_card check (card in (
    'seal_experiment', 'raise_budget', 'add_credential', 'attach_repo',
    'change_environment', 'open_pull_request', 'keep_going', 'continue_on_another_agent',
    'ask_first'));

-- A claim crosses organizations and reads each session's status, which the runner's setting does
-- not open, so it runs as the table owner. Lapsed leases go back to the queue first, and their
-- commands that were not finished are sent again.
create function claim_machines(p_holder text, p_limit integer, p_now timestamptz, p_until timestamptz)
returns setof machines language plpgsql security definer set search_path = public as $$
begin
    update machine_commands c set delivered = false
        from machines m
        where m.session_id = c.session_id and m.state in ('claimed', 'running', 'idle')
            and m.lease_until < p_now and not c.finished;
    update machines set state = 'queued', holder = null, updated_at = p_now
        where state in ('claimed', 'running', 'idle') and lease_until < p_now;
    return query
        update machines m set state = 'claimed', holder = p_holder, lease_until = p_until, updated_at = p_now
        where m.session_id in (
            select q.session_id from machines q join sessions s on s.id = q.session_id
            where q.state = 'queued' and s.status in ('working', 'waiting_for_you')
            order by q.updated_at, q.session_id limit p_limit for update of q skip locked)
        returning m.*;
end $$;
