# Non-goals

These are decided. Do not build them, propose them in passing, or leave a hook "for later". Raising one means asking the owner to reverse a ruling, not slipping it into a change.

- **No modes or plan step.** An experiment and its sessions have no mode switch and no approve-a-plan stage. The agent drives, and a person talks to it at any time.
- **No orchestration loop on the server.** The server runs no step machine and no agent loop. The only server-side state machines are the judge's queue and the budget meter.
- **No settings-first setup.** Nobody visits a settings page before the first message. Setup is an approved tool call in the chat. Two things stay in Settings because no experiment can run without them: model keys and members and sign-in. Connectors, agents, evaluations, skill packs and plugins can be set up once in Customize, but nothing requires it before the first message; a chat asks with a card.
- **No second way to mark a win.** Only the acceptance criteria mark a win, as one pure function in code. A hand-marked or agent-marked win is a number nobody can trust.
- **No generic coding-agent chat without the judge.** Every session always has the Antilog tools. A chat that cannot measure is a product we do not need to build.
- **Stored passwords or a built-in user database.** Sign-in is through an identity provider: OpenID Connect, plus GitHub's OAuth on Antilog cloud, which offers only GitHub and Google. We do not want to hold credentials worth stealing.
- **A hosted third-party auth service.** Self-hosting must not depend on a vendor the administrator did not choose.
- **A separate policy engine.** Roles are checked in the one action layer, and approval policy is a list of card kinds. A second place that decides who may do what is a second truth.
- **A mobile app.** The product is a desk tool; a phone adds a client to secure and keep in step for no job we have.
- **A wrapped desktop app for Windows and Linux.** The native app is macOS only; Windows and Linux use the web app, installed as a progressive web app.
- **A second design language.** One look, Midnight Brass, in every client and in the docs.
- **Transforms or data pipelines inside Antilog.** Connectors say what an agent's code may read; Antilog does not reshape data. Customers already own their pipelines.
- **Agents holding write, cloud, model or holdout credentials.** Agents get a short-lived git token that may push only to the session's own branch, through the server (see `docs/V0.md`). Credentials are typed into a secure form, never the chat; a model key never enters a session machine.
- **Antilog holding access to the customer's cloud.** No cross-account role and no stored cloud keys. Antilog holds only the customer's model keys, saved per organization per provider.
- **Antilog storing or brokering customer data.** The server never connects to data and stores none. It never stores tool output, file contents or diffs. There is no plane that pulls or proxies data.
- **Letting chat change what counts.** Chat can change what an agent tries. It can never change a sealed experiment, the protected files, the holdout or the acceptance criteria.
- **A plugin, skill pack or evaluation template that changes what counts.** A plugin has no powers of its own and can never mark, unmark or grant a dataset. The judge never reads a skill pack, and a pack never holds holdout data. A template is where an experiment starts, not something that edits a sealed one.
- **A project that is a second permission system.** A project sets a member's role for that project and nothing else. Permission is still decided in the one action layer.
- **Per-repo visibility.** Roles follow the organization and the project, never a repo.
- **Vendor benchmark claims.** Antilog shows what the judge measured on the customer's code ("Measured by the judge on your code. Not a vendor benchmark."). It never ranks or promotes an agent or provider from outside figures, and the word is "agent scoreboard", never "leaderboard".
- **A value model set by an agent.** An agent may suggest one in prose. Only a project owner sets or approves it, and it is sealed with the experiment.
- **An agent chosen by brand.** No agent is preselected anywhere. Auto chooses by measured cost per win among the agents an organization turned on, and says why.
- **Sessions directing sessions.** No nested sessions, no `spawn` tool: a person starts sessions ([docs/V0.md](../../../docs/V0.md), Not in v0).
- **An infrastructure tool in the product.** No OpenTofu, Terraform or sealed templates; an agent writes and runs infrastructure code itself in its machine ([docs/V0.md](../../../docs/V0.md)).
- **Paying for the customer's model calls.** Customers bring their model keys; the proxy meters the spend.
