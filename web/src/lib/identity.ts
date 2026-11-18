import { readJson, useStored, writeJson } from "./storage";

const KEY = "antilog.identity.v1";

export interface Identity {
  memberId: string;
  organizationId: string;
}

function parse(raw: unknown): Identity | null {
  if (typeof raw !== "object" || raw === null) return null;
  const { memberId, organizationId } = raw as Record<string, unknown>;
  if (typeof memberId === "string" && typeof organizationId === "string") {
    return { memberId, organizationId };
  }
  return null;
}

/** Who this browser last signed in as. The session itself is a cookie; this is only a label for the UI. */
export function useIdentity(): Identity | null {
  return useStored(KEY, parse);
}
export function readIdentity(): Identity | null {
  return parse(readJson(KEY));
}
export function rememberIdentity(identity: Identity): void {
  writeJson(KEY, identity);
}
export function forgetIdentity(): void {
  writeJson(KEY, null);
}
