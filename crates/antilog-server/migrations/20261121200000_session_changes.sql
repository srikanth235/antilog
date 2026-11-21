-- Owner: session. What a session's machine changed in each repo, against the commit it was cloned
-- at: the latest report per (session, repo), replaced on each report. Patch text is code, so it
-- is kept here only as the Changes panel's view of the machine (cut to 2 MB per diff by the
-- runner); the machine still holds the code. `files` and `commits` are JSON as text.
set local lock_timeout = '5s';

create table session_changes (
    session_id uuid not null,
    organization_id uuid not null,
    repo text not null check (length(repo) between 1 and 400),
    base_branch text not null check (length(base_branch) <= 400),
    head_branch text not null check (length(head_branch) <= 400),
    base_sha text not null check (length(base_sha) <= 64),
    head_sha text not null check (length(head_sha) <= 64),
    commits text not null check (length(commits) <= 200000),
    files text not null check (length(files) <= 16000000),
    reported_at timestamptz not null,
    primary key (session_id, repo),
    foreign key (organization_id, session_id) references sessions (organization_id, id)
);
create index session_changes_reported on session_changes (organization_id, reported_at);

create table session_commit_changes (
    session_id uuid not null,
    organization_id uuid not null,
    repo text not null check (length(repo) between 1 and 400),
    sha text not null check (length(sha) between 7 and 64),
    files text not null check (length(files) <= 16000000),
    primary key (session_id, repo, sha),
    foreign key (organization_id, session_id) references sessions (organization_id, id)
);

alter table session_changes enable row level security;
create policy session_changes_scope on session_changes
    using (organization_id = current_organization())
    with check (organization_id = current_organization());

alter table session_commit_changes enable row level security;
create policy session_commit_changes_scope on session_commit_changes
    using (organization_id = current_organization())
    with check (organization_id = current_organization());
