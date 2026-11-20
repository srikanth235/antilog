import { Link, useNavigate } from "@tanstack/react-router";
import { useMemo } from "react";
import type { Schemas } from "../../api/client";
import { useAgentLabel, useProjects, useReport } from "../../api/queries";
import { Icon } from "../../components/icon/Icon";
import { Select } from "../../components/select/Select";
import { Empty, Failed, Loading } from "../../components/state/State";
import { dollars } from "../../lib/format";
import { OUTCOME_WORDS } from "../../lib/history";
import { agentTotals, costPerWin, fromTo, monthOptions, percent } from "../../lib/value";
import { w } from "../../words";
import { ValueTiles } from "../value/ValueTiles";
import { WinState } from "../value/WinState";

const t = w.report;
const ALL = "all";
const KIND_CLASS = { tokens: "ap-k1", machine: "ap-k2", judge: "ap-k3" } as const;

export function Report(props: { month: string | undefined; project: string | undefined }) {
  const navigate = useNavigate();
  const projects = useProjects();
  const months = useMemo(() => {
    const list = monthOptions(new Date());
    const first = list[0];
    const wanted = props.month ?? first?.value ?? "";
    return {
      current: wanted,
      options: list.some((item) => item.value === wanted)
        ? list
        : [{ value: wanted, label: wanted }, ...list],
    };
  }, [props.month]);
  const report = useReport(months.current, props.project ?? null);
  const monthName = months.options.find((item) => item.value === months.current)?.label ?? "";
  const projectName = (id: string) =>
    projects.data?.find((project) => project.id === id)?.name ?? "";

  const choose = (next: { month?: string; project?: string }) =>
    void navigate({
      to: "/report",
      search: {
        ...(next.month === undefined ? {} : { month: next.month }),
        ...(next.project === undefined ? {} : { project: next.project }),
      },
    });

  return (
    <article className="page ct ls">
      <Link className="textbtn ct-back" to="/projects">
        <Icon name="back" size="s" />
        {t.back}
      </Link>
      <div className="ct-head">
        <div className="ct-ttl">
          <div className="rw">
            <h1 className="t-title">{t.title}</h1>
            <span className="tag">{monthName}</span>
          </div>
          <span className="ink2">{t.lead}</span>
        </div>
      </div>
      <search className="ap-fl" aria-label={t.filters} style={{ marginTop: 16 }}>
        <span className="rw g8">
          <span className="lbl sm ink2">{t.project}</span>
          <Select
            label={t.project}
            value={props.project ?? ALL}
            options={[
              { value: ALL, label: t.allProjects },
              ...(projects.data ?? []).map((project) => ({
                value: project.id,
                label: project.name,
              })),
            ]}
            onChange={(value) =>
              choose({
                month: months.current,
                ...(value === ALL ? {} : { project: value }),
              })
            }
          />
        </span>
        <span className="rw g8">
          <span className="lbl sm ink2">{t.month}</span>
          <Select
            label={t.month}
            value={months.current}
            options={months.options}
            onChange={(value) =>
              choose({ month: value, ...(props.project ? { project: props.project } : {}) })
            }
          />
        </span>
      </search>
      {report.isPending ? (
        <div style={{ marginTop: 24 }}>
          <Loading />
        </div>
      ) : report.isError ? (
        <div style={{ marginTop: 24 }}>
          <Failed error={report.error} retry={() => void report.refetch()} />
        </div>
      ) : (
        <ReportBody report={report.data} monthName={monthName} projectName={projectName} />
      )}
    </article>
  );
}

function ReportBody(props: {
  report: Schemas["Report"];
  monthName: string;
  projectName: (id: string) => string;
}) {
  const { report } = props;
  const f = report.figures;
  const noWin = report.spentNoWinCents + report.spentNotSealedCents;
  const quiet =
    f.spentCents === 0 &&
    report.agents.length === 0 &&
    report.topWins.length === 0 &&
    report.stopped.length === 0 &&
    f.deliveredCents === 0 &&
    f.pendingCents === 0 &&
    f.atRiskCents === 0;
  return (
    <>
      <ValueTiles figures={f} label={t.figuresLabel} spent={{ noWinCents: noWin }} />
      {quiet ? (
        <div style={{ marginTop: 24 }}>
          <Empty title={t.emptyTitle} hint={t.emptyHint} />
        </div>
      ) : (
        <>
          <Spend report={report} />
          <Agents report={report} monthName={props.monthName} />
          <TopWins report={report} projectName={props.projectName} />
          <Stopped report={report} projectName={props.projectName} />
        </>
      )}
    </>
  );
}

function Spend({ report }: { report: Schemas["Report"] }) {
  const total = report.figures.spentCents;
  const split = [
    [t.withWin, report.spentWithWinCents],
    [t.noWin, report.spentNoWinCents],
    [t.notSealed, report.spentNotSealedCents],
  ] as const;
  return (
    <section className="ds" aria-labelledby="h-spend">
      <div className="ds-h">
        <h2 className="t-section" id="h-spend">
          {t.spendTitle}
        </h2>
        <span className="xs mut">{dollars(total)}</span>
      </div>
      <p className="ds-n">{t.spendNote}</p>
      {total === 0 ? null : (
        <div
          className="ap-sp"
          role="img"
          aria-label={`${t.spendTitle}. ${report.spendByKind.map((row) => `${w.spendKinds[row.kind]} ${dollars(row.cents)}`).join(", ")}`}
        >
          {report.spendByKind
            .filter((row) => row.cents > 0)
            .map((row) => (
              <span
                key={row.kind}
                className={KIND_CLASS[row.kind]}
                style={{ width: `${(row.cents / total) * 100}%` }}
                title={`${w.spendKinds[row.kind]} ${dollars(row.cents)}`}
              />
            ))}
        </div>
      )}
      <div className="tb tb-sp" style={{ marginTop: 12 }}>
        <div className="tb-r th">
          <span>{t.kind}</span>
          <span className="r">{t.spentCol}</span>
          <span className="r">{t.shareCol}</span>
        </div>
        {report.spendByKind.map((row) => (
          <div key={row.kind} className="tb-r">
            <span className="ap-nm">
              <i className={`ap-sw ${KIND_CLASS[row.kind]}`} />
              {w.spendKinds[row.kind]}
            </span>
            <span className="mono r">{dollars(row.cents)}</span>
            <span className="mono r ink2">{percent(row.cents, total)}%</span>
          </div>
        ))}
        <div className="tb-r tot">
          <span>{t.total}</span>
          <span className="mono r">{dollars(total)}</span>
          <span className="mono r">{total === 0 ? "" : "100%"}</span>
        </div>
      </div>
      <div className="tb tb-sp" style={{ marginTop: 20 }}>
        <div className="tb-r th">
          <span>{t.byOutcome}</span>
          <span className="r">{t.spentCol}</span>
          <span className="r">{t.shareCol}</span>
        </div>
        {report.spendByOutcome.map((row) => {
          const label = row.outcome == null ? t.notMeasured : OUTCOME_WORDS[row.outcome];
          return (
            <div key={row.outcome ?? "not-measured"} className="tb-r">
              <span>{label}</span>
              <span className="mono r">{dollars(row.cents)}</span>
              <span className="mono r ink2">{percent(row.cents, total)}%</span>
            </div>
          );
        })}
      </div>
      <div className="tb tb-sp" style={{ marginTop: 20 }}>
        <div className="tb-r th">
          <span>{t.becameOf}</span>
          <span className="r">{t.spentCol}</span>
          <span className="r">{t.shareCol}</span>
        </div>
        {split.map(([label, cents]) => (
          <div key={label} className="tb-r">
            <span>{label}</span>
            <span className="mono r">{dollars(cents)}</span>
            <span className="mono r ink2">{percent(cents, total)}%</span>
          </div>
        ))}
      </div>
    </section>
  );
}

function Agents({ report, monthName }: { report: Schemas["Report"]; monthName: string }) {
  const agentLabel = useAgentLabel();
  const total = agentTotals(report.agents);
  const cost = (spent: number, delivered: number) => {
    const each = costPerWin(spent, delivered);
    return each === null ? t.noWinYet : dollars(each);
  };
  return (
    <section className="ds" aria-labelledby="h-agents">
      <div className="ds-h">
        <h2 className="t-section" id="h-agents">
          {t.agentsTitle}
        </h2>
        <Link className="textbtn" to="/customize/evaluations">
          {t.agentsLink}
        </Link>
      </div>
      <p className="ds-n">{t.agentsNote(monthName)}</p>
      {report.agents.length === 0 ? (
        <p className="sm mut">{t.agentsNone}</p>
      ) : (
        <div className="tb ap-rp3" style={{ marginTop: 12 }}>
          <div className="tb-r th">
            <span>{t.agent}</span>
            <span className="r">{t.trials}</span>
            <span className="r">{t.kept}</span>
            <span className="r">{t.winsDelivered}</span>
            <span className="r">{t.spentCol}</span>
            <span className="r">{t.costPerWin}</span>
            <span className="r">{t.deliveredCol}</span>
          </div>
          {report.agents.map((row) => (
            <div key={row.agent} className="tb-r">
              <span className="n">
                <span>{agentLabel(row.agent)}</span>
                {row.winsPending === 0 ? null : (
                  <span className="m">
                    {t.pendingLine(row.winsPending, dollars(row.pendingCents))}
                  </span>
                )}
              </span>
              <span className="mono r">{row.trials}</span>
              <span className="mono r">{row.kept}</span>
              <span className="mono r">{row.winsDelivered}</span>
              <span className="mono r">{dollars(row.spentCents)}</span>
              <span className="mono r">{cost(row.spentCents, row.winsDelivered)}</span>
              <span className="mono r">{dollars(row.deliveredCents)}</span>
            </div>
          ))}
          <div className="tb-r tot">
            <span>{t.allAgents}</span>
            <span className="mono r">{total.trials}</span>
            <span className="mono r">{total.kept}</span>
            <span className="mono r">{total.winsDelivered}</span>
            <span className="mono r">{dollars(total.spentCents)}</span>
            <span className="mono r">{cost(total.spentCents, total.winsDelivered)}</span>
            <span className="mono r">{dollars(total.deliveredCents)}</span>
          </div>
        </div>
      )}
    </section>
  );
}

function TopWins(props: { report: Schemas["Report"]; projectName: (id: string) => string }) {
  const agentLabel = useAgentLabel();
  return (
    <section className="ds" aria-labelledby="h-wins">
      <div className="ds-h">
        <h2 className="t-section" id="h-wins">
          {t.winsTitle}
        </h2>
      </div>
      <p className="ds-n">{t.winsNote}</p>
      {props.report.topWins.length === 0 ? (
        <p className="sm mut">{t.winsNone}</p>
      ) : (
        <div className="tb ap-wn" style={{ marginTop: 12 }}>
          <div className="tb-r th">
            <span>{t.win}</span>
            <span>{t.agent}</span>
            <span>{t.state}</span>
            <span className="r">{t.spentCol}</span>
            <span className="r">{t.worth}</span>
            <span />
          </div>
          {props.report.topWins.map((top) => (
            <div key={top.win.trialId} className="tb-r">
              <span className="n">
                <span>{top.goal}</span>
                <span className="m">
                  {props.projectName(top.projectId)} · {fromTo(top.win)}
                </span>
              </span>
              <span className="ink2">{top.agent == null ? "-" : agentLabel(top.agent)}</span>
              <span>
                <WinState state={top.win.state} />
              </span>
              <span className="mono r">{dollars(top.spentCents)}</span>
              <span className="mono r" style={{ whiteSpace: "nowrap" }}>
                {top.win.yearlyCents == null ? t.noWorth : dollars(top.win.yearlyCents)}
              </span>
              <Link
                className="btn sm"
                to="/receipts/$trialId"
                params={{ trialId: top.win.trialId }}
                style={{ justifySelf: "end" }}
              >
                {t.viewReceipt}
              </Link>
            </div>
          ))}
        </div>
      )}
    </section>
  );
}

function Stopped(props: { report: Schemas["Report"]; projectName: (id: string) => string }) {
  const total = props.report.stopped.reduce((sum, row) => sum + row.spentCents, 0);
  return (
    <section className="ds" aria-labelledby="h-stopped">
      <div className="ds-h">
        <h2 className="t-section" id="h-stopped">
          {t.stoppedTitle}
        </h2>
      </div>
      <p className="ds-n">{t.stoppedNote}</p>
      {props.report.stopped.length === 0 ? (
        <p className="sm mut">{t.stoppedNone}</p>
      ) : (
        <div className="tb ap-st" style={{ marginTop: 12 }}>
          <div className="tb-r th">
            <span>{t.experiment}</span>
            <span>{t.why}</span>
            <span className="r">{t.spentCol}</span>
          </div>
          {props.report.stopped.map((row) => (
            <Link
              key={row.experimentId}
              className="tb-r"
              to="/experiments/$experimentId/details"
              params={{ experimentId: row.experimentId }}
            >
              <span className="n">
                <span>{row.goal}</span>
                <span className="m">{props.projectName(row.projectId)}</span>
              </span>
              <span className="ink2">{t.stoppedWhy(row.trials, row.budgetSpent)}</span>
              <span className="mono r">{dollars(row.spentCents)}</span>
            </Link>
          ))}
          <div className="tb-r tot">
            <span>{t.stoppedTotal}</span>
            <span />
            <span className="mono r">{dollars(total)}</span>
          </div>
        </div>
      )}
    </section>
  );
}
