-- Owner: organization. Policy, approvals and the append-only audit log.
set local lock_timeout = '5s';

create table policies (
    organization_id uuid not null references organizations (id),
    card text not null constraint policies_card check (card in (
        'seal_experiment', 'raise_budget', 'add_credential', 'attach_repo',
        'change_environment', 'open_pull_request', 'keep_going', 'continue_on_another_agent')),
    mode text not null constraint policies_mode check (mode in ('always_ask', 'allow_up_to', 'allow')),
    limit_cents bigint check (limit_cents >= 0),
    primary key (organization_id, card),
    constraint policies_limit check ((mode = 'allow_up_to') = (limit_cents is not null)),
    constraint policies_add_credential_asks check (card <> 'add_credential' or mode = 'always_ask')
);

create table approvals (
    id uuid primary key,
    organization_id uuid not null,
    experiment_id uuid not null,
    session_id uuid,
    card text not null constraint approvals_card check (card in (
        'seal_experiment', 'raise_budget', 'add_credential', 'attach_repo',
        'change_environment', 'open_pull_request', 'keep_going', 'continue_on_another_agent')),
    summary text not null check (length(summary) <= 2000),
    cost_cents bigint check (cost_cents >= 0),
    status text not null constraint approvals_status
        check (status in ('pending', 'approved', 'declined', 'approved_by_policy')),
    decided_by uuid,
    created_at timestamptz not null,
    decided_at timestamptz,
    foreign key (organization_id, experiment_id) references experiments (organization_id, id),
    foreign key (organization_id, session_id) references sessions (organization_id, id),
    foreign key (organization_id, decided_by) references members (organization_id, id),
    check ((status = 'pending') = (decided_at is null)),
    check ((status in ('approved', 'declined')) = (decided_by is not null)),
    constraint approvals_credential_asks check (card <> 'add_credential' or status <> 'approved_by_policy')
);
create index approvals_experiment on approvals (organization_id, experiment_id, created_at, id);

create function approvals_guard() returns trigger language plpgsql as $$
begin
    if old.status <> 'pending' then
        raise exception 'an approval is decided once' using errcode = 'AL003';
    end if;
    return new;
end $$;

create trigger approvals_guard before update on approvals
    for each row execute function approvals_guard();

create table audit_log (
    id uuid primary key,
    organization_id uuid not null references organizations (id),
    actor_id uuid,
    action text not null check (length(action) between 1 and 200),
    target text not null check (length(target) <= 200),
    outcome text not null constraint audit_outcome
        check (outcome in ('done', 'denied', 'approved', 'declined', 'approved_by_policy')),
    at timestamptz not null,
    foreign key (organization_id, actor_id) references members (organization_id, id)
);
create index audit_log_page on audit_log (organization_id, at desc, id desc);

create trigger audit_log_append_only before update or delete or truncate on audit_log
    for each statement execute function refuse_change();
