// The busy-board frame probe's numbers (issue #3605; SPEC §2.4's lane, backrow and hand caps,
// §4's combat), and nothing else. CLAUDE.md rule 9: every number the probe judges or times by is
// named here, so the budgets' headroom for CI's shared runners is one place to change. The
// engine's and the server's numbers stay in their crates; this file names only the client's own.

/** The frame rate a smooth board holds, in frames per second: a unit, not configuration. */
export const FRAME_EXPECTED_FPS = 60;

/**
 * A frame counts as dropped when it lasts longer than one expected frame times this factor
 * (issue #3605: the probe's dropped-frame share).
 */
export const FRAME_DROPPED_FACTOR = 1.5;

/** Issue #3605: seconds of requestAnimationFrame deltas the probe's idle phase samples. */
export const FRAME_PROBE_IDLE_SECONDS = 5;

/** Issue #3605: seconds the probe's combat phase samples while a scripted turn plays out. */
export const FRAME_PROBE_COMBAT_SECONDS = 8;

/** Issue #3605: seconds the probe samples with a deliberately expensive loop injected. */
export const FRAME_PROBE_LOOP_SECONDS = 3;

/**
 * Issue #3605: the idle board's 95th-percentile frame time, in milliseconds. Set by the
 * 5×-of-three-runs recipe (`crates/tools/src/perf.rs` header): five times the largest p95 the
 * probe measured on the bot's shared machine, rounded up to two significant figures, so CI's
 * shared runners hold it with headroom.
 */
export const FRAME_P95_BUDGET_IDLE_MS = 25;

/** Issue #3605: the combat phase's 95th-percentile frame time, in milliseconds. Same recipe. */
export const FRAME_P95_BUDGET_COMBAT_MS = 30;

/** Issue #3605: long tasks (`PerformanceObserver "longtask"`) the idle phase may hold. Same recipe. */
export const FRAME_LONG_TASK_BUDGET_IDLE = 1;

/** Issue #3605: long tasks the combat phase may hold. Same recipe. */
export const FRAME_LONG_TASK_BUDGET_COMBAT = 2;
