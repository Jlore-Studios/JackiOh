// "Your table": the fun statistics the device has kept about its player's games (SPEC R639), on the
// homescreen. It reads the device's own totals (store.ts) and nothing else, so it makes no request
// and shows no card the player was not shown while playing (track.ts). It draws nothing before the
// first finished game.

import type { ReactElement } from "react";
import catalogJson from "@jackioh/cards/catalog.json";
import type { CardDefs } from "@jackioh/shared";

import { landingTestid } from "../auth/testids.ts";
import { readerName } from "../cards/chinese.ts";
import { t, useLocale } from "../i18n/i18n.ts";
import { STATS_TOP_CARDS } from "./config.ts";
import { topCards, winPercent, type CardCounter, type PlayerStats } from "./model.ts";
import { resetPlayerStats, usePlayerStats } from "./store.ts";

const CATALOG = catalogJson as unknown as CardDefs;

/**
 * The four lists the card shows, in order: what each counter means to the player. The title is the
 * English (looked up where it is drawn); the verb says its sentence when it is called, never at import.
 */
const LISTS: readonly { counter: CardCounter; title: string; verb: (n: number) => string }[] = [
  { counter: "played", title: "Your favourites", verb: (n) => t("played {n}×", { n }) },
  { counter: "playedAgainst", title: "Your nemeses", verb: (n) => t("played against you {n}×", { n }) },
  { counter: "destroyed", title: "Fallen most often", verb: (n) => t("lost {n}×", { n }) },
  { counter: "defeated", title: "Toppled most often", verb: (n) => t("taken down {n}×", { n }) },
];

function nameOf(id: string): string {
  const def = CATALOG[id];
  return def === undefined ? id : readerName(def);
}

/** The record line: "12 games: 7 won, 4 lost, 1 drawn (58% won)". */
export function recordLine(stats: PlayerStats): string {
  const percent = winPercent(stats);
  const games = stats.games === 1 ? t("{n} game", { n: stats.games }) : t("{n} games", { n: stats.games });
  const counts = { games, won: stats.wins, lost: stats.losses, drawn: stats.draws, percent: percent ?? 0 };
  if (stats.draws > 0) {
    return percent === null
      ? t("{games}: {won} won, {lost} lost, {drawn} drawn", counts)
      : t("{games}: {won} won, {lost} lost, {drawn} drawn ({percent}% won)", counts);
  }
  return percent === null
    ? t("{games}: {won} won, {lost} lost", counts)
    : t("{games}: {won} won, {lost} lost ({percent}% won)", counts);
}

export function PlayerStatsCard(): ReactElement | null {
  useLocale();
  const stats = usePlayerStats();
  if (stats.games === 0) return null;

  const lists = LISTS.map((list) => ({ ...list, tallies: topCards(stats, list.counter, STATS_TOP_CARDS) })).filter(
    (list) => list.tallies.length > 0,
  );

  return (
    <section className="landing-stats" data-testid={landingTestid.stats} aria-labelledby="landing-stats-title">
      <h2 id="landing-stats-title" className="landing-how-title">
        {t("Your table")}
      </h2>
      <p className="landing-stats-record">{recordLine(stats)}</p>
      {lists.length === 0 ? null : (
        <div className="landing-stats-lists">
          {lists.map((list) => (
            <div key={list.counter} className="landing-stats-list" data-stat={list.counter}>
              <h3>{t(list.title)}</h3>
              <ol>
                {list.tallies.map((tally) => (
                  <li key={tally.id}>
                    <span className="landing-stats-card">{nameOf(tally.id)}</span> <span>{list.verb(tally.count)}</span>
                  </li>
                ))}
              </ol>
            </div>
          ))}
        </div>
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
