// The animation budgets (game/animations.ts): what each entry costs in time, as literals. The
// relations are asserted elsewhere; here the shipped numbers themselves are pinned, so a mutant
// moving any budget fails here by name. STARSHIP_FUSE_MS is the starshipFused row's own time,
// not a second copy of it.

import { describe, expect, it } from "vitest";

import {
  ANIMATIONS,
  CAST_BUDGET_MS,
  CAST_ENTRY_MS,
  ROLLED_ENTRY_MS,
  SET_PIECE_HOLD_CAP_MS,
  SET_PIECE_HOLD_MS,
  SET_PIECE_SHORT_HOLD_MS,
  SET_PIECES_PER_ACTION,
  SET_PIECE_SKIP_HINT_MS,
  SIGNATURE_RUN_MS,
  STARSHIP_FUSE_MS,
  STARSHIP_FUSE_STAGGERED_MAX,
  STARSHIP_LAUNCH_HOLD_MS,
  STARSHIP_SET_PIECE,
  SWEEP_MS,
  TAKEOVER_HOLD_MS,
  TAKEOVER_SET_PIECE,
  WIPE_MS,
  ZONE_IMPACT_MS,
  starshipFuseMs,
} from "./animations.ts";
import { FX_STARSHIP_FUSE_STAGGER_MS } from "../fx/constants.ts";

describe("animation budgets", () => {
  it("pins the cast, zone and sweep budgets", () => {
    expect(CAST_ENTRY_MS).toBe(800);
    expect(CAST_BUDGET_MS).toBe(2200);
    expect(CAST_BUDGET_MS).toBeGreaterThan(CAST_ENTRY_MS);
    expect(ZONE_IMPACT_MS).toBe(700);
    expect(SWEEP_MS).toBe(600);
    expect(WIPE_MS).toBe(600);
    expect(SIGNATURE_RUN_MS).toBe(1200);
    expect(ROLLED_ENTRY_MS).toBe(1200);
  });

  it("pins the set-piece and takeover holds and the one-action cap", () => {
    expect(TAKEOVER_HOLD_MS).toBe(1000);
    expect(SET_PIECE_HOLD_MS).toBe(2000);
    expect(SET_PIECE_HOLD_CAP_MS).toBe(3000);
    expect(SET_PIECE_SHORT_HOLD_MS).toBe(600);
    expect(SET_PIECES_PER_ACTION).toBe(1);
    expect(SET_PIECE_SKIP_HINT_MS).toBe(500);
    expect(TAKEOVER_SET_PIECE).toBe("takeover");
    expect(STARSHIP_SET_PIECE).toBe("starship");
    expect(STARSHIP_LAUNCH_HOLD_MS).toBe(2400);
  });

  it("STARSHIP_FUSE_MS is the starshipFused row's own time, staggered six deep", () => {
    expect(STARSHIP_FUSE_MS).toBe(ANIMATIONS.starshipFused.durationMs);
    expect(STARSHIP_FUSE_STAGGERED_MAX).toBe(6);
    expect(FX_STARSHIP_FUSE_STAGGER_MS).toBe(250);
    expect(starshipFuseMs(1)).toBe(STARSHIP_FUSE_MS);
    expect(starshipFuseMs(2)).toBe(STARSHIP_FUSE_MS + FX_STARSHIP_FUSE_STAGGER_MS);
    expect(starshipFuseMs(0)).toBe(STARSHIP_FUSE_MS);
    expect(starshipFuseMs(99)).toBe(starshipFuseMs(STARSHIP_FUSE_STAGGERED_MAX));
  });
});
