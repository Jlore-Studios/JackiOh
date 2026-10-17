// Downloading a replay as JSON (#1141, R1141000): every step of a replay, as its seat saw it, in one
// file a player attaches to a bug report. Nothing but each step's view leaves (R768, CLAUDE.md rule 7)
// and no account (as a GIF names none, R513000): the steps, the row's own facts and the build's
// catalog version. Nothing is uploaded or kept: the file is made in memory and handed to the
// browser's downloads.

import { CATALOG_VERSION } from "@jackioh/cards";
import type { ReplayStep } from "@jackioh/engine";

import type { OnlineReplayListing } from "../net/api.ts";
import type { PracticeReplaySummary } from "../practice/protocol.ts";
import { REPLAY_GIF_REVOKE_MS } from "./config.ts";
import type { ReplaySource } from "./source.ts";
import { createStepCache, loadClip } from "./steps.ts";

/** What a reader of the file checks first: what it is, and which shape of it. */
export const REPLAY_JSON_FORMAT = "jackioh-replay";
export const REPLAY_JSON_VERSION = 1;

/** The game a file is of: a practice game by its number on the device, an online match by its id. */
export type ReplayJsonGame = Pick<PracticeReplaySummary, "endedAt" | "result" | "turns" | "steps" | "portraits"> &
  ({ kind: "practice"; game: number } | { kind: "online"; matchId: string; mode: OnlineReplayListing["mode"]; seat: OnlineReplayListing["seat"] });

export type ReplayJson = {
  format: typeof REPLAY_JSON_FORMAT;
  version: typeof REPLAY_JSON_VERSION;
  /** The catalog version of the build that made the file. */
  catalogVersion: string;
  /** When the file was made, in milliseconds since the epoch. */
  savedAt: number;
  game: ReplayJsonGame;
  steps: ReplayStep[];
};

/** A practice row's facts, named one by one so a field the listing gains is never carried by accident. */
export function practiceGame(row: PracticeReplaySummary): ReplayJsonGame {
  const { game, endedAt, result, turns, steps, portraits } = row;
  return { kind: "practice", game, endedAt, result, turns, steps, portraits };
}

/** An online row's facts, without the opponent's name or profile id. */
export function onlineGame(row: OnlineReplayListing): ReplayJsonGame {
  const { matchId, mode, seat, endedAt, result, turns, steps, portraits } = row;
  return { kind: "online", matchId, mode, seat, endedAt, result, turns, steps, portraits };
}

/** Every step of `source`, in order, a page at a time; rejects with the first page's `ReplayFailure`. */
export function allSteps(source: ReplaySource): Promise<ReplayStep[]> {
  return loadClip(createStepCache(source), 0, source.total - 1);
}

/** The file for `game`'s replay, made at `savedAt`. */
export async function replayJsonFile(game: ReplayJsonGame, source: ReplaySource, savedAt: number): Promise<File> {
  const json: ReplayJson = {
    format: REPLAY_JSON_FORMAT,
    version: REPLAY_JSON_VERSION,
    catalogVersion: CATALOG_VERSION,
    savedAt,
    game,
    steps: await allSteps(source),
  };
  const name = game.kind === "practice" ? `jackioh-replay-practice-${String(game.game)}.json` : `jackioh-replay-${game.matchId}.json`;
  return new File([JSON.stringify(json)], name, { type: "application/json" });
}

/** Hands `file` to the browser's downloads; its address is let go `REPLAY_GIF_REVOKE_MS` after the click. */
export function downloadFile(file: File): void {
  const url = URL.createObjectURL(file);
  const link = document.createElement("a");
  link.href = url;
  link.download = file.name;
  link.click();
  window.setTimeout(() => {
    URL.revokeObjectURL(url);
  }, REPLAY_GIF_REVOKE_MS);
}
