// C #59 Plague Doctor — SPEC §8.6 row 59, BUILD M9 Classic row C 59: "Cry: one hit of N on a declared
// target (Unit or hero), N = every Plague Counter on the field, both sides and face-down cards included,
// counted as it resolves; N = 0 → no hit (R63); its preview is N (R280); radiant 4/6: first place 2
// tokens on itself, then count them too (its preview includes them); its tuned number (radiant tokens)
// reads through `param()` (R386)".
//
// The preview's proofs — its value on both faces against what the Cry then deals, its label in each
// face's text, and that it reads only public facts — are in `test/preview.test.ts`.

import { legalActions, stepParam, type CardInstance } from "@jackioh/engine";
import type { ActionBody, Selection } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { scenario, type Scenario } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic/059-plague-doctor";

const DOCTOR = "classic-059";
const VANILLA = "core-008"; // (1) Unit 4/4.
const MENACE = "core-019"; // (3) Unit 9/9 Taunt.
const FIENDER = "core-092"; // (2) Unit 5/7 Stack.
const PAWN = "core-096"; // (1) Trap: answers only an attack that would be lethal.
const MANA_WELL = "core-006"; // (3) Field Spell.
const ANCHOR = "core-010"; // (0) Spell (§2.5).

type Play = Extract<ActionBody, { type: "play" }>;

function at(card: CardInstance): Selection[] {
  return [{ pick: "instance", instanceId: card.id }];
}

const ENEMY_HERO: Selection[] = [{ pick: "hero", player: "p2" }];

function hits(s: Scenario, targetId: string): number[] {
  return s.events.flatMap((event) => (event.type === "damage" && event.targetId === targetId ? [event.amount] : []));
}

function allHits(s: Scenario): number {
  return s.events.filter((event) => event.type === "damage").length;
}

describe("C #59 Plague Doctor", () => {
  it("declares one target, a Unit or a hero on either side, its one number, and one script on both faces", () => {
    expect(def.id).toBe(DOCTOR);
    expect(base.targets).toEqual([{ kind: "target", min: 1, max: 1, filter: { side: "any", of: ["unit", "hero"] } }]);
    expect(def.params).toEqual([{ key: "tokens", base: 2, radiant: 2, better: "up", step: 1, min: 1 }]);
    expect(radiant).toBe(base);
  });

  describe("base", () => {
    it("is a 2/3; its Cry deals one hit of N, every Plague Counter on both sides, face-down cards included", () => {
      const s = scenario({
        p1: { hand: [DOCTOR, ANCHOR], field: [{ def: VANILLA, counters: { plague: 2 } }], backrow: [{ def: MANA_WELL, counters: { plague: 1 } }] },
        p2: { hand: [ANCHOR], field: [{ def: MENACE, counters: { plague: 1 } }], backrow: [{ def: PAWN, faceUp: false, counters: { plague: 3 } }] },
      });
      const menace = s.card(MENACE);

      s.play(DOCTOR, { targets: at(menace) });

      s.expectStats(DOCTOR, { attack: 2, health: 3 });
      expect(hits(s, menace.id)).toEqual([7]);
      s.expectStats(menace, { health: 2 });
      // Nothing places or removes a token.
      expect(s.events.filter((event) => event.type === "counterChanged")).toEqual([]);
    });

    it("R81 a hero is a target too: the enemy hero, or your own", () => {
      const s = scenario({ p1: { hand: [DOCTOR, DOCTOR, ANCHOR], field: [{ def: VANILLA, counters: { plague: 3 } }], health: 20 }, p2: { hand: [ANCHOR], health: 20 } });
      const [first, second] = s.hand("p1").filter((card) => card.defId === DOCTOR);
      if (first === undefined || second === undefined) throw new Error("two Doctors in hand");

      s.play(first, { targets: ENEMY_HERO });
      s.play(second, { targets: [{ pick: "hero", player: "p1" }] });

      s.expectHealth("p2", 17);
      s.expectHealth("p1", 17);
    });

    it("R81 legalActions offers every Unit and both heroes, and nothing in a backrow", () => {
      const s = scenario({ p1: { hand: [DOCTOR, ANCHOR], field: [VANILLA] }, p2: { hand: [ANCHOR], field: [MENACE], backrow: [MANA_WELL] } });
      const doctor = s.card(DOCTOR);
      const offered = new Set(
        legalActions(s.state, "p1")
          .filter((action): action is Play => action.type === "play" && action.instanceId === doctor.id)
          .flatMap((play) => (play.targets ?? []).map((target) => (target.pick === "instance" ? target.instanceId : target.pick === "hero" ? `hero-${target.player}` : "?"))),
      );

      expect(offered).toEqual(new Set([s.card(VANILLA).id, s.card(MENACE).id, "hero-p1", "hero-p2"]));
      expect(() => s.play(doctor, { targets: at(s.card(MANA_WELL)) })).toThrow();
    });

    it("R63 N = 0 is no hit: with no Plague Counter on the field nothing is dealt", () => {
      const s = scenario({ p1: { hand: [DOCTOR, ANCHOR] }, p2: { hand: [ANCHOR], field: [MENACE], health: 20 } });

      s.play(DOCTOR, { targets: ENEMY_HERO });

      s.expectInZone(DOCTOR, "field");
      expect(allHits(s)).toBe(0);
      s.expectHealth("p2", 20);
    });

    it("§3.2 R13 a card dormant under a Stack pile is not on the field: its tokens are not counted", () => {
      const s = scenario({
        p1: { hand: [DOCTOR, ANCHOR] },
        p2: { hand: [ANCHOR], field: [{ def: VANILLA, counters: { plague: 4 } }, { def: FIENDER, stack: true, counters: { plague: 1 } }], health: 20 },
      });

      s.play(DOCTOR, { targets: ENEMY_HERO });

      s.expectHealth("p2", 19);
    });

    it("counted as it resolves: the tokens a C #53 Plague Crawler placed earlier this turn count", () => {
      const s = scenario({ p1: { hand: ["classic-053", DOCTOR, ANCHOR] }, p2: { hand: [ANCHOR], field: [MENACE], health: 20 } });
      expect(s.state.players.p2.hero.health).toBe(20);
      s.play("classic-053", { targets: at(s.card(MENACE)) });

      s.play(DOCTOR, { targets: ENEMY_HERO });

      s.expectHealth("p2", 19);
    });

    it("R177 the tokens on a face-down enemy trap count, and the hit never names the trap to you", () => {
      const s = scenario({ p1: { hand: [DOCTOR, ANCHOR] }, p2: { hand: [ANCHOR], backrow: [{ def: PAWN, faceUp: false, counters: { plague: 2 } }], health: 20 } });

      s.play(DOCTOR, { targets: ENEMY_HERO });

      s.expectHealth("p2", 18);
      expect(JSON.stringify(s.view("p1"))).not.toContain(PAWN);
    });

    it("R386 the base face never reads its tokens: an Upgrade of them changes nothing", () => {
      const s = scenario({ p1: { hand: [DOCTOR, ANCHOR], field: [{ def: VANILLA, counters: { plague: 1 } }] }, p2: { hand: [ANCHOR], health: 20 } });
      stepParam(s.card(DOCTOR), "tokens", 1);

      s.play(DOCTOR, { targets: ENEMY_HERO });

      s.expectHealth("p2", 19);
      expect(s.card(DOCTOR).counters.plague).toBeUndefined();
    });
  });

  describe("radiant", () => {
    it("is a 4/6; its Cry first places 2 Plague Counters on itself, then deals N counting them", () => {
      const s = scenario({ p1: { hand: [{ def: DOCTOR, radiant: true }, ANCHOR] }, p2: { hand: [ANCHOR], field: [{ def: MENACE, counters: { plague: 1 } }] } });
      const menace = s.card(MENACE);

      s.play(DOCTOR, { targets: at(menace) });

      s.expectStats(DOCTOR, { attack: 4, health: 6 });
      expect(s.card(DOCTOR).counters.plague).toBe(2);
      expect(hits(s, menace.id)).toEqual([3]);
      // The placement comes first, then the hit.
      const order = s.events.flatMap((event) => (event.type === "counterChanged" || event.type === "damage" ? [event.type] : []));
      expect(order).toEqual(["counterChanged", "damage"]);
    });

    it("with no other token on the field it still deals 2: its own", () => {
      const s = scenario({ p1: { hand: [{ def: DOCTOR, radiant: true }, ANCHOR] }, p2: { hand: [ANCHOR], health: 20 } });

      s.play(DOCTOR, { targets: ENEMY_HERO });

      s.expectHealth("p2", 18);
    });

    it("its placement on itself is one placement: a placement trigger on it answers once", () => {
      const s = scenario({ p1: { hand: [{ def: DOCTOR, radiant: true }, ANCHOR] }, p2: { hand: [ANCHOR], health: 20 } });

      s.play(DOCTOR, { targets: ENEMY_HERO });

      const doctor = s.card(DOCTOR);
      expect(
        s.events.filter((event) => event.type === "counterChanged" && event.instanceId === doctor.id && event.placed !== undefined),
      ).toEqual([{ type: "counterChanged", instanceId: doctor.id, counter: "plague", value: 2, placed: 2 }]);
    });

    it("R386 an Upgrade places 3 on itself and deals 3 more than the field held; a Degrade places 1", () => {
      const s = scenario({
        p1: { hand: [{ def: DOCTOR, radiant: true }, { def: DOCTOR, radiant: true }, ANCHOR], field: [{ def: VANILLA, counters: { plague: 1 } }] },
        p2: { hand: [ANCHOR], health: 20 },
      });
      const [up, down] = s.hand("p1").filter((card) => card.defId === DOCTOR);
      if (up === undefined || down === undefined) throw new Error("two Doctors in hand");
      stepParam(up, "tokens", 1);
      stepParam(down, "tokens", -1);

      s.play(up, { targets: ENEMY_HERO });
      expect(s.card(up).counters.plague).toBe(3);
      s.expectHealth("p2", 16);

      // 1 + 3 on the field, then 1 more of its own.
      s.play(down, { targets: ENEMY_HERO });
      expect(s.card(down).counters.plague).toBe(1);
      s.expectHealth("p2", 11);
    });
  });
});
