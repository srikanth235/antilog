-- Owner: judge. What turns a measured win into delivered value: the pull requests opened for a
-- win and what the code host said of them, the card's typed fields, and the trial a spend row paid for.
set local lock_timeout = '5s';

alter table approvals add column payload text check (length(payload) <= 100000);

-- A spend row may name the trial it paid for. Spend stays append-only: this adds a column only.
alter table spend add column trial_id uuid;
alter table spend add constraint spend_trial foreign key (organization_id, trial_id)
    references trials (organization_id, id);
create index spend_trial_idx on spend (organization_id, trial_id) where trial_id is not null;

-- One row per pull request Antilog opened. The state is the code host's last word, kept so a
-- linked set of pull requests counts as merged only when every one is.
create table pull_requests (
    organization_id uuid not null,
    repo text not null check (length(repo) between 1 and 300),
    number bigint not null check (number >= 1),
    experiment_id uuid not null,
    version integer not null check (version >= 1),
    trial_id uuid not null,
    url text not null check (length(url) <= 2000),
    state text not null constraint pull_requests_state check (state in ('open', 'merged', 'closed')),
    primary key (organization_id, repo, number),
    foreign key (organization_id, trial_id) references trials (organization_id, id)
);
create index pull_requests_trial on pull_requests (organization_id, trial_id);

-- Deliveries the code host sent, so a redelivery changes nothing.
create table code_host_deliveries (
    organization_id uuid not null,
    delivery_id text not null check (length(delivery_id) between 1 and 200),
    at bigint not null,
    primary key (organization_id, delivery_id)
);

alter table pull_requests enable row level security;
create policy pull_requests_scope on pull_requests
    using (organization_id = current_organization())
    with check (organization_id = current_organization());

alter table code_host_deliveries enable row level security;
create policy code_host_deliveries_scope on code_host_deliveries
    using (organization_id = current_organization())
    with check (organization_id = current_organization());

-- A webhook names a repo and a number and no organization. This finds the one organization that
-- opened that pull request, and nothing else; a pull request two organizations share is not found.
create function pull_request_organization(wanted_repo text, wanted_number bigint)
returns uuid language sql stable security definer set search_path = public as $$
    select case when count(*) = 1 then (array_agg(organization_id))[1] end
    from pull_requests where repo = wanted_repo and number = wanted_number
$$;
