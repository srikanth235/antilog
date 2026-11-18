-- Owner: session. What the relay keeps between steps: the platform's cursor,
-- the seal a person is asked to approve, and which decided approvals the session was told of.
set local lock_timeout = '5s';

create table session_relay (
    organization_id uuid not null,
    session_id uuid primary key,
    cursor text not null,
    updated_at timestamptz not null,
    foreign key (organization_id, session_id) references sessions (organization_id, id)
);

create table seal_proposals (
    approval_id uuid primary key references approvals (id),
    organization_id uuid not null,
    body text not null check (length(body) <= 1000000)
);

create table approvals_told (
    approval_id uuid primary key references approvals (id),
    organization_id uuid not null
);
