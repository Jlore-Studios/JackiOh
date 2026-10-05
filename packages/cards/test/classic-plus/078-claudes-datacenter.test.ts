// C+ #78 Claude's Datacenter — SPEC §8.7 row 78, BUILD M9 Classic+ row C+ 78: "Field Spell: at each end
// of your turn adds a random AI generated card (T-AI-1 to T-AI-10, the pool its text names) that costs
// (0) to your hand; nothing at the opponent's end; a full hand burns it; hidden from the opponent (R97);
// the count is fixed at 1 with no tunable (balance patch 1); radiant the card is Radiant".

import { stepParam } from "@jackioh/engine";
import type { CardView } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { cardDef } from "../../src/catalog-data";
import { query } from "../../src/query";
import { scenario, type Scenario } from "../_harness";
import { expectAnimated } from "../_animated";
import { base, def, radiant } from "../../src/scripts/classic-plus/078-claudes-datacenter";

const DATACENTER = "classicplus-078";
const VANILLA = "core-008"; // (1) Unit
const TIMMY = "core-011"; // (1) Unit
const AI_IDS = Array.from({ length: 10 }, (_, at) => `classicplus-t-ai-${String(at + 1).padStart(2, "0")}`);

function setup(opts: { radiantFace?: boolean; hand?: number; seed?: string } = {}): Scenario {
  return scenario({
    ...(opts.seed === undefined ? {} : { seed: opts.seed }),
    p1: {
      hand: Array.from({ length: opts.hand ?? 1 }, () => VANILLA),
      backrow: [{ def: DATACENTER, radiant: opts.radiantFace === true, faceUp: true }],
      library: [TIMMY, TIMMY, TIMMY],
    },
    p2: { hand: [VANILLA], library: [TIMMY, TIMMY, TIMMY] },
  });
}

function aiCardsIn(s: Scenario): ReturnType<Scenario["hand"]> {
  return s.hand("p1").filter((card) => AI_IDS.includes(card.defId));
}

describe("C+ #78 Claude's Datacenter", () => {
  it("is a (2) Legendary Field Spell with no tunable count", () => {
    expect(def.type).toBe("Field Spell");
    expect(def.cost).toBe(2);
    expect(def.rarity).toBe("Legendary");
    expect(def.params).toBeUndefined();
    expect(base.endOfTurn).toBeTypeOf("function");
    expect(radiant.endOfTurn).toBeTypeOf("function");
  });

  it("§7 the pool its text names is the ten AI generated cards, tokens all", () => {
    const pool = query({ tags: ["AI"], token: true }).map((card) => card.id);
    expect([...pool].sort()).toEqual(AI_IDS);
    for (const id of AI_IDS) expect(cardDef(id).token).toBe(true);
  });

  describe("base", () => {
    it("R62 at the end of your turn adds a random AI generated card that costs (0)", () => {
      const s = setup().endTurn();
      const added = aiCardsIn(s);
      expect(added).toHaveLength(1);
      expect(added[0]?.costOverride).toBe(0);
      expect(added[0]?.radiant).toBe(false);
      expect(added[0]?.owner).toBe("p1");
      const shown = (s.view("p1").you.hand as CardView[]).find((card) => card.instanceId === added[0]?.id);
      expect(shown?.cost).toBe(0);
    });

    it("R60 every seed draws from the ten AI cards and nothing else, and more than one of them turns up", () => {
      const seen = new Set<string>();
      for (let seed = 0; seed < 24; seed += 1) {
        const s = setup({ seed: `datacenter-${seed}` }).endTurn();
        const added = aiCardsIn(s);
        expect(added).toHaveLength(1);
        seen.add(added[0]?.defId ?? "");
      }
      expect(seen.size).toBeGreaterThan(3);
    });

    it("§6.2 nothing at the opponent's end of turn", () => {
      const s = setup().endTurn();
      expect(aiCardsIn(s)).toHaveLength(1);
      s.endTurn();
      expect(s.state.active).toBe("p1");
      // p2's end of turn added nothing; p1 drew one Timmy at the start of its turn.
      expect(aiCardsIn(s)).toHaveLength(1);
      expect(s.hand("p2").some((card) => AI_IDS.includes(card.defId))).toBe(false);
    });

    it("adds again at each end of your turn", () => {
      const s = setup().endTurn().endTurn().endTurn();
      expect(aiCardsIn(s)).toHaveLength(2);
    });

    it("§2.4 a full hand burns it", () => {
      const s = setup({ hand: 10 }).endTurn();
      expect(aiCardsIn(s)).toHaveLength(0);
      const burned = s.events.filter((event) => event.type === "burned");
      expect(burned).toHaveLength(1);
      expect(AI_IDS).toContain(burned[0]?.type === "burned" ? burned[0].defId : "");
    });

    it("R97 hidden from the opponent: their view never names the card", () => {
      const s = setup().endTurn();
      const added = aiCardsIn(s)[0];
      const theirs = JSON.stringify(s.view("p2"));
      expect(theirs).not.toContain(added?.id ?? "?");
      expect(theirs).not.toContain(added?.defId ?? "?");
    });

    it("R386 the count is fixed at 1: an Upgrade still adds exactly one card", () => {
      const s = setup();
      stepParam(s.card(DATACENTER), "cards", 1);
      s.endTurn();
      expect(aiCardsIn(s)).toHaveLength(1);
      expect(aiCardsIn(s).every((card) => card.costOverride === 0)).toBe(true);
    });
  });

  describe("radiant", () => {
    it("R62 the card it adds is Radiant and costs (0)", () => {
      const s = setup({ radiantFace: true }).endTurn();
      const added = aiCardsIn(s);
      expect(added).toHaveLength(1);
      expect(added[0]?.radiant).toBe(true);
      expect(added[0]?.costOverride).toBe(0);
    });

    it("R97 hidden from the opponent", () => {
      const s = setup({ radiantFace: true }).endTurn();
      expect(JSON.stringify(s.view("p2"))).not.toContain(aiCardsIn(s)[0]?.id ?? "?");
    });
  });
});

describe("C+ #78 Claude's Datacenter: Animated (patch v0.2.10)", () => {
  it("R383 played, it animates into its lane's unit zone, else the leftmost open one, a 0/5 Unit; with none open it stays a Field Spell", () => {
    expectAnimated({ def: "classicplus-078", stats: { attack: 0, health: 5 } });
  });

  it("R383 radiant: a 0/10 Unit", () => {
    expectAnimated({ def: "classicplus-078", radiant: true, stats: { attack: 0, health: 10 } });
  });

  it("R383 played, it keeps its text as a Unit: at the end of your turn it adds an AI generated card that costs (0)", () => {
    const s = scenario({
      p1: { hand: [DATACENTER, VANILLA], library: [TIMMY, TIMMY, TIMMY] },
      p2: { hand: [VANILLA], library: [TIMMY, TIMMY, TIMMY] },
    });

    s.play(DATACENTER, { zone: 3 });
    expect(s.unit("p1", 3)?.defId).toBe(DATACENTER);
    s.endTurn();

    const added = aiCardsIn(s);
    expect(added).toHaveLength(1);
    expect(added[0]?.costOverride).toBe(0);
  });
});
