-- Owner: session. The conversation: what people and agents said, and the name of each tool call.
-- Never a tool's input or output, a file or a diff. Append-only, one row per item; `key` makes a
-- relay's at-least-once delivery land once.
set local lock_timeout = '5s';

create table session_events (
    seq bigint generated always as identity primary key,
    organization_id uuid not null,
    session_id uuid not null,
    key text not null check (length(key) <= 200),
    kind text not null constraint session_events_kind check (kind in ('person', 'agent', 'tool')),
    member_id uuid,
    text text not null check (length(text) <= 100000),
    at timestamptz not null,
    unique (session_id, key),
    foreign key (organization_id, session_id) references sessions (organization_id, id),
    foreign key (organization_id, member_id) references members (organization_id, id)
);
create index session_events_session on session_events (organization_id, session_id, seq);

create trigger session_events_append_only before update or delete or truncate on session_events
    for each statement execute function refuse_change();

alter table session_events enable row level security;
create policy session_events_scope on session_events
    using (organization_id = current_organization())
    with check (organization_id = current_organization());
