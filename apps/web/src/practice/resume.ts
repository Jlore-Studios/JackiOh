// `saveStore.ts` keeps action logs worker-side because they name hidden AI cards (SPEC §9.9, R668;
// rule 7). R765 records Save and leave; malformed or inaccessible storage is no resume.

import { DIFFICULTIES } from "@jackioh/engine/config";

import type { PracticeDeckChoice, PracticeStartConfig } from "./protocol.ts";

export const PRACTICE_RESUME_STORAGE_KEY = "jackioh.practice.game";

function isDeck(value: unknown): value is PracticeDeckChoice {
  if (typeof value !== "object" || value === null) return false;
  const deck = value as Record<string, unknown>;
  switch (deck.kind) {
    case "random":
      // R1373: a setup kept before the lean carries none, and reads as off.
      return deck.leanNewest === undefined || typeof deck.leanNewest === "boolean";
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

/** R765: persisted practice setup and its Save-and-leave state. */
export type PracticeResume = {
  config: PracticeStartConfig;
  /** Save and leave; otherwise the player remained in the game (R668). */
  saved: boolean;
};

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

export function readPracticeResume(): PracticeStartConfig | null {
  return readPracticeResumeState()?.config ?? null;
}

/** R765: lessons clear because they cannot resume. */
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

export function clearPracticeResume(): void {
  try {
    window.localStorage.removeItem(PRACTICE_RESUME_STORAGE_KEY);
  } catch {
    // Blocked storage held nothing to clear.
  }
}
