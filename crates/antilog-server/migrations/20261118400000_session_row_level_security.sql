-- Owner: session. An experiment's Mode, and row-level security on the session area's tables,
-- scoped by the same setting as the organization floor's.
set local lock_timeout = '5s';

alter table experiments add column follow_policy boolean not null default false;

alter table experiments enable row level security;
create policy experiments_scope on experiments
    using (organization_id = current_organization())
    with check (organization_id = current_organization());

alter table sessions enable row level security;
create policy sessions_scope on sessions
    using (organization_id = current_organization())
    with check (organization_id = current_organization());

alter table spend enable row level security;
create policy spend_scope on spend
    using (organization_id = current_organization())
    with check (organization_id = current_organization());

alter table session_relay enable row level security;
create policy session_relay_scope on session_relay
    using (organization_id = current_organization())
    with check (organization_id = current_organization());

alter table seal_proposals enable row level security;
create policy seal_proposals_scope on seal_proposals
    using (organization_id = current_organization())
    with check (organization_id = current_organization());

alter table approvals_told enable row level security;
create policy approvals_told_scope on approvals_told
    using (organization_id = current_organization())
    with check (organization_id = current_organization());
