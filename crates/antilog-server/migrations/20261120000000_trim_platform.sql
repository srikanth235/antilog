-- Owner: session. The agent platform and its stepping are gone (docs/V0.md): a session no longer
-- names a platform, keeps a platform's handle or clean-up state, and the cursor the relay saved
-- between steps has nothing to save. Existing migrations stay as they were.
set local lock_timeout = '5s';

drop table session_relay;
alter table sessions drop column platform, drop column platform_handle, drop column cleanup;
alter table agents drop column platform;
