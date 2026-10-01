// C #77 Anti-Magic Monkey — SPEC §8.6 row 77, BUILD M9 Classic row C 77: "Stack; Aura: every Spell (the
// Spell type, not a Field Spell or a Trap) either player could play (in a hand, or in a graveyard a
// permission lets its owner play from, R65) costs (1) more; X-cost Spells untouched (R65); casts pay
// nothing (R70); it is off while dormant under a Stack pile and gone when it leaves; a hidden hand
// card's `costChanged` shows −1 to the other player (R177); radiant 10/10: (2) more; its tuned number
// (surcharge) reads through `param()` (R386)".
//
// The aura is B5 E15's price rule (B5 E15), read by R65's `effectiveCost` wherever a play takes a card
// from. A graveyard play needs a permission (C #28, C #74, C #90's reward L), each another
// workstream's card: the engine's own cost-rules tests prove the graveyard half of `effectiveCost`
// through fixtures, and this file proves the hand half and every other clause.

import { legalActions, stepParam } from "@jackioh/engine";
import type { PlayerId } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { base, def, radiant } from "../../src/scripts/classic/077-anti-magic-monkey";
import { scenario, type Scenario } from "../_harness";

const MONKEY = "classic-077";
const STOCKPILE = "core-005"; // (1) Spell: Draw 2. Heal your hero 2.
const ECLIPSE = "core-035"; // (1) Spell: Deal 3 damage to a target …
const DIVIDEND = "core-024"; // (X) Spell, Choose one
const ARMOR = "core-073"; // (2) Field Spell
const HONEYPOT = "core-060"; // (1) Trap
const VANILLA = "core-008"; // (1) Unit 4/4
const FIENDER = "core-092"; // (2) Stack Unit
const HIT_JOB = "core-016"; // (3) Spell: Destroy target Unit.
const CN_VIRUS = "core-090-1"; // (1) Spell, Cast on draw
const RISKY_DIE = "classic-079"; // (1) Spell: Draw 3; they cost (1) less; …
const FILLER = "core-010"; // (0) Spell

/** The cost a player's own view gives a card in their hand (R65). */
function handCost(s: Scenario, player: PlayerId, defId: string): number | undefined {
  const hand = s.view(player).you.hand;
  return Array.isArray(hand) ? hand.find((card) => card.defId === defId)?.cost : undefined;
}

describe("C #77 Anti-Magic Monkey", () => {
  it("is a (2) 5/5 Stack Unit (10/10 Radiant), its surcharge a declared number", () => {
    expect(def.cost).toBe(2);
    expect(def.base.keywords).toEqual([{ kind: "Stack" }]);
    expect(def.radiant.keywords).toEqual([{ kind: "Stack" }]);
    expect(def.params).toEqual([{ key: "surcharge", base: 1, radiant: 2, better: "up", step: 1, min: 1 }]);
    expect(radiant).toBe(base);
  });

  describe("base", () => {
    it("every Spell in either player's hand costs (1) more, and a play pays it", () => {
      const s = scenario({ p1: { hand: [STOCKPILE, FILLER], field: [MONKEY] }, p2: { hand: [ECLIPSE] } });
      expect(handCost(s, "p1", STOCKPILE)).toBe(2);
      expect(handCost(s, "p1", FILLER)).toBe(1);
      expect(handCost(s, "p2", ECLIPSE)).toBe(2);
      s.play(STOCKPILE).expectMana("p1", 2);
      expect(s.lastEvents.find((event) => event.type === "cardPlayed")).toMatchObject({ costPaid: 2 });
    });

    it("the Spell type only: a Field Spell, a Trap and a Unit cost what they print", () => {
      const s = scenario({ p1: { hand: [ARMOR, HONEYPOT, VANILLA], field: [MONKEY] } });
      expect(handCost(s, "p1", ARMOR)).toBe(2);
      expect(handCost(s, "p1", HONEYPOT)).toBe(1);
      expect(handCost(s, "p1", VANILLA)).toBe(1);
    });

    it("R65 an X-cost Spell is untouched: it costs exactly its X, up to all your mana", () => {
      const s = scenario({ p1: { hand: [DIVIDEND, FILLER], field: [MONKEY] } });
      expect(handCost(s, "p1", DIVIDEND)).toBe(0);
      const id = s.card(DIVIDEND).id;
      const xs = legalActions(s.state, "p1").flatMap((action) =>
        action.type === "play" && action.instanceId === id ? [action.x] : [],
      );
      expect(Math.max(...xs.map((x) => x ?? 0))).toBe(4);
      s.play(DIVIDEND, { x: 4, modes: ["damage"], targets: [{ pick: "hero", player: "p2" }] }).expectMana("p1", 0);
    });

    it("R70 a cast pays nothing: a Spell cast on draw is played for (0)", () => {
      const s = scenario({ p1: { hand: [STOCKPILE, FILLER], field: [MONKEY], library: [CN_VIRUS, VANILLA, VANILLA] } });
      s.play(STOCKPILE);
      const cast = s.lastEvents.find((event) => event.type === "cardPlayed" && event.defId === CN_VIRUS);
      expect(cast).toMatchObject({ costPaid: 0 });
    });

    it("§3.2 R13 it is off while dormant under a Stack pile", () => {
      const s = scenario({ p1: { hand: [STOCKPILE, FILLER], field: [MONKEY, { def: FIENDER, stack: true }] } });
      expect(handCost(s, "p1", STOCKPILE)).toBe(1);
    });

    it("§6.2 Stack: it may be played onto an occupied unit zone, and its aura starts as it lands", () => {
      const s = scenario({ p1: { hand: [MONKEY, STOCKPILE], field: [VANILLA] } });
      s.play(MONKEY, { zone: 1 });
      expect(s.unit("p1", 1)?.defId).toBe(MONKEY);
      expect(handCost(s, "p1", STOCKPILE)).toBe(2);
    });

    it("gone when it leaves: destroyed, a Spell costs what it prints again", () => {
      const s = scenario({ p1: { hand: [HIT_JOB, STOCKPILE], field: [MONKEY], mana: 9 } });
      expect(handCost(s, "p1", HIT_JOB)).toBe(4);
      s.play(HIT_JOB, { targets: [{ pick: "instance", instanceId: s.card(MONKEY).id }] });
      s.expectInZone(MONKEY, "graveyard");
      expect(handCost(s, "p1", STOCKPILE)).toBe(1);
    });

    it("R65 two Monkeys add up: a Spell costs (2) more", () => {
      const s = scenario({ p1: { hand: [STOCKPILE], field: [MONKEY, MONKEY] } });
      expect(handCost(s, "p1", STOCKPILE)).toBe(3);
    });

    it("a Spell you cannot afford with the surcharge is not offered, and its play is refused", () => {
      const s = scenario({ p1: { hand: [STOCKPILE, FILLER], field: [MONKEY], mana: 1 } });
      const id = s.card(STOCKPILE).id;
      expect(legalActions(s.state, "p1").some((action) => action.type === "play" && action.instanceId === id)).toBe(false);
      expect(() => s.play(STOCKPILE)).toThrow(/costs 2/);
    });

    it("R177 a hidden hand card's `costChanged` shows −1 and the sentinel to the other player", () => {
      // Radiant Risky Die keeps a drawn (1) Spell: (1) − 1 + the Monkey's (1) = (1), not more than (1).
      const s = scenario({
        p1: { hand: [{ def: RISKY_DIE, radiant: true }, FILLER], field: [MONKEY], library: [ECLIPSE, VANILLA, VANILLA] },
      });
      s.play(RISKY_DIE);
      const eclipse = s.card(ECLIPSE);
      s.expectInZone(eclipse, "hand");
      const own = s.view("p1").events.filter((event) => event.type === "costChanged" && event.instanceId === eclipse.id);
      expect(own).toEqual([{ type: "costChanged", instanceId: eclipse.id, cost: 1 }]);
      const theirs = s.view("p2").events.filter((event) => event.type === "costChanged");
      expect(theirs.length).toBeGreaterThan(0);
      for (const event of theirs) expect(event).toMatchObject({ instanceId: "hidden", cost: -1 });
    });

    it("R386 its surcharge is the declared number: an Upgrade's step makes Spells cost (2) more", () => {
      const s = scenario({ p1: { hand: [STOCKPILE], field: [MONKEY] } });
      stepParam(s.card(MONKEY), "surcharge", 1);
      expect(handCost(s, "p1", STOCKPILE)).toBe(3);
    });
  });

  describe("radiant", () => {
    it("is a 10/10 with Stack", () => {
      const s = scenario({ p1: { hand: [{ def: MONKEY, radiant: true }, FILLER] } });
      s.play(MONKEY).expectStats(MONKEY, { attack: 10, health: 10 });
      expect(s.stats(MONKEY).keywords.map((keyword) => keyword.kind)).toEqual(["Stack"]);
    });

    it("every Spell in either player's hand costs (2) more", () => {
      const s = scenario({ p1: { hand: [STOCKPILE], field: [{ def: MONKEY, radiant: true }] }, p2: { hand: [ECLIPSE, ARMOR] } });
      expect(handCost(s, "p1", STOCKPILE)).toBe(3);
      expect(handCost(s, "p2", ECLIPSE)).toBe(3);
      expect(handCost(s, "p2", ARMOR)).toBe(2);
    });

    it("R386 its declared surcharge steps from 2: an Upgrade makes it (3), a Degrade (1)", () => {
      const up = scenario({ p1: { hand: [STOCKPILE], field: [{ def: MONKEY, radiant: true }] } });
      stepParam(up.card(MONKEY), "surcharge", 1);
      expect(handCost(up, "p1", STOCKPILE)).toBe(4);
      const down = scenario({ p1: { hand: [STOCKPILE], field: [{ def: MONKEY, radiant: true }] } });
      stepParam(down.card(MONKEY), "surcharge", -1);
      expect(handCost(down, "p1", STOCKPILE)).toBe(2);
    });

    it("R70 a cast still pays nothing", () => {
      const s = scenario({
        p1: { hand: [STOCKPILE, FILLER], field: [{ def: MONKEY, radiant: true }], library: [CN_VIRUS, VANILLA, VANILLA], mana: 9 },
      });
      s.play(STOCKPILE);
      expect(s.lastEvents.find((event) => event.type === "cardPlayed" && event.defId === CN_VIRUS)).toMatchObject({ costPaid: 0 });
    });
  });
});
