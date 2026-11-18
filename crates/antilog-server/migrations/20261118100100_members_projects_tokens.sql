-- Owner: organization. Members, projects, project roles, access tokens.
-- Errcodes AL001..AL005 are Antilog's invariants, mapped by the store.
set local lock_timeout = '5s';

create table members (
    id uuid primary key,
    organization_id uuid not null references organizations (id),
    kind text not null constraint members_kind check (kind in ('person', 'service')),
    name text not null check (length(name) between 1 and 200),
    provider text,
    subject text,
    role text not null constraint members_role check (role in ('viewer', 'editor', 'owner')),
    created_at timestamptz not null,
    unique (organization_id, id),
    check ((kind = 'person') = (provider is not null and subject is not null))
);
create unique index members_identity on members (organization_id, provider, subject)
    where provider is not null;

create table projects (
    id uuid primary key,
    organization_id uuid not null references organizations (id),
    name text not null check (length(name) between 1 and 200),
    description text not null check (length(description) <= 2000),
    monthly_cap_cents bigint check (monthly_cap_cents >= 0),
    default_budget_cents bigint check (default_budget_cents >= 0),
    created_at timestamptz not null,
    unique (organization_id, name),
    unique (organization_id, id),
    constraint projects_default_within_cap check (default_budget_cents <= monthly_cap_cents)
);

create table project_roles (
    organization_id uuid not null,
    project_id uuid not null,
    member_id uuid not null,
    role text not null constraint project_roles_role check (role in ('viewer', 'editor', 'owner')),
    primary key (project_id, member_id),
    foreign key (organization_id, project_id) references projects (organization_id, id),
    foreign key (organization_id, member_id) references members (organization_id, id)
);
create index project_roles_member on project_roles (organization_id, member_id);

create table access_tokens (
    id uuid primary key,
    organization_id uuid not null,
    member_id uuid not null,
    name text not null check (length(name) between 1 and 200),
    hash bytea not null unique check (length(hash) = 32),
    created_at timestamptz not null,
    revoked_at timestamptz,
    foreign key (organization_id, member_id) references members (organization_id, id)
);
create index access_tokens_member on access_tokens (organization_id, member_id);

create function assert_project_owners(org uuid) returns void language plpgsql as $$
begin
    perform 1 from organizations where id = org for update;
    if exists (
        select 1 from projects p
        where p.organization_id = org
          and not exists (
              select 1 from members m
              left join project_roles r on r.project_id = p.id and r.member_id = m.id
              where m.organization_id = org and coalesce(r.role, m.role) = 'owner')
    ) then
        raise exception 'a project needs at least one owner' using errcode = 'AL001';
    end if;
end $$;

create function members_role_changed() returns trigger language plpgsql as $$
begin
    perform assert_project_owners(new.organization_id);
    return null;
end $$;

create trigger members_keep_owners after update of role on members
    for each row when (old.role is distinct from new.role)
    execute function members_role_changed();

create function project_roles_changed() returns trigger language plpgsql as $$
begin
    perform assert_project_owners(coalesce(new.organization_id, old.organization_id));
    return null;
end $$;

create trigger project_roles_keep_owners after insert or update or delete on project_roles
    for each row execute function project_roles_changed();

create function projects_have_owner() returns trigger language plpgsql as $$
begin
    perform assert_project_owners(new.organization_id);
    return null;
end $$;

create constraint trigger projects_keep_owners after insert on projects
    deferrable initially deferred
    for each row execute function projects_have_owner();
