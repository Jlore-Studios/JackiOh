// R670: the bespoke entrances of the marquee Legendary and Mythic Units (entrances.ts), planned the
// way FxLayer plans them: each entry remembered, then planned against the view the runner planned it
// against.

import catalogJson from "@jackioh/cards/catalog.json";
import type { GameEvent, PlayerView } from "@jackioh/shared";
import { describe, expect, it } from "vitest";

import { ANIMATIONS, planEntries } from "../game/animations.ts";
import { testid } from "../game/contract.ts";
import { fullBoardView, withEvents } from "../test/fixtures.ts";
import { CARD_FX } from "./cardFx.ts";
import { FX_INTENSITY_SCALE, FX_MAX_PARTICLE_LIFE_MS, FX_MAX_TAIL_MS, FX_SLAM_AT } from "./constants.ts";
import { planFx } from "./cues.ts";
import { ENTRANCES, ENTRANCE_TRAUMA, isEntranceKey, type EntranceKey } from "./entrances.ts";
import { createFxMemory } from "./memory.ts";
import type { FxCardFacts, FxCue, FxPlanEnv } from "./types.ts";

type CatalogEntry = { name: string; type: string; rarity: string };
const CATALOG = catalogJson as unknown as Record<string, CatalogEntry>;

const MARQUEE: readonly (readonly [string, EntranceKey])[] = Object.entries(CARD_FX).flatMap(([defId, key]) =>
  isEntranceKey(key) ? [[defId, key] as const] : [],
);

const LEGENDARY_FACTS: FxCardFacts = { rarity: "Legendary", attack: 6, health: 6 };

function envOf(over: Partial<FxPlanEnv> = {}): FxPlanEnv {
  return { intensity: FX_INTENSITY_SCALE.normal, card: () => LEGENDARY_FACTS, memory: createFxMemory(), ...over };
}

const summoned = (player: "p1" | "p2", instanceId: string, defId: string, row: "units" | "backrow" = "units", lane = 3): GameEvent => ({
  type: "summoned",
  player,
  instanceId,
  defId,
  row,
  lane,
});
const played = (player: "p1" | "p2", instanceId: string, defId: string): GameEvent => ({ type: "cardPlayed", player, instanceId, defId, costPaid: 7 });

function planAll(events: readonly GameEvent[], env: FxPlanEnv = envOf(), D?: number, view: PlayerView = fullBoardView()): FxCue[] {
  return planEntries(events, withEvents(view, [...events]), false).flatMap((original) => {
    const entry = D === undefined ? original : { ...original, durationMs: D };
    env.memory.remember(entry.events);
    return planFx(entry, entry.view, env);
  });
}

function endOf(cue: FxCue): number {
  switch (cue.kind) {
    case "burst":
      return cue.delayMs + FX_MAX_PARTICLE_LIFE_MS;
    case "projectile":
      return cue.delayMs + cue.flightMs + FX_MAX_PARTICLE_LIFE_MS;
    case "shake":
      return cue.delayMs;
    default:
      return cue.delayMs + ("durationMs" in cue ? cue.durationMs : 0);
  }
}

describe("R670 marquee Legendary and Mythic entrances", () => {
  it("R670 a handful of cards have one, each a Legendary or Mythic Unit of the real catalog, and every key names a recipe", () => {
    expect(MARQUEE.length).toBeGreaterThanOrEqual(4);
    for (const [defId, key] of MARQUEE) {
      const def = CATALOG[defId];
      expect(def, defId).toBeDefined();
      expect(def?.type, defId).toBe("Unit");
      expect(["Legendary", "Mythic"], defId).toContain(def?.rarity);
      expect(typeof ENTRANCES[key]).toBe("function");
    }
    // Each entrance is its own: no two marquee cards share one.
    expect(new Set(MARQUEE.map(([, key]) => key)).size).toBe(MARQUEE.length);
  });

  it("R670 a marquee summoned into a unit zone plans its own entrance in place of the rarity's rays and gold", () => {
    for (const [defId, key] of MARQUEE) {
      const cues = planAll([summoned("p1", "m1", defId)]);
      const D = ANIMATIONS.summoned.durationMs;
      const expected = ENTRANCES[key]({ D, tgt: testid.zone("you", "units", 3), intensity: FX_INTENSITY_SCALE.normal });
      expect(cues, defId).toEqual(expected);
      expect(cues.some((cue) => cue.kind === "burst" && cue.preset === "gold" && cue.spread === "area"), defId).toBe(false);
      const shakes = cues.filter((cue) => cue.kind === "shake");
      expect(shakes, defId).toEqual([{ kind: "shake", trauma: ENTRANCE_TRAUMA[key], delayMs: Math.round(FX_SLAM_AT * D) }]);
    }
  });

  it("R670 a plain Legendary still gets the rarity entrance", () => {
    const cues = planAll([summoned("p1", "m1", "core-052")]);
    expect(cues.some((cue) => cue.kind === "rays" && cue.tone === "legendary")).toBe(true);
    expect(cues.some((cue) => cue.kind === "burst" && cue.preset === "gold" && cue.spread === "area")).toBe(true);
  });

  it("R670 played from the hand (the collapsed play and summon) and on the other seat, it plays the same entrance at that seat's zone", () => {
    const [defId, key] = MARQUEE[0] ?? ["", "voidCollapse"];
    const pair = [played("p1", "m1", defId), summoned("p1", "m1", defId)];
    // The pair is one entry, with the duration the runner gives the pair.
    const D = planEntries(pair, withEvents(fullBoardView(), pair), false)[0]?.durationMs ?? 0;
    const mine = planAll(pair);
    expect(mine).toEqual(ENTRANCES[key]({ D, tgt: testid.zone("you", "units", 3), intensity: FX_INTENSITY_SCALE.normal }));
    const theirs = planAll([played("p2", "m2", defId), summoned("p2", "m2", defId, "units", 2)]);
    expect(theirs).toEqual(ENTRANCES[key]({ D, tgt: testid.zone("opponent", "units", 2), intensity: FX_INTENSITY_SCALE.normal }));
    // R202: no cue carries a definition or a name.
    const text = JSON.stringify(theirs);
    expect(text).not.toContain(defId);
    expect(text).not.toContain(CATALOG[defId]?.name ?? "\u0000");
  });

  it("R670 R202 a hidden summon and a backrow set get no entrance", () => {
    const hidden = planAll([summoned("p2", "hidden", "hidden")]);
    expect(hidden.some((cue) => cue.kind === "rays" || cue.kind === "crack" || cue.kind === "shake")).toBe(false);
    for (const [defId] of MARQUEE) {
      const set = planAll([summoned("p1", "m1", defId, "backrow")]);
      expect(set.map((cue) => cue.kind), defId).toEqual(["burst"]);
    }
  });

  it("R670 R201 an entrance paces nothing: the entry's duration is the summon's, marquee or not", () => {
    for (const [defId] of MARQUEE) {
      const view = fullBoardView();
      const marquee = planEntries([played("p1", "m1", defId), summoned("p1", "m1", defId)], view, false);
      const plain = planEntries([played("p1", "m1", "core-052"), summoned("p1", "m1", "core-052")], view, false);
      expect(marquee.map((entry) => entry.durationMs)).toEqual(plain.map((entry) => entry.durationMs));
    }
  });

  it("R200 every entrance cue starts inside its entry and ends within FX_MAX_TAIL_MS of its end, for short and long entries", () => {
    for (const D of [120, 250, 400, 732, 1400]) {
      for (const [defId] of MARQUEE) {
        const cues = planAll([summoned("p1", "m1", defId)], envOf(), D);
        expect(cues.length).toBeGreaterThan(3);
        for (const cue of cues) {
          const label = `${defId} ${cue.kind} at D=${String(D)}`;
          expect(cue.delayMs, label).toBeGreaterThanOrEqual(0);
          expect(cue.delayMs, label).toBeLessThanOrEqual(D);
          expect(endOf(cue), label).toBeLessThanOrEqual(D + FX_MAX_TAIL_MS);
          if (cue.kind === "shake") expect(cue.trauma).toBeLessThanOrEqual(1);
        }
      }
    }
  });

  it("R200 intensity 0 (the effects off, or reduced motion) plans none of it", () => {
    for (const [defId] of MARQUEE) expect(planAll([summoned("p1", "m1", defId)], envOf({ intensity: 0 }))).toEqual([]);
  });
});
