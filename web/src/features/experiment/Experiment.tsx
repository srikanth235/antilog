import { useQueryClient } from "@tanstack/react-query";
import { Link, useNavigate } from "@tanstack/react-router";
import { type ReactNode, useState } from "react";
import {
  keys,
  useAgentLabel,
  useConnectors,
  useExperiment,
  useExperimentApprovals,
  useExperimentConnectors,
  useExperimentSessions,
  useExperimentSpend,
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
import { activityOf, canEdit, leadSession, MCP_AGENT, pendingCards } from "../../lib/domain";
import { parseHistory } from "../../lib/history";
import { groupEvents } from "../../lib/toolcalls";
import { w } from "../../words";
import { ApprovalCard } from "./ApprovalCard";
import { BudgetDialog } from "./BudgetDialog";
import { Composer } from "./Composer";
import { Header, type Panel } from "./Header";
import { OverviewPanel, PlatformPanel, TrialsPanel } from "./Panels";
import { Scoreboard } from "./Scoreboard";
import { ToolGroup } from "./ToolCalls";
import { Message, TranscriptError } from "./Transcript";
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
  const spend = useExperimentSpend(experimentId);
  const allConnectors = useConnectors();
  const switchedOn = useExperimentConnectors(experimentId);
  const agentLabel = useAgentLabel();
  const client = useQueryClient();
  const navigate = useNavigate();
  const [budgetOpen, setBudgetOpen] = useState(false);
  const [confirmStop, setConfirmStop] = useState(false);

  const list = sessions.data ?? [];
  const lead = leadSession(list);
  const active = list.find((session) => session.id === props.sessionId) ?? lead;
  const send = useSendMessage(experimentId, active?.id ?? "");
  const transcript = useTranscript(active?.id ?? "", active?.status);
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
  const onConnectors = (switchedOn.data ?? []).flatMap((id) => {
    const found = allConnectors.data?.find((connector) => connector.id === id);
    return found === undefined ? [] : [{ id, name: found.name }];
  });
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

  const events = transcript.data ?? [];
  const opening = events.find((event) => event.kind === "person");
  // The conversation in time order: what was said, the sessions started and the cards raised, where they happened.
  const timeline: { key: string; at: number; rank: number; node: ReactNode }[] = [];
  if (opening !== undefined)
    timeline.push({
      key: `event-${opening.seq}`,
      at: Number.NEGATIVE_INFINITY,
      rank: 0,
      node: (
        <div key={`event-${opening.seq}`} className="bubble">
          <span>{opening.text}</span>
        </div>
      ),
    });
  timeline.push({
    key: "sealed",
    at: e.createdAt,
    rank: 1,
    node: (
      <div key="sealed" className="tl">
        {e.status === "not_sealed" ? w.experiment.notSealedLine : w.experiment.sealedLine}
      </div>
    ),
  });
  for (const session of list) {
    timeline.push({
      key: `start-${session.id}`,
      at: session.createdAt,
      rank: 2,
      node: (
        <div key={`start-${session.id}`} className="tl">
          {session.agent === MCP_AGENT
            ? w.experiment.connectedAgent
            : w.experiment.startedSession(agentLabel(session.agent))}
        </div>
      ),
    });
    if (session.id !== lead?.id)
      timeline.push({
        key: `session-${session.id}`,
        at: session.createdAt,
        rank: 3,
        node: (
          <Link
            key={`session-${session.id}`}
            className="cd spc"
            to="/experiments/$experimentId"
            params={{ experimentId: e.id }}
            search={{ session: session.id }}
          >
            <span className="tx">
              <span className="nm">{w.sidebar.session(agentLabel(session.agent))}</span>
            </span>
            <span className="go">
              <span className="spill">{w.activity[session.status]}</span>
            </span>
          </Link>
        ),
      });
  }
  const blocks = groupEvents(events.filter((event) => event !== opening));
  blocks.forEach((block, index) => {
    if (block.type === "tools") {
      const first = events.find((event) => event.seq === block.group.key);
      timeline.push({
        key: `tools-${block.group.key}`,
        at: first?.at ?? 0,
        rank: 4 + block.group.key / 1e12,
        node: (
          <ToolGroup
            key={`tools-${block.group.key}`}
            group={block.group}
            running={index === blocks.length - 1 && active?.status === "working"}
          />
        ),
      });
      return;
    }
    const { event } = block;
    timeline.push({
      key: `event-${event.seq}`,
      at: event.at,
      rank: 4 + event.seq / 1e12,
      node: (
        <Message key={`event-${event.seq}`} event={event} agent={agentLabel(active?.agent ?? "")} />
      ),
    });
  });
  for (const approval of cards)
    timeline.push({
      key: `card-${approval.id}`,
      at: approval.createdAt,
      rank: 5,
      node: (
        <ApprovalCard
          key={approval.id}
          approval={approval}
          experiment={e.id}
          canDecide={editor}
          decidedBy={someone(approval.decidedBy)}
          budgetCapCents={e.budgetCapCents}
          spentCents={spend.data?.spentCents}
          history={parsed}
        />
      ),
    });
  timeline.sort((a, b) => (a.at === b.at ? a.rank - b.rank : a.at < b.at ? -1 : 1));
  // Trials carry no time yet, so they follow what has a time. (API work owed)
  for (const row of parsed.trials)
    timeline.push({
      key: `trial-${row.trial}`,
      at: Number.POSITIVE_INFINITY,
      rank: row.number,
      node: <TrialCard key={`trial-${row.trial}`} row={row} unit={parsed.unit} />,
    });
  if (parsed.best !== null)
    timeline.push({
      key: "win",
      at: Number.POSITIVE_INFINITY,
      rank: Number.MAX_SAFE_INTEGER,
      node: (
        <Win
          key="win"
          history={parsed}
          trial={parsed.best}
          experiment={e.id}
          canDeliver={editor}
          onCard={() => void refresh()}
        />
      ),
    });

  const thread = (
    <>
      {opening === undefined && transcript.isSuccess ? (
        <div className="bubble">
          <span>{e.goal}</span>
        </div>
      ) : null}
      {timeline.map((item) => item.node)}
    </>
  );

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
              {thread}
              <TranscriptError error={transcript.error} onRetry={() => void transcript.refetch()} />
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
            hasCard={waiting.length > 0}
            canSend={editor}
            agent={active === undefined ? w.home.auto : agentLabel(active.agent)}
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
          <OverviewPanel
            experiment={e}
            sessions={list}
            history={parsed}
            spentCents={spend.data?.spentCents}
            connectors={onConnectors}
            onClose={() => props.onPanel(undefined)}
            onPanel={props.onPanel}
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
