-- Owning area: the judge. A value model belongs to a project and a metric: one id per metric,
-- a new version on every change, never edited. The win's delivery and regression facts are
-- append-only rows; Delivered, Pending, At risk and Return are derived from them.

alter table value_models
    add column project_id uuid not null,
    add column metric text not null check (length(metric) between 1 and 200),
    add column set_at bigint not null,
    add constraint value_models_project foreign key (organization_id, project_id)
        references projects (organization_id, id),
    add constraint value_models_metric_version unique (organization_id, project_id, metric, version);

create index value_models_project_metric on value_models (organization_id, project_id, metric, version desc);

create table win_events (
    id uuid primary key,
    organization_id uuid not null,
    experiment_id uuid not null,
    version integer not null check (version >= 1),
    trial_id uuid not null,
    kind text not null constraint win_events_kind
        check (kind in ('opened', 'merged', 'closed', 'regressed', 'recovered')),
    -- The metric value the watch read; only a regression carries one.
    value double precision check (value is null or value = value),
    link text check (length(link) <= 2000),
    at bigint not null,
    seq bigint generated always as identity,
    constraint win_events_regressed_value check ((kind = 'regressed') = (value is not null)),
    foreign key (organization_id, trial_id) references trials (organization_id, id)
);
create index win_events_experiment on win_events (organization_id, experiment_id, seq);

create trigger win_events_immutable before update or delete or truncate on win_events
    for each statement execute function refuse_change();

alter table win_events enable row level security;
create policy win_events_scope on win_events
    using (organization_id = current_organization())
    with check (organization_id = current_organization());
