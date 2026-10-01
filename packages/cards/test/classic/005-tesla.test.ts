// C #5 Tesla — SPEC §8.6 row 5, BUILD M9 Classic row C 5: "Face-down, read by its controller only
// (R33); fires on every Unit summoned on the opponent's side however it is summoned (played, cast,
// token, Recruit, Reborn), while a Unit stolen across (R171) or an Animated card animating there (R383)
// was not summoned and leaves it set; a played one only after it resolves so its Cry happens first
// (R17's Bear Honeypot timing); your own Units never set it off; deals 4 to it with Tesla as the
// source, its Lifesteal healing you the amount dealt (0 into a Divine Shield); then animates (R383) in
// Defense Position into its own lane's unit zone, else the leftmost open, unlocked, unreserved one; no
// open zone → it stays face-up in its backrow zone and keeps firing; a Field Trap, it is never
// consumed; already a Unit when it fires again → it stays put in its position; animated, it is a Unit
// for every rule (attacked, damaged, counted among your Units, to its owner's graveyard when it dies)
// and still fires; the move keeps its damage and counters (R78 does not apply) and it is summoning
// sick; the opponent's view never names it before it fires, and `animated` names it after; radiant
// 2/8, 8 damage; its tuned number (damage) reads through `param()` (R386)".
//
// Tesla sits face-down in p1's backrow lane 3; p2, the opponent, is active and brings Units in.

import { describe, expect, it } from "vitest";
import { stepParam } from "@jackioh/engine";
import type { GameEvent } from "@jackioh/shared";
import { scenario, type Scenario, type SideSetup } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic/005-tesla";

const TESLA = "classic-005";
const VANILLA = "core-008"; // (1) 4/4.
const TIMMY = "core-011"; // (1) 3/3.
const MENACE = "core-019"; // (3) 9/9 Taunt.
const TOKEN_MAN = "core-015"; // (1) 1/1, Cry: Summon a Rush Token (3/3).
const CALL = "core-069"; // (2) Spell: Recruit 3 (1) Cost or less Units.
const DEFENDER = "core-003"; // (1) 1/1 Taunt, Divine Shield, Reborn.
const HIT_JOB = "core-016"; // (3) Destroy target Unit.
const FEELINGS = "classic-032"; // (0) Steal an enemy permanent in a lane where you control a (1) Cost Unit.
const FILLER = "core-010";

function setTesla(radiantFace = false, lane = 3): { def: string; radiant: boolean; faceUp: boolean; lane: number } {
  return { def: TESLA, radiant: radiantFace, faceUp: false, lane };
}

function setup(p2: SideSetup, p1: SideSetup = {}, radiantFace = false): Scenario {
  return scenario({
    active: "p2",
    p1: { hand: [FILLER], backrow: [setTesla(radiantFace)], health: 20, library: [VANILLA, VANILLA], ...p1 },
    p2: { hand: [FILLER], library: [VANILLA, VANILLA], ...p2 },
  });
}

function count(events: readonly GameEvent[], type: GameEvent["type"]): number {
  return events.filter((event) => event.type === type).length;
}

describe("C #5 Tesla", () => {
  it("is a Field Trap with Animated and Lifesteal printed; both faces run one script", () => {
    expect(def.type).toBe("Field Trap");
    expect(def.base.keywords.map((keyword) => keyword.kind)).toEqual(["Animated", "Lifesteal"]);
    expect(radiant).toBe(base);
  });

  describe("base", () => {
    it("R33 face-down, it is read by its controller only", () => {
      const s = setup({ hand: [VANILLA] });
      expect(JSON.stringify(s.view("p1").you.backrow)).toContain(TESLA);
      expect(JSON.stringify(s.view("p2"))).not.toContain(TESLA);
    });

    it("R17 a played Unit is answered after it resolves: 4 damage from Tesla, then Tesla animates in Defense", () => {
      const s = setup({ hand: [VANILLA, FILLER] });
      const vanilla = s.card(VANILLA);

      s.play(vanilla, { zone: 2 });

      s.expectInZone(vanilla, "graveyard");
      const tesla = s.card(TESLA);
      expect(tesla.zone).toMatchObject({ z: "field", row: "units", lane: 3 });
      expect(s.stats(tesla).position).toBe("DEF");
      s.expectEvents("cardPlayed", "cardResolved", "trapFired", "damage", "animated");
      const hit = s.events.find((event) => event.type === "damage");
      expect(hit).toMatchObject({ sourceId: tesla.id, amount: 4 });
    });

    it("§4.4 step 8 its Lifesteal heals you the amount dealt", () => {
      const s = setup({ hand: [VANILLA, FILLER] });

      s.play(VANILLA, { zone: 2 });

      s.expectHealth("p1", 24);
    });

    it("R17 the played Unit's Cry happens first: its token is answered, then the Unit itself", () => {
      const s = setup({ hand: [TOKEN_MAN, FILLER] });
      const man = s.card(TOKEN_MAN);

      s.play(man, { zone: 1 });

      const fired = s.events.filter((event) => event.type === "trapFired");
      expect(fired).toHaveLength(2);
      // The Cry's token is hit first (its summon), the played Unit only once the play has resolved.
      const token = s.events.find((event) => event.type === "summoned" && event.defId === "core-t-rush");
      if (token === undefined || token.type !== "summoned") throw new Error("no token was summoned");
      const hits = s.events.flatMap((event) => (event.type === "damage" ? [event.targetId] : []));
      expect(hits).toEqual([token.instanceId, man.id]);
      s.expectInZone(man, "graveyard");
      expect(s.unit("p2", 1)).toBeNull();
      expect(s.unit("p2", 2)).toBeNull();
      // Both hits heal: 4 into the 3/3 token and 4 into the 1/1.
      s.expectHealth("p1", 28);
    });

    it("§6.3 a Recruit's arrivals are summons too: each is answered", () => {
      const s = setup({ hand: [CALL, FILLER], library: [TIMMY, TIMMY, VANILLA, MENACE] });

      s.play(CALL);

      // Call to Arms recruits the three (1) Cost Units (Timmy, Timmy, Vanilla), never the (3) Menace.
      expect(count(s.events, "trapFired")).toBe(3);
      expect(s.pile("p2", "graveyard").filter((card) => card.defId === TIMMY)).toHaveLength(2);
      expect(s.pile("p2", "graveyard").filter((card) => card.defId === VANILLA)).toHaveLength(1);
    });

    it("§4.5 a Reborn body is summoned, and answered — on its controller's own turn too", () => {
      // On p1's turn p1 destroys p2's Right-house defender; Reborn brings it back on p2's side, which
      // is a Unit summoned there, so p1's Tesla answers it (its Divine Shield takes the hit).
      const s = scenario({
        p1: { hand: [HIT_JOB, FILLER], backrow: [setTesla()], health: 20, library: [VANILLA] },
        p2: { hand: [FILLER], field: [{ def: DEFENDER, lane: 2 }] },
      });
      const defender = s.card(DEFENDER);

      s.play(HIT_JOB, { targets: [{ pick: "instance", instanceId: defender.id }] });

      s.expectInZone(defender, "field");
      expect(count(s.events, "trapFired")).toBe(1);
      expect(count(s.events, "animated")).toBe(1);
      s.expectEvents("destroyed", "summoned", "trapFired", "divineShieldLost");
    });

    it("§4.4 a Divine Shield takes the hit: 0 dealt, so its Lifesteal heals 0", () => {
      const s = setup({ hand: [DEFENDER, FILLER] });

      s.play(DEFENDER, { zone: 1 });

      s.expectInZone(DEFENDER, "field");
      s.expectEvents("trapFired", "divineShieldLost");
      s.expectHealth("p1", 20);
    });

    it("your own Units never set it off", () => {
      const s = scenario({
        p1: { hand: [VANILLA, FILLER], backrow: [setTesla()] },
        p2: { hand: [FILLER] },
      });

      s.play(VANILLA, { zone: 1 });

      expect(count(s.events, "trapFired")).toBe(0);
      expect(s.card(TESLA).faceUp).toBe(false);
    });

    it("R171 a Unit stolen across was not summoned and leaves it set", () => {
      const s = setup({ hand: [FEELINGS, FILLER], field: [{ def: TIMMY, lane: 4 }] }, { field: [{ def: MENACE, lane: 4 }] });
      const menace = s.card(MENACE);

      s.play(FEELINGS, { targets: [{ pick: "instance", instanceId: menace.id }] });

      expect(s.card(menace).controller).toBe("p2");
      expect(count(s.events, "trapFired")).toBe(0);
    });

    it("R383 an Animated card animating on the opponent's side was not summoned and leaves it set", () => {
      // p1 plays a Unit on p1's turn: p2's own Tesla answers it and animates on p2's side. That move
      // is no summon, so p1's Tesla stays face-down.
      const s = scenario({
        p1: { hand: [VANILLA, FILLER], backrow: [setTesla()], library: [VANILLA] },
        p2: { hand: [FILLER], backrow: [{ def: TESLA, faceUp: false, lane: 3 }], library: [VANILLA] },
      });
      const mine = s.backrow("p1", 3);
      const theirs = s.backrow("p2", 3);
      if (mine === null || theirs === null) throw new Error("fixture");

      s.play(VANILLA, { zone: 1 });

      expect(s.card(theirs).zone).toMatchObject({ z: "field", row: "units", player: "p2" });
      expect(count(s.events, "animated")).toBe(1);
      expect(count(s.events, "trapFired")).toBe(1);
      expect(s.card(mine).faceUp).toBe(false);
      expect(s.card(mine).zone).toMatchObject({ row: "backrow", lane: 3 });
    });

    it("R383 no open unit zone: it stays face-up in its backrow zone, and keeps firing", () => {
      const full = [VANILLA, VANILLA, VANILLA, VANILLA, VANILLA];
      const s = setup({ hand: [TIMMY, TIMMY, FILLER] }, { field: full });
      const tesla = s.card(TESLA);

      s.play(TIMMY, { zone: 1 });
      expect(s.card(tesla).zone).toMatchObject({ z: "field", row: "backrow", lane: 3 });
      expect(s.card(tesla).faceUp).toBe(true);

      const second = s.hand("p2").find((card) => card.defId === TIMMY);
      if (second === undefined) throw new Error("fixture");
      s.play(second, { zone: 2 });

      expect(count(s.events, "trapFired")).toBe(2);
    });

    it("R383 its own lane's unit zone taken: it animates into the leftmost open one", () => {
      const s = setup({ hand: [VANILLA, FILLER] }, { field: [{ def: TIMMY, lane: 3 }, { def: TIMMY, lane: 1 }] });

      s.play(VANILLA, { zone: 2 });

      expect(s.card(TESLA).zone).toMatchObject({ z: "field", row: "units", lane: 2 });
    });

    it("never consumed; already a Unit, it fires again and stays put, still in Defense", () => {
      const s = setup({ hand: [VANILLA, TIMMY, FILLER] });

      s.play(VANILLA, { zone: 1 });
      s.play(TIMMY, { zone: 2 });

      const tesla = s.card(TESLA);
      expect(tesla.zone).toMatchObject({ z: "field", row: "units", lane: 3 });
      expect(s.stats(tesla).position).toBe("DEF");
      expect(count(s.events, "trapFired")).toBe(2);
      expect(count(s.events, "animated")).toBe(1);
      s.expectInZone(TIMMY, "graveyard");
    });

    it("R383 animated it is a Unit for every rule: it can be attacked and dies to its owner's graveyard", () => {
      const s = setup({ hand: [VANILLA, FILLER], field: [{ def: MENACE, lane: 1 }] });

      s.play(VANILLA, { zone: 2 });
      const tesla = s.card(TESLA);
      s.attack(MENACE, tesla);

      s.expectInZone(tesla, "graveyard");
      expect(s.pile("p1", "graveyard").map((card) => card.id)).toContain(tesla.id);
    });

    it("R383 the move keeps nothing reset: an animated Tesla is summoning sick on its first turn as a Unit", () => {
      const s = setup({ hand: [VANILLA, FILLER] });
      s.play(VANILLA, { zone: 2 });
      // p1's own turn comes: Tesla entered the unit row on p2's turn, so it may attack now — but the
      // turn it animated on, a p1 attack would have been refused.
      const tesla = s.card(TESLA);
      expect(s.card(tesla).summonedTurn).toBe(s.state.turn);
    });

    it("R383 moving is not leaving the field: its counters ride from the backrow into the unit row", () => {
      const s = scenario({
        active: "p2",
        p1: { hand: [FILLER], backrow: [{ ...setTesla(), counters: { plague: 2 } }], library: [VANILLA] },
        p2: { hand: [VANILLA, FILLER], library: [VANILLA] },
      });

      s.play(VANILLA, { zone: 2 });

      const tesla = s.card(TESLA);
      expect(tesla.zone).toMatchObject({ row: "units" });
      expect(tesla.counters.plague).toBe(2);
    });

    it("R97 the opponent's view never named it before it fired; `animated` names it after", () => {
      const s = setup({ hand: [VANILLA, FILLER] });
      expect(JSON.stringify(s.view("p2"))).not.toContain(TESLA);

      s.play(VANILLA, { zone: 2 });

      const theirs = s.view("p2");
      const animated = (theirs.events ?? []).find((event) => event.type === "animated");
      expect(JSON.stringify(animated)).toContain(TESLA);
    });

    it("R386 an Upgrade of its damage deals 5", () => {
      const s = setup({ hand: [MENACE, FILLER] });
      stepParam(s.card(TESLA), "damage", 1);

      s.play(MENACE, { zone: 2 });

      s.expectStats(MENACE, { health: 4 });
      s.expectHealth("p1", 25);
    });
  });

  describe("radiant", () => {
    it("R275 its unit face is a 2/8", () => {
      const s = setup({ hand: [VANILLA, FILLER] }, {}, true);

      s.play(VANILLA, { zone: 2 });

      s.expectStats(TESLA, { attack: 2, health: 8, maxHealth: 8 });
    });

    it("deals 8 and heals 8", () => {
      const s = setup({ hand: [MENACE, FILLER] }, {}, true);

      s.play(MENACE, { zone: 2 });

      s.expectStats(MENACE, { health: 1 });
      s.expectHealth("p1", 28);
    });
  });
});
