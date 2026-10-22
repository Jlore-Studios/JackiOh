// What a finished game paid, as the server answers it (#1664, part of #1651; SPEC §14.7–§14.9,
// R1483000, R1502000): the game's Jlore and XP, every set track it moved from where it stood to where
// it stands with each level it reached and what that level paid, and every quest it completed. It
// mirrors `crates/server/src/economy/rewards.rs`' `GameEconomy`, which the server puts on the match
// socket's `view` frame, the co-op frame and the paid practice claim once the result is written.
//
// THE SERVER DECIDES (CLAUDE.md rule 7). The client computes no curve: the bar's size, where it was
// and where it is, the levels and their rewards are the answer's, and `stagesOf` only lays them out
// as the bars `FillBar` draws.

import { SHIPPED_SETS, type SetName } from "@jackioh/shared";

import type { TrackRewards } from "../net/api.ts";
import { isRecord } from "../wire/json.ts";

/** How long a level's number takes to pop when its bar fills (`xp.css`, through `--pop-ms`). */
export const LEVEL_POP_MS = 450;

/**
 * Where a track stands: its level and XP, the XP its level spans and the XP to the next level (both
 * null on the last level, R1302005).
 */
export type TrackPoint = { level: number; xp: number; levelXp: number | null; xpToNext: number | null };

/** A level a game reached and what it paid. */
export type LevelReached = { level: number; rewards: TrackRewards };

/** How far a game moved one set's track: where it stood before and after, and each level it paid, in order. */
export type TrackGain = { setId: SetName; before: TrackPoint; after: TrackPoint; levels: LevelReached[] };

/** A quest the game completed (§14.9), with what it paid. */
export type QuestPaid = { questId: string; title: string; jlore: number; xp: number };

/** What one game paid its account. */
export type GameEconomy = {
  jlore: number;
  xp: number;
  jloreCapped: boolean;
  tracks: TrackGain[];
  quests: QuestPaid[];
};

const isCount = (value: unknown): value is number => typeof value === "number" && Number.isFinite(value);
const isCountOrNull = (value: unknown): value is number | null => value === null || isCount(value);

function parsePoint(value: unknown): TrackPoint | null {
  if (!isRecord(value)) return null;
  const { level, xp, levelXp, xpToNext } = value;
  if (!isCount(level) || !isCount(xp) || !isCountOrNull(levelXp) || !isCountOrNull(xpToNext)) return null;
  return { level, xp, levelXp, xpToNext };
}

function parseRewards(value: unknown): TrackRewards | null {
  if (!isRecord(value)) return null;
  const { jlore, setPacks, cardTokens } = value;
  if (!isCount(jlore) || !isCount(setPacks) || !isCount(cardTokens)) return null;
  return { jlore, setPacks, cardTokens };
}

function parseLevel(value: unknown): LevelReached | null {
  if (!isRecord(value)) return null;
  const rewards = parseRewards(value.rewards);
  return isCount(value.level) && rewards !== null ? { level: value.level, rewards } : null;
}

function parseTrack(value: unknown): TrackGain | null {
  if (!isRecord(value) || !Array.isArray(value.levels)) return null;
  const setId = SHIPPED_SETS.find((set) => set === value.setId);
  const before = parsePoint(value.before);
  const after = parsePoint(value.after);
  if (setId === undefined || before === null || after === null) return null;
  const levels = value.levels.map(parseLevel);
  // A level the answer lays out wrongly makes the whole track's reveal untrustworthy: none of it.
  if (levels.some((level) => level === null)) return null;
  return { setId, before, after, levels: levels as LevelReached[] };
}

function parseQuest(value: unknown): QuestPaid | null {
  if (!isRecord(value)) return null;
  const { questId, title, jlore, xp } = value;
  return typeof questId === "string" && typeof title === "string" && isCount(jlore) && isCount(xp)
    ? { questId, title, jlore, xp }
    : null;
}

/**
 * The economy a frame carries, or null for none. A frame is never refused for it (the board must
 * not go down over a bar): an answer that is not whole shows nothing, a track or quest that is not
 * whole (or a set this build does not ship) is left out of it.
 */
export function parseGameEconomy(value: unknown): GameEconomy | null {
  if (!isRecord(value) || !Array.isArray(value.tracks) || !Array.isArray(value.quests)) return null;
  const { jlore, xp, jloreCapped } = value;
  if (!isCount(jlore) || !isCount(xp) || typeof jloreCapped !== "boolean") return null;
  return {
    jlore,
    xp,
    jloreCapped,
    tracks: value.tracks.map(parseTrack).filter((track) => track !== null),
    quests: value.quests.map(parseQuest).filter((quest) => quest !== null),
  };
}

/** A frame's `economy` as the field a parsed frame holds: none to hold when it carries none (`parseGameEconomy`). */
export function economyField(value: unknown): { economy?: GameEconomy } {
  const economy = parseGameEconomy(value);
  return economy === null ? {} : { economy };
}

/** The XP a track's level has taken so far: `levelXp − xpToNext` of `levelXp` (null on the last level). */
function pointsOf(point: TrackPoint): number | null {
  return point.levelXp === null || point.xpToNext === null ? null : point.levelXp - point.xpToNext;
}

/**
 * One bar `FillBar` plays: the level it is on, and where it starts (null for a bar that does not
 * move) and rests. `fills` is 1 for a bar that fills and passes to the next level, 0 for the last
 * one of a track.
 */
export type Stage = { level: number; size: number; from: number | null; value: number; fills: 0 | 1 };

/**
 * The bars a track's gain plays, one for each level it passes through: the level it stood on, from
 * where it stood to full; each level between, full; and the level it ends on, from empty to where it
 * stands. Levels span different XP (the answer gives the first's and the last's), so each bar is its
 * own: the bars between give no size, and take the last one's, which only sets how long they sweep.
 * A track that reaches its last level ends full, that level having no span, at the XP it holds; one
 * that stood there already never moves.
 */
export function stagesOf({ before, after }: TrackGain): Stage[] {
  const start = pointsOf(before);
  const end = pointsOf(after);
  const finalSize = after.levelXp ?? Math.max(1, after.xp);
  const finalValue = end ?? finalSize;
  const passed = Math.max(0, after.level - before.level);
  if (passed === 0 || before.levelXp === null || start === null) {
    return [{ level: after.level, size: finalSize, from: start, value: finalValue, fills: 0 }];
  }
  const stages: Stage[] = [{ level: before.level, size: before.levelXp, from: start, value: 0, fills: 1 }];
  for (let level = before.level + 1; level < after.level; level += 1) {
    stages.push({ level, size: finalSize, from: 0, value: 0, fills: 1 });
  }
  stages.push({ level: after.level, size: finalSize, from: 0, value: finalValue, fills: 0 });
  return stages;
}
