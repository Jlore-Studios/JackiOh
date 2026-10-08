// Which free practice game this device left in progress, so a reload or a closed tab picks it up
// again (SPEC §9.9, R668), and whether the player left it on purpose (R765).
//
// The game itself is the worker's to keep (`saveStore.ts`): its log names the AI's hidden cards, so
// it never reaches the page (rule 7). The page keeps only the setup it chose itself, the seed, seat,
// difficulty and deck, which tells it to ask the worker for a resume and what to label the game
// while it folds. The controller writes it when a free game starts and clears it when the game
// ends; the route clears it when the player leaves the game without saving it.
//
// R765: it also keeps where the player is. In the game (a reload, a closed tab, a Back), the next
// `/practice` picks the game up at once, as R668 says; left with Save and leave (`saved`), it shows
// the practice menu with a banner that offers the game back. A setup kept before R765 carries no
// flag, and reads as in the game. Storage is untrusted and may be missing: a read that throws or
// finds anything malformed is no game, and a write that throws leaves the player at the setup after
// a reload, as before R668.

import { DIFFICULTIES } from "@jackioh/engine/config";

import type { PracticeDeckChoice, PracticeStartConfig } from "./protocol.ts";

export const PRACTICE_RESUME_STORAGE_KEY = "jackioh.practice.game";

function isDeck(value: unknown): value is PracticeDeckChoice {
  if (typeof value !== "object" || value === null) return false;
  const deck = value as Record<string, unknown>;
  switch (deck.kind) {
    case "random":
      return true;
    case "preset":
      return typeof deck.id === "string";
    case "saved":
      return typeof deck.index === "number" && Array.isArray(deck.cards) && deck.cards.every((card) => typeof card === "string");
    default:
      return false;
  }
}

function isConfig(value: unknown): value is PracticeStartConfig {
  if (typeof value !== "object" || value === null) return false;
  const config = value as Record<string, unknown>;
  return (
    typeof config.seed === "string" &&
    (DIFFICULTIES as readonly unknown[]).includes(config.difficulty) &&
    (config.humanSeat === "p1" || config.humanSeat === "p2") &&
    isDeck(config.deck) &&
    config.lesson === undefined
  );
}

/** R765: the game kept on this device, and whether the player left it with Save and leave. */
export type PracticeResume = {
  config: PracticeStartConfig;
  /** Left with Save and leave: the practice menu offers it. False: the player was in the game (R668). */
  saved: boolean;
};

/** The free game to resume, as its setup and where the player left it; or none. */
export function readPracticeResumeState(): PracticeResume | null {
  try {
    const raw = window.localStorage.getItem(PRACTICE_RESUME_STORAGE_KEY);
    if (raw === null) return null;
    const parsed: unknown = JSON.parse(raw);
    if (typeof parsed !== "object" || parsed === null) return null;
    const { saved, ...config } = parsed as Record<string, unknown>;
    if (saved !== undefined && typeof saved !== "boolean") return null;
    return isConfig(config) ? { config, saved: saved === true } : null;
  } catch {
    return null;
  }
}

/** The free game to resume, as its setup; or none. */
export function readPracticeResume(): PracticeStartConfig | null {
  return readPracticeResumeState()?.config ?? null;
}

/**
 * Remembers `config`'s game as the one to resume: with the player in it, or (R765, `saved`) left
 * with Save and leave. A lesson is never resumed, so it clears instead.
 */
export function writePracticeResume(config: PracticeStartConfig, options: { saved?: boolean } = {}): void {
  if (config.lesson !== undefined) {
    clearPracticeResume();
    return;
  }
  const { seed, difficulty, humanSeat, deck } = config;
  const kept = options.saved === true ? { seed, difficulty, humanSeat, deck, saved: true } : { seed, difficulty, humanSeat, deck };
  try {
    window.localStorage.setItem(PRACTICE_RESUME_STORAGE_KEY, JSON.stringify(kept));
  } catch {
    // Blocked or full storage: a reload starts at the setup.
  }
}

/** No game to resume. */
export function clearPracticeResume(): void {
  try {
    window.localStorage.removeItem(PRACTICE_RESUME_STORAGE_KEY);
  } catch {
    // Blocked storage held nothing to clear.
  }
}
