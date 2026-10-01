// The human's last practice board, kept on the device (R417, R508; Classic+ #29 Portal to the Past).
//
// A worker has no `localStorage`, so the page keeps it: the controller reads it into each start's
// config and writes what a finished free game's snapshot carries (`PracticeSnapshot.lastBoard`).
// Storage is untrusted and may be missing (private windows, blocked site data): a read that throws or
// finds anything malformed is no board, and a write that throws keeps the board before it.

import type { LastBoardCard } from "./protocol.ts";

export const LAST_BOARD_STORAGE_KEY = "jackioh.practice.lastBoard";

function isCard(value: unknown): value is LastBoardCard {
  if (typeof value !== "object" || value === null) return false;
  const card = value as Record<string, unknown>;
  return typeof card.defId === "string" && typeof card.radiant === "boolean";
}

/** The human's last practice board, or none. */
export function readLastBoard(): LastBoardCard[] {
  try {
    const raw = window.localStorage.getItem(LAST_BOARD_STORAGE_KEY);
    const parsed: unknown = raw === null ? [] : JSON.parse(raw);
    return Array.isArray(parsed) ? parsed.filter(isCard).map(({ defId, radiant }) => ({ defId, radiant })) : [];
  } catch {
    return [];
  }
}

/** Keeps `board` as the human's last practice board. */
export function writeLastBoard(board: readonly LastBoardCard[]): void {
  try {
    window.localStorage.setItem(LAST_BOARD_STORAGE_KEY, JSON.stringify(board));
  } catch {
    // Blocked storage: the next game starts with the board kept before, or none.
  }
}
