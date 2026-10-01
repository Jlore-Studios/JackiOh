// C #63 Crop Dusting — SPEC §8.6 row 63, BUILD M9 Classic row C 63: "Face-down (R33); fires at the start
// of your next turn with the start-of-turn triggers (R62) and goes to the graveyard; places 1 Plague
// Token on each permanent on the field, both sides, face-down ones included (C #27 doubles its own),
// then draws 1; the placement on a face-down card never names it to the player who can't read it (R97);
// radiant: 2 tokens each and draw 2 (the adopted Radiant face, R276); its tuned numbers (tokens, draw)
// read through `param()` (R386)".
//
// The C #27 Pestilent Slime case needs C #27's script (cards-classic-a) registered.

import { stepParam, type CardInstance } from "@jackioh/engine";
import type { GameEvent, PlayerId } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { scenario, type Scenario } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic/063-crop-dusting";

const DUSTING = "classic-063";
const CRAWLER = "classic-053"; // (1) Unit: whenever Plague Tokens are placed on this, draw 1.
const SLIME = "classic-027"; // (0) Unit: Plague Tokens placed on this are doubled.
const VANILLA = "core-008"; // (1) Unit 4/4.
const MENACE = "core-019"; // (3) Unit 9/9 Taunt.
const FIENDER = "core-092"; // (2) Unit 5/7 Stack.
const PAWN = "core-096"; // (1) Trap: answers only an attack that would be lethal.
const MANA_WELL = "core-006"; // (3) Field Spell.
const FILLER = "core-005"; // (1) Spell, a card to keep a turn from auto-ending (§2.5).
const X = "core-020"; // library filler.
const Y = "core-011"; // library filler of another kind, to tell draws apart.

function lib(n: number, card = X): string[] {
  return Array.from({ length: n }, () => card);
}

function placedOn(s: Scenario): Map<string, number[]> {
  const out = new Map<string, number[]>();
  for (const event of s.events) {
    if (event.type !== "counterChanged" || event.placed === undefined) continue;
    out.set(event.instanceId, [...(out.get(event.instanceId) ?? []), event.placed]);
  }
  return out;
}

function drawsBy(events: readonly GameEvent[], player: PlayerId): GameEvent[] {
  return events.filter((event) => event.type === "drawn" && event.player === player);
}

/** p1 sets Crop Dusting on a board with permanents on both sides. */
function setUp(radiantFace = false): Scenario {
  const s = scenario({
    p1: {
      hand: [{ def: DUSTING, radiant: radiantFace }, FILLER],
      field: [VANILLA],
      backrow: [{ def: MANA_WELL, lane: 2 }],
      library: [...lib(2, Y), ...lib(4)],
    },
    p2: {
      hand: [FILLER],
      field: [MENACE],
      backrow: [{ def: PAWN, faceUp: false }],
      library: lib(4),
    },
  });
  s.play(DUSTING);
  return s;
}

describe("C #63 Crop Dusting", () => {
  it("is a Trap answering its controller's turn start, with two numbers and one script on both faces", () => {
    expect(def.id).toBe(DUSTING);
    expect(def.type).toBe("Trap");
    expect(base.triggers?.map((trigger) => [trigger.id, trigger.on])).toEqual([["crop-dusting", ["turnStarted"]]]);
    expect(def.params).toEqual([
      { key: "tokens", base: 1, radiant: 2, better: "up", step: 1, min: 1 },
      { key: "draw", base: 1, radiant: 2, better: "up", step: 1, min: 1 },
    ]);
    expect(radiant).toBe(base);
  });

  describe("base", () => {
    it("R33 it is set face-down: the opponent sees a card back and never its name", () => {
      const s = setUp();
      const dusting = s.card(DUSTING);

      expect(dusting.zone.z).toBe("field");
      expect(s.card(dusting).faceUp).not.toBe(true);
      expect(JSON.stringify(s.view("p2"))).not.toContain(DUSTING);
    });

    it("it stays set through the opponent's turn start: their turnStarted is not its condition", () => {
      const s = setUp();
      const dusting = s.card(DUSTING);

      s.endTurn();

      expect(s.state.active).toBe("p2");
      s.expectInZone(dusting, "field");
      expect(s.events.some((event) => event.type === "trapFired")).toBe(false);
      expect(placedOn(s).size).toBe(0);
    });

    it("R62 R550 at the start of your next turn it fires: 1 token on each permanent on both sides, itself included, then draw 1, then the graveyard", () => {
      const s = setUp();
      const dusting = s.card(DUSTING);
      const vanilla = s.card(VANILLA);
      const well = s.card(MANA_WELL);
      const menace = s.card(MENACE);
      const pawn = s.card(PAWN);
      s.endTurn();

      s.endTurn();

      expect(s.state.active).toBe("p1");
      s.expectInZone(dusting, "graveyard");
      expect(s.lastEvents.some((event) => event.type === "trapFired" && event.instanceId === dusting.id)).toBe(true);
      for (const card of [vanilla, well, menace, pawn]) expect(s.card(card).counters.plague, card.defId).toBe(1);
      // The firing trap is a permanent too; its token goes with it to the graveyard (R78).
      expect([...placedOn(s).keys()]).toEqual([vanilla.id, dusting.id, well.id, menace.id, pawn.id]);
      expect([...placedOn(s).values()]).toEqual([[1], [1], [1], [1], [1]]);
      expect(s.card(dusting).counters.plague).toBeUndefined();
      // Its draw (the first of the library, Y) then the turn's draw (Y): the trap fired first.
      const draws = drawsBy(s.lastEvents, "p1");
      expect(draws).toHaveLength(2);
      const order = s.lastEvents.flatMap((event) =>
        event.type === "trapFired" || event.type === "counterChanged" || (event.type === "drawn" && event.player === "p1") ? [event.type] : [],
      );
      expect(order).toEqual(["trapFired", ...Array.from({ length: 5 }, () => "counterChanged"), "drawn", "drawn"]);
    });

    it("R97 the placement on the opponent's face-down trap never names it to you", () => {
      const s = setUp();
      s.endTurn();
      s.endTurn();

      expect(s.card(PAWN).counters.plague).toBe(1);
      expect(JSON.stringify(s.view("p1"))).not.toContain(PAWN);
      expect(JSON.stringify(s.view("p1"))).not.toContain("My Pawn");
    });

    it("R97 nor the placement on your own face-down trap to the opponent", () => {
      const s = scenario({
        p1: { hand: [DUSTING, FILLER], backrow: [{ def: PAWN, faceUp: false, lane: 2 }], library: lib(4) },
        p2: { hand: [FILLER], library: lib(4) },
      });
      s.play(DUSTING);
      s.endTurn();
      s.endTurn();

      expect(s.card(PAWN).counters.plague).toBe(1);
      expect(JSON.stringify(s.view("p2"))).not.toContain(PAWN);
      expect(JSON.stringify(s.view("p2"))).not.toContain("My Pawn");
    });

    it("§3.2 R13 a card dormant under a Stack pile takes none; the top of the pile does", () => {
      const s = scenario({
        p1: { hand: [DUSTING, FILLER], library: lib(4) },
        p2: { hand: [FILLER], field: [VANILLA, { def: FIENDER, stack: true }], library: lib(4) },
      });
      s.play(DUSTING);
      s.endTurn();
      s.endTurn();

      expect(s.card(FIENDER).counters.plague).toBe(1);
      expect(s.card(VANILLA).counters.plague).toBeUndefined();
    });

    it("with no other permanent on the field it still fires and draws, placing only on itself", () => {
      const s = scenario({ p1: { hand: [DUSTING, FILLER], library: lib(4) }, p2: { hand: [FILLER], library: lib(4) } });
      s.play(DUSTING);
      s.endTurn();
      s.endTurn();

      s.expectInZone(DUSTING, "graveyard");
      expect([...placedOn(s).keys()]).toEqual([s.card(DUSTING).id]);
      expect(drawsBy(s.lastEvents, "p1")).toHaveLength(2);
    });

    it("each placement is its own: a C #53 Plague Crawler on either side draws its controller 1", () => {
      const s = scenario({
        p1: { hand: [DUSTING, FILLER], field: [CRAWLER], library: lib(5) },
        p2: { hand: [FILLER], field: [CRAWLER], library: lib(5) },
      });
      s.play(DUSTING);
      s.endTurn();
      const p2Before = drawsBy(s.events, "p2").length;

      s.endTurn();

      // p1: the trap's draw, the Crawler's, and the turn's draw; p2: its Crawler's.
      expect(drawsBy(s.lastEvents, "p1")).toHaveLength(3);
      expect(drawsBy(s.events, "p2").length - p2Before).toBe(1);
    });

    it("C #27 a Pestilent Slime doubles its own placement: it takes 2", () => {
      const s = scenario({ p1: { hand: [DUSTING, FILLER], library: lib(4) }, p2: { hand: [FILLER], field: [SLIME, { def: VANILLA, lane: 2 }], library: lib(4) } });
      s.play(DUSTING);
      s.endTurn();
      s.endTurn();

      expect(s.card(SLIME).counters.plague).toBe(2);
      expect(s.card(VANILLA).counters.plague).toBe(1);
    });

    it("§2.4 a full hand burns the draw", () => {
      const s = scenario({
        p1: { hand: [DUSTING, ...Array.from({ length: 9 }, () => FILLER)], library: lib(4) },
        p2: { hand: [FILLER], library: lib(4) },
      });
      s.play(DUSTING);
      s.play(FILLER); // Stockpile: draw 2 refills the hand to 10.
      expect(s.hand("p1")).toHaveLength(10);
      s.endTurn();

      s.endTurn();

      expect(s.lastEvents.filter((event) => event.type === "burned")).toHaveLength(2);
      expect(s.hand("p1")).toHaveLength(10);
    });

    it("R386 an Upgrade places 2 on each permanent and draws 2", () => {
      const s = scenario({ p1: { hand: [DUSTING, FILLER], field: [VANILLA], library: lib(5) }, p2: { hand: [FILLER], field: [MENACE], library: lib(4) } });
      stepParam(s.card(DUSTING), "tokens", 1);
      stepParam(s.card(DUSTING), "draw", 1);
      s.play(DUSTING);
      s.endTurn();

      s.endTurn();

      expect([...placedOn(s).values()]).toEqual([[2], [2], [2]]);
      expect(drawsBy(s.lastEvents, "p1")).toHaveLength(3);
    });
  });

  describe("radiant", () => {
    it("R276 places 2 Plague Tokens on each permanent, one placement each, and draws 2", () => {
      const s = setUp(true);
      const cards: CardInstance[] = [s.card(VANILLA), s.card(MANA_WELL), s.card(MENACE), s.card(PAWN)];
      s.endTurn();

      s.endTurn();

      for (const card of cards) expect(s.card(card).counters.plague, card.defId).toBe(2);
      expect([...placedOn(s).values()]).toEqual([[2], [2], [2], [2], [2]]);
      // Its two draws, then the turn's.
      expect(drawsBy(s.lastEvents, "p1")).toHaveLength(3);
      s.expectInZone(DUSTING, "graveyard");
    });

    it("it too waits through the opponent's turn start", () => {
      const s = setUp(true);
      s.endTurn();
      expect(placedOn(s).size).toBe(0);
      s.expectInZone(DUSTING, "field");
    });

    it("R386 a Degrade places 1 on each and draws 1", () => {
      const s = scenario({ p1: { hand: [{ def: DUSTING, radiant: true }, FILLER], field: [VANILLA], library: lib(5) }, p2: { hand: [FILLER], library: lib(4) } });
      stepParam(s.card(DUSTING), "tokens", -1);
      stepParam(s.card(DUSTING), "draw", -1);
      s.play(DUSTING);
      s.endTurn();

      s.endTurn();

      expect(s.card(VANILLA).counters.plague).toBe(1);
      expect(drawsBy(s.lastEvents, "p1")).toHaveLength(2);
    });
  });
});
