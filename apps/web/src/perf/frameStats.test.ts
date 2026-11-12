// Issue #3605: the frame probe's statistics fail a slow board and pass a smooth one.

import { describe, expect, it } from "vitest";

import { judgePhase, summarizeFrameSamples } from "./frameStats.ts";

describe("summarizeFrameSamples", () => {
  it("summarizes median, p95 and dropped share", () => {
    const stats = summarizeFrameSamples([8, 8, 16.7, 16.7, 50]);
    expect(stats.median).toBeCloseTo(16.7, 5);
    expect(stats.p95).toBeCloseTo(50, 5);
    expect(stats.droppedShare).toBeCloseTo(1 / 5, 5);
    expect(stats.count).toBe(5);
  });

  it("summarizes an empty sample as zeros", () => {
    expect(summarizeFrameSamples([])).toEqual({ median: 0, p95: 0, droppedShare: 0, count: 0 });
  });
});

describe("judgePhase", () => {
  it("judges p95 over its budget as failed, naming the constant", () => {
    const stats = summarizeFrameSamples([8, 8, 16.7, 16.7, 50]);
    const verdict = judgePhase(stats, 0, {
      p95BudgetMs: 25,
      p95BudgetName: "FRAME_P95_BUDGET_IDLE_MS",
      longTaskBudget: 1,
      longTaskBudgetName: "FRAME_LONG_TASK_BUDGET_IDLE",
    });
    expect(verdict.ok).toBe(false);
    expect(verdict.failedBy ?? "").toContain("FRAME_P95_BUDGET_IDLE_MS");
    expect(verdict.failedBy ?? "").toContain("25");
  });

  it("judges long tasks over their budget as failed, naming the constant", () => {
    const stats = summarizeFrameSamples([8, 8, 16.7, 16.7, 16.7]);
    const verdict = judgePhase(stats, 3, {
      p95BudgetMs: 25,
      p95BudgetName: "FRAME_P95_BUDGET_IDLE_MS",
      longTaskBudget: 1,
      longTaskBudgetName: "FRAME_LONG_TASK_BUDGET_IDLE",
    });
    expect(verdict.ok).toBe(false);
    expect(verdict.failedBy ?? "").toContain("FRAME_LONG_TASK_BUDGET_IDLE");
  });

  it("passes a smooth board", () => {
    const stats = summarizeFrameSamples(Array.from({ length: 300 }, () => 16.7));
    const verdict = judgePhase(stats, 0, {
      p95BudgetMs: 25,
      p95BudgetName: "FRAME_P95_BUDGET_IDLE_MS",
      longTaskBudget: 1,
      longTaskBudgetName: "FRAME_LONG_TASK_BUDGET_IDLE",
    });
    expect(verdict).toEqual({ ok: true, failedBy: null });
  });

  it("fails a board with a deliberately expensive loop", () => {
    const smooth = Array.from({ length: 300 }, () => 16.7);
    const looped = smooth.map((sample, at) => (at % 3 === 2 ? sample + 40 : sample));
    const verdict = judgePhase(summarizeFrameSamples(looped), 0, {
      p95BudgetMs: 25,
      p95BudgetName: "FRAME_P95_BUDGET_IDLE_MS",
      longTaskBudget: 1,
      longTaskBudgetName: "FRAME_LONG_TASK_BUDGET_IDLE",
    });
    expect(verdict.ok).toBe(false);
    expect(verdict.failedBy ?? "").toContain("FRAME_P95_BUDGET_IDLE_MS");
  });
});
