-- Owner: session (reach). Connectors (remote MCP servers), organization skills with fixed
-- versions, plugins that bring them, and what each experiment chose. A connector's credential is
-- sealed (XChaCha20-Poly1305, bound to its row) and is never read back to a client.
set local lock_timeout = '5s';

alter table policies drop constraint policies_card;
alter table policies add constraint policies_card check (card in (
    'seal_experiment', 'raise_budget', 'add_credential', 'attach_repo',
    'change_environment', 'open_pull_request', 'keep_going', 'continue_on_another_agent',
    'use_connector_tool'));
alter table approvals drop constraint approvals_card;
alter table approvals add constraint approvals_card check (card in (
    'seal_experiment', 'raise_budget', 'add_credential', 'attach_repo',
    'change_environment', 'open_pull_request', 'keep_going', 'continue_on_another_agent',
    'use_connector_tool'));

create table plugins (
    organization_id uuid not null references organizations (id),
    id text not null check (id ~ '^[a-z0-9][a-z0-9-]{0,63}$'),
    name text not null check (length(name) between 1 and 100),
    description text not null check (length(description) <= 500),
    version text not null check (length(version) between 1 and 40),
    parts text not null check (length(parts) <= 100000),
    created_by uuid not null,
    created_at timestamptz not null,
    primary key (organization_id, id),
    foreign key (organization_id, created_by) references members (organization_id, id)
);

create table connectors (
    organization_id uuid not null references organizations (id),
    id uuid not null,
    name text not null check (length(name) between 1 and 100),
    slug text not null check (slug ~ '^[a-z0-9][a-z0-9-]{0,63}$'),
    url text not null check (length(url) between 1 and 2000),
    sealed bytea check (length(sealed) <= 10000),
    health text not null constraint connectors_health check (health in ('connected', 'reconnect', 'not_checked')),
    tools text not null check (length(tools) <= 1000000),
    plugin_id text,
    created_by uuid not null,
    created_at timestamptz not null,
    primary key (organization_id, id),
    unique (organization_id, slug),
    foreign key (organization_id, plugin_id) references plugins (organization_id, id),
    foreign key (organization_id, created_by) references members (organization_id, id)
);

create table skill_versions (
    organization_id uuid not null references organizations (id),
    name text not null check (name ~ '^[a-z0-9][a-z0-9-]{0,63}$'),
    version integer not null check (version >= 1),
    description text not null check (length(description) between 1 and 1024),
    files text not null check (length(files) <= 2000000),
    size_bytes bigint not null check (size_bytes >= 0),
    plugin_id text,
    created_by uuid not null,
    created_at timestamptz not null,
    primary key (organization_id, name, version),
    foreign key (organization_id, plugin_id) references plugins (organization_id, id),
    foreign key (organization_id, created_by) references members (organization_id, id)
);

create table project_skills_off (
    organization_id uuid not null,
    project_id uuid not null,
    name text not null,
    primary key (organization_id, project_id, name),
    foreign key (organization_id, project_id) references projects (organization_id, id)
);

create table experiment_connectors (
    organization_id uuid not null,
    experiment_id uuid not null,
    connector_id uuid not null,
    primary key (organization_id, experiment_id, connector_id),
    foreign key (organization_id, experiment_id) references experiments (organization_id, id),
    foreign key (organization_id, connector_id) references connectors (organization_id, id)
);

create table experiment_skills (
    organization_id uuid not null,
    experiment_id uuid not null,
    name text not null,
    version integer not null,
    primary key (organization_id, experiment_id, name),
    foreign key (organization_id, experiment_id) references experiments (organization_id, id),
    foreign key (organization_id, name, version) references skill_versions (organization_id, name, version)
);

-- An approved "ask first" card pays for one call: the row says it was spent.
create table connector_tool_uses (
    organization_id uuid not null,
    approval_id uuid not null,
    primary key (organization_id, approval_id),
    foreign key (approval_id) references approvals (id)
);

alter table plugins enable row level security;
create policy plugins_scope on plugins using (organization_id = current_organization()) with check (organization_id = current_organization());
alter table connectors enable row level security;
create policy connectors_scope on connectors using (organization_id = current_organization()) with check (organization_id = current_organization());
alter table skill_versions enable row level security;
create policy skill_versions_scope on skill_versions using (organization_id = current_organization()) with check (organization_id = current_organization());
alter table project_skills_off enable row level security;
create policy project_skills_off_scope on project_skills_off using (organization_id = current_organization()) with check (organization_id = current_organization());
alter table experiment_connectors enable row level security;
create policy experiment_connectors_scope on experiment_connectors using (organization_id = current_organization()) with check (organization_id = current_organization());
alter table experiment_skills enable row level security;
create policy experiment_skills_scope on experiment_skills using (organization_id = current_organization()) with check (organization_id = current_organization());
alter table connector_tool_uses enable row level security;
create policy connector_tool_uses_scope on connector_tool_uses using (organization_id = current_organization()) with check (organization_id = current_organization());

-- A session's token names only the session. A read-only window on the one session (and its
-- experiment) a transaction names with antilog.session_id, so its organization can be found.
create function current_session() returns uuid language sql stable as $$
    select nullif(current_setting('antilog.session_id', true), '')::uuid
$$;
create policy sessions_session_lookup on sessions for select using (id = current_session());
create policy experiments_session_lookup on experiments for select
    using (id in (select experiment_id from sessions));
