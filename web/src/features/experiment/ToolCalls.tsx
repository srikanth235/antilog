import { useState } from "react";
import { Icon } from "../../components/icon/Icon";
import {
  failedCount,
  groupTitle,
  rowTitle,
  type ToolCallView,
  type ToolGroupView,
} from "../../lib/toolcalls";
import { w } from "../../words";

function Copy(props: { text: string }) {
  const [copied, setCopied] = useState(false);
  return (
    <button
      type="button"
      className="icon-btn sm"
      aria-label={copied ? w.experiment.tools.copied : w.experiment.tools.copy}
      title={w.experiment.tools.copy}
      onClick={() => {
        void navigator.clipboard
          .writeText(props.text)
          .then(() => setCopied(true))
          .catch(() => undefined);
      }}
    >
      <Icon name={copied ? "check" : "copy"} size="s" />
    </button>
  );
}

function ToolRow(props: { call: ToolCallView; id: string }) {
  const { call } = props;
  const [open, setOpen] = useState(false);
  const { verb, label } = rowTitle(call);
  return (
    <div className={`tcr${call.failed ? " bad" : ""}`}>
      <button
        type="button"
        className="tcr-h"
        aria-expanded={open}
        aria-controls={props.id}
        onClick={() => setOpen(!open)}
      >
        {call.failed ? (
          <span role="img" aria-label={w.experiment.tools.failed} className="st gi st-fail" />
        ) : null}
        <span className="tcr-t">
          {verb} <code>{label}</code>
        </span>
        {call.failed ? <span className="tcr-f">{w.experiment.tools.failed}</span> : null}
        <Icon name="chev" size="s" className="chev" />
      </button>
      {open ? (
        <div className="tcr-d" id={props.id}>
          {call.input === null && call.output === null ? (
            <span className="xs mut">{w.experiment.tools.nothingKept}</span>
          ) : null}
          {call.input === null ? null : (
            <>
              <div className="tcr-k">
                <span>
                  {call.kind === "command" ? w.experiment.tools.command : w.experiment.tools.input}
                </span>
                <Copy text={call.input} />
              </div>
              <pre className="tcr-p">{call.input}</pre>
            </>
          )}
          {call.output === null ? null : (
            <>
              <div className="tcr-k">
                <span>{w.experiment.tools.output}</span>
                <Copy text={call.output} />
              </div>
              <pre className={`tcr-p${call.failed ? " bad" : ""}`}>{call.output}</pre>
            </>
          )}
        </div>
      ) : null}
    </div>
  );
}

/**
 * One run of tool calls between two messages: a quiet line until opened, then a bordered list.
 * The latest group is open while the agent works and folds when it writes.
 */
export function ToolGroup(props: { group: ToolGroupView; running: boolean }) {
  const { group, running } = props;
  const [chosen, setChosen] = useState<boolean | undefined>(undefined);
  const open = chosen ?? running;
  const failed = failedCount(group.calls);
  const id = `tools-${group.key}`;
  return (
    <div className="tcg">
      <button
        type="button"
        className="tcg-h"
        aria-expanded={open}
        aria-controls={id}
        onClick={() => setChosen(!open)}
      >
        {running ? (
          <span
            role="img"
            aria-label={w.experiment.tools.running}
            className="st run gi st-work"
            style={{ color: "var(--accent)" }}
          />
        ) : null}
        <span>{groupTitle(group.calls, running)}</span>
        {failed === 0 ? null : (
          <span className="bad">{w.experiment.tools.failedCount(failed)}</span>
        )}
        <Icon name="chev" size="s" className="chev" />
      </button>
      {open ? (
        <div className="tcg-l" id={id}>
          {group.calls.map((call) => (
            <ToolRow key={call.seq} call={call} id={`${id}-${call.seq}`} />
          ))}
        </div>
      ) : null}
    </div>
  );
}
