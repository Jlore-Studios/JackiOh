// R1935000: the Daily runs tab's summary of the runs it lists.

import { describe, expect, it } from "vitest";

import type { CiRun } from "../net/api.ts";
import { ciInsights } from "./ciInsights.ts";

function run(id: number, result: CiRun["result"], failed: string[] = []): CiRun {
  return {
    id,
    url: `https://github.com/o/r/actions/runs/${String(id)}`,
    startedAt: "2026-10-25T07:17:00Z",
    commit: `sha${String(id)}`,
    result,
    failedJobs: failed.map((name) => ({ name, url: `u/${name}`, conclusion: "failure" })),
  };
}

describe("R1935000 the daily runs' insights", () => {
  it("R1935000 counts the finished runs that passed and leaves a running one out of every figure", () => {
    const insights = ciInsights([
      run(5, "running"),
      run(4, "failed", ["fuzz (1/4)"]),
      run(3, "failed", ["fuzz (1/4)", "e2e (4/8)"]),
      run(2, "passed"),
      run(1, "failed", ["e2e (4/8)"]),
    ]);
    expect(insights.finished).toBe(4);
    expect(insights.passed).toBe(1);
    expect(insights.streak).toEqual({ result: "failed", runs: 2 });
  });

  it("R1935000 the streak runs to the first finished run with the other result, or through every one", () => {
    expect(ciInsights([run(3, "passed"), run(2, "passed"), run(1, "failed", ["a"])]).streak).toEqual({ result: "passed", runs: 2 });
    expect(ciInsights([run(2, "passed"), run(1, "passed")]).streak).toEqual({ result: "passed", runs: 2 });
    expect(ciInsights([run(1, "running")]).streak).toBeNull();
    expect(ciInsights([]).streak).toBeNull();
  });

  it("R1935000 the job that failed most is the one that failed in the most runs, a tie going to the newer run's job", () => {
    const most = ciInsights([
      run(4, "failed", ["golden"]),
      run(3, "failed", ["fuzz (2/4)", "golden"]),
      run(2, "failed", ["fuzz (2/4)"]),
      run(1, "failed", ["fuzz (2/4)"]),
    ]).mostFailed;
    expect(most).toEqual({ name: "fuzz (2/4)", runs: 3 });

    const tied = ciInsights([run(2, "failed", ["golden"]), run(1, "failed", ["fuzz (2/4)"])]).mostFailed;
    expect(tied).toEqual({ name: "golden", runs: 1 });

    // A job listed twice in one run counts that run once.
    expect(ciInsights([run(1, "failed", ["db", "db"])]).mostFailed).toEqual({ name: "db", runs: 1 });
    expect(ciInsights([run(2, "passed"), run(1, "running")]).mostFailed).toBeNull();
  });
});
