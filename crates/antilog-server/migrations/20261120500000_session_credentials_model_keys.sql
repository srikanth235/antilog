-- Owner: proxy. Short-lived tokens a session's machine holds (only their hash is kept), and the
-- model keys an organization brings (sealed with the master key; the key is never a column).
set local lock_timeout = '5s';

create table session_credentials (
    hash bytea primary key check (length(hash) = 32),
    organization_id uuid not null,
    session_id uuid not null,
    purpose text not null constraint session_credentials_purpose check (purpose in ('model', 'mcp', 'runner')),
    created_at timestamptz not null default now(),
    expires_at timestamptz not null,
    foreign key (organization_id, session_id) references sessions (organization_id, id)
);
create index session_credentials_session on session_credentials (organization_id, session_id);

-- The machine presents a token and nothing else, so no organization is known when it is minted,
-- checked or revoked: these three functions are the whole of that window.
alter table session_credentials enable row level security;
create policy session_credentials_scope on session_credentials
    using (organization_id = current_organization())
    with check (organization_id = current_organization());

create function mint_session_credential(token_hash bytea, session uuid, token_purpose text, ttl_seconds double precision)
    returns void language plpgsql security definer set search_path = public as $$
begin
    insert into session_credentials (hash, organization_id, session_id, purpose, expires_at)
        select token_hash, s.organization_id, s.id, token_purpose, now() + make_interval(secs => ttl_seconds)
        from sessions s where s.id = session;
    if not found then
        raise foreign_key_violation using message = 'no such session';
    end if;
end $$;

create function verify_session_credential(token_hash bytea, token_purpose text)
    returns uuid language sql stable security definer set search_path = public as $$
    select session_id from session_credentials
    where hash = token_hash and purpose = token_purpose and expires_at > now()
$$;

create function revoke_session_credentials(session uuid)
    returns void language sql security definer set search_path = public as $$
    delete from session_credentials where session_id = session
$$;

-- The gateway holds a session token and needs the session's organization and experiment.
create function gateway_scope(session uuid)
    returns table (organization_id uuid, experiment_id uuid)
    language sql stable security definer set search_path = public as $$
    select organization_id, experiment_id from sessions where id = session
$$;

create table model_keys (
    organization_id uuid not null references organizations (id),
    provider text not null constraint model_keys_provider check (provider in ('anthropic', 'openai')),
    sealed bytea not null check (length(sealed) > 24),
    set_at timestamptz not null,
    primary key (organization_id, provider)
);

alter table model_keys enable row level security;
create policy model_keys_scope on model_keys
    using (organization_id = current_organization())
    with check (organization_id = current_organization());
