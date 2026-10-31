// What a challenge is made or accepted with (#1831, R1831000), shared by `ChallengeDialog.tsx` and the
// toast's Accept, the last pick remembered for the browser. A per-viewer convenience like the lobby's
// (`PLAY_CHOICE_KEY`), kept beside the settings and not in them, which sync to the account (R634).
// CLAUDE.md rule 7: a pick is only what the player chose; whether a deck may be played is the
// server's, at the room. R2031000: All Random's "More cards from" pick is kept in the device's online
// pick instead (`readPlayLean`, `game/LeanNewest.tsx`).

import { DECK_SIZE } from "@jackioh/engine/config";
import type { LeanPick } from "@jackioh/shared";

import type { ModeChoice, QueueMode, SavedDeck, SavedTrio } from "../net/api.ts";

export const CHALLENGE_CHOICE_KEY = "jackioh.challenge.choice";

export type StoredChallengeChoice = { mode: QueueMode; deckId: string | null; trioId: string | null };

/** The first complete deck, else the first; as the lobby picks one with nothing remembered. */
export function firstDeck(decks: readonly SavedDeck[]): SavedDeck | null {
  return decks.find((deck) => deck.cards.length === DECK_SIZE) ?? decks[0] ?? null;
}

export function firstTrio(trios: readonly SavedTrio[]): SavedTrio | null {
  return trios.find((trio) => trio.deckIds.every((id) => id !== null)) ?? trios[0] ?? null;
}

export function pickDeck(decks: readonly SavedDeck[], id: string | null): SavedDeck | null {
  return decks.find((deck) => deck.id === id) ?? firstDeck(decks);
}

export function pickTrio(trios: readonly SavedTrio[], id: string | null): SavedTrio | null {
  return trios.find((trio) => trio.id === id) ?? firstTrio(trios);
}

/** What a room is made with, or null while its pick is missing (`choiceFor` in `routes/play.tsx`; R924000). */
export function choiceOf(
  mode: QueueMode,
  deckId: string | null,
  trioId: string | null,
  lean: LeanPick | null = null,
): ModeChoice | null {
  switch (mode) {
    case "random":
      return lean === null ? { mode: "random" } : { mode: "random", leanSet: lean };
    case "bo1":
      return deckId === null ? null : { mode: "bo1", deckId };
    case "bo3":
      return trioId === null ? null : { mode: "bo3", trioId };
  }
}

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
 * The mode is kept only when `setsMode`: a challenge sent sets the dialog's next mode, while an
 * accepted one says nothing of the mode the player would challenge in.
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
