// C+ #33 Ivory Tower — SPEC §8.7 row 33, R418, R651, BUILD M9 Classic+ row C+ 33: "Field Spell: the first
// Unit you play onto its zone stands on it while its play resolves, its Cry included, and is then fused
// into it per R77, the Tower the kept card: a Field Spell still, with the Unit's text, keywords and stats,
// so the Unit's aura covers your side, its end-of-turn line runs and its Death fires when the Tower dies;
// the Unit ceases to exist, with no Death; after that first Unit, no other may be played onto it this
// stay; a Tower that leaves before the play resolves fuses nothing, its Unit stepping down (R446); an
// answer that replaced the Unit in place leaves its replacement to be fused; the old aura (your cards have
// Stack) and the Unit that could neither attack nor be attacked are gone; radiant the Unit becomes Radiant
// as it lands, so its Cry runs on that face, and is fused in on its Radiant face (R469)".

import { HERO_HEALTH, carriedAt, defOf, effectiveCost, legalActions } from "@jackioh/engine";
import type { Action } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { scenario, type FieldSetup, type Scenario } from "../_harness";

const TOWER = "classicplus-033";
const TOKENS = "core-015"; // (1) 1/1, Cry: summon a Rush Token (Radiant: 3).
const RUSH = "core-t-rush";
const MENACE = "core-019"; // (3) 9/9 Taunt.
const TOE_CRACKER = "classic-006"; // (2) 3/4, Aura: your Traps cost (0).
const JAY = "classicplus-049"; // (2) 3/6 Taunt, End of turn: a random card in your hand costs (2) less.
const SHEEPLE = "classic-082"; // (1) 1/1, Death: draw 2.
const GUY_ATT = "classicplus-005"; // (2) 6/8, Cry: destroy every backrow card you control.
const SHEEPISH = "core-041"; // (1) Trap: transforms an opponent's played Unit once its Cry resolves.
const SHEEP = "core-t-sheep";
const MAGIC_JAMMED = "core-036"; // (1) Spell: destroy target backrow card, Lock its zone.
const BIG_SPELL = "core-035"; // (1) Spell, a hand card to discount.
const FILLER = "core-005";
const DECK = [FILLER, FILLER, FILLER, FILLER];

/** p1 with a Tower in backrow lane 2; plays `rider` onto it. */
function towerWith(
  rider: string,
  opts: { radiant?: boolean; field?: readonly FieldSetup[]; hand?: string[]; p2Backrow?: readonly FieldSetup[] } = {},
): { s: Scenario; tower: string; rider: string } {
  const s = scenario({
    p1: {
      hand: [rider, ...(opts.hand ?? [FILLER])],
      field: opts.field ?? [],
      backrow: [{ def: TOWER, lane: 2, radiant: opts.radiant === true }],
      library: DECK,
      mana: 8,
    },
    p2: { hand: [FILLER], field: [MENACE], backrow: opts.p2Backrow ?? [], library: DECK },
  });
  const tower = s.card(TOWER).id;
  const riderId = s.card(rider).id;
  s.play(rider, { zone: 2, row: "backrow" });
  return { s, tower, rider: riderId };
}

const textOf = (s: Scenario, id: string): string => {
  const card = s.card(id);
  const def = defOf(s.state, card.defId);
  return (card.radiant ? def.radiant : def.base).text;
};

describe("C+ #33 Ivory Tower", () => {
  describe("base", () => {
    it("R418 R651 a Unit you play may name the Tower's zone; its Cry resolves, then it is fused into the Tower", () => {
      const { s, tower, rider } = towerWith(TOKENS);
      // Its Cry ran while it stood on the Tower: a Rush Token in the unit row.
      expect(s.unit("p1", 1)?.defId).toBe(RUSH);
      // Then it was fused in: the Tower is the kept card, still a Field Spell in its zone.
      expect(s.backrow("p1", 2)?.id).toBe(tower);
      const fused = defOf(s.state, s.card(tower).defId);
      expect(fused.type).toBe("Field Spell");
      expect(fused.name).toBe("Me and Mr Token + Ivory Tower");
      expect(carriedAt(s.state, { player: "p1", row: "backrow", lane: 2 })).toBeNull();
      s.expectInZone(rider, "gone");
      const types = s.events.map((event) => event.type);
      expect(types.indexOf("cardResolved")).toBeLessThan(types.indexOf("fused"));
    });

    it("R651 the Unit ceases to exist: no Death, not destroyed, never in a graveyard", () => {
      const { s, rider } = towerWith(SHEEPLE);
      s.expectInZone(rider, "gone");
      expect(s.pile("p1", "graveyard")).toHaveLength(0);
      expect(s.events.some((event) => event.type === "destroyed")).toBe(false);
      expect(s.hand("p1")).toHaveLength(1); // the filler: Sheeople's Death drew nothing
    });

    it("R651 the Tower takes the Unit's text and keywords: its aura covers your side from the backrow", () => {
      const { s, tower } = towerWith(TOE_CRACKER, { hand: [SHEEPISH, FILLER] });
      expect(textOf(s, tower)).toContain("Aura: Your Traps cost (0).");
      expect(effectiveCost(s.state, s.card(SHEEPISH))).toBe(0);
    });

    it("R651 the Unit's keywords do nothing in the backrow: a fused Taunt binds no attacker, and its end-of-turn line runs", () => {
      const { s, tower } = towerWith(JAY, { hand: [BIG_SPELL, FILLER] });
      expect(defOf(s.state, s.card(tower).defId).base.keywords).toContainEqual({ kind: "Taunt" });
      const before = s.hand("p1").map((card) => effectiveCost(s.state, card));
      s.endTurn();
      const after = s.hand("p1").map((card) => effectiveCost(s.state, card));
      expect(after.reduce((a, b) => a + b, 0)).toBeLessThan(before.reduce((a, b) => a + b, 0));
      // p2's Menace may attack the hero: the Tower's Taunt is a backrow card's, and binds no attacker.
      s.attack(MENACE, "hero");
      s.expectHealth("p1", HERO_HEALTH - 9);
    });

    it("R418 the Tower stays a backrow card, and the Unit's Death lives on: it fires when the Tower is destroyed", () => {
      const { s, tower } = towerWith(SHEEPLE, { hand: [MAGIC_JAMMED, FILLER] });
      const held = s.hand("p1").length;
      s.play(MAGIC_JAMMED, { targets: [{ pick: "instance", instanceId: tower }] });
      s.expectInZone(tower, "graveyard");
      // Magic Jammed left the hand; Sheeople's Death drew 2.
      expect(s.hand("p1")).toHaveLength(held - 1 + 2);
    });

    it("R651 after the first Unit, no more Units can be stacked onto it", () => {
      const { s } = towerWith(TOKENS, { hand: [MENACE, FILLER] });
      expect(() => s.play(MENACE, { zone: 2, row: "backrow" })).toThrow();
      const plays = legalActions(s.state, "p1").filter(
        (action): action is Extract<Action, { type: "play" }> => action.type === "play" && action.instanceId === s.card(MENACE).id,
      );
      expect(plays.length).toBeGreaterThan(0);
      expect(plays.some((action) => action.zone?.row === "backrow")).toBe(false);
    });

    it("R651 a Tower its Unit's Cry destroys fuses nothing: Guy Att steps down into a unit zone (R446)", () => {
      const { s, tower, rider } = towerWith(GUY_ATT);
      s.expectInZone(tower, "graveyard");
      expect(s.unit("p1", 2)?.id).toBe(rider);
      expect(s.events.some((event) => event.type === "fused")).toBe(false);
      expect(s.card(rider).defId).toBe(GUY_ATT);
    });

    it("R651 an answer that replaces the Unit where it stands leaves its replacement to be fused: Sheepish's Sheep", () => {
      const { s, tower } = towerWith(TOKENS, { p2Backrow: [{ def: SHEEPISH, faceUp: false }] });
      expect(s.events.some((event) => event.type === "trapFired")).toBe(true);
      expect(defOf(s.state, s.card(tower).defId).name).toBe("Sheep Token + Ivory Tower");
      expect(carriedAt(s.state, { player: "p1", row: "backrow", lane: 2 })).toBeNull();
      expect(s.unit("p1", 2)).toBeNull();
      expect(s.events.some((event) => event.type === "summoned" && event.defId === SHEEP)).toBe(false);
    });

    it("the old aura is gone: your cards gain no Stack, and an occupied unit zone takes no Unit", () => {
      const s = scenario({
        p1: { hand: [MENACE, FILLER], field: [{ def: MENACE, lane: 1 }], backrow: [TOWER], mana: 8 },
        p2: { hand: [FILLER] },
      });
      const held = s.hand("p1").find((card) => card.defId === MENACE);
      if (held === undefined) throw new Error("Menace in hand");
      expect(s.stats(held).keywords).not.toContainEqual({ kind: "Stack" });
      expect(() => s.play(held, { zone: 1 })).toThrow();
    });

    it("a Unit played into a unit zone is untouched, and the Tower may still take a Unit after it", () => {
      const s = scenario({
        p1: { hand: [MENACE, TOKENS, FILLER], backrow: [{ def: TOWER, lane: 2 }], library: DECK, mana: 8 },
        p2: { hand: [FILLER] },
      });
      s.play(MENACE, { zone: 1 });
      expect(s.unit("p1", 1)?.defId).toBe(MENACE);
      expect(s.events.some((event) => event.type === "fused")).toBe(false);
      s.play(TOKENS, { zone: 2, row: "backrow" });
      expect(s.events.some((event) => event.type === "fused")).toBe(true);
    });
  });

  describe("radiant", () => {
    it("the Unit becomes Radiant as it lands, so its Cry runs on the Radiant face", () => {
      const { s } = towerWith(TOKENS, { radiant: true });
      const types = s.events.map((event) => event.type);
      expect(types.indexOf("radiantSet")).toBeLessThan(types.indexOf("cardResolved"));
      // Me and Mr Token's Radiant Cry summons 3 Rush Tokens, its base Cry 1.
      expect(s.events.filter((event) => event.type === "summoned" && event.defId === RUSH)).toHaveLength(3);
    });

    it("R469 it is fused in on its Radiant face", () => {
      const { s, tower } = towerWith(TOE_CRACKER, { radiant: true });
      // Cloaked Toe Cracker's Radiant face adds "After you play a Trap, gain 1 mana", on both fused faces.
      expect(textOf(s, tower)).toContain("After you play a Trap");
      expect(defOf(s.state, s.card(tower).defId).base.text).toContain("After you play a Trap");
    });

    it("a Unit played into a unit zone is not made Radiant", () => {
      const s = scenario({
        p1: { hand: [MENACE, FILLER], backrow: [{ def: TOWER, radiant: true }], mana: 8 },
        p2: { hand: [FILLER] },
      });
      s.play(MENACE, { zone: 1 });
      expect(s.card(MENACE).radiant).toBe(false);
    });
  });
});
