import { useMemo, useState } from "react";
import type { Schemas } from "../../api/client";
import { useChanges, useCommitChanges } from "../../api/queries";
import { Glyph, Icon } from "../../components/icon/Icon";
import {
  type FileChange,
  parsePatch,
  splitPath,
  totals,
  unmodifiedBefore,
  withheld,
} from "../../lib/diff";
import { ago } from "../../lib/format";
import { w } from "../../words";
import { PanelFrame } from "./Panels";

const t = w.experiment.changesView;

type RepoChanges = Schemas["RepoChanges"];

function Counts({ added, removed }: { added: number; removed: number }) {
  return (
    <>
      <span className="ad">{`+${added}`}</span>
      <span className="rm2">{`-${removed}`}</span>
    </>
  );
}

function FileDiff({ file }: { file: FileChange }) {
  const hunks = useMemo(() => parsePatch(file.patch), [file.patch]);
  const why = withheld(file);
  if (why !== null) return <p className="note">{why === "binary" ? t.binary : t.tooLarge}</p>;
  if (hunks.length === 0) return <p className="note">{t.noCommitChanges}</p>;
  return (
    <section className="diff" aria-label={t.diffOf(file.path)}>
      {hunks.map((hunk, index) => {
        const hidden = unmodifiedBefore(hunks, index);
        return (
          <div key={`${hunk.oldStart}:${hunk.newStart}`}>
            {hidden > 0 ? (
              <div className="ux">
                <Icon name="chev" size="s" />
                {t.unmodified(hidden)}
              </div>
            ) : null}
            <div className="hk">{`@@ -${hunk.oldStart},${hunk.oldLines} +${hunk.newStart},${hunk.newLines} @@ ${hunk.heading}`}</div>
            {hunk.lines.map((line, at) => (
              <div
                // biome-ignore lint/suspicious/noArrayIndexKey: lines of a patch are never reordered
                key={at}
                className={line.kind === "context" ? "l" : `l ${line.kind}`}
              >
                <span>{line.old}</span>
                <span>{line.next}</span>
                <span className="sg">{{ add: "+", del: "-", context: " " }[line.kind]}</span>
                <span>{line.text}</span>
              </div>
            ))}
          </div>
        );
      })}
    </section>
  );
}

const fileId = (path: string) => `file-${path}`;

function FileSection(props: { file: FileChange; open: boolean; onToggle: () => void }) {
  const { file } = props;
  return (
    <section className="fd" id={fileId(file.path)}>
      <button
        type="button"
        className="fh"
        aria-expanded={props.open}
        aria-label={props.open ? t.hideDiff(file.path) : t.showDiff(file.path)}
        onClick={props.onToggle}
      >
        <Icon name="chev" size="s" className="chev" />
        <span className="p">
          <bdi>{file.path}</bdi>
        </span>
        {file.status === "deleted" ? <span className="tag">{t.removed}</span> : null}
        <Counts added={file.additions} removed={file.deletions} />
      </button>
      {props.open ? <FileDiff file={file} /> : null}
    </section>
  );
}

function BranchLine(props: { repo: RepoChanges; repos: string[]; onRepo: (name: string) => void }) {
  const { repo } = props;
  const sameBranch = repo.baseBranch === repo.headBranch || repo.headBranch === "HEAD";
  return (
    <div className="chg-h">
      {props.repos.length > 1 ? (
        <select
          className="opt"
          aria-label={t.repo}
          value={repo.repo}
          onChange={(event) => props.onRepo(event.target.value)}
        >
          {props.repos.map((name) => (
            <option key={name} value={name}>
              {name}
            </option>
          ))}
        </select>
      ) : (
        <span className="rp">
          <Glyph name="code" size="s" />
          {repo.repo}
        </span>
      )}
      <b>{repo.baseBranch}</b>
      {sameBranch ? (
        <span className="arr">{t.workingTree}</span>
      ) : (
        <>
          <span className="arr" aria-hidden="true">
            →
          </span>
          <b>{repo.headBranch}</b>
        </>
      )}
    </div>
  );
}

function Commits(props: {
  repo: RepoChanges;
  selected: string | null;
  onSelect: (sha: string | null) => void;
  now: number;
}) {
  const { commits } = props.repo;
  return (
    <>
      <h3>{t.commits}</h3>
      <div className="cm-list">
        <button
          type="button"
          className="cm"
          aria-current={props.selected === null}
          onClick={() => props.onSelect(null)}
        >
          <span className="s">{t.allChanges}</span>
          <span className="m">
            <span>{t.commitCount(commits.length)}</span>
            <span>{t.uncommitted}</span>
          </span>
        </button>
        {commits.map((commit) => (
          <button
            type="button"
            key={commit.sha}
            className="cm"
            aria-current={props.selected === commit.sha}
            onClick={() => props.onSelect(commit.sha)}
          >
            <span className="s">{commit.subject}</span>
            <span className="m">
              <span>{commit.sha.slice(0, 7)}</span>
              <span>{ago(commit.time, props.now)}</span>
            </span>
          </button>
        ))}
      </div>
    </>
  );
}

function FileList(props: {
  files: FileChange[];
  current: string | null;
  onPick: (path: string) => void;
}) {
  return (
    <>
      <h3>{t.files}</h3>
      {props.files.map((file) => {
        const { name, folder } = splitPath(file.path);
        return (
          <button
            type="button"
            key={file.path}
            className={file.status === "deleted" ? "fi rm" : "fi"}
            aria-current={props.current === file.path}
            onClick={() => props.onPick(file.path)}
          >
            <Glyph name="file" size="s" />
            <span className="nm">{name}</span>
            <span className="dir">{folder}</span>
            <Counts added={file.additions} removed={file.deletions} />
          </button>
        );
      })}
    </>
  );
}

/** One repo's diff: the branch line, every changed file as a collapsible unified diff, and
 * on the right the commits (All changes, or one commit) and the file list. */
function RepoView(props: {
  sessionId: string;
  repo: RepoChanges;
  repos: string[];
  onRepo: (name: string) => void;
}) {
  const { repo } = props;
  const [selected, setSelected] = useState<string | null>(null);
  const [closed, setClosed] = useState<Set<string>>(new Set());
  const [current, setCurrent] = useState<string | null>(null);
  const commit = useCommitChanges(props.sessionId, repo.repo, selected);
  const files = selected === null ? repo.files : (commit.data?.files ?? []);
  const loading = selected !== null && commit.data === undefined;
  const failed = selected !== null && commit.isError;
  const toggle = (path: string) =>
    setClosed((before) => {
      const next = new Set(before);
      if (!next.delete(path)) next.add(path);
      return next;
    });
  const now = Date.now();
  return (
    <>
      <BranchLine repo={repo} repos={props.repos} onRepo={props.onRepo} />
      <div className="chg-body">
        <section className="chg-main" aria-label={t.diffOf(repo.repo)}>
          {failed ? <p className="sd-none">{t.loadFailed}</p> : null}
          {!failed && !loading && files.length === 0 ? (
            <p className="sd-none">{selected === null ? t.noChanges : t.noCommitChanges}</p>
          ) : null}
          {files.map((file) => (
            <FileSection
              key={`${selected}:${file.path}`}
              file={file}
              open={!closed.has(file.path)}
              onToggle={() => toggle(file.path)}
            />
          ))}
        </section>
        <div className="chg-side">
          <Commits repo={repo} selected={selected} onSelect={setSelected} now={now} />
          <FileList
            files={files}
            current={current}
            onPick={(path) => {
              setCurrent(path);
              setClosed((before) => {
                const next = new Set(before);
                next.delete(path);
                return next;
              });
              requestAnimationFrame(() =>
                document.getElementById(fileId(path))?.scrollIntoView({ block: "start" }),
              );
            }}
          />
        </div>
      </div>
    </>
  );
}

export function ChangesPanel(props: { sessionId: string; onClose: () => void }) {
  const changes = useChanges(props.sessionId);
  const repos = changes.data?.repos ?? [];
  const [name, setName] = useState<string | null>(null);
  const repo = repos.find((one) => one.repo === name) ?? repos[0];
  const sum = totals(repo?.files ?? []);
  return (
    <PanelFrame
      panel="changes"
      wide
      onClose={props.onClose}
      actions={
        repo === undefined ? null : (
          <>
            <span className="xs mut">{t.fileCount(sum.files)}</span>
            <span className="dlt">{`+${sum.additions}`}</span>
            <span className="dlt bad">{`-${sum.deletions}`}</span>
          </>
        )
      }
    >
      {changes.isError ? (
        <p className="sd-none">{t.loadFailed}</p>
      ) : repo === undefined ? (
        <p className="sd-none">{changes.isPending ? "" : t.noChanges}</p>
      ) : (
        <RepoView
          key={repo.repo}
          sessionId={props.sessionId}
          repo={repo}
          repos={repos.map((one) => one.repo)}
          onRepo={setName}
        />
      )}
    </PanelFrame>
  );
}
