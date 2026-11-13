// S+ #109 Lancer, #126 Vela, #127 Corvus's line on the effects layer (R3554005). R200 binds
// it: every cue starts inside its entry, a beam lands by its end, and a ring is gone within the
// tail.

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

describe("R3554005 the line recipe", () => {
  const cells = [
    { player: ME, row: "units", lane: 1 },
    { player: THEM, row: "units", lane: 3 },
    { player: THEM, row: "backrow", lane: 5 },
  ] as const;
  const zone = (at: number): string => {
    const cell = cells[at];
    if (cell === undefined) throw new Error(`no cell ${String(at)}`);
    return testid.zone(sideOf(VIEW, cell.player), cell.row, cell.lane);
  };

  it("R3554005 the line rings each cell and fires one beam", () => {
    const event: GameEvent = { type: "lineDrawn", player: ME, cells: [...cells] };
    const { cues } = plan(event);
    const rings = cues.flatMap((cue) => (cue.kind === "ring" && cue.at.kind === "testid" ? [cue.at.testid] : []));
    expect(rings).toEqual([zone(1), zone(2)]);
    const beams = cues.flatMap((cue) =>
      cue.kind === "projectile" && cue.from.kind === "testid" && cue.to.kind === "testid" ? [[cue.from.testid, cue.to.testid]] : [],
    );
    expect(beams).toEqual([[zone(0), zone(2)]]);
  });

  it("R3554005 a laser fires a fire beam", () => {
    const event: GameEvent = { type: "lineDrawn", player: ME, cells: [...cells], laser: true };
    const { cues } = plan(event);
    const beams = cues.filter((cue) => cue.kind === "projectile");
    expect(beams).toHaveLength(1);
    expect(beams[0]).toMatchObject({ preset: "fire" });
  });

  it("R3554005 R200 every cue starts inside its entry, a beam lands by its end and a ring is gone within the tail", () => {
    const event: GameEvent = { type: "lineDrawn", player: ME, cells: [...cells] };
    const { cues, D } = plan(event);
    for (const cue of cues) {
      expect(cue.delayMs).toBeGreaterThanOrEqual(0);
      expect(cue.delayMs).toBeLessThanOrEqual(D);
      if (cue.kind === "projectile") expect(cue.delayMs + cue.flightMs).toBeLessThanOrEqual(D);
      if (cue.kind === "ring") expect(cue.delayMs + cue.durationMs).toBeLessThanOrEqual(D + FX_MAX_TAIL_MS);
    }
  });

  it("R3554005 a line through one cell plans nothing", () => {
    expect(plan({ type: "lineDrawn", player: ME, cells: [cells[0]] }).cues).toEqual([]);
  });
});
