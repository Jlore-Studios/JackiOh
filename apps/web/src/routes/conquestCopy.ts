// Message dictionary and persistence helpers for Conquest walkthrough and onboarding (SPEC §9.5, §9.10; R330–R334, R604).

import { SERIES_MAX_GAMES, SERIES_PICK_SECONDS, SERIES_WINS_NEEDED } from "@jackioh/server-config";

export const CONQUEST_EXPLAINER_STORAGE_KEY = "jackioh.conquest.explainer.v1";

export function isExplainerDismissed(): boolean {
  try {
    if (typeof window === "undefined" || !window.localStorage) return false;
    return window.localStorage.getItem(CONQUEST_EXPLAINER_STORAGE_KEY) === "true";
  } catch {
    return false;
  }
}

export function setExplainerDismissed(dismissed = true): void {
  try {
    if (typeof window === "undefined" || !window.localStorage) return;
    if (dismissed) {
      window.localStorage.setItem(CONQUEST_EXPLAINER_STORAGE_KEY, "true");
    } else {
      window.localStorage.removeItem(CONQUEST_EXPLAINER_STORAGE_KEY);
    }
  } catch {
    // Ignore storage errors in restricted environments
  }
}

export const CONQUEST_MESSAGES = {
  explainerTitle: "How Conquest works",
  winsNeeded: `Win a game with each of your ${String(SERIES_WINS_NEEDED)} decks to take the series (first to ${String(SERIES_WINS_NEEDED)} wins).`,
  lockOnWin: "A deck that wins a game is locked for the rest of the series. Decks that lost or drew may be played again.",
  sealedPicks: "Picks are simultaneous and sealed: neither player sees the opponent’s choice until both have picked and the game starts.",
  pickClock: `You have ${String(SERIES_PICK_SECONDS)} seconds to pick. If time runs out, your first unlocked deck is chosen automatically.`,
  draws: `A drawn game counts for neither player and locks neither deck. The series ends after at most ${String(SERIES_MAX_GAMES)} games.`,
  concedeVsForfeit: "Conceding or disconnecting during a game loses only that game. Between games, you may forfeit the entire series.",
  ratingMovesOnce: "Games in a series do not move your rating individually; a ranked series moves your rating once when it ends.",
  dismiss: "Got it",
  toggleOpen: "How Conquest works",
  lockInReminder: "Locking in is final and sealed. A win locks this deck for the rest of the series.",
  seriesEndRatingReminder: "Your rating moves once for the whole series, not for each of its games.",
  trioL4Hint: "Three decks with no cards in common make a trio for Conquest (Rule L4). Each card can appear in only one deck.",
} as const;

export type ConquestMessageKey = keyof typeof CONQUEST_MESSAGES;

export function conquestText(key: ConquestMessageKey): string {
  return CONQUEST_MESSAGES[key];
}
