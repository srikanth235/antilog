# Add a dataset kind

A dataset kind is a shape of governed data, such as a table, a file set, or the history of attempts. No dataset exists in the repo yet, so every step here is the requirement.

1. **Name and shape.** Define the kind's schema in one place: how rows or files are addressed, and what makes two versions different. (target)
2. **Versions are immutable.** Creating a version never changes an earlier one. A virtual dataset is pinned by an as-of time, and the pin is part of the version id. (target)
3. **Named by a connector.** Only a connector names a dataset, through its translator. The kind never reads an outside system itself, and the server stores no rows. (target)
4. **No transforms.** The kind offers no join, filter or derive step. (target)
5. **Marking support.** The kind says where a marking applies (whole dataset, partition) and how the grant check finds it. A holdout marking applies to the whole version. See [add-marking.md](add-marking.md). (target)
6. **Scoped credential.** The kind says how its connector mints a short-lived, read-only credential for it, with the holdout left out for agents, and that it expires with the run. (target)
7. **Run pinning.** A run record lists the version of every dataset of this kind it read. Today the ledger has no such field (`crates/ratchet-ledger/migrations/0001_init.sql`). (target)
8. **History is a dataset.** If the kind is the history of attempts, every candidate and verdict is stored, failures included. Today the rows are in the ledger (`crates/ratchet-ledger/src/store.rs`) and the `history` tool reads them (`crates/ratchet-mcp/src/sets.rs`).
9. **Words.** Add the kind to [model.md](model.md).
10. **Tests.** Version immutability, an unpinned read failing, a holdout read refused for the agent's credential, and no rows through the server.

## Proving it

Follow [../land/SKILL.md](../land/SKILL.md).
