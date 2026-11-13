# Infra: rules that never bend

- Keep measuring on different machines from proposing, and never let an agent reach the evaluator, because an agent that can touch the evaluator can touch the score.
- Keep a benchmark on one provider and one machine type, because results across providers are not comparable.
- Tag every created resource with its run before anything else happens to it, because an untagged resource cannot be found or removed.
- Do not call a run finished until clean-up is verified, because finished with resources left is a spend leak.
- Record spend and stop at the cap optimization set, because a cap that is only advice is not a cap.
- Treat agents as workers here. An optimization chooses which agents and how many, because the choice of worker is not part of what better means.
- Keep provider words (regions, instance ids) inside this area, because they leak a vendor into every other skill.
- Run in the customer's account and never create resources in an account Ratchet operates.
- Never put a secret in the infrastructure state, a tag, a log or a response, because those are read by more people than the secret is meant for.
- Fail closed: a machine that lacks its run tag is denied everything.
