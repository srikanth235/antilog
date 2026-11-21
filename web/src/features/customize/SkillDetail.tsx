import { Link, useNavigate } from "@tanstack/react-router";
import { useState } from "react";
import { ApiFailure } from "../../api/client";
import { useMember, useRemoveSkill, useSkills, useSkillVersion } from "../../api/queries";
import { Confirm } from "../../components/dialog/Dialog";
import { Icon } from "../../components/icon/Icon";
import { Empty, Failed, Loading, reason } from "../../components/state/State";
import { canEdit } from "../../lib/domain";
import { w } from "../../words";
import { Row, Table } from "../settings/Table";
import { DetailHead, Section, TwoColumns } from "./ListShell";

const s = w.skill;

function size(bytes: number): string {
  return bytes < 1024 ? `${bytes} B` : `${(bytes / 1024).toFixed(1)} KB`;
}

function Back() {
  return (
    <Link className="textbtn" to="/customize/skills" style={{ alignSelf: "flex-start" }}>
      <Icon name="back" size="s" />
      {s.back}
    </Link>
  );
}

export function SkillDetail({ name }: { name: string }) {
  const skills = useSkills();
  const item = skills.data?.find((skill) => skill.name === name);
  if (item === undefined) {
    return (
      <section className="page">
        <Back />
        {skills.isPending ? (
          <Loading />
        ) : skills.isError && skills.error instanceof ApiFailure && skills.error.status !== 404 ? (
          <Failed error={skills.error} retry={() => void skills.refetch()} />
        ) : (
          <Empty title={s.empty} />
        )}
      </section>
    );
  }
  return (
    <Loaded
      name={name}
      description={item.description}
      versions={item.versions}
      latest={item.latest}
      fromPlugin={item.fromPlugin}
    />
  );
}

function Loaded(props: {
  name: string;
  description: string;
  versions: number[];
  latest: number;
  fromPlugin?: string | null | undefined;
}) {
  const { name } = props;
  const me = useMember();
  const editable = canEdit(me?.role);
  const navigate = useNavigate();
  const remove = useRemoveSkill(name);
  const [confirming, setConfirming] = useState(false);
  const [shown, setShown] = useState<number | undefined>(undefined);
  const version = shown ?? props.latest;
  const files = useSkillVersion(name, version);
  const versions = [...props.versions].sort((a, b) => b - a);
  const cols = "120px minmax(0, 1fr) 120px";
  return (
    <section className="page">
      <Back />
      <DetailHead
        glyph="book"
        title={name}
        subtitle={props.description}
        state={
          <span className="rw g8 sm">
            <span className="mono xs">{s.version(props.latest)}</span>
            <span className="mut">
              {props.fromPlugin == null ? s.uploaded : s.fromPlugin(props.fromPlugin)}
            </span>
          </span>
        }
        actions={
          <>
            <Link className="btn" to="/customize/skills/new">
              {s.addVersion}
            </Link>
            <button
              type="button"
              className="btn danger"
              disabled={!editable}
              onClick={() => setConfirming(true)}
            >
              {s.remove}
            </button>
          </>
        }
      />
      {editable ? null : <p className="sm mut">{w.readOnly.onlyEditors}</p>}
      {remove.isError ? (
        <p className="error-line" role="alert">
          {reason(remove.error)}
        </p>
      ) : null}
      <TwoColumns
        left={
          <>
            <Section title={s.versions}>
              <Table columns={cols} min={360} label={s.versions} head={[s.columns.version, "", ""]}>
                {versions.map((v) => (
                  <Row key={v} columns={cols}>
                    <span className="mono xs">{s.version(v)}</span>
                    <span>
                      {v === props.latest ? <span className="tag">{s.latest}</span> : null}
                    </span>
                    <button
                      type="button"
                      className="btn sm"
                      aria-pressed={v === version}
                      aria-label={`${s.showFiles} ${s.version(v)}`}
                      onClick={() => setShown(v)}
                    >
                      {s.showFiles}
                    </button>
                  </Row>
                ))}
              </Table>
            </Section>
            <Section title={s.filesIn(s.version(version))}>
              {files.isPending ? (
                <Loading rows={2} />
              ) : files.isError ? (
                <Failed error={files.error} retry={() => void files.refetch()} />
              ) : (
                <div className="col g8">
                  {files.data.files.map((file) => (
                    <details key={file.path} className="card" style={{ padding: "8px 16px" }}>
                      <summary className="rw between">
                        <span className="mono xs">{file.path}</span>
                        <span className="sm ink2">{size(file.content.length)}</span>
                      </summary>
                      <pre
                        className="mono xs"
                        style={{ whiteSpace: "pre-wrap", margin: "8px 0 0" }}
                      >
                        {file.content}
                      </pre>
                    </details>
                  ))}
                </div>
              )}
            </Section>
          </>
        }
        right={
          <Section title={s.howTitle}>
            <div className="card" style={{ padding: "4px 20px" }}>
              <div className="kv">
                <span>{s.name}</span>
                <span>
                  <span className="mono xs">{name}</span>
                </span>
              </div>
              <div className="kv">
                <span>{s.pinned}</span>
                <span>{s.pinnedText}</span>
              </div>
              <div className="kv">
                <span>{s.loaded}</span>
                <span>{s.loadedText}</span>
              </div>
            </div>
          </Section>
        }
      />
      <Confirm
        open={confirming}
        onOpenChange={setConfirming}
        title={s.removeTitle(name)}
        description={s.removeText}
        confirm={s.removeConfirm}
        keep={s.removeKeep}
        onConfirm={() => {
          setConfirming(false);
          remove
            .mutateAsync()
            .then(() => navigate({ to: "/customize/skills" }))
            .catch(() => undefined);
        }}
      />
    </section>
  );
}
