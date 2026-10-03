// C #21 Turtinator — SPEC §8.6 row 21, BUILD M9 Classic row C 21: "Activate ♾️ (R384): its cost,
// Tributing one of your Units (a pick carried in the action, Turtinator itself allowed), is paid as it
// activates, so with no Unit it can't activate; then one hit from Turtinator on a declared target equal
// to that Unit's attack as it stood (last-known, R78), the hit still coming when it tributed itself;
// 0 attack → no hit (R63); a Sheep or C #82 is one Unit here (a script's Tribute, §6.3); the Tribute is
// a death (Death fires); it needs no zone (R391); at most `ACTIVATE_UNLIMITED_CAP` uses per turn; not a
// play; radiant 10/8: twice that attack; its tuned number (multiplier) reads through `param()` (R386)".
//
// Turtinator is always one of its controller's Units while it can be activated, so "no Unit" never
// arises on the field; what the cost refuses is an activation that names no Tribute, or a Tribute that
// is not one of its controller's own Units.

import { describe, expect, it } from "vitest";
import { ACTIVATE_UNLIMITED_CAP, legalActions, stepParam, subsystems } from "@jackioh/engine";
import type { PlayerId, Selection } from "@jackioh/shared";
import { scenario, type Scenario, type SideSetup } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic/021-turtinator";

const TURTLE = "classic-021";
const FILLER = "core-005"; // (1) Spell, a card to keep a hand from auto-ending (§2.5).
const VANILLA = "core-008"; // 4/4.
const TIMMY = "core-011"; // 3/3.
const POINTMASTER = "core-020"; // 7/1.
const D_FENDER = "core-001"; // 0/7.
const SAINTESS = "core-081"; // 2/2, "Death: Make your other Units Radiant."
const WEAPONS = "core-014"; // Field Spell, "Aura: Your Units have +4 attack, Rush and First Strike."
const SHEEP = "core-t-sheep"; // 1/1, worth 2 Tributes.
const ROCK = "core-066"; // 10/10 Indestructible.

const ENEMY_HERO: readonly Selection[] = [{ pick: "hero", player: "p2" }];

function setup(p1: SideSetup, p2: SideSetup = {}): Scenario {
  return scenario({
    p1: { hand: [FILLER], library: [FILLER], ...p1 },
    p2: { hand: [FILLER], library: [FILLER], ...p2 },
  });
}

function eat(s: Scenario, tribute: string, targets: readonly Selection[] = ENEMY_HERO, who = TURTLE): Scenario {
  return s.activate(who, { tributes: [tribute], targets });
}

function damageEvents(s: Scenario): unknown[] {
  return s.lastEvents.filter((event) => event.type === "damage");
}

function activationsOf(s: Scenario, player: PlayerId, instanceId: string): unknown[] {
  return legalActions(s.state, player).filter((action) => action.type === "activate" && action.instanceId === instanceId);
}

describe("C #21 Turtinator", () => {
  it("R384 declares one Activate ♾️ whose cost is one Tribute, with a declared unit-or-hero target", () => {
    expect(def.id).toBe(TURTLE);
    expect(radiant).toBe(base);
    const ability = base.activations?.[0];
    expect(ability?.uses).toBe("unlimited");
    expect(ability?.cost).toEqual({ tribute: 1, tributeExcludesSelf: true });
    expect(ability?.targets?.[0]?.filter).toEqual({ of: ["unit", "hero"] });
  });

  describe("base", () => {
    it("R384 Tributes one of your Units as it activates, then deals that Unit's attack to the declared target", () => {
      const s = setup({ field: [TURTLE, POINTMASTER] });
      const point = s.card(POINTMASTER);

      eat(s, POINTMASTER);

      s.expectInZone(point, "graveyard");
      s.expectHealth("p2", 23);
      s.expectEvents("activated", "destroyed", "damage");
    });

    it("the hit's source is Turtinator", () => {
      const s = setup({ field: [TURTLE, POINTMASTER] });
      const turtle = s.card(TURTLE);

      eat(s, POINTMASTER);

      const hit = s.lastEvents.find((event) => event.type === "damage");
      expect(hit).toMatchObject({ sourceId: turtle.id, amount: 7 });
    });

    it("R78 the amount is the tributed Unit's attack as it stood, an aura's bonus included", () => {
      const s = setup({ field: [TURTLE, TIMMY], backrow: [WEAPONS] });
      s.expectStats(TIMMY, { attack: 7 });

      eat(s, TIMMY);

      // Timmy is 3 in the graveyard; the aura's +4 counted, because it was paid as it stood.
      s.expectHealth("p2", 23);
    });

    it("R642 Turtinator cannot Tribute itself: alone on its side, no activation is listed", () => {
      const s = setup({ field: [TURTLE] });
      const turtle = s.card(TURTLE);

      expect(activationsOf(s, "p1", turtle.id)).toHaveLength(0);
      expect(() => eat(s, TURTLE)).toThrow();

      s.expectInZone(turtle, "field");
      s.expectHealth("p2", 30);
    });

    it("R384 the Tribute is its cost: an activation that names none is refused and changes nothing", () => {
      const s = setup({ field: [TURTLE, POINTMASTER] });

      expect(() => s.activate(TURTLE, { targets: ENEMY_HERO, tributes: [] })).toThrow();

      s.expectInZone(POINTMASTER, "field");
      s.expectHealth("p2", 30);
    });

    it("R384 an enemy Unit is no Tribute for it", () => {
      const s = setup({ field: [TURTLE] }, { field: [POINTMASTER] });

      expect(() => eat(s, POINTMASTER)).toThrow();

      s.expectInZone(POINTMASTER, "field");
      s.expectHealth("p2", 30);
    });

    it("R384 one Unit is one Tribute: two named are refused", () => {
      const s = setup({ field: [TURTLE, TIMMY, POINTMASTER] });

      expect(() => s.activate(TURTLE, { tributes: [TIMMY, POINTMASTER], targets: ENEMY_HERO })).toThrow();
    });

    it("§6.3 a Sheep Token is one Unit here, not two Tributes: it pays the cost and deals its 1", () => {
      const s = setup({ field: [TURTLE, SHEEP] });

      eat(s, SHEEP);

      s.expectHealth("p2", 29);
      expect(s.unit("p1", 2)).toBeNull();
    });

    it("§6.3 a Tribute bypasses Indestructible: The Rock is tributed and deals its 10", () => {
      const s = setup({ field: [TURTLE, ROCK] });
      const rock = s.card(ROCK);

      eat(s, ROCK);

      s.expectInZone(rock, "graveyard");
      s.expectHealth("p2", 20);
    });

    it("R13 a Unit dormant under a Stack pile is no Tribute for it", () => {
      const s = setup({ field: [TURTLE, { def: TIMMY, lane: 2 }, { def: POINTMASTER, stack: true }] });
      const dormant = s.card(TIMMY);

      expect(() => s.activate(TURTLE, { tributes: [dormant.id], targets: ENEMY_HERO })).toThrow();

      s.expectInZone(dormant, "field");
      s.expectHealth("p2", 30);
    });

    it("R63 a Unit with 0 attack pays the cost but deals no hit at all", () => {
      const s = setup({ field: [TURTLE, D_FENDER] });

      eat(s, D_FENDER);

      s.expectInZone(D_FENDER, "graveyard");
      s.expectHealth("p2", 30);
      expect(damageEvents(s)).toHaveLength(0);
    });

    it("§6.3 the Tribute is a death: the tributed Unit's Death fires", () => {
      const s = setup({ field: [TURTLE, SAINTESS, VANILLA] });
      const vanilla = s.card(VANILLA);

      eat(s, SAINTESS);

      s.expectEvents("destroyed", "radiantSet");
      expect(s.card(vanilla).radiant).toBe(true);
    });

    it("targets any Unit either side, or a hero", () => {
      const s = setup({ field: [TURTLE, POINTMASTER, VANILLA] }, { field: ["core-019"] });
      const menace = s.card("core-019");

      eat(s, POINTMASTER, [{ pick: "instance", instanceId: menace.id }]);
      s.expectStats(menace, { health: 2 });

      eat(s, VANILLA, [{ pick: "hero", player: "p1" }]);
      s.expectHealth("p1", 26);
    });

    it("R391 needs no zone: it activates with its controller's unit row full", () => {
      const s = setup({ field: [TURTLE, TIMMY, VANILLA, POINTMASTER, D_FENDER] });

      eat(s, TIMMY);

      s.expectHealth("p2", 27);
    });

    it("R384 Activate ♾️: many uses in one turn, each paying its own Tribute", () => {
      const s = setup({ field: [TURTLE, TIMMY, VANILLA, POINTMASTER] });

      eat(s, TIMMY);
      eat(s, VANILLA);
      eat(s, POINTMASTER);

      s.expectHealth("p2", 30 - 3 - 4 - 7);
      // R642: with only itself left to Tribute, no further activation is listed.
      expect(activationsOf(s, "p1", s.card(TURTLE).id)).toHaveLength(0);
    });

    it("R384 stops at ACTIVATE_UNLIMITED_CAP uses in a turn", () => {
      const s = setup({ field: [TURTLE, TIMMY] });
      const turtle = s.card(TURTLE);
      turtle.memory[subsystems.ACTIVATIONS_MEMORY_KEY] = { turn: s.state.turn, count: ACTIVATE_UNLIMITED_CAP };

      expect(activationsOf(s, "p1", turtle.id)).toHaveLength(0);
      expect(() => eat(s, TIMMY)).toThrow();
      s.expectInZone(TIMMY, "field");
    });

    it("R384 activating is not a play: no cardPlayed and no play counted", () => {
      const s = setup({ field: [TURTLE, TIMMY] });
      const played = s.state.players.p1.turnLog.cardsPlayed;

      eat(s, TIMMY);

      expect(s.events.some((event) => event.type === "cardPlayed")).toBe(false);
      expect(s.state.players.p1.turnLog.cardsPlayed).toBe(played);
    });

    it("R386 an Upgrade of its multiplier doubles the hit; a Degrade never takes it below 1", () => {
      const up = setup({ field: [TURTLE, POINTMASTER] });
      stepParam(up.card(TURTLE), "multiplier", 1);
      eat(up, POINTMASTER);
      up.expectHealth("p2", 16);

      const down = setup({ field: [TURTLE, POINTMASTER] });
      stepParam(down.card(TURTLE), "multiplier", -1);
      eat(down, POINTMASTER);
      down.expectHealth("p2", 23);
    });
  });

  describe("radiant", () => {
    it("R275 the Radiant face is a 10/8", () => {
      const s = setup({ field: [{ def: TURTLE, radiant: true }] });
      s.expectStats(TURTLE, { attack: 10, health: 8, maxHealth: 8 });
    });

    it("deals twice the tributed Unit's attack", () => {
      const s = setup({ field: [{ def: TURTLE, radiant: true }, POINTMASTER] });

      eat(s, POINTMASTER);

      s.expectHealth("p2", 16);
    });

    it("R642 the Radiant face cannot Tribute itself either", () => {
      const s = setup({ field: [{ def: TURTLE, radiant: true }] }, { health: 30 });
      const turtle = s.card(TURTLE);

      expect(activationsOf(s, "p1", turtle.id)).toHaveLength(0);
      expect(() => eat(s, TURTLE)).toThrow();

      s.expectInZone(turtle, "field");
      s.expectHealth("p2", 30);
    });

    it("R386 an Upgrade takes the Radiant multiplier to 3", () => {
      const s = setup({ field: [{ def: TURTLE, radiant: true }, TIMMY] });
      stepParam(s.card(TURTLE), "multiplier", 1);

      eat(s, TIMMY);

      s.expectHealth("p2", 21);
    });
  });
});
