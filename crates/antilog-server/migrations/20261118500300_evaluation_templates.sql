-- Owner: judge (kept beside the session's Customize tables, written with one audit row).
-- A reusable starting point: an experiment starts from a copy, so editing or removing a template
-- never changes a sealed experiment. Append-only here: a template is added, never edited.
set local lock_timeout = '5s';

create table evaluation_templates (
    organization_id uuid not null references organizations (id),
    id text not null check (id ~ '^[a-z0-9][a-z0-9-]{0,63}$'),
    name text not null check (length(name) between 1 and 200),
    summary text not null check (length(summary) <= 500),
    kind text not null constraint evaluation_templates_kind check (kind in ('speed', 'quality', 'size', 'cost')),
    metric text not null check (length(metric) between 1 and 200),
    lower_is_better boolean not null,
    command text not null check (length(command) between 1 and 2000),
    protected_paths text not null check (length(protected_paths) <= 10000),
    created_by uuid not null,
    created_at timestamptz not null,
    primary key (organization_id, id),
    foreign key (organization_id, created_by) references members (organization_id, id)
);

create trigger evaluation_templates_append_only before update or delete or truncate on evaluation_templates
    for each statement execute function refuse_change();

alter table evaluation_templates enable row level security;
create policy evaluation_templates_scope on evaluation_templates
    using (organization_id = current_organization())
    with check (organization_id = current_organization());
