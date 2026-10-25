// R1935000: what the Daily runs tab sums up from the runs it lists (SPEC §9.11). A run still running
// has no result yet, so every figure reads the finished runs alone.

import type { CiRun } from "../net/api.ts";

type Finished = "passed" | "failed";

export type CiInsights = {
  /** The finished runs listed. */
  finished: number;
  /** How many of them passed. */
  passed: number;
  /** The newest finished run's result and how many finished runs in a row, newest first, share it; null with none finished. */
  streak: { result: Finished; runs: number } | null;
  /** The job that did not pass in the most finished runs, and in how many; null when none failed. A tie goes to the newer run's job. */
  mostFailed: { name: string; runs: number } | null;
};

export function ciInsights(runs: readonly CiRun[]): CiInsights {
  const finished = runs.filter((run): run is CiRun & { result: Finished } => run.result !== "running");
  const newest = finished[0];
  let streak: CiInsights["streak"] = null;
  if (newest !== undefined) {
    const broken = finished.findIndex((run) => run.result !== newest.result);
    streak = { result: newest.result, runs: broken === -1 ? finished.length : broken };
  }
  // Newest run first, so a name met first belongs to the newer run and keeps a tie.
  const failedIn = new Map<string, number>();
  for (const run of finished) {
    for (const name of new Set(run.failedJobs.map((job) => job.name))) {
      failedIn.set(name, (failedIn.get(name) ?? 0) + 1);
    }
  }
  let mostFailed: CiInsights["mostFailed"] = null;
  for (const [name, count] of failedIn) {
    if (mostFailed === null || count > mostFailed.runs) mostFailed = { name, runs: count };
  }
  return {
    finished: finished.length,
    passed: finished.filter((run) => run.result === "passed").length,
    streak,
    mostFailed,
  };
}
