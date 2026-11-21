-- Owner: organization. A person's organizations are found from their identity, before any
-- organization is known: a read-only window on the rows of the one identity a transaction names
-- (antilog.identity_provider and antilog.identity_subject), and on those rows' organizations.
set local lock_timeout = '5s';

create function current_identity_provider() returns text language sql stable as $$
    select nullif(current_setting('antilog.identity_provider', true), '')
$$;

create function current_identity_subject() returns text language sql stable as $$
    select nullif(current_setting('antilog.identity_subject', true), '')
$$;

create policy members_identity_lookup on members for select
    using (provider = current_identity_provider() and subject = current_identity_subject());

create policy organizations_identity_lookup on organizations for select
    using (id in (select organization_id from members));
