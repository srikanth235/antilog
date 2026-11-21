import { ApiFailure, api, unwrap } from "../../api/client";

export type SignInFailure = "refused" | "noaccess";

export function failureOf(error: unknown): SignInFailure {
  return error instanceof ApiFailure && error.status === 403 ? "noaccess" : "refused";
}

/** Sends the browser to the provider. It comes back to /auth/<provider>/callback. */
export async function startRedirect(provider: string): Promise<void> {
  const start = unwrap(await api.GET("/auth/{provider}/start", { params: { path: { provider } } }));
  window.location.assign(start.authorizeUrl);
}

/** The server finds the person's organization from who the provider says they are. */
export async function finishRedirect(provider: string, code: string, state: string): Promise<void> {
  unwrap(
    await api.POST("/auth/{provider}/callback", {
      params: { path: { provider } },
      body: { code, state },
    }),
  );
}

/** The fake provider: a code is "fake:<name>". For local servers and agents; never offered on a real one. */
export async function fakeSignIn(input: {
  name: string;
  newOrganization: string | null;
}): Promise<void> {
  const start = unwrap(
    await api.GET("/auth/{provider}/start", { params: { path: { provider: "fake" } } }),
  );
  const code = `fake:${input.name}`;
  if (input.newOrganization !== null) {
    unwrap(
      await api.POST("/auth/{provider}/organization", {
        params: { path: { provider: "fake" } },
        body: { name: input.newOrganization, code, state: start.state },
      }),
    );
    return;
  }
  unwrap(
    await api.POST("/auth/{provider}/callback", {
      params: { path: { provider: "fake" } },
      body: { code, state: start.state },
    }),
  );
}
