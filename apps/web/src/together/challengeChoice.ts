// What a challenge is made or accepted with (#1831, R1831000): the picks the challenge dialog
// (`ChallengeDialog.tsx`) and the toast's Accept (`ChallengeToast.tsx`) share, and the last of them
// remembered for the browser, so a repeat challenge or an accept starts on the pick the player made
// before. A per-viewer convenience like the lobby's own (`PLAY_CHOICE_KEY`, `routes/play.tsx`), kept
// beside the settings and not in them, which sync to the account (R634).
//
// CLAUDE.md rule 7: a pick is only what the player chose; whether a deck may be played is the
// server's, at the room.
//
// R915005: a deck naming a custom card, or a trio holding one, plays only in practice, so no pick here is
// one (`deckPracticeOnly`, `trioPracticeOnly`); the pickers list it off, "(practice only)", as the lobby's do.
//
// R2031000: All Random's "More cards from" pick is not kept here but in the device's online pick
// (`readPlayLean`, `game/LeanNewest.tsx`), the lobby's and the rematch's.

import { DECK_SIZE } from "@jackioh/engine/config";
import type { LeanPick } from "@jackioh/shared";

import { namesCustomCard } from "../creator/useCustomCards.ts";
import { t } from "../i18n/i18n.ts";
import type { ModeChoice, QueueMode, SavedDeck, SavedTrio } from "../net/api.ts";

export const CHALLENGE_CHOICE_KEY = "jackioh.challenge.choice";

export type StoredChallengeChoice = { mode: QueueMode; deckId: string | null; trioId: string | null };

/** R915005: a deck that names a custom card plays only in practice, never online. */
export function deckPracticeOnly(deck: SavedDeck): boolean {
  return namesCustomCard(deck.cards);
}

/** R915005: a trio holding such a deck is no online trio either. */
export function trioPracticeOnly(trio: SavedTrio, decks: readonly SavedDeck[]): boolean {
  return trio.deckIds.some((id) => {
    const deck = id === null ? undefined : decks.find((saved) => saved.id === id);
    return deck !== undefined && deckPracticeOnly(deck);
  });
}

/** "Name (practice only)": a deck or trio the picker shows but does not offer (R915005). */
export function practiceOnlyLabel(name: string): string {
  return t("{name} (practice only)", { name });
}

/** The first complete deck, else the first, as the lobby picks one with nothing remembered; never one that plays only in practice (R915005). */
export function firstDeck(decks: readonly SavedDeck[]): SavedDeck | null {
  const online = decks.filter((deck) => !deckPracticeOnly(deck));
  return online.find((deck) => deck.cards.length === DECK_SIZE) ?? online[0] ?? null;
}

/** The first trio with three decks, else the first; never one holding a deck that plays only in practice (R915005). */
export function firstTrio(trios: readonly SavedTrio[], decks: readonly SavedDeck[]): SavedTrio | null {
  const online = trios.filter((trio) => !trioPracticeOnly(trio, decks));
  return online.find((trio) => trio.deckIds.every((id) => id !== null)) ?? online[0] ?? null;
}

/** The deck with `id` while it is still saved and plays online, else the first (`firstDeck`). */
export function pickDeck(decks: readonly SavedDeck[], id: string | null): SavedDeck | null {
  return decks.find((deck) => deck.id === id && !deckPracticeOnly(deck)) ?? firstDeck(decks);
}

/** The trio with `id` while it is still saved and plays online, else the first (`firstTrio`). */
export function pickTrio(
  trios: readonly SavedTrio[],
  id: string | null,
  decks: readonly SavedDeck[],
): SavedTrio | null {
  return trios.find((trio) => trio.id === id && !trioPracticeOnly(trio, decks)) ?? firstTrio(trios, decks);
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
