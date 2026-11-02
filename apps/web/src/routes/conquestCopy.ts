// Message dictionary and persistence helpers for Conquest walkthrough and onboarding (SPEC §9.5, §9.10; R330–R334, R604).

import { SERIES_MAX_GAMES, SERIES_PICK_SECONDS, SERIES_WINS_NEEDED } from "@jackioh/server-config";

import { t } from "../i18n/i18n.ts";
import { getStored, removeStored, setStored } from "../net/storage.ts";

export const CONQUEST_EXPLAINER_STORAGE_KEY = "jackioh.conquest.explainer.v1";

export function isExplainerDismissed(): boolean {
  return getStored(CONQUEST_EXPLAINER_STORAGE_KEY) === "true";
}

export function setExplainerDismissed(dismissed = true): void {
  // Storage errors in restricted environments are ignored.
  if (dismissed) setStored(CONQUEST_EXPLAINER_STORAGE_KEY, "true");
  else removeStored(CONQUEST_EXPLAINER_STORAGE_KEY);
}

// English with `{name}` for the numbers, so each sentence is one dictionary entry (#711, R711000).
export const CONQUEST_MESSAGES = {
  explainerTitle: "How Conquest works",
  winsNeeded: "Win a game with each of your {wins} decks to take the series (first to {wins} wins).",
  lockOnWin: "A deck that wins a game is locked for the rest of the series. Decks that lost or drew may be played again.",
  sealedPicks: "Picks are simultaneous and sealed: neither player sees the opponent's choice until both have picked and the game starts.",
  pickClock: "You have {seconds} seconds to pick. If time runs out, your first unlocked deck is chosen automatically.",
  draws: "A drawn game counts for neither player and locks neither deck. The series ends after at most {games} games.",
  concedeVsForfeit: "Conceding or disconnecting during a game loses only that game. Between games, you may forfeit the entire series.",
  ratingMovesOnce: "Games in a series do not move your rating individually; a ranked series moves your rating once when it ends.",
  dismiss: "Got it",
  toggleOpen: "How Conquest works",
  lockInReminder: "Locking in is final and sealed. A win locks this deck for the rest of the series.",
  seriesEndRatingReminder: "Your rating moves once for the whole series, not for each of its games.",
  trioL4Hint: "Three decks with no card in common make a trio for Conquest: each card can be in only one of them.",
} as const;

export type ConquestMessageKey = keyof typeof CONQUEST_MESSAGES;

const CONQUEST_NUMBERS = { wins: SERIES_WINS_NEEDED, seconds: SERIES_PICK_SECONDS, games: SERIES_MAX_GAMES } as const;

/** A message in the reader's language, its numbers filled. */
export function conquestText(key: ConquestMessageKey): string {
  return t(CONQUEST_MESSAGES[key], CONQUEST_NUMBERS);
}
