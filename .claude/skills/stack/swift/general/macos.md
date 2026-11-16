# macOS specifics for a SwiftUI app

Source: Ratchet's own notes from Apple's SwiftUI and AppKit documentation, kept here because the digests next to this file ([swift.md](swift.md), [views.md](views.md)) cover iOS and macOS together and say little about the Mac shell. [`../ratchet.md`](../ratchet.md) wins where they differ. Check each API against the deployment target before use.

## Scenes and windows

- The app is a set of scenes in `App.body`. Use `WindowGroup` for the main window, `Window` for a single-instance utility window (one only, reopened rather than duplicated), `Settings` for the preferences window and `MenuBarExtra` for a menu bar item.
- Window-specific state (selection, navigation path) lives in the scene root with `@State` or `@SceneStorage`. Two windows of the same app must not share it. State that is truly app-wide lives in one `@Observable` model passed through the environment.
- Open and close windows with the `openWindow` and `dismissWindow` environment actions, by id. Do not create `NSWindow` by hand unless SwiftUI has no API for it.
- Set a sensible minimum size with `.frame(minWidth:minHeight:)` on the root view and let the window resize. Use `.windowResizability` for fixed-size windows such as an about box.
- Use the system chrome: `NavigationSplitView` for sidebar and detail, `.toolbar` for the toolbar, `.searchable` for search, `.inspector` for a trailing panel, `.sheet` and `.confirmationDialog` for modal steps. Do not draw your own title bar.

## Menu bar item (`MenuBarExtra`)

- `MenuBarExtra("Ratchet", systemImage: "...") { ... }` with `.menuBarExtraStyle(.menu)` gives a plain menu; `.window` gives a small popover-like panel for richer content. Pick `.menu` unless the content needs it.
- A menu bar item is an entry point, not the app. Anything it shows (running experiments, spend) must also be reachable from the main window.
- Its label should be a template image so it follows the light and dark menu bar. Do not put a coloured logo there.
- To make an app menu-bar-only, set `LSUIElement` in `Info.plist`; then the Dock icon and main menu are absent, so Settings must be reachable from the menu bar item (`SettingsLink`).

## The Settings scene

- `Settings { ... }` gives the standard window, the Settings menu item and the Cmd-, shortcut for free. Use a `TabView` of panes inside it, each with a label and system image.
- Open it from code with `SettingsLink` or the `openSettings` environment action, not by sending AppKit selectors.
- Settings hold preferences of this Mac. They do not hold organisation data; that is the server's.

## Menus and commands

- Add menu items with `.commands { CommandGroup(...) ; CommandMenu(...) }`. Every action reachable by a toolbar button should also be a menu item; Mac users look in the menus first.
- Give common commands a `.keyboardShortcut(...)` (for example Cmd-N, Cmd-F, Cmd-R) and keep to the system conventions: Cmd-W closes a window, Cmd-Q quits, Cmd-, opens Settings, Cmd-Z undoes. Do not reuse a system shortcut for something else.
- Menu items act on the focused window. Expose the focused window's actions with `focusedSceneValue` or `focusedValue` and read them in the command with `@FocusedValue`, so a command is disabled when it does not apply.
- Context menus (`.contextMenu`) belong on rows and items; every context menu action should exist elsewhere too.

## Keyboard and focus

- Every control is reachable by keyboard. Use `@FocusState` and `.focusable()` for custom controls, `.defaultFocus` to choose the first responder, and `.onKeyPress` or `.keyboardShortcut` for key handling. Return activates the default button (`.keyboardShortcut(.defaultAction)`), Escape cancels (`.cancelAction`).
- Lists and tables support arrow-key selection and type-to-select without extra code when `selection:` is bound. Do not break it with `onTapGesture` on rows.

## Mac-native controls

- Prefer `Table` for multi-column data (sortable columns with `KeyPathComparator`), `Form` with `.formStyle(.grouped)` for settings, `LabeledContent` for label-value rows, and `ControlGroup` and `Picker` styles that exist on the Mac.
- Hover, right-click, drag and drop (`draggable`, `dropDestination`), and multiple selection are normal on the Mac. Provide them where users expect them, with a keyboard route as well.
- Sizes are for a pointer: smaller controls are fine, but keep click targets comfortable and respect the user's sidebar icon size and accent colour. Do not set a fixed accent colour over the system's unless the design requires it.

## AppKit interop

- Reach for AppKit only when SwiftUI has no API: wrap with `NSViewRepresentable` and `NSViewControllerRepresentable`, and use an `NSApplicationDelegateAdaptor` for app lifecycle hooks (`applicationShouldTerminate`, launch at login registration, handling a URL). Keep the adapter small and typed.
- AppKit calls run on the main thread. Keep them `@MainActor`.
- Keep AppKit types out of model code so it stays testable and can be shared.

## Helpers, processes and files

- Start a bundled helper with `Process` or, for a login item or daemon, `SMAppService`. Run it from the app's `Contents/MacOS` or `Contents/Helpers`, never from a path outside the bundle. Supervise it: watch `terminationHandler`, restart with back-off and show a state the user can read.
- Never build a shell command from strings: set `executableURL` and an `arguments` array.
- Keep secrets in the Keychain (`SecItem` APIs), not in `UserDefaults` or files. Use `ASWebAuthenticationSession` for browser sign-in; it needs a presentation context anchor (the key window).
- For files the user chooses, use `fileImporter` and `fileExporter`. A sandboxed app needs security-scoped bookmarks to reopen them later.
- Respond to sleep and wake (`NSWorkspace` notifications) and to the network changing: a long-lived connection must reconnect.
