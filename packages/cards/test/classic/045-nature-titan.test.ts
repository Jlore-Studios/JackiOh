// C #45 Nature Titan — SPEC §8.6 row 45, BUILD M9 Classic row C 45: "Tribute 1 (§6.3, R101): with no
// Unit to Tribute it can't be played; on a full board it may take the tributed Unit's zone when that
// pile is the one card, without Reborn and not Locked (R391), never the top of a Stack pile; a Sheep
// overpays (R101); Cry, and whenever it attacks (each `attackDeclared` for it, forced attacks
// included): draw 1 and heal your hero 3 with no cap (§6.3 Heal); defending does nothing; radiant
// 12/12: draw 2, heal 6; its tuned numbers (draw, heal) read through `param()` (R386)".
//
// R391's full-board cases need the play validator of the engine's play-pipeline branch (eng-play-b);
// they are real tests here and run once that branch is integrated.

import { stepParam } from "@jackioh/engine";
import { describe, expect, it } from "vitest";
import { scenario, type Scenario } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic/045-nature-titan";

const TITAN = "classic-045";
const VANILLA = "core-008"; // (1) Unit 4/4.
const MENACE = "core-019"; // (3) Unit 9/9 Taunt.
const SHEEP = "core-t-sheep"; // Sheep Token, worth 2 Tributes.
const DEFENDER = "core-003"; // Right-house defender 1/1: Taunt, Divine Shield, Reborn.
const FIENDER = "core-092"; // Felinor Fiender: Stack.
const MOTHS = "core-009"; // 1/14: Start of turn: every enemy Unit attacks this.
const TEMPO = "core-011"; // (1) Unit 3/3 Rush.
const ANCHOR = "core-010";
const A = "core-020";
const B = "core-001";
const C = "core-045";

function handDefs(s: Scenario, player: "p1" | "p2" = "p1"): string[] {
  return s.hand(player).map((card) => card.defId);
}

function unitDefs(s: Scenario, player: "p1" | "p2" = "p1"): (string | null)[] {
  return [1, 2, 3, 4, 5].map((lane) => s.unit(player, lane)?.defId ?? null);
}

describe("C #45 Nature Titan", () => {
  it("declares Tribute 1 and runs one script on both faces", () => {
    expect(def.id).toBe(TITAN);
    expect(base.staticFlags?.tribute).toBe(1);
    expect(radiant).toBe(base);
  });

  describe("base", () => {
    it("R101 with no Unit to Tribute it can't be played", () => {
      const s = scenario({ p1: { hand: [TITAN, ANCHOR], library: [A] }, p2: { hand: [ANCHOR] } });

      expect(() => s.play(TITAN)).toThrow(/Tribute/);
      s.expectInZone(TITAN, "hand");
    });

    it("Tribute 1, then its Cry: draw 1 and heal your hero 3", () => {
      const s = scenario({ p1: { hand: [TITAN, ANCHOR], field: [VANILLA], library: [A, B], health: 20 }, p2: { hand: [ANCHOR] } });

      s.play(TITAN, { tributes: [VANILLA] });

      s.expectInZone(VANILLA, "graveyard").expectInZone(TITAN, "field");
      s.expectStats(TITAN, { attack: 6, health: 6 });
      expect(handDefs(s)).toEqual([ANCHOR, A]);
      s.expectHealth("p1", 23);
    });

    it("§6.3 the heal has no cap: a hero at 30 reaches 33", () => {
      const s = scenario({ p1: { hand: [TITAN, ANCHOR], field: [VANILLA], library: [A] }, p2: { hand: [ANCHOR] } });

      s.play(TITAN, { tributes: [VANILLA] });

      s.expectHealth("p1", 33);
    });

    it("R101 a Sheep Token overpays the Tribute 1 alone", () => {
      const s = scenario({ p1: { hand: [TITAN, ANCHOR], field: [SHEEP], library: [A] }, p2: { hand: [ANCHOR] } });

      s.play(TITAN, { tributes: [SHEEP] });

      s.expectInZone(TITAN, "field");
      expect(unitDefs(s).filter((id) => id === SHEEP)).toHaveLength(0);
    });

    it("R101 only a card that says so tributes the opponent's Units: an enemy Unit can't pay it", () => {
      const s = scenario({ p1: { hand: [TITAN, ANCHOR], field: [VANILLA], library: [A] }, p2: { hand: [ANCHOR], field: [TEMPO] } });

      expect(() => s.play(TITAN, { tributes: [TEMPO] })).toThrow(/tribute/i);
      s.expectInZone(TITAN, "hand").expectInZone(TEMPO, "field");
    });

    it("R391 on a full board it may take the zone its Tribute empties", () => {
      const s = scenario({
        p1: { hand: [TITAN, ANCHOR], field: [MENACE, MENACE, VANILLA, MENACE, MENACE], library: [A] },
        p2: { hand: [ANCHOR] },
      });

      s.play(TITAN, { tributes: [VANILLA], zone: 3 });

      expect(unitDefs(s)).toEqual([MENACE, MENACE, TITAN, MENACE, MENACE]);
    });

    it("R391 R64 on a full board it can't take the zone of a tributed Reborn Unit, which is reserved", () => {
      const s = scenario({
        p1: { hand: [TITAN, ANCHOR], field: [MENACE, MENACE, DEFENDER, MENACE, MENACE], library: [A] },
        p2: { hand: [ANCHOR] },
      });

      expect(() => s.play(TITAN, { tributes: [DEFENDER], zone: 3 })).toThrow();
      s.expectInZone(TITAN, "hand");
    });

    it("R391 R13 on a full board it never takes the zone of a tributed Stack pile's top: the card beneath resumes", () => {
      const s = scenario({
        p1: { hand: [TITAN, ANCHOR], field: [VANILLA, { def: FIENDER, stack: true }, MENACE, MENACE, MENACE, MENACE], library: [A] },
        p2: { hand: [ANCHOR] },
      });

      expect(() => s.play(TITAN, { tributes: [FIENDER], zone: 1 })).toThrow();
      s.expectInZone(TITAN, "hand");
    });

    it("R391 on a full board it can't take a Locked zone, even the one its Tribute empties", () => {
      const s = scenario({
        p1: { hand: [TITAN, ANCHOR], field: [MENACE, MENACE, VANILLA, MENACE, MENACE], library: [A] },
        p2: { hand: [ANCHOR] },
      });
      // Lane 3 is Locked with its Vanilla in it (no card in this worktree locks a unit zone).
      s.state.players.p1.locks.units[2] = true;

      expect(() => s.play(TITAN, { tributes: [VANILLA], zone: 3 })).toThrow();
      s.expectInZone(TITAN, "hand").expectInZone(VANILLA, "field");
    });

    it("whenever it attacks: draw 1 and heal your hero 3, after the attack", () => {
      const s = scenario({ p1: { hand: [ANCHOR], field: [TITAN], library: [A, B], health: 20 }, p2: { hand: [ANCHOR] } });

      s.attack(TITAN, "hero");

      s.expectHealth("p2", 24).expectHealth("p1", 23);
      expect(handDefs(s)).toEqual([ANCHOR, A]);
      s.expectEvents("attackDeclared", "damage", "drawn", "healed");
    });

    it("R53 a forced attack is an attack too: Moths to the Flame's start of turn makes it attack, and it draws and heals", () => {
      const s = scenario({
        p1: { hand: [ANCHOR], field: [TITAN], library: [A, B], health: 20 },
        p2: { hand: [ANCHOR], field: [MOTHS], library: [C] },
      });

      s.endTurn();

      expect(s.state.active).toBe("p2");
      expect(s.events.some((event) => event.type === "attackDeclared" && event.forced)).toBe(true);
      expect(handDefs(s)).toEqual([ANCHOR, A]);
      s.expectHealth("p1", 23);
    });

    it("R212 the attack trigger is answered after that combat's state check, so a Titan that died in it draws and heals nothing", () => {
      const s = scenario({ p1: { hand: [ANCHOR], field: [TITAN], library: [A, B], health: 20 }, p2: { hand: [ANCHOR], field: [MENACE] } });

      s.attack(TITAN, MENACE);

      s.expectInZone(TITAN, "graveyard");
      expect(handDefs(s)).toEqual([ANCHOR]);
      s.expectHealth("p1", 20);
    });

    it("defending does nothing: an attack on it draws and heals nothing", () => {
      const s = scenario({
        active: "p2",
        p1: { hand: [ANCHOR], field: [TITAN], library: [A], health: 20 },
        p2: { hand: [ANCHOR], field: [TEMPO] },
      });

      s.attack(TEMPO, TITAN);

      expect(handDefs(s, "p1")).toEqual([ANCHOR]);
      s.expectHealth("p1", 20);
    });

    it("R386 an Upgrade of draw makes it draw 2; of heal, heal 4", () => {
      const drawUp = scenario({ p1: { hand: [TITAN, ANCHOR], field: [VANILLA], library: [A, B, C], health: 20 }, p2: { hand: [ANCHOR] } });
      stepParam(drawUp.card(TITAN), "draw", 1);
      drawUp.play(TITAN, { tributes: [VANILLA] });
      expect(handDefs(drawUp)).toEqual([ANCHOR, A, B]);
      drawUp.expectHealth("p1", 23);

      const healUp = scenario({ p1: { hand: [TITAN, ANCHOR], field: [VANILLA], library: [A, B, C], health: 20 }, p2: { hand: [ANCHOR] } });
      stepParam(healUp.card(TITAN), "heal", 1);
      healUp.play(TITAN, { tributes: [VANILLA] });
      healUp.expectHealth("p1", 24);
    });
  });

  describe("radiant", () => {
    it("is a 12/12 whose Cry draws 2 and heals 6", () => {
      const s = scenario({
        p1: { hand: [{ def: TITAN, radiant: true }, ANCHOR], field: [VANILLA], library: [A, B, C], health: 20 },
        p2: { hand: [ANCHOR] },
      });

      s.play(TITAN, { tributes: [VANILLA] });

      s.expectStats(TITAN, { attack: 12, health: 12 });
      expect(handDefs(s)).toEqual([ANCHOR, A, B]);
      s.expectHealth("p1", 26);
    });

    it("whenever it attacks it draws 2 and heals 6", () => {
      const s = scenario({ p1: { hand: [ANCHOR], field: [{ def: TITAN, radiant: true }], library: [A, B, C], health: 20 }, p2: { hand: [ANCHOR] } });

      s.attack(TITAN, "hero");

      s.expectHealth("p2", 18).expectHealth("p1", 26);
      expect(handDefs(s)).toEqual([ANCHOR, A, B]);
    });

    it("R101 still needs its Tribute 1", () => {
      const s = scenario({ p1: { hand: [{ def: TITAN, radiant: true }, ANCHOR], library: [A] }, p2: { hand: [ANCHOR] } });

      expect(() => s.play(TITAN)).toThrow(/Tribute/);
    });

    it("R386 an Upgrade of heal on the Radiant face steps 6 to 7", () => {
      const s = scenario({ p1: { hand: [ANCHOR], field: [{ def: TITAN, radiant: true }], library: [A, B, C], health: 20 }, p2: { hand: [ANCHOR] } });
      stepParam(s.card(TITAN), "heal", 1);

      s.attack(TITAN, "hero");

      s.expectHealth("p1", 27);
    });
  });
});
