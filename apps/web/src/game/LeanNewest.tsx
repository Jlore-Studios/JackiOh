// "More cards from the newest set" (R1372, R1373): the switch beside every random deck a player is
// dealt (All Random's in the lobby and the rematch, practice's Random deck). It deals nothing
// (CLAUDE.md rule 7): on, the dealer leans on the newest set that ships, at least `AI_DECK.leanMinShare`
// of the deck (R1370). The set is named from `newestShippedSet()` (R1371), the share read from WASM (rule 9).
//
// The online pick is kept in try/catch'd `localStorage` (`PLAY_LEAN_NEWEST_KEY`): blocked storage only
// means it starts off. Practice keeps its own (`routes/practice.tsx`).

import { useId, type ReactElement } from "react";

import { AI_DECK } from "@jackioh/ai";
import { DECK_SIZE } from "@jackioh/engine/config";
import { newestShippedSet } from "@jackioh/shared";

import "./leanNewest.css";

/** Where this device remembers the online All Random pick (the lobby's and the rematch's). */
export const PLAY_LEAN_NEWEST_KEY = "jackioh.play.leanNewest";

/** The online pick this device made last; off in a private window, with blocked storage or junk. */
export function readPlayLeanNewest(): boolean {
  try {
    return window.localStorage.getItem(PLAY_LEAN_NEWEST_KEY) === "true";
  } catch {
    return false;
  }
}

export function writePlayLeanNewest(on: boolean): void {
  try {
    window.localStorage.setItem(PLAY_LEAN_NEWEST_KEY, on ? "true" : "false");
  } catch {
    // Remembering the pick is a convenience; a refusal costs nothing.
  }
}

/** R1372, R1373: the switch's words, naming the newest set: "More cards from the newest set (Classic+)". */
export function leanNewestLabel(): string {
  return `More cards from the newest set (${newestShippedSet()})`;
}

/** The cards a deck of `size` holds from the set at least (R1370: rounded up, as the deck builder rounds). */
export function leanNewestFloor(size: number = DECK_SIZE): number {
  return Math.ceil(size * AI_DECK.leanMinShare);
}

/** The line under the switch: what it asks of the deal. */
export function leanNewestNote(size: number = DECK_SIZE): string {
  return `At least ${String(leanNewestFloor(size))} of your ${String(size)} cards come from ${newestShippedSet()}.`;
}

export type LeanNewestToggleProps = {
  checked: boolean;
  onChange(on: boolean): void;
  /** On the `<input>` itself, as the lobby's mode radios carry theirs. */
  testid: string;
  disabled?: boolean;
};

/** A checkbox with the label and the note. */
export function LeanNewestToggle({ checked, onChange, testid, disabled = false }: LeanNewestToggleProps): ReactElement {
  const noteId = useId();
  return (
    <label className="lean-newest" data-checked={checked ? "true" : "false"}>
      <input
        type="checkbox"
        className="lean-newest__input"
        data-testid={testid}
        checked={checked}
        disabled={disabled}
        aria-describedby={noteId}
        onChange={(event) => {
          onChange(event.currentTarget.checked);
        }}
      />
      <span className="lean-newest__text">
        <span className="lean-newest__label">{leanNewestLabel()}</span>
        <span className="lean-newest__note" id={noteId}>
          {leanNewestNote()}
        </span>
      </span>
    </label>
  );
}
