// The online "More cards from" pick (R1372, R924000, R2031000): the one this device made last in the lobby,
// the rematch, a friend's challenge or a Duos room, so a pick made in one carries to the next. Practice keeps
// its own (`routes/practice.tsx`). The picker is `LeanNewest.tsx`'s; this half reads no engine value, so the
// shell's challenge dialog and toast hold it on a screen that never loads the engine (#778).
//
// It is kept in try/catch'd `localStorage` (`PLAY_LEAN_SET_KEY`): blocked storage only means it
// starts with no lean. A device that kept R1372's switch (`PLAY_LEAN_NEWEST_KEY`) reads it as
// "newest" when on, until the first pick is written.

import { LEAN_NEWEST, isLeanPick, type LeanPick } from "@jackioh/shared";
import { getStored, removeStored, setStored } from "../net/storage.ts";

export const PLAY_LEAN_SET_KEY = "jackioh.play.leanSet";

export const PLAY_LEAN_NEWEST_KEY = "jackioh.play.leanNewest";

/** No lean, as stored and as the picker's `<select>` value. */
export const NO_LEAN = "";

export function readPlayLean(): LeanPick | null {
  const stored = getStored(PLAY_LEAN_SET_KEY);
  if (stored !== null) return isLeanPick(stored) ? stored : null;
  return getStored(PLAY_LEAN_NEWEST_KEY) === "true" ? LEAN_NEWEST : null;
}

export function writePlayLean(pick: LeanPick | null): void {
  // Remembering the pick is a convenience; a refusal costs nothing.
  if (setStored(PLAY_LEAN_SET_KEY, pick ?? NO_LEAN)) removeStored(PLAY_LEAN_NEWEST_KEY);
}
