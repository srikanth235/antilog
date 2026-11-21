-- Owner: session. What a tool call did, kept small: a short label, how it ended, and its input and output cut
-- to 4 KB with credentials taken out (antilog-types `ToolRecord::cleaned`). All null on a person's
-- or an agent's words, and on tool calls recorded before this column existed.
set local lock_timeout = '5s';

alter table session_events
    add column tool_label text check (length(tool_label) <= 1000),
    add column tool_status text constraint session_events_tool_status check (tool_status in ('done', 'failed')),
    add column tool_input text check (length(tool_input) <= 20000),
    add column tool_output text check (length(tool_output) <= 20000);
