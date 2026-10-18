// Downloading a replay as JSON (#1141, R1141000): every step of a replay, as its seat saw it, in one
// file a player attaches to a bug report. Nothing but each step's view leaves (R768, CLAUDE.md rule 7)
// and no account (as a GIF names none, R513000): the steps, the row's own facts and the build's
// catalog version. Nothing is uploaded or kept: the file is made in memory and handed to the
// browser's downloads. And back in (#1160): a file the page wrote opens again, parsed in the page
// against that same shape, nothing sent and nothing stored.

import { CATALOG_VERSION } from "@jackioh/cards";
import type { ReplayStep } from "@jackioh/engine";

import type { OnlineReplayListing } from "../net/api.ts";
import type { PracticeReplaySummary } from "../practice/protocol.ts";
import { REPLAY_GIF_REVOKE_MS, REPLAY_JSON_MAX_BYTES, REPLAY_JSON_MAX_STEPS } from "./config.ts";
import { ReplayFailure } from "./source.ts";
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

const TOO_BIG = "This replay file is too big to open.";
const NOT_A_REPLAY = "This file is not a JackiOh replay file.";
const UNSUPPORTED = "This replay file uses an unsupported format.";
const OTHER_PATCH = "This replay file is from another patch and cannot be opened here.";

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

/**
 * The steps of a file `replayJsonFile` wrote (#1160), or a `ReplayFailure` saying why it is refused:
 * too big, not JSON of the documented shape, another format or version, or another catalog version
 * (a file from a newer build is refused as a stale one is, since its views may differ). The file is
 * untrusted, so it is held to the caps before its text is read and again before its steps are kept.
 * A step is checked for its index, a turn that never goes back and a view that is an object; the
 * catalog gate is what makes that enough, and the page never enforces rules on a view (rule 7).
 */
export async function parseReplayJsonFile(file: File): Promise<ReplayStep[]> {
  if (file.size > REPLAY_JSON_MAX_BYTES) throw new ReplayFailure("error", TOO_BIG);
  const text = await file.text();
  if (text.length > REPLAY_JSON_MAX_BYTES) throw new ReplayFailure("error", TOO_BIG);
  let json: unknown;
  try {
    json = JSON.parse(text);
  } catch {
    throw new ReplayFailure("error", NOT_A_REPLAY);
  }
  if (!isRecord(json)) throw new ReplayFailure("error", NOT_A_REPLAY);
  if (json.format !== REPLAY_JSON_FORMAT || json.version !== REPLAY_JSON_VERSION) throw new ReplayFailure("error", UNSUPPORTED);
  if (json.catalogVersion !== CATALOG_VERSION) throw new ReplayFailure("error", OTHER_PATCH);
  const { steps } = json;
  if (!Array.isArray(steps) || steps.length === 0) throw new ReplayFailure("error", NOT_A_REPLAY);
  if (steps.length > REPLAY_JSON_MAX_STEPS) throw new ReplayFailure("error", TOO_BIG);
  const checked: ReplayStep[] = [];
  let turn = 0;
  for (const [index, step] of (steps as unknown[]).entries()) {
    if (!isRecord(step) || step.step !== index || !isRecord(step.view)) throw new ReplayFailure("error", NOT_A_REPLAY);
    if (typeof step.turn !== "number" || !Number.isInteger(step.turn) || step.turn < turn) throw new ReplayFailure("error", NOT_A_REPLAY);
    turn = step.turn;
    checked.push({ step: index, turn, view: step.view as ReplayStep["view"] });
  }
  return checked;
}
