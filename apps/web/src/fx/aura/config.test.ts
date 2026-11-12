// R3342000's tile-aura numbers (fx/aura/config.ts, CLAUDE.md rule 9): the edge glow and motes
// for Legendary, the richer pattern for Mythic. Each shipped value is pinned; a mutant moving one
// fails here by name.

import { describe, expect, it } from "vitest";

import {
  TILE_DIM,
  TILE_EDGE,
  TILE_FADE_OUT_MS,
  TILE_FLICKER_MS,
  TILE_GLOW_PX,
  TILE_HUE_SPREAD_DEG,
  TILE_LIT_BOOST,
  TILE_MOTE_TRAVEL_PX,
  TILE_MOTES_LEGENDARY,
  TILE_MOTES_MYTHIC,
  TILE_PARTICLE_SHARE,
  TILE_PATTERN_OPACITY,
  TILE_PULSE_MS,
  TILE_RIPPLE_MS,
  TILE_TOUCH_GAP_MS,
  TILE_VARIANTS,
} from "./config.ts";

describe("tile aura config", () => {
  it("pins the mote counts, the particle share and the hue spread", () => {
    expect(TILE_MOTES_LEGENDARY).toBe(3);
    expect(TILE_MOTES_MYTHIC).toBe(5);
    expect(TILE_PARTICLE_SHARE).toBe(0.2);
    expect(TILE_HUE_SPREAD_DEG).toBe(18);
    expect(TILE_VARIANTS).toBe(12);
  });

  it("gives Mythic the wider glow and the stronger edge", () => {
    expect(TILE_GLOW_PX).toEqual({ Legendary: 8, Mythic: 12 });
    expect(TILE_EDGE).toEqual({ Legendary: 0.45, Mythic: 0.6 });
    expect(TILE_GLOW_PX.Mythic).toBeGreaterThan(TILE_GLOW_PX.Legendary);
    expect(TILE_EDGE.Mythic).toBeGreaterThan(TILE_EDGE.Legendary);
  });

  it("pins the pattern opacity and the dim/lit levels", () => {
    expect(TILE_PATTERN_OPACITY).toBe(0.35);
    expect(TILE_DIM).toBe(0.3);
    expect(TILE_LIT_BOOST).toBe(0.4);
  });

  it("pins the pulse, ripple, flicker and fade schedule", () => {
    expect(TILE_PULSE_MS).toBe(6000);
    expect(TILE_RIPPLE_MS).toBe(700);
    expect(TILE_FLICKER_MS).toBe(450);
    expect(TILE_FADE_OUT_MS).toBe(350);
  });

  it("pins the touch gap and the mote travel", () => {
    expect(TILE_TOUCH_GAP_MS).toBe(250);
    expect(TILE_MOTE_TRAVEL_PX).toBe(24);
  });
});
