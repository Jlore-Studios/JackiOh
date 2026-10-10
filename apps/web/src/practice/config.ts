// Browser-only practice constants; AI handicaps remain engine-owned (CLAUDE.md rule 9, SPEC §9.9).

import type { Difficulty } from "@jackioh/engine/config";

import { FX_LETHAL_LEAD_MAX_MS, FX_RESULT_MS } from "../fx/constants.ts";

/** AI gaps leave room for action animation and voice holds; prompt answers use their own gap. */
export type PracticePacing = {
  firstActionMs: number;
  actionGapMs: number;
  promptAnswerMs: number;
  /** R200: delay the result dialog until game-over effects finish; absent opens at once. */
  resultDelayMs?: number;
};

/** Time gaps from board idle so effects finish before the next action; first actions wait longest. */
export const PRACTICE_PACING: PracticePacing = {
  firstActionMs: 800,
  actionGapMs: 550,
  promptAnswerMs: 450,
  // Wait for lethal replay and result effects.
  resultDelayMs: FX_LETHAL_LEAD_MAX_MS + FX_RESULT_MS,
};

/** Bound a forgotten voice hold so it cannot stall play. */
export const PRACTICE_VOICE_HOLD_MAX_MS = 4000;

/** Bound a forgotten showcase hold so it cannot stall play. */
export const PRACTICE_SHOWCASE_HOLD_MAX_MS = 4000;

export const PRACTICE_PACING_REDUCED: PracticePacing = { firstActionMs: 150, actionGapMs: 150, promptAnswerMs: 100 };

/** E2E-only fast pacing, guarded outside production. */
export const PRACTICE_PACING_FAST: PracticePacing = { firstActionMs: 0, actionGapMs: 30, promptAnswerMs: 0 };

/** Wall-clock safety cap for one worker decision (`AiOptions.shouldStop`). */
export const PRACTICE_AI_CLOCK_MS = 1500;

/** localStorage key, read and written inside try/catch. */
export const PRACTICE_SETUP_KEY = "jackioh.practice.setup";

export const PRACTICE_DEFAULT_DIFFICULTY: Difficulty = "easy";

export const PRACTICE_SEED_BYTES = 4;

export const PRACTICE_SEED_MAX_LENGTH = 64;

export const PRACTICE_RESULT_PARTICLES = 18;

/** R768: retained free-game replays. */
export const PRACTICE_REPLAYS_KEPT = 10;
