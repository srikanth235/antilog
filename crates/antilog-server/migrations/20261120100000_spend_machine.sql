-- Owner: judge. Machine time is what a session's machine costs; "sandbox" named the retired
-- platform's container. Existing migrations stay as they were.
set local lock_timeout = '5s';

alter table spend drop constraint spend_kind;
-- Spend is append-only; the one rename is the exception, so the guard is lifted for it.
alter table spend disable trigger spend_append_only;
update spend set kind = 'machine' where kind = 'sandbox';
alter table spend enable trigger spend_append_only;
alter table spend add constraint spend_kind check (kind in ('tokens', 'machine', 'judge'));

-- Owner: session. Nested sessions are not in v0 (docs/V0.md): a session has no parent, and the
-- trigger no longer checks one.
alter table sessions drop column parent_session_id;

create or replace function sessions_guard() returns trigger language plpgsql as $$
begin
    if tg_op = 'UPDATE' and old.status in ('done', 'failed') and new.status <> old.status then
        raise exception 'a % session stays so', old.status using errcode = 'AL003';
    end if;
    return new;
end $$;
