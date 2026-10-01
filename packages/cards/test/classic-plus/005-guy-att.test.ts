// C+ #5 Guy Att — SPEC §8.7 row 5, BUILD M9 Classic+ row C+ 5: "Cry destroys every backrow card you
// control, face-down traps and Field Spells alike, an Indestructible one staying (R46); a destroyed
// backrow card that prints Death fires it (§4.5, C+ #61); an 'Animated on your turn' card animated as a
// Unit is not a backrow card and stays; no Cry when copied or recruited (R1); radiant destroys every
// backrow card on both sides, the opponent's face-down cards reaching their graveyard openly (R97)".
//
// The "prints Death" clause is proved with C+ #12.8 Frostspatula, a Field Spell whose Death fires when
// it is destroyed in the backrow (C+ #61 proves it again in its own file).

import { carriedAt } from "@jackioh/engine";
import { describe, expect, it } from "vitest";
import { scenario, type Scenario, type SideSetup } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic-plus/005-guy-att";

const GUY = "classicplus-005";
const SHEEPISH = "core-041"; // (1) Trap.
const MY_PAWN = "core-096"; // (1) Trap: fires only on a lethal attack, so a play never sets it off.
const MANA_WELL = "core-006"; // (3) Field Spell.
const HEROIC_POWER = "core-098"; // Indestructible Field Spell.
const FROSTSPATULA = "classicplus-012-8"; // Field Spell token: Animated on your turn, Rush.
const TOWER = "classicplus-033"; // Ivory Tower: a Unit may be played on top of it.
const VANILLA = "core-008"; // (1) 4/4.
const COOKIE_GUILD = "classic-031"; // (2) Cry: Recruit 1 Unit of (2) Cost or less.
const MR_TOKEN = "core-015"; // (1) 1/1, Cry: summon a Rush Token.
const FILLER = "core-010";
const STOCKPILE = "core-005";

function setup(p1: SideSetup, p2: SideSetup = {}, radiantFace = false): Scenario {
  return scenario({
    p1: { hand: [{ def: GUY, radiant: radiantFace }, FILLER], library: [STOCKPILE, STOCKPILE], mana: 8, ...p1 },
    p2: { hand: [FILLER], library: [STOCKPILE, STOCKPILE], ...p2 },
  });
}

const MY_BACKROW = [
  { def: SHEEPISH, faceUp: false, lane: 1 },
  { def: MANA_WELL, lane: 2 },
];
const THEIR_BACKROW = [
  { def: MY_PAWN, faceUp: false, lane: 1 },
  { def: MANA_WELL, lane: 3 },
];

describe("C+ #5 Guy Att", () => {
  it("is a (2) 6/8 Human Unit (Radiant 12/16) with a Cry on each face", () => {
    expect([def.cost, def.base.attack, def.base.health, def.radiant.attack, def.radiant.health]).toEqual([2, 6, 8, 12, 16]);
    expect(def.tags).toEqual(["Human"]);
    expect(base.cry).toBeTypeOf("function");
    expect(radiant.cry).toBeTypeOf("function");
  });

  describe("base", () => {
    it("its Cry destroys every backrow card you control, face-down traps and Field Spells alike; theirs stay", () => {
      const s = setup({ backrow: MY_BACKROW }, { backrow: THEIR_BACKROW });
      const mine = [s.backrow("p1", 1)!, s.backrow("p1", 2)!];
      const theirs = [s.backrow("p2", 1)!, s.backrow("p2", 3)!];

      s.play(GUY, { zone: 1 });

      for (const card of mine) s.expectInZone(card, "graveyard");
      for (const card of theirs) expect(s.card(card).zone.z).toBe("field");
      expect(s.events.filter((event) => event.type === "destroyed")).toHaveLength(2);
    });

    it("R46 an Indestructible backrow card stays", () => {
      const s = setup({ backrow: [{ def: HEROIC_POWER, lane: 3 }, { def: MANA_WELL, lane: 4 }] });

      s.play(GUY, { zone: 1 });

      expect(s.backrow("p1", 3)?.defId).toBe(HEROIC_POWER);
      s.expectInZone(MANA_WELL, "graveyard");
    });

    it("R383 an 'Animated on your turn' card standing animated in a unit zone is a Unit, and stays", () => {
      const s = setup({ backrow: [{ def: FROSTSPATULA, lane: 3 }, { def: MANA_WELL, lane: 4 }] });
      const spatula = s.card(FROSTSPATULA);
      s.startTurn(); // B3.1: it animates at its controller's start of turn, into its lane's unit zone.
      expect(s.unit("p1", 3)?.id).toBe(spatula.id);

      s.play(GUY, { zone: 1 });

      expect(s.unit("p1", 3)?.id).toBe(spatula.id);
      s.expectInZone(MANA_WELL, "graveyard");
    });

    it("R418 an Ivory Tower is a backrow card and is destroyed; the Unit it carried is not, and steps down", () => {
      const s = setup({ hand: [VANILLA, { def: GUY }, FILLER], backrow: [{ def: TOWER, lane: 2 }] });
      const rider = s.card(VANILLA);
      s.play(rider, { zone: 2, row: "backrow" });
      expect(carriedAt(s.state, { player: "p1", row: "backrow", lane: 2 })?.id).toBe(rider.id);

      s.play(GUY, { zone: 1 });

      s.expectInZone(TOWER, "graveyard");
      expect(s.unit("p1", 2)?.id).toBe(rider.id);
    });

    it("R1 a recruited Guy Att fires no Cry: your backrow stays", () => {
      const s = scenario({
        p1: { hand: [COOKIE_GUILD, FILLER], library: [GUY, STOCKPILE], backrow: MY_BACKROW, mana: 8 },
        p2: { hand: [FILLER] },
      });

      s.play(COOKIE_GUILD, { zone: 1 });

      expect(s.unit("p1", 2)?.defId).toBe(GUY);
      expect(s.backrow("p1", 1)?.defId).toBe(SHEEPISH);
      expect(s.backrow("p1", 2)?.defId).toBe(MANA_WELL);
      expect(s.events.filter((event) => event.type === "destroyed")).toHaveLength(0);
    });

    it("with an empty backrow it destroys nothing", () => {
      const s = setup({}, { backrow: THEIR_BACKROW });

      s.play(GUY, { zone: 1 });

      expect(s.events.filter((event) => event.type === "destroyed")).toHaveLength(0);
    });
  });

  describe("radiant", () => {
    it("destroys every backrow card on both sides", () => {
      const s = setup({ backrow: MY_BACKROW }, { backrow: THEIR_BACKROW }, true);
      const all = [s.backrow("p1", 1)!, s.backrow("p1", 2)!, s.backrow("p2", 1)!, s.backrow("p2", 3)!];

      s.play(GUY, { zone: 1 });

      for (const card of all) s.expectInZone(card, "graveyard");
      s.expectStats(GUY, { attack: 12, health: 16 });
    });

    it("R97 the opponent's face-down trap reaches their graveyard openly: both players read it there", () => {
      const s = setup({}, { backrow: THEIR_BACKROW }, true);
      expect(JSON.stringify(s.view("p1").opponent.backrow)).not.toContain(MY_PAWN);

      s.play(GUY, { zone: 1 });

      expect(s.events.filter((event) => event.type === "trapFired")).toHaveLength(0);
      expect(s.view("p1").opponent.graveyard.map((card) => card.defId)).toContain(MY_PAWN);
      expect(s.view("p2").you.graveyard.map((card) => card.defId)).toContain(MY_PAWN);
    });

    it("§4.5 a destroyed backrow card that prints Death fires it: the opponent's Frostspatula resummons its kill", () => {
      const s = scenario({
        active: "p2",
        p1: { hand: [{ def: GUY, radiant: true }, FILLER], library: [STOCKPILE, STOCKPILE], field: [MR_TOKEN], mana: 8 },
        p2: { hand: [FROSTSPATULA, FILLER], library: [STOCKPILE, STOCKPILE], mana: 8 },
      });
      const victim = s.card(MR_TOKEN);
      s.play(FROSTSPATULA, { zone: 2 }); // R383: it animates into p2's unit zone 2 at once
      s.attack(FROSTSPATULA, victim); // its Rush reaches units: the 1/1 dies, remembered (R42)
      s.endTurn(); // it returns to p2's backrow zone 2 at their cleanup
      expect(s.backrow("p2", 2)?.defId).toBe(FROSTSPATULA);

      s.play(GUY, { zone: 2 });

      s.expectInZone(FROSTSPATULA, "graveyard");
      const copy = s.unit("p2", 1);
      expect(copy?.defId).toBe(MR_TOKEN);
      expect(copy?.id).not.toBe(victim.id);
      expect(copy?.controller).toBe("p2");
      // A summon, not a play: the copy's Cry (a Rush Token) never runs (R1).
      expect(s.unit("p2", 2)).toBeNull();
    });

    it("R46 an Indestructible backrow card on either side stays", () => {
      const s = setup({}, { backrow: [{ def: HEROIC_POWER, lane: 2 }, { def: MY_PAWN, faceUp: false, lane: 4 }] }, true);

      s.play(GUY, { zone: 1 });

      expect(s.backrow("p2", 2)?.defId).toBe(HEROIC_POWER);
      s.expectInZone(MY_PAWN, "graveyard");
    });
  });
});
