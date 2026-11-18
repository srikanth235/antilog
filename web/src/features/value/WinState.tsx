import type { Schemas } from "../../api/client";
import { useExperimentValue } from "../../api/queries";
import { w } from "../../words";

const COLOR: Record<Schemas["WinState"], { color: string; hollow: boolean }> = {
  delivered: { color: "var(--good)", hollow: false },
  pending: { color: "var(--warn)", hollow: true },
  atRisk: { color: "var(--bad)", hollow: false },
  notOpened: { color: "var(--ink3)", hollow: true },
};

/** Delivered, Pending, At risk or No pull request yet, as the board's chip. */
export function WinState({ state }: { state: Schemas["WinState"] }) {
  const look = COLOR[state];
  return (
    <span className={`tg${look.hollow ? " hollow" : ""}`} style={{ ["--c" as string]: look.color }}>
      {w.value.wins[state]}
    </span>
  );
}

/** The win of one trial as the value endpoint states it: its state and what it is worth a year. */
export function useWinValue(experiment: string, trial: string): Schemas["WinView"] | undefined {
  return useExperimentValue(experiment).data?.wins.find((win) => win.trialId === trial);
}
