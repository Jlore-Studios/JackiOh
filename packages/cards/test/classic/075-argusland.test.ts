// C #75 Argusland — SPEC §8.6 row 75, BUILD M9 Classic row C 75: "Aura: damage to your hero is
// halved, rounded up, after Armor and before the hit caps (§4.4 step 2), so a 5 becomes 3 and a 1
// stays 1, fatigue included; losing health (R18) and damage to Units are untouched; several
// multiply; the lethal window reads the halved amount (§4.4 step 4a); gone when it leaves; radiant:
// quartered, rounded up; its tuned number (divisor) reads through `param()` (R386)".
//
// The hits come from Core cards with their own tests: attacks by Gary the Gambler (1/1), Mr. Vanilla
// (4/4), Prem Panther (5/4), Pointmaster (7/1) and Midrange Menace (9/9); Lunar Eclipse's 3 damage to
// a target; Stockpile's "Draw 2" into an empty deck for fatigue; Blood Ridden Glowy Jelly Bean's
// "lose 5 health" cast on draw. Going Long gives the hero Armor 2, Anti-oneshot Armor caps a hit at
// 5, Magic Jammed destroys a backrow card, and My Pawn (R44) and C #52 Final Gambit read the lethal
// projection and the lethal window.

import { stepParam } from "@jackioh/engine";
import type { GameEvent, PlayerId } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { scenario, type Scenario } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic/075-argusland";

const ARGUSLAND = "classic-075";
const GAMBIT = "classic-052"; // C #52 Final Gambit: redirects a lethal hit on your hero.
const GARY = "core-004"; // 1/1
const VANILLA = "core-008"; // 4/4
const PANTHER = "core-032"; // 5/4 Rush
const POINTMASTER = "core-020"; // 7/1 First Strike
const MENACE = "core-019"; // 9/9 Taunt
const LUNAR = "core-035"; // (1) Spell: deal 3 damage to a target.
const STOCKPILE = "core-005"; // (1) Spell: Draw 2. Heal your hero 2.
const BLOOD_BEAN = "core-027"; // Cast on draw: make a random hand card Radiant. Lose 5 health.
const GOING_LONG = "core-084"; // Field Spell: your hero has Armor 2.
const ANTI_ONESHOT = "core-073"; // Field Spell: your hero can't take more than 5 damage at once.
const MAGIC_JAMMED = "core-036"; // (1) Spell: destroy target backrow card; lock its zone.
const MIND_CONTROL = "core-049"; // (4) Spell: steal target enemy permanent.
const MY_PAWN = "core-096"; // Trap: cancels a declared attack that would be lethal to your hero (R44).
const FILLER = "core-005";

function heroHits(s: Scenario, player: PlayerId): number[] {
  return s.events.flatMap((event: GameEvent) =>
    event.type === "damage" && event.targetId === `hero-${player}` ? [event.amount] : [],
  );
}

/** p2 attacks p1's hero with `attacker`, p1 guarded by the backrow it is given. */
function attackOnGuarded(attacker: string, backrow: readonly (string | { def: string; radiant?: boolean })[], health = 30): Scenario {
  const s = scenario({
    p1: { hand: [FILLER], backrow, health, library: [FILLER, FILLER, FILLER] },
    p2: { hand: [FILLER], field: [attacker] },
    active: "p2",
  });
  s.attack(attacker, "hero");
  return s;
}

describe("C #75 Argusland", () => {
  it("runs one script on both faces: a hero guard whose divisor is the declared number", () => {
    expect(def.id).toBe(ARGUSLAND);
    expect(def.type).toBe("Field Spell");
    expect(def.params).toEqual([{ key: "divisor", base: 2, radiant: 4, better: "up", step: 1, min: 2 }]);
    expect(radiant).toBe(base);
    expect(base.heroGuard).toBeDefined();
  });

  describe("base", () => {
    it("halves a hit on your hero, rounded up: a 5 becomes 3", () => {
      const s = attackOnGuarded(PANTHER, [ARGUSLAND]);
      expect(heroHits(s, "p1")).toEqual([3]);
      s.expectHealth("p1", 27);
    });

    it("rounds up, so a 1 stays 1", () => {
      const s = attackOnGuarded(GARY, [ARGUSLAND]);
      expect(heroHits(s, "p1")).toEqual([1]);
      s.expectHealth("p1", 29);
    });

    it("a Spell's damage to your hero is halved too: Lunar Eclipse's 3 becomes 2", () => {
      const s = scenario({
        p1: { hand: [FILLER], backrow: [ARGUSLAND] },
        p2: { hand: [LUNAR, FILLER] },
        active: "p2",
      });
      s.play(LUNAR, { targets: [{ pick: "hero", player: "p1" }] });
      expect(heroHits(s, "p1")).toEqual([2]);
      s.expectHealth("p1", 28);
    });

    it("§4.4 step 2: it divides after Armor — Going Long's 2 off a 7 leaves 5, halved to 3", () => {
      const s = attackOnGuarded(POINTMASTER, [ARGUSLAND, GOING_LONG]);
      expect(heroHits(s, "p1")).toEqual([3]);
      s.expectHealth("p1", 27);
    });

    it("§4.4 step 3: it divides before the hit caps — a 9 halved to 5 meets Anti-oneshot Armor's 5", () => {
      const s = attackOnGuarded(MENACE, [ARGUSLAND, ANTI_ONESHOT]);
      // Capped first it would be 5 halved to 3.
      expect(heroHits(s, "p1")).toEqual([5]);
      s.expectHealth("p1", 25);
    });

    it("R125 fatigue is damage and is halved: the 1st and 2nd empty draws deal 1 and 1, not 1 and 2", () => {
      const s = scenario({ p1: { hand: [STOCKPILE, FILLER], backrow: [ARGUSLAND], health: 20 }, p2: { hand: [FILLER] } });
      s.play(STOCKPILE);
      expect(heroHits(s, "p1")).toEqual([1, 1]);
      s.expectHealth("p1", 20);
    });

    it("R18 losing health is not damage: Blood Ridden Glowy Jelly Bean's 5 is lost whole", () => {
      const s = scenario({
        p1: { hand: [STOCKPILE, FILLER], backrow: [ARGUSLAND], library: [BLOOD_BEAN, FILLER, FILLER], health: 20 },
        p2: { hand: [FILLER] },
      });
      s.play(STOCKPILE);
      expect(heroHits(s, "p1")).toEqual([]);
      s.expectHealth("p1", 17);
    });

    it("damage to a Unit is untouched: Lunar Eclipse deals its whole 3 to your Mr. Vanilla", () => {
      const s = scenario({
        p1: { hand: [FILLER], field: [VANILLA], backrow: [ARGUSLAND] },
        p2: { hand: [LUNAR, FILLER] },
        active: "p2",
      });
      const vanilla = s.unit("p1", 1);
      if (vanilla === null) throw new Error("Mr. Vanilla should be on the board");
      s.play(LUNAR, { targets: [{ pick: "instance", instanceId: vanilla.id }] });
      s.expectStats(vanilla, { health: 1 });
    });

    it("guards only your hero: your opponent's takes the whole hit", () => {
      const s = scenario({ p1: { hand: [FILLER], field: [VANILLA], backrow: [ARGUSLAND] }, p2: { hand: [FILLER] } });
      s.attack(VANILLA, "hero");
      expect(heroHits(s, "p2")).toEqual([4]);
    });

    it("it guards its controller's hero: stolen by Snom Bunny Mind Control, it halves hits on the thief's hero", () => {
      const s = scenario({
        p1: { hand: [FILLER], field: [VANILLA], backrow: [ARGUSLAND], library: [FILLER] },
        p2: { hand: [MIND_CONTROL, FILLER], field: [PANTHER] },
        active: "p2",
      });
      s.play(MIND_CONTROL, { targets: [{ pick: "instance", instanceId: s.card(ARGUSLAND).id }] });
      expect(s.card(ARGUSLAND).controller).toBe("p2");
      s.attack(PANTHER, "hero");
      expect(heroHits(s, "p1")).toEqual([5]);
      // p2 has nothing left it can do, so its turn has ended on its own (R82).
      expect(s.state.active).toBe("p1");
      s.attack(VANILLA, "hero");
      expect(heroHits(s, "p2")).toEqual([2]);
    });

    it("several multiply: two Arguslands divide by 4, so a 9 becomes 3", () => {
      const s = attackOnGuarded(MENACE, [ARGUSLAND, ARGUSLAND]);
      expect(heroHits(s, "p1")).toEqual([3]);
    });

    it("several multiply across faces: a base and a Radiant one divide by 8, so a 9 becomes 2", () => {
      const s = attackOnGuarded(MENACE, [ARGUSLAND, { def: ARGUSLAND, radiant: true }]);
      expect(heroHits(s, "p1")).toEqual([2]);
    });

    it("§4.4 step 4a the lethal window reads the halved amount: a 7 halved to 4 is lethal at 4 and Final Gambit fires", () => {
      const s = attackOnGuarded(POINTMASTER, [ARGUSLAND, GAMBIT], 4);
      s.expectEvents("trapFired", "redirected");
      s.expectInZone(GAMBIT, "graveyard");
      s.expectHealth("p1", 14);
    });

    it("§4.4 step 4a ... and at 5 health the same hit is not lethal, so Final Gambit stays set", () => {
      const s = attackOnGuarded(POINTMASTER, [ARGUSLAND, GAMBIT], 5);
      expect(s.events.some((event) => event.type === "trapFired")).toBe(false);
      s.expectInZone(GAMBIT, "field");
      expect(s.card(GAMBIT).faceUp).not.toBe(true);
      s.expectHealth("p1", 1);
    });

    it("R44 My Pawn's lethal projection reads the halved amount: a 4 at 4 health is a 2, so the attack stands", () => {
      const s = attackOnGuarded(VANILLA, [ARGUSLAND, MY_PAWN], 4);
      expect(s.events.some((event) => event.type === "trapFired")).toBe(false);
      s.expectHealth("p1", 2);
    });

    it("is gone when it leaves: destroyed by Magic Jammed, the next hit lands whole", () => {
      const s = scenario({
        p1: { hand: [FILLER], backrow: [ARGUSLAND] },
        p2: { hand: [MAGIC_JAMMED, FILLER], field: [VANILLA] },
        active: "p2",
      });
      const argus = s.card(ARGUSLAND);
      s.play(MAGIC_JAMMED, { targets: [{ pick: "instance", instanceId: argus.id }] });
      s.expectInZone(argus, "graveyard");
      s.attack(VANILLA, "hero");
      expect(heroHits(s, "p1")).toEqual([4]);
      s.expectHealth("p1", 26);
    });

    it("R386 an Upgrade makes the divisor 3 (a 7 becomes 3); a Degrade never takes it below 2 (a 7 stays 4)", () => {
      const up = scenario({
        p1: { hand: [FILLER], backrow: [ARGUSLAND] },
        p2: { hand: [FILLER], field: [POINTMASTER] },
        active: "p2",
      });
      stepParam(up.card(ARGUSLAND), "divisor", 1);
      up.attack(POINTMASTER, "hero");
      expect(heroHits(up, "p1")).toEqual([3]);

      const down = scenario({
        p1: { hand: [FILLER], backrow: [ARGUSLAND] },
        p2: { hand: [FILLER], field: [POINTMASTER] },
        active: "p2",
      });
      stepParam(down.card(ARGUSLAND), "divisor", -1);
      down.attack(POINTMASTER, "hero");
      expect(heroHits(down, "p1")).toEqual([4]);
    });
  });

  describe("radiant", () => {
    it("quarters a hit on your hero, rounded up: a 9 becomes 3, a 7 becomes 2", () => {
      const nine = attackOnGuarded(MENACE, [{ def: ARGUSLAND, radiant: true }]);
      expect(heroHits(nine, "p1")).toEqual([3]);
      const seven = attackOnGuarded(POINTMASTER, [{ def: ARGUSLAND, radiant: true }]);
      expect(heroHits(seven, "p1")).toEqual([2]);
    });

    it("rounds up, so a 1 stays 1", () => {
      const s = attackOnGuarded(GARY, [{ def: ARGUSLAND, radiant: true }]);
      expect(heroHits(s, "p1")).toEqual([1]);
    });

    it("§4.4 steps 2 and 3: after Armor (7 − 2 = 5, quartered to 2) and before a cap", () => {
      const armored = attackOnGuarded(POINTMASTER, [{ def: ARGUSLAND, radiant: true }, GOING_LONG]);
      expect(heroHits(armored, "p1")).toEqual([2]);
      const capped = attackOnGuarded(MENACE, [{ def: ARGUSLAND, radiant: true }, ANTI_ONESHOT]);
      expect(heroHits(capped, "p1")).toEqual([3]);
    });

    it("R125 fatigue is quartered too, and R18 losing health still is not", () => {
      const s = scenario({
        p1: { hand: [STOCKPILE, FILLER], backrow: [{ def: ARGUSLAND, radiant: true }], library: [BLOOD_BEAN], health: 20 },
        p2: { hand: [FILLER] },
      });
      s.play(STOCKPILE);
      // The Bean is cast on draw (lose 5) and its draw is replaced; the next two are fatigue 1 and 2, each quartered to 1.
      expect(heroHits(s, "p1")).toEqual([1, 1]);
      s.expectHealth("p1", 20 - 5 - 1 - 1 + 2);
    });

    it("damage to a Unit is untouched", () => {
      const s = scenario({
        p1: { hand: [FILLER], field: [VANILLA], backrow: [{ def: ARGUSLAND, radiant: true }] },
        p2: { hand: [LUNAR, FILLER] },
        active: "p2",
      });
      const vanilla = s.unit("p1", 1);
      if (vanilla === null) throw new Error("Mr. Vanilla should be on the board");
      s.play(LUNAR, { targets: [{ pick: "instance", instanceId: vanilla.id }] });
      s.expectStats(vanilla, { health: 1 });
    });

    it("R386 its divisor is the declared number: a Degrade takes 4 to 3, so a 7 becomes 3", () => {
      const s = scenario({
        p1: { hand: [FILLER], backrow: [{ def: ARGUSLAND, radiant: true }] },
        p2: { hand: [FILLER], field: [POINTMASTER] },
        active: "p2",
      });
      stepParam(s.card(ARGUSLAND), "divisor", -1);
      s.attack(POINTMASTER, "hero");
      expect(heroHits(s, "p1")).toEqual([3]);
    });
  });
});
