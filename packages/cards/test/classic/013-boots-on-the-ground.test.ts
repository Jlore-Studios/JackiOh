// C #13 Boots on the Ground — SPEC §8.6 row 13, BUILD M9 Classic row C 13: "Charge (it may hit the
// hero the turn it enters); after each combat it attacked in, forced attacks included, draw 1, even
// when it died in that combat (last-known state, R78); defending draws nothing; radiant 4/2 Charge:
// after each attack Recruit the first permanent from the top of your deck instead of drawing; none,
// or its row full, → nothing; a recruited trap lands face-down, never named in the opponent's view
// (R33); its tuned numbers (draw, radiant recruits) read through `param()` (R386)".
//
// Every "after this attacks" case waits for the engine to run `Script.afterAttack` (after the check
// that closes each combat the card attacked in, on its last-known snapshot); see the script's header.
//
// Forced attacks come from #9 Moths to the Flame (1/14, "Start of turn: every enemy Unit attacks
// this") on the opponent's side: ending p1's turn starts p2's, and p1's units attack it.

import { stepParam } from "@jackioh/engine";
import { describe, expect, it } from "vitest";
import { scenario, type Scenario } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic/013-boots-on-the-ground";

const BOOTS = "classic-013";
const MENACE = "core-019"; // (3) Unit 9/9 Taunt.
const MOTHS = "core-009"; // 1/14: start of turn, every enemy Unit attacks this.
const TEMPO = "core-011"; // (1) Unit 3/3 Rush.
const VANILLA = "core-008"; // (1) Unit 4/4.
const STOCKPILE = "core-005"; // (1) Spell.
const SHEEPISH = "core-041"; // (1) Trap.
const ANCHOR = "core-010";
const A = "core-020";
const B = "core-001";

function handDefs(s: Scenario, player: "p1" | "p2" = "p1"): string[] {
  return s.hand(player).map((card) => card.defId);
}

function unitDefs(s: Scenario, player: "p1" | "p2" = "p1"): (string | null)[] {
  return [1, 2, 3, 4, 5].map((lane) => s.unit(player, lane)?.defId ?? null);
}

function drawsBy(s: Scenario, player: "p1" | "p2"): number {
  return s.events.filter((event) => event.type === "drawn" && event.player === player).length;
}

describe("C #13 Boots on the Ground", () => {
  it("declares its text as an afterAttack hook on each face, and no trigger", () => {
    expect(def.id).toBe(BOOTS);
    expect(base.afterAttack).toBeTypeOf("function");
    expect(radiant.afterAttack).toBeTypeOf("function");
    expect(base.triggers).toBeUndefined();
  });

  describe("base", () => {
    it("is a 2/1 with Charge: played this turn, it hits the hero at once, and then draws 1", () => {
      const s = scenario({ p1: { hand: [BOOTS, ANCHOR], library: [A, B] }, p2: { hand: [ANCHOR] } });

      s.play(BOOTS);
      s.expectStats(BOOTS, { attack: 2, health: 1 });
      s.attack(BOOTS, "hero");

      s.expectHealth("p2", 28);
      expect(handDefs(s)).toEqual([ANCHOR, A]);
      s.expectEvents("attackDeclared", "damage", "drawn");
    });

    it("§6.1 Charge: the turn it is played it may attack the hero", () => {
      const s = scenario({ p1: { hand: [BOOTS, ANCHOR], library: [A] }, p2: { hand: [ANCHOR] } });

      s.play(BOOTS);
      expect(s.stats(BOOTS).keywords.map((keyword) => keyword.kind)).toContain("Charge");
      s.attack(BOOTS, "hero");

      s.expectHealth("p2", 28);
    });

    it("draws once per attack, not twice", () => {
      const s = scenario({ p1: { hand: [ANCHOR], field: [BOOTS], library: [A, B] }, p2: { hand: [ANCHOR] } });

      s.attack(BOOTS, "hero");

      expect(drawsBy(s, "p1")).toBe(1);
    });

    it("R78 it draws even when it died in that combat, reading its last-known state", () => {
      const s = scenario({ p1: { hand: [ANCHOR], field: [BOOTS], library: [A, B] }, p2: { hand: [ANCHOR], field: [MENACE] } });

      s.attack(BOOTS, MENACE);

      s.expectInZone(BOOTS, "graveyard");
      expect(handDefs(s)).toEqual([ANCHOR, A]);
      expect(drawsBy(s, "p1")).toBe(1);
    });

    it("R53 R78 a forced attack counts: Moths to the Flame makes it attack, it dies, and it still draws", () => {
      const s = scenario({
        p1: { hand: [ANCHOR], field: [BOOTS], library: [A, B] },
        p2: { hand: [ANCHOR], field: [MOTHS], library: [VANILLA] },
      });

      s.endTurn();

      expect(s.events.some((event) => event.type === "attackDeclared" && event.forced)).toBe(true);
      s.expectInZone(BOOTS, "graveyard");
      expect(handDefs(s)).toEqual([ANCHOR, A]);
    });

    it("defending draws nothing, even when it dies defending", () => {
      const s = scenario({
        active: "p2",
        p1: { hand: [ANCHOR], field: [BOOTS], library: [A] },
        p2: { hand: [ANCHOR], field: [TEMPO] },
      });

      s.attack(TEMPO, BOOTS);

      s.expectInZone(BOOTS, "graveyard");
      expect(drawsBy(s, "p1")).toBe(0);
    });

    it("a dead one in the graveyard never answers another unit's attack", () => {
      const s = scenario({
        p1: { hand: [ANCHOR], field: [BOOTS, VANILLA], library: [A, B] },
        p2: { hand: [ANCHOR], field: [MENACE] },
      });

      s.attack(BOOTS, MENACE);
      expect(drawsBy(s, "p1")).toBe(1);
      s.attack(VANILLA, "hero");

      expect(drawsBy(s, "p1")).toBe(1);
    });

    it("R386 an Upgrade makes it draw 2", () => {
      const s = scenario({ p1: { hand: [ANCHOR], field: [BOOTS], library: [A, B] }, p2: { hand: [ANCHOR] } });
      stepParam(s.card(BOOTS), "draw", 1);

      s.attack(BOOTS, "hero");

      expect(handDefs(s)).toEqual([ANCHOR, A, B]);
    });
  });

  describe("radiant", () => {
    it("is a 4/2 with Charge that, after attacking, Recruits the first permanent from the top of your deck instead of drawing", () => {
      const s = scenario({
        p1: { hand: [{ def: BOOTS, radiant: true }, ANCHOR], library: [STOCKPILE, TEMPO, VANILLA] },
        p2: { hand: [ANCHOR] },
      });

      s.play(BOOTS);
      s.expectStats(BOOTS, { attack: 4, health: 2 });
      s.attack(BOOTS, "hero");

      s.expectHealth("p2", 26);
      expect(unitDefs(s)).toEqual([BOOTS, TEMPO, null, null, null]);
      expect(s.pile("p1", "library").map((card) => card.defId)).toEqual([STOCKPILE, VANILLA]);
      expect(drawsBy(s, "p1")).toBe(0);
    });

    it("§6.1 is a 4/2 with Charge: the turn it is played it may attack the hero", () => {
      const s = scenario({ p1: { hand: [{ def: BOOTS, radiant: true }, ANCHOR], library: [STOCKPILE] }, p2: { hand: [ANCHOR] } });

      s.play(BOOTS);
      s.expectStats(BOOTS, { attack: 4, health: 2 });
      expect(s.stats(BOOTS).keywords.map((keyword) => keyword.kind)).toContain("Charge");
      s.attack(BOOTS, "hero");

      s.expectHealth("p2", 26);
    });

    it("defending recruits nothing", () => {
      const s = scenario({
        active: "p2",
        p1: { hand: [ANCHOR], field: [{ def: BOOTS, radiant: true }], library: [TEMPO] },
        p2: { hand: [ANCHOR], field: [TEMPO] },
      });

      s.attack(s.unit("p2", 1) ?? TEMPO, BOOTS);

      expect(s.pile("p1", "library").map((card) => card.defId)).toEqual([TEMPO]);
      expect(unitDefs(s)).toEqual([null, null, null, null, null]);
    });

    it("with no permanent in the deck nothing happens", () => {
      const s = scenario({ p1: { hand: [ANCHOR], field: [{ def: BOOTS, radiant: true }], library: [STOCKPILE] }, p2: { hand: [ANCHOR] } });

      s.attack(BOOTS, "hero");

      expect(unitDefs(s)).toEqual([BOOTS, null, null, null, null]);
      expect(s.pile("p1", "library").map((card) => card.defId)).toEqual([STOCKPILE]);
    });

    it("with its row full nothing happens, and the card stays in the deck", () => {
      const s = scenario({
        p1: { hand: [ANCHOR], field: [{ def: BOOTS, radiant: true }, VANILLA, VANILLA, VANILLA, VANILLA], library: [TEMPO] },
        p2: { hand: [ANCHOR] },
      });

      s.attack(BOOTS, "hero");

      expect(s.pile("p1", "library").map((card) => card.defId)).toEqual([TEMPO]);
    });

    it("R33 a recruited Trap lands face-down in the backrow and is never named in the opponent's view", () => {
      const s = scenario({ p1: { hand: [ANCHOR], field: [{ def: BOOTS, radiant: true }], library: [SHEEPISH] }, p2: { hand: [ANCHOR] } });

      s.attack(BOOTS, "hero");

      const trap = s.backrow("p1", 1);
      expect(trap?.defId).toBe(SHEEPISH);
      expect(trap?.faceUp).not.toBe(true);
      expect(JSON.stringify(s.view("p2"))).not.toContain(SHEEPISH);
      expect(JSON.stringify(s.view("p1"))).toContain(SHEEPISH);
    });

    it("R78 it recruits even when it died in that combat", () => {
      const s = scenario({
        p1: { hand: [ANCHOR], field: [{ def: BOOTS, radiant: true }], library: [TEMPO] },
        p2: { hand: [ANCHOR], field: [MENACE] },
      });

      s.attack(BOOTS, MENACE);

      s.expectInZone(BOOTS, "graveyard");
      expect(unitDefs(s)).toEqual([TEMPO, null, null, null, null]);
    });

    it("R53 a forced attack it survives recruits too", () => {
      const s = scenario({
        p1: { hand: [ANCHOR], field: [{ def: BOOTS, radiant: true }], library: [TEMPO, VANILLA] },
        p2: { hand: [ANCHOR], field: [MOTHS], library: [VANILLA] },
      });

      s.endTurn();

      s.expectInZone(BOOTS, "field");
      expect(unitDefs(s)).toEqual([BOOTS, TEMPO, null, null, null]);
    });

    it("R386 an Upgrade of recruits makes two scans", () => {
      const s = scenario({ p1: { hand: [ANCHOR], field: [{ def: BOOTS, radiant: true }], library: [TEMPO, STOCKPILE, VANILLA] }, p2: { hand: [ANCHOR] } });
      stepParam(s.card(BOOTS), "recruits", 1);

      s.attack(BOOTS, "hero");

      expect(unitDefs(s)).toEqual([BOOTS, TEMPO, VANILLA, null, null]);
    });
  });
});
