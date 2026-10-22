// R1583000: the Pocket Pick, the card the player plays far more often than the player base does,
// read off the device's tally (R639) against `GET /api/stats/play-rates`. Presentation only; the
// request sends nothing of the tally (CLAUDE.md rule 7).

import { getPlayRates, type PlayRatesResponse } from "../net/api.ts";
import { POCKET_PICK_MIN_GAMES, POCKET_PICK_MIN_PLAYS } from "./config.ts";
import type { PlayerStats } from "./model.ts";

/** The card, the share of the player's games it was played in, and the player base's. */
export type PocketPick = { readonly id: string; readonly playerRate: number; readonly baseRate: number };

/** A higher player-to-base ratio (cross-multiplied, so a base rate of 0 ranks first), then a higher player rate, then the lower id. */
function beats(a: PocketPick, b: PocketPick): boolean {
  const ours = a.playerRate * b.baseRate;
  const theirs = b.playerRate * a.baseRate;
  if (ours !== theirs) return ours > theirs;
  if (a.playerRate !== b.playerRate) return a.playerRate > b.playerRate;
  return a.id < b.id;
}

/** R1583000: the Pocket Pick, or null below `POCKET_PICK_MIN_GAMES` games or when no listed card qualifies. */
export function pocketPick(stats: PlayerStats, rates: PlayRatesResponse): PocketPick | null {
  if (stats.games < POCKET_PICK_MIN_GAMES) return null;
  let best: PocketPick | null = null;
  for (const { id, rate } of rates.cards) {
    const played = stats.cards[id]?.played ?? 0;
    if (played < POCKET_PICK_MIN_PLAYS) continue;
    // R639 counts plays; one copy a deck makes that a game each but for a card played twice.
    const playerRate = Math.min(played, stats.games) / stats.games;
    if (!(playerRate > rate)) continue;
    const candidate: PocketPick = { id, playerRate, baseRate: rate };
    if (best === null || beats(candidate, best)) best = candidate;
  }
  return best;
}

/** R1583000: the Pocket Pick against the server's rates; null when the request fails or the tally is too short to ask. */
export async function loadPocketPick(stats: PlayerStats, signal?: AbortSignal): Promise<PocketPick | null> {
  if (stats.games < POCKET_PICK_MIN_GAMES) return null;
  try {
    return pocketPick(stats, await getPlayRates(signal));
  } catch {
    return null;
  }
}
