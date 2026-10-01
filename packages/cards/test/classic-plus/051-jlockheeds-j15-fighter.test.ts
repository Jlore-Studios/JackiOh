// C+ #51 Jlockheed's J15 Fighter — SPEC §8.7 row 51, BUILD M9 Classic+ row C+ 51: "First Strike, Rush;
// cannot be in Defense Position; can't be attacked: never the target of a declared or forced attack (a
// random-enemy forced attack never draws it), while effects still target it and "all" effects hit it;
// a Taunt given to it binds no attacker; radiant 14/4 with Divine Shield too".

import { randomAttackTargets } from "@jackioh/engine";
import { describe, expect, it } from "vitest";
import { scenario, type Scenario } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic-plus/051-jlockheeds-j15-fighter";

const J15 = "classicplus-051";
const VANILLA = "core-008"; // Mr. Vanilla 4/4
const HIT_JOB = "core-016"; // (3) Spell: destroy target Unit
const NETHER = "core-088"; // (4) Spell: destroy all permanents
const HONEYPOT = "core-060"; // Trap: Radiant fires on any card the opponent plays; Rush Tokens attack a Unit
const FILLER = "core-005";

function kinds(s: Scenario, ref: string): string[] {
  return s.stats(ref).keywords.map((keyword) => keyword.kind);
}

/** J15 on p1's side; p2 to act with a 4/4 that may attack. */
function defended(opts: { radiant?: boolean; p1Field?: readonly string[] } = {}): Scenario {
  return scenario({
    active: "p2",
    p1: {
      field: [{ def: J15, ...(opts.radiant === true ? { radiant: true } : {}) }, ...(opts.p1Field ?? [])],
      hand: [FILLER],
    },
    p2: { field: [VANILLA], hand: [HIT_JOB, NETHER, FILLER] },
  });
}

describe("C+ #51 Jlockheed's J15 Fighter", () => {
  it("is a 7/2 First Strike, Rush Jlockeed Unit; both faces run one script of two flags", () => {
    expect(def.id).toBe(J15);
    expect(def.tags).toEqual(["Jlockeed"]);
    expect(base.staticFlags).toEqual({ neverDefense: true, cantBeAttacked: true });
    expect(radiant).toBe(base);
  });

  describe("base", () => {
    it("§6.1 7/2 with First Strike and Rush", () => {
      const s = defended();
      s.expectStats(J15, { attack: 7, health: 2 });
      expect(kinds(s, J15)).toEqual(expect.arrayContaining(["First Strike", "Rush"]));
      expect(kinds(s, J15)).not.toContain("Divine Shield");
    });

    it("§4.1 a switch to Defense Position is refused", () => {
      const s = scenario({ p1: { field: [J15], hand: [FILLER] } });
      expect(() => s.switchPosition(J15)).toThrow(/Defense Position/);
    });

    it("§4.2 step 2 a declared attack on it is refused; the hero stays open", () => {
      const s = defended();
      expect(() => s.attack(VANILLA, J15)).toThrow(/cannot be attacked/);
      s.attack(VANILLA, "hero");
      s.expectHealth("p1", 26);
      s.expectStats(J15, { health: 2 });
    });

    it("E35 a random-enemy forced attack never draws it: the hero and the other Units only", () => {
      const s = defended({ p1Field: [VANILLA] });
      const attacker = s.unit("p2", 1);
      if (attacker === null) throw new Error("no attacker");
      const targets = randomAttackTargets(s.state, attacker, "enemies").map((target) =>
        target.kind === "hero" ? `hero-${target.player}` : target.instance.defId,
      );
      expect(targets.sort()).toEqual([VANILLA, "hero-p1"]);
      expect(randomAttackTargets(s.state, attacker, "enemyUnits").map((target) => (target.kind === "unit" ? target.instance.defId : ""))).toEqual([VANILLA]);
    });

    it("R53 a forced attack on it does not happen: a Radiant Bear Honeypot's Rush Tokens never hit it", () => {
      const s = scenario({
        p1: { hand: [J15, FILLER] },
        p2: { backrow: [{ def: HONEYPOT, radiant: true, faceUp: false }], hand: [FILLER] },
      });
      s.play(J15);
      expect(s.events.some((event) => event.type === "trapFired")).toBe(true);
      expect(s.events.filter((event) => event.type === "summoned" && event.player === "p2").length).toBeGreaterThan(0);
      expect(s.events.some((event) => event.type === "attackDeclared")).toBe(false);
      s.expectStats(J15, { health: 2 });
    });

    it("E35 a Taunt it gains binds no attacker", () => {
      const s = defended({ p1Field: [VANILLA] });
      s.card(J15).grantedKeywords.push({ kind: "Taunt" });
      expect(kinds(s, J15)).toContain("Taunt");
      s.attack(VANILLA, s.unit("p1", 2) ?? "");
      expect(s.events.some((event) => event.type === "attackDeclared")).toBe(true);
    });

    it("E35 an effect still targets it: Hit Job destroys it", () => {
      const s = defended();
      s.play(HIT_JOB, { targets: [{ pick: "instance", instanceId: s.card(J15).id }] });
      s.expectInZone(J15, "graveyard");
    });

    it("E35 an \"all\" effect still hits it: Twisting Nether destroys it", () => {
      const s = defended();
      s.play(NETHER);
      s.expectInZone(J15, "graveyard");
    });
  });

  describe("radiant", () => {
    it("§5.2 14/4 with First Strike, Rush and Divine Shield, still never attacked", () => {
      const s = defended({ radiant: true });
      s.expectStats(J15, { attack: 14, health: 4 });
      expect(kinds(s, J15)).toEqual(expect.arrayContaining(["First Strike", "Rush", "Divine Shield"]));
      expect(() => s.attack(VANILLA, J15)).toThrow(/cannot be attacked/);
    });

    it("§4.1 the Radiant face still refuses Defense Position", () => {
      const s = scenario({ p1: { field: [{ def: J15, radiant: true }], hand: [FILLER] } });
      expect(() => s.switchPosition(J15)).toThrow(/Defense Position/);
    });
  });
});
