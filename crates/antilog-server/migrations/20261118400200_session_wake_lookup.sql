-- Owner: session. A wake-up names only a session id; it may read that one row to learn its organization.
set local lock_timeout = '5s';

create policy sessions_lookup on sessions for select using (id = current_token());
