// "Your table" (SPEC R639), below the homescreen's first screen: the device's own totals (store.ts), no card
// the player was not shown (track.ts). R1584000: one stat at a time (StatShowcase.tsx), loaded only when there
// is one, so the first screen stays light. R1743000: 35 stats, none about a token. Its one request is
// R1583000's, for the Pocket Pick's play rates; it sends nothing of the tally.

import { lazy, Suspense, useEffect, useMemo, useState, type ReactElement } from "react";

import { landingTestid } from "../auth/testids.ts";
import { t, useLocale } from "../i18n/i18n.ts";
import type { RandomSource } from "../routes/landingFan.ts";
import { loadPocketPick, type PocketPick } from "./pocketPick.ts";
import { pocketStat, recordLine, showcaseStats } from "./showcase.ts";
import { usePlayerStats } from "./store.ts";
import type { PlayerStats } from "./model.ts";

/** The showcase and its stage (the board's animations and effects), loaded when a stat is first shown. */
const StatShowcase = lazy(() => import("./StatShowcase.tsx").then((module) => ({ default: module.StatShowcase })));

/** R1583000: the Pocket Pick for this tally, null until the play rates answer and when they name none. */
function usePocketPick(stats: PlayerStats): PocketPick | null {
  const [pick, setPick] = useState<PocketPick | null>(null);
  useEffect(() => {
    const controller = new AbortController();
    let live = true;
    void loadPocketPick(stats, controller.signal).then((found) => {
      if (live) setPick(found);
    });
    return () => {
      live = false;
      controller.abort();
    };
  }, [stats]);
  return pick;
}

export type PlayerStatsCardProps = {
  reduced: boolean;
  /** R1743000: where the showcase's random picks and its order are drawn from (landing.tsx's `random`). */
  random: RandomSource;
};

export function PlayerStatsCard({ reduced, random }: PlayerStatsCardProps): ReactElement | null {
  useLocale();
  const stats = usePlayerStats();
  const pocket = usePocketPick(stats);
  // The pick's answer redraws nothing: the random stats are drawn once per tally, the pick added to them.
  const drawn = useMemo(() => showcaseStats(stats, null, random), [stats, random]);
  const showcase = useMemo(() => [...drawn, ...pocketStat(stats, pocket)], [drawn, stats, pocket]);
  if (stats.games === 0) return null;

  // With no card to show, the record alone (R1584000).
  const record = <p className="landing-stats-record">{recordLine(stats)}</p>;

  return (
    <section className="landing-stats" data-testid={landingTestid.stats} aria-labelledby="landing-stats-title">
      <h2 id="landing-stats-title" className="landing-how-title">
        {t("Your table")}
      </h2>
      {showcase.length === 0 ? (
        record
      ) : (
        <Suspense fallback={record}>
          <StatShowcase stats={showcase} reduced={reduced} random={random} />
        </Suspense>
      )}
    </section>
  );
}
