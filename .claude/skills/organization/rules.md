# Organization: rules that never bend

- Send every write through one action layer that checks permission, writes the audit entry and emits the event, because three separate steps get skipped one at a time.
- Put the organization on every table row, because a query without it can cross customers.
- Build no built-in passwords, because Ratchet never stores a password. Sign-in is OpenID Connect only: GitHub and Google on Ratchet cloud, the customer's own identity provider when self-hosted.
- Make every external endpoint configuration, never a constant, because a self-hosted server must contact only what its administrator set.
- Show a token once and store only its hash, because a stored token is a stolen token waiting.
- Use one cryptography stack, because two stacks mean two sets of bugs to patch.
- Check permission in one place, never in a handler, because a check in a handler is missing from the next handler.
- Never put a secret in a response, an event, a log or an audit entry, because those are read by more people than the secret is meant for.
- Keep the audit log append-only, because a log that can be edited is not a record.
- Keep organization a floor, not a fourth basic. It owns no optimization, dataset or machine.
