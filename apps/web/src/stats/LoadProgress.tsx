// R5186000, R5219000: the server's load of the card statistics as /stats shows it while a read waits on it:
// a bar and a line. A bar that stands still reads as a page that stopped, so this one fills on the page's own
// clock and never stops or falls back (R5219000): it creeps on a curve that slows and never stops, three
// quarters full at the time to expect and never full before the read answers. `GET /api/stats/loading`
// (crates/server/src/api/stats.rs) names the step and the time to expect. A read that times out while the
// server still reports a load asks again (`askAgain`), since the load goes on without it.

import { type ReactElement, useEffect, useState } from "react";

import { MS_PER_SECOND } from "../game/clockConstants.ts";
import { t } from "../i18n/i18n.ts";
import { ApiUnreachableError, getStatsLoading, type StatsLoading } from "../net/api.ts";
import {
  STATS_LOADING_AFTER_MS,
  STATS_LOADING_GUESS_MS,
  STATS_LOADING_POLL_MS,
  STATS_LOADING_SHARE_AT_EXPECTED,
  STATS_LOADING_SHARE_MAX,
  STATS_LOADING_TICK_MS,
} from "./config.ts";

const SECONDS_PER_MINUTE = 60;

/** A whole share as a percentage. */
const PERCENT = 100;

/**
 * R5219000: how full the bar is after `elapsedMs`: `t / (t + τ)` of `STATS_LOADING_SHARE_MAX`, with `τ` set so
 * the bar stands at `STATS_LOADING_SHARE_AT_EXPECTED` of it at the time to expect (`expectedMs`, or
 * `STATS_LOADING_GUESS_MS` when none is known). It rises for as long as the load runs, ever slower, and never
 * reaches the end.
 */
export function loadShare(elapsedMs: number, expectedMs: number | null): number {
  const expected = expectedMs !== null && expectedMs > 0 ? expectedMs : STATS_LOADING_GUESS_MS;
  const tau = (expected * (1 - STATS_LOADING_SHARE_AT_EXPECTED)) / STATS_LOADING_SHARE_AT_EXPECTED;
  const elapsed = Math.max(0, elapsedMs);
  return (STATS_LOADING_SHARE_MAX * elapsed) / (elapsed + tau);
}

/** A duration in ms as minutes and seconds, `m:ss`. */
export function clockOf(ms: number): string {
  const seconds = Math.max(0, Math.round(ms / MS_PER_SECOND));
  return `${String(Math.floor(seconds / SECONDS_PER_MINUTE))}:${String(seconds % SECONDS_PER_MINUTE).padStart(2, "0")}`;
}

/** What the load's step `step` (the server's `LoadStep`, in order from 1) is doing. */
function stepWords(step: number): string {
  const words = [t("Reading the games played"), t("Reading the card totals"), t("Working out the figures")];
  return words[Math.min(words.length, Math.max(1, step)) - 1] ?? "";
}

/**
 * The line under the bar after `elapsedMs`: the step when the server reports a load, and the time so far
 * against the time to expect. Past the time to expect it says the load is almost done, not late (R5219000).
 */
export function loadWords(loading: StatsLoading | null, elapsedMs: number): string {
  const doing =
    loading !== null && loading.loading
      ? t("Step {step} of {steps}: {doing}", {
          step: String(loading.step),
          steps: String(loading.steps),
          doing: stepWords(loading.step),
        })
      : t("Loading card statistics…");
  const elapsed = clockOf(elapsedMs);
  const expected = loading?.expectedMs ?? null;
  let time = t("{elapsed} so far", { elapsed });
  if (expected !== null && elapsedMs < expected) {
    time = t("{elapsed} of about {expected}", { elapsed, expected: clockOf(expected) });
  } else if (expected !== null) {
    time = t("{elapsed} · almost done", { elapsed });
  }
  return `${doing} · ${time}`;
}

/**
 * Whether a card statistics read that failed should be asked again rather than shown as not loaded: it
 * found no answer in time (`ApiUnreachableError`) while the server still reports a load, which goes on
 * without the read.
 */
export async function askAgain(cause: unknown, ask: () => Promise<StatsLoading> = getStatsLoading): Promise<boolean> {
  if (!(cause instanceof ApiUnreachableError)) return false;
  try {
    return (await ask()).loading;
  } catch {
    return false;
  }
}

/** What the bar draws: how full it is, and its line. */
type BarView = { share: number; words: string };

/**
 * The progress bar, drawn while it is mounted (the page mounts it while its read waits). From
 * `STATS_LOADING_AFTER_MS` on, whether or not the server has answered, it redraws every
 * `STATS_LOADING_TICK_MS` on the page's own clock, and asks the server every `STATS_LOADING_POLL_MS`. A
 * reported load's elapsed time is carried forward between answers; with none reported, the page's own wait
 * stands in. The bar never draws less than it drew (R5219000).
 */
export function StatsLoadProgress(): ReactElement | null {
  const [view, setView] = useState<BarView | null>(null);

  useEffect(() => {
    const controller = new AbortController();
    const mountedAt = Date.now();
    let heard: { answer: StatsLoading; at: number } | null = null;
    let timer: ReturnType<typeof setTimeout> | undefined;
    let ticker: ReturnType<typeof setInterval> | undefined;
    const ask = async (): Promise<void> => {
      try {
        const answer = await getStatsLoading(controller.signal);
        if (!controller.signal.aborted) heard = { answer, at: Date.now() };
      } catch {
        // A failed ask leaves the bar on its own clock; the next may answer.
      }
      if (!controller.signal.aborted) timer = setTimeout(() => void ask(), STATS_LOADING_POLL_MS);
    };
    const draw = (): void => {
      const now = Date.now();
      const last = heard;
      const answer = last?.answer ?? null;
      const elapsed = last !== null && last.answer.loading ? last.answer.elapsedMs + (now - last.at) : now - mountedAt;
      const share = loadShare(elapsed, answer?.expectedMs ?? null);
      const words = loadWords(answer, elapsed);
      setView((before) => ({ share: Math.max(before?.share ?? 0, share), words }));
    };
    timer = setTimeout(() => {
      void ask();
      draw();
      ticker = setInterval(draw, STATS_LOADING_TICK_MS);
    }, STATS_LOADING_AFTER_MS);
    return () => {
      controller.abort();
      clearTimeout(timer);
      clearInterval(ticker);
    };
  }, []);

  if (view === null) return null;
  return (
    <div className="stats-load" role="status">
      <div
        className="stats-load__bar"
        role="progressbar"
        aria-label={t("Loading card statistics…")}
        aria-valuemin={0}
        aria-valuemax={PERCENT}
        aria-valuenow={Math.round(view.share * PERCENT)}
      >
        <div className="stats-load__fill" style={{ width: `${String(view.share * PERCENT)}%` }} />
      </div>
      <p className="stats-load__words">{view.words}</p>
    </div>
  );
}
