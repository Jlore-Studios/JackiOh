// C #62 Living Bomb — SPEC §8.6 row 62, BUILD M9 Classic row C 62: "At the start of each player's turn,
// in R68's order: destroy every permanent that player controls with a Plague Token on it, face-down ones
// and Living Bomb itself included, the other player's untouched (R400); Indestructible ones survive
// (R46); a card's tokens are gone once it leaves (R78); radiant: only at the start of your opponent's
// turn, and only their permanents; no tuned numbers".
//
// The opponent's turn is answered on `turnStarted`, which the engine dispatches ahead of R62's
// start-of-turn trigger point; its own turn is its `startOfTurn` hook, queued at that point in R68's
// order (the script's header says why).

import { describe, expect, it } from "vitest";
import { scenario, type Scenario } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic/062-living-bomb";

const BOMB = "classic-062";
const VANILLA = "core-008"; // (1) Unit 4/4.
const MENACE = "core-019"; // (3) Unit 9/9 Taunt.
const ROCK = "core-066"; // (4) Unit 10/10 Indestructible, Tribute 1.
const FAUCI = "core-091"; // (2) Unit 1/6 Rush; Start of turn: +1 mana per Plague Token.
const PAWN = "core-096"; // (1) Trap: answers only an attack that would be lethal.
const MANA_WELL = "core-006"; // (3) Field Spell.
const FLOOD = "core-017"; // (4) Spell: Bounce all Units.
const FILLER = "core-005"; // (1) Spell, a card to keep a turn from auto-ending (§2.5).
const X = "core-020"; // library filler.

function lib(n: number): string[] {
  return Array.from({ length: n }, () => X);
}

function destroyedIds(s: Scenario): string[] {
  return s.events.flatMap((event) => (event.type === "destroyed" ? [event.instanceId] : []));
}

/** p1's Living Bomb, and a plagued and a clean board on each side. */
function board(radiantFace: boolean, bombTokens = 0): Scenario {
  return scenario({
    p1: {
      hand: [FILLER],
      field: [{ def: VANILLA, counters: { plague: 1 } }, { def: MENACE, lane: 2 }],
      backrow: [{ def: BOMB, radiant: radiantFace, counters: bombTokens > 0 ? { plague: bombTokens } : {} }, { def: PAWN, faceUp: false, lane: 2, counters: { plague: 2 } }],
      library: lib(4),
    },
    p2: {
      hand: [FILLER],
      field: [{ def: VANILLA, counters: { plague: 2 } }, { def: MENACE, lane: 2 }],
      backrow: [{ def: MANA_WELL, counters: { plague: 1 } }, { def: PAWN, faceUp: false, lane: 2, counters: { plague: 1 } }, { def: PAWN, faceUp: false, lane: 3 }],
      library: lib(4),
    },
  });
}

describe("C #62 Living Bomb", () => {
  it("has no declared numbers; the base face holds both turns, the Radiant face the opponent's only", () => {
    expect(def.id).toBe(BOMB);
    expect(def.params).toBeUndefined();
    expect(base.startOfTurn).toBeTypeOf("function");
    expect(base.triggers?.map((trigger) => trigger.on)).toEqual([["turnStarted"]]);
    expect(radiant.startOfTurn).toBeUndefined();
    expect(radiant.triggers).toEqual(base.triggers);
  });

  describe("base", () => {
    it("R400 at the start of the opponent's turn: every permanent THEY control with a token is destroyed, face-down ones included", () => {
      const s = board(false);
      const theirVanilla = s.unit("p2", 1);
      const theirWell = s.backrow("p2", 1);
      const theirTrap = s.backrow("p2", 2);
      const theirCleanTrap = s.backrow("p2", 3);
      if (theirVanilla === null || theirWell === null || theirTrap === null || theirCleanTrap === null) throw new Error("board");

      s.endTurn();

      expect(s.state.active).toBe("p2");
      expect(new Set(destroyedIds(s))).toEqual(new Set([theirVanilla.id, theirWell.id, theirTrap.id]));
      s.expectInZone(theirVanilla, "graveyard");
      s.expectInZone(theirWell, "graveyard");
      s.expectInZone(theirTrap, "graveyard");
      // The clean ones stay, the clean face-down trap still unnamed to p1.
      expect(s.unit("p2", 2)?.defId).toBe(MENACE);
      expect(s.backrow("p2", 3)?.id).toBe(theirCleanTrap.id);
    });

    it("R400 the other player's permanents are untouched: p1's plagued ones survive p2's turn start", () => {
      const s = board(false, 1);

      s.endTurn();

      expect(s.unit("p1", 1)?.counters.plague).toBe(1);
      expect(s.backrow("p1", 1)?.defId).toBe(BOMB);
      expect(s.backrow("p1", 2)?.defId).toBe(PAWN);
    });

    it("R400 at the start of your own turn your plagued permanents are destroyed, face-down ones and Living Bomb itself included", () => {
      const s = board(false, 1);
      const myVanilla = s.unit("p1", 1);
      const bomb = s.backrow("p1", 1);
      const myTrap = s.backrow("p1", 2);
      if (myVanilla === null || bomb === null || myTrap === null) throw new Error("board");
      s.endTurn(); // p2's turn: their plagued permanents go

      const before = destroyedIds(s).length;
      s.endTurn(); // p1's turn

      expect(s.state.active).toBe("p1");
      expect(new Set(destroyedIds(s).slice(before))).toEqual(new Set([myVanilla.id, bomb.id, myTrap.id]));
      s.expectInZone(bomb, "graveyard");
      expect(s.unit("p1", 2)?.defId).toBe(MENACE);
      // p2's survivors are untouched on p1's turn.
      expect(s.unit("p2", 2)?.defId).toBe(MENACE);
    });

    it("§4.5 they die together, at one state check after the trigger", () => {
      const s = board(false);

      s.endTurn();

      const kinds = s.lastEvents.flatMap((event) => (event.type === "destroyed" ? [event.type] : []));
      expect(kinds).toHaveLength(3);
    });

    it("R46 an Indestructible permanent with a token survives, knocked into Attack Position", () => {
      const s = scenario({
        p1: { hand: [FILLER], backrow: [BOMB], library: lib(2) },
        p2: { hand: [FILLER], field: [{ def: ROCK, position: "DEF", counters: { plague: 2 } }], library: lib(2) },
      });
      const rock = s.card(ROCK);

      s.endTurn();

      s.expectInZone(rock, "field");
      expect(s.stats(rock).position).toBe("ATK");
      expect(s.card(rock).counters.plague).toBe(2);
    });

    it("R78 a card's tokens are gone once it leaves: bounced and played again, it survives your next turn start", () => {
      const s = scenario({
        p1: { hand: [FLOOD, FILLER], field: [{ def: VANILLA, counters: { plague: 3 } }], backrow: [BOMB], library: lib(4), mana: 10 },
        p2: { hand: [FILLER], library: lib(4) },
      });
      const vanilla = s.card(VANILLA);
      s.play(FLOOD);
      expect(s.card(vanilla).counters.plague).toBeUndefined();
      s.play(vanilla);

      s.endTurn();
      s.endTurn();

      expect(s.state.active).toBe("p1");
      s.expectInZone(vanilla, "field");
      expect(destroyedIds(s)).toEqual([]);
    });

    it("R68 on your own turn it is a start-of-turn trigger in queue order: a plagued Fed Fauci in lane 1 gains its mana first, then dies", () => {
      const s = scenario({
        active: "p2",
        p1: { hand: [FILLER], field: [{ def: FAUCI, counters: { plague: 2 } }], backrow: [BOMB], library: lib(2) },
        p2: { hand: [FILLER], library: lib(2) },
      });
      const fauci = s.card(FAUCI);

      s.endTurn();

      expect(s.state.active).toBe("p1");
      s.expectMana("p1", 4 + 2);
      s.expectInZone(fauci, "graveyard");
    });

    it("with no plagued permanent it destroys nothing", () => {
      const s = scenario({
        p1: { hand: [FILLER], field: [VANILLA], backrow: [BOMB], library: lib(2) },
        p2: { hand: [FILLER], field: [MENACE], library: lib(2) },
      });

      s.endTurn();
      s.endTurn();

      expect(destroyedIds(s)).toEqual([]);
    });

    it("leaving the field ends it: once it is gone, no turn start destroys anything", () => {
      const s = scenario({
        p1: { hand: [FILLER], backrow: [{ def: BOMB, counters: { plague: 1 } }], field: [{ def: VANILLA, counters: { plague: 1 } }], library: lib(3) },
        p2: { hand: [FILLER], field: [{ def: VANILLA, counters: { plague: 1 } }], library: lib(3) },
      });
      s.endTurn(); // p2's Vanilla goes
      s.endTurn(); // p1's Vanilla and the Bomb go together
      const after = destroyedIds(s).length;
      expect(after).toBe(3);

      s.endTurn();

      expect(destroyedIds(s)).toHaveLength(after);
    });
  });

  describe("radiant", () => {
    it("R400 at the start of the opponent's turn it destroys their plagued permanents, face-down ones included", () => {
      const s = board(true);
      const theirVanilla = s.unit("p2", 1);
      const theirWell = s.backrow("p2", 1);
      const theirTrap = s.backrow("p2", 2);
      if (theirVanilla === null || theirWell === null || theirTrap === null) throw new Error("board");

      s.endTurn();

      expect(new Set(destroyedIds(s))).toEqual(new Set([theirVanilla.id, theirWell.id, theirTrap.id]));
    });

    it("R400 at the start of your own turn it does nothing: your plagued permanents and the Bomb itself stay", () => {
      const s = board(true, 2);
      s.endTurn();
      const before = destroyedIds(s).length;

      s.endTurn();

      expect(s.state.active).toBe("p1");
      expect(destroyedIds(s)).toHaveLength(before);
      expect(s.unit("p1", 1)?.counters.plague).toBe(1);
      expect(s.backrow("p1", 1)?.defId).toBe(BOMB);
      expect(s.backrow("p1", 2)?.defId).toBe(PAWN);
    });

    it("R46 an Indestructible enemy permanent survives it too", () => {
      const s = scenario({
        p1: { hand: [FILLER], backrow: [{ def: BOMB, radiant: true }], library: lib(2) },
        p2: { hand: [FILLER], field: [{ def: ROCK, counters: { plague: 1 } }], library: lib(2) },
      });

      s.endTurn();

      s.expectInZone(ROCK, "field");
    });
  });
});
