import { useQueryClient } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";
import { type ReactNode, useState } from "react";
import {
  keys,
  useAgentLabel,
  useMember,
  useMembers,
  useProjects,
  useSendMessage,
  useSession,
  useSessionApprovals,
  useSessionSpend,
  useStartSession,
  useStopSession,
  useTranscript,
} from "../../api/queries";
import { Failed, Loading } from "../../components/state/State";
import { activityOf, canEdit, MCP_AGENT, pendingCards } from "../../lib/domain";
import { groupEvents } from "../../lib/toolcalls";
import { w } from "../../words";
import { ApprovalCard } from "./ApprovalCard";
import { BudgetDialog } from "./BudgetDialog";
import { ChangesPanel } from "./Changes";
import { Composer } from "./Composer";
import { Header, type Panel } from "./Header";
import { OverviewPanel, PlatformPanel } from "./Panels";
import { TerminalPanel } from "./Terminal";
import { ToolGroup } from "./ToolCalls";
import { Message, TranscriptError } from "./Transcript";

/** The session page: the chat of one session and the cards it raised. */
export function Session(props: {
  sessionId: string;
  panel: Panel | undefined;
  onPanel: (panel: Panel | undefined) => void;
}) {
  const { sessionId } = props;
  const session = useSession(sessionId);
  const approvals = useSessionApprovals(sessionId);
  const projects = useProjects();
  const members = useMembers();
  const me = useMember();
  const spend = useSessionSpend(sessionId);
  const agentLabel = useAgentLabel();
  const client = useQueryClient();
  const navigate = useNavigate();
  const [budgetOpen, setBudgetOpen] = useState(false);
  const [confirmStop, setConfirmStop] = useState(false);

  const active = session.data;
  const send = useSendMessage(sessionId);
  const transcript = useTranscript(sessionId, active?.status);
  const start = useStartSession(active?.projectId ?? "");
  const stop = useStopSession();

  if (session.isPending)
    return (
      <div className="wsp">
        <div className="chat">
          <Loading />
        </div>
      </div>
    );
  if (session.isError) return <Failed error={session.error} retry={() => void session.refetch()} />;
  const s = session.data;
  const project = projects.data?.find((p) => p.id === s.projectId);
  const editor = canEdit(me?.role);
  const cards = approvals.data ?? [];
  const waiting = pendingCards(cards);
  const activity = activityOf([s], waiting.length);
  const nameOf = (id: string | null | undefined) =>
    id == null
      ? w.card.allowedByPolicy
      : (members.data?.find((member) => member.id === id)?.name ?? w.card.allowedByPolicy);

  const events = transcript.data ?? [];
  const opening = events.find((event) => event.kind === "person");
  // The conversation in time order: what was said, and the cards raised, where they happened.
  const timeline: { key: string; at: number; rank: number; node: ReactNode }[] = [];
  timeline.push({
    key: "start",
    at: s.createdAt,
    rank: 1,
    node: (
      <div key="start" className="tl">
        {s.agent === MCP_AGENT
          ? w.experiment.connectedAgent
          : w.experiment.startedSession(agentLabel(s.agent))}
      </div>
    ),
  });
  const blocks = groupEvents(events);
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
            running={index === blocks.length - 1 && s.status === "working"}
          />
        ),
      });
      return;
    }
    const { event } = block;
    timeline.push({
      key: `event-${event.seq}`,
      at: event === opening ? Number.NEGATIVE_INFINITY : event.at,
      rank: 4 + event.seq / 1e12,
      node: <Message key={`event-${event.seq}`} event={event} agent={agentLabel(s.agent)} />,
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
          session={s.id}
          canDecide={editor}
          decidedBy={nameOf(approval.decidedBy)}
          budgetCapCents={s.budgetCapCents ?? null}
          spentCents={spend.data?.spentCents}
        />
      ),
    });
  timeline.sort((a, b) => (a.at === b.at ? a.rank - b.rank : a.at < b.at ? -1 : 1));

  return (
    <>
      <Header
        project={project}
        title={s.title}
        activity={activity}
        panel={props.panel}
        onPanel={props.onPanel}
        canStop={s.status === "queued" || s.status === "working" || s.status === "waiting_for_you"}
        onStop={() => setConfirmStop(true)}
        readOnly={!editor && me !== undefined}
      />
      <div className="wsp">
        <div className="chat">
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
              {opening === undefined && transcript.isSuccess ? (
                <div className="bubble">
                  <span>{s.title}</span>
                </div>
              ) : null}
              {timeline.map((item) => item.node)}
              <TranscriptError error={transcript.error} onRetry={() => void transcript.refetch()} />
              {s.status === "failed" ? (
                <div className="tl" role="alert">
                  {s.note ? w.experiment.sessionFailedWith(s.note) : w.experiment.sessionFailed}
                </div>
              ) : null}
              {s.status === "stopped" ? (
                <div className="tl">{s.note ?? w.activity.stopped}</div>
              ) : null}
              {s.status === "done" && waiting.length === 0 ? (
                <div className="tl">{w.experiment.sessionDone}</div>
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
                          stop.mutate(s.id, { onSuccess: () => setConfirmStop(false) })
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
            status={s.status}
            hasCard={waiting.length > 0}
            canSend={editor}
            agent={agentLabel(s.agent)}
            budgetCents={s.budgetCapCents ?? null}
            onInterrupt={() => setConfirmStop(true)}
            onSend={async (text) => {
              await send.mutateAsync(text);
              await client.invalidateQueries({ queryKey: keys.transcript(s.id) });
            }}
            onContinue={async (text) => {
              const created = await start.mutateAsync({
                agent: s.agent,
                model: s.model,
                prompt: text,
                repos: s.repos,
                ...(s.environment == null ? {} : { environment: s.environment }),
              });
              await navigate({ to: "/sessions/$sessionId", params: { sessionId: created.id } });
            }}
          />
        </div>
        {props.panel === "details" ? (
          <OverviewPanel
            session={s}
            spentCents={spend.data?.spentCents}
            onClose={() => props.onPanel(undefined)}
            onBudget={() => setBudgetOpen(true)}
            canEdit={editor}
          />
        ) : props.panel === "changes" ? (
          <ChangesPanel sessionId={s.id} onClose={() => props.onPanel(undefined)} />
        ) : props.panel === undefined || props.panel === "terminal" ? null : (
          <PlatformPanel panel={props.panel} onClose={() => props.onPanel(undefined)} />
        )}
        <TerminalPanel
          sessionId={s.id}
          visible={props.panel === "terminal"}
          onClose={() => props.onPanel(undefined)}
        />
      </div>
      <BudgetDialog open={budgetOpen} onClose={() => setBudgetOpen(false)} session={s} />
    </>
  );
}
