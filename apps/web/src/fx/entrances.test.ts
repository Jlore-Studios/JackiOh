// R670: the bespoke entrances of the marquee Legendary and Mythic Units (entrances.ts), planned the
// way FxLayer plans them. A card whose entry is a set piece lands with its set piece's entrance (R2800000).
// R3389001: a Field Spell's own entrance (Classic's signatures') lands face up in the backrow, which
// fx/cards/classic/signatures.test.ts plays; the Units' are these.

import catalogJson from "@jackioh/cards/catalog.json";
import type { GameEvent, PlayerView } from "@jackioh/shared";
import { describe, expect, it } from "vitest";

import { ANIMATIONS, planEntries } from "../game/animations.ts";
import { testid } from "../game/contract.ts";
import { fullBoardView, withEvents } from "../test/fixtures.ts";
import { entranceKeyOf } from "./cardFx.ts";
import { FX_CARD_TABLE } from "./cards/tables.ts";
import { FX_INTENSITY_SCALE, FX_MAX_PARTICLE_LIFE_MS, FX_MAX_TAIL_MS, FX_SLAM_AT } from "./constants.ts";
import { planFx } from "./cues.ts";
import { ENTRANCES, ENTRANCE_TRAUMA, type EntranceKey } from "./entrances.ts";
import { createFxMemory } from "./memory.ts";
import type { FxCardFacts, FxCue, FxPlanEnv } from "./types.ts";

type CatalogEntry = { name: string; type: string; rarity: string; printedRarity?: string };
const CATALOG = catalogJson as unknown as Record<string, CatalogEntry>;

/** Every card with an entrance of its own, a Unit's or a Field Spell's. */
const ENTERING: readonly (readonly [string, EntranceKey])[] = [...FX_CARD_TABLE.keys()].flatMap((defId) => {
  const key = entranceKeyOf(defId);
  return key === undefined ? [] : [[defId, key] as const];
});

/** The Units among them, which land in a unit zone. */
const MARQUEE = ENTERING.filter(([defId]) => CATALOG[defId]?.type === "Unit");

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

/** R2791000: a plain summon needs an empty zone (§3.2); the fixture lane stands occupied. */
function emptyLane(side: "you" | "opponent", row: "units" | "backrow", lane: number): PlayerView {
  const view = fullBoardView();
  if (row === "units") view[side].units[lane - 1] = null;
  else view[side].backrow[lane - 1] = null;
  return view;
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
      // A token by the rarity it prints (B2.5): R3400001's Training-Grounds Golem.
      expect(["Legendary", "Mythic"], defId).toContain(def?.printedRarity ?? def?.rarity);
      expect(typeof ENTRANCES[key]).toBe("function");
    }
    // Each entrance is its own: no two cards share one. R3389001: every other is a Legendary or Mythic Field Spell's.
    expect(new Set(ENTERING.map(([, key]) => key)).size).toBe(ENTERING.length);
    for (const [defId] of ENTERING.filter(([id]) => CATALOG[id]?.type !== "Unit")) {
      expect(CATALOG[defId]?.type, defId).toBe("Field Spell");
      expect(["Legendary", "Mythic"], defId).toContain(CATALOG[defId]?.rarity);
    }
  });

  it("R670 a marquee summoned into a unit zone plans its own entrance in place of the rarity's rays and gold", () => {
    for (const [defId, key] of MARQUEE) {
      const landing = summoned("p1", "m1", defId);
      const cues = planAll([landing], envOf(), undefined, emptyLane("you", "units", 3));
      const D = ANIMATIONS.summoned.durationMs;
      // R3332001, R3400001: an entrance that reads the board and its landing (its side, its lane) is given both;
      // R3389000: and when in the entry its card lands.
      const view = withEvents(emptyLane("you", "units", 3), [landing]);
      const expected = ENTRANCES[key]({
        D,
        tgt: testid.zone("you", "units", 3),
        intensity: FX_INTENSITY_SCALE.normal,
        view,
        landing: landing.type === "summoned" ? landing : undefined,
        landMs: Math.round(FX_SLAM_AT * D),
      });
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
    const landMs = Math.round(FX_SLAM_AT * D);
    const mineView = emptyLane("you", "units", 3);
    const mine = planAll(pair, envOf(), undefined, mineView);
    const own = pair[1]?.type === "summoned" ? pair[1] : undefined;
    expect(mine).toEqual(ENTRANCES[key]({ D, tgt: testid.zone("you", "units", 3), intensity: FX_INTENSITY_SCALE.normal, view: mineView, landing: own, landMs }));
    const theirLanding = summoned("p2", "m2", defId, "units", 2);
    const theirView = emptyLane("opponent", "units", 2);
    const theirs = planAll([played("p2", "m2", defId), theirLanding], envOf(), undefined, theirView);
    const other = theirLanding.type === "summoned" ? theirLanding : undefined;
    expect(theirs).toEqual(ENTRANCES[key]({ D, tgt: testid.zone("opponent", "units", 2), intensity: FX_INTENSITY_SCALE.normal, view: theirView, landing: other, landMs }));
    // R202: no cue carries a definition or a name.
    const text = JSON.stringify(theirs);
    expect(text).not.toContain(defId);
    expect(text).not.toContain(CATALOG[defId]?.name ?? "\u0000");
  });

  it("R670 R202 a hidden summon and a backrow set get no entrance", () => {
    const hidden = planAll([summoned("p2", "hidden", "hidden")]);
    expect(hidden.some((cue) => cue.kind === "rays" || cue.kind === "crack" || cue.kind === "shake")).toBe(false);
    for (const [defId] of MARQUEE) {
      const set = planAll([summoned("p1", "m1", defId, "backrow")], envOf(), undefined, emptyLane("you", "backrow", 3));
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
