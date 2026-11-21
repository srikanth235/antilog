import createClient from "openapi-fetch";
import type { components, paths } from "./schema";

export type Schemas = components["schemas"];

/** What the app calls: its own origin, `/api/*`. The server routes sit at the root behind a proxy. */
export const API_BASE = "/api";

export const api = createClient<paths>({ baseUrl: API_BASE, credentials: "include" });

/** A refused or failed call: the status and the server's own words. */
export class ApiFailure extends Error {
  readonly status: number;
  readonly code: string;
  constructor(status: number, code: string, message: string) {
    super(message);
    this.name = "ApiFailure";
    this.status = status;
    this.code = code;
  }
}

function failure(status: number, error: unknown): ApiFailure {
  if (typeof error === "object" && error !== null) {
    const { code, message } = error as { code?: unknown; message?: unknown };
    if (typeof code === "string" && typeof message === "string") {
      return new ApiFailure(status, code, message);
    }
  }
  return new ApiFailure(status, "error", `The server answered ${status}.`);
}

type Settled<T> = { data?: T; error?: unknown; response: Response };

/** Returns the data of a call or throws the server's reason. */
export function unwrap<T>(result: Settled<T>): T {
  if (result.error !== undefined || result.data === undefined) {
    if (result.response.ok) return undefined as T; // 204: no body
    throw failure(result.response.status, result.error);
  }
  return result.data;
}
