import { type FormEvent, useId, useState } from "react";
import type { Schemas } from "../../api/client";
import { useEvaluations, useMember, useSetValueModel } from "../../api/queries";
import { Dialog } from "../../components/dialog/Dialog";
import { Icon } from "../../components/icon/Icon";
import { reason } from "../../components/state/State";
import {
  checkValueForm,
  readableUnitSize,
  type ValueForm,
  type ValueFormField,
  valueSentence,
} from "../../lib/value";
import { w } from "../../words";

const t = w.addValueModel;

/** The dialog over the Project page. `change` starts the form from a model the project already holds. */
export function AddValueModel(props: {
  open: boolean;
  onClose: () => void;
  project: Pick<Schemas["Project"], "id" | "name">;
  models: readonly Schemas["ProjectValueModel"][];
  change?: Schemas["ProjectValueModel"] | undefined;
}) {
  return (
    <Dialog
      open={props.open}
      onClose={props.onClose}
      title={props.change === undefined ? t.title : t.changeTitle}
      description={t.lead(props.project.name)}
    >
      <ModelForm {...props} />
    </Dialog>
  );
}

function startingForm(change: Schemas["ProjectValueModel"] | undefined): ValueForm {
  if (change === undefined)
    return { metric: "", unit: "", unitSize: "1", dollarsPerYear: "", source: "", link: "" };
  const size = readableUnitSize(change.model.centsPerUnit);
  return {
    metric: change.model.metric,
    unit: change.model.unit,
    unitSize: String(size),
    dollarsPerYear: String(Math.round(change.model.centsPerUnit * size) / 100),
    source: change.model.source.text,
    link: change.model.source.link ?? "",
  };
}

// Mounted only while the dialog is open, so each opening starts from what the server holds.
function ModelForm(props: {
  onClose: () => void;
  project: Pick<Schemas["Project"], "id" | "name">;
  models: readonly Schemas["ProjectValueModel"][];
  change?: Schemas["ProjectValueModel"] | undefined;
}) {
  const set = useSetValueModel(props.project.id);
  const me = useMember();
  const evaluations = useEvaluations();
  const listId = useId();
  const [form, setForm] = useState<ValueForm>(() => startingForm(props.change));
  const [shown, setShown] = useState(false);
  const checked = checkValueForm(form, t.errors);
  const errors = checked.ok ? {} : checked.errors;
  const known = [
    ...new Set([
      ...props.models.map((entry) => entry.model.metric),
      ...(evaluations.data ?? []).map((evaluation) => evaluation.metric),
    ]),
  ];
  const existing = props.models.find((entry) => entry.model.metric === form.metric.trim());
  const sentence = valueSentence(
    form.metric,
    form.unit.trim() === "" ? "units" : form.unit.trim(),
    Number(form.unitSize),
    Number(form.dollarsPerYear.replace(/[$,\s]/g, "")),
  );

  const invalid = (field: ValueFormField) => shown && errors[field] !== undefined;
  const described = (field: ValueFormField) =>
    invalid(field) ? `${listId}-${field}-e` : undefined;
  const edit = (field: ValueFormField) => (event: { target: { value: string } }) =>
    setForm({ ...form, [field]: event.target.value });
  const problem = (field: ValueFormField) =>
    shown && errors[field] !== undefined ? (
      <span className="error-line" id={`${listId}-${field}-e`}>
        {errors[field]}
      </span>
    ) : null;

  const submit = async (event: FormEvent) => {
    event.preventDefault();
    setShown(true);
    if (!checked.ok) return;
    await set.mutateAsync(checked.body);
    props.onClose();
  };

  return (
    <form
      className="form-col"
      noValidate
      onSubmit={(event) => void submit(event).catch(() => undefined)}
    >
      <label className="field">
        {t.metric}
        <input
          className="input"
          aria-label={t.metric}
          list={`${listId}-metrics`}
          aria-invalid={shown && errors.metric !== undefined}
          value={form.metric}
          readOnly={props.change !== undefined}
          onChange={(event) => setForm({ ...form, metric: event.target.value })}
        />
        <datalist id={`${listId}-metrics`}>
          {known.map((metric) => (
            <option key={metric} value={metric} />
          ))}
        </datalist>
        <span className="xs mut">{t.metricHint}</span>
        {problem("metric")}
      </label>
      {existing === undefined || props.change !== undefined ? null : (
        <p className="ap-note" style={{ marginTop: 0 }}>
          <Icon name="verify" size="s" />
          <span>{t.changesExisting(existing.model.metric, existing.model.version + 1)}</span>
        </p>
      )}
      <label className="field">
        {t.unit}
        <input
          className="input"
          aria-label={t.unit}
          placeholder={t.unitPlaceholder}
          aria-invalid={shown && errors.unit !== undefined}
          value={form.unit}
          onChange={(event) => setForm({ ...form, unit: event.target.value })}
        />
        {problem("unit")}
      </label>
      <div className="rw g12" style={{ alignItems: "flex-start", flexWrap: "wrap" }}>
        <label className="field grow" style={{ minWidth: 140 }}>
          {t.oneUnit}
          <input
            className="input mono"
            aria-label={t.oneUnit}
            aria-invalid={invalid("unitSize")}
            aria-describedby={described("unitSize")}
            value={form.unitSize}
            onChange={edit("unitSize")}
          />
          <span className="xs mut">{t.oneUnitHint}</span>
          {problem("unitSize")}
        </label>
        <label className="field grow" style={{ minWidth: 140 }}>
          {t.worth}
          <input
            className="input mono"
            aria-label={t.worth}
            aria-invalid={invalid("dollarsPerYear")}
            aria-describedby={described("dollarsPerYear")}
            value={form.dollarsPerYear}
            onChange={edit("dollarsPerYear")}
          />
          {problem("dollarsPerYear")}
        </label>
      </div>
      {sentence === null ? null : (
        <p className="ap-note" style={{ marginTop: 0 }} aria-live="polite">
          <Icon name="verify" size="s" />
          <span>{sentence}</span>
        </p>
      )}
      <label className="field">
        {t.source}
        <input
          className="input"
          aria-label={t.source}
          aria-invalid={invalid("source")}
          aria-describedby={described("source")}
          value={form.source}
          onChange={edit("source")}
        />
        {problem("source")}
      </label>
      <label className="field">
        <span>
          {t.link} <span className="mut">{t.optional}</span>
        </span>
        <input
          className="input mono"
          aria-label={t.link}
          inputMode="url"
          placeholder={t.linkPlaceholder}
          aria-invalid={shown && errors.link !== undefined}
          value={form.link}
          onChange={(event) => setForm({ ...form, link: event.target.value })}
        />
        {problem("link")}
      </label>
      <div className="dr" style={{ borderTop: "1px solid var(--line)" }}>
        <span className="k">{t.setByLabel}</span>
        <span className="v">
          {t.setBy(me?.name ?? w.you)}
          <span className="m">{t.audit}</span>
        </span>
      </div>
      {set.isError ? (
        <p className="error-line" role="alert">
          {reason(set.error)}
        </p>
      ) : null}
      <div className="rw g12">
        <button type="submit" className="btn primary" disabled={set.isPending}>
          {props.change === undefined ? t.submit : t.submitChange}
        </button>
        <button type="button" className="btn" onClick={props.onClose}>
          {w.cancel}
        </button>
      </div>
    </form>
  );
}
