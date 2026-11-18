// (target) The API serves no evaluations yet. These typed samples stand in until an endpoint exists, and every
// screen that shows them says so. Replace with generated types when the endpoint lands.
export type EvaluationKind = "Speed" | "Quality" | "Size" | "Cost";

export interface EvaluationStub {
  id: string;
  name: string;
  summary: string;
  kind: EvaluationKind;
  metric: string;
  command: string;
  protectedPaths: string[];
  sealChecksPassed: boolean;
  usedIn: { id: string; title: string }[];
}

export const EVALUATIONS: readonly EvaluationStub[] = [
  {
    id: "load-test",
    name: "Load test",
    summary: "Replays traffic on a fresh machine and reports latency percentiles",
    kind: "Speed",
    metric: "p95 latency, lower is better",
    command: "k6 run bench/load.js --vus 200 --duration 5m",
    protectedPaths: ["bench/", "fixtures/"],
    sealChecksPassed: true,
    usedIn: [],
  },
  {
    id: "test-suite-duration",
    name: "Test suite duration",
    summary: "Runs the test suite on a fresh machine and reports wall time",
    kind: "Speed",
    metric: "Suite duration, lower is better",
    command: "make test-ci",
    protectedPaths: ["tests/"],
    sealChecksPassed: true,
    usedIn: [],
  },
  {
    id: "bundle-size",
    name: "Bundle size",
    summary: "Builds the production bundle and reports its gzipped size",
    kind: "Size",
    metric: "Gzipped bundle size, lower is better",
    command: "pnpm build && node scripts/size.js",
    protectedPaths: ["scripts/size.js"],
    sealChecksPassed: false,
    usedIn: [],
  },
];

export function evaluation(id: string): EvaluationStub | undefined {
  return EVALUATIONS.find((item) => item.id === id);
}
