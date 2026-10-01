// SPEC §9.11, R378: an internal AI development run. AI-against-AI games played on a build before its
// patch ships, each filed as a game record (R376) of its own source, so the card win rates of a
// patch's pre-release run can be set beside the live games played on it once it ships.
//
// A development game is an All Random game (R258) with two AI pilots: both decks are dealt by the
// game's weighted random deck-builder with nothing banned, from the game seed and the seat
// (`${seed}:p1-deck`, `${seed}:p2-deck`), exactly as the server deals a live one; both seats play on
// this spec's own resources, no handicap (R180), at the browser's budget. So a run files under All
// Random, and compares with that mode's live games like for like. Its source is "dev", which no live
// figure counts unless it is asked for (R378). A card the shadow ban keeps out of the AI's own decks
// (R186) is dealt here like any other — All Random bans nothing — and is one the AI is known to play
// badly, so its development figures say more about the AI than about the card.
//
// Pure and seeded like the rest of src/ (CLAUDE.md rule 4): `scripts/stats.ts` loops over the games
// and writes the file.

import { DECK_SIZE, createRng, summarizeGame } from "@jackioh/engine";
import { DEV_RECORD_ID_PREFIX, type GameRecord } from "@jackioh/shared";
import { AI_BUDGET } from "./config";
import { buildAiDeck } from "./deck";
import { playMatch, type MatchConfig } from "./match";
import type { SearchBudget } from "./types";

export const AI_DEV_RUN = {
  /** Games a run plays when it is not told how many. */
  games: 200,
  /** The default seed series: game n's seed is `${series}:${n}`. No gate or tuning run plays it. */
  series: "dev",
} as const;

export type DevRunOptions = {
  /** The seed series; a run told another one plays other games. */
  series: string;
  /** R375's version the run tests: the record's patch. */
  patch: string;
  /** The AI's budget; the browser's (`AI_BUDGET`) unless a test asks for less. */
  budget?: SearchBudget;
};

/** Game n of a series: its seed, its two dealt decks, and two AI seats at this spec's resources. */
export function devGameConfig(n: number, options: Pick<DevRunOptions, "series" | "budget">): MatchConfig {
  const seed = `${options.series}:${String(n)}`;
  const budget = options.budget ?? AI_BUDGET;
  // R258: All Random's deal, `buildAiDeck` with nothing banned, as `apps/server`'s engine port deals it.
  const deal = (seat: string): string[] => buildAiDeck(createRng(`${seed}:${seat}-deck`), DECK_SIZE, { banned: [] });
  return {
    seed,
    decks: [deal("p1"), deal("p2")],
    controllers: { p1: { kind: "ai", budget }, p2: { kind: "ai", budget } },
  };
}

/**
 * R378: a development record's id, `dev:<patch>:<seed>`. The patch is part of it, so the same seeds
 * played again for another patch are other records, which `stats:import` adds rather than skipping
 * as a run it has already loaded.
 */
export function devRecordId(patch: string, seed: string): string {
  return `${DEV_RECORD_ID_PREFIX}${patch}:${seed}`;
}

/**
 * R376, R378: plays game n and files it. Null when the game did not finish — it hit the match's
 * action limit or an AI threw — since only a finished game is a record.
 */
export function devGameRecord(n: number, options: DevRunOptions): GameRecord | null {
  const config = devGameConfig(n, options);
  const played = playMatch(config);
  if (played.result === null) return null;
  const game = summarizeGame({ seed: config.seed, decks: config.decks, log: played.log });
  if (game === null) return null;
  return {
    id: devRecordId(options.patch, config.seed),
    source: "dev",
    mode: "random",
    patch: options.patch,
    pilots: { p1: "ai", p2: "ai" },
    game,
  };
}
