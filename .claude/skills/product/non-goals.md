# Non-goals

These are decided. Do not build them, propose them in passing, or leave a hook "for later". Raising one means asking the owner to reverse a ruling, not slipping it into a change.

- **An evaluator on a laptop.** Measuring never runs on a user's machine: the evaluator runs only on the measuring platform. Agents do not run on a Mac either: "This machine" is deferred (see [rulings](rulings.md)).
- **Running sandboxes ourselves.** Ratchet has no launcher, runner or sandbox backend. Agent platforms (Claude Managed Agents, the OpenAI Agents API, AWS AgentCore) run every agent's loop and every sandbox, behind one trait. Building our own sandbox layer would put us back in the business those platforms are in.
- **Stored passwords or a built-in user database.** Sign-in is through an identity provider: OpenID Connect, plus GitHub's OAuth on Ratchet cloud. We do not want to hold credentials worth stealing.
- **A hosted third-party auth service.** Self-hosting must not depend on a vendor the administrator did not choose.
- **A separate policy engine.** Roles are checked in the one action layer. A second place that decides who may do what is a second truth.
- **Running the simulation in the browser.** The benchmark runs in an evaluator sandbox, never in a page.
- **A mobile app.** The product is a desk tool; a phone adds a client to secure and keep in step for no job we have.
- **A wrapped desktop app for Windows and Linux.** The desktop app is native macOS only; Windows and Linux use the web app.
- **Dependence on one hosting vendor or agent platform.** The self-hosted server runs wherever a container runs, and agent platforms sit behind one trait. Ratchet cloud runs on Cloudflare, and the self-hosted server is its exit, so the core never depends on Cloudflare. Choices that tie the core to one host or one agent platform are refused.
- **A second design language.** One look, Midnight Brass, in every client and in the docs.
- **Transforms or data pipelines inside Ratchet.** Context says what a proposer may read; it does not reshape data. Customers already own their pipelines.
- **Third-party code in a plugin.** A plugin is a declarative bundle of evaluations, skill packs, suggested connectors and recommended agents. It is not a marketplace that runs third-party code inside the server.
- **Agents holding write, cloud, model or holdout credentials.** Agents get only read-only connector credentials that exclude the holdout, and a short-lived git token that may push only to the agent's own branch, all through the platform's credential store. Connector admin secrets never reach an agent, or the server. A credential that can write or can see the holdout is one a proposer can leak or fit. Connector credentials are registered once by the customer (see [rulings](rulings.md)).
- **Ratchet holding access to the customer's cloud.** No cross-account role and no stored cloud keys. Ratchet holds only agent platform credentials: the customer's when self-hosted, its own on Ratchet cloud.
- **Ratchet storing or brokering customer data.** The server never connects to data. The agent's code reads it through connectors. There is no context plane that pulls or proxies data.
- **Marking a win by hand.** Only the benchmark decides. A hand-marked win is a number nobody can trust.

The simulation mode (`ratchetd serve --mode sim`, scripted agents, mock cloud) is a developer and demo tool. It is not a product feature and must not shape the product's design.
