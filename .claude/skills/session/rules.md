# Session: rules that never bend

All target.

- Hand credentials to a session only through the platform's credential store, never in a prompt, a chat message, a repo file, an environment variable Ratchet writes in plain text, or Ratchet's database, because a secret in any of those is read by more people than it is meant for.
- Never give an agent the holdout credential. Give it read-only connector credentials that leave the holdout out, because a credential that can write or can see the holdout is one a proposer can leak or fit. Only the judge sandbox holds the holdout credential (see [../judge/rules.md](../judge/rules.md)).
- Make a git token push only to the session's own branch, short-lived and one per repo, because a token that reaches another branch can rewrite what the judge measured.
- Run no loop on the server and keep no step machine. The agent drives. The only server-side state machines are the judge's queue and the budget meter, because a loop on the server decides things a person and an agent should.
- Store no tool output, file contents, diffs or data. Store the conversation's words, tool call names, card states, commit ids, measurements, spend and audit rows, because the rest can carry customer data.
- Never let a spawned session spawn, because an unbounded tree is unbounded spend.
- Privilege no agent's vendor. Only the judge decides, so every agent races on the same contract and budget, and none is favoured in the product.
- Run every session on an agent platform, through the one agent trait, because a second way to start work is a second way to leak it. Ratchet runs no sandboxes of its own.
- Serve the Ratchet tools to a session over its own event stream, and never require a platform to reach the server, because a self-hosted server may sit behind a firewall.
- Never give Ratchet access to the customer's cloud: no cross-account role, no stored cloud keys. Ratchet holds only platform credentials, because access we hold is access that can be stolen. On Ratchet cloud they sit in Cloudflare's Secrets Store, bound to the Worker.
- Keep platform credentials out of every response, event, log and tag.
- Record spend from the platform's usage, and make the judge refuse `measure` when the budget is spent, because a cap that is only advice is not a cap. Use the platform's own limits (turns, time, tokens) as a second stop where it has them.
- Attribute every spend record to its session and agent, because one session must not spend another's money unseen.
- End a stopped or idle session's sandbox at once, because an idle sandbox is spend with no work.
- Do not call a session finished until its sandbox is verified deleted on its platform, because finished with resources left is a spend leak.
- Keep platform words (agent ids, vault ids, ARNs, regions) inside each adapter, because they leak a vendor into every other skill.
- Fail closed: a session Ratchet cannot tie to an organization and a budget is stopped and deleted.
