import { type FormEvent, useState } from "react";
import type { Schemas } from "../../api/client";
import { useSetProjectLimits } from "../../api/queries";
import { Dialog } from "../../components/dialog/Dialog";
import { reason } from "../../components/state/State";
import { parseDollars } from "../../lib/format";
import { w } from "../../words";

function asDollars(cents: number | null | undefined): string {
  return cents == null ? "" : String(cents / 100);
}

export function ProjectBudget(props: {
  open: boolean;
  onClose: () => void;
  project: Schemas["Project"];
}) {
  return (
    <Dialog
      open={props.open}
      onClose={props.onClose}
      title={w.projects.changeBudget}
      description={w.projects.budgetLead}
    >
      <BudgetForm project={props.project} onClose={props.onClose} />
    </Dialog>
  );
}

// Mounted only while the dialog is open, so each opening starts from what the server holds.
function BudgetForm(props: { onClose: () => void; project: Schemas["Project"] }) {
  const set = useSetProjectLimits(props.project.id);
  const [cap, setCap] = useState(asDollars(props.project.monthlyCapCents));
  const [def, setDef] = useState(asDollars(props.project.defaultBudgetCents));
  const bad = (text: string) => text.trim() !== "" && parseDollars(text) === null;

  const submit = async (event: FormEvent) => {
    event.preventDefault();
    await set.mutateAsync({
      monthlyCapCents: cap.trim() === "" ? null : parseDollars(cap),
      defaultBudgetCents: def.trim() === "" ? null : parseDollars(def),
    });
    props.onClose();
  };

  return (
    <form className="form-col" onSubmit={(event) => void submit(event).catch(() => undefined)}>
      <label className="field">
        {w.projects.capLabel}
        <input
          className="input"
          inputMode="decimal"
          value={cap}
          onChange={(event) => setCap(event.target.value)}
        />
        <span className="xs mut">{w.projects.clearCap}</span>
      </label>
      <label className="field">
        {w.projects.defaultBudget}
        <input
          className="input"
          inputMode="decimal"
          value={def}
          onChange={(event) => setDef(event.target.value)}
        />
      </label>
      {set.isError ? (
        <p className="error-line" role="alert">
          {reason(set.error)}
        </p>
      ) : null}
      <button
        type="submit"
        className="btn primary"
        disabled={set.isPending || bad(cap) || bad(def)}
      >
        {w.save}
      </button>
    </form>
  );
}
