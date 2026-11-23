// #3701: the effects tests' one cue-end helper and one planning setup (signatures.ts), which every test of
// R200's bound takes in place of a copy of its own.

import { describe, expect, it } from "vitest";

import { FX_INTENSITY_SCALE, FX_MAX_PARTICLE_LIFE_MS } from "../fx/constants.ts";
import { createFxMemory } from "../fx/memory.ts";
import type { FxCardFacts, FxCue } from "../fx/types.ts";
import { endOf, envOf, tid } from "./signatures.ts";

const AT = tid("board");
const DELAY = 100;
const FLIGHT = 300;
const DURATION = 250;

describe("#3701 the shared cue-end helper and planning setup", () => {
  it("#3701 a burst ends its particles' life after its delay", () => {
    const burst: FxCue = { kind: "burst", preset: "fire", at: AT, delayMs: DELAY, count: 8, spread: "point", power: 1 };
    expect(endOf(burst)).toBe(DELAY + FX_MAX_PARTICLE_LIFE_MS);
  });

  it("#3701 a projectile ends after its flight and the burst it lands in", () => {
    const projectile: FxCue = { kind: "projectile", preset: "fire", from: AT, to: AT, delayMs: DELAY, flightMs: FLIGHT, density: 1 };
    expect(endOf(projectile)).toBe(DELAY + FLIGHT + FX_MAX_PARTICLE_LIFE_MS);
  });

  it("#3701 a shot ends after its flight and the burst it lands in", () => {
    const shot: FxCue = { kind: "shot", shot: { motion: "beam" }, colors: ["#ffffff"], preset: "spark", from: AT, to: AT, delayMs: DELAY, flightMs: FLIGHT, density: 1 };
    expect(endOf(shot)).toBe(DELAY + FLIGHT + FX_MAX_PARTICLE_LIFE_MS);
  });

  it("#3701 a shake ends at its delay", () => {
    const shake: FxCue = { kind: "shake", trauma: 0.5, delayMs: DELAY };
    expect(endOf(shake)).toBe(DELAY);
  });

  it("#3701 a cue with a durationMs ends at its delay plus its duration", () => {
    const crack: FxCue = { kind: "crack", at: AT, delayMs: DELAY, durationMs: DURATION };
    expect(endOf(crack)).toBe(DELAY + DURATION);
  });

  it("#3701 envOf plans at normal intensity with no card facts, a fresh memory each call", () => {
    const env = envOf();
    expect(env.intensity).toBe(FX_INTENSITY_SCALE.normal);
    expect(env.card("core-001")).toBeUndefined();
    expect(envOf().memory).not.toBe(env.memory);
  });

  it("#3701 envOf's over replaces the intensity, the card facts and the memory", () => {
    const facts: FxCardFacts = { rarity: "Legendary", attack: 6, health: 6 };
    const memory = createFxMemory();
    const env = envOf({ intensity: FX_INTENSITY_SCALE.off, card: () => facts, memory });
    expect(env.intensity).toBe(FX_INTENSITY_SCALE.off);
    expect(env.card("core-001")).toBe(facts);
    expect(env.memory).toBe(memory);
  });
});
