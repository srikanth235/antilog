import { Link } from "@tanstack/react-router";
import { useState } from "react";
import { Icon } from "../../components/icon/Icon";
import { Select } from "../../components/select/Select";
import { w } from "../../words";
import type { EvaluationKind } from "./stub";

/** The form is the board's; saving needs an endpoint the API does not serve yet. (target) */
export function AddEvaluation() {
  const [kind, setKind] = useState<EvaluationKind>("Speed");
  const c = w.customize;
  return (
    <section className="page">
      <Link className="textbtn" to="/customize/evaluations" style={{ alignSelf: "flex-start" }}>
        <Icon name="back" size="s" />
        {c.back}
      </Link>
      <h1 className="t-title">{c.addTitle}</h1>
      <form
        className="form-col"
        style={{ maxWidth: 560 }}
        onSubmit={(event) => event.preventDefault()}
      >
        <label className="field">
          {c.addName}
          <input className="input" />
        </label>
        <div className="field">
          <span>{c.addKind}</span>
          <Select
            label={c.addKind}
            value={kind}
            options={(["Speed", "Quality", "Size", "Cost"] as const).map((value) => ({
              value,
              label: c.kinds[value],
            }))}
            onChange={setKind}
          />
        </div>
        <label className="field">
          {c.addCommand}
          <input className="input mono" />
        </label>
        <label className="field">
          {c.addPaths}
          <input className="input mono" />
        </label>
        <fieldset className="field" style={{ border: 0, padding: 0 }}>
          <legend>{c.addCheats}</legend>
          {Object.values(c.cheats).map((label) => (
            <label key={label} className="rw g8">
              <input type="checkbox" defaultChecked /> {label}
            </label>
          ))}
        </fieldset>
        <p className="target-note">{c.addNote}</p>
        <button type="submit" className="btn primary" disabled>
          {c.addSave}
        </button>
      </form>
    </section>
  );
}
