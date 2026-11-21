# A tool

A tool is one of the setup tools `attach_repo`, `request_credential`, `set_environment` and `request_budget`. Every session's MCP endpoint (`/mcp/sessions/{id}`) serves them, plus the tools of the organization's connectors. They are listed in [session/tools.md](../session/tools.md); `antilog mcp` and REST tool endpoints do not exist.

## Steps

1. **Decide the product call first.** A new tool is a product change; see [product](../product/SKILL.md). A tool that would give an agent a model key or a way past a card is refused: see [session/rules.md](../session/rules.md).
2. **Define it.** Name, description, input and result types in `crates/antilog-core/src/tools/` (`mod.rs` lists the definitions and dispatches; `setup.rs` holds the setup tools). The description is what the agent is told, so it is part of the prompts, reviewed like product: see [session/prompts.md](../session/prompts.md). There is one definition and no second copy.
3. **Does it show a card?** A setup tool shows a card in the chat, and any tool that changes something that matters is an approval card. A card has a title, what would happen, what it may cost, and Approve and Decline. A new kind of card is a change to the card kinds in [organization/approvals.md](../organization/approvals.md) and to the board.
4. **Policy default.** Say which of Always ask, Allow up to a limit and Allow it starts with, and who may approve (owner, editor; never a viewer). A tool that needs a value only a person can type, such as `request_credential`, always asks and can never be pre-approved.
5. **Audit row.** A call that changes something, and its approval, is an audited action: register it with [organization/add-audited-action.md](../organization/add-audited-action.md). Then do [write-action.md](write-action.md).
6. **Session event.** Say what the session receives; a decided card reaches the agent as a message from Antilog. Store ids and states; never tool output, files, diffs or data.
7. **The words and the board.** Add the card and the result card to a board in `design/mockups/` first, then the strings: see [screen.md](screen.md) and [design wording](../design/wording.md).
8. **Tests.** A role that may not approve cannot call it through; a denied call changes nothing and audits the denial; the result carries no secret and no tool output. See [../land/tests.md](../land/tests.md).
