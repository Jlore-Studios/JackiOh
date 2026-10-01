// C+ #33 Ivory Tower — SPEC §8.7 row 33, R418, BUILD M9 Classic+ row C+ 33: "Field Spell with the
// aura "Your cards have Stack", in hand and on the field, so your Units may be played onto your
// occupied unit zones and your backrow cards onto your occupied backrow zones (a backrow pile: only the
// top acts, a face-down trap beneath can't fire); a Unit you play may name the Tower's zone and top it,
// one Unit at most: a Unit for every rule (its text works, it is targeted and hit by "all Units") that
// can neither attack nor be attacked, declared or forced; the Tower stays active beneath it (R418), its
// aura holding and "all Field Spells" effects finding it; backrow effects (Guy Att, Crushing Walls)
// pass the Unit by; when the Tower leaves, the Unit moves to your unit zone in that lane, else the
// leftmost open one (R64), with no Cry and no reset (R78 does not apply), and is destroyed with no open
// zone; the opponent's cards gain no Stack; radiant the Unit that tops it becomes Radiant".

import { carriedAt, legalActions } from "@jackioh/engine";
import type { Action } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { scenario, type FieldSetup, type Scenario } from "../_harness";

const TOWER = "classicplus-033";
const MENACE = "core-019"; // (3) 9/9 Taunt.
const TOKENS = "core-015"; // (1) 1/1, Cry: summon a Rush Token.
const RUSH = "core-t-rush";
const LUNAR = "core-035"; // (1) Spell: 3 damage to a target.
const BONE_STORM = "classicplus-036-1"; // (1) Spell: 1 damage to each enemy.
const MAGIC_JAMMED = "core-036"; // (1) Spell: destroy target backrow card, Lock its zone.
const NETHER = "core-088"; // (4) Spell: destroy all permanents.
const MANA_WELL = "core-006"; // (3) Field Spell.
const SHEEPISH = "core-041"; // (1) Trap: transforms an opponent's played Unit after its Cry.
const HONEYPOT = "core-060"; // (1) Trap; Radiant: any play fills its board with Rush Tokens that attack a played Unit.
const FILLER = "core-005";

/** p1 with a Tower in backrow lane 2; plays `rider` on top of it. */
function towerWith(rider: string, opts: { radiant?: boolean; field?: readonly FieldSetup[]; hand?: string[] } = {}): Scenario {
  const s = scenario({
    p1: {
      hand: [rider, ...(opts.hand ?? [FILLER])],
      field: opts.field ?? [],
      backrow: [{ def: TOWER, lane: 2, radiant: opts.radiant === true }],
      library: [FILLER, FILLER, FILLER],
      mana: 8,
    },
    p2: { hand: [FILLER], field: [MENACE], library: [FILLER, FILLER, FILLER] },
  });
  s.play(rider, { zone: 2, row: "backrow" });
  return s;
}

function riderOf(s: Scenario, player: "p1" | "p2" = "p1"): string | undefined {
  return carriedAt(s.state, { player, row: "backrow", lane: 2 })?.id;
}

describe("C+ #33 Ivory Tower", () => {
  describe("base", () => {
    it("the aura gives your Units in hand Stack: one may be played onto your occupied unit zone", () => {
      const s = scenario({
        p1: { hand: [MENACE, FILLER], field: [{ def: MENACE, lane: 1 }], backrow: [TOWER], mana: 8 },
        p2: { hand: [FILLER] },
      });
      const held = s.hand("p1").find((card) => card.defId === MENACE);
      if (held === undefined) throw new Error("Menace in hand");
      expect(s.stats(held).keywords).toContainEqual({ kind: "Stack" });
      s.play(held, { zone: 1 });
      expect(s.unit("p1", 1)?.id).toBe(held.id);
      expect(s.pile("p1", "graveyard")).toHaveLength(0);
    });

    it("the opponent's cards gain no Stack", () => {
      const s = scenario({
        active: "p2",
        p1: { hand: [FILLER], backrow: [TOWER] },
        p2: { hand: [MENACE, FILLER], field: [{ def: MENACE, lane: 1 }], mana: 8 },
      });
      const held = s.hand("p2").find((card) => card.defId === MENACE);
      if (held === undefined) throw new Error("Menace in hand");
      expect(s.stats(held).keywords).not.toContainEqual({ kind: "Stack" });
      expect(() => s.play(held, { zone: 1 })).toThrow();
    });

    it("backrow cards pile on backrow cards; a face-down trap beneath can't fire", () => {
      const s = scenario({
        p1: { hand: [MANA_WELL, FILLER], backrow: [TOWER, { def: SHEEPISH, lane: 3, faceUp: false }], mana: 8 },
        p2: { hand: [TOKENS, FILLER], library: [FILLER, FILLER] },
      });
      s.play(MANA_WELL, { zone: 3 });
      expect(s.backrow("p1", 3)?.defId).toBe(MANA_WELL);
      s.endTurn();
      s.play(TOKENS); // a Unit: Sheepish would transform it, but it lies dormant.
      expect(s.unit("p2", 1)?.defId).toBe(TOKENS);
      expect(s.events.some((event) => event.type === "trapFired")).toBe(false);
    });

    it("R418 a Unit you play may name the Tower's zone and top it; its own text works", () => {
      const s = towerWith(TOKENS);
      const rider = s.card(TOKENS);
      expect(riderOf(s)).toBe(rider.id);
      expect(s.backrow("p1", 2)?.defId).toBe(TOWER);
      // Its Cry summoned a Rush Token into the unit row.
      expect(s.unit("p1", 1)?.defId).toBe(RUSH);
    });

    it("one Unit at most: a second Unit can't name the Tower's zone", () => {
      const s = towerWith(MENACE, { hand: [TOKENS, FILLER] });
      expect(() => s.play(TOKENS, { zone: 2, row: "backrow" })).toThrow();
    });

    it("it can neither attack nor be attacked", () => {
      const s = towerWith(MENACE);
      s.endTurn().endTurn(); // no longer summoning sick
      expect(() => s.attack(MENACE, "hero")).toThrow();
      s.endTurn();
      const enemy = s.unit("p2", 1);
      if (enemy === null) throw new Error("p2's Menace");
      const rider = riderOf(s) ?? "";
      expect(() => s.attack(enemy, rider)).toThrow();
      const attacks = legalActions(s.state, "p2").filter(
        (action): action is Extract<Action, { type: "attack" }> => action.type === "attack",
      );
      expect(attacks.some((action) => action.targetId === rider)).toBe(false);
      expect(attacks.length).toBeGreaterThan(0);
    });

    it("nor attacked by a forced attack: a Radiant Bear Honeypot's tokens leave it alone", () => {
      const s = scenario({
        p1: { hand: [MENACE, FILLER], backrow: [{ def: TOWER, lane: 2 }], mana: 8 },
        p2: { hand: [FILLER], backrow: [{ def: HONEYPOT, radiant: true, faceUp: false }] },
      });
      s.play(MENACE, { zone: 2, row: "backrow" });
      expect(s.events.some((event) => event.type === "trapFired")).toBe(true);
      expect(s.events.some((event) => event.type === "summoned" && event.defId === RUSH)).toBe(true);
      expect(s.events.some((event) => event.type === "damage" && event.targetId === s.card(MENACE).id)).toBe(false);
      s.expectStats(MENACE, { health: 9 });
    });

    it("it is a Unit for every rule: targeted, and hit by 'all Units' effects", () => {
      const s = scenario({
        active: "p2",
        p1: { hand: [MENACE, FILLER], backrow: [{ def: TOWER, lane: 2 }], mana: 8 },
        p2: { hand: [LUNAR, BONE_STORM, FILLER], mana: 8 },
      });
      s.endTurn();
      s.play(MENACE, { zone: 2, row: "backrow" });
      s.endTurn();
      s.play(LUNAR, { targets: [{ pick: "instance", instanceId: s.card(MENACE).id }] });
      s.expectStats(MENACE, { health: 6 });
      s.play(BONE_STORM);
      s.expectStats(MENACE, { health: 5 });
    });

    it("R418 the Tower stays active beneath it: its aura holds and 'all permanents' still finds it", () => {
      const s = towerWith(MENACE, { hand: [TOKENS, NETHER, FILLER] });
      expect(s.stats(s.card(TOKENS)).keywords).toContainEqual({ kind: "Stack" });
      s.play(NETHER);
      s.expectInZone(TOWER, "graveyard").expectInZone(MENACE, "graveyard");
    });

    it("backrow effects pass the Unit by: Magic Jammed may target the Tower, never the Unit", () => {
      const s = towerWith(MENACE, { hand: [MAGIC_JAMMED, FILLER] });
      expect(() => s.play(MAGIC_JAMMED, { targets: [{ pick: "instance", instanceId: s.card(MENACE).id }] })).toThrow();
      s.play(MAGIC_JAMMED, { targets: [{ pick: "instance", instanceId: s.card(TOWER).id }] });
      s.expectInZone(TOWER, "graveyard");
      s.expectInZone(MENACE, "field");
    });

    it("R418 when the Tower leaves, the Unit moves to its lane's unit zone with no Cry and no reset", () => {
      const s = towerWith(TOKENS, { hand: [MAGIC_JAMMED, LUNAR, FILLER] });
      const rider = s.card(TOKENS).id;
      const tokens = s.events.filter((event) => event.type === "summoned" && event.defId === RUSH).length;
      // Lane 2's unit zone is open (the Cry's Rush Token took lane 1).
      s.play(MAGIC_JAMMED, { targets: [{ pick: "instance", instanceId: s.card(TOWER).id }] });
      expect(s.unit("p1", 2)?.id).toBe(rider);
      expect(s.events.filter((event) => event.type === "summoned" && event.defId === RUSH)).toHaveLength(tokens);
      expect(s.card(rider).id).toBe(rider);
    });

    it("R64 with its lane's unit zone taken, it moves to the leftmost open one, keeping its damage", () => {
      const s = towerWith(MENACE, { field: [{ def: TOKENS, lane: 2 }], hand: [LUNAR, MAGIC_JAMMED, FILLER] });
      s.play(LUNAR, { targets: [{ pick: "instance", instanceId: s.card(MENACE).id }] });
      s.play(MAGIC_JAMMED, { targets: [{ pick: "instance", instanceId: s.card(TOWER).id }] });
      expect(s.unit("p1", 1)?.defId).toBe(MENACE);
      s.expectStats(MENACE, { health: 6 });
    });

    it("with no open unit zone, the Unit is destroyed when the Tower leaves", () => {
      const full = [MENACE, MENACE, MENACE, MENACE, MENACE];
      const s = towerWith(TOKENS, { field: full, hand: [MAGIC_JAMMED, FILLER] });
      const rider = s.card(TOKENS).id;
      s.play(MAGIC_JAMMED, { targets: [{ pick: "instance", instanceId: s.card(TOWER).id }] });
      s.expectInZone(rider, "graveyard");
    });
  });

  describe("radiant", () => {
    it("the Unit that tops it becomes Radiant", () => {
      const s = towerWith(MENACE, { radiant: true });
      expect(s.card(MENACE).radiant).toBe(true);
      s.expectStats(MENACE, { attack: 18, health: 18 });
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
