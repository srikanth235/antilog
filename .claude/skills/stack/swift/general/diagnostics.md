# Swift compiler and concurrency diagnostics

Source: the Swift and concurrency diagnostics of [Bama](https://github.com/daveebbelaar/bama) by Dave Ebbelaar (MIT, [`../licenses/Bama-LICENSE`](../licenses/Bama-LICENSE)), adapted. [`../antilog.md`](../antilog.md) wins where they differ.

Fix the first diagnostic first; later errors can be consequences of it. The app and its tests use Swift 6 language mode, which enforces complete concurrency checking. Preserve actor isolation and Sendable ownership; do not weaken language mode or suppress diagnostics to get a build. The `xcodebuild` invocation is in [`../antilog.md`](../antilog.md).

## Compiler errors

| Diagnostic | First thing to check |
| --- | --- |
| `cannot find 'X' in scope` after adding a file | Check the file is a member of the target (or, if the project is generated, regenerate it), then rebuild. |
| `Main actor-isolated … from a nonisolated context` | Isolate the caller with `@MainActor` or make the call through an appropriate `await`. |
| `Sending 'x' risks causing data races` | Make the transferred value `Sendable`, or keep it within one actor. |
| Capture of a non-Sendable value in a `@Sendable` closure | Capture an immutable value such as an id or copy, or give the value a sound `Sendable` design. |
| The compiler cannot type-check an expression in reasonable time | Split the expression, add useful explicit types, or extract a view struct. |
| `ambiguous use of 'Task'` | Check for a type in the module named `Task`; rename it so the concurrency type is unambiguous. |
| `Call to main actor-isolated … in a synchronous nonisolated context` | Isolate the caller or explicitly hop to the main actor when that matches the callback's lifetime. |
| `Type 'X' does not conform to protocol 'View'` with a long diagnostic | Find the first error inside `body`; the conformance error is often a consequence. |
| An API is only available on a newer macOS version | Add an availability check with a suitable fallback; do not raise the deployment target just for one call. |
| A closure requires explicit `self` | Write `self.member`, or use `[weak self]` when the closure can outlive the owner and weak ownership is correct. |
| A generated type (colours, words, API client) is missing or stale | Run the generator and rebuild; never edit the generated file. |

## Concurrency patterns

- Put views and AppKit/SwiftUI state on `@MainActor`; put shared mutable state behind an actor. Keep CPU-heavy pure work off the main actor.
- Prefer structured `async`/`await`. A `Task` created directly is unstructured: retain and cancel it with its owner when its lifetime can outlast that owner. SwiftUI's `.task` is usually preferable for work tied to a view.
- `Task {}` inherits the caller's actor and priority. `Task.detached` does not; use it only when that loss is intentional and explain why.
- Make values that cross actor boundaries `Sendable`. Prefer correcting the type or ownership boundary over `@unchecked Sendable` or `nonisolated(unsafe)`; document the invariant if either escape hatch is unavoidable.
- Re-check state invariants after `await`, since another actor task can change state while the current task is suspended.
- For callbacks on unknown threads, hop to the required actor with a `Task { @MainActor in … }` when appropriate. Use `MainActor.assumeIsolated` only when the API contract guarantees the main thread; it traps otherwise.
- When a task-group closure triggers a region-based isolation diagnostic, simplify captures and keep mutable state in one isolation domain. Do not replace the group with an array of independent `Task`s: those tasks are unstructured and lose the group's cancellation and lifetime guarantees. Choose a different design that preserves structured cancellation, or use a narrowly scoped documented workaround only when its safety and lifetime are proven.

For broader Swift and concurrency API guidance, see [Swift](swift.md).
