// C+ #19.1 Top Loser — SPEC §8.7 row 19.1, BUILD M9 Classic+ row C+ 19.1: "Armor 3; cannot be in
// Defense Position (a switch is refused, a switch-all effect leaves it in Attack); only an enemy Unit
// in its own lane's unit zone may attack it, declared or forced: a forced attack from another lane
// does not happen and a random-enemy forced attack from another lane never draws it (§4.2 step 2); a
// Taunt given to it binds only the attacker in its lane; effects still target it; a Token, it ceases
// to exist when it leaves; its Armor reads through `param()`; radiant 10/10 Armor 6 and Immune to
// Spells: no Spell targets it and no Spell's effect touches it (Powder Spray, Whirlwind, Brawl pass it
// by), while Field Spells, Traps and Units still reach it and it may still be attacked from its lane".
//
// Its Armor is a numbered keyword, which B3.4's X change tunes rather than a declared param (R386,
// R482): the "reads through param()" clause is proved by an Upgrade of that number.

import { addStep, legalActions, tuningOf } from "@jackioh/engine";
import type { Selection } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { scenario, type Scenario, type SideSetup } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic-plus/019-1-top-loser";

const TOP = "classicplus-019-1";
const JUNGLE = "classicplus-019-2";
const BODY = "core-019"; // 9/9 Taunt
const VANILLA = "core-008"; // 4/4
const HIT_JOB = "core-016"; // Spell: destroy target Unit
const SORCERER = "core-068"; // Unit: Cry deal 4 damage to a target
const SWITCH_ALL = "core-048"; // Spell: switch the position of every Unit
const NETHER = "core-088"; // Spell: destroy all permanents
const BIG_FELINOR = "core-043"; // Unit: Cry destroy all non-Felinor Units
const WHIRLWIND = "classicplus-021";
const HONEYPOT = "core-060"; // Trap; radiant: fill your board with Rush Tokens, they attack a played Unit
const FILLER = "core-005";
const DECK = [FILLER, FILLER, FILLER, FILLER];

/** p2's Top Loser in lane 3; p1 is active with the given side. */
function topLoser(p1: SideSetup, radiantFace = false, p2: SideSetup = {}): Scenario {
  return scenario({
    p1: { hand: [FILLER], library: DECK, ...p1 },
    p2: { hand: [FILLER], library: DECK, ...p2, field: [{ def: TOP, lane: 3, radiant: radiantFace }, ...(p2.field ?? [])] },
  });
}

function top(s: Scenario): ReturnType<Scenario["card"]> {
  const unit = s.unit("p2", 3);
  if (unit === null || unit.defId !== TOP) throw new Error("no Top Loser in lane 3");
  return unit;
}

function at(s: Scenario, player: "p1" | "p2", lane: number): Selection[] {
  return [{ pick: "instance", instanceId: s.unit(player, lane)?.id ?? "none" }];
}

function attackTargets(s: Scenario, attackerId: string): string[] {
  return legalActions(s.state, "p1").flatMap((action) =>
    action.type === "attack" && action.attackerId === attackerId ? [action.targetId] : [],
  );
}

describe("C+ #19.1 Top Loser", () => {
  it("is a (2) 5/5 Unit token with a printed rarity; one script serves both faces", () => {
    expect(def.token).toBe(true);
    expect(def.rarity).toBe("Token");
    expect(def.printedRarity).toBe("Legendary");
    expect(base.staticFlags).toEqual({ attackedOnlyFromLane: true, neverDefense: true });
    expect(radiant).toBe(base);
  });

  describe("base", () => {
    it("has Armor 3: a 4-damage hit leaves it at 4", () => {
      const s = topLoser({ field: [{ def: VANILLA, lane: 3 }] });
      expect(s.stats(top(s)).armor).toBe(3);
      s.attack(s.unit("p1", 3) ?? "", top(s));
      s.expectStats(top(s), { health: 4 });
    });

    it("R386 its Armor is tuned as a numbered keyword: an Upgrade makes it Armor 4", () => {
      const s = topLoser({ field: [{ def: VANILLA, lane: 3 }] });
      const tuning = tuningOf(top(s));
      tuning.x = addStep(tuning.x, "Armor", 1);
      expect(s.stats(top(s)).armor).toBe(4);
      s.attack(s.unit("p1", 3) ?? "", top(s));
      s.expectStats(top(s), { health: 5 });
    });

    it("§4.1 it can't be switched to Defense Position: the switch is refused", () => {
      const s = scenario({ p1: { hand: [FILLER], field: [{ def: TOP, lane: 1 }] }, p2: { hand: [FILLER] } });
      expect(() => s.switchPosition(s.unit("p1", 1) ?? "")).toThrow(/Defense/);
      expect(s.stats(s.unit("p1", 1) ?? "").position).toBe("ATK");
    });

    it("§4.1 a switch-all effect leaves it in Attack Position", () => {
      const s = topLoser({ hand: [SWITCH_ALL, FILLER], field: [{ def: VANILLA, lane: 1 }] });
      s.play(SWITCH_ALL);
      expect(s.stats(top(s)).position).toBe("ATK");
      expect(s.stats(s.unit("p1", 1) ?? "").position).toBe("DEF");
    });

    it("§4.2 step 2 only the enemy Unit in its own lane may declare an attack on it", () => {
      const s = topLoser({ field: [{ def: VANILLA, lane: 2 }, { def: VANILLA, lane: 3 }] });
      const across = s.unit("p1", 3);
      const beside = s.unit("p1", 2);
      if (across === null || beside === null) throw new Error("setup");
      expect(attackTargets(s, beside.id)).not.toContain(top(s).id);
      expect(attackTargets(s, across.id)).toContain(top(s).id);
      expect(() => s.attack(beside, top(s))).toThrow(/lane/);
      s.attack(across, top(s));
      s.expectStats(top(s), { health: 4 });
    });

    it("R53 a forced attack from another lane does not happen: a radiant Honeypot's tokens attack it only from its lane", () => {
      const s = scenario({
        active: "p2",
        p1: { hand: [FILLER], backrow: [{ def: HONEYPOT, lane: 1, faceUp: false, radiant: true }], library: DECK },
        p2: { hand: [TOP, FILLER], library: DECK },
      });
      s.play(TOP, { zone: 3 });
      // The board filled with five Rush Tokens; only the one in lane 3 attacked (and died to the strike back).
      const tokens = s.events.flatMap((event) => (event.type === "summoned" && event.player === "p1" ? [event] : []));
      expect(tokens.map((event) => event.lane)).toEqual([1, 2, 3, 4, 5]);
      const attackers = s.events.flatMap((event) => (event.type === "attackDeclared" ? [event.attackerId] : []));
      expect(attackers).toEqual([tokens[2]?.instanceId]);
    });

    it("§4.2 step 2 a random-enemy forced attack from another lane never draws it", () => {
      for (let seed = 1; seed <= 10; seed += 1) {
        const s = scenario({
          seed: `top-random-${seed}`,
          p1: { hand: [FILLER], field: [{ def: JUNGLE, lane: 1, radiant: true }], library: DECK },
          p2: { hand: [FILLER], field: [{ def: TOP, lane: 3 }, { def: BODY, lane: 5 }], library: DECK },
        });
        s.endTurn();
        const attacks = s.events.filter((event) => event.type === "attackDeclared");
        for (const event of attacks) if (event.type === "attackDeclared") expect(event.targetId).not.toBe(s.unit("p2", 3)?.id ?? "-");
      }
    });

    it("§4.2 step 3 a Taunt on it binds only the attacker in its lane", () => {
      const s = topLoser({ field: [{ def: VANILLA, lane: 2 }, { def: VANILLA, lane: 3 }] });
      top(s).grantedKeywords.push({ kind: "Taunt" });
      const beside = s.unit("p1", 2);
      const across = s.unit("p1", 3);
      if (beside === null || across === null) throw new Error("setup");
      // The lane-2 attacker cannot reach the Taunt, so it binds nothing: the hero is open to it.
      expect(attackTargets(s, beside.id)).toContain("hero-p2");
      // The lane-3 attacker can, so it must attack the Taunt.
      expect(attackTargets(s, across.id)).toEqual([top(s).id]);
    });

    it("effects still target it: a Spell destroys it and, a Token, it ceases to exist", () => {
      const s = topLoser({ hand: [HIT_JOB, FILLER] });
      const loser = top(s);
      s.play(HIT_JOB, { targets: [{ pick: "instance", instanceId: loser.id }] });
      s.expectInZone(loser, "gone");
    });
  });

  describe("radiant", () => {
    it("is 10/10 with Armor 6 and Immune to Spells", () => {
      const s = topLoser({}, true);
      s.expectStats(top(s), { attack: 10, health: 10 });
      expect(s.stats(top(s)).armor).toBe(6);
      expect(s.stats(top(s)).keywords.some((keyword) => keyword.kind === "Immune to Spells")).toBe(true);
    });

    it("§6.1 no Spell targets it", () => {
      const s = topLoser({ hand: [HIT_JOB, FILLER] }, true, { field: [{ def: VANILLA, lane: 1 }] });
      expect(() => s.play(HIT_JOB, { targets: [{ pick: "instance", instanceId: top(s).id }] })).toThrow();
      const offered = legalActions(s.state, "p1").flatMap((action) =>
        action.type === "play" && action.instanceId === s.card(HIT_JOB).id ? (action.targets ?? []) : [],
      );
      expect(JSON.stringify(offered)).not.toContain(top(s).id);
    });

    it("§6.1 no Spell's effect touches it: Whirlwind and Twisting Nether pass it by", () => {
      const s = topLoser({ hand: [WHIRLWIND, NETHER, FILLER], mana: 5 }, true, { field: [{ def: BODY, lane: 1 }] });
      s.play(WHIRLWIND);
      s.expectStats(top(s), { health: 10 });
      s.play(NETHER);
      expect(s.unit("p2", 3)?.defId).toBe(TOP);
      expect(s.unit("p2", 1)).toBeNull();
    });

    it("§6.1 a Unit still reaches it: a Cry targets it and a Unit's sweep destroys it", () => {
      // Radiant Twisted Sorcerer's 8 through the Radiant Top Loser's Armor 6 leaves 2.
      const s = topLoser({ hand: [{ def: SORCERER, radiant: true }, FILLER] }, true);
      s.play(SORCERER, { zone: 1, targets: [{ pick: "instance", instanceId: top(s).id }] });
      s.expectStats(top(s), { health: 8 });
      const felinor = topLoser({ hand: [BIG_FELINOR, FILLER] }, true);
      const loser = top(felinor);
      felinor.play(BIG_FELINOR, { zone: 1 });
      felinor.expectInZone(loser, "gone");
    });

    it("§4.2 it may still be attacked from its lane", () => {
      const s = topLoser({ field: [{ def: BODY, lane: 3 }] }, true);
      s.attack(s.unit("p1", 3) ?? "", top(s));
      s.expectStats(top(s), { health: 7 });
    });

    it("§4.1 it still can't be in Defense Position", () => {
      const s = scenario({ p1: { hand: [FILLER], field: [{ def: TOP, lane: 1, radiant: true }] }, p2: { hand: [FILLER] } });
      expect(() => s.switchPosition(s.unit("p1", 1) ?? "")).toThrow(/Defense/);
    });

    it("unused helper guard", () => {
      expect(at).toBeDefined();
    });
  });
});
