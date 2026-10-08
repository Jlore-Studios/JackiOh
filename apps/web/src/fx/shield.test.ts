// R1363 (docs/meditative-set.md M8, MN05): the shield the effects layer flashes up as Armor takes a
// hit (shield.ts): small over a hit Armor took half or more of, full over one it took whole.

import type { GameEvent } from "@jackioh/shared";
import { describe, expect, it } from "vitest";

import { ANIMATIONS, planEntries } from "../game/animations.ts";
import { testid } from "../game/contract.ts";
import { fullBoardView } from "../test/fixtures.ts";
import { FX_INTENSITY_SCALE, FX_MAX_PARTICLE_LIFE_MS, FX_MAX_TAIL_MS, FX_SHIELD_TAIL_MS } from "./constants.ts";
import { planFx } from "./cues.ts";
import { mountDomEffect } from "./dom.ts";
import { createFxMemory } from "./memory.ts";
import { shieldCues } from "./shield.ts";
import type { FxAnchor, FxCue, FxShieldCue } from "./types.ts";

/** fullBoardView's enemy lane-1 unit is u6, and its own lane-1 unit u1. */
const TARGET = "u6";
const AT: FxAnchor = { kind: "testid", testid: testid.card(TARGET) };

type DamageEvent = Extract<GameEvent, { type: "damage" }>;

const hit = (amount: number, absorbed?: number, combat = true): GameEvent => {
  const event: DamageEvent = { type: "damage", sourceId: "u1", targetId: TARGET, amount, combat };
  return absorbed === undefined ? event : { ...event, absorbed };
};
const whole = (absorbed: number, targetId = TARGET): GameEvent => ({ type: "damageAbsorbed", sourceId: "u1", targetId, absorbed, combat: true });

function plan(events: GameEvent[], intensity: number = FX_INTENSITY_SCALE.normal): FxCue[] {
  const view = { ...fullBoardView(), events };
  const entries = planEntries(events, view, false);
  const env = { intensity, card: () => undefined, memory: createFxMemory() };
  return entries.flatMap((entry) => {
    env.memory.remember(entry.events);
    return planFx(entry, view, env);
  });
}

const shields = (cues: readonly FxCue[]): FxShieldCue[] => cues.filter((cue): cue is FxShieldCue => cue.kind === "shield");

describe("R1363 the shield flash", () => {
  it("R1363 a hit Armor took half or more of glances a small shield up over its target as it lands", () => {
    const D = ANIMATIONS.damage.durationMs;
    expect(shields(plan([hit(2, 2)]))).toEqual([{ kind: "shield", size: "small", at: AT, delayMs: 0, durationMs: D + FX_SHIELD_TAIL_MS }]);
    expect(shields(plan([hit(1, 6)]))).toHaveLength(1);
  });

  it("R1363 under half, or with no Armor in it, a hit flashes no shield", () => {
    expect(shields(plan([hit(3, 2)]))).toEqual([]);
    expect(shields(plan([hit(3)]))).toEqual([]);
  });

  it("R1363 a hit Armor took whole blooms a full shield with a ring and sparks, through its own row", () => {
    expect(ANIMATIONS.damageAbsorbed.fx).toEqual({ recipe: "armor" });
    const D = ANIMATIONS.damageAbsorbed.durationMs;
    const cues = plan([whole(3)]);
    expect(shields(cues)).toEqual([{ kind: "shield", size: "full", at: AT, delayMs: 0, durationMs: D + FX_SHIELD_TAIL_MS }]);
    expect(cues).toContainEqual(expect.objectContaining({ kind: "ring", at: AT, delayMs: 0 }));
    expect(cues).toContainEqual(expect.objectContaining({ kind: "burst", preset: "spark", at: AT, delayMs: 0 }));
    // A hero's shield is drawn on its portrait.
    const hero = shields(plan([whole(2, "hero-p1")]));
    expect(hero.map((cue) => cue.at)).toEqual([{ kind: "testid", testid: testid.hero("you") }]);
  });

  it("R1363 R200 under Reduce motion (the layer at intensity 0) nothing is drawn, the shield included", () => {
    expect(plan([whole(3)], 0)).toEqual([]);
    expect(plan([hit(2, 2)], 0)).toEqual([]);
  });

  it("R1363 R200 the shield starts inside its entry and is gone within FX_MAX_TAIL_MS of its end", () => {
    for (const D of [120, 300, 1600]) {
      for (const size of ["small", "full"] as const) {
        for (const delay of [0, D / 2, D * 2]) {
          for (const cue of shieldCues(size, AT, D, delay, 1)) {
            expect(cue.delayMs).toBeLessThanOrEqual(D);
            const end = cue.kind === "burst" ? cue.delayMs + FX_MAX_PARTICLE_LIFE_MS : cue.delayMs + ("durationMs" in cue ? cue.durationMs : 0);
            expect(end).toBeLessThanOrEqual(D + FX_MAX_TAIL_MS);
          }
        }
      }
    }
  });

  it("R1363 R1361 a board-wide hit the fog sweeps over blooms a shield where a unit's Armor took its hit whole", () => {
    // fullBoardView's enemy row is u6 to u10; the sweep's source is a spell no board shows.
    const sweep = (target: string, amount: number): DamageEvent => ({ type: "damage", sourceId: "s9", targetId: target, amount, combat: false });
    const events: GameEvent[] = [
      sweep("u6", 2),
      sweep("u7", 2),
      sweep("u8", 2),
      { type: "damageAbsorbed", sourceId: "s9", targetId: "u9", absorbed: 2, combat: false },
      { ...sweep("u10", 1), absorbed: 1 },
    ];
    const cues = plan(events);
    expect(cues.some((cue) => cue.kind === "fog")).toBe(true);
    const at = (id: string): FxAnchor => ({ kind: "testid", testid: testid.card(id) });
    expect(shields(cues).map((cue) => [cue.size, cue.at])).toEqual([
      ["full", at("u9")],
      ["small", at("u10")],
    ]);
  });

  it("R1363 R202 a target the board does not render plans no shield", () => {
    expect(shields(plan([whole(3, "gone")]))).toEqual([]);
  });

  it("R1363 the DOM flourish is centred on its target and says its size, as data only", () => {
    const root = document.createElement("div");
    const box = { x: 10, y: 20, width: 80, height: 100 };
    const effect = mountDomEffect(root, { kind: "shield", size: "full", at: AT, delayMs: 0, durationMs: 750 }, { at: box });
    expect(effect?.el.className).toBe("fx-shield");
    expect(effect?.el.getAttribute("data-fx")).toBe("shield");
    expect(effect?.el.getAttribute("data-size")).toBe("full");
    expect(effect?.el.getAttribute("aria-hidden")).toBe("true");
    expect(effect?.el.style.getPropertyValue("--fx-x")).toBe("50px");
    expect(effect?.el.style.getPropertyValue("--fx-y")).toBe("70px");
    expect(effect?.el.style.getPropertyValue("--fx-ms")).toBe("750ms");
    expect(effect?.el.childNodes).toHaveLength(0);
    expect(mountDomEffect(root, { kind: "shield", size: "small", at: AT, delayMs: 0, durationMs: 400 }, {})).toBeNull();
  });
});
