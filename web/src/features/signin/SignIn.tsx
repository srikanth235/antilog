import { useQueryClient } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";
import { type FormEvent, useState } from "react";
import { useProviders } from "../../api/queries";
import { Mark } from "../../components/brand/Brand";
import { Icon } from "../../components/icon/Icon";
import { w } from "../../words";
import { failureOf, fakeSignIn, startRedirect } from "./signin";

export function SignIn({ failure }: { failure: "refused" | "noaccess" | undefined }) {
  const providers = useProviders();
  const navigate = useNavigate();
  const client = useQueryClient();
  const [error, setError] = useState<"refused" | "noaccess" | undefined>(failure);
  const names = providers.data ?? [];

  const redirect = async (provider: string) => {
    setError(undefined);
    try {
      await startRedirect(provider);
    } catch (caught) {
      setError(failureOf(caught));
    }
  };

  const done = async () => {
    await client.invalidateQueries();
    await navigate({ to: "/" });
  };

  if (error === "noaccess") {
    return (
      <main className="main" style={{ alignItems: "center", justifyContent: "center" }}>
        <div className="card" style={cardStyle}>
          <Heading title={w.signIn.noAccessTitle} lead={w.signIn.noAccess(window.location.host)} />
          <div className="col" style={{ gap: 12 }}>
            <button
              type="button"
              className="btn primary full"
              style={{ minHeight: 48 }}
              onClick={() => setError(undefined)}
            >
              {w.signIn.anotherAccount}
            </button>
          </div>
        </div>
      </main>
    );
  }

  return (
    <main className="main" style={{ alignItems: "center", justifyContent: "center" }}>
      <div className="card" style={cardStyle}>
        <Heading title={w.signIn.title} lead={w.signIn.purpose} />
        <div className="col" style={{ gap: 12 }}>
          {names.includes("github") ? (
            <button
              type="button"
              className="btn primary full"
              style={{ minHeight: 48 }}
              onClick={() => void redirect("github")}
            >
              <svg
                aria-hidden="true"
                width="18"
                height="18"
                viewBox="0 0 24 24"
                fill="currentColor"
              >
                <path d="M12 2a10 10 0 0 0-3.16 19.49c.5.09.68-.22.68-.48v-1.7c-2.78.6-3.37-1.34-3.37-1.34-.45-1.16-1.11-1.47-1.11-1.47-.91-.62.07-.6.07-.6 1 .07 1.53 1.03 1.53 1.03.89 1.53 2.34 1.09 2.91.83.09-.65.35-1.09.63-1.34-2.22-.25-4.56-1.11-4.56-4.94 0-1.09.39-1.98 1.03-2.68-.1-.25-.45-1.27.1-2.65 0 0 .84-.27 2.75 1.02A9.6 9.6 0 0 1 12 6.84c.85 0 1.71.11 2.51.34 1.91-1.29 2.75-1.02 2.75-1.02.55 1.38.2 2.4.1 2.65.64.7 1.03 1.59 1.03 2.68 0 3.84-2.34 4.68-4.57 4.93.36.31.68.92.68 1.85v2.74c0 .27.18.58.69.48A10 10 0 0 0 12 2z" />
              </svg>
              {w.signIn.github}
            </button>
          ) : null}
          {names.includes("google") ? (
            <button
              type="button"
              className="btn full"
              style={{ minHeight: 48 }}
              onClick={() => void redirect("google")}
            >
              <span className="lg lg-google" role="img" aria-label="Google" />
              {w.signIn.google}
            </button>
          ) : null}
          {providers.isSuccess &&
          !names.some((name) => name !== "fake") &&
          !names.includes("fake") ? (
            <p className="t-body ink2">{w.signIn.noProviders}</p>
          ) : null}
        </div>
        {names.includes("fake") ? <FakeForm onDone={done} onFail={setError} /> : null}
        {error === "refused" ? (
          <p
            className="rw g8 sm"
            role="alert"
            style={{ color: "var(--bad)", alignItems: "flex-start", margin: 0 }}
          >
            <Icon name="lock" size="s" />
            <span>
              <span style={{ fontWeight: 500 }}>{w.signIn.refused}</span> {w.signIn.refusedDetail}
            </span>
          </p>
        ) : null}
      </div>
    </main>
  );
}

const cardStyle = {
  width: 480,
  maxWidth: "calc(100% - 32px)",
  padding: 40,
  display: "flex",
  flexDirection: "column",
  gap: 28,
  alignItems: "stretch",
} as const;

function Heading({ title, lead }: { title: string; lead: string }) {
  return (
    <div className="col" style={{ gap: 20, alignItems: "center", textAlign: "center" }}>
      <Mark width={60} />
      <div className="col" style={{ gap: 8 }}>
        <h1 className="t-title">{title}</h1>
        <p className="t-body ink2">{lead}</p>
      </div>
    </div>
  );
}

function FakeForm(props: {
  onDone: () => Promise<void>;
  onFail: (failure: "refused" | "noaccess") => void;
}) {
  const [name, setName] = useState("");
  const [create, setCreate] = useState(false);
  const [orgName, setOrgName] = useState("");

  const submit = async (event: FormEvent) => {
    event.preventDefault();
    try {
      await fakeSignIn({ name, newOrganization: create ? orgName : null });
      await props.onDone();
    } catch (caught) {
      props.onFail(failureOf(caught));
    }
  };

  return (
    <form
      className="form-col"
      onSubmit={(event) => void submit(event)}
      aria-label={w.signIn.testAccount}
    >
      <h2 className="t-label">{w.signIn.testAccount}</h2>
      <p className="target-note">{w.signIn.testAccountHint}</p>
      <label className="field">
        {w.signIn.testName}
        <input className="input" required value={name} onChange={(e) => setName(e.target.value)} />
      </label>
      <label className="rw g8">
        <input type="checkbox" checked={create} onChange={(e) => setCreate(e.target.checked)} />
        {w.signIn.testNew}
      </label>
      {create ? (
        <label className="field">
          {w.signIn.testOrgName}
          <input
            className="input"
            required
            value={orgName}
            onChange={(e) => setOrgName(e.target.value)}
          />
        </label>
      ) : null}
      <button type="submit" className="btn full">
        {w.signIn.testGo}
      </button>
    </form>
  );
}
