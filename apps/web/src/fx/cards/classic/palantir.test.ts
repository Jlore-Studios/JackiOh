// R3389001 Palantir's entrance numbers (fx/cards/classic/palantir.ts): the seeing-orb's particle
// counts at intensity "normal" (rule 9). The signatures suite plans the entrance end to end but pins
// no count; a 16→17 TUNING mutant survived it. Here the counts themselves are pinned, so that mutant
// fails here.

import { describe, expect, it } from "vitest";

import type { GameEvent } from "@jackioh/shared";

import type { FxBurstCue } from "../../types.ts";
import { seeingOrb } from "./palantir.ts";
import { baseView } from "../../../test/fixtures.ts";

function bursts(intensity: number, landed = false): FxBurstCue[] {
  const landing: Extract<GameEvent, { type: "summoned" }> = {
    type: "summoned",
    player: "p1",
    instanceId: "u1",
    defId: "core-004",
    row: "units",
    lane: 1,
  };
  return seeingOrb({
    D: 2000,
    tgt: "fx-board",
    intensity,
    ...(landed ? { view: baseView(), landing } : {}),
  }).filter((cue): cue is FxBurstCue => cue.kind === "burst");
}

describe("R3389001 the seeing-orb's numbers", () => {
  it("throws the orb's, the iris's and the scan's counts at intensity normal", () => {
    const counts = bursts(1, true).map((cue) => cue.count);
    // The orb, the iris, and one scan burst per lane (game/contract LANES).
    expect(counts).toEqual([16, 20, 5, 5, 5, 5, 5]);
  });

  it("draws no scan with no board to cross", () => {
    expect(bursts(1).map((cue) => cue.count)).toEqual([16, 20]);
  });

  it("scales every burst with the intensity, never below one particle", () => {
    const counts = bursts(0.5, true).map((cue) => cue.count);
    expect(counts).toEqual([8, 10, 3, 3, 3, 3, 3]);
    expect(bursts(0).every((cue) => cue.count >= 1)).toBe(true);
  });
});
