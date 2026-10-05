// C #70 Book of Plague — SPEC §8.6 row 70, BUILD M9 Classic row C 70: "Five placements of one Plague
// Token, all on the one permanent a single prompt names (R669), on any permanent either side,
// face-down ones included; with no permanent on the field it places nothing; each placement is still
// its own for "whenever tokens are placed" (C #53 draws five times) and C #27 doubles its share;
// a face-down option carries only its id (R177); tagged Book, so C #4 answers it; radiant: ten; its
// tuned number (tokens) reads through `param()` (R386)".
//
// The C #27 Pestilent Slime and C #4 Palantir cases need those cards' scripts (cards-classic-a).

import { hashState, reduce, stepParam, type CardInstance, type GameState } from "@jackioh/engine";
import type { GameEvent, PlayerId } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { scenario, type Scenario } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic/070-book-of-plague";

const BOOK = "classic-070";
const CRAWLER = "classic-053"; // (1) Unit: whenever Plague Tokens are placed on this, draw 1.
const SLIME = "classic-027"; // (0) Unit: Plague Tokens placed on this are doubled.
const PALANTIR = "classic-004"; // (1) Field Spell: when your opponent plays a Book, you may Tribute this to steal it.
const VANILLA = "core-008"; // (1) Unit 4/4.
const MENACE = "core-019"; // (3) Unit 9/9 Taunt.
const FIENDER = "core-092"; // (2) Unit 5/7 Stack.
const PAWN = "core-096"; // (1) Trap: answers only an attack that would be lethal.
const MANA_WELL = "core-006"; // (3) Field Spell.
const ANCHOR = "core-010"; // (0) Spell (§2.5).
const X = "core-020"; // library filler.

function lib(n: number): string[] {
  return Array.from({ length: n }, () => X);
}

function must<T>(value: T | null | undefined, what: string): T {
  if (value === null || value === undefined) throw new Error(`missing: ${what}`);
  return value;
}

function optionIds(s: Scenario): string[] {
  return must(s.state.pending, "an open prompt").options.flatMap((option) =>
    option.selection.pick === "instance" ? [option.selection.instanceId] : [],
  );
}

function placements(s: Scenario): number[] {
  return s.events.flatMap((event) => (event.type === "counterChanged" && event.placed !== undefined ? [event.placed] : []));
}

function drawsBy(events: readonly GameEvent[], player: PlayerId): number {
  return events.filter((event) => event.type === "drawn" && event.player === player).length;
}

/** Answer the one placement prompt on `card`: every placement lands there (R669). */
function placeAll(s: Scenario, card: CardInstance): void {
  s.answer(card.id);
}

function board(radiantFace = false): Scenario {
  return scenario({
    p1: { hand: [{ def: BOOK, radiant: radiantFace }, ANCHOR], field: [VANILLA], library: lib(3) },
    p2: { hand: [ANCHOR], field: [MENACE], backrow: [MANA_WELL, { def: PAWN, faceUp: false, lane: 2 }] },
  });
}

describe("C #70 Book of Plague", () => {
  it("is a Spell tagged Book and Plague with one number, and one script on both faces", () => {
    expect(def.id).toBe(BOOK);
    expect(def.type).toBe("Spell");
    expect(def.tags).toEqual(["Book", "Plague"]);
    expect(def.params).toEqual([{ key: "tokens", base: 5, radiant: 10, better: "up", step: 1, min: 1 }]);
    expect(radiant).toBe(base);
  });

  describe("base", () => {
    it("five placements of one token on the one permanent a single prompt names, over every permanent on either side, face-down included", () => {
      const s = board();
      const vanilla = s.card(VANILLA);
      const menace = s.card(MENACE);
      const well = s.card(MANA_WELL);
      const pawn = s.card(PAWN);

      s.play(BOOK);

      expect(s.state.pending?.playerId).toBe("p1");
      expect(new Set(optionIds(s))).toEqual(new Set([vanilla.id, menace.id, well.id, pawn.id]));
      // One answer puts all five on the pick: no second prompt opens.
      s.answer(menace.id);

      expect(s.state.pending).toBeNull();
      expect(placements(s)).toEqual([1, 1, 1, 1, 1]);
      expect([vanilla, menace, well, pawn].map((card) => s.card(card).counters.plague ?? 0)).toEqual([0, 5, 0, 0]);
      s.expectInZone(BOOK, "graveyard");
    });

    it("R669 the counters cannot be spread: the whole effect lands on the one pick", () => {
      const s = board();
      s.play(BOOK);
      placeAll(s, s.card(MENACE));

      expect(s.state.pending).toBeNull();
      expect(s.card(MENACE).counters.plague).toBe(5);
    });

    it("§3.2 R13 a card dormant under a Stack pile is no option; the top of the pile is", () => {
      const s = scenario({ p1: { hand: [BOOK, ANCHOR] }, p2: { hand: [ANCHOR], field: [VANILLA, { def: FIENDER, stack: true }] } });
      s.play(BOOK);

      expect(optionIds(s)).toEqual([s.card(FIENDER).id]);
    });

    it("with no permanent on the field it places nothing, asks nothing, and still resolves", () => {
      const s = scenario({ p1: { hand: [BOOK, ANCHOR] }, p2: { hand: [ANCHOR] } });

      s.play(BOOK);

      expect(s.state.pending).toBeNull();
      expect(placements(s)).toEqual([]);
      s.expectInZone(BOOK, "graveyard");
    });

    it("R177 a face-down enemy option carries only its id, and placing on it never names it to you", () => {
      const s = board();
      s.play(BOOK);
      const pawn = s.card(PAWN);

      const mine = must(s.view("p1").pending, "p1's view of the prompt");
      if (!mine.forYou) throw new Error("the prompt is p1's");
      const option = must(mine.options.find((entry) => entry.instanceId === pawn.id || entry.key.includes(pawn.id)), "the trap's option");
      expect(option.defId).toBeUndefined();
      expect(JSON.stringify(s.view("p1"))).not.toContain(PAWN);
      placeAll(s, pawn);

      expect(s.card(pawn).counters.plague).toBe(5);
      expect(JSON.stringify(s.view("p1"))).not.toContain(PAWN);
      expect(JSON.stringify(s.view("p1"))).not.toContain("My Pawn");
    });

    it("§10.6 the opponent sees only that a prompt is open", () => {
      const s = board();
      s.play(BOOK);

      expect(s.view("p2").pending).toEqual({ forYou: false, pendingFor: "p1" });
    });

    it("each placement is its own: a C #53 Plague Crawler that takes all five draws five times", () => {
      const s = scenario({ p1: { hand: [BOOK, ANCHOR], field: [CRAWLER], library: lib(6) }, p2: { hand: [ANCHOR] } });
      s.play(BOOK);

      placeAll(s, s.card(CRAWLER));

      expect(drawsBy(s.events, "p1")).toBe(5);
    });

    it("C #27 a Pestilent Slime doubles its share: all five on it put 10", () => {
      const s = scenario({ p1: { hand: [BOOK, ANCHOR], field: [SLIME, { def: VANILLA, lane: 2 }] }, p2: { hand: [ANCHOR] } });
      s.play(BOOK);
      const slime = s.card(SLIME);

      placeAll(s, slime);

      expect(s.card(slime).counters.plague).toBe(10);
      expect(s.card(VANILLA).counters.plague ?? 0).toBe(0);
      expect(placements(s)).toEqual([2, 2, 2, 2, 2]);
    });

    it("C #4 tagged Book: the opponent's Palantir steals it outright, and the stolen Book places nothing", () => {
      const s = scenario({ p1: { hand: [BOOK, ANCHOR], field: [VANILLA] }, p2: { hand: [ANCHOR], backrow: [PALANTIR] } });
      const book = s.card(BOOK);

      s.play(book);

      // No "you may": the steal is answered without asking, so no prompt ever opens.
      expect(s.state.pending).toBeNull();
      expect(placements(s)).toEqual([]);
      expect(s.hand("p2").map((card) => card.id)).toContain(book.id);
      // Palantir paid its price.
      s.expectInZone(PALANTIR, "graveyard");
    });

    it("§9.3 the open prompt survives a JSON round trip and finishes as the live one does", () => {
      const s = board();
      s.play(BOOK);
      const menace = s.card(MENACE);

      const revived = JSON.parse(JSON.stringify(s.state)) as GameState;
      expect(revived).toEqual(s.state);
      const choice = must(revived.pending, "the placement prompt");
      const result = reduce(revived, {
        type: "answer",
        playerId: "p1",
        choiceId: choice.id,
        selection: [{ pick: "instance", instanceId: menace.id }],
        nonce: "book-json-0",
      });
      expect(result.error).toBeUndefined();
      placeAll(s, menace);

      expect(result.state.pending).toBeNull();
      expect(hashState(result.state)).toBe(hashState(s.state));
      expect(s.card(menace).counters.plague).toBe(5);
    });

    it("R386 an Upgrade places six", () => {
      const s = board();
      stepParam(s.card(BOOK), "tokens", 1);
      s.play(BOOK);

      placeAll(s, s.card(MENACE));

      expect(s.state.pending).toBeNull();
      expect(s.card(MENACE).counters.plague).toBe(6);
    });
  });

  describe("radiant", () => {
    it("ten placements on the one pick", () => {
      const s = board(true);
      s.play(BOOK);
      const menace = s.card(MENACE);

      placeAll(s, menace);

      expect(s.state.pending).toBeNull();
      expect(s.card(menace).counters.plague).toBe(10);
      expect(placements(s)).toHaveLength(10);
    });

    it("R386 a Degrade places nine", () => {
      const s = board(true);
      stepParam(s.card(BOOK), "tokens", -1);
      s.play(BOOK);

      placeAll(s, s.card(MENACE));

      expect(s.state.pending).toBeNull();
      expect(s.card(MENACE).counters.plague).toBe(9);
    });
  });
});
