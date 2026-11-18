import { ApiFailure, api, unwrap } from "../../api/client";
import { rememberIdentity } from "../../lib/identity";
import { readJson, writeJson } from "../../lib/storage";

const ORG_KEY = "antilog.organization.v1";

export function lastOrganization(): string {
  const raw = readJson(ORG_KEY);
  return typeof raw === "string" ? raw : "";
}

export type SignInFailure = "refused" | "noaccess";

export function failureOf(error: unknown): SignInFailure {
  return error instanceof ApiFailure && error.status === 403 ? "noaccess" : "refused";
}

/** Sends the browser to the provider. It comes back to /auth/<provider>/callback. */
export async function startRedirect(provider: string, organization: string): Promise<void> {
  writeJson(ORG_KEY, organization.trim() === "" ? null : organization.trim());
  const start = unwrap(await api.GET("/auth/{provider}/start", { params: { path: { provider } } }));
  window.location.assign(start.authorizeUrl);
}

function remember(issued: { member: { id: string; organizationId: string } }): void {
  writeJson(ORG_KEY, issued.member.organizationId);
  rememberIdentity({ memberId: issued.member.id, organizationId: issued.member.organizationId });
}

export async function finishRedirect(provider: string, code: string, state: string): Promise<void> {
  const organization = lastOrganization();
  const issued = unwrap(
    await api.POST("/auth/{provider}/callback", {
      params: { path: { provider } },
      body: { code, state, organization },
    }),
  );
  remember(issued);
}

/** The fake provider: a code is "fake:<name>". For local servers and agents; never offered on a real one. */
export async function fakeSignIn(input: {
  name: string;
  organization: string;
  newOrganization: string | null;
}): Promise<void> {
  const start = unwrap(
    await api.GET("/auth/{provider}/start", { params: { path: { provider: "fake" } } }),
  );
  const code = `fake:${input.name}`;
  if (input.newOrganization !== null) {
    const created = unwrap(
      await api.POST("/auth/{provider}/organization", {
        params: { path: { provider: "fake" } },
        body: { name: input.newOrganization, code, state: start.state },
      }),
    );
    remember(created.session);
    return;
  }
  const issued = unwrap(
    await api.POST("/auth/{provider}/callback", {
      params: { path: { provider: "fake" } },
      body: { code, state: start.state, organization: input.organization },
    }),
  );
  remember(issued);
}
