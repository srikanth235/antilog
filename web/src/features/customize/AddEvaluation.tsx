import { Link, useNavigate } from "@tanstack/react-router";
import { type FormEvent, useState } from "react";
import { ApiFailure } from "../../api/client";
import { useAddEvaluation } from "../../api/queries";
import { Icon } from "../../components/icon/Icon";
import { Select } from "../../components/select/Select";
import { reason } from "../../components/state/State";
import { w } from "../../words";

type Kind = "speed" | "quality" | "size" | "cost";

/** Adds an evaluation template. The sealed experiment keeps its own copy, so editing one later changes nothing sealed. */
export function AddEvaluation() {
  const c = w.customize;
  const add = useAddEvaluation();
  const navigate = useNavigate();
  const [name, setName] = useState("");
  const [summary, setSummary] = useState("");
  const [kind, setKind] = useState<Kind>("speed");
  const [metric, setMetric] = useState("");
  const [lower, setLower] = useState<"lower" | "higher">("lower");
  const [command, setCommand] = useState("");
  const [paths, setPaths] = useState("");
  const [error, setError] = useState<string | null>(null);

  const save = async (event: FormEvent) => {
    event.preventDefault();
    setError(null);
    try {
      const made = await add.mutateAsync({
        name,
        summary,
        kind,
        metric,
        lowerIsBetter: lower === "lower",
        command,
        protectedPaths: paths
          .split("\n")
          .map((line) => line.trim())
          .filter((line) => line !== ""),
      });
      await navigate({ to: "/customize/evaluations/$id", params: { id: made.id } });
    } catch (caught) {
      setError(caught instanceof ApiFailure && caught.status === 409 ? c.exists : reason(caught));
    }
  };

  return (
    <section className="page">
      <Link className="textbtn" to="/customize/evaluations" style={{ alignSelf: "flex-start" }}>
        <Icon name="back" size="s" />
        {c.back}
      </Link>
      <h1 className="t-title">{c.addTitle}</h1>
      <form className="form-col" style={{ maxWidth: 560 }} onSubmit={(event) => void save(event)}>
        <label className="field">
          {c.addName}
          <input
            className="input"
            required
            value={name}
            onChange={(e) => setName(e.target.value)}
          />
        </label>
        <label className="field">
          {c.addSummary}
          <input className="input" value={summary} onChange={(e) => setSummary(e.target.value)} />
        </label>
        <div className="field">
          <span>{c.addKind}</span>
          <Select
            label={c.addKind}
            value={kind}
            options={(["speed", "quality", "size", "cost"] as const).map((value) => ({
              value,
              label: c.kinds[value],
            }))}
            onChange={setKind}
          />
        </div>
        <label className="field">
          {c.addMetric}
          <input
            className="input mono"
            required
            value={metric}
            onChange={(e) => setMetric(e.target.value)}
          />
        </label>
        <div className="field">
          <span>{c.addDirection}</span>
          <Select
            label={c.addDirection}
            value={lower}
            options={[
              { value: "lower", label: c.lower },
              { value: "higher", label: c.higher },
            ]}
            onChange={setLower}
          />
        </div>
        <label className="field">
          {c.addCommand}
          <input
            className="input mono"
            required
            value={command}
            onChange={(e) => setCommand(e.target.value)}
          />
        </label>
        <label className="field">
          {c.addPaths}
          <textarea
            className="input mono"
            rows={3}
            placeholder={c.addPathsHint}
            value={paths}
            onChange={(e) => setPaths(e.target.value)}
          />
        </label>
        <p className="sm mut">{c.addCheats}</p>
        {error === null ? null : (
          <p className="error-line" role="alert">
            {error}
          </p>
        )}
        <button type="submit" className="btn primary" disabled={add.isPending}>
          {c.addSave}
        </button>
      </form>
    </section>
  );
}
