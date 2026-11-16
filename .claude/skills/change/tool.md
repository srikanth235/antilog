# A tool

A tool is one of the Ratchet tools: `seal`, `measure`, `history`, `spawn`, `deliver`, `watch`, and the setup tools `attach_repo`, `request_credential`, `set_environment` and `request_budget`. Every session gets them. `ratchet mcp` serves the same tools to an agent on the person's own machine. A change proposed anywhere is measured only by the judge. The tools are listed in [session/tools.md](../session/tools.md) and [judge/tools.md](../judge/tools.md). (target: nothing here is built yet.)

## Steps (target)

1. **Decide the product call first.** A new tool is a product change; see [product](../product/SKILL.md). A tool that would let a person or an agent mark a win, or let an agent read the holdout, is refused: see [judge/rules.md](../judge/rules.md).
2. **Define it.** Name, description, input and result types in `crates/ratchet-core/src/tools/` (target). The description is what the agent is told, so it lives with the other prompts, versioned and tested: see [session/prompts.md](../session/prompts.md). The same definition is served to a session and by `ratchet mcp`; there is no second copy.
3. **Does it show a card?** A setup tool shows a card in the chat. `seal`, `deliver` and any tool that changes something that matters are approval cards. A card has a title, what would happen, what it may cost, and Approve and Decline. A new kind of card is a change to the card kinds in [organization/approvals.md](../organization/approvals.md) and to the board.
4. **Policy default.** Say which of Always ask, Allow up to a limit and Allow it starts with, and who may approve (owner, editor; never a viewer). A tool that needs a value only a person can type, such as `request_credential`, always asks and can never be pre-approved.
5. **Audit row.** A call that changes something, and its approval, is an audited action: register it with [organization/add-audited-action.md](../organization/add-audited-action.md). Then do [write-action.md](write-action.md).
6. **Session event.** Say what the session receives. An asynchronous tool such as `measure` returns a ticket at once and the result arrives later as an event; see [live-event.md](live-event.md). Store ids and states; never tool output, files, diffs or data.
7. **Platform.** If the tool needs work on a platform, queue it as in [platform-job.md](platform-job.md). A tool must work on all five platforms through the agent trait; a platform that cannot serve it is a gap in [session/add-platform.md](../session/add-platform.md).
8. **CLI and MCP.** `ratchet mcp` serves it with no further work. Add a CLI command only if a person needs it from the shell.
9. **The words and the board.** Add the card and the result card to a board in `design/mockups/` first, then the strings: see [screen.md](screen.md) and [design wording](../design/wording.md).
10. **Tests.** A role that may not approve cannot call it through; a denied call changes nothing and audits the denial; the result carries no secret, no holdout value and no tool output; the tool definition is the same in a session and in `ratchet mcp`; the description is checked by the prompt tests. See [../land/tests.md](../land/tests.md).
