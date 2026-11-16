# Session: rules that never bend

All target.

- Hand credentials to a session only through the platform's credential store, never in a prompt, a chat message, a repo file, an environment variable Ratchet writes in plain text, or Ratchet's database, because a secret in any of those is read by more people than it is meant for.
- Never show an agent a holdout value, in a message, a card or a tool result, because what an agent reads it can fit. It learns only Kept or Failed holdout.
- Never give an agent the holdout credential. Give it read-only connector credentials that leave the holdout out, because a credential that can write or can see the holdout is one a proposer can leak or fit. Only the judge sandbox holds the holdout credential (see [../judge/rules.md](../judge/rules.md)).
- Give each outside system exactly one translator and stop its vocabulary there, because a second translator forks the meaning of a column.
- Never reshape or pipe data. Build no transform and no pipeline: the customer prepares data in their own warehouse, because a transform layer makes Ratchet responsible for data quality it cannot see.
- Never change a dataset version after it is created, and never let a holdout be granted to an agent: the control is absent, because a version that moves cannot be reproduced and an agent that reads the holdout fits it.
- Register each connector's two credentials once, with the customer's own system, and keep neither value in Ratchet, because the server must hold no access to customer data. The agent credential is read-only and leaves out the holdout. The judge credential is separate. Both are short-lived and minted per session.
- Let a session contact only the endpoints its experiment and environment allow, because a sandbox with open egress can send customer data anywhere.
- Keep a skill pack to written text, never hold holdout data in one, and never let the judge read one, because a pack is advice to agents and an advice channel into the judge is a way to move what counts. Record the versions an experiment read.
- Never let a plugin mark, unmark or grant a dataset, because a bundle has no powers of its own.
- Never connect the Ratchet server to data and never route dataset rows through it, because data is read only inside platform sandboxes.
- Make a git token push only to the session's own branch, short-lived and one per repo, because a token that reaches another branch can rewrite what the judge measured.
- Run no loop on the server and keep no step machine. The agent drives. The only server-side state machines are the judge's queue and the budget meter, because a loop on the server decides things a person and an agent should.
- Open a terminal, a file or a preview only in the agent's own sandbox, for a person with edit access (read-only for a viewer), and record that it was opened in the audit log, because a shell in a sandbox is a way into it. Store none of what passes through it.
- Store no tool output, file contents, diffs or data. Store the conversation's words, tool call names, card states, commit ids, measurements, spend and audit rows, because the rest can carry customer data.
- Give each agent its own session and branch, because a shared session leaks one agent's state into another's.
- Never let a spawned session spawn, because an unbounded tree is unbounded spend.
- Privilege no agent's vendor. Only the judge decides, so every agent works in the same experiment on the same budget, and none is favoured in the product.
- Run every session on an agent platform, through the one agent trait, because a second way to start work is a second way to leak it. Ratchet runs no sandboxes of its own.
- Serve the Ratchet tools to a session over its own event stream, and never require a platform to reach the server, because a self-hosted server may sit behind a firewall.
- Never give Ratchet access to the customer's cloud: no cross-account role, no stored cloud keys. Ratchet holds only platform credentials, because access we hold is access that can be stolen. On Ratchet cloud they sit in Cloudflare's Secrets Store, bound to the Worker.
- Keep platform credentials out of every response, event, log and tag.
- Record spend from the platform's usage, and make the judge refuse `measure` when the budget is spent, because a cap that is only advice is not a cap. Use the platform's own limits (turns, time, tokens) as a second stop where it has them.
- Attribute every spend record to its session and agent, because one session must not spend another's money unseen.
- End and delete a stopped or idle session's sandbox at once, because an idle sandbox is spend with no work. A session that is not removed is shown as Not removed, retried, and its cost counts until it is gone.
- Do not call a session finished until its sandbox is verified deleted on its platform, because finished with resources left is a spend leak.
- Treat agents as workers here. Choosing which agents and how many is not part of what better means, because the choice of worker does not change the experiment's definition. A person or an agent chooses them in the chat.
- Keep platform words (agent ids, vault ids, ARNs, regions) inside each adapter, because they leak a vendor into every other skill.
- Fail closed: a session Ratchet cannot tie to an organization and a budget is stopped and deleted.
