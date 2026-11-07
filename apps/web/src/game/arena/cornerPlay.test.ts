// R3348001 (#3348): the corner props' machine: escalation, the rare outcome from the client's own random
// source, the hidden sequences and the sounds, all replayed with a scripted or seeded draw.

import { describe, expect, it } from "vitest";

import { QUIET_SFX_GAIN } from "../../audio/constants.ts";
import { createRng } from "../../fx/rng.ts";
import {
  CORNER_ESCALATE_STEP_MS,
  CORNER_ESCALATE_TAPS,
  CORNER_RARE_CHANCE,
  CORNER_RARE_STEP_MS,
  CORNER_SEQUENCE_GAP_MS,
  CORNER_STREAK_GAP_MS,
} from "./config.ts";
import { emptyCornerState, soundsFor, tapCorner, type CornerOutcome, type CornerState } from "./cornerPlay.ts";
import { propsOf, sequenceOf } from "./props.ts";

const TAVERN = "table-tavern";
const WOOD = "table-wood";
const [CANDLE, MUG] = propsOf(TAVERN);
if (CANDLE === undefined || MUG === undefined) throw new Error("the tavern has two props");

/** A draw that reads `script` in order, then `rest` for ever. */
function scripted(script: readonly number[], rest = 0.5): () => number {
  let at = 0;
  return () => script[at++] ?? rest;
}

/** `taps` taps of `propId` on `mat`, `gap` ms apart, from `state`; the outcome of each. */
function run(
  state: CornerState,
  mat: string,
  propId: string,
  taps: number,
  random: () => number,
  start = 0,
  gap = 100,
): { state: CornerState; outcomes: CornerOutcome[]; end: number } {
  const outcomes: CornerOutcome[] = [];
  let now = start;
  let current = state;
  for (let tap = 0; tap < taps; tap += 1) {
    const result = tapCorner(current, mat, propId, now, random);
    current = result.state;
    outcomes.push(result.outcome);
    now += gap;
  }
  return { state: current, outcomes, end: now };
}

describe("R3348001 escalation", () => {
  it("R3348001 the goal runs from CORNER_ESCALATE_TAPS min to max, and the tap that reaches it escalates", () => {
    expect(CORNER_ESCALATE_TAPS).toEqual({ min: 5, max: 7 });
    // The goal is the first draw: 0.2 gives the fewest taps, 0.99 the most, 0.5 the middle.
    for (const [draw, goal] of [[0.2, 5], [0.5, 6], [0.99, 7]] as const) {
      const { outcomes } = run(emptyCornerState(), WOOD, "wood-dice", goal + 1, scripted([draw, 0.5]));
      expect(outcomes.slice(0, goal - 1), `goal ${String(goal)}`).toEqual(Array<CornerOutcome>(goal - 1).fill("tap"));
      expect(outcomes[goal - 1], `goal ${String(goal)}`).toBe("escalate");
      expect(outcomes[goal], "the streak starts again after a payoff").toBe("tap");
    }
  });

  it("R3348001 taps further apart than CORNER_STREAK_GAP_MS start a new streak", () => {
    const random = scripted([0.2], 0.5);
    const { state, end } = run(emptyCornerState(), WOOD, "wood-dice", 4, random);
    // A fifth tap too late: the count restarts at 1, so it is a plain tap, not the goal's escalation.
    const late = tapCorner(state, WOOD, "wood-dice", end + CORNER_STREAK_GAP_MS + 1, random);
    expect(late.outcome).toBe("tap");
    expect(late.state.streaks["wood-dice"]?.count).toBe(1);
    // Just inside the gap keeps counting.
    const kept = run(emptyCornerState(), WOOD, "wood-dice", 4, scripted([0.2], 0.5), 0, CORNER_STREAK_GAP_MS);
    expect(kept.state.streaks["wood-dice"]?.count).toBe(4);
  });

  it("R3348001 each prop keeps its own streak", () => {
    const random = scripted([], 0.5);
    let state = emptyCornerState();
    state = run(state, WOOD, "wood-dice", 3, random).state;
    state = run(state, WOOD, "wood-coin", 2, random, 1000).state;
    expect(state.streaks["wood-dice"]?.count).toBe(3);
    expect(state.streaks["wood-coin"]?.count).toBe(2);
  });
});

describe("R3348001 the rare outcome", () => {
  it("R3348001 below CORNER_RARE_CHANCE is rare, and exactly it is not", () => {
    expect(CORNER_RARE_CHANCE).toBe(1 / 1000);
    // First tap: the goal draw, then the rare draw.
    expect(tapCorner(emptyCornerState(), WOOD, "wood-dice", 0, scripted([0.5, CORNER_RARE_CHANCE / 2])).outcome).toBe("rare");
    expect(tapCorner(emptyCornerState(), WOOD, "wood-dice", 0, scripted([0.5, CORNER_RARE_CHANCE])).outcome).toBe("tap");
  });

  it("R3348001 a seeded client random gives about one rare tap in a thousand", () => {
    const random = createRng(3348);
    let state = emptyCornerState();
    let rare = 0;
    const taps = 200_000;
    for (let tap = 0; tap < taps; tap += 1) {
      // Spread over time so streaks stay short; the rare roll is independent of them.
      const result = tapCorner(state, WOOD, tap % 2 === 0 ? "wood-dice" : "wood-coin", tap * (CORNER_STREAK_GAP_MS + 1), random);
      state = result.state;
      if (result.outcome === "rare") rare += 1;
    }
    expect(rare).toBeGreaterThan((taps * CORNER_RARE_CHANCE) / 2);
    expect(rare).toBeLessThan((taps * CORNER_RARE_CHANCE) * 1.5);
  });

  it("R3348001 the rare roll wins over a payoff due at the same tap", () => {
    // Goal 5 (0.2), then one rare draw a tap: the 5th tap's is below the chance.
    const draws = [0.2, 0.5, 0.5, 0.5, 0.5, CORNER_RARE_CHANCE / 2];
    const { outcomes } = run(emptyCornerState(), WOOD, "wood-dice", 5, scripted(draws));
    expect(outcomes).toEqual(["tap", "tap", "tap", "tap", "rare"]);
  });
});

describe("R3348001 the hidden sequences", () => {
  const sequence = sequenceOf(TAVERN) ?? [];
  const plain = (): number => 0.5;

  it("R3348001 tapping the sequence's props in order completes it, and a second go needs it again", () => {
    expect(sequence).toEqual([MUG.id, CANDLE.id, MUG.id]);
    let state = emptyCornerState();
    const outcomes: CornerOutcome[] = [];
    sequence.forEach((id, step) => {
      const result = tapCorner(state, TAVERN, id, step * 200, plain);
      state = result.state;
      outcomes.push(result.outcome);
    });
    expect(outcomes).toEqual(["tap", "tap", "sequence"]);
    expect(tapCorner(state, TAVERN, MUG.id, 700, plain).outcome).toBe("tap");
  });

  it("R3348001 a wrong tap loses its place, but the sequence's first prop starts it over", () => {
    let state = emptyCornerState();
    state = tapCorner(state, TAVERN, MUG.id, 0, plain).state;
    state = tapCorner(state, TAVERN, MUG.id, 100, plain).state; // wrong: the candle was next, but a mug starts over
    state = tapCorner(state, TAVERN, CANDLE.id, 200, plain).state;
    expect(tapCorner(state, TAVERN, MUG.id, 300, plain).outcome).toBe("sequence");

    let lost = emptyCornerState();
    lost = tapCorner(lost, TAVERN, MUG.id, 0, plain).state;
    lost = tapCorner(lost, TAVERN, CANDLE.id, 100, plain).state;
    lost = tapCorner(lost, TAVERN, CANDLE.id, 200, plain).state; // wrong, and not the first: nothing held
    expect(tapCorner(lost, TAVERN, MUG.id, 300, plain).outcome).toBe("tap");
  });

  it("R3348001 a sequence not finished within CORNER_SEQUENCE_GAP_MS of its last tap is lost", () => {
    let state = emptyCornerState();
    state = tapCorner(state, TAVERN, MUG.id, 0, plain).state;
    state = tapCorner(state, TAVERN, CANDLE.id, 100, plain).state;
    expect(tapCorner(state, TAVERN, MUG.id, 100 + CORNER_SEQUENCE_GAP_MS + 1, plain).outcome).toBe("tap");
    expect(tapCorner(state, TAVERN, MUG.id, 100 + CORNER_SEQUENCE_GAP_MS, plain).outcome).toBe("sequence");
  });

  it("R3348001 a sequence outranks an escalation due with it, but never the rare roll", () => {
    // Four mug taps (goal 5) leave the sequence at its first step; the candle is its second.
    const warm = run(emptyCornerState(), TAVERN, MUG.id, 4, scripted([0.2], 0.5), 0, 10).state;
    const before = tapCorner(warm, TAVERN, CANDLE.id, 40, plain);
    expect(before.outcome).toBe("tap");
    // The fifth mug tap reaches the goal and finishes the sequence: the sequence is what plays.
    expect(tapCorner(before.state, TAVERN, MUG.id, 50, plain).outcome).toBe("sequence");
    // The streak's draw is spent, so the first draw is the rare roll, and it wins over both.
    expect(tapCorner(before.state, TAVERN, MUG.id, 50, scripted([CORNER_RARE_CHANCE / 2])).outcome).toBe("rare");
  });

  it("R3348001 a mat with no sequence never completes one", () => {
    const { outcomes } = run(emptyCornerState(), WOOD, "wood-dice", 3, plain);
    expect(outcomes).not.toContain("sequence");
  });
});

describe("R3348001 the sounds", () => {
  it("R3348001 a tap sounds once; an escalation three times; the rare outcome four astral chimes; a sequence four", () => {
    const tap = soundsFor(CANDLE, "tap", false);
    expect(tap).toHaveLength(1);
    expect(tap[0]?.sfx).toBe(CANDLE.sfx);
    expect(tap[0]?.params).toEqual({ variation: CANDLE.pitch, sandVariant: CANDLE.variant, sandBuild: 2 });

    const escalate = soundsFor(CANDLE, "escalate", false);
    expect(escalate.map((cue) => cue.delayMs)).toEqual([0, CORNER_ESCALATE_STEP_MS, CORNER_ESCALATE_STEP_MS * 2]);
    expect(escalate.every((cue) => cue.sfx === CANDLE.sfx && cue.params.sandBuild === 6)).toBe(true);

    const rare = soundsFor(CANDLE, "rare", false);
    expect(rare.map((cue) => cue.sfx)).toEqual(Array(4).fill("tapAstral"));
    expect(rare.map((cue) => cue.params.sandVariant)).toEqual([0, 1, 2, 3]);
    expect(rare.map((cue) => cue.delayMs)).toEqual([0, 1, 2, 3].map((step) => step * CORNER_RARE_STEP_MS));

    const sequence = soundsFor(CANDLE, "sequence", false);
    expect(sequence).toHaveLength(4);
    expect(sequence.every((cue) => cue.sfx === CANDLE.sfx)).toBe(true);
    expect(sequence.map((cue) => cue.params.sandVariant)).toEqual([0, 1, 2, 3]);
  });

  it("R3348001 quiet keeps the first cue only, marked quiet, and the others are never marked", () => {
    for (const outcome of ["tap", "escalate", "rare", "sequence"] as const) {
      const quiet = soundsFor(CANDLE, outcome, true);
      expect(quiet, outcome).toHaveLength(1);
      expect(quiet[0]?.params.quiet, outcome).toBe(true);
      expect(soundsFor(CANDLE, outcome, false).some((cue) => cue.params.quiet === true), outcome).toBe(false);
    }
    expect(QUIET_SFX_GAIN).toBeLessThan(1);
  });
});
