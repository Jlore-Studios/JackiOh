// The busy-board frame probe's sampler (issue #3605; SPEC §2.4, §4). It runs against the
// application's window (`Cypress.AUTWindow`): requestAnimationFrame deltas for a fixed number of
// seconds, a `PerformanceObserver("longtask")` count over the same window, and the count of
// running `document.getAnimations()`. Judgement stays in `apps/web/src/perf/frameStats.ts`, so the
// spec and the unit tests share it (CLAUDE.md rule 9).

import {
  FRAME_LONG_TASK_BUDGET_COMBAT,
  FRAME_LONG_TASK_BUDGET_IDLE,
  FRAME_P95_BUDGET_COMBAT_MS,
  FRAME_P95_BUDGET_IDLE_MS,
} from "../../apps/web/src/perf/frameBudgets.ts";
import {
  judgePhase,
  summarizeFrameSamples,
  type PhaseBudget,
  type PhaseReport,
  type PhaseStats,
  type PhaseVerdict,
  type ProbeReport,
} from "../../apps/web/src/perf/frameStats.ts";

/** Issue #3605: the idle phase's budgets, naming the constants the verdict cites. */
export const IDLE_BUDGET: PhaseBudget = {
  p95BudgetMs: FRAME_P95_BUDGET_IDLE_MS,
  p95BudgetName: "FRAME_P95_BUDGET_IDLE_MS",
  longTaskBudget: FRAME_LONG_TASK_BUDGET_IDLE,
  longTaskBudgetName: "FRAME_LONG_TASK_BUDGET_IDLE",
};

/** Issue #3605: the combat phase's budgets. */
export const COMBAT_BUDGET: PhaseBudget = {
  p95BudgetMs: FRAME_P95_BUDGET_COMBAT_MS,
  p95BudgetName: "FRAME_P95_BUDGET_COMBAT_MS",
  longTaskBudget: FRAME_LONG_TASK_BUDGET_COMBAT,
  longTaskBudgetName: "FRAME_LONG_TASK_BUDGET_COMBAT",
};

/** One sampling window's raw findings. */
export type FrameSample = {
  samplesMs: number[];
  longTasks: number;
  runningAnimations: number;
};

/**
 * Start one sampling window on the application's window. The probe is passive listeners in the
 * page: `done` resolves once `seconds` elapse, so the combat phase can act through the UI while
 * the window runs (Cypress commands stay serial; the page keeps sampling underneath them).
 */
export function startProbe(win: Cypress.AUTWindow, seconds: number): { done: () => Promise<FrameSample> } {
  let longTasks = 0;
  const observer = new win.PerformanceObserver((entries) => {
    longTasks += entries.getEntries().length;
  });
  observer.observe({ entryTypes: ["longtask"] });
  const samplesMs: number[] = [];
  const deadline = win.performance.now() + seconds * 1000;
  let previous: number | null = null;
  let stopped = false;
  const tick = (now: number): void => {
    if (stopped) return;
    if (previous !== null) samplesMs.push(now - previous);
    previous = now;
    if (now < deadline) win.requestAnimationFrame(tick);
  };
  win.requestAnimationFrame(tick);
  return {
    done: () =>
      new Promise<FrameSample>((resolve) => {
        const wait = Math.max(0, deadline - win.performance.now());
        win.setTimeout(() => {
          stopped = true;
          observer.disconnect();
          const runningAnimations = win.document
            .getAnimations()
            .filter((animation) => animation.playState === "running").length;
          resolve({ samplesMs, longTasks, runningAnimations });
        }, wait);
      }),
  };
}

/**
 * Hover the board minion matching `selector` with real mouse events. Headless Chromium has no
 * pointer, and `trigger("mouseover")` does not always engage CSS `:hover`, so the idle phase
 * approximates a hovering pointer this way rather than claiming pointer-exact costs.
 */
export function hoverBoardMinion(win: Cypress.AUTWindow, selector: string): void {
  const element = win.document.querySelector(selector);
  if (element === null) throw new Error(`hoverBoardMinion: nothing matches ${selector}`);
  const box = element.getBoundingClientRect();
  const init = {
    bubbles: true,
    composed: true,
    view: win,
    clientX: box.left + box.width / 2,
    clientY: box.top + box.height / 2,
  };
  element.dispatchEvent(new win.MouseEvent("mouseover", init));
  element.dispatchEvent(new win.MouseEvent("mousemove", init));
}

/** Wire one sampling window through the shared statistics and judgement. */
export function reportPhase(name: string, sample: FrameSample, budget: PhaseBudget): PhaseReport {
  const stats: PhaseStats = summarizeFrameSamples(sample.samplesMs);
  const verdict: PhaseVerdict = judgePhase(stats, sample.longTasks, budget);
  return { name, stats, longTasks: sample.longTasks, verdict };
}

/** One human-readable line per phase, for `cy.task("log", …)`. */
export function formatPhaseLine(report: PhaseReport): string {
  const { stats } = report;
  const verdict = report.verdict.ok ? "PASS" : `FAIL ${report.verdict.failedBy ?? ""}`;
  return (
    `[frame-probe] ${report.name}: median ${stats.median.toFixed(2)}ms, ` +
    `p95 ${stats.p95.toFixed(2)}ms, dropped ${(stats.droppedShare * 100).toFixed(1)}% ` +
    `(${stats.count} frames), long tasks ${report.longTasks} — ${verdict}`
  );
}

/** The whole probe as human-readable lines. */
export function formatProbeReport(report: ProbeReport): string {
  const lines = report.phases.map(formatPhaseLine);
  if (report.idleAnimations !== null) {
    lines.push(`[frame-probe] idle running animations: ${report.idleAnimations}`);
  }
  return lines.join("\n");
}
