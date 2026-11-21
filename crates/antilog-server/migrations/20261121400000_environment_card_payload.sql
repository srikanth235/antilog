-- A Change environment card now carries the whole proposal (`CardPayload::ChangeEnvironment`:
-- the change, the environment before and after). The cards raised with the old shape (image,
-- script line count, size, network) cannot be read as it, so their payload is dropped; the
-- summary still says what they asked.
update approvals set payload = null where card = 'change_environment' and payload is not null;
