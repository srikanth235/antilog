import { Link, useNavigate } from "@tanstack/react-router";
import { type ChangeEvent, type FormEvent, useState } from "react";
import type { Schemas } from "../../api/client";
import { useAddPlugin } from "../../api/queries";
import { Icon } from "../../components/icon/Icon";
import { reason } from "../../components/state/State";
import { w } from "../../words";

const p = w.plugin;

/** The manifest, or null when the text is not a JSON object with a name and a version. */
function parse(text: string): Schemas["NewPlugin"] | null {
  try {
    const value: unknown = JSON.parse(text);
    if (typeof value !== "object" || value === null || Array.isArray(value)) return null;
    const { name, version } = value as { name?: unknown; version?: unknown };
    return typeof name === "string" && typeof version === "string"
      ? (value as Schemas["NewPlugin"])
      : null;
  } catch {
    return null;
  }
}

/** Adds an Antilog plugin from a plugin.json, pasted or chosen. Its parts land in their own tabs. */
export function AddPlugin() {
  const add = useAddPlugin();
  const navigate = useNavigate();
  const [text, setText] = useState("");
  const [error, setError] = useState<string | null>(null);

  const choose = async (event: ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0];
    if (file !== undefined) setText(await file.text());
  };

  const save = async (event: FormEvent) => {
    event.preventDefault();
    setError(null);
    const manifest = parse(text);
    if (manifest === null) {
      setError(p.invalid);
      return;
    }
    try {
      const made = await add.mutateAsync(manifest);
      await navigate({ to: "/customize/plugins/$id", params: { id: made.id } });
    } catch (caught) {
      setError(reason(caught));
    }
  };

  return (
    <section className="page">
      <Link className="textbtn" to="/customize/plugins" style={{ alignSelf: "flex-start" }}>
        <Icon name="back" size="s" />
        {p.back}
      </Link>
      <h1 className="t-title">{p.addTitle}</h1>
      <p className="ink2" style={{ maxWidth: 560 }}>
        {p.addLead}
      </p>
      <form className="form-col" style={{ maxWidth: 560 }} onSubmit={(event) => void save(event)}>
        <label className="field">
          {p.paste}
          <textarea
            className="input mono"
            rows={10}
            value={text}
            onChange={(event) => setText(event.target.value)}
          />
        </label>
        <label className="field">
          {p.choose}
          <input
            className="input"
            type="file"
            accept="application/json,.json"
            onChange={(event) => void choose(event)}
          />
        </label>
        {error === null ? null : (
          <p className="error-line" role="alert">
            {error}
          </p>
        )}
        <button type="submit" className="btn primary" disabled={add.isPending}>
          {p.save}
        </button>
      </form>
    </section>
  );
}
