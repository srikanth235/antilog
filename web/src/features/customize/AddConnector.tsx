import { Link, useNavigate } from "@tanstack/react-router";
import { type FormEvent, useState } from "react";
import { useAddConnector } from "../../api/queries";
import { Icon } from "../../components/icon/Icon";
import { reason } from "../../components/state/State";
import { w } from "../../words";

const c = w.connector;

/** Adds a remote MCP server. The credential goes to the server once, sealed, and is never shown again. */
export function AddConnector() {
  const add = useAddConnector();
  const navigate = useNavigate();
  const [name, setName] = useState("");
  const [url, setUrl] = useState("");
  const [credential, setCredential] = useState("");
  const [error, setError] = useState<string | null>(null);

  const save = async (event: FormEvent) => {
    event.preventDefault();
    setError(null);
    try {
      const made = await add.mutateAsync({
        name,
        url,
        credential: credential === "" ? null : credential,
      });
      await navigate({ to: "/customize/connectors/$id", params: { id: made.id } });
    } catch (caught) {
      setError(reason(caught));
    }
  };

  return (
    <section className="page">
      <Link className="textbtn" to="/customize/connectors" style={{ alignSelf: "flex-start" }}>
        <Icon name="back" size="s" />
        {c.back}
      </Link>
      <h1 className="t-title">{c.addTitle}</h1>
      <p className="ink2" style={{ maxWidth: 560 }}>
        {c.addLead}
      </p>
      <form className="form-col" style={{ maxWidth: 560 }} onSubmit={(event) => void save(event)}>
        <label className="field">
          {c.name}
          <input
            className="input"
            required
            value={name}
            onChange={(event) => setName(event.target.value)}
          />
          <span className="xs mut">{c.nameHint}</span>
        </label>
        <label className="field">
          {c.url}
          <input
            className="input mono"
            required
            type="url"
            value={url}
            onChange={(event) => setUrl(event.target.value)}
          />
        </label>
        <label className="field">
          {c.optionalCredential}
          <input
            className="input mono"
            type="password"
            autoComplete="off"
            value={credential}
            onChange={(event) => setCredential(event.target.value)}
          />
          <span className="xs mut">{c.optionalHint}</span>
        </label>
        {error === null ? null : (
          <p className="error-line" role="alert">
            {error}
          </p>
        )}
        <button type="submit" className="btn primary" disabled={add.isPending}>
          {c.save}
        </button>
      </form>
    </section>
  );
}
