import {
  useConnectors,
  useExperimentConnectors,
  useExperimentSkills,
  useMember,
  useSetExperimentConnectors,
  useSetExperimentSkills,
  useSkills,
} from "../../api/queries";
import { Loading, reason } from "../../components/state/State";
import { canEdit } from "../../lib/domain";
import { w } from "../../words";

const t = w.experimentDetail;

function Choice(props: {
  label: string;
  note?: string | undefined;
  checked: boolean;
  disabled: boolean;
  onChange: (checked: boolean) => void;
}) {
  return (
    <label className="rw g8" style={{ minHeight: 32 }}>
      <input
        type="checkbox"
        checked={props.checked}
        disabled={props.disabled}
        onChange={(event) => props.onChange(event.target.checked)}
      />
      <span>{props.label}</span>
      {props.note === undefined ? null : <span className="mut">{props.note}</span>}
    </label>
  );
}

/** Which skill packs and connectors this experiment uses. A change saves at once. */
export function ExperimentUses({
  experimentId,
  projectId,
}: {
  experimentId: string;
  projectId: string;
}) {
  const me = useMember();
  const editable = canEdit(me?.role);
  const skills = useSkills(projectId);
  const connectors = useConnectors();
  const chosenSkills = useExperimentSkills(experimentId);
  const chosenConnectors = useExperimentConnectors(experimentId);
  const setSkills = useSetExperimentSkills(experimentId);
  const setConnectors = useSetExperimentConnectors(experimentId);
  if (
    skills.isPending ||
    connectors.isPending ||
    chosenSkills.isPending ||
    chosenConnectors.isPending
  )
    return <Loading rows={2} />;
  // While a change is on its way the box already shows it.
  const selectedSkills = new Set(
    setSkills.isPending && setSkills.variables !== undefined
      ? setSkills.variables
      : (chosenSkills.data ?? []).map((ref) => ref.name),
  );
  const selectedConnectors = new Set(
    setConnectors.isPending && setConnectors.variables !== undefined
      ? setConnectors.variables
      : (chosenConnectors.data ?? []),
  );
  // A pack that is Off in the project is not offered, unless the experiment already has it: then it can be cleared.
  const offered = (skills.data ?? []).filter((skill) => skill.on || selectedSkills.has(skill.name));
  const flip = (names: Set<string>, name: string, on: boolean) => {
    const next = new Set(names);
    if (on) next.add(name);
    else next.delete(name);
    return [...next];
  };
  const failure = [setSkills, setConnectors].find((call) => call.isError)?.error;
  return (
    <div className="ds-b">
      <div className="dr">
        <span className="k">{t.skillPacks}</span>
        <span className="v">
          {offered.length === 0 ? <span className="mut">{t.noSkillPacks}</span> : null}
          {offered.map((skill) => (
            <Choice
              key={skill.name}
              label={skill.name}
              note={w.skill.version(skill.latest)}
              checked={selectedSkills.has(skill.name)}
              disabled={!editable}
              onChange={(on) => setSkills.mutate(flip(selectedSkills, skill.name, on))}
            />
          ))}
          <span className="m">{t.skillPacksNote}</span>
        </span>
      </div>
      <div className="dr">
        <span className="k">{t.connectors}</span>
        <span className="v">
          {(connectors.data ?? []).length === 0 ? (
            <span className="mut">{t.noConnectors}</span>
          ) : null}
          {(connectors.data ?? []).map((connector) => (
            <Choice
              key={connector.id}
              label={connector.name}
              note={w.connector.health[connector.health]}
              checked={selectedConnectors.has(connector.id)}
              disabled={!editable}
              onChange={(on) => setConnectors.mutate(flip(selectedConnectors, connector.id, on))}
            />
          ))}
          <span className="m">{t.connectorsNote}</span>
        </span>
      </div>
      {failure === undefined ? null : (
        <p className="error-line" role="alert">
          {reason(failure)}
        </p>
      )}
    </div>
  );
}
