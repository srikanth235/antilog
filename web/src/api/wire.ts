// The server sends enum values in snake_case ("not_sealed") while openapi/antilog.json declares
// them in PascalCase ("NotSealed"). Until the schema is fixed on the server, the client converts at
// the fetch boundary so the rest of the app sees exactly the generated types. Delete this file when
// the committed schema matches the wire.

const VALUES: Record<string, readonly string[]> = {
  role: ["Viewer", "Editor", "Owner"],
  status: [
    "NotSealed",
    "Sealed",
    "Working",
    "WaitingForYou",
    "Done",
    "Failed",
    "Pending",
    "Approved",
    "Declined",
    "ApprovedByPolicy",
  ],
  kind: ["Person", "Service"],
  card: [
    "SealExperiment",
    "RaiseBudget",
    "AddCredential",
    "AttachRepo",
    "ChangeEnvironment",
    "OpenPullRequest",
    "KeepGoing",
    "ContinueOnAnotherAgent",
  ],
  mode: ["AlwaysAsk", "AllowUpTo", "Allow"],
  decision: ["Approve", "Decline"],
  outcome: ["Done", "Denied", "Approved", "Declined", "ApprovedByPolicy"],
  cleanup: ["Removed", "NotRemoved"],
};

/** Keys whose values are free-form (a tool's input and answer): never converted. */
const OPAQUE = new Set(["input", "answer", "inputSchema"]);

export function toSnake(pascal: string): string {
  return pascal.replace(/(?<!^)([A-Z])/g, "_$1").toLowerCase();
}

export function toPascal(snake: string): string {
  return snake
    .split("_")
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join("");
}

type Direction = "in" | "out";

function convert(value: unknown, direction: Direction, deep: boolean): unknown {
  if (Array.isArray(value)) return value.map((item) => convert(item, direction, deep));
  if (typeof value !== "object" || value === null) return value;
  const result: Record<string, unknown> = {};
  for (const [key, item] of Object.entries(value)) {
    const allowed = VALUES[key];
    if (OPAQUE.has(key)) {
      result[key] = item;
    } else if (allowed !== undefined && typeof item === "string") {
      if (direction === "in") {
        const pascal = toPascal(item);
        result[key] = allowed.includes(pascal) ? pascal : item;
      } else {
        result[key] = allowed.includes(item) ? toSnake(item) : item;
      }
    } else {
      result[key] = deep ? convert(item, direction, deep) : item;
    }
  }
  return result;
}

/** A response body from the server, as the generated types describe it. */
export function fromWire(value: unknown): unknown {
  return convert(value, "in", true);
}

/** A request body, as the server reads it. Only the top level and arrays of objects carry enums. */
export function toWire(value: unknown): unknown {
  return convert(value, "out", true);
}
