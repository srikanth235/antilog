# Swift (target)

Nothing of this exists yet. The macOS app is SwiftUI, macOS only. Windows and Linux use the web app ([typescript.md](typescript.md)). Toolchain and versions are in [libraries.md](libraries.md), including the open question of Sparkle against the Mac App Store.

- **The app is a client and a local runner.** It talks to the server through the same API as the web app, from types generated from the OpenAPI document. It may be the "This machine" backend where agents propose, with the same short-lived, read-only, scoped credentials. Measuring never runs on the laptop.
- **The runner is Rust.** The same `ratchet-runner`, bundled as a helper. Swift starts and supervises it; it does not reimplement it.
- **Generated, not hand-written.** Colours come from `design/tokens.toml` and words from the words catalogue, written as Swift by the generator. A literal colour or string in a view is a finding.
- **Share behaviour, words and element ids with the web app, never pixels.** Use the system's chrome: window, toolbar, sidebar, Settings window, notifications. Each view sets the same accessibility identifiers as the web element ids.
- **Sign-in is OpenID Connect in the browser.** The app never shows a password field and holds no long-lived cloud or holdout credential. Tokens live in the Keychain.
- **Swift 6 strict concurrency.** UI on the main actor, no data races, no force unwraps in app code.
- **Check (target).** Build and test with `xcodebuild`; add the exact commands here with the first code.
