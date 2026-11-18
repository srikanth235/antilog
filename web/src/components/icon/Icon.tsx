// One mask sprite (design/mockups/icons.css), coloured by currentColor.
export type IconName =
  | "archive"
  | "arrow"
  | "back"
  | "bell"
  | "chart"
  | "check"
  | "chev"
  | "close"
  | "copy"
  | "device"
  | "diff"
  | "dl"
  | "edit"
  | "ext"
  | "filter"
  | "flag"
  | "folder"
  | "globe"
  | "kbd"
  | "key"
  | "list"
  | "lock"
  | "mic"
  | "moon"
  | "more"
  | "org"
  | "out"
  | "pin"
  | "plus"
  | "reload"
  | "return"
  | "search"
  | "share"
  | "shell"
  | "sliders"
  | "sort"
  | "sun"
  | "users"
  | "verify";

export type GlyphName =
  | "blocks"
  | "book"
  | "bot"
  | "box"
  | "chart"
  | "cloud"
  | "code"
  | "db"
  | "file"
  | "gauge"
  | "git"
  | "hist"
  | "note"
  | "spark"
  | "term"
  | "user";

export function Icon({ name, size }: { name: IconName; size?: "s" | "l" }) {
  return <i aria-hidden="true" className={`ic ${size ?? ""} i-${name}`.replace("  ", " ")} />;
}

export function Glyph({ name, size }: { name: GlyphName; size?: "s" | "l" }) {
  return <i aria-hidden="true" className={`ic ${size ?? ""} g-${name}`.replace("  ", " ")} />;
}

export type Activity = "Working" | "WaitingForYou" | "Done" | "Failed";

const STATUS: Record<Activity, { cls: string; color: string; label: string }> = {
  Working: { cls: "st-work run", color: "var(--accent)", label: "Working" },
  WaitingForYou: { cls: "st-wait", color: "var(--warn)", label: "Waiting for you" },
  Done: { cls: "st-done", color: "var(--good)", label: "Done" },
  Failed: { cls: "st-fail", color: "var(--bad)", label: "Failed" },
};

/** An activity as a shape and a word, never colour alone. */
export function StatusIcon({ activity }: { activity: Activity }) {
  const s = STATUS[activity];
  return (
    <span
      role="img"
      aria-label={s.label}
      title={s.label}
      className={`st gi ${s.cls}`}
      style={{ color: s.color }}
    />
  );
}

export function activityColor(activity: Activity): string {
  return STATUS[activity].color;
}
