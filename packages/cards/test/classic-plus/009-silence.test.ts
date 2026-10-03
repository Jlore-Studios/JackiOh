// C+ #9 Silence — SPEC §8.7 row 9, BUILD M9 Classic+ row C+ 9: "Vanilla a target Unit on either side:
// printed keywords and text gone, stats, buffs, damage and granted keywords kept (§10.4); an Immutable
// Unit is unchanged (R23); radiant any permanent (R407): a Vanilla Field Spell's aura stops, a Vanilla
// face-down trap never fires and sits inert, an Animated card loses Animated where it stands and no
// longer moves; an enemy face-down trap is offered by id only and the Vanilla on it names nothing to the
// caster (R97, R177)".

import { legalActions } from "@jackioh/engine";
import type { Selection } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { scenario, type Scenario, type SideSetup } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic-plus/009-silence";

const SILENCE = "classicplus-009";
const MENACE = "core-019"; // (3) 9/9 Taunt. End of turn: heal to full. Radiant: Immutable.
const SURGERY = "core-063"; // (1) Give target Unit +3/+3 and 1 random keyword.
const VANILLA = "core-008"; // (1) 4/4.
const WEAPONS = "core-014"; // Field Spell: your Units have +4 attack, Rush and First Strike.
const SHEEPISH = "core-041"; // (1) Trap: transforms the opponent's played Unit.
const TESLA = "classic-005"; // Animated Field Trap: fires when the opponent summons a Unit.
const FROSTSPATULA = "classicplus-012-8"; // Field Spell token: Animated on your turn, Rush.
const FILLER = "core-010";
const STOCKPILE = "core-005";

function setup(p1: SideSetup = {}, p2: SideSetup = {}, radiantFace = false): Scenario {
  return scenario({
    p1: { hand: [{ def: SILENCE, radiant: radiantFace }, FILLER], library: [STOCKPILE, STOCKPILE], mana: 8, ...p1 },
    p2: { hand: [FILLER], library: [STOCKPILE, STOCKPILE], ...p2 },
  });
}

function at(id: string): Selection[] {
  return [{ pick: "instance", instanceId: id }];
}

function kinds(s: Scenario, id: string): string[] {
  return s.stats(id).keywords.map((k) => k.kind);
}

describe("C+ #9 Silence", () => {
  it("is a (0) Spell targeting a Unit (Radiant: a Unit or a backrow card), on either side", () => {
    expect([def.type, def.cost]).toEqual(["Spell", 0]);
    expect(base.targets).toEqual([{ kind: "target", min: 1, max: 1, filter: { side: "any", of: ["unit"] } }]);
    expect(radiant.targets).toEqual([{ kind: "target", min: 1, max: 1, filter: { side: "any", of: ["unit", "backrow"] } }]);
  });

  describe("base", () => {
    it("§10.4 an enemy Unit loses its printed keywords and text, and keeps its stats, buffs, damage and granted keywords", () => {
      const s = setup({ hand: [{ def: SILENCE }, SURGERY, FILLER] }, { field: [{ def: MENACE, damage: 3 }] });
      const menace = s.card(MENACE);
      s.play(SURGERY, { targets: at(menace.id) });
      const granted = s.card(menace).grantedKeywords.map((k) => k.kind);
      expect(granted).toHaveLength(1);

      s.play(SILENCE, { targets: at(menace.id) });

      expect(s.card(menace).vanilla).toBe(true);
      s.expectStats(menace, { attack: 12, maxHealth: 12, health: 9 });
      const now = kinds(s, menace.id);
      expect(now).toEqual(granted);
      expect(now).not.toContain("Taunt"); // R21 never grants a keyword the unit has, so Taunt was printed only
      s.expectEvents("transformed");
    });

    it("its text is gone: a Vanilla Midrange Menace no longer heals to full at end of turn", () => {
      const s = setup({ field: [{ def: MENACE, damage: 4 }] });
      const menace = s.card(MENACE);

      s.play(SILENCE, { targets: at(menace.id) });
      s.endTurn();

      s.expectStats(menace, { health: 5 });
    });

    it("reaches your own Unit too", () => {
      const s = setup({ field: [MENACE] });

      s.play(SILENCE, { targets: at(s.card(MENACE).id) });

      expect(kinds(s, MENACE)).not.toContain("Taunt");
    });

    it("R23 an Immutable Unit is unchanged", () => {
      const s = setup({}, { field: [{ def: MENACE, radiant: true }] });
      const menace = s.card(MENACE);

      s.play(SILENCE, { targets: at(menace.id) });

      expect(s.card(menace).vanilla).not.toBe(true);
      expect(kinds(s, menace.id)).toEqual(expect.arrayContaining(["Taunt", "Immutable"]));
      s.expectInZone(SILENCE, "graveyard");
    });

    it("R81 a backrow card is no target for the base face; with no Unit on the field it is played and fizzles (§8)", () => {
      const s = setup({ backrow: [WEAPONS] });
      expect(() => s.play(SILENCE, { targets: at(s.card(WEAPONS).id) })).toThrow();

      s.play(SILENCE);

      s.expectInZone(SILENCE, "graveyard");
      expect(s.card(WEAPONS).vanilla).not.toBe(true);
      expect(s.events.filter((event) => event.type === "transformed")).toHaveLength(0);
    });
  });

  describe("radiant", () => {
    it("R407 a Vanilla Field Spell's aura stops", () => {
      const s = setup({ field: [VANILLA], backrow: [WEAPONS] }, {}, true);
      s.expectStats(VANILLA, { attack: 8 });

      s.play(SILENCE, { targets: at(s.card(WEAPONS).id) });

      s.expectStats(VANILLA, { attack: 4 });
      expect(kinds(s, VANILLA)).not.toContain("Rush");
      expect(s.card(WEAPONS).vanilla).toBe(true);
    });

    it("R407 a Vanilla face-down trap never fires and sits inert in its zone, still face-down", () => {
      const s = setup({ hand: [{ def: SILENCE, radiant: true }, VANILLA, FILLER] }, { backrow: [{ def: SHEEPISH, faceUp: false, lane: 2 }] }, true);
      const trap = s.backrow("p2", 2)!;

      s.play(SILENCE, { targets: at(trap.id) });
      s.play(VANILLA, { zone: 1 });

      expect(s.events.filter((event) => event.type === "trapFired")).toHaveLength(0);
      expect(s.unit("p1", 1)?.defId).toBe(VANILLA);
      expect(s.card(trap).zone).toMatchObject({ z: "field", row: "backrow", lane: 2 });
      expect(s.card(trap).faceUp).toBe(false);
      expect(s.card(trap).vanilla).toBe(true);
    });

    it("R97 R177 an enemy face-down trap is offered by id only, and the Vanilla on it names nothing to the caster", () => {
      const s = setup({}, { backrow: [{ def: SHEEPISH, faceUp: false, lane: 2 }] }, true);
      const trap = s.backrow("p2", 2)!;
      const silence = s.card(SILENCE);
      const offers = legalActions(s.state, "p1").filter((action) => action.type === "play" && action.instanceId === silence.id);
      expect(JSON.stringify(offers)).toContain(trap.id);
      expect(JSON.stringify(offers)).not.toContain(SHEEPISH);

      s.play(SILENCE, { targets: at(trap.id) });

      expect(JSON.stringify(s.view("p1"))).not.toContain(SHEEPISH);
      expect(JSON.stringify(s.view("p2").you.backrow)).toContain(SHEEPISH);
    });

    it("R407 R383 a face-down Animated trap made Vanilla never fires, so never animates", () => {
      const s = setup({ hand: [{ def: SILENCE, radiant: true }, VANILLA, FILLER] }, { backrow: [{ def: TESLA, faceUp: false, lane: 3 }] }, true);
      const tesla = s.backrow("p2", 3)!;

      s.play(SILENCE, { targets: at(tesla.id) });
      s.play(VANILLA, { zone: 1 });

      expect(s.events.filter((event) => event.type === "trapFired" || event.type === "animated")).toHaveLength(0);
      s.expectStats(VANILLA, { health: 4 });
      expect(s.card(tesla).zone).toMatchObject({ row: "backrow", lane: 3 });
    });

    it("R407 R383 an 'Animated on your turn' card made Vanilla in its backrow zone no longer animates", () => {
      const s = setup({ backrow: [{ def: FROSTSPATULA, lane: 2 }] }, {}, true);
      const spatula = s.card(FROSTSPATULA);

      s.play(SILENCE, { targets: at(spatula.id) });
      s.endTurn();
      s.endTurn();

      expect(s.state.active).toBe("p1");
      expect(s.card(spatula).zone).toMatchObject({ z: "field", row: "backrow", lane: 2 });
      expect(s.events.filter((event) => event.type === "animated")).toHaveLength(0);
    });

    it("R407 R383 one made Vanilla while animated stays a Unit and no longer returns at cleanup", () => {
      const s = setup({ backrow: [{ def: FROSTSPATULA, lane: 2 }] }, {}, true);
      const spatula = s.card(FROSTSPATULA);
      s.startTurn();
      expect(s.unit("p1", 2)?.id).toBe(spatula.id);

      s.play(SILENCE, { targets: at(spatula.id) });
      s.endTurn();

      expect(s.card(spatula).zone).toMatchObject({ z: "field", row: "units", lane: 2 });
      expect(s.events.filter((event) => event.type === "deanimated")).toHaveLength(0);
    });
  });
});
