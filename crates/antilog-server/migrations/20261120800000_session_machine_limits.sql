-- Owner: session (the runner's queue). Limits on the session machine (docs/V0.md): a runner's
-- organization lock lapses when it stops polling, a machine has a maximum lifetime, and an
-- organization may run only so many sessions at once.
set local lock_timeout = '5s';

alter table organizations add column max_running_sessions integer not null default 10
    check (max_running_sessions >= 1);

-- When each claim began, for the lifetime limit; `expiring` is set once the release for it was sent.
alter table machines add column claimed_at timestamptz;
alter table machines add column expiring boolean not null default false;
update machines set claimed_at = updated_at where state in ('claimed', 'running', 'idle');

-- The runner's last poll.
alter table runner_locks add column polled_at timestamptz;
update runner_locks set polled_at = locked_at;
alter table runner_locks alter column polled_at set not null;

drop function claim_machines(text, integer, timestamptz, timestamptz);

-- As before, plus: a lock whose runner has not polled for `p_lock_lapse_ms` lapses (the lapsed
-- runner's sessions went back to the queue above, as an expired lease does); the poll is recorded;
-- and an organization is handed no claim beyond its `max_running_sessions` (claimed, running and
-- idle machines count). A runner with no lock takes the organization of the first queued session
-- whose organization has room.
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
            where q.state = 'queued' and s.status in ('working', 'waiting_for_you')
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
            where q.state = 'queued' and q.organization_id = v_org and s.status in ('working', 'waiting_for_you')
            order by q.updated_at, q.session_id limit least(p_limit, v_room) for update of q skip locked)
        returning m.*;
end $$;
