# Accessibility

Source: [SwiftUI Pro](https://github.com/twostraws/SwiftUI-Agent-Skill) by Paul Hudson (MIT, [`../licenses/twostraws-SwiftUI-Agent-Skill-LICENSE`](../licenses/twostraws-SwiftUI-Agent-Skill-LICENSE)), condensed and adapted for a macOS app. The app is macOS only, so iOS-only advice is dropped; macOS points are in [macos.md](macos.md). [`../antilog.md`](../antilog.md) wins where they differ.

- Respect the user's accessibility settings for fonts, colors, animations, and more.
- Do not force specific font sizes. Prefer Dynamic Type (`.font(.body)`, `.font(.headline)`, etc.).
- Read `@Environment(\.dynamicTypeSize)` for the user's text-size setting, rather than soft-deprecated `sizeCategory` and `ContentSizeCategory`. Its `DynamicTypeSize` value supports `isAccessibilitySize`, useful for selecting a layout that accommodates the largest text sizes.
- If you _need_ a custom font size, use `@ScaledMetric` when targeting macOS 15 and earlier. When targeting macOS 26 or later, `.font(.body.scaled(by:))` is also available to get font size adjustment.
- Flag instances where images have unclear or unhelpful VoiceOver readings, e.g. `Image(.newBanner2026)`. If they are decorative, suggest using `Image(decorative:)` or `accessibilityHidden()`, otherwise attach an `accessibilityLabel()`.
- When `@Environment(\.accessibilityReduceMotion)` is true, replace large, motion-based animations with opacity changes instead.
- If buttons have complex or frequently changing labels, recommend using `accessibilityInputLabels()` to provide better Voice Control commands. For example, if a button had a live-updating share price for Apple such as "AAPL $271.68", adding an input label for "Apple" would be a big improvement.
- Buttons with image labels must always include text, even if the text is invisible: `Button("Label", systemImage: "plus", action: myAction)`. Flag icon-only buttons that lack a text label as being bad for VoiceOver. Usually SwiftUI will make labels use the correct label style based on their context - e.g. buttons in toolbars will often be icon-only by default - but if there's a specific reason for a button to remain visually icon-only, apply `.labelStyle(.iconOnly)` to preserve the visual while keeping the text available for VoiceOver.
- Give `Menu` a text title as well as an icon: `Menu("Options", systemImage: "ellipsis.circle") { }` is much better than an image alone, because VoiceOver reads the title. In the rare case where the menu trigger should really display only the icon, apply `.labelStyle(.iconOnly)`.
- If color is an important differentiator in the user interface, make sure to respect the environment's `.accessibilityDifferentiateWithoutColor` setting by showing some kind of variation beyond just color - icons, patterns, strokes, etc.
- Never use `onTapGesture()` unless you specifically need tap location or tap count. All other tappable elements should be a `Button`.
- If `onTapGesture()` must be used, make sure to add `.accessibilityAddTraits(.isButton)` or similar so it can be read by VoiceOver correctly.
- The app must work from the keyboard alone: every control reachable with Tab or Full Keyboard Access, a visible focus ring, and a keyboard shortcut for the common commands (see [macos.md](macos.md)).
