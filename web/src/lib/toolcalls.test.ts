import { describe, expect, it } from "vitest";
import type { Schemas } from "../api/client";
import { type Block, failedCount, groupEvents, groupTitle, rowTitle } from "./toolcalls";

type Event = Schemas["SessionEvent"];

let seq = 0;
const say = (
  kind: Event["kind"],
  text: string,
  tool?: Partial<NonNullable<Event["tool"]>>,
): Event =>
  ({
    seq: ++seq,
    sessionId: "s",
    kind,
    memberId: null,
    text,
    at: seq,
    tool:
      kind === "tool" ? { label: text, status: "done", input: null, output: null, ...tool } : null,
  }) as Event;
const bash = (command: string, failed = false) =>
  say("tool", "bash", { label: command, status: failed ? "failed" : "done" });

const groups = (blocks: Block[]) => blocks.flatMap((b) => (b.type === "tools" ? [b.group] : []));

describe("tool call groups", () => {
  it("folds consecutive tool events into one group and a message splits groups", () => {
    const blocks = groupEvents([
      say("agent", "Reading."),
      bash("ls"),
      bash("pwd"),
      say("agent", "Now fixing."),
      bash("cargo test"),
      say("person", "thanks"),
    ]);
    expect(blocks.map((b) => b.type)).toEqual(["event", "tools", "event", "tools", "event"]);
    expect(groups(blocks).map((g) => g.calls.length)).toEqual([2, 1]);
  });

  it("counts a failure in the title and marks the call", () => {
    const [group] = groups(groupEvents([bash("ls"), bash("cargo test", true), bash("pwd")]));
    expect(group?.calls.map((c) => c.failed)).toEqual([false, true, false]);
    expect(failedCount(group?.calls ?? [])).toBe(1);
    expect(groupTitle(group?.calls ?? [], false)).toBe("Ran 3 commands");
  });

  it("names a group by what it did, in plain words", () => {
    const title = (events: Event[], running = false) =>
      groupTitle(groups(groupEvents(events))[0]?.calls ?? [], running);
    expect(title([bash("ls")])).toBe("Ran 1 command");
    expect(title([say("tool", "edit"), say("tool", "write")])).toBe("Edited 2 files");
    expect(title([say("tool", "read")])).toBe("Read 1 file");
    expect(title([say("tool", "seal"), say("tool", "seal"), say("tool", "seal")])).toBe(
      "Called seal 3 times",
    );
    expect(title([say("tool", "seal")])).toBe("Called seal");
    expect(title([bash("ls"), say("tool", "edit"), say("tool", "seal"), say("tool", "grep")])).toBe(
      "Ran 4 steps",
    );
    expect(title([bash("ls"), bash("pwd")], true)).toBe("Running… 2 commands");
  });

  it("shows the Antilog name once, whichever way it was recorded", () => {
    const [group] = groups(
      groupEvents([
        say("tool", "seal"),
        say("tool", "antilog_seal"),
        say("tool", "antilog_measure"),
      ]),
    );
    expect(group?.calls.map((c) => c.name)).toEqual(["seal", "measure"]);
    const [alone] = groups(groupEvents([say("tool", "antilog_history")]));
    expect(alone?.calls.map((c) => c.name)).toEqual(["history"]);
  });

  it("reads an old call that carries only its name", () => {
    const old = { ...say("tool", "bash"), tool: null } as Event;
    const [group] = groups(groupEvents([old]));
    expect(group?.calls[0]).toMatchObject({
      label: "bash",
      failed: false,
      input: null,
      output: null,
    });
  });

  it("puts a verb before each row", () => {
    const [group] = groups(
      groupEvents([
        bash("cargo test\n--all"),
        say("tool", "edit", { label: "src/a.rs" }),
        say("tool", "grep", { label: "TODO" }),
      ]),
    );
    expect((group?.calls ?? []).map(rowTitle)).toEqual([
      { verb: "Ran", label: "cargo test\n--all" },
      { verb: "Edited", label: "src/a.rs" },
      { verb: "Searched", label: "TODO" },
    ]);
  });
});
