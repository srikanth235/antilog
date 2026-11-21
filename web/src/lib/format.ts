export function dollars(cents: number): string {
  const whole = cents % 100 === 0;
  return new Intl.NumberFormat("en-US", {
    style: "currency",
    currency: "USD",
    minimumFractionDigits: whole ? 0 : 2,
    maximumFractionDigits: 2,
  }).format(cents / 100);
}

export function plural(count: number, one: string, many = `${one}s`): string {
  return `${count} ${count === 1 ? one : many}`;
}

export function initial(name: string): string {
  return name.trim().charAt(0).toUpperCase() || "?";
}

const dateFormat = new Intl.DateTimeFormat("en-US", {
  month: "short",
  day: "numeric",
  year: "numeric",
});
const timeFormat = new Intl.DateTimeFormat("en-US", {
  hour: "2-digit",
  minute: "2-digit",
  hour12: false,
});

export function date(ms: number): string {
  return dateFormat.format(new Date(ms));
}
export function time(ms: number): string {
  return timeFormat.format(new Date(ms));
}
export function dateTime(ms: number): string {
  return `${date(ms)}, ${time(ms)}`;
}

/** "5 min ago", "3 h ago", or the date. */
export function ago(ms: number, now: number): string {
  const minutes = Math.max(0, Math.round((now - ms) / 60000));
  if (minutes < 1) return "just now";
  if (minutes < 60) return `${minutes} min ago`;
  const hours = Math.round(minutes / 60);
  if (hours < 24) return `${hours} h ago`;
  return date(ms);
}

/** Cents a person typed as dollars ("12.5"), or null when it is not a number of dollars. */
export function parseDollars(text: string): number | null {
  const value = Number(text.replace(/[$,\s]/g, ""));
  if (!Number.isFinite(value) || value < 0 || text.trim() === "") return null;
  return Math.round(value * 100);
}

/** Share of a budget used, 0 to 1. */
export function share(spent: number, cap: number): number {
  return cap <= 0 ? 1 : Math.min(1, spent / cap);
}
