import { type FormEvent, useState } from "react";
import type { Schemas } from "../../api/client";
import { useSetSessionBudget } from "../../api/queries";
import { Dialog } from "../../components/dialog/Dialog";
import { reason } from "../../components/state/State";
import { parseDollars } from "../../lib/format";
import { w } from "../../words";

export function BudgetDialog(props: {
  open: boolean;
  onClose: () => void;
  session: Schemas["Session"];
}) {
  return (
    <Dialog
      open={props.open}
      onClose={props.onClose}
      title={w.experiment.budgetSave}
      description={w.experiment.budgetLower}
    >
      <Form session={props.session} onClose={props.onClose} />
    </Dialog>
  );
}

function Form(props: { session: Schemas["Session"]; onClose: () => void }) {
  const set = useSetSessionBudget(props.session.id);
  const [text, setText] = useState(
    props.session.budgetCapCents == null ? "" : String(props.session.budgetCapCents / 100),
  );
  const cents = parseDollars(text);
  const submit = async (event: FormEvent) => {
    event.preventDefault();
    if (cents === null) return;
    await set.mutateAsync(cents);
    props.onClose();
  };
  return (
    <form className="form-col" onSubmit={(event) => void submit(event).catch(() => undefined)}>
      <label className="field">
        {w.experiment.budgetNew}
        <input
          className="input"
          inputMode="decimal"
          value={text}
          onChange={(event) => setText(event.target.value)}
        />
      </label>
      {set.isError ? (
        <p className="error-line" role="alert">
          {reason(set.error)}
        </p>
      ) : null}
      <button type="submit" className="btn primary" disabled={cents === null || set.isPending}>
        {w.experiment.budgetSave}
      </button>
    </form>
  );
}
