// "More cards from" (R1372, R1373, R924000): the picker beside every random deck a player is dealt
// (All Random's in the lobby and the rematch, practice's Random deck). It deals nothing (CLAUDE.md
// rule 7): with a pick, the dealer leans on that set, at least `AI_DECK.leanMinShare` of the deck
// (R1370). "Newest" names the set from `newestShippedSet()` (R1371) and the dealer resolves it when
// it deals; the other choices are the sets that ship (R1420). The share is read from WASM (rule 9).
//
// The online pick is kept in try/catch'd `localStorage` (`PLAY_LEAN_SET_KEY`): blocked storage only
// means it starts with no lean. A device that kept R1372's switch (`PLAY_LEAN_NEWEST_KEY`) reads it
// as "newest" when on, until the first pick is written. Practice keeps its own (`routes/practice.tsx`).

import { useId, type ReactElement } from "react";

import { AI_DECK } from "@jackioh/ai";
import { DECK_SIZE } from "@jackioh/engine/config";
import { LEAN_NEWEST, isLeanPick, leanPicks, leanSetOf, newestShippedSet, type LeanPick } from "@jackioh/shared";

import { t } from "../i18n/i18n.ts";

import "./leanNewest.css";

/** Where this device remembers the online All Random pick (the lobby's and the rematch's). */
export const PLAY_LEAN_SET_KEY = "jackioh.play.leanSet";

/** R1372's switch, before the picker: `"true"` when it was on. Read only when no pick was written. */
export const PLAY_LEAN_NEWEST_KEY = "jackioh.play.leanNewest";

/** The `<select>` value of no lean. */
const NO_LEAN = "";

/**
 * The online pick this device made last: a pick, or `null` for none. None in a private window, with
 * blocked storage or junk; R1372's switch, left on, is "newest".
 */
export function readPlayLean(): LeanPick | null {
  try {
    const stored = window.localStorage.getItem(PLAY_LEAN_SET_KEY);
    if (stored !== null) return isLeanPick(stored) ? stored : null;
    return window.localStorage.getItem(PLAY_LEAN_NEWEST_KEY) === "true" ? LEAN_NEWEST : null;
  } catch {
    return null;
  }
}

export function writePlayLean(pick: LeanPick | null): void {
  try {
    window.localStorage.setItem(PLAY_LEAN_SET_KEY, pick ?? NO_LEAN);
    window.localStorage.removeItem(PLAY_LEAN_NEWEST_KEY);
  } catch {
    // Remembering the pick is a convenience; a refusal costs nothing.
  }
}

/** R924000: the picker's words before its choices. */
export function leanLabel(): string {
  return t("More cards from");
}

/** R924000: a choice's words: "newest" names the set it is now ("The newest set (Meditative)"). */
export function leanChoiceLabel(pick: LeanPick | null): string {
  if (pick === null) return t("No set in particular");
  if (pick === LEAN_NEWEST) return t("The newest set ({set})", { set: t(newestShippedSet()) });
  return t(pick);
}

/** The cards a deck of `size` holds from the set at least (R1370: rounded up, as the deck builder rounds). */
export function leanFloor(size: number = DECK_SIZE): number {
  return Math.ceil(size * AI_DECK.leanMinShare);
}

/** The line under the picker when it holds a pick: what it asks of the deal. */
export function leanNote(pick: LeanPick, size: number = DECK_SIZE): string {
  return t("At least {floor} of your {size} cards come from {set}.", {
    floor: leanFloor(size),
    size,
    set: t(leanSetOf(pick)),
  });
}

export type LeanSetPickerProps = {
  /** The pick, or `null` for none. */
  value: LeanPick | null;
  onChange(pick: LeanPick | null): void;
  /** On the `<select>` itself, as the lobby's mode radios carry theirs. */
  testid: string;
  disabled?: boolean;
};

/** A labelled `<select>`: none, "newest", then every set that ships, with the note when one is picked. */
export function LeanSetPicker({ value, onChange, testid, disabled = false }: LeanSetPickerProps): ReactElement {
  const noteId = useId();
  return (
    <label className="lean-newest" data-pick={value ?? "none"}>
      <span className="lean-newest__text">
        <span className="lean-newest__label">{leanLabel()}</span>
        <select
          className="lean-newest__input"
          data-testid={testid}
          value={value ?? NO_LEAN}
          disabled={disabled}
          aria-describedby={value === null ? undefined : noteId}
          onChange={(event) => {
            const picked = event.currentTarget.value;
            onChange(isLeanPick(picked) ? picked : null);
          }}
        >
          <option value={NO_LEAN}>{leanChoiceLabel(null)}</option>
          {leanPicks().map((pick) => (
            <option key={pick} value={pick}>
              {leanChoiceLabel(pick)}
            </option>
          ))}
        </select>
        {value === null ? null : (
          <span className="lean-newest__note" id={noteId}>
            {leanNote(value)}
          </span>
        )}
      </span>
    </label>
  );
}
