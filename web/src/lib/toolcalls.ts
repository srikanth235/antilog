// A session's tool calls, folded the way Claude Code's cloud session shows them: every run of
// tool calls between two messages is one quiet group, titled in plain words by what it did.

import type { Schemas } from "../api/client";

export type ToolKind = "command" | "edit" | "read" | "tool" | "step";

export interface ToolCallView {
  seq: number;
  /** The tool as the agent knows it, without the MCP entry's prefix. */
  name: string;
  kind: ToolKind;
  /** What the row says after its verb: the command's first line, a path, the tool's name. */
  label: string;
  failed: boolean;
  input: string | null;
  output: string | null;
}

export interface ToolGroupView {
  /** The first call's `seq`, stable while the group grows. */
  key: number;
  calls: ToolCallView[];
}

export type Block =
  | { type: "event"; event: Schemas["SessionEvent"] }
  | { type: "tools"; group: ToolGroupView };

/** OpenCode names the tools of the `antilog` MCP entry `antilog_<tool>`; the server records each of them too. */
const MCP_PREFIX = "antilog_";

const KINDS: Record<string, ToolKind> = {
  bash: "command",
  edit: "edit",
  write: "edit",
  patch: "edit",
  multiedit: "edit",
  read: "read",
};

const VERBS: Record<ToolKind, string> = {
  command: "Ran",
  edit: "Edited",
  read: "Read",
  tool: "Called",
  step: "Ran",
};

export function toolName(text: string): string {
  return text.startsWith(MCP_PREFIX) ? text.slice(MCP_PREFIX.length) : text;
}

export function kindOf(name: string): ToolKind {
  return KINDS[name] ?? (name === "grep" || name === "glob" || name === "list" ? "step" : "tool");
}

function view(event: Schemas["SessionEvent"]): ToolCallView {
  const name = toolName(event.text);
  const kind = kindOf(name);
  const record = event.tool ?? null;
  return {
    seq: event.seq,
    name,
    kind,
    label: record?.label || name,
    failed: record?.status === "failed",
    input: record?.input ?? null,
    output: record?.output ?? null,
  };
}

/** Calls recorded before the prefix was dropped show twice, once as `seal` and once as `antilog_seal`: the second goes. */
function withoutDuplicates(events: Schemas["SessionEvent"][]): Schemas["SessionEvent"][] {
  const plain = new Map<string, number>();
  for (const event of events)
    if (!event.text.startsWith(MCP_PREFIX)) plain.set(event.text, (plain.get(event.text) ?? 0) + 1);
  return events.filter((event) => {
    if (!event.text.startsWith(MCP_PREFIX)) return true;
    const name = toolName(event.text);
    const left = plain.get(name) ?? 0;
    if (left === 0) return true;
    plain.set(name, left - 1);
    return false;
  });
}

/** Consecutive tool events become one group; a person's or an agent's words split groups. */
export function groupEvents(events: Schemas["SessionEvent"][]): Block[] {
  const blocks: Block[] = [];
  let run: Schemas["SessionEvent"][] = [];
  const flush = () => {
    const kept = withoutDuplicates(run);
    const first = kept[0];
    if (first !== undefined)
      blocks.push({ type: "tools", group: { key: first.seq, calls: kept.map(view) } });
    run = [];
  };
  for (const event of events) {
    if (event.kind === "tool") run.push(event);
    else {
      flush();
      blocks.push({ type: "event", event });
    }
  }
  flush();
  return blocks;
}

function plural(count: number, one: string, many: string): string {
  return `${count} ${count === 1 ? one : many}`;
}

export function failedCount(calls: ToolCallView[]): number {
  return calls.filter((call) => call.failed).length;
}

/** "Ran 19 commands", "Edited 2 files", "Called seal 3 times", or "Ran 4 steps" when the kinds differ. Running adds the live count. */
export function groupTitle(calls: ToolCallView[], running: boolean): string {
  const count = calls.length;
  const kinds = new Set(calls.map((call) => call.kind));
  const names = new Set(calls.map((call) => call.name));
  const only = kinds.size === 1 ? [...kinds][0] : undefined;
  if (running) {
    const what =
      only === "command"
        ? plural(count, "command", "commands")
        : only === "edit" || only === "read"
          ? plural(count, "file", "files")
          : plural(count, "step", "steps");
    return `Running… ${what}`;
  }
  if (only === "command") return `Ran ${plural(count, "command", "commands")}`;
  if (only === "edit") return `Edited ${plural(count, "file", "files")}`;
  if (only === "read") return `Read ${plural(count, "file", "files")}`;
  const name = [...names][0];
  if (only === "tool" && names.size === 1 && name !== undefined)
    return count === 1 ? `Called ${name}` : `Called ${name} ${count} times`;
  return `Ran ${plural(count, "step", "steps")}`;
}

/** A row's words: the verb for its kind, then its label. */
export function rowTitle(call: ToolCallView): { verb: string; label: string } {
  if (call.kind === "step" && call.name !== "bash")
    return { verb: call.name === "list" ? "Listed" : "Searched", label: call.label };
  return { verb: VERBS[call.kind], label: call.label };
}
