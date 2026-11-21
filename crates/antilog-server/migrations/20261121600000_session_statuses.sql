-- Owner: session. Two more session statuses and one more kind of event (docs/V0.md, "Fixes from the
-- first end-to-end run"): a session not yet held by a runner is `queued`; one a person stopped is
-- `stopped`, not `done`; and an error from the model's provider or the machine is an `error` event.
-- The queue also offers a queued session to a runner.
set local lock_timeout = '5s';

alter table sessions drop constraint sessions_status;
alter table sessions add constraint sessions_status
    check (status in ('queued', 'working', 'waiting_for_you', 'done', 'stopped', 'failed'));
-- A session that has ended stays so; the one change is the rename of what a person stopped.
alter table sessions disable trigger sessions_guard;
update sessions set status = 'stopped' where status = 'done' and note like 'Stopped by %';
alter table sessions enable trigger sessions_guard;
create or replace function sessions_guard() returns trigger language plpgsql as $$
begin
    if tg_op = 'UPDATE' and old.status in ('done', 'stopped', 'failed') and new.status <> old.status then
        raise exception 'a % session stays so', old.status using errcode = 'AL003';
    end if;
    return new;
end $$;
update sessions s set status = 'queued'
    from machines m where m.session_id = s.id and m.state = 'queued' and s.status = 'working';

alter table session_events drop constraint session_events_kind;
alter table session_events add constraint session_events_kind
    check (kind in ('person', 'agent', 'tool', 'error'));

drop function claim_machines(text, integer, timestamptz, timestamptz, bigint);

create function claim_machines(p_holder text, p_limit integer, p_now timestamptz, p_until timestamptz, p_lock_lapse_ms bigint)
returns setof machines language plpgsql security definer set search_path = public as $$
declare
    v_org uuid;
    v_room integer;
begin
    update machine_commands c set delivered = false
        from machines m
        where m.session_id = c.session_id and m.state in ('claimed', 'running', 'idle')
            and m.lease_until < p_now and not c.finished;
    update machines set state = 'queued', holder = null, updated_at = p_now
        where state in ('claimed', 'running', 'idle') and lease_until < p_now;
    delete from runner_locks where polled_at < p_now - p_lock_lapse_ms * interval '1 millisecond';
    update runner_locks set polled_at = p_now where holder = p_holder;
    if p_limit <= 0 then
        return;
    end if;
    select l.organization_id into v_org from runner_locks l where l.holder = p_holder;
    if v_org is null then
        select q.organization_id into v_org
            from machines q join sessions s on s.id = q.session_id
            join organizations o on o.id = q.organization_id
            where q.state = 'queued' and s.status in ('queued', 'working', 'waiting_for_you')
                and (select count(*) from machines h where h.organization_id = q.organization_id
                    and h.state in ('claimed', 'running', 'idle')) < o.max_running_sessions
            order by q.updated_at, q.session_id limit 1;
        if v_org is null then
            return;
        end if;
        insert into runner_locks (holder, organization_id, locked_at, polled_at) values (p_holder, v_org, p_now, p_now)
            on conflict (holder) do nothing;
        select l.organization_id into v_org from runner_locks l where l.holder = p_holder;
    end if;
    -- One claim at a time per organization, so two runners cannot both take the last place.
    select o.max_running_sessions into v_room from organizations o where o.id = v_org for update;
    v_room := v_room - (select count(*) from machines h where h.organization_id = v_org
        and h.state in ('claimed', 'running', 'idle'));
    if v_room <= 0 then
        return;
    end if;
    return query
        update machines m set state = 'claimed', holder = p_holder, lease_until = p_until, updated_at = p_now,
            claimed_at = p_now, expiring = false
        where m.session_id in (
            select q.session_id from machines q join sessions s on s.id = q.session_id
            where q.state = 'queued' and q.organization_id = v_org and s.status in ('queued', 'working', 'waiting_for_you')
            order by q.updated_at, q.session_id limit least(p_limit, v_room) for update of q skip locked)
        returning m.*;
end $$;
