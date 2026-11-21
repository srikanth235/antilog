-- Owner: session (the runner's queue). A runner locks to the organization of the first session it
-- claims and claims only that organization's sessions until it exits, so customers never share a
-- host (docs/V0.md). The server enforces it at claim time; the runner says it is leaving with its
-- last poll, which deletes the lock when the runner holds no session.
set local lock_timeout = '5s';

create table runner_locks (
    holder text primary key check (length(holder) between 1 and 200),
    organization_id uuid not null references organizations (id),
    locked_at timestamptz not null
);

-- Not scoped to an organization: only a runner reads or writes it.
alter table runner_locks enable row level security;
create policy runner_locks_scope on runner_locks
    using (runner_access())
    with check (runner_access());

-- As before, plus the lock. A call with no room claims nothing and locks nothing. A runner with
-- no lock takes the organization of the first session in the queue and keeps it.
create or replace function claim_machines(p_holder text, p_limit integer, p_now timestamptz, p_until timestamptz)
returns setof machines language plpgsql security definer set search_path = public as $$
declare
    v_org uuid;
begin
    update machine_commands c set delivered = false
        from machines m
        where m.session_id = c.session_id and m.state in ('claimed', 'running', 'idle')
            and m.lease_until < p_now and not c.finished;
    update machines set state = 'queued', holder = null, updated_at = p_now
        where state in ('claimed', 'running', 'idle') and lease_until < p_now;
    if p_limit <= 0 then
        return;
    end if;
    select l.organization_id into v_org from runner_locks l where l.holder = p_holder;
    if v_org is null then
        select q.organization_id into v_org
            from machines q join sessions s on s.id = q.session_id
            where q.state = 'queued' and s.status in ('working', 'waiting_for_you')
            order by q.updated_at, q.session_id limit 1;
        if v_org is null then
            return;
        end if;
        insert into runner_locks (holder, organization_id, locked_at) values (p_holder, v_org, p_now)
            on conflict (holder) do nothing;
        select l.organization_id into v_org from runner_locks l where l.holder = p_holder;
    end if;
    return query
        update machines m set state = 'claimed', holder = p_holder, lease_until = p_until, updated_at = p_now
        where m.session_id in (
            select q.session_id from machines q join sessions s on s.id = q.session_id
            where q.state = 'queued' and q.organization_id = v_org and s.status in ('working', 'waiting_for_you')
            order by q.updated_at, q.session_id limit p_limit for update of q skip locked)
        returning m.*;
end $$;
