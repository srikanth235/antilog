-- Owner: session. The agents an organization turned on, and which one is the default.
-- Auto picks among the enabled ones; an owner sets the rest.
set local lock_timeout = '5s';

create table agents (
    organization_id uuid not null references organizations (id),
    id text not null check (id ~ '^[a-z0-9][a-z0-9-]{0,63}$'),
    name text not null check (length(name) between 1 and 200),
    platform text not null check (length(platform) between 1 and 64),
    enabled boolean not null,
    is_default boolean not null,
    cost_tier text not null constraint agents_cost_tier check (cost_tier in ('low', 'medium', 'high')),
    created_at timestamptz not null,
    primary key (organization_id, id),
    check (enabled or not is_default)
);
create unique index agents_one_default on agents (organization_id) where is_default;

alter table agents enable row level security;
create policy agents_scope on agents
    using (organization_id = current_organization())
    with check (organization_id = current_organization());
