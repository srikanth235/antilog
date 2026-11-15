# Infra: rules that never bend

- Keep measuring on different machines from proposing, and never let an agent reach the evaluator, because an agent that can touch the evaluator can touch the score.
- Keep a benchmark on one provider and one machine type, because results across providers are not comparable.
- Put every sandbox backend behind one trait, and start and remove sandboxes only through the launcher, because a second way to start work is a second way to leak it.
- Never give Ratchet access to the customer's cloud: no cross-account role, no stored cloud keys, no compute in Ratchet's account except Ratchet-managed sandboxes, which an owner chooses. The runner connects out and opens no inbound port, because access we hold is access that can be stolen.
- Mint scoped credentials per run, short-lived and read-only, and keep connector admin secrets in the customer's secret manager (Ratchet-managed: our encrypted secret store, held by the managed runner), because the launcher is the only place that needs them.
- Give model keys to a sandbox only through the egress proxy, never as a variable, whether Ratchet-supplied or the customer's own, because a key in a sandbox is a key an agent can leak.
- Tag every created resource with its run before anything else happens to it, because an untagged resource cannot be found or removed.
- Do not call a run finished until clean-up is verified, because finished with resources left is a spend leak.
- Record spend and stop at the cap optimization set, because a cap that is only advice is not a cap.
- Start a run's budget as even shares, attribute every spend record to its agent, and stop an agent at its share, because one agent must not spend another's money. Move only what a stopped agent left, evenly to the agents still iterating, and record the move (target).
- Give each agent its own workspace, and start an evaluator sandbox per trial that is removed when the trial ends, because a shared sandbox leaks one agent's state into another's and an evaluator that lingers can be reached (target).
- Remove a stopped agent's sandboxes at once, because an idle sandbox is spend with no work.
- Treat agents as workers here. An experiment chooses which agents and how many, because the choice of worker is not part of what better means.
- Keep provider words (regions, instance ids, node pools) inside this area and inside each backend adapter, because they leak a vendor into every other skill.
- Run in the customer's infrastructure and never create resources in an account Ratchet operates, except Ratchet-managed sandboxes, which only an owner chooses, and which are removed when the run ends (target).
- Never put a secret in the infrastructure state, a tag, a log or a response, because those are read by more people than the secret is meant for.
- Fail closed: a machine that lacks its run tag is denied everything.
