// #1444: with `FX_FLAVOR_LIVE` off (mocked off here; it ships on) every cue is exactly today's:
// a Ranged attacker lunges as any other does.

import type { GameEvent, PlayerView, UnitView } from "@jackioh/shared";
import { describe, expect, it, vi } from "vitest";

vi.mock("../fx/constants.ts", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../fx/constants.ts")>()),
  FX_FLAVOR_LIVE: false,
}));

import { FX_INTENSITY_SCALE } from "../fx/constants.ts";
import { createFxMemory } from "../fx/memory.ts";
import { planStage } from "../fx/stage.ts";
import type { FxCue } from "../fx/types.ts";
import { fullBoardView, withEvents } from "../test/fixtures.ts";
import { planEntries } from "./animations.ts";

/** `fullBoardView` with your unit `id` given Ranged. */
function rangedBoard(id: string): PlayerView {
  const view = fullBoardView();
  const patch = (unit: UnitView | null): UnitView | null =>
    unit !== null && unit.instanceId === id ? { ...unit, keywords: [...unit.keywords, { kind: "Ranged" }] } : unit;
  return { ...view, you: { ...view.you, units: view.you.units.map(patch) } };
}

function attack(view: PlayerView, attackerId: string, targetId: string): FxCue[] {
  const events: GameEvent[] = [{ type: "attackDeclared", attackerId, targetId, forced: false }];
  const shown = withEvents(view, events);
  const entries = planEntries(events, shown, false);
  expect(entries).toHaveLength(1);
  const env = { intensity: FX_INTENSITY_SCALE.normal, card: () => undefined, memory: createFxMemory() };
  return planStage({ ...entries[0]!, durationMs: 350 }, shown, env);
}

describe("#1444 the flavor effects stay off when the flag is off", () => {
  it("#1444 with it off a Ranged attacker lunges exactly as today", () => {
    const ranged = attack(rangedBoard("u1"), "u1", "u6");
    expect(ranged).toEqual(attack(fullBoardView(), "u1", "u6"));
    expect(ranged.map((cue) => cue.kind)).toEqual(["lunge", "burst"]);
    expect(ranged[0]).not.toHaveProperty("still");
    // At a hero too.
    expect(attack(rangedBoard("u1"), "u1", "hero-p2")).toEqual(attack(fullBoardView(), "u1", "hero-p2"));
  });
});
