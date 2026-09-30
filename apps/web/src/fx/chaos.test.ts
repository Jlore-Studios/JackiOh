// R436: Call to Chaos names the effects it rolled, to both players: the words table and its one
// adapter (chaos.ts), and the slot-machine reveal the effects layer plans for a `chaosRolled` entry.

import type { GameEvent, PlayerView } from "@jackioh/shared";
import { describe, expect, it } from "vitest";

import { ANIMATIONS, planEntries } from "../game/animations.ts";
import { baseView, fullBoardView } from "../test/fixtures.ts";
import {
  CHAOS_CLASSIC_PLUS,
  CHAOS_CORE,
  CHAOS_EFFECT_NAMES,
  CHAOS_GENERIC_NAMES,
  chaosCues,
  chaosEffectName,
  chaosLandAt,
  chaosNames,
  chaosReel,
  chaosRollOf,
} from "./chaos.ts";
import { FX_BANNER_TAIL_MS, FX_CHAOS_LAND_LAST, FX_CHAOS_REEL_DECOYS, FX_INTENSITY_SCALE, FX_MAX_PARTICLE_LIFE_MS, FX_MAX_TAIL_MS, FX_TEXT } from "./constants.ts";
import { planFx } from "./cues.ts";
import { createFxMemory } from "./memory.ts";
import type { FxChaosCue, FxCue } from "./types.ts";

/** The Core Edition's keys, as the engine's `CHAOS_EFFECTS` names them (subsystems/callToChaos.ts). */
const CORE_KEYS = ["units", "heal", "draw", "add", "radiant", "tokens", "discount", "golem", "backrow", "recast"];
const PLUS_KEYS = ["fruits", "books", "destroy", "classics", "upgrade", "fuse", "degrade", "golem", "replace", "recast"];

function rolled(effects: string[], defId = CHAOS_CORE, player: "p1" | "p2" = "p2"): GameEvent {
  return { type: "chaosRolled", player, instanceId: "c95", defId, effects };
}

function panelOf(cues: readonly FxCue[]): FxChaosCue {
  const panel = cues.find((cue): cue is FxChaosCue => cue.kind === "chaos");
  if (panel === undefined) throw new Error("no chaos panel planned");
  return panel;
}

function planRoll(event: GameEvent, view: PlayerView, D = ANIMATIONS.chaosRolled.durationMs): FxCue[] {
  const [entry] = planEntries([event], { ...view, events: [event] }, false);
  if (entry === undefined) throw new Error("no entry");
  const env = { intensity: FX_INTENSITY_SCALE.normal, card: () => undefined, memory: createFxMemory() };
  env.memory.remember(entry.events);
  return planFx({ ...entry, durationMs: D }, view, env);
}

describe("R436 the words for each rolled effect", () => {
  it("R436 every Core key the engine rolls, and every Classic+ key, has its own short words", () => {
    expect(Object.keys(CHAOS_EFFECT_NAMES[CHAOS_CORE] ?? {})).toEqual(CORE_KEYS);
    expect(Object.keys(CHAOS_EFFECT_NAMES[CHAOS_CLASSIC_PLUS] ?? {})).toEqual(PLUS_KEYS);
    for (const table of Object.values(CHAOS_EFFECT_NAMES)) {
      for (const name of Object.values(table)) {
        expect(name.length, name).toBeLessThanOrEqual(48);
        // R373, R432: "Deck" and "Tribute", "(N) Cost" as the noun ("Cost (N)" is the old noun).
        expect(name, name).not.toMatch(/\blibrar|\bsacrific|costs? \d|\d-cost/i);
        expect(name, name).not.toMatch(/\bCost \(\d/);
      }
    }
    expect(chaosEffectName(CHAOS_CORE, "units")).toBe("Summon 3 random (3) Cost Units");
    expect(chaosEffectName(CHAOS_CORE, "discount")).toBe("Hand and Deck cost (2) less");
  });

  it("R436 a key both editions share reads per edition, and neutrally when the edition is hidden", () => {
    expect(chaosEffectName(CHAOS_CORE, "golem")).toBe("Summon a Chaos Golem");
    expect(chaosEffectName(CHAOS_CLASSIC_PLUS, "golem")).toBe("Summon a Classic Golem");
    expect(chaosEffectName("hidden", "golem")).toBe(CHAOS_GENERIC_NAMES.golem);
    // Spelled the same in both, so it reads the same whoever cast it.
    expect(chaosEffectName("hidden", "recast")).toBe("Cast a random Call to Chaos");
    expect(chaosEffectName("hidden", "fruits")).toBe("Add 5 Fruits costing (0)");
  });

  it("R436 an unknown key still reads: words stay words, one word is capitalised", () => {
    expect(chaosEffectName(CHAOS_CORE, "Destroy all enemy permanents")).toBe("Destroy all enemy permanents");
    expect(chaosEffectName(CHAOS_CORE, "meteors")).toBe("Meteors");
    expect(chaosEffectName(CHAOS_CORE, " ")).toBe("Unknown effect");
  });

  it("R436 the one adapter reads the event, and nothing else", () => {
    expect(chaosRollOf(rolled(["heal", "units", "golem"]))).toEqual({ player: "p2", instanceId: "c95", defId: CHAOS_CORE, effects: ["heal", "units", "golem"] });
    expect(chaosRollOf({ type: "turnStarted", player: "p1", turn: 2 })).toBeNull();
    expect(chaosNames({ player: "p1", instanceId: "c1", defId: CHAOS_CORE, effects: ["heal", "recast"] })).toEqual([
      "Heal the caster's hero 30",
      "Cast a random Call to Chaos",
    ]);
  });

  it("R436 a reel spins through distinct other effects of the edition and lands on the one rolled", () => {
    for (const [defId, keys] of [
      [CHAOS_CORE, CORE_KEYS],
      [CHAOS_CLASSIC_PLUS, PLUS_KEYS],
    ] as const) {
      for (const key of keys) {
        const reel = chaosReel(defId, key, 1);
        const landed = chaosEffectName(defId, key);
        expect(reel).toHaveLength(FX_CHAOS_REEL_DECOYS + 1);
        expect(reel[reel.length - 1]).toBe(landed);
        const decoys = reel.slice(0, -1);
        expect(decoys).not.toContain(landed);
        expect(new Set(decoys).size, `${defId} ${key}`).toBe(decoys.length);
      }
    }
    expect(chaosReel(CHAOS_CORE, "heal", 0)).toEqual(chaosReel(CHAOS_CORE, "heal", 0));
  });
});

describe("R436 the reveal the effects layer plans", () => {
  it("R436 one line per effect, in the order they resolve, stacked for three, under the Call to Chaos title", () => {
    const panel = panelOf(planRoll(rolled(["units", "heal", "golem"]), fullBoardView()));
    expect(panel.title).toBe(FX_TEXT.chaosRolled);
    expect(panel.lines.map((line) => line.text)).toEqual(["Summon 3 random (3) Cost Units", "Heal the caster's hero 30", "Summon a Chaos Golem"]);
    const lands = panel.lines.map((line) => line.landMs);
    expect([...lands].sort((a, b) => a - b)).toEqual(lands);
    expect(new Set(lands).size).toBe(3);
  });

  it("R436 both seats plan the same reveal: the roll is public, whoever cast it", () => {
    const mine = planRoll(rolled(["destroy"], CHAOS_CLASSIC_PLUS, "p1"), fullBoardView());
    const asP2 = baseView({ viewer: "p2", you: fullBoardView().opponent, opponent: fullBoardView().you });
    const theirs = planRoll(rolled(["destroy"], CHAOS_CLASSIC_PLUS, "p1"), asP2);
    expect(panelOf(theirs)).toEqual(panelOf(mine));
  });

  it("R200 every line lands inside the entry, the last by FX_CHAOS_LAND_LAST, and the panel is gone within D + FX_BANNER_TAIL_MS", () => {
    for (const D of [120, 300, 900, 1400, 3600]) {
      for (const n of [1, 2, 3, 5]) {
        const cues = chaosCues(rolled(CORE_KEYS.slice(0, n)), D, 1);
        const panel = panelOf(cues);
        expect(panel.delayMs).toBe(0);
        expect(panel.durationMs).toBe(D + FX_BANNER_TAIL_MS);
        expect(FX_BANNER_TAIL_MS).toBeLessThanOrEqual(FX_MAX_TAIL_MS);
        for (const line of panel.lines) {
          expect(line.landMs).toBeGreaterThanOrEqual(0);
          expect(line.landMs).toBeLessThanOrEqual(Math.round(FX_CHAOS_LAND_LAST * D));
        }
        for (const cue of cues) {
          if (cue.kind === "burst") expect(cue.delayMs + FX_MAX_PARTICLE_LIFE_MS).toBeLessThanOrEqual(D + FX_MAX_TAIL_MS);
        }
      }
    }
    expect(chaosLandAt(0, 1)).toBeLessThan(chaosLandAt(1, 2));
  });

  it("R436 an empty roll or another event plans nothing, and intensity 0 plans nothing at all", () => {
    expect(chaosCues(rolled([]), 900, 1)).toEqual([]);
    expect(chaosCues({ type: "turnStarted", player: "p1", turn: 1 }, 900, 1)).toEqual([]);
    const [entry] = planEntries([rolled(["heal"])], fullBoardView(), false);
    if (entry === undefined) throw new Error("no entry");
    expect(planFx(entry, fullBoardView(), { intensity: 0, card: () => undefined, memory: createFxMemory() })).toEqual([]);
  });

  it("R436 the ANIMATIONS row names the chaos recipe", () => {
    expect(ANIMATIONS.chaosRolled.fx).toEqual({ recipe: "chaos" });
  });
});
