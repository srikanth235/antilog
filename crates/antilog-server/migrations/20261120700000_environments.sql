-- Owner: session. The environments an organization manages: the machine a session runs on
-- (docs/V0.md). Every change is a new row in `environment_versions`; a session's seed names the
-- version it started on, so the versions are never changed or removed. Archiving hides an
-- environment from new sessions and nothing else.
set local lock_timeout = '5s';

create table environments (
    organization_id uuid not null references organizations (id),
    id text not null check (id ~ '^[a-z0-9][a-z0-9-]{0,63}$' and id <> 'default'),
    archived boolean not null default false,
    created_at timestamptz not null,
    primary key (organization_id, id)
);

create table environment_versions (
    organization_id uuid not null,
    id text not null,
    version integer not null check (version >= 1),
    name text not null check (length(name) between 1 and 200),
    network text not null constraint environment_versions_network check (network in ('none', 'limited', 'full')),
    -- JSON as text: a list of host names, and an object of variables.
    allowed_hosts text not null check (length(allowed_hosts) <= 40000),
    allow_package_managers boolean not null,
    variables text not null check (length(variables) <= 1000000),
    setup_script text not null check (octet_length(setup_script) <= 65536),
    cpus integer not null check (cpus between 1 and 16),
    memory_mb integer not null check (memory_mb between 1024 and 65536),
    created_by uuid,
    created_at timestamptz not null,
    primary key (organization_id, id, version),
    foreign key (organization_id, id) references environments (organization_id, id)
);

alter table environments enable row level security;
create policy environments_scope on environments
    using (organization_id = current_organization())
    with check (organization_id = current_organization());

alter table environment_versions enable row level security;
create policy environment_versions_scope on environment_versions
    using (organization_id = current_organization())
    with check (organization_id = current_organization());

-- A version is history: it is written once and never changed or removed.
create trigger environment_versions_append_only before update or delete or truncate on environment_versions
    for each statement execute function refuse_change();
