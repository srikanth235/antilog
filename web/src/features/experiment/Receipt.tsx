import { Link } from "@tanstack/react-router";
import { ApiFailure } from "../../api/client";
import { useReceipt } from "../../api/queries";
import { Icon } from "../../components/icon/Icon";
import { Empty, Failed, Loading } from "../../components/state/State";
import { dollars } from "../../lib/format";
import { w } from "../../words";

function Rows({ rows }: { rows: [string, React.ReactNode][] }) {
  return (
    <div className="ds-b">
      {rows.map(([k, v]) => (
        <div key={k} className="dr">
          <span className="k">{k}</span>
          <span className="v">
            <span>{v}</span>
          </span>
        </div>
      ))}
    </div>
  );
}

function Section(props: { title: string; children: React.ReactNode }) {
  return (
    <section className="ds" aria-label={props.title}>
      <div className="ds-h">
        <h2 className="t-section">{props.title}</h2>
      </div>
      {props.children}
    </section>
  );
}

/** One win, as the judge signed it. */
export function Receipt({ trialId }: { trialId: string }) {
  const receipt = useReceipt(trialId);
  const r = w.receipt;
  if (receipt.isPending)
    return (
      <section className="page ct">
        <Loading />
      </section>
    );
  if (receipt.isError) {
    const missing = receipt.error instanceof ApiFailure && receipt.error.status === 404;
    return (
      <section className="page ct">
        {missing ? <Empty title={r.notFound} /> : <Failed error={receipt.error} />}
      </section>
    );
  }
  const { body, keyId } = receipt.data;
  const e = body.experiment;
  const list = (items: string[]) => (items.length === 0 ? w.noData : items.join(", "));
  return (
    <article className="page ct">
      <Link
        className="textbtn ct-back"
        to="/experiments/$experimentId"
        params={{ experimentId: e.experiment }}
      >
        <Icon name="back" size="s" />
        {r.back}
      </Link>
      <div className="ct-head">
        <div className="ct-ttl">
          <div className="rw">
            <h1 className="t-title">{r.title(e.trial)}</h1>
            <span className="tag">{r.signedBy}</span>
          </div>
          <span className="ink2 mono xs">
            {r.keyId}: {keyId}
          </span>
        </div>
        <div className="ct-ctl">
          <button type="button" className="btn" disabled title={r.verifyNote}>
            <Icon name="verify" size="s" />
            {r.verify}
          </button>
        </div>
      </div>
      <div className="vs c4">
        <div>
          <span className="k">{r.baseline}</span>
          <span className="v">{e.baseline}</span>
        </div>
        <div>
          <span className="k">{r.noise}</span>
          <span className="v">{e.noise}</span>
        </div>
        <div>
          <span className="k">{r.thisTrial}</span>
          <span className="v">{e.trial.slice(0, 8)}</span>
        </div>
        <div>
          <span className="k">{r.trialsTried}</span>
          <span className="v">{e.trialsTried}</span>
        </div>
      </div>
      <Section title={r.experiment}>
        <Rows
          rows={[
            [r.version, `v${e.version}`],
            [
              r.seal,
              <span className="mono xs" key="s">
                {e.seal}
              </span>,
            ],
            [r.worth, e.valueModel == null ? r.noValue : dollars(Math.round(e.value * 100))],
            [r.cost, dollars(e.costOfWin)],
            [r.return, e.winReturn == null ? r.noValue : `${e.winReturn}×`],
          ]}
        />
      </Section>
      <Section title={r.commits}>
        <Rows
          rows={body.data.commits.map(
            (c) =>
              [
                c.repo,
                <span className="mono xs" key={c.sha}>
                  {c.sha}
                </span>,
              ] as [string, React.ReactNode],
          )}
        />
      </Section>
      <Section title={r.data}>
        <Rows
          rows={[
            [r.datasets, list(body.data.datasets.map((d) => `${d.id} v${d.version}`))],
            [r.skills, list(body.data.skillPacks.map((d) => `${d.id} v${d.version}`))],
            [r.credentials, list(body.data.credentialScopes.map((c) => `${c.holder}: ${c.scope}`))],
          ]}
        />
      </Section>
      <Section title={r.compute}>
        <Rows
          rows={[
            [r.judgePlatform, body.compute.platform],
            [r.environment, body.compute.environment],
            [
              r.judgeImage,
              <span className="mono xs" key="i">
                {body.compute.judgeImage} {body.compute.judgeDigest}
              </span>,
            ],
            [r.agents, list(body.compute.agents.map((a) => `${a.agent} on ${a.platform}`))],
          ]}
        />
      </Section>
      <Section title={r.repeats}>
        <Rows
          rows={[
            [r.repeatsLabel, e.repeats],
            [r.noise, e.noise],
          ]}
        />
      </Section>
      <Section title={r.holdout}>
        <Rows rows={[[r.holdout, e.holdout === "passed" ? r.passed : r.failed]]} />
      </Section>
      <Section title={r.secondOpinion}>
        <p className="ds-n">
          {e.secondOpinion == null
            ? r.noOpinion
            : e.secondOpinion.flags.length === 0
              ? r.clean(e.secondOpinion.provider)
              : r.flagged(e.secondOpinion.provider, e.secondOpinion.flags.join(", "))}
        </p>
      </Section>
      <Section title={r.spend}>
        <Rows rows={[[r.cost, dollars(e.spend)]]} />
      </Section>
    </article>
  );
}
