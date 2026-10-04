// C #62 Living Bomb — SPEC §8.6 row 62, BUILD M9 Classic row C 62: "At the start of each player's turn,
// in R68's order: destroy every permanent that player controls with a Plague Counter on it, face-down ones
// and Living Bomb itself included, the other player's untouched (R400); Indestructible ones survive
// (R46); a card's tokens are gone once it leaves (R78); radiant: only at the start of your opponent's
// turn, and only their permanents; no tuned numbers".
//
// Both halves are start-of-turn hooks queued at R62's start-of-turn trigger point in R68's order: its own
// turn's `startOfTurn`, and the opponent's turn's `startOfOpponentTurn`, after the turn player's hooks.

import { describe, expect, it } from "vitest";
import { scenario, type Scenario } from "../_harness";
import { expectAnimated } from "../_animated";
import { base, def, radiant } from "../../src/scripts/classic/062-living-bomb";

const BOMB = "classic-062";
const VANILLA = "core-008"; // (1) Unit 4/4.
const MENACE = "core-019"; // (3) Unit 9/9 Taunt.
const ROCK = "core-066"; // (4) Unit 10/10 Indestructible, Tribute 1.
const FAUCI = "core-091"; // (2) Unit 1/6 Rush; Start of turn: +1 mana per Plague Counter.
const FIENDER = "core-092"; // (2) Unit 5/7 Stack.
const REBORN = "core-003"; // (1) Unit 1/1 Taunt, Divine Shield, Reborn.
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
    expect(base.startOfOpponentTurn).toBeTypeOf("function");
    expect(radiant.startOfTurn).toBeUndefined();
    expect(radiant.startOfOpponentTurn).toBe(base.startOfOpponentTurn);
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

    it("R68 on the opponent's turn it comes after their own start-of-turn triggers: their plagued Fed Fauci gains its mana, then dies", () => {
      const s = scenario({
        p1: { hand: [FILLER], backrow: [BOMB], library: lib(2) },
        p2: { hand: [FILLER], field: [{ def: FAUCI, counters: { plague: 2 } }], library: lib(2) },
      });
      const fauci = s.card(FAUCI);

      s.endTurn();

      expect(s.state.active).toBe("p2");
      s.expectMana("p2", 4 + 2);
      s.expectInZone(fauci, "graveyard");
    });

    it("§3.2 R13 a card dormant under a Stack pile is not on the field: the plagued top goes, the plagued card beneath resumes", () => {
      const s = scenario({
        p1: { hand: [FILLER], backrow: [BOMB], library: lib(2) },
        p2: { hand: [FILLER], field: [{ def: VANILLA, counters: { plague: 1 } }, { def: FIENDER, stack: true, counters: { plague: 1 } }], library: lib(2) },
      });
      const top = s.card(FIENDER);
      const beneath = s.card(VANILLA);

      s.endTurn();

      expect(destroyedIds(s)).toEqual([top.id]);
      expect(s.unit("p2", 1)?.id).toBe(beneath.id);
      expect(s.card(beneath).counters.plague).toBe(1);
    });

    it("§6.1 R78 a plagued Reborn unit comes back without its token, and the next turn start leaves it", () => {
      const s = scenario({
        p1: { hand: [FILLER], backrow: [BOMB], library: lib(3) },
        p2: { hand: [FILLER], field: [{ def: REBORN, counters: { plague: 1 } }], library: lib(3) },
      });
      const unit = s.card(REBORN);

      s.endTurn();

      expect(destroyedIds(s)).toEqual([unit.id]);
      expect(s.unit("p2", 1)?.defId).toBe(REBORN);
      expect(s.unit("p2", 1)?.counters.plague).toBeUndefined();
      s.endTurn();
      s.endTurn();
      expect(destroyedIds(s)).toEqual([unit.id]);
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

describe("C #62 Living Bomb: Animated (patch v0.2.10)", () => {
  it("R383 played, it animates into its lane's unit zone, else the leftmost open one, a 2/1 Unit; with none open it stays a Field Spell", () => {
    expectAnimated({ def: "classic-062", stats: { attack: 2, health: 1 } });
  });

  it("R383 radiant: a 4/2 Unit", () => {
    expectAnimated({ def: "classic-062", radiant: true, stats: { attack: 4, health: 2 } });
  });

  it("R383 played, it keeps its text as a Unit: at the start of the opponent's turn their plagued permanent is destroyed", () => {
    const s = scenario({
      p1: { hand: [BOMB, FILLER], library: lib(2) },
      p2: { hand: [FILLER], field: [{ def: VANILLA, counters: { plague: 2 } }, { def: MENACE, lane: 2 }], library: lib(2) },
    });
    const plagued = s.unit("p2", 1)!;
    const clean = s.unit("p2", 2)!;

    s.play(BOMB, { zone: 3 });
    expect(s.unit("p1", 3)?.defId).toBe(BOMB);
    s.endTurn();

    s.expectInZone(plagued, "graveyard");
    s.expectInZone(clean, "field");
    s.expectInZone(BOMB, "field");
  });
});
