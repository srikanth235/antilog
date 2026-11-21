import type { Schemas } from "../api/client";

const REPO = /^[\w.-]+\/[\w.-]+$/;
const BRANCH = /^[\w./-]+$/;

/** Repos used in the organization's sessions, newest session first, then the environments' default repos. */
export function recentRepos(
  sessions: readonly Schemas["Session"][],
  environments: readonly Schemas["Environment"][],
): string[] {
  const newest = [...sessions].sort((a, b) => b.createdAt - a.createdAt);
  const all = [
    ...newest.flatMap((session) => session.repos),
    ...environments.filter((item) => !item.archived).flatMap((item) => item.repos),
  ];
  return [...new Set(all)];
}

/** The repos whose name holds every word of the query. */
export function filterRepos(repos: readonly string[], query: string): string[] {
  const words = query.toLowerCase().split(/\s+/).filter(Boolean);
  return repos.filter((repo) => words.every((word) => repo.toLowerCase().includes(word)));
}

/** What the query offers to use: a well-formed owner/repo that the list does not have. */
export function typedRepo(query: string, repos: readonly string[]): string | null {
  const text = query.trim();
  return REPO.test(text) && !repos.includes(text) ? text : null;
}

/** Adds the repo to the choice, or takes it out when it is there. */
export function toggleRepo(chosen: readonly string[], repo: string): string[] {
  return chosen.includes(repo) ? chosen.filter((item) => item !== repo) : [...chosen, repo];
}

export function validBranch(branch: string): boolean {
  return BRANCH.test(branch) && !branch.includes("..") && !/^[-/]/.test(branch);
}

/** The label of the repos button: none, the one, or the first and how many more. */
export function reposLabel(
  chosen: readonly string[],
  labels: { none: string; more: (first: string, rest: number) => string },
): string {
  const [first] = chosen;
  if (first === undefined) return labels.none;
  return chosen.length === 1 ? first : labels.more(first, chosen.length - 1);
}
