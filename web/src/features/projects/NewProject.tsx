import { type FormEvent, useState } from "react";
import { useCreateProject } from "../../api/queries";
import { Dialog } from "../../components/dialog/Dialog";
import { reason } from "../../components/state/State";
import { w } from "../../words";

export function NewProject(props: {
  open: boolean;
  onClose: () => void;
  onCreated?: (id: string) => void;
}) {
  return (
    <Dialog
      open={props.open}
      onClose={props.onClose}
      title={w.projects.newProjectTitle}
      size="narrow"
    >
      <Form onClose={props.onClose} onCreated={props.onCreated} />
    </Dialog>
  );
}

function Form(props: { onClose: () => void; onCreated?: ((id: string) => void) | undefined }) {
  const create = useCreateProject();
  const [name, setName] = useState("");
  const [description, setDescription] = useState("");
  const submit = async (event: FormEvent) => {
    event.preventDefault();
    const project = await create.mutateAsync({
      name: name.trim(),
      description: description.trim(),
    });
    props.onCreated?.(project.id);
    props.onClose();
  };
  return (
    <form className="form-col" onSubmit={(event) => void submit(event).catch(() => undefined)}>
      <label className="field">
        {w.projects.name}
        <input
          className="input"
          required
          value={name}
          onChange={(event) => setName(event.target.value)}
        />
      </label>
      <label className="field">
        {w.projects.description}
        <input
          className="input"
          value={description}
          onChange={(event) => setDescription(event.target.value)}
        />
      </label>
      {create.isError ? (
        <p className="error-line" role="alert">
          {reason(create.error)}
        </p>
      ) : null}
      <button
        type="submit"
        className="btn primary"
        disabled={create.isPending || name.trim() === ""}
      >
        {w.projects.create}
      </button>
    </form>
  );
}
