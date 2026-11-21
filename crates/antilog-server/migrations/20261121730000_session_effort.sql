-- Owner: session. The effort level chosen for a session's model (an OpenCode model variant such as
-- "high"); null is the model's own default. The model column already exists; both can change
-- mid-session from the composer (docs/V0.md, "Model picker").
set local lock_timeout = '5s';

alter table sessions add column effort text;
alter table sessions add constraint sessions_effort check (effort is null or (length(effort) between 1 and 32 and effort !~ '\s'));

-- A neutral line of the timeline ("Switched to GLM 5.3 Flash · High"), shown as one, not as a message.
alter table session_events drop constraint session_events_kind;
alter table session_events add constraint session_events_kind
    check (kind in ('person', 'agent', 'tool', 'error', 'note'));
