-- Owner: organization. An organization keeps an owner, sign-in sessions, owner-only
-- approvals and row-level security on the organization's own tables.
-- The server connects as a role that does not own the tables; it sets
-- antilog.organization_id for each transaction. Guards that look across rows are
-- security definer, so they see every row of the organization they are checking.
set local lock_timeout = '5s';

alter table approvals
    add column needs_owner boolean not null default false,
    add constraint approvals_owner_asks check (not needs_owner or status <> 'approved_by_policy');

create table sign_in_sessions (
    id uuid primary key,
    organization_id uuid not null,
    member_id uuid not null,
    hash bytea not null unique check (length(hash) = 32),
    created_at timestamptz not null,
    expires_at timestamptz not null,
    ended_at timestamptz,
    foreign key (organization_id, member_id) references members (organization_id, id)
);
create index sign_in_sessions_member on sign_in_sessions (organization_id, member_id);

create function assert_organization_owner(org uuid) returns void
    language plpgsql security definer set search_path = public as $$
begin
    perform 1 from organizations where id = org for update;
    if exists (select 1 from members where organization_id = org)
       and not exists (select 1 from members where organization_id = org and role = 'owner') then
        raise exception 'an organization needs at least one owner' using errcode = 'AL001';
    end if;
end $$;

create function members_keep_organization_owner() returns trigger language plpgsql as $$
begin
    perform assert_organization_owner(coalesce(new.organization_id, old.organization_id));
    return null;
end $$;

create trigger members_keep_organization_owner after insert or update of role or delete on members
    for each row execute function members_keep_organization_owner();

alter function assert_project_owners(uuid) security definer set search_path = public;
alter function experiments_guard() security definer set search_path = public;
alter function projects_limits_guard() security definer set search_path = public;
alter function organizations_limits_guard() security definer set search_path = public;

create function current_organization() returns uuid language sql stable as $$
    select nullif(current_setting('antilog.organization_id', true), '')::uuid
$$;

create function current_token() returns uuid language sql stable as $$
    select nullif(current_setting('antilog.token_id', true), '')::uuid
$$;

alter table organizations enable row level security;
create policy organizations_scope on organizations
    using (id = current_organization()) with check (id = current_organization());

alter table members enable row level security;
create policy members_scope on members
    using (organization_id = current_organization())
    with check (organization_id = current_organization());

alter table projects enable row level security;
create policy projects_scope on projects
    using (organization_id = current_organization())
    with check (organization_id = current_organization());

alter table project_roles enable row level security;
create policy project_roles_scope on project_roles
    using (organization_id = current_organization())
    with check (organization_id = current_organization());

alter table policies enable row level security;
create policy policies_scope on policies
    using (organization_id = current_organization())
    with check (organization_id = current_organization());

alter table approvals enable row level security;
create policy approvals_scope on approvals
    using (organization_id = current_organization())
    with check (organization_id = current_organization());

-- An audit row may be appended with no organization set, as the session area's writes
-- do; reading the log always needs the organization.
alter table audit_log enable row level security;
create policy audit_log_read on audit_log for select
    using (organization_id = current_organization());
create policy audit_log_append on audit_log for insert
    with check (current_organization() is null or organization_id = current_organization());

alter table access_tokens enable row level security;
create policy access_tokens_scope on access_tokens
    using (organization_id = current_organization())
    with check (organization_id = current_organization());
create policy access_tokens_lookup on access_tokens for select using (id = current_token());

alter table sign_in_sessions enable row level security;
create policy sign_in_sessions_scope on sign_in_sessions
    using (organization_id = current_organization())
    with check (organization_id = current_organization());
create policy sign_in_sessions_lookup on sign_in_sessions for select using (id = current_token());
