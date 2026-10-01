// C #76 Plague Bringer — SPEC §8.6 row 76, BUILD M9 Classic row C 76: "Rush; Cry: two placements of one
// Plague Token, one prompt each, on any permanent either side (a face-down option carries only its id,
// R177), then draw 1; radiant 8/8: four placements, draw 2; its tuned numbers (tokens, draw) read
// through `param()` (R386)".

import { hashState, reduce, stepParam, type CardInstance, type GameState } from "@jackioh/engine";
import type { GameEvent, PlayerId } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { scenario, type Scenario } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic/076-plague-bringer";

const BRINGER = "classic-076";
const CRAWLER = "classic-053"; // (1) Unit: whenever Plague Tokens are placed on this, draw 1.
const VANILLA = "core-008"; // (1) Unit 4/4.
const MENACE = "core-019"; // (3) Unit 9/9 Taunt.
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

function drawsBy(events: readonly GameEvent[], player: PlayerId): number {
  return events.filter((event) => event.type === "drawn" && event.player === player).length;
}

function placeAll(s: Scenario, card: CardInstance, times: number): void {
  for (let n = 0; n < times; n += 1) s.answer(card.id);
}

function board(radiantFace = false, library = 4): Scenario {
  return scenario({
    p1: { hand: [{ def: BRINGER, radiant: radiantFace }, ANCHOR], library: lib(library) },
    p2: { hand: [ANCHOR], field: [{ def: VANILLA, lane: 2 }], backrow: [MANA_WELL, { def: PAWN, faceUp: false, lane: 2 }], health: 20 },
  });
}

describe("C #76 Plague Bringer", () => {
  it("declares its two numbers and one script on both faces; Rush is printed", () => {
    expect(def.id).toBe(BRINGER);
    expect(def.params).toEqual([
      { key: "tokens", base: 2, radiant: 4, better: "up", step: 1, min: 1 },
      { key: "draw", base: 1, radiant: 2, better: "up", step: 1, min: 1 },
    ]);
    expect(def.base.keywords).toEqual([{ kind: "Rush" }]);
    expect(def.radiant.keywords).toEqual([{ kind: "Rush" }]);
    expect(radiant).toBe(base);
  });

  describe("base", () => {
    it("is a 4/4 with Rush: it attacks a Unit the turn it enters", () => {
      const s = board();
      s.play(BRINGER);
      const bringer = s.card(BRINGER);
      placeAll(s, bringer, 2);
      const theirs = must(s.unit("p2", 2), "p2's Vanilla");

      s.expectStats(bringer, { attack: 4, health: 4 });
      s.attack(bringer, theirs);

      s.expectInZone(theirs, "graveyard");
      expect(() => s.attack(bringer, "hero")).toThrow();
    });

    it("Cry: two placements, one prompt each, over every permanent on either side, itself and face-down cards included", () => {
      const s = board();
      s.play(BRINGER);
      const bringer = s.card(BRINGER);

      expect(new Set(optionIds(s))).toEqual(new Set([bringer.id, s.card(VANILLA).id, s.card(MANA_WELL).id, s.card(PAWN).id]));
      s.answer(s.card(VANILLA).id);
      expect(s.state.pending).not.toBeNull();
      s.answer(s.card(MANA_WELL).id);

      expect(s.state.pending).toBeNull();
      expect(s.card(VANILLA).counters.plague).toBe(1);
      expect(s.card(MANA_WELL).counters.plague).toBe(1);
    });

    it("repeats are allowed; then draw 1, after the last placement", () => {
      const s = board();
      s.play(BRINGER);
      const vanilla = s.card(VANILLA);

      s.answer(vanilla.id);
      expect(drawsBy(s.events, "p1")).toBe(0);
      s.answer(vanilla.id);

      expect(s.card(vanilla).counters.plague).toBe(2);
      expect(drawsBy(s.lastEvents, "p1")).toBe(1);
    });

    it("R177 a face-down enemy option carries only its id, and the placement on it never names it to you", () => {
      const s = board();
      s.play(BRINGER);
      const pawn = s.card(PAWN);

      const mine = must(s.view("p1").pending, "p1's view of the prompt");
      if (!mine.forYou) throw new Error("the prompt is p1's");
      const option = must(mine.options.find((entry) => entry.instanceId === pawn.id || entry.key.includes(pawn.id)), "the trap's option");
      expect(option.defId).toBeUndefined();
      placeAll(s, pawn, 2);

      expect(s.card(pawn).counters.plague).toBe(2);
      expect(JSON.stringify(s.view("p1"))).not.toContain(PAWN);
      expect(s.view("p2").you.backrow.some((card) => card !== null && "defId" in card && card.defId === PAWN)).toBe(true);
    });

    it("§10.6 the opponent sees only that a prompt is open", () => {
      const s = board();
      s.play(BRINGER);
      expect(s.view("p2").pending).toEqual({ forYou: false, pendingFor: "p1" });
    });

    it("each placement is its own: a C #53 Plague Crawler that takes both draws twice", () => {
      const s = scenario({ p1: { hand: [BRINGER, ANCHOR], field: [CRAWLER], library: lib(4) }, p2: { hand: [ANCHOR] } });
      s.play(BRINGER);

      placeAll(s, s.card(CRAWLER), 2);

      // Two Crawler draws and the Bringer's own.
      expect(drawsBy(s.events, "p1")).toBe(3);
    });

    it("§9.3 the chain paused after one placement survives a JSON round trip", () => {
      const s = board();
      s.play(BRINGER);
      const vanilla = s.card(VANILLA);
      s.answer(vanilla.id);

      const revived = JSON.parse(JSON.stringify(s.state)) as GameState;
      expect(revived).toEqual(s.state);
      const choice = must(revived.pending, "the second placement");
      const result = reduce(revived, {
        type: "answer",
        playerId: "p1",
        choiceId: choice.id,
        selection: [{ pick: "instance", instanceId: vanilla.id }],
        nonce: "bringer-json",
      });
      expect(result.error).toBeUndefined();
      s.answer(vanilla.id);

      expect(hashState(result.state)).toBe(hashState(s.state));
      expect(drawsBy(result.events, "p1")).toBe(1);
    });

    it("§2.4 with an empty deck the draw is fatigue", () => {
      const s = board(false, 0);
      s.play(BRINGER);
      placeAll(s, s.card(VANILLA), 2);

      expect(s.lastEvents.filter((event) => event.type === "fatigue" && event.player === "p1")).toHaveLength(1);
    });

    it("R386 an Upgrade asks three times and draws 2", () => {
      const s = board();
      stepParam(s.card(BRINGER), "tokens", 1);
      stepParam(s.card(BRINGER), "draw", 1);
      s.play(BRINGER);

      placeAll(s, s.card(VANILLA), 3);

      expect(s.state.pending).toBeNull();
      expect(s.card(VANILLA).counters.plague).toBe(3);
      expect(drawsBy(s.lastEvents, "p1")).toBe(2);
    });
  });

  describe("radiant", () => {
    it("is an 8/8 with Rush; four placements, then draw 2", () => {
      const s = board(true);
      s.play(BRINGER);
      const bringer = s.card(BRINGER);

      s.expectStats(bringer, { attack: 8, health: 8 });
      placeAll(s, s.card(VANILLA), 3);
      expect(s.state.pending).not.toBeNull();
      expect(drawsBy(s.events, "p1")).toBe(0);
      s.answer(s.card(MANA_WELL).id);

      expect(s.card(VANILLA).counters.plague).toBe(3);
      expect(s.card(MANA_WELL).counters.plague).toBe(1);
      expect(drawsBy(s.lastEvents, "p1")).toBe(2);
    });

    it("R386 a Degrade asks three times and draws 1", () => {
      const s = board(true);
      stepParam(s.card(BRINGER), "tokens", -1);
      stepParam(s.card(BRINGER), "draw", -1);
      s.play(BRINGER);

      placeAll(s, s.card(VANILLA), 3);

      expect(s.state.pending).toBeNull();
      expect(drawsBy(s.lastEvents, "p1")).toBe(1);
    });

    it("§10.6 the opponent's Menace is offered too, and the opponent sees only that a prompt is open", () => {
      const s = scenario({ p1: { hand: [{ def: BRINGER, radiant: true }, ANCHOR], library: lib(3) }, p2: { hand: [ANCHOR], field: [MENACE] } });
      s.play(BRINGER);

      expect(optionIds(s)).toContain(s.card(MENACE).id);
      expect(s.view("p2").pending).toEqual({ forYou: false, pendingFor: "p1" });
    });
  });
});
