-- Owner: judge. Sealed versions, trials, receipts and value models. Documents are the
-- judge's own JSON, checked again when read; a seal, a receipt and a value model never change.
set local lock_timeout = '5s';

create table sealed_experiments (
    organization_id uuid not null,
    experiment_id uuid not null,
    version integer not null check (version >= 1),
    doc text not null check (length(doc) <= 1000000),
    primary key (experiment_id, version),
    unique (organization_id, experiment_id, version),
    foreign key (organization_id, experiment_id) references experiments (organization_id, id)
);

create table trials (
    seq bigint generated always as identity unique,
    id uuid primary key,
    organization_id uuid not null,
    experiment_id uuid not null,
    version integer not null,
    doc text not null check (length(doc) <= 1000000),
    finished text check (length(finished) <= 1000000),
    unique (organization_id, id),
    foreign key (organization_id, experiment_id, version)
        references sealed_experiments (organization_id, experiment_id, version)
);
create index trials_version on trials (organization_id, experiment_id, version, seq);

create table receipts (
    trial_id uuid primary key,
    organization_id uuid not null,
    doc text not null check (length(doc) <= 1000000),
    foreign key (organization_id, trial_id) references trials (organization_id, id)
);

create table value_models (
    id uuid not null,
    version integer not null check (version >= 1),
    organization_id uuid not null,
    doc text not null check (length(doc) <= 100000),
    primary key (id, version)
);

create trigger sealed_experiments_immutable before update or delete or truncate on sealed_experiments
    for each statement execute function refuse_change();
create trigger receipts_immutable before update or delete or truncate on receipts
    for each statement execute function refuse_change();
create trigger value_models_immutable before update or delete or truncate on value_models
    for each statement execute function refuse_change();

alter table sealed_experiments enable row level security;
create policy sealed_experiments_scope on sealed_experiments
    using (organization_id = current_organization())
    with check (organization_id = current_organization());

alter table trials enable row level security;
create policy trials_scope on trials
    using (organization_id = current_organization())
    with check (organization_id = current_organization());

alter table receipts enable row level security;
create policy receipts_scope on receipts
    using (organization_id = current_organization())
    with check (organization_id = current_organization());

alter table value_models enable row level security;
create policy value_models_scope on value_models
    using (organization_id = current_organization())
    with check (organization_id = current_organization());
