// "Your table": the fun statistics the device has kept about its player's games (SPEC R639), on the
// homescreen. It reads the device's own totals (store.ts) and nothing else, so it makes no request
// and shows no card the player was not shown while playing (track.ts). It draws nothing before the
// first finished game.

import type { ReactElement } from "react";
import catalogJson from "@jackioh/cards/catalog.json";
import type { CardDefs } from "@jackioh/shared";

import { landingTestid } from "../auth/testids.ts";
import { STATS_TOP_CARDS } from "./config.ts";
import { topCards, winPercent, type CardCounter, type PlayerStats } from "./model.ts";
import { resetPlayerStats, usePlayerStats } from "./store.ts";

const CATALOG = catalogJson as unknown as CardDefs;

/** The four lists the card shows, in order: what each counter means to the player. */
const LISTS: readonly { counter: CardCounter; title: string; verb: (n: number) => string }[] = [
  { counter: "played", title: "Your favourites", verb: (n) => `played ${String(n)}×` },
  { counter: "playedAgainst", title: "Your nemeses", verb: (n) => `played against you ${String(n)}×` },
  { counter: "destroyed", title: "Fallen most often", verb: (n) => `lost ${String(n)}×` },
  { counter: "defeated", title: "Toppled most often", verb: (n) => `taken down ${String(n)}×` },
];

function nameOf(id: string): string {
  return CATALOG[id]?.name ?? id;
}

/** The record line: "12 games: 7 won, 4 lost, 1 drawn (58% won)". */
export function recordLine(stats: PlayerStats): string {
  const percent = winPercent(stats);
  const games = `${String(stats.games)} ${stats.games === 1 ? "game" : "games"}`;
  const parts = [`${String(stats.wins)} won`, `${String(stats.losses)} lost`];
  if (stats.draws > 0) parts.push(`${String(stats.draws)} drawn`);
  return `${games}: ${parts.join(", ")}${percent === null ? "" : ` (${String(percent)}% won)`}`;
}

export function PlayerStatsCard(): ReactElement | null {
  const stats = usePlayerStats();
  if (stats.games === 0) return null;

  const lists = LISTS.map((list) => ({ ...list, tallies: topCards(stats, list.counter, STATS_TOP_CARDS) })).filter(
    (list) => list.tallies.length > 0,
  );

  return (
    <section className="landing-stats" data-testid={landingTestid.stats} aria-labelledby="landing-stats-title">
      <h2 id="landing-stats-title" className="landing-how-title">
        Your table
      </h2>
      <p className="landing-stats-record">{recordLine(stats)}</p>
      {lists.length === 0 ? null : (
        <div className="landing-stats-lists">
          {lists.map((list) => (
            <div key={list.counter} className="landing-stats-list" data-stat={list.counter}>
              <h3>{list.title}</h3>
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
        Kept on this device only, from what you were shown.{" "}
        <button type="button" className="link-button" data-testid={landingTestid.statsClear} onClick={resetPlayerStats}>
          Clear
        </button>
      </p>
    </section>
  );
}
