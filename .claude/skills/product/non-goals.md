# Non-goals

These are decided. Do not build them, propose them in passing, or leave a hook "for later". Raising one means asking the owner to reverse a ruling, not slipping it into a change.

- **No modes or plan step.** An experiment and its sessions have no mode switch and no approve-a-plan stage. The agent drives, and a person talks to it at any time.
- **No orchestration loop on the server.** The server runs no step machine and no agent loop. The only server-side state machines are the judge's queue and the budget meter.
- **No settings-first setup.** Nobody visits a settings page before the first message. Setup is an approved tool call in the chat. Two things stay in Settings because no experiment can run without them: agent platform accounts (self-hosted) and members and sign-in. A connector can also be registered once in Settings, Connectors, but nothing requires it before the first message; a chat asks with an Add credential card.
- **No second way to mark a win.** Only the acceptance criteria mark a win, as one pure function in code. A hand-marked or agent-marked win is a number nobody can trust.
- **No generic coding-agent chat without the judge.** Every session always has the Ratchet tools. A chat that cannot measure is a product we do not need to build.
- **A judge on a laptop.** Measuring never runs on a user's machine. A judge sandbox runs only on a judge platform, with no agent in it. Agents do not run on a Mac either: "This machine" is deferred (see [rulings](rulings.md)).
- **Running sandboxes ourselves.** Ratchet has no launcher, runner or sandbox backend. Agent platforms (Claude Managed Agents, AWS AgentCore, the OpenAI Agents API, Microsoft Foundry Agent Service, Google Agent Runtime) run every agent and every judge sandbox, behind their traits. Building our own sandbox layer would put us back in the business those platforms are in.
- **Stored passwords or a built-in user database.** Sign-in is through an identity provider: OpenID Connect, plus GitHub's OAuth on Ratchet cloud. We do not want to hold credentials worth stealing.
- **A hosted third-party auth service.** Self-hosting must not depend on a vendor the administrator did not choose.
- **A separate policy engine.** Roles are checked in the one action layer, and approval policy is a list of card kinds. A second place that decides who may do what is a second truth.
- **Running the measurement in the browser.** A measurement runs in a judge sandbox, never in a page.
- **A mobile app.** The product is a desk tool; a phone adds a client to secure and keep in step for no job we have.
- **A wrapped desktop app for Windows and Linux.** The native app is macOS only; Windows and Linux use the web app, installed as a progressive web app.
- **Dependence on one hosting vendor or agent platform.** The self-hosted server runs wherever a container runs, and agent platforms sit behind one trait. Ratchet cloud runs on Cloudflare, and the self-hosted server is its exit, so the core never depends on Cloudflare.
- **A second design language.** One look, Midnight Brass, in every client and in the docs.
- **Transforms or data pipelines inside Ratchet.** Connectors say what an agent's code may read; Ratchet does not reshape data. Customers already own their pipelines.
- **Agents holding write, cloud, model or holdout credentials.** Agents get only read-only connector credentials that exclude the holdout, and a short-lived git token that may push only to the session's own branch, all through the platform's credential store. Credentials are typed into a secure form, never the chat, and the agent never sees them.
- **Ratchet holding access to the customer's cloud.** No cross-account role and no stored cloud keys. Ratchet holds only agent platform credentials: the customer's when self-hosted, its own on Ratchet cloud.
- **Ratchet storing or brokering customer data.** The server never connects to data and stores none. It never stores tool output, file contents or diffs. There is no plane that pulls or proxies data.
- **Letting chat change what counts.** Chat can change what an agent tries. It can never change a sealed experiment, the protected files, the holdout or the acceptance criteria.
