import { Link, useNavigate } from "@tanstack/react-router";
import { type ChangeEvent, type FormEvent, useState } from "react";
import { ApiFailure } from "../../api/client";
import { useAddSkill } from "../../api/queries";
import { Icon } from "../../components/icon/Icon";
import { reason } from "../../components/state/State";
import { w } from "../../words";

interface SkillFile {
  path: string;
  content: string;
}

const s = w.skill;
const MAX_FILE_BYTES = 1_000_000;

/** A text file, or null for what is not one: a hidden path, a large file, or bytes with a NUL in them. */
async function readText(file: File, path: string): Promise<SkillFile | null> {
  if (path.split("/").some((part) => part.startsWith("."))) return null;
  if (file.size > MAX_FILE_BYTES) return null;
  const content = await file.text();
  return content.includes("\u0000") ? null : { path, content };
}

/** Adds a skill pack: a pasted SKILL.md, chosen text files, or a chosen folder. A name that exists becomes its next version. */
export function AddSkill() {
  const add = useAddSkill();
  const navigate = useNavigate();
  const [pasted, setPasted] = useState("");
  const [chosen, setChosen] = useState<SkillFile[]>([]);
  const [error, setError] = useState<string | null>(null);

  const choose = (folder: boolean) => async (event: ChangeEvent<HTMLInputElement>) => {
    const list = Array.from(event.target.files ?? []);
    const read = await Promise.all(
      list.map((file) => {
        // A picked folder names its top folder in every path; SKILL.md belongs at the top of the pack.
        const path = folder ? file.webkitRelativePath.split("/").slice(1).join("/") : file.name;
        return readText(file, path);
      }),
    );
    const files = read.filter((file): file is SkillFile => file !== null);
    setChosen(files);
    setError(list.length > 0 && files.length === 0 ? s.noText : null);
  };

  const save = async (event: FormEvent) => {
    event.preventDefault();
    setError(null);
    const files = chosen.length > 0 ? chosen : [{ path: "SKILL.md", content: pasted }];
    if (files.every((file) => file.content.trim() === "")) {
      setError(s.pasteEmpty);
      return;
    }
    try {
      const made = await add.mutateAsync({ files });
      await navigate({ to: "/customize/skills/$name", params: { name: made.name } });
    } catch (caught) {
      setError(caught instanceof ApiFailure ? caught.message : reason(caught));
    }
  };

  return (
    <section className="page">
      <Link className="textbtn" to="/customize/skills" style={{ alignSelf: "flex-start" }}>
        <Icon name="back" size="s" />
        {s.back}
      </Link>
      <h1 className="t-title">{s.addTitle}</h1>
      <p className="ink2" style={{ maxWidth: 560 }}>
        {s.addLead}
      </p>
      <form className="form-col" style={{ maxWidth: 560 }} onSubmit={(event) => void save(event)}>
        <label className="field">
          {s.paste}
          <textarea
            className="input mono"
            rows={9}
            value={pasted}
            disabled={chosen.length > 0}
            onChange={(event) => setPasted(event.target.value)}
          />
        </label>
        <label className="field">
          {s.choose}
          <input className="input" type="file" multiple onChange={(e) => void choose(false)(e)} />
          <span className="xs mut">{s.chooseHint}</span>
        </label>
        <label className="field">
          {s.chooseFolder}
          <input
            className="input"
            type="file"
            // Not in React's typings: the folder picker.
            {...{ webkitdirectory: "" }}
            onChange={(e) => void choose(true)(e)}
          />
          <span className="xs mut">{s.chooseFolderHint}</span>
        </label>
        {chosen.length === 0 ? null : (
          <p className="sm ink2">
            {s.chosen(chosen.length)}:{" "}
            <span className="mono xs">{chosen.map((f) => f.path).join(", ")}</span>
          </p>
        )}
        {error === null ? null : (
          <p className="error-line" role="alert">
            {error}
          </p>
        )}
        <button type="submit" className="btn primary" disabled={add.isPending}>
          {s.save}
        </button>
      </form>
    </section>
  );
}
