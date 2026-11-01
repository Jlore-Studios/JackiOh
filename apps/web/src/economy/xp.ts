// What a finished game paid, as the server answers it (#1664, part of #1651; SPEC §14.7–§14.9,
// R1483000, R1502000). It mirrors `crates/server/src/economy/rewards.rs`' `GameEconomy`, which the
// match socket's `view` frame, the co-op frame and the paid practice claim carry. THE SERVER DECIDES
// (CLAUDE.md rule 7): the client computes no curve; `stagesOf` only lays the answer out as bars.

import { SHIPPED_SETS, type SetName } from "@jackioh/shared";

import type { TrackRewards } from "../net/api.ts";
import { isRecord } from "../wire/json.ts";

/** How long a level's number takes to pop when its bar fills (`xp.css`, through `--pop-ms`). */
export const LEVEL_POP_MS = 450;

/** Where a track stands; `levelXp` and `xpToNext` are null on the last level (R1302005). */
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

/** The XP a track's level has taken so far; null on the last level. */
function pointsOf(point: TrackPoint): number | null {
  return point.levelXp === null || point.xpToNext === null ? null : point.levelXp - point.xpToNext;
}

/**
 * One bar `FillBar` plays: its level, where it starts (null for a bar that does not move) and rests.
 * `fills` is 1 for a bar that passes to the next level, 0 for a track's last.
 */
export type Stage = { level: number; size: number; from: number | null; value: number; fills: 0 | 1 };

/**
 * The bars a track's gain plays, one per level it passes through. The answer gives only the first
 * and last spans, so a bar between takes the last's size, which only sets how long it sweeps. A track
 * reaching its last level ends full at the XP it holds (that level has no span).
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
