// C #37 Last Hurrah — SPEC §8.6 row 37, BUILD M9 Classic row C 37: "Draws your whole deck, as many
// draws as its size when the effect starts (R58), the cards past 10 burning (§2.4, R317) and a draw
// limit stopping the rest; an empty deck draws nothing and takes no fatigue; then at the end of this
// turn, in the end-of-turn delayed-effect step (R62), your whole hand is discarded with no prompt, a
// discard (C #64 sees it); the drawn cards are never named in the opponent's view; radiant: the discard
// comes at the end of your next turn instead, taking the hand you hold then, and nothing is discarded
// at this turn's end; no tuned numbers".
//
// The draw-limit case uses C #4 Palantir's aura ("your opponent can't draw more than 1 card each
// turn"), whose own test file proves the limit in full.

import { describe, expect, it } from "vitest";
import { HAND_CAP } from "@jackioh/engine";
import type { GameEvent } from "@jackioh/shared";
import { scenario, type Scenario, type SideSetup } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic/037-last-hurrah";

const HURRAH = "classic-037";
const PALANTIR = "classic-004";
const FILLER = "core-005"; // (1) Spell.
const VANILLA = "core-008";
const TIMMY = "core-011";
const MENACE = "core-019";
const POINTMASTER = "core-020";
const FELINORS = "core-012";

/** Five distinct cards, top first, so each draw is visible by def. */
const DECK = [VANILLA, TIMMY, MENACE, POINTMASTER, FELINORS] as const;

function setup(p1: SideSetup = {}, p2: SideSetup = {}, radiantFace = false): Scenario {
  return scenario({
    p1: { hand: [{ def: HURRAH, radiant: radiantFace }, "core-010"], library: [...DECK], ...p1 },
    p2: { hand: [FILLER, FILLER], library: [FILLER, FILLER, FILLER, FILLER], ...p2 },
  });
}

function count(events: readonly GameEvent[], type: GameEvent["type"]): number {
  return events.filter((event) => event.type === type).length;
}

describe("C #37 Last Hurrah", () => {
  it("has no declared numbers", () => {
    expect(def.id).toBe(HURRAH);
    expect(def.params).toBeUndefined();
    expect(base.cry).toBeTypeOf("function");
    expect(radiant.cry).toBeTypeOf("function");
  });

  describe("base", () => {
    it("R58 draws your whole deck: as many draws as it holds when the effect starts", () => {
      const s = setup();

      s.play(HURRAH);

      expect(s.pile("p1", "library")).toHaveLength(0);
      expect(s.hand("p1").map((card) => card.defId)).toEqual(["core-010", ...DECK]);
      expect(count(s.lastEvents, "drawn")).toBe(DECK.length);
      expect(count(s.lastEvents, "fatigue")).toBe(0);
    });

    it("R4 R317 the cards past the hand cap of 10 burn into the graveyard, both players reading which", () => {
      const eight = [FILLER, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER];
      const s = setup({ hand: [HURRAH, "core-010", ...eight] });
      expect(s.hand("p1")).toHaveLength(10);

      s.play(HURRAH);

      expect(s.hand("p1")).toHaveLength(HAND_CAP);
      expect(s.pile("p1", "library")).toHaveLength(0);
      expect(count(s.lastEvents, "burned")).toBe(DECK.length - 1);
      expect(s.pile("p1", "graveyard").map((card) => card.defId)).toEqual(expect.arrayContaining([MENACE, POINTMASTER, FELINORS]));
      // R317: both players read which cards burned; the one kept in hand stays unread by p2.
      const burned = (player: "p1" | "p2"): string[] =>
        s.view(player).events.flatMap((event) => (event.type === "burned" ? [event.defId] : []));
      expect(burned("p2")).toEqual(burned("p1"));
      expect(burned("p2")).toEqual(expect.arrayContaining([MENACE, POINTMASTER, FELINORS]));
      expect(JSON.stringify(s.view("p2"))).not.toContain(VANILLA);
    });

    it("§2.4 a draw limit stops the rest: those draws do not happen at all", () => {
      const s = setup({}, { backrow: [{ def: PALANTIR, faceUp: true }] });

      s.play(HURRAH);

      expect(count(s.lastEvents, "drawn")).toBe(1);
      expect(s.pile("p1", "library")).toHaveLength(DECK.length - 1);
      expect(count(s.lastEvents, "fatigue")).toBe(0);
      expect(count(s.lastEvents, "drawLimited")).toBeGreaterThan(0);
    });

    it("an empty deck draws nothing and takes no fatigue", () => {
      const s = setup({ library: [] });

      s.play(HURRAH);

      expect(count(s.lastEvents, "drawn")).toBe(0);
      expect(count(s.lastEvents, "fatigue")).toBe(0);
      s.expectHealth("p1", 30);
    });

    it("R97 the drawn cards are never named in the opponent's view", () => {
      const s = setup();

      s.play(HURRAH);

      const seen = JSON.stringify(s.view("p2"));
      for (const defId of [TIMMY, MENACE, POINTMASTER, FELINORS]) expect(seen).not.toContain(defId);
    });

    it("R62 at the end of this turn the whole hand is discarded, with no prompt, before the next turn starts", () => {
      const s = setup();
      s.play(HURRAH);
      const held = s.hand("p1").map((card) => card.id);

      s.endTurn();

      expect(s.hand("p1")).toHaveLength(0);
      expect(s.pile("p1", "graveyard").map((card) => card.id)).toEqual(expect.arrayContaining(held));
      // R62: `turnEnded` closes the end-of-turn triggers; the trap window and then the end-of-turn
      // delayed effects follow it, all before the next turn starts.
      const types = s.lastEvents.map((event) => event.type);
      const firstDiscard = types.indexOf("discarded");
      const ended = types.indexOf("turnEnded");
      const started = types.lastIndexOf("turnStarted");
      expect(ended).toBeGreaterThanOrEqual(0);
      expect(firstDiscard).toBeGreaterThan(ended);
      expect(firstDiscard).toBeLessThan(started);
      expect(s.lastEvents.filter((event) => event.type === "promptOpened")).toHaveLength(0);
    });

    it("§6.3 each card goes as a discard, with its own discarded event", () => {
      const s = setup();
      s.play(HURRAH);
      const held = s.hand("p1").length;

      s.endTurn();

      expect(s.lastEvents.filter((event) => event.type === "discarded" && event.owner === "p1")).toHaveLength(held);
    });

    it("it discards only its controller's hand", () => {
      const s = setup();
      s.play(HURRAH);

      s.endTurn();

      // p2 has drawn for their turn and still holds everything.
      expect(s.hand("p2").length).toBeGreaterThanOrEqual(2);
    });

    it("the discard takes the hand as it is then: cards played in between are not discarded", () => {
      const s = setup();
      s.play(HURRAH);
      s.play(TIMMY, { zone: 1 });

      s.endTurn();

      s.expectInZone(TIMMY, "field");
      expect(s.hand("p1")).toHaveLength(0);
    });
  });

  describe("radiant", () => {
    it("R58 draws your whole deck", () => {
      const s = setup({}, {}, true);

      s.play(HURRAH);

      expect(s.pile("p1", "library")).toHaveLength(0);
      expect(s.hand("p1")).toHaveLength(DECK.length + 1);
    });

    it("nothing is discarded at this turn's end", () => {
      const s = setup({}, {}, true);
      s.play(HURRAH);
      const held = s.hand("p1").length;

      s.endTurn();

      expect(s.hand("p1")).toHaveLength(held);
      expect(s.lastEvents.some((event) => event.type === "discarded" && event.owner === "p1")).toBe(false);
    });

    it("R62 at the end of your next turn it discards the hand you hold then", () => {
      const s = setup({}, {}, true);
      s.play(HURRAH);
      s.play(TIMMY, { zone: 1 });
      s.endTurn();
      s.endTurn();
      expect(s.state.active).toBe("p1");
      // p1's start-of-turn draw found an empty deck: fatigue, no card.
      const held = s.hand("p1").map((card) => card.id);
      expect(held.length).toBeGreaterThan(0);

      s.endTurn();

      expect(s.hand("p1")).toHaveLength(0);
      expect(s.pile("p1", "graveyard").map((card) => card.id)).toEqual(expect.arrayContaining(held));
      s.expectInZone(TIMMY, "field");
    });
  });
});
