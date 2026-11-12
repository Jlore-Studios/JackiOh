// Pure statistics for the busy-board frame probe (issue #3605; SPEC §2.4, §4). No DOM, no clock,
// no I/O: the durations in, the verdict out, so the unit tests and the Cypress spec share the
// judgement (CLAUDE.md rule 9: the budgets live in frameBudgets.ts).

import { FRAME_DROPPED_FACTOR, FRAME_EXPECTED_FPS } from "./frameBudgets.ts";

/** One phase's frame-time summary, in milliseconds. */
export type PhaseStats = {
  median: number;
  p95: number;
  droppedShare: number;
  count: number;
};

/** A phase's verdict: pass, or the failure naming the budget constant, the value and the budget. */
export type PhaseVerdict = {
  ok: boolean;
  failedBy: string | null;
};

/** The budgets one judged phase is held to, each naming its constant. */
export type PhaseBudget = {
  p95BudgetMs: number;
  p95BudgetName: string;
  longTaskBudget: number;
  longTaskBudgetName: string;
};

/** One phase's report: its stats, its long-task count and its verdict. */
export type PhaseReport = {
  name: string;
  stats: PhaseStats;
  longTasks: number;
  verdict: PhaseVerdict;
};

/** Issue #3605: the measured phases, plus the idle board's running animation count (reported only). */
export type ProbeReport = {
  seed: string;
  phases: PhaseReport[];
  /** The idle board's running `document.getAnimations()` count; null for a report without one. */
  idleAnimations: number | null;
};

/** The frame time over which a frame counts as dropped (issue #3605). */
export function droppedFrameThresholdMs(): number {
  return (1000 / FRAME_EXPECTED_FPS) * FRAME_DROPPED_FACTOR;
}

function quantile(sorted: readonly number[], q: number): number {
  if (sorted.length === 0) return 0;
  const rank = Math.min(sorted.length - 1, Math.max(0, Math.ceil(q * sorted.length) - 1));
  return sorted[rank] ?? 0;
}

/** Summarize requestAnimationFrame deltas: median, 95th percentile and dropped share. */
export function summarizeFrameSamples(samplesMs: readonly number[]): PhaseStats {
  if (samplesMs.length === 0) return { median: 0, p95: 0, droppedShare: 0, count: 0 };
  const sorted = [...samplesMs].sort((a, b) => a - b);
  const middle = (sorted.length - 1) / 2;
  const median = (sorted[Math.floor(middle)] ?? 0) + (sorted[Math.ceil(middle)] ?? 0);
  const threshold = droppedFrameThresholdMs();
  const dropped = samplesMs.filter((sample) => sample > threshold).length;
  return {
    median: median / 2,
    p95: quantile(sorted, 0.95),
    droppedShare: dropped / samplesMs.length,
    count: samplesMs.length,
  };
}

/**
 * Judge one phase: the p95 over its budget fails first, else the long-task count over its own.
 * The failure names the constant, the measured value and the budget, so the Cypress log says
 * what to raise and where it lives.
 */
export function judgePhase(stats: PhaseStats, longTasks: number, budget: PhaseBudget): PhaseVerdict {
  if (stats.p95 > budget.p95BudgetMs) {
    return {
      ok: false,
      failedBy: `${budget.p95BudgetName}: p95 ${stats.p95.toFixed(2)}ms over budget ${budget.p95BudgetMs}ms`,
    };
  }
  if (longTasks > budget.longTaskBudget) {
    return {
      ok: false,
      failedBy: `${budget.longTaskBudgetName}: ${longTasks} long tasks over budget ${budget.longTaskBudget}`,
    };
  }
  return { ok: true, failedBy: null };
}
