// "Your table": the fun statistics the device has kept about its player's games (SPEC R639), on the
// floor below the homescreen's first screen. It reads the device's own totals (store.ts) and shows no
// card the player was not shown while playing (track.ts). It draws nothing before the first finished
// game. R1584000: it shows them one at a time (StatShowcase.tsx), each card playing the board's
// animation for its stat; the showcase is loaded when there is a stat to show, so the first screen
// stays light. The one request it makes is R1583000's, for the player base's play rates behind the
// Pocket Pick, and that request sends nothing of the tally.

import { lazy, Suspense, useEffect, useMemo, useState, type ReactElement } from "react";

import { landingTestid } from "../auth/testids.ts";
import { t, useLocale } from "../i18n/i18n.ts";
import { loadPocketPick, type PocketPick } from "./pocketPick.ts";
import { recordLine, showcaseStats } from "./showcase.ts";
import { resetPlayerStats, usePlayerStats } from "./store.ts";
import type { PlayerStats } from "./model.ts";

/** R1584000: the showcase and its stage (the board's animations and effects), loaded when a stat is first shown. */
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

export function PlayerStatsCard({ reduced }: { reduced: boolean }): ReactElement | null {
  useLocale();
  const stats = usePlayerStats();
  const pocket = usePocketPick(stats);
  const showcase = useMemo(() => showcaseStats(stats, pocket), [stats, pocket]);
  if (stats.games === 0) return null;

  // With no card to show, the record alone, as before R1584000.
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
          <StatShowcase stats={showcase} reduced={reduced} />
        </Suspense>
      )}
      <p className="landing-stats-note">
        {t("Kept on this device only, from what you were shown.")}{" "}
        <button type="button" className="link-button" data-testid={landingTestid.statsClear} onClick={resetPlayerStats}>
          {t("Clear")}
        </button>
      </p>
    </section>
  );
}
