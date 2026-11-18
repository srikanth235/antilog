import { useState } from "react";
import type { Schemas } from "../../api/client";
import { useMembers, useProjectValue, useValueModels } from "../../api/queries";
import { Icon } from "../../components/icon/Icon";
import { Empty, Failed, Loading } from "../../components/state/State";
import { isHttpUrl, modelLine } from "../../lib/value";
import { w } from "../../words";
import { ValueTiles } from "../value/ValueTiles";
import { AddValueModel } from "./AddValueModel";

const t = w.value;

export function SourceText({ source }: { source: Schemas["ValueSource"] }) {
  return source.link != null && isHttpUrl(source.link) ? (
    <a className="textbtn" href={source.link} target="_blank" rel="noopener noreferrer">
      {source.text}
    </a>
  ) : (
    <span>{source.text}</span>
  );
}

/** The Project page's Value section: the value models, and what the project's wins add up to. */
export function ProjectValue(props: {
  project: Pick<Schemas["Project"], "id" | "name">;
  owner: boolean;
}) {
  const models = useValueModels(props.project.id);
  const figures = useProjectValue(props.project.id);
  const members = useMembers();
  const [dialog, setDialog] = useState<{ change?: Schemas["ProjectValueModel"] } | null>(null);
  const list = models.data ?? [];
  const who = (id: string) => members.data?.find((member) => member.id === id)?.name ?? t.unknown;
  const ownerTitle = props.owner ? undefined : w.readOnly.onlyOwners;

  return (
    <section className="ds" aria-labelledby="h-value">
      <div className="ds-h">
        <h2 className="t-section" id="h-value">
          {t.title}
        </h2>
        <button
          type="button"
          className="btn sm"
          disabled={!props.owner}
          title={ownerTitle}
          onClick={() => setDialog({})}
        >
          <Icon name="plus" size="s" />
          {t.addModel}
        </button>
      </div>
      <p className="ds-n">{t.lead}</p>
      {figures.isPending ? null : figures.isError ? (
        <Failed error={figures.error} retry={() => void figures.refetch()} />
      ) : (
        <ValueTiles figures={figures.data} label={t.figuresLabel} />
      )}
      {models.isPending ? (
        <Loading rows={1} />
      ) : models.isError ? (
        <Failed error={models.error} retry={() => void models.refetch()} />
      ) : list.length === 0 ? (
        <Empty title={t.noValueSet} hint={t.noValueHint} />
      ) : (
        <div className="tb ap-vm" style={{ marginTop: 12 }}>
          <div className="tb-r th">
            <span>{t.columns.metric}</span>
            <span>{t.columns.model}</span>
            <span>{t.columns.source}</span>
            <span>{t.columns.setBy}</span>
            <span>{t.columns.version}</span>
          </div>
          {list.map((entry) => (
            <div key={entry.model.metric} className="tb-r">
              <span className="n">
                <span>{entry.model.metric}</span>
              </span>
              <span>{modelLine(entry.model)}</span>
              <span className="ink2">
                <SourceText source={entry.model.source} />
              </span>
              <span className="ink2">{who(entry.model.setBy)}</span>
              <span className="rw g8">
                <span className="mono">v{entry.model.version}</span>
                {props.owner ? (
                  <button
                    type="button"
                    className="btn sm"
                    aria-label={t.changeModel(entry.model.metric)}
                    onClick={() => setDialog({ change: entry })}
                  >
                    {t.change}
                  </button>
                ) : null}
              </span>
            </div>
          ))}
        </div>
      )}
      {dialog === null ? null : (
        <AddValueModel
          open
          onClose={() => setDialog(null)}
          project={props.project}
          models={list}
          change={dialog.change}
        />
      )}
    </section>
  );
}
