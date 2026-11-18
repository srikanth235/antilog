---
name: organization
description: >-
  Who may approve what, and how is it recorded? Use for organizations, projects, members and roles (owner, editor, viewer, in the organization and per project), sign-in (GitHub and Google on Antilog cloud, the client's OpenID Connect provider when self-hosted), access tokens and service accounts, approvals and approval cards, policy (pre-approval), budgets and the monthly cap, the audit log, notifications, live events, permission checks, and adding a role, a sign-in provider or an audited action.
---

# Organization

The shared floor under sessions and the judge. It holds members, roles, projects, sign-in, approvals, budgets and the audit log. Built on this branch: the action layer, the permission matrix, sign-in, approvals, policy, budgets and caps, and the audit log reads (`antilog-core/src/organization.rs`, `auth.rs`, `approval.rs`, `budget.rs`; `antilog-server/src/api/auth.rs`, `api/organization.rs`, `signin.rs`). `GET /me`, sign-in without an organization id and the pending-cards list are built too. Live events are built too (`api/live.rs`, `GET /events/stream`, see [../change/live-event.md](../change/live-event.md)). Group mappings, notifications and the Settings pages are target.

| If you are doing | Read |
| --- | --- |
| Deciding what this area promises | [owes.md](owes.md) |
| Looking up a word (organization, project, project member, role, approval, action, audit entry) | [model.md](model.md) |
| Checking a change against the rules that never bend | [rules.md](rules.md) |
| Approval cards, policy, who may approve, budgets and the monthly cap | [approvals.md](approvals.md) |
| What notifies whom, where, and what a notification may carry | [notifications.md](notifications.md) |
| Adding a role, or changing what a role may do in the organization or in a project | [add-role.md](add-role.md) |
| Adding a sign-in provider | [add-sign-in-provider.md](add-sign-in-provider.md) |
| Adding a write, so it is permitted, audited and emitted | [add-audited-action.md](add-audited-action.md) |
| What a session is, the setup tools, the spend meter | [../session/SKILL.md](../session/SKILL.md), [../session/model.md](../session/model.md) |
| What a win is, the judge | [../judge/SKILL.md](../judge/SKILL.md) |
| Deciding whether a change belongs in the product | [../product/SKILL.md](../product/SKILL.md) |
| Making the change, then proving it | [../change/SKILL.md](../change/SKILL.md), [../land/SKILL.md](../land/SKILL.md) |
