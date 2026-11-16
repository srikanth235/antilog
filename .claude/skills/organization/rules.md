# Organization: rules that never bend

- Send every write through one action layer that checks permission, writes the audit entry and emits the event, because three separate steps get skipped one at a time.
- Put the organization on every table row, because a query without it can cross customers.
- Resolve a role in one place: the project's role for the member if the project set one, else the organization role, because a second lookup is a second truth. Offer no per-repo visibility, because a third scope doubles every check.
- Let only a project owner set or approve a value model, and never an agent, because the worth of a metric decides what Return says and must belong to someone accountable.
- Never pre-approve Add credential, because a person must type the value into the secure form.
- Keep a report's reader list to members who may read the project, because a report carries spend and wins. A viewer reads only their project's report.
- Record every approval and every tool call that changes something, because a change nobody can trace is not governed.
- Let policy only pre-approve a card, never skip its audit row, because an automatic approval is still an approval.
- Build no built-in passwords, because Ratchet never stores a password. Sign-in is OpenID Connect, plus GitHub's OAuth: GitHub, Google and an organization's own OpenID Connect provider (single sign-on, set up by an owner) on Ratchet cloud, the customer's own OpenID Connect provider when self-hosted.
- Make every external endpoint configuration, never a constant, because a self-hosted server must contact only what its administrator set.
- Show a token once and store only its hash, because a stored token is a stolen token waiting.
- Use one cryptography stack, because two stacks mean two sets of bugs to patch.
- Check permission in one place, never in a handler, because a check in a handler is missing from the next handler.
- Never put a secret in a response, an event, a log or an audit entry, because those are read by more people than the secret is meant for.
- Keep the audit log append-only, because a log that can be edited is not a record.
- Keep every project with at least one owner, because a project nobody can manage cannot change its members or budget.
- Keep a project's budget inside the organization's cap and an experiment's inside its project's, because a nested limit that can be passed is not a limit.
- Keep organization a floor. It owns no experiment, session or sandbox. A project owns experiments only as a group and a permission scope.
