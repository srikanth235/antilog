---
name: stack
description: >-
  How do I write this language here, and which library may I use? Use for Rust, TypeScript, Swift (the macOS app), SQL and the ledger, OpenTofu and Packer, or adding a dependency: the repo's own constraints, the check command, and the chosen target libraries.
---

# Stack

Read the file for the language first: it holds this repo's constraints and its check. Which checks to run before you call a change done, and how to report them: [land](../land/SKILL.md).

| File type | Read | Check |
| --- | --- | --- |
| `.rs`, `Cargo.toml` | [rust.md](rust.md) | `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace --no-fail-fast` |
| `.ts` in `examples/`, `infra/tests/` | [typescript.md](typescript.md) | `pnpm install && pnpm build && pnpm typecheck && pnpm test` |
| `.swift`, the macOS app (target) | [swift.md](swift.md) | Not built yet |
| SQL, migrations | [sql.md](sql.md) | `cargo test -p ratchet-ledger` |
| `.tf`, `.hcl`, `infra/` | [infra-code.md](infra-code.md) | `cd infra/aws && tofu fmt -check -recursive && tofu init -backend=false && tofu validate && tofu test` |
| Adding or changing a dependency | [libraries.md](libraries.md) | the check of the language |
