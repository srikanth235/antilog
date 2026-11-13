# Non-goals

These are decided. Do not build them, propose them in passing, or leave a hook "for later". Raising one means asking the owner to reverse a ruling, not slipping it into a change.

- **A local runner in the server product.** Agents running on the user's own machine are out of the server product. Work runs in the customer's cloud, where it can be governed, budgeted and cleaned up. The local mode of the removed Mac app is not carried forward.
- **Stored passwords or a built-in user database.** Sign-in is OpenID Connect only. We do not want to hold credentials worth stealing.
- **A hosted third-party auth service.** Self-hosting must not depend on a vendor the administrator did not choose.
- **A separate policy engine.** Roles are checked in the one action layer. A second place that decides who may do what is a second truth.
- **Running the simulation in the browser.** The benchmark runs in the customer's cloud, never in a page.
- **A mobile app.** The product is a desk tool; a phone adds a client to secure and keep in step for no job we have.
- **A wrapped desktop app for Windows and Linux.** The web app serves them.
- **Dependence on one hosting vendor.** The server runs wherever a container runs. Choices that tie it to one host are refused.
- **A second design language.** One look, Midnight Brass, in every client and in the docs.
- **Transforms or data pipelines inside Ratchet.** Context says what a proposer may read; it does not reshape data. Customers already own their pipelines.
- **Third-party code in a plugin.** A plugin is a declarative bundle of evaluations, skill packs, suggested connectors and recommended agents. It is not a marketplace that runs third-party code inside the server.
- **Agents holding source or cloud credentials.** Agents get the access a run grants and nothing more. A credential in an agent is a credential a proposer can leak.
- **Marking a win by hand.** Only the benchmark decides. A hand-marked win is a number nobody can trust.

The simulation mode (`ratchetd serve --mode sim`, scripted agents, mock cloud) is a developer and demo tool. It is not a product feature and must not shape the product's design.
