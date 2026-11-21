# Non-goals

These are decided. Do not build them, propose them in passing, or leave a hook "for later". Raising one means asking the owner to reverse a ruling, not slipping it into a change.

- **No modes or plan step.** A session has no mode switch and no approve-a-plan stage. The agent drives, and a person talks to it at any time.
- **No orchestration loop on the server.** The server runs no step machine and no agent loop. The only server-side state machines are the runner's queue and the budget meter.
- **No settings-first setup.** Nobody visits a settings page before the first message. Setup is an approved tool call in the chat. Two things stay in Settings because no session can run without them: model keys, and members and sign-in. Connectors, agents, skill packs and plugins can be set up once in Customize, but nothing requires it before the first message; a chat asks with a card.
- **Experiments and the judge, for now.** Measuring a change, trials, wins, receipts, value models and the holdout are parked (owner, 21 Nov 2026); the code is deleted and `daa445f` keeps it. Do not rebuild a piece of it under another name.
- **Stored passwords or a built-in user database.** Sign-in is through an identity provider: OpenID Connect, plus GitHub's OAuth on Antilog cloud, which offers only GitHub and Google. We do not want to hold credentials worth stealing.
- **A hosted third-party auth service.** Self-hosting must not depend on a vendor the administrator did not choose.
- **A separate policy engine.** Roles are checked in the one action layer, and approval policy is a list of card kinds. A second place that decides who may do what is a second truth.
- **A mobile app.** The product is a desk tool.
- **A wrapped desktop app for Windows and Linux.** The native app is macOS only; Windows and Linux use the web app, installed as a progressive web app.
- **A second design language.** One look, Midnight Brass, in every client and in the docs.
- **Transforms or data pipelines inside Antilog.** Connectors say what an agent's code may reach; Antilog does not reshape data.
- **Agents holding model or cloud credentials.** A model key never enters a session machine. Credentials are typed into a secure form, never the chat. A service key may sit in an environment variable the agent can read, and the screen says so ([docs/V0.md](../../../docs/V0.md)).
- **Antilog holding access to the customer's cloud.** No cross-account role and no stored cloud keys. Antilog holds only the customer's model keys, saved per organization per provider.
- **Antilog storing or brokering customer data.** The server never connects to data and stores none. It never stores tool output, file contents or diffs. There is no plane that pulls or proxies data.
- **A plugin or skill pack with powers of its own.** A plugin is skills and connectors, each governed as usual.
- **A project that is a second permission system.** A project sets a member's role for that project and nothing else. Permission is still decided in the one action layer.
- **Per-repo visibility.** Roles follow the organization and the project, never a repo.
- **An agent chosen by brand.** No agent is preselected anywhere; `auto` picks the organization's default agent.
- **Sessions directing sessions.** No nested sessions, no `spawn` tool: a person starts sessions ([docs/V0.md](../../../docs/V0.md), Not in v0).
- **An infrastructure tool in the product.** No OpenTofu, Terraform or sealed templates; an agent writes and runs infrastructure code itself in its machine ([docs/V0.md](../../../docs/V0.md)).
- **Paying for the customer's model calls.** Customers bring their model keys; the proxy meters the spend.
- **A tool surface outside the session.** `antilog mcp` and REST tool endpoints are removed; an agent reaches Antilog only through its session's MCP endpoint.
