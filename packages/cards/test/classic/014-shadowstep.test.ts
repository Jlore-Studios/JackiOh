// C #14 Shadowstep — SPEC §8.6 row 14, BUILD M9 Classic row C 14: "Face-down (R33); fires once for
// all of your Units one state-check pass collects; each card still in a graveyard afterwards returns
// to its owner's hand (a stolen one to the opponent's, §3.2) and costs (0) (`costOverride`, kept,
// R78), a full hand burning the overflow (R317); unit tokens have ceased to exist (R11), a Reborn Unit
// already back is skipped, and an Indestructible Unit is collected only when its max health falls to
// 0 (R69); enemy deaths don't fire it; radiant: fires in the "would die" window at §4.5 step 1: those
// Units leave the collection and flicker (the same zone, reset per R78, full health, summoning sick,
// no Cry, no Death), and a fresh copy of each (its Radiant flag kept, R57) goes to your hand and costs
// (0); the copies are never named in the opponent's view (R97); its tuned number (cost) reads through
// `param()` (R386)".
//
// The stolen-unit case needs the `destroyed` event to say who controlled the unit as it died (it names
// the owner only): it waits for that engine change.

import { describe, expect, it } from "vitest";
import { effectiveCost, stepParam, type CardInstance } from "@jackioh/engine";
import { scenario, type Scenario } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic/014-shadowstep";

const SHADOWSTEP = "classic-014";
const BIG_FELINOR = "core-043"; // (4) Unit 3/10: "Cry: Destroy all non-Felinor Units."
const MIND_CONTROL = "core-049"; // (4) Spell: "Steal target enemy permanent."
const VANILLA = "core-008"; // (1) Unit 4/4
const MENACE = "core-019"; // (3) Unit 9/9
const TIMMY = "core-011"; // (1) Unit 3/3
const DEFENDER = "core-003"; // (1) Unit 1/1 Taunt, Divine Shield, Reborn
const STATE_OF_GAME = "classic-041"; // (1) Unit 3/3 Indestructible
const SAINTESS = "core-081"; // (1) Unit 2/2: "Death: Make your other Units Radiant."
const MR_TOKEN = "core-015"; // (1) Unit 1/1: "Cry: Summon a Rush Token."
const RUSH_TOKEN = "core-t-rush"; // (1) Unit token 3/3
const FELINORS = "core-012"; // (2) Unit 3/4, a Felinor Big Felinor spares
const STOCKPILE = "core-005"; // (1) Spell, a spare card (§2.5)
const SUPPRESSIVE_AURA = "core-046"; // (2) Field Spell: "Aura: All Units have −1/−1."

/** p1 sets the trap; p2, active, plays Big Felinor and destroys every non-Felinor Unit in one pass. */
function wipe(
  radiantFace: boolean,
  mine: readonly (string | { def: string; radiant?: boolean })[],
  opts: { myHand?: readonly string[]; theirs?: readonly string[] } = {},
): Scenario {
  return scenario({
    active: "p2",
    p1: {
      hand: [...(opts.myHand ?? [STOCKPILE])],
      field: [...mine],
      backrow: [{ def: SHADOWSTEP, faceUp: false, radiant: radiantFace }],
      library: [STOCKPILE],
    },
    p2: { hand: [BIG_FELINOR, STOCKPILE], field: [...(opts.theirs ?? [])], library: [STOCKPILE] },
  });
}

function fired(s: Scenario): number {
  return s.events.filter((event) => event.type === "trapFired").length;
}

function mineOnField(s: Scenario): (string | null)[] {
  return [1, 2, 3, 4, 5].map((lane) => s.unit("p1", lane)?.defId ?? null);
}

describe("C #14 Shadowstep", () => {
  it("declares its one number, the cost the cards come back at (R386): 0, never below 0", () => {
    expect(def.params).toEqual([{ key: "setCost", base: 0, radiant: 0, better: "down", step: 1, min: 0 }]);
    expect(base.triggers?.map((trigger) => trigger.on)).toEqual([["destroyed"]]);
    expect(radiant.replacements?.map((entry) => entry.on)).toEqual(["wouldDie"]);
  });

  describe("base", () => {
    it("R33 it sits face-down", () => {
      const s = wipe(false, [VANILLA]);
      expect(s.view("p2").opponent.backrow[0]).toEqual({ faceDown: true, cost: 2 });
    });

    it("fires once for all of your Units one state-check pass collects, and returns each to your hand", () => {
      const s = wipe(false, [VANILLA, MENACE, TIMMY]);
      const cards = [s.card(VANILLA), s.card(MENACE), s.card(TIMMY)];
      s.play(BIG_FELINOR);
      expect(fired(s)).toBe(1);
      for (const card of cards) s.expectInZone(card, "hand");
      expect(s.hand("p1").map((card) => card.defId)).toEqual([STOCKPILE, VANILLA, MENACE, TIMMY]);
      s.expectInZone(SHADOWSTEP, "graveyard");
    });

    it("R78 they cost (0) (costOverride), kept in every zone", () => {
      const s = wipe(false, [MENACE]);
      const menace = s.card(MENACE);
      s.play(BIG_FELINOR);
      expect(s.card(menace).costOverride).toBe(0);
      expect(effectiveCost(s.state, s.card(menace))).toBe(0);
    });

    it("enemy deaths don't fire it", () => {
      const s = wipe(false, [FELINORS], { theirs: [VANILLA, MENACE] });
      s.play(BIG_FELINOR);
      expect(fired(s)).toBe(0);
      expect(s.backrow("p1", 1)?.defId).toBe(SHADOWSTEP);
      s.expectInZone(FELINORS, "field");
    });

    it("the enemy's Units that die in the same pass stay in their graveyard", () => {
      const s = wipe(false, [VANILLA], { theirs: [MENACE] });
      const theirs = s.card(MENACE);
      s.play(BIG_FELINOR);
      s.expectInZone(theirs, "graveyard");
      s.expectInZone(VANILLA, "hand");
    });

    it("R11 a unit token has ceased to exist: nothing of it comes back", () => {
      const s = wipe(false, [RUSH_TOKEN, VANILLA]);
      const token = s.card(RUSH_TOKEN);
      s.play(BIG_FELINOR);
      s.expectInZone(token, "gone");
      expect(s.hand("p1").map((card) => card.defId)).toEqual([STOCKPILE, VANILLA]);
    });

    it("a Reborn Unit already back on the field is skipped", () => {
      const s = wipe(false, [DEFENDER, VANILLA]);
      const defender = s.card(DEFENDER);
      s.play(BIG_FELINOR);
      s.expectInZone(defender, "field");
      expect(s.hand("p1").map((card) => card.defId)).toEqual([STOCKPILE, VANILLA]);
    });

    it("R69 an Indestructible Unit a destroy leaves standing is not collected, and fires nothing", () => {
      const s = wipe(false, [STATE_OF_GAME]);
      s.play(BIG_FELINOR);
      s.expectInZone(STATE_OF_GAME, "field");
      expect(fired(s)).toBe(0);
    });

    it("R69 an Indestructible Unit whose max health falls to 0 is collected, fires it and comes back", () => {
      // A State of the Game standing as a 1/1 meets Suppressive Aura's −1/−1: max health 0.
      const s = scenario({
        active: "p2",
        p1: {
          hand: [STOCKPILE],
          field: [{ def: STATE_OF_GAME, statsOverride: { attack: 1, health: 1 } }, VANILLA],
          backrow: [{ def: SHADOWSTEP, faceUp: false }],
        },
        p2: { hand: [SUPPRESSIVE_AURA, STOCKPILE] },
      });
      const state = s.card(STATE_OF_GAME);
      s.play(SUPPRESSIVE_AURA, { zone: 1 });
      expect(fired(s)).toBe(1);
      s.expectInZone(state, "hand");
      expect(s.card(state).costOverride).toBe(0);
      s.expectInZone(VANILLA, "field");
    });

    it("your Unit the opponent controls is theirs while it is on the field: its death leaves it set", () => {
      // "Your Units" on the field are the ones you control (§3.2, R12): a Menace of yours they stole
      // dies under their control, so it is not one of your Units dying.
      const s = scenario({
        active: "p2",
        p1: { hand: [BIG_FELINOR, STOCKPILE], field: [MENACE], backrow: [{ def: SHADOWSTEP, faceUp: false }], library: [STOCKPILE] },
        p2: { hand: [MIND_CONTROL, STOCKPILE], library: [STOCKPILE, STOCKPILE] },
      });
      const menace = s.card(MENACE);
      s.play(MIND_CONTROL, { targets: [{ pick: "instance", instanceId: menace.id }] });
      expect(s.card(menace).controller).toBe("p2");
      s.endTurn();
      s.play(BIG_FELINOR);
      expect(fired(s)).toBe(0);
      s.expectInZone(menace, "graveyard");
      expect(s.backrow("p1", 1)?.defId).toBe(SHADOWSTEP);
    });

    it("R317 a full hand burns the overflow into your graveyard", () => {
      const nine = Array.from({ length: 9 }, () => STOCKPILE);
      const s = wipe(false, [VANILLA, MENACE], { myHand: nine });
      const vanilla = s.card(VANILLA);
      const menace = s.card(MENACE);
      s.play(BIG_FELINOR);
      s.expectInZone(vanilla, "hand");
      s.expectInZone(menace, "graveyard");
      expect(s.events.filter((event) => event.type === "burned").map((event) => (event.type === "burned" ? event.instanceId : ""))).toEqual([menace.id]);
    });

    it("its Death hooks still happen: the Units died", () => {
      const s = wipe(false, [SAINTESS, VANILLA]);
      s.play(BIG_FELINOR);
      s.expectInZone(SAINTESS, "hand");
      s.expectEvents("destroyed", "trapFired");
    });

    it("R97 once in your hand the returned cards are named in none of the opponent's view", () => {
      const s = wipe(false, [VANILLA, MENACE]);
      const cards: CardInstance[] = [s.card(VANILLA), s.card(MENACE)];
      s.play(BIG_FELINOR);
      const theirs = JSON.stringify(s.view("p2"));
      for (const card of cards) {
        expect(theirs).not.toContain(`"${card.id}"`);
      }
      expect(s.view("p2").opponent.hand).toEqual({ count: 3 });
    });

    it("§3.2 R640 a stolen Unit you control that dies returns to your hand, as its current owner's", () => {
      const s = scenario({
        p1: { hand: [MIND_CONTROL, STOCKPILE], backrow: [{ def: SHADOWSTEP, faceUp: false }], library: [STOCKPILE] },
        p2: { hand: [BIG_FELINOR, STOCKPILE], field: [MENACE], library: [STOCKPILE, STOCKPILE] },
      });
      const menace = s.card(MENACE);
      s.play(MIND_CONTROL, { targets: [{ pick: "instance", instanceId: menace.id }] });
      expect(s.card(menace).controller).toBe("p1");
      s.endTurn();
      s.play(BIG_FELINOR);
      expect(fired(s)).toBe(1);
      s.expectInZone(menace, "hand");
      // "Return them to your hand": the thief's, its current owner since the steal (R640).
      expect(s.hand("p1").map((card) => card.id)).toContain(menace.id);
      expect(s.hand("p2").map((card) => card.id)).not.toContain(menace.id);
    });

    it("R386 a Degrade of the cost makes them cost (1)", () => {
      const s = wipe(false, [MENACE]);
      stepParam(s.card(SHADOWSTEP), "setCost", 1);
      const menace = s.card(MENACE);
      s.play(BIG_FELINOR);
      expect(s.card(menace).costOverride).toBe(1);
    });
  });

  describe("radiant", () => {
    it("E5 fires in the would-die window: your Units flicker instead and stay in their zones", () => {
      const s = wipe(true, [VANILLA, MENACE]);
      const vanilla = s.card(VANILLA);
      const menace = s.card(MENACE);
      s.play(BIG_FELINOR);
      expect(fired(s)).toBe(1);
      expect(s.unit("p1", 1)?.id).toBe(vanilla.id);
      expect(s.unit("p1", 2)?.id).toBe(menace.id);
      expect(s.events.some((event) => event.type === "destroyed" && event.owner === "p1")).toBe(false);
      s.expectEvents("trapFired", "flickered");
      s.expectInZone(SHADOWSTEP, "graveyard");
    });

    it("R78 flickered: reset, at full health, summoning sick", () => {
      const s = scenario({
        active: "p2",
        p1: { hand: [STOCKPILE], field: [{ def: MENACE, damage: 5 }], backrow: [{ def: SHADOWSTEP, faceUp: false, radiant: true }] },
        p2: { hand: [BIG_FELINOR, STOCKPILE] },
      });
      const menace = s.card(MENACE);
      s.play(BIG_FELINOR);
      s.expectStats(menace, { health: 9, maxHealth: 9 });
      expect(s.card(menace).summonedTurn).toBe(s.state.turn);
    });

    it("no Cry and no Death: a flickered Saintess makes nothing Radiant, a Mr. Token summons nothing", () => {
      const s = wipe(true, [SAINTESS, MR_TOKEN, VANILLA]);
      s.play(BIG_FELINOR);
      expect(mineOnField(s)).toEqual([SAINTESS, MR_TOKEN, VANILLA, null, null]);
      expect(s.events.some((event) => event.type === "radiantSet")).toBe(false);
      expect(s.events.some((event) => event.type === "summoned" && event.defId === RUSH_TOKEN)).toBe(false);
    });

    it("R57 a fresh copy of each goes to your hand, its Radiant flag kept, costing (0)", () => {
      const s = wipe(true, [{ def: VANILLA, radiant: true }, MENACE]);
      const vanilla = s.card(VANILLA);
      s.play(BIG_FELINOR);
      const copies = s.hand("p1").filter((card) => card.defId !== STOCKPILE);
      expect(copies.map((card) => [card.defId, card.radiant, card.costOverride])).toEqual([
        [VANILLA, true, 0],
        [MENACE, false, 0],
      ]);
      expect(copies.map((card) => card.id)).not.toContain(vanilla.id);
    });

    it("R97 the copies are never named in the opponent's view", () => {
      const s = wipe(true, [VANILLA, MENACE]);
      s.play(BIG_FELINOR);
      const copies = s.hand("p1").filter((card) => card.defId !== STOCKPILE);
      const theirs = JSON.stringify(s.view("p2"));
      for (const card of copies) expect(theirs).not.toContain(`"${card.id}"`);
    });

    it("enemy Units still die, and an all-enemy pass leaves it set", () => {
      const s = wipe(true, [FELINORS], { theirs: [VANILLA] });
      s.play(BIG_FELINOR);
      s.expectInZone(VANILLA, "graveyard");
      expect(fired(s)).toBe(0);
      expect(s.backrow("p1", 1)?.defId).toBe(SHADOWSTEP);
    });

    it("R317 a full hand burns the overflowing copies", () => {
      const nine = Array.from({ length: 9 }, () => STOCKPILE);
      const s = wipe(true, [VANILLA, MENACE], { myHand: nine });
      s.play(BIG_FELINOR);
      expect(s.hand("p1")).toHaveLength(10);
      expect(s.events.filter((event) => event.type === "burned")).toHaveLength(1);
    });

    it("R386 a Degrade of the cost makes the copies cost (1)", () => {
      const s = wipe(true, [MENACE]);
      stepParam(s.card(SHADOWSTEP), "setCost", 1);
      s.play(BIG_FELINOR);
      const copy = s.hand("p1").find((card) => card.defId === MENACE);
      expect(copy?.costOverride).toBe(1);
    });
  });
});
