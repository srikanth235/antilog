import { expect, test } from "vitest";
import { parse, refetchable, staleBy } from "./live";
import { keys } from "./queries";

test("a message refetches its conversation and nothing else", () => {
  expect(staleBy({ type: "message", sessionId: "s1", experimentId: "e1", seq: 4 })).toEqual([
    keys.transcript("s1"),
  ]);
});

test("a session change refetches its experiment's sessions and the cards list", () => {
  const stale = staleBy({ type: "session", sessionId: "s1", experimentId: "e1", status: "done" });
  expect(stale).toContainEqual(keys.sessions("e1"));
  expect(stale).toContainEqual(keys.pending());
});

test("hello and resync refetch what a screen shows", () => {
  expect(staleBy({ type: "hello", cursor: 3 })).toEqual(staleBy({ type: "resync", cursor: 3 }));
  expect(staleBy({ type: "hello", cursor: 3 })).toContainEqual(["sessions"]);
});

test("cards refetch the pending list", () => {
  expect(staleBy({ type: "cards", pending: 2 })).toContainEqual(keys.pending());
});

test("a half message is ignored", () => {
  expect(parse('{"type":"hello","cursor":1}')).toEqual({ type: "hello", cursor: 1 });
  expect(parse('{"type":"hel')).toBeNull();
  expect(parse("7")).toBeNull();
});

test("the history tool call is never refetched by an event, or it would open a session each time", () => {
  expect(refetchable({ queryKey: keys.history("e1") })).toBe(false);
  expect(refetchable({ queryKey: keys.sessions("e1") })).toBe(true);
});
