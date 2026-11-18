import { useQueryClient } from "@tanstack/react-query";
import { Link, useNavigate } from "@tanstack/react-router";
import { useState } from "react";
import {
  keys,
  useExperiment,
  useExperimentApprovals,
  useExperimentSessions,
  useHistory,
  useMember,
  useMembers,
  useProjects,
  useSendMessage,
  useStartSession,
  useStopSession,
  useTranscript,
} from "../../api/queries";
import { Failed, Loading } from "../../components/state/State";
import {
  activityOf,
  agentName,
  canEdit,
  leadSession,
  pendingCards,
  platformName,
} from "../../lib/domain";
import { parseHistory } from "../../lib/history";
import { w } from "../../words";
import { ApprovalCard } from "./ApprovalCard";
import { BudgetDialog } from "./BudgetDialog";
import { Composer } from "./Composer";
import { Header, type Panel } from "./Header";
import { DetailsPanel, PlatformPanel, TrialsPanel } from "./Panels";
import { Scoreboard } from "./Scoreboard";
import { Transcript } from "./Transcript";
import { TrialCard } from "./TrialCard";
import { Win } from "./Win";

/** The experiment page: the chat of its lead session, its cards, and the judge's numbers. */
export function Experiment(props: {
  experimentId: string;
  sessionId: string | undefined;
  panel: Panel | undefined;
  onPanel: (panel: Panel | undefined) => void;
}) {
  const { experimentId } = props;
  const experiment = useExperiment(experimentId);
  const sessions = useExperimentSessions(experimentId);
  const approvals = useExperimentApprovals(experimentId);
  const history = useHistory(experimentId);
  const projects = useProjects();
  const members = useMembers();
  const me = useMember();
  const client = useQueryClient();
  const navigate = useNavigate();
  const [budgetOpen, setBudgetOpen] = useState(false);
  const [confirmStop, setConfirmStop] = useState(false);

  const list = sessions.data ?? [];
  const lead = leadSession(list);
  const active = list.find((session) => session.id === props.sessionId) ?? lead;
  const send = useSendMessage(experimentId, active?.id ?? "");
  const transcript = useTranscript(active?.id ?? "");
  const start = useStartSession(experimentId);
  const stop = useStopSession(experimentId);

  if (experiment.isPending)
    return (
      <div className="wsp">
        <div className="chat">
          <Loading />
        </div>
      </div>
    );
  if (experiment.isError)
    return <Failed error={experiment.error} retry={() => void experiment.refetch()} />;
  const e = experiment.data;
  const project = projects.data?.find((p) => p.id === e.projectId);
  const editor = canEdit(me?.role);
  const cards = approvals.data ?? [];
  const waiting = pendingCards(cards);
  const activity = activityOf(list, waiting.length);
  const parsed = parseHistory(history.data);
  const nameOf = (id: string | null | undefined) =>
    members.data?.find((member) => member.id === id)?.name ?? w.card.allowedByPolicy;
  const someone = (id: string | null | undefined) =>
    id == null ? w.card.allowedByPolicy : nameOf(id);
  const stopped = active?.status === "done" && active.note?.toLowerCase().includes("stopped");

  const refresh = async () => {
    await client.invalidateQueries({ queryKey: keys.approvals(experimentId) });
    await client.invalidateQueries({ queryKey: keys.history(experimentId) });
  };

  return (
    <>
      <Header
        project={project}
        title={e.goal}
        experimentId={e.id}
        state={e.status}
        activity={activity}
        panel={props.panel}
        onPanel={props.onPanel}
        canStop={active?.status === "working" || active?.status === "waiting_for_you"}
        onStop={() => setConfirmStop(true)}
        readOnly={!editor && me !== undefined}
      />
      <div className="wsp">
        <div className="chat">
          {e.status === "sealed" ? (
            <Scoreboard
              history={parsed}
              experiment={e.id}
              budgetCents={e.budgetCapCents}
              metric={null}
            />
          ) : null}
          <div
            className="scroll"
            style={{
              display: "flex",
              flexDirection: "column",
              justifyContent: "flex-end",
              overflow: "auto",
            }}
          >
            <div className="c760 thread" style={{ gap: 12, paddingTop: 12, paddingBottom: 8 }}>
              {transcript.isSuccess && !transcript.data.some((event) => event.kind === "person") ? (
                <div className="bubble">
                  <span>{e.goal}</span>
                </div>
              ) : null}
              {e.status === "not_sealed" ? (
                <div className="tl">{w.experiment.notSealedLine}</div>
              ) : (
                <div className="tl">{w.experiment.sealedLine}</div>
              )}
              {list.map((session) => (
                <div key={session.id} className="tl">
                  {w.experiment.startedSession(
                    agentName(session.agent),
                    platformName(session.platform),
                  )}
                </div>
              ))}
              {list
                .filter((session) => session.id !== lead?.id)
                .map((session) => (
                  <Link
                    key={session.id}
                    className="cd spc"
                    to="/experiments/$experimentId"
                    params={{ experimentId: e.id }}
                    search={{ session: session.id }}
                  >
                    <span className="tx">
                      <span className="nm">{w.sidebar.session(agentName(session.agent))}</span>
                    </span>
                    <span className="go">
                      <span className="spill">{w.activity[session.status]}</span>
                    </span>
                  </Link>
                ))}
              {parsed.trials.map((row) => (
                <TrialCard key={row.trial} row={row} unit={parsed.unit} />
              ))}
              {parsed.best === null ? null : (
                <Win
                  history={parsed}
                  trial={parsed.best}
                  experiment={e.id}
                  canDeliver={editor}
                  onCard={() => void refresh()}
                />
              )}
              {cards.map((approval) => (
                <ApprovalCard
                  key={approval.id}
                  approval={approval}
                  experiment={e.id}
                  canDecide={editor}
                  decidedBy={someone(approval.decidedBy)}
                />
              ))}
              <Transcript
                events={transcript.data ?? []}
                agent={active?.agent ?? ""}
                error={transcript.error}
                onRetry={() => void transcript.refetch()}
              />
              {active?.status === "failed" ? (
                <div className="tl" role="alert">
                  {w.experiment.sessionFailed}
                </div>
              ) : null}
              {active?.status === "done" && parsed.trials.length === 0 && waiting.length === 0 ? (
                <div className="tl">
                  {stopped ? w.experiment.stoppedBy(w.you) : w.experiment.sessionDone}
                </div>
              ) : null}
              {active?.status === "done" && parsed.trials.length > 0 && parsed.best === null ? (
                <div className="tl">
                  <b>{w.experiment.noWinTitle}.</b> {w.experiment.noWin}
                </div>
              ) : null}
              {confirmStop ? (
                <section className="cd ap" aria-label={w.experiment.stopSession}>
                  <p className="ap-lead">{w.experiment.stopConfirm}</p>
                  <div className="ap-f">
                    <div className="bt">
                      <button
                        type="button"
                        className="btn primary"
                        disabled={stop.isPending}
                        onClick={() =>
                          active !== undefined &&
                          stop.mutate(active.id, { onSuccess: () => setConfirmStop(false) })
                        }
                      >
                        {w.experiment.stopSession}
                      </button>
                      <button type="button" className="btn" onClick={() => setConfirmStop(false)}>
                        {w.experiment.keepWorking}
                      </button>
                    </div>
                  </div>
                </section>
              ) : null}
            </div>
          </div>
          <Composer
            status={active?.status ?? "none"}
            canSend={editor}
            agent={active === undefined ? w.home.auto : agentName(active.agent)}
            budgetCents={e.budgetCapCents}
            onInterrupt={() => setConfirmStop(true)}
            onSend={async (text) => {
              if (active === undefined) return;
              await send.mutateAsync(text);
              await client.invalidateQueries({ queryKey: keys.transcript(active.id) });
            }}
            onContinue={async (text) => {
              const created = await start.mutateAsync({
                agent: active?.agent ?? "auto",
                prompt: text,
                repos: [],
              });
              await navigate({
                to: "/experiments/$experimentId",
                params: { experimentId: e.id },
                search: { session: created.id },
              });
            }}
          />
        </div>
        {props.panel === "details" ? (
          <DetailsPanel
            experiment={e}
            sessions={list}
            onClose={() => props.onPanel(undefined)}
            onBudget={() => setBudgetOpen(true)}
            canEdit={editor}
          />
        ) : props.panel === "trials" ? (
          <TrialsPanel history={parsed} sessions={list} onClose={() => props.onPanel(undefined)} />
        ) : props.panel === undefined ? null : (
          <PlatformPanel panel={props.panel} onClose={() => props.onPanel(undefined)} />
        )}
      </div>
      <BudgetDialog open={budgetOpen} onClose={() => setBudgetOpen(false)} experiment={e} />
    </>
  );
}
