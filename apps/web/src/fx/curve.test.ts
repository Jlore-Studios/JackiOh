// C+ #62 KY's Papaya's curve on the effects layer (R831003): `curveDrawn` names the zones the curve
// passes through, in lane order, and the `curve` recipe runs an arcane bolt from each to the next with a
// ring at each as the line reaches it. R200 binds it: every cue starts inside its entry, a bolt lands by
// its end, and a ring is gone within the tail.

import type { GameEvent, PlayerView } from "@jackioh/shared";
import { describe, expect, it } from "vitest";

import { planEntries, type AnimationEntry } from "../game/animations.ts";
import { sideOf, testid } from "../game/contract.ts";
import { fullBoardView, withEvents } from "../test/fixtures.ts";
import { FX_INTENSITY_SCALE, FX_MAX_TAIL_MS } from "./constants.ts";
import { planFx } from "./cues.ts";
import { createFxMemory } from "./memory.ts";
import type { FxCue } from "./types.ts";

const VIEW: PlayerView = fullBoardView();
const ME = VIEW.viewer;
const THEM = ME === "p1" ? "p2" : "p1";

function plan(event: GameEvent): { cues: FxCue[]; D: number } {
  const shown = withEvents(VIEW, [event]);
  const entries = planEntries([event], shown, false);
  expect(entries).toHaveLength(1);
  const entry = entries[0] as AnimationEntry;
  const env = { intensity: FX_INTENSITY_SCALE.normal, card: () => undefined, memory: createFxMemory() };
  env.memory.remember(entry.events);
  return { cues: planFx(entry, shown, env), D: entry.durationMs };
}

describe("R831003 the curve recipe", () => {
  const cells = [
    { player: ME, row: "backrow", lane: 1 },
    { player: ME, row: "units", lane: 3 },
    { player: THEM, row: "units", lane: 5 },
  ] as const;
  const event: GameEvent = { type: "curveDrawn", player: ME, cells: [...cells] };
  const zone = (at: number): string => {
    const cell = cells[at];
    if (cell === undefined) throw new Error(`no cell ${String(at)}`);
    return testid.zone(sideOf(VIEW, cell.player), cell.row, cell.lane);
  };

  it("R831003 rings each cell in lane order and runs a bolt from each to the next", () => {
    const { cues } = plan(event);
    const rings = cues.flatMap((cue) => (cue.kind === "ring" && cue.at.kind === "testid" ? [cue.at.testid] : []));
    expect(rings).toEqual([zone(0), zone(1), zone(2)]);
    const bolts = cues.flatMap((cue) =>
      cue.kind === "projectile" && cue.from.kind === "testid" && cue.to.kind === "testid" ? [[cue.from.testid, cue.to.testid]] : [],
    );
    expect(bolts).toEqual([
      [zone(0), zone(1)],
      [zone(1), zone(2)],
    ]);
  });

  it("R831003 R200 every cue starts inside its entry, a bolt lands by its end and a ring is gone within the tail", () => {
    const { cues, D } = plan(event);
    for (const cue of cues) {
      expect(cue.delayMs).toBeGreaterThanOrEqual(0);
      expect(cue.delayMs).toBeLessThanOrEqual(D);
      if (cue.kind === "projectile") expect(cue.delayMs + cue.flightMs).toBeLessThanOrEqual(D);
      if (cue.kind === "ring") expect(cue.delayMs + cue.durationMs).toBeLessThanOrEqual(D + FX_MAX_TAIL_MS);
    }
  });

  it("R831003 a curve through no cell plans nothing", () => {
    expect(plan({ type: "curveDrawn", player: ME, cells: [] }).cues).toEqual([]);
  });
});
