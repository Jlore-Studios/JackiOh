// R5186000: the server's load of the card statistics as /stats shows it while a read waits on it: a bar,
// the step, the time so far and the time to expect, read from `GET /api/stats/loading`
// (crates/server/src/api/stats.rs). A skeleton says nothing of a load that runs for minutes. A read that
// times out while the server still reports a load asks again (`askAgain`), since the load goes on
// without it, rather than saying the statistics didn't load.

import { type ReactElement, useEffect, useState } from "react";

import { MS_PER_SECOND } from "../game/clockConstants.ts";
import { t } from "../i18n/i18n.ts";
import { ApiUnreachableError, getStatsLoading, type StatsLoading } from "../net/api.ts";
import { STATS_LOADING_AFTER_MS, STATS_LOADING_POLL_MS, STATS_LOADING_SHARE_MAX } from "./config.ts";

const SECONDS_PER_MINUTE = 60;

/** A whole share as a percentage. */
const PERCENT = 100;

/**
 * How full the bar is, from 0 to 1: the share of the time to expect gone by, when the last load's time is
 * known, or the share of the steps done, whichever is more, and never more than `STATS_LOADING_SHARE_MAX`
 * before the load ends. 0 when no load is under way.
 */
export function loadShare(loading: StatsLoading): number {
  if (!loading.loading || loading.steps <= 0) return 0;
  const byStep = Math.max(0, loading.step - 1) / loading.steps;
  const byTime = loading.expectedMs !== null && loading.expectedMs > 0 ? loading.elapsedMs / loading.expectedMs : 0;
  return Math.min(STATS_LOADING_SHARE_MAX, Math.max(byStep, byTime));
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

/** The line under the bar: the step, and the time so far against the time to expect when it is known. */
export function loadWords(loading: StatsLoading): string {
  const step = t("Step {step} of {steps}: {doing}", {
    step: String(loading.step),
    steps: String(loading.steps),
    doing: stepWords(loading.step),
  });
  const time =
    loading.expectedMs === null
      ? t("{elapsed} so far", { elapsed: clockOf(loading.elapsedMs) })
      : t("{elapsed} of about {expected}", { elapsed: clockOf(loading.elapsedMs), expected: clockOf(loading.expectedMs) });
  return `${step} · ${time}`;
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

/**
 * The progress bar, drawn while it is mounted (the page mounts it while its read waits): it asks the server
 * from `STATS_LOADING_AFTER_MS` on, every `STATS_LOADING_POLL_MS`, and draws nothing until an answer says a
 * load is under way, so a quick read never shows it.
 */
export function StatsLoadProgress(): ReactElement | null {
  const [loading, setLoading] = useState<StatsLoading | null>(null);

  useEffect(() => {
    const controller = new AbortController();
    let timer: ReturnType<typeof setTimeout> | undefined;
    const ask = async (): Promise<void> => {
      try {
        const answer = await getStatsLoading(controller.signal);
        if (!controller.signal.aborted) setLoading(answer);
      } catch {
        // A failed ask leaves the bar as it was; the next may answer.
      }
      if (!controller.signal.aborted) timer = setTimeout(() => void ask(), STATS_LOADING_POLL_MS);
    };
    timer = setTimeout(() => void ask(), STATS_LOADING_AFTER_MS);
    return () => {
      controller.abort();
      clearTimeout(timer);
    };
  }, []);

  if (loading === null || !loading.loading) return null;
  const percent = Math.round(loadShare(loading) * PERCENT);
  return (
    <div className="stats-load" role="status">
      <div
        className="stats-load__bar"
        role="progressbar"
        aria-label={t("Loading card statistics…")}
        aria-valuemin={0}
        aria-valuemax={PERCENT}
        aria-valuenow={percent}
      >
        <div className="stats-load__fill" style={{ width: `${String(percent)}%` }} />
      </div>
      <p className="stats-load__words">{loadWords(loading)}</p>
    </div>
  );
}
