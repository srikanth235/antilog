# Non-goals

These are decided. Do not build them, propose them in passing, or leave a hook "for later". Raising one means asking the owner to reverse a ruling, not slipping it into a change.

- **An evaluator on a laptop.** Measuring never runs on a user's machine: the evaluator stays in the customer's infrastructure. A Mac may be the "This machine" backend where agents propose, with short-lived, read-only, scoped credentials, governed, budgeted and cleaned up like any backend (see [rulings](rulings.md)). Nothing else runs locally.
- **Stored passwords or a built-in user database.** Sign-in is OpenID Connect only. We do not want to hold credentials worth stealing.
- **A hosted third-party auth service.** Self-hosting must not depend on a vendor the administrator did not choose.
- **A separate policy engine.** Roles are checked in the one action layer. A second place that decides who may do what is a second truth.
- **Running the simulation in the browser.** The benchmark runs in the customer's infrastructure, never in a page.
- **A mobile app.** The product is a desk tool; a phone adds a client to secure and keep in step for no job we have.
- **A wrapped desktop app for Windows and Linux.** The desktop app is native macOS only; Windows and Linux use the web app.
- **Dependence on one hosting vendor.** The server runs wherever a container runs, and sandboxes run behind one backend trait. Choices that tie either to one host are refused.
- **A second design language.** One look, Midnight Brass, in every client and in the docs.
- **Transforms or data pipelines inside Ratchet.** Context says what a proposer may read; it does not reshape data. Customers already own their pipelines.
- **Third-party code in a plugin.** A plugin is a declarative bundle of evaluations, skill packs, suggested connectors and recommended agents. It is not a marketplace that runs third-party code inside the server.
- **Agents holding long-lived, write, cloud or holdout credentials.** Agents get only short-lived, read-only, scoped connector credentials minted per run, and a git token that is short-lived and scoped. Connector admin secrets never reach an agent. A credential that outlives the run, can write or can see the holdout is one a proposer can leak or fit.
- **Ratchet holding access to the customer's cloud.** No cross-account role, no stored cloud keys, no serverless compute in Ratchet's account (for now). The launcher runs in the customer's infrastructure.
- **Ratchet storing or brokering customer data.** The server never connects to data. The agent's code reads it through connectors. There is no context plane that pulls or proxies data.
- **Marking a win by hand.** Only the benchmark decides. A hand-marked win is a number nobody can trust.

The simulation mode (`ratchetd serve --mode sim`, scripted agents, mock cloud) is a developer and demo tool. It is not a product feature and must not shape the product's design.
