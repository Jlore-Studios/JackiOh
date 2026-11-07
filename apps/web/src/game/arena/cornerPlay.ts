// What a tap on a corner prop does, as a pure machine (#3348, R3348001): escalation after a streak of
// taps, the rare outcome, a hidden sequence, and the sound each outcome makes. It holds no clock and no
// random source of its own: the caller passes `now` and a `random` draw, so a test replays it exactly.

import type { SfxId, SfxParams } from "../../audio/types.ts";
import {
  CORNER_BIG_BUILD,
  CORNER_ESCALATE_CUES,
  CORNER_ESCALATE_STEP_MS,
  CORNER_ESCALATE_TAPS,
  CORNER_RARE_CHANCE,
  CORNER_RARE_STEP_MS,
  CORNER_SEQUENCE_GAP_MS,
  CORNER_SEQUENCE_STEP_MS,
  CORNER_SOUND_VARIANTS,
  CORNER_STREAK_GAP_MS,
  CORNER_TAP_BUILD,
} from "./config.ts";
import { sequenceOf, type CornerPropDef } from "./props.ts";

export type CornerOutcome = "tap" | "escalate" | "rare" | "sequence";

type Streak = { count: number; goal: number; at: number };
type Progress = { step: number; at: number };

export type CornerState = {
  /** Each prop's streak of taps, by prop id. */
  streaks: Readonly<Record<string, Streak>>;
  /** Each playmat's progress through its hidden sequence. */
  progress: Readonly<Record<string, Progress>>;
};

export function emptyCornerState(): CornerState {
  return { streaks: {}, progress: {} };
}

/** The taps one streak needs: drawn from CORNER_ESCALATE_TAPS, both ends included. */
function drawGoal(random: () => number): number {
  const { min, max } = CORNER_ESCALATE_TAPS;
  return min + Math.floor(random() * (max - min + 1));
}

/**
 * One tap on `propId` of `mat` at `now`. A tap within CORNER_STREAK_GAP_MS of the prop's last adds to its
 * streak, and the streak's goal is drawn when it starts, then the rare roll (that is the draw order).
 * A rare outcome wins over a completed sequence, which wins over the streak's escalation.
 */
export function tapCorner(
  state: CornerState,
  mat: string,
  propId: string,
  now: number,
  random: () => number,
): { state: CornerState; outcome: CornerOutcome } {
  const before = state.streaks[propId];
  const continues = before !== undefined && now - before.at <= CORNER_STREAK_GAP_MS;
  const count = continues ? before.count + 1 : 1;
  const goal = continues ? before.goal : drawGoal(random);
  const rare = random() < CORNER_RARE_CHANCE;

  const sequence = sequenceOf(mat);
  let progress = state.progress;
  let completed = false;
  if (sequence !== undefined) {
    const last = state.progress[mat];
    const held = last !== undefined && now - last.at <= CORNER_SEQUENCE_GAP_MS ? last.step : 0;
    const step = sequence[held] === propId ? held + 1 : sequence[0] === propId ? 1 : 0;
    completed = step === sequence.length;
    progress = { ...state.progress, [mat]: { step: completed ? 0 : step, at: now } };
  }

  const escalated = count >= goal;
  const { [propId]: _spent, ...others } = state.streaks;
  const streaks = escalated ? others : { ...state.streaks, [propId]: { count, goal, at: now } };
  const outcome: CornerOutcome = rare ? "rare" : completed ? "sequence" : escalated ? "escalate" : "tap";
  return { state: { streaks, progress }, outcome };
}

export type CornerCue = { sfx: SfxId; params: SfxParams; delayMs: number };

const cue = (sfx: SfxId, def: CornerPropDef, variant: number, build: number, delayMs: number, quiet: boolean): CornerCue => ({
  sfx,
  params: { variation: def.pitch, sandVariant: variant % CORNER_SOUND_VARIANTS, sandBuild: build, ...(quiet ? { quiet: true } : {}) },
  delayMs,
});

/**
 * The sound of an outcome: a plain tap one cue; an escalation three of the prop's own; the rare outcome
 * four astral chimes up the variants; a sequence four of the prop's own. `quiet` (the animation queue is
 * draining) keeps the first cue only, turned down.
 */
export function soundsFor(def: CornerPropDef, outcome: CornerOutcome, quiet: boolean): CornerCue[] {
  const cues: CornerCue[] = [];
  switch (outcome) {
    case "tap":
      cues.push(cue(def.sfx, def, def.variant, CORNER_TAP_BUILD, 0, quiet));
      break;
    case "escalate":
      for (let step = 0; step < CORNER_ESCALATE_CUES; step += 1) cues.push(cue(def.sfx, def, def.variant + step, CORNER_BIG_BUILD, step * CORNER_ESCALATE_STEP_MS, quiet));
      break;
    case "rare":
      for (let step = 0; step < CORNER_SOUND_VARIANTS; step += 1) cues.push(cue("tapAstral", def, step, CORNER_BIG_BUILD, step * CORNER_RARE_STEP_MS, quiet));
      break;
    case "sequence":
      for (let step = 0; step < CORNER_SOUND_VARIANTS; step += 1) cues.push(cue(def.sfx, def, step, CORNER_BIG_BUILD, step * CORNER_SEQUENCE_STEP_MS, quiet));
      break;
  }
  return quiet ? cues.slice(0, 1) : cues;
}
