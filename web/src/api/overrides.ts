// The committed schema has three defects the generated types inherit. This file restates the affected
// routes by hand, from the same schemas, so the client is typed correctly meanwhile. Delete it when the
// server's OpenAPI document is fixed:
//   1. two operations share the id `start` (sign-in start and session start): the second is lost;
//   2. a free-form `object` becomes `Record<string, never>`, so a tool's input cannot be typed;
//   3. `GET /audit` and `GET /tokens` list their query parameters as path parameters (see withQuery).
import type { components, paths } from "./schema";

type S = components["schemas"];
type Json<T> = { content: { "application/json": T } };
type Open = { query?: never; header?: never; cookie?: never };
type Free = Record<string, unknown>;

interface Fixed {
  "/auth/{provider}/start": {
    parameters: { query?: never; header?: never; path?: never; cookie?: never };
    get: {
      parameters: Open & { path: { provider: string } };
      requestBody?: never;
      responses: {
        200: { headers: Record<string, unknown>; content: Json<S["SignInStart"]>["content"] };
      };
    };
  };
  "/experiments/{id}/sessions": {
    parameters: { query?: never; header?: never; path?: never; cookie?: never };
    get: paths["/experiments/{id}/sessions"]["get"];
    post: {
      parameters: Open & { path: { id: string } };
      requestBody: Json<S["StartSession"]>;
      responses: {
        201: { headers: Record<string, unknown>; content: Json<S["Session"]>["content"] };
      };
    };
  };
  "/experiments/{id}/tools/{tool}": {
    parameters: { query?: never; header?: never; path?: never; cookie?: never };
    post: {
      parameters: Open & { path: { id: string; tool: string } };
      requestBody: Json<{ call?: string | null; input: Free; session?: string | null }>;
      responses: {
        200: {
          headers: Record<string, unknown>;
          content: Json<{ answer: Free; followup?: string | null; session: string }>["content"];
        };
      };
    };
  };
}

export type ApiPaths = Omit<paths, keyof Fixed> & Fixed;
