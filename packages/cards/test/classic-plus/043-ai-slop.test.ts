// C+ #43 AI Slop — SPEC §8.7 row 43, R11, R60, R77, R97, R102, R179, R386, R469, R582, BUILD M9 row
// C+ 43. The fusion is the engine's `fuseGenerated` (packages/engine/test/fuse-variants.test.ts).

import { defOf, playedThisGameWithTag, stepParam, subsystems } from "@jackioh/engine";
import { describe, expect, it } from "vitest";
import { cardDef } from "../../src/index";
import { base, def, radiant } from "../../src/scripts/classic-plus/043-ai-slop";
import { scenario, type Scenario } from "../_harness";

const SLOP = "classicplus-043";
const FILLER = "core-005";
const AI_IDS = Array.from({ length: 10 }, (_, n) => `classicplus-t-ai-${String(n + 1).padStart(2, "0")}`);

function slop(seed: string, opts: { radiant?: boolean; hand?: readonly string[] } = {}): Scenario {
  return scenario({ seed, p1: { hand: [{ def: SLOP, radiant: opts.radiant === true }, ...(opts.hand ?? [FILLER])] }, p2: { hand: [FILLER] } });
}

/** The card AI Slop made: p1's hand card that is not a filler. */
function made(s: Scenario) {
  const card = s.hand("p1").find((held) => held.defId !== FILLER);
  if (card === undefined) throw new Error("the fused card in hand");
  return card;
}

function specs(s: Scenario): { defId: string; radiant?: true }[] {
  return subsystems.fusedIngredientSpecs(made(s).defId) ?? [];
}

describe("C+ #43 AI Slop", () => {
  it("runs one script on both faces; its card count is the declared `cards`", () => {
    expect(def.id).toBe(SLOP);
    expect(radiant).toBe(base);
    expect(def.params?.map((entry) => [entry.key, entry.base, entry.radiant])).toEqual([["cards", 3, 3]]);
  });

  describe("base", () => {
    it("R77 R179 fuses three random AI generated cards into your hand at (0): its id names the three", () => {
      const s = slop("slop-base").play(SLOP);
      const card = made(s);
      expect(specs(s)).toHaveLength(3);
      for (const spec of specs(s)) {
        expect(AI_IDS).toContain(spec.defId);
        expect(spec.radiant).toBeUndefined();
      }
      expect(card.costOverride).toBe(0);
      expect(card.radiant).toBe(false);
    });

    it("R102 a Token with the AI tag, of the shared type else the first's, its texts joined", () => {
      const shapes = new Set<string>();
      for (let n = 0; n < 12; n += 1) {
        const s = slop(`slop-shape-${n}`).play(SLOP);
        const fused = defOf(s.state, made(s).defId);
        const parts = specs(s).map((spec) => cardDef(spec.defId));
        expect(fused.token).toBe(true);
        expect(fused.tags).toContain("AI");
        const types = new Set(parts.map((part) => part.type));
        shapes.add(types.size === 1 ? "shared" : "mixed");
        // No AI generated card is a Field Trap, so R102's promotion never applies.
        expect(fused.type).toBe(types.size === 1 ? [...types][0] : parts[0]?.type);
        for (const part of parts) expect(fused.base.text).toContain(part.base.text);
      }
      expect(shapes.has("mixed")).toBe(true);
    });

    it("R60 picks are independent: a card may come up twice", () => {
      let repeated = false;
      for (let n = 0; n < 40 && !repeated; n += 1) {
        const ids = specs(slop(`slop-repeat-${n}`).play(SLOP)).map((spec) => spec.defId);
        repeated = new Set(ids).size < ids.length;
      }
      expect(repeated).toBe(true);
    });

    it("R97 R179 the opponent's view names neither the card nor its ingredients", () => {
      const s = slop("slop-hidden").play(SLOP);
      const seen = JSON.stringify(s.view("p2"));
      for (const id of AI_IDS) expect(seen).not.toContain(id);
      expect(seen).not.toContain(made(s).defId);
      expect(JSON.stringify(s.view("p1"))).toContain(made(s).defId);
    });

    it("§2.4 R11 a full hand burns it: a fused Unit token ceases to exist, anything else is in the graveyard", () => {
      const seen = new Set<string>();
      for (let n = 0; n < 40 && seen.size < 2; n += 1) {
        const s = slop(`slop-burn-${n}`, { hand: Array.from({ length: 10 }, () => FILLER) }).play(SLOP);
        const burned = s.events.find((event) => event.type === "burned");
        expect(burned).toBeDefined();
        expect(s.hand("p1")).toHaveLength(10);
        const id = burned?.type === "burned" ? burned.instanceId : "";
        const fused = defOf(s.state, burned?.type === "burned" ? burned.defId : "");
        seen.add(fused.type === "Unit" ? "Unit" : "other");
        if (fused.type === "Unit") s.expectInZone(id, "gone");
        else s.expectInZone(id, "graveyard");
      }
      expect([...seen].sort()).toEqual(["Unit", "other"]);
    });

    it("played, it counts once toward the AI generated cards played this game (Scaling Law's count)", () => {
      for (let n = 0; n < 20; n += 1) {
        const s = slop(`slop-play-${n}`).play(SLOP);
        const fused = defOf(s.state, made(s).defId);
        if (fused.type !== "Spell" || (subsystems.fusedIngredients(fused.id) ?? []).length !== 3) continue;
        expect(playedThisGameWithTag(s.state, "p1", "AI")).toBe(0);
        s.play(made(s));
        expect(playedThisGameWithTag(s.state, "p1", "AI")).toBe(1);
        return;
      }
      throw new Error("no seed fused three Spells");
    });

    it("R386 its card count reads through param(): an Upgrade fuses 4, a Degrade 2", () => {
      const up = slop("slop-up");
      stepParam(up.card(SLOP), "cards", 1);
      expect(specs(up.play(SLOP))).toHaveLength(4);
      const down = slop("slop-down");
      stepParam(down.card(SLOP), "cards", -1);
      expect(specs(down.play(SLOP))).toHaveLength(2);
    });

    it("R582 tuned to one card, that card is added as it is, unfused, at (0)", () => {
      const s = slop("slop-one");
      stepParam(s.card(SLOP), "cards", -1);
      stepParam(s.card(SLOP), "cards", -1);
      s.play(SLOP);
      const card = made(s);
      expect(AI_IDS).toContain(card.defId);
      expect(card.costOverride).toBe(0);
    });
  });

  describe("radiant", () => {
    it("R469 the three go in on their Radiant faces, and the result is Radiant", () => {
      const s = slop("slop-radiant", { radiant: true }).play(SLOP);
      expect(specs(s)).toHaveLength(3);
      for (const spec of specs(s)) expect(spec.radiant).toBe(true);
      const fused = defOf(s.state, made(s).defId);
      const parts = specs(s).map((spec) => cardDef(spec.defId));
      for (const part of parts) expect(fused.base.text).toContain(part.radiant.text);
      expect(made(s).costOverride).toBe(0);
      expect(made(s).radiant).toBe(true);
    });

    it("R582 tuned to one card, the card is Radiant", () => {
      const s = slop("slop-radiant-one", { radiant: true });
      stepParam(s.card(SLOP), "cards", -1);
      stepParam(s.card(SLOP), "cards", -1);
      s.play(SLOP);
      expect(made(s).radiant).toBe(true);
    });
  });
});
