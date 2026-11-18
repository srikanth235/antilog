set local lock_timeout = '5s';

create table organizations (
    id uuid primary key,
    name text not null unique check (length(name) between 1 and 200),
    created_at timestamptz not null
);
