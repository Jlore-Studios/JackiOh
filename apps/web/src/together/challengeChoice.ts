// What a challenge is made or accepted with (#1831, R1831000): the picks the challenge dialog
// (`ChallengeDialog.tsx`) and the toast's Accept (`ChallengeToast.tsx`) share, and the last of them
// remembered for the browser, so a repeat challenge or an accept starts on the pick the player made
// before. A per-viewer convenience like the lobby's own (`PLAY_CHOICE_KEY`, `routes/play.tsx`), kept
// beside the settings and not in them, which sync to the account (R634).
//
// CLAUDE.md rule 7: a pick is only what the player chose; whether a deck may be played is the
// server's, at the room.

import { DECK_SIZE } from "@jackioh/engine/config";

import type { ModeChoice, QueueMode, SavedDeck, SavedTrio } from "../net/api.ts";

export const CHALLENGE_CHOICE_KEY = "jackioh.challenge.choice";

export type StoredChallengeChoice = { mode: QueueMode; deckId: string | null; trioId: string | null };

/** The first complete deck, else the first; as the lobby picks one with nothing remembered. */
export function firstDeck(decks: readonly SavedDeck[]): SavedDeck | null {
  return decks.find((deck) => deck.cards.length === DECK_SIZE) ?? decks[0] ?? null;
}

/** The first trio with three decks, else the first. */
export function firstTrio(trios: readonly SavedTrio[]): SavedTrio | null {
  return trios.find((trio) => trio.deckIds.every((id) => id !== null)) ?? trios[0] ?? null;
}

/** The deck with `id` while it is still saved, else the first (`firstDeck`). */
export function pickDeck(decks: readonly SavedDeck[], id: string | null): SavedDeck | null {
  return decks.find((deck) => deck.id === id) ?? firstDeck(decks);
}

/** The trio with `id` while it is still saved, else the first (`firstTrio`). */
export function pickTrio(trios: readonly SavedTrio[], id: string | null): SavedTrio | null {
  return trios.find((trio) => trio.id === id) ?? firstTrio(trios);
}

/** What a room is made with for `mode`, or null while its pick is missing (`choiceFor` in `routes/play.tsx`). */
export function choiceOf(mode: QueueMode, deckId: string | null, trioId: string | null): ModeChoice | null {
  switch (mode) {
    case "random":
      return { mode: "random" };
    case "bo1":
      return deckId === null ? null : { mode: "bo1", deckId };
    case "bo3":
      return trioId === null ? null : { mode: "bo3", trioId };
  }
}

/** The last pick made in this browser: Best of 1 and no deck or trio when there is none, or it is not readable. */
export function readChallengeChoice(): StoredChallengeChoice {
  const none: StoredChallengeChoice = { mode: "bo1", deckId: null, trioId: null };
  try {
    const raw = window.localStorage.getItem(CHALLENGE_CHOICE_KEY);
    if (raw === null) return none;
    const parsed: unknown = JSON.parse(raw);
    if (typeof parsed !== "object" || parsed === null) return none;
    const { mode, deckId, trioId } = parsed as { mode?: unknown; deckId?: unknown; trioId?: unknown };
    return {
      mode: mode === "bo1" || mode === "bo3" || mode === "random" ? mode : none.mode,
      deckId: typeof deckId === "string" ? deckId : null,
      trioId: typeof trioId === "string" ? trioId : null,
    };
  } catch {
    return none;
  }
}

/**
 * Remembers the deck or trio `choice` was made with, keeping the other as it was. The mode is kept
 * only when `setsMode`: a challenge sent sets the dialog's next mode, while an accepted one says
 * nothing of the mode the player would challenge in.
 */
export function rememberChallengeChoice(choice: ModeChoice, setsMode: boolean): void {
  const before = readChallengeChoice();
  const next: StoredChallengeChoice = {
    mode: setsMode ? choice.mode : before.mode,
    deckId: choice.mode === "bo1" ? choice.deckId : before.deckId,
    trioId: choice.mode === "bo3" ? choice.trioId : before.trioId,
  };
  try {
    window.localStorage.setItem(CHALLENGE_CHOICE_KEY, JSON.stringify(next));
  } catch {
    // Remembering the pick is a convenience; a refusal costs nothing.
  }
}
