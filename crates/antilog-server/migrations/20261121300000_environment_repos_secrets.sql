-- Owner: session. An environment's default repos and network secrets (docs/V0.md). The repos
-- and the secrets' metadata (host, header, prefix) are part of a version, as JSON text like the
-- hosts and variables. A secret's value is sealed in its own table and never leaves the server:
-- a later version without a new value copies the sealed value of the one before it.
set local lock_timeout = '5s';

alter table environment_versions
    add column repos text not null default '[]' check (length(repos) <= 20000),
    add column network_secrets text not null default '[]' check (length(network_secrets) <= 40000);

create table environment_secrets (
    organization_id uuid not null,
    id text not null,
    version integer not null,
    host text not null,
    header text not null,
    -- Nonce and ciphertext (XChaCha20-Poly1305); the row binds it to the environment, host and header.
    sealed bytea not null,
    -- No foreign key to the version: it is written in the version's own transaction, and a
    -- reference would turn a truncate of the versions into an error before the guard answers.
    primary key (organization_id, id, version, host, header)
);

alter table environment_secrets enable row level security;
create policy environment_secrets_scope on environment_secrets
    using (organization_id = current_organization())
    with check (organization_id = current_organization());

create trigger environment_secrets_append_only before update or delete or truncate on environment_secrets
    for each statement execute function refuse_change();
