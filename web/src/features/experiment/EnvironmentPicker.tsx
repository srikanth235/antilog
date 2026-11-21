import { useState } from "react";
import type { Schemas } from "../../api/client";
import { useEnvironments, useSetSessionEnvironment } from "../../api/queries";
import { Dialog } from "../../components/dialog/Dialog";
import { Failed, Loading, reason } from "../../components/state/State";
import { w } from "../../words";

type Environment = Schemas["Environment"];

/**
 * Moves the session to another environment: the newest version of the one picked. The network applies
 * at once; variables, repos and the setup script reach the machine as the Overview says. Editors only.
 */
export function EnvironmentPicker(props: {
  open: boolean;
  onClose: () => void;
  session: Schemas["Session"];
  /** Opens the environment's own edit dialog instead. */
  onEdit: (environment: Environment) => void;
}) {
  const o = w.experiment.overview;
  return (
    <Dialog
      open={props.open}
      onClose={props.onClose}
      title={o.changeEnvironmentTitle}
      description={o.changeEnvironmentLead}
    >
      {props.open ? (
        <Picker session={props.session} onClose={props.onClose} onEdit={props.onEdit} />
      ) : null}
    </Dialog>
  );
}

function Picker(props: {
  session: Schemas["Session"];
  onClose: () => void;
  onEdit: (environment: Environment) => void;
}) {
  const o = w.experiment.overview;
  const t = w.settings.environments;
  const environments = useEnvironments();
  const move = useSetSessionEnvironment(props.session.id);
  const current = props.session.environment ?? "default";
  const [picked, setPicked] = useState(current);

  if (environments.isPending) return <Loading rows={3} />;
  if (environments.isError)
    return <Failed error={environments.error} retry={() => void environments.refetch()} />;
  // An archived environment is not offered, unless it is the one the session is on.
  const offered = environments.data.filter((row) => !row.archived || row.id === current);
  const chosen = offered.find((row) => row.id === picked);
  const same = picked === current && chosen?.version === (props.session.environmentVersion ?? 1);
  return (
    <form
      className="form-col"
      onSubmit={(event) => {
        event.preventDefault();
        void move.mutateAsync(picked).then(props.onClose, () => undefined);
      }}
    >
      <fieldset className="radio-list" style={{ border: 0, padding: 0, margin: 0 }}>
        <legend className="sr-only">{w.experiment.sections.environment}</legend>
        {offered.map((row) => (
          <label key={row.id} className="stg" data-environment={row.id}>
            <span className="tx">
              <span className="t-item">{row.name}</span>
              <span className="xs mut">
                {t.summary(row.network, row.allowedHosts.length, row.allowPackageManagers)}
                {" · "}
                {t.version(row.version)}
                {row.id === current ? ` · ${o.thisSession}` : ""}
              </span>
            </span>
            <span className="ct">
              <input
                type="radio"
                name="environment"
                value={row.id}
                aria-label={row.name}
                checked={picked === row.id}
                onChange={() => setPicked(row.id)}
              />
            </span>
          </label>
        ))}
      </fieldset>
      <p className="xs mut">{o.pickNote}</p>
      {move.isError ? (
        <p className="error-line" role="alert">
          {reason(move.error)}
        </p>
      ) : null}
      <div className="rw g12 sp">
        <button
          type="button"
          className="textbtn"
          disabled={chosen === undefined || chosen.builtIn}
          onClick={() => {
            if (chosen !== undefined) props.onEdit(chosen);
          }}
        >
          {o.editThis}
        </button>
        <span className="rw g12">
          <button type="button" className="btn" onClick={props.onClose}>
            {w.cancel}
          </button>
          <button type="submit" className="btn primary" disabled={same || move.isPending}>
            {o.move}
          </button>
        </span>
      </div>
    </form>
  );
}
