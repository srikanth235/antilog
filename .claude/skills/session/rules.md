# Session: rules that never bend

> Parts of this page still name agent platforms, the judge sandbox or the relay; `docs/V0.md` replaces them: [docs/V0.md](../../../docs/V0.md).

All target.

- Hand credentials to a session only through the platform's credential store, never in a prompt, chat message, repo file, plain-text environment variable or Antilog's database, because a secret there is read by more people than it is meant for.
- Keep platform credentials out of every response, event, log and tag.
- Never show an agent a holdout value, in a message, card or tool result, and never give it the holdout credential, because what an agent reads it can fit. It learns only Kept or Failed holdout. Give it read-only connector credentials that leave the holdout out; only the judge sandbox holds the holdout credential (see [../judge/rules.md](../judge/rules.md)).
- Give each outside system exactly one translator and stop its vocabulary there, because a second translator forks the meaning of a column.
- Never reshape or pipe data. Build no transform and no pipeline: the customer prepares data in their own warehouse, because a transform layer makes Antilog responsible for data quality it cannot see.
- Never change a dataset version after it is created, and never let a holdout be granted to an agent (the control is absent), because a version that moves cannot be reproduced.
- Register each connector's two credentials once, in the customer's own system, and keep neither value in Antilog, because the server must hold no access to customer data. The agent credential is read-only and leaves out the holdout; the judge credential is separate. Both are short-lived and minted per session.
- Let a session contact only the endpoints its experiment and environment allow, because a sandbox with open egress can send customer data anywhere.
- Keep a skill pack to written text, never hold holdout data in one, and never let the judge read one, because a pack is advice to agents and an advice channel into the judge is a way to move what counts. Record the versions an experiment read.
- Never let a plugin mark, unmark or grant a dataset, because a bundle has no powers of its own.
- Never connect the Antilog server to data or route dataset rows through it, because data is read only inside platform sandboxes.
- Make a git token push only to the session's own branch, short-lived and one per repo, because a token that reaches another branch can rewrite what the judge measured.
- Run no loop on the server and keep no step machine. The agent drives. The only server-side state machines are the judge's queue and the budget meter, because a loop on the server decides things a person and an agent should.
- Open a terminal, a file or a preview only in the agent's own sandbox, for a person with edit access (read-only for a viewer), and record that it was opened in the audit log, because a shell in a sandbox is a way into it. Store none of what passes through it.
- Store no tool output, file contents, diffs or data. Store the conversation's words, tool call names, card states, commit ids, measurements, spend and audit rows, because the rest can carry customer data.
- Give each agent its own session and branch, because a shared session leaks one agent's state into another's.
- Privilege no agent's vendor: every agent works in the same experiment on the same budget. Preselect none by brand. Auto picks by measured cost per win among the agents turned on and says why, and a person can always pick a named one, because a default that favours a vendor ties the customer to it.
- Show agent figures only as measured by the judge on this organization's code, never as a vendor benchmark, because an outside number is a claim nobody here can check.
- Fail over only to an agent the organization turned on, and record it as a timeline line and an audit row, because a session that moves silently moves spend and code access silently.
- Hand a handed-off session a summary and the history, never tool output or diffs, because the server stores neither.
- Never give Antilog access to the customer's cloud: no cross-account role, no stored cloud keys. Antilog holds only platform credentials, because access we hold is access that can be stolen. On Antilog cloud they sit in AWS Secrets Manager, read by the server through workload identity.
- Book model spend only in the proxy, from the usage each reply reports (a stream's last usage event; an aborted stream books what was seen and is logged), and refuse a model call and `measure` once the budget is spent, because a cap that is only advice is not a cap and the harness's own counts are not authoritative. An unknown model is booked at the highest known price.
- Attribute every spend record to its session and agent, because one session must not spend another's money unseen.
- End and delete a stopped or idle session's sandbox at once, and call it finished only when deletion is verified on its platform, because an idle sandbox is spend with no work. One not removed shows Not removed, is retried, and its cost counts until it is gone.
- Treat agents as workers: choosing which and how many is not part of what better means, because the choice of worker does not change the experiment's definition. A person or an agent chooses them in the chat.
- Keep platform words (agent ids, vault ids, ARNs, regions) inside each adapter, because they leak a vendor into every other skill.
- Assume every agent can be prompt-injected, and check every tool argument it sends as untrusted input ([threat-model.md](threat-model.md)), because it reads text that people outside the organization write.
- Fail closed: a session Antilog cannot tie to an organization and a budget is stopped and deleted.
