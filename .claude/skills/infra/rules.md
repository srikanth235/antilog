# Infra: rules that never bend

All target.

- Run every session and every sandbox on an agent platform, through the one platform trait, because a second way to start work is a second way to leak it. Ratchet runs no sandboxes of its own.
- Keep measuring in an evaluator sandbox with no agent in it, separate from every agent's session, and never give an agent's session the holdout credential, the evaluation's code or a route to the evaluator, because an agent that can touch the evaluator can touch the score.
- Keep a benchmark on one measuring platform and one environment, because results across platforms or machine sizes are not comparable.
- Answer the `measure` tool over the session's own event stream, and never require a platform to reach the server, because a self-hosted server may sit behind a firewall.
- Hand credentials to a session only through the platform's credential store, never in a prompt, a repo file, an environment variable Ratchet writes in plain text, or Ratchet's database, because a secret in any of those is read by more people than it is meant for.
- Give an agent only read-only connector credentials that exclude the holdout and a short-lived git token that may push only to its own branch, because a credential that can write or see the holdout is one a proposer can leak or fit.
- Never give Ratchet access to the customer's cloud: no cross-account role, no stored cloud keys. Ratchet holds only platform credentials, because access we hold is access that can be stolen.
- Keep platform credentials out of every response, event, log and tag. On Ratchet cloud they sit in Cloudflare's Secrets Store, bound to the Worker.
- Record spend from the platform's usage and stop a session at the cap optimization set, because a cap that is only advice is not a cap. Use the platform's own limits (turns, time, tokens) as a second stop where it has them.
- Start a run's budget as even shares, attribute every spend record to its agent, and stop an agent at its share, because one agent must not spend another's money. Move only what a stopped agent left, evenly to the agents still iterating, and record the move.
- Give each agent its own session and branch, and start an evaluator sandbox per trial that is removed when the trial ends, because a shared session leaks one agent's state into another's and an evaluator that lingers can be reached.
- End and delete a stopped agent's session at once, because an idle session is spend with no work.
- Do not call a run finished until every session and sandbox is verified deleted on its platform, because finished with resources left is a spend leak.
- Store commit ids, never diffs, transcripts or logs, because they can carry customer data.
- Treat agents as workers here. An experiment chooses which agents and how many, because the choice of worker is not part of what better means.
- Keep platform words (agent ids, vault ids, ARNs, regions) inside each adapter, because they leak a vendor into every other skill.
- Fail closed: a session Ratchet cannot tie to a run and an agent is stopped and deleted.
