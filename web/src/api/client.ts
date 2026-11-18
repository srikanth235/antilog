import createClient, { type Middleware } from "openapi-fetch";
import type { ApiPaths } from "./overrides";
import type { components } from "./schema";
import { fromWire, toWire } from "./wire";

export type Schemas = components["schemas"];

/** What the app calls: its own origin, `/api/*`. The server routes sit at the root behind a proxy. */
export const API_BASE = "/api";

const wire: Middleware = {
  async onRequest({ request }) {
    const type = request.headers.get("content-type") ?? "";
    if (!type.includes("json")) return request;
    const text = await request.clone().text();
    if (text === "") return request;
    return new Request(request, { body: JSON.stringify(toWire(JSON.parse(text) as unknown)) });
  },
  async onResponse({ response }) {
    const type = response.headers.get("content-type") ?? "";
    if (!type.includes("json")) return response;
    const text = await response.clone().text();
    if (text === "") return response;
    return new Response(JSON.stringify(fromWire(JSON.parse(text) as unknown)), {
      status: response.status,
      statusText: response.statusText,
      headers: response.headers,
    });
  },
};

export const api = createClient<ApiPaths>({ baseUrl: API_BASE, credentials: "include" });
api.use(wire);

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

/**
 * `GET /audit` and `GET /tokens` take query parameters, but the committed schema files them as path
 * parameters, so the generated types cannot express them. This one cast hands them over as a query;
 * remove it when the schema says `in: query`.
 */
export function withQuery(query: Record<string, string | number | null>): never {
  const clean: Record<string, string | number> = {};
  for (const [key, value] of Object.entries(query)) if (value !== null) clean[key] = value;
  return { query: clean, path: {} } as never;
}
