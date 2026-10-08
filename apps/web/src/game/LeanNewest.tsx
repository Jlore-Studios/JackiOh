// "More cards from the newest set" (R1372, R1373): the switch beside every random deck a player is
// dealt, All Random's in the lobby and the rematch, and practice's Random deck.
//
// IT DEALS NOTHING (CLAUDE.md rule 7). On, it asks the dealer for a deck that leans on the newest set
// that ships: at least `AI_DECK.leanMinShare` of it (R1370), the rest dealt as without it. The server
// deals All Random's decks from the `leanNewest` the lobby and the rematch send; the practice worker
// deals practice's. The set is named here from `newestShippedSet()` (R1371) and never written down,
// so the words move on by themselves when the next set ships, and the share is the deck builder's own
// number, read from the WebAssembly module (rule 9).
//
// The online pick is kept on this device (`PLAY_LEAN_NEWEST_KEY`), for the lobby and the rematch
// alike, in try/catch'd `localStorage` like the lobby's other picks: a private window or blocked
// storage only means it starts off. Practice keeps its own beside its setup (`routes/practice.tsx`).

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
