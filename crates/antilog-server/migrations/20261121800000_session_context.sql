-- Owner: session. How full a session's context window is: the latest figures only, one row per
-- session, replaced on each report. The runner writes `used_tokens` and `window_tokens` after
-- every model call; the model gateway writes `parts` (a JSON list as text) from the request it
-- forwarded. Numbers and kinds only: no words of the conversation are kept here.
set local lock_timeout = '5s';

create table session_context (
    session_id uuid primary key,
    organization_id uuid not null,
    used_tokens bigint check (used_tokens >= 0),
    window_tokens bigint check (window_tokens > 0),
    parts text not null default '[]' check (length(parts) <= 4000),
    reported_at timestamptz not null,
    foreign key (organization_id, session_id) references sessions (organization_id, id)
);
create index session_context_reported on session_context (organization_id, reported_at);

alter table session_context enable row level security;
create policy session_context_scope on session_context
    using (organization_id = current_organization())
    with check (organization_id = current_organization());
