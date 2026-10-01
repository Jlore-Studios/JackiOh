// C #64 Malzahar's Recycler — SPEC §8.6 row 64, BUILD M9 Classic row C 64: "End of your turn: discard
// 2 cards of your choice (R16; fewer → all, none → nothing); whenever you discard, draw as many as that
// effect discarded, one answer per discarding effect (its own end-of-turn discard draws 2); every
// discard of yours counts, chosen or random (C #15, C #26, C #37, C #8 played against you), an
// opponent's discard does not, and a Brittle crumble is no discard; the discard prompt's options and
// the drawn cards are never named in the opponent's view; radiant: each discarding effect draws your
// whole deck instead (R58); no tuned numbers".
//
// The discards of C #15, #26, #37 and #8 are other workstreams' cards; a random discard is shown with
// Core #80 Zao Gao ("Discard 2 random cards"), a chosen one with Core #21 Hinder's cast-on-draw
// discard and C #89 Paul Allen's Ghost's targeting cost.

import { reduce, type GameState } from "@jackioh/engine";
import type { Action, GameEvent, PlayerId, Selection } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { base, def, radiant } from "../../src/scripts/classic/064-malzahars-recycler";
import { scenario, type Scenario } from "../_harness";

const RECYCLER = "classic-064";
const ZAO_GAO = "core-080"; // (2) Spell: Discard 2 random cards. Summon 2 Rush Tokens …
const HINDER = "core-021"; // (0) Spell, Cast on draw: … Discard 1.
const STOCKPILE = "core-005"; // (1) Spell: Draw 2.
const VANILLA = "core-008"; // (1) Unit 4/4
const MENACE = "core-019"; // (3) Unit 9/9
const FILLER = "core-010"; // (0) Spell

function drawnBy(events: readonly GameEvent[], player: PlayerId): GameEvent[] {
  return events.filter((event) => event.type === "drawn" && event.player === player);
}

function discardedBy(events: readonly GameEvent[], player: PlayerId): GameEvent[] {
  return events.filter((event) => event.type === "discarded" && event.owner === player);
}

function pick(s: Scenario, ...defIds: string[]): Selection[] {
  const used = new Set<string>();
  return defIds.map((defId) => {
    const card = s.hand("p1").find((held) => held.defId === defId && !used.has(held.id));
    if (card === undefined) throw new Error(`no ${defId} in hand`);
    used.add(card.id);
    return { pick: "instance", instanceId: card.id };
  });
}

/** p1's Recycler on the field; p1 to end its turn. */
function recycling(opts: { radiant?: boolean; hand: readonly string[]; library?: readonly string[] }): Scenario {
  return scenario({
    p1: {
      hand: [...opts.hand],
      backrow: [{ def: RECYCLER, radiant: opts.radiant === true }],
      library: [...(opts.library ?? [VANILLA, VANILLA, VANILLA, VANILLA])],
    },
    p2: { hand: [FILLER], library: [VANILLA, VANILLA, VANILLA] },
  });
}

describe("C #64 Malzahar's Recycler", () => {
  it("is a (2) Field Spell with an end-of-turn discard and a discard trigger, no tuned numbers", () => {
    expect(def.type).toBe("Field Spell");
    expect(def.cost).toBe(2);
    expect(def.params).toBeUndefined();
    expect(base.endOfTurn).toBeTypeOf("function");
    expect(base.triggers?.map((trigger) => trigger.on)).toEqual([["discarded"]]);
    expect(radiant.triggers?.map((trigger) => trigger.on)).toEqual([["discarded"]]);
  });

  describe("base", () => {
    it("R16 at the end of your turn you pick 2 cards to discard, and its own discard draws 2", () => {
      const s = recycling({ hand: [MENACE, VANILLA, FILLER] });
      s.endTurn();
      const pending = s.state.pending;
      expect(pending).toMatchObject({ playerId: "p1", kind: "hand", min: 2, max: 2 });
      expect(pending?.options).toHaveLength(3);
      s.answer(pick(s, MENACE, VANILLA));
      expect(discardedBy(s.lastEvents, "p1").map((event) => (event as { defId: string }).defId)).toEqual([MENACE, VANILLA]);
      expect(s.state.active).toBe("p2");
      // The two draws happen at p1's end of turn, before p2's turn starts.
      const events = s.lastEvents;
      const p2Starts = events.findIndex((event) => event.type === "turnStarted");
      expect(drawnBy(events.slice(0, p2Starts), "p1")).toHaveLength(2);
      expect(s.hand("p1")).toHaveLength(3);
    });

    it("R16 with fewer than 2 cards it discards them all, and draws that many", () => {
      const s = recycling({ hand: [MENACE] });
      s.endTurn();
      expect(s.state.pending).toMatchObject({ min: 1, max: 1 });
      s.answer(pick(s, MENACE));
      const events = s.lastEvents;
      const p2Starts = events.findIndex((event) => event.type === "turnStarted");
      expect(discardedBy(events, "p1")).toHaveLength(1);
      expect(drawnBy(events.slice(0, p2Starts), "p1")).toHaveLength(1);
    });

    it("with an empty hand it asks nothing and draws nothing", () => {
      const s = recycling({ hand: [] });
      s.endTurn();
      expect(s.events.some((event) => event.type === "promptOpened")).toBe(false);
      const p2Starts = s.lastEvents.findIndex((event) => event.type === "turnStarted");
      expect(drawnBy(s.lastEvents.slice(0, p2Starts), "p1")).toHaveLength(0);
    });

    it("§9.3 its discard prompt survives a JSON round trip and answers the same way", () => {
      const s = recycling({ hand: [MENACE, VANILLA, FILLER] });
      s.endTurn();
      const round = JSON.parse(JSON.stringify(s.state)) as GameState;
      const action: Action = {
        type: "answer",
        playerId: "p1",
        nonce: "c64-round-trip",
        choiceId: s.state.pending?.id ?? "",
        selection: pick(s, MENACE, FILLER),
      };
      const live = reduce(s.state, action);
      const revived = reduce(round, action);
      expect(live.error).toBeUndefined();
      expect(revived.state).toEqual(live.state);
      expect(live.state.players.p1.graveyard.map((card) => card.defId)).toEqual([MENACE, FILLER]);
    });

    it("a random discard of yours counts: Zao Gao's 2 random discards draw 2", () => {
      const s = recycling({ hand: [ZAO_GAO, MENACE, VANILLA, FILLER] });
      s.play(ZAO_GAO);
      expect(discardedBy(s.lastEvents, "p1")).toHaveLength(2);
      expect(drawnBy(s.lastEvents, "p1")).toHaveLength(2);
    });

    it("R16 a chosen discard of yours counts: Hinder cast on draw makes you discard 1, and you draw 1", () => {
      const s = recycling({ hand: [STOCKPILE, MENACE], library: [HINDER, VANILLA, VANILLA, VANILLA, VANILLA] });
      s.play(STOCKPILE);
      // Hinder is cast on the first draw and asks for its discard (R16).
      expect(s.state.pending?.kind).toBe("hand");
      s.answer(pick(s, MENACE));
      expect(discardedBy(s.lastEvents, "p1")).toHaveLength(1);
      // Stockpile's two draws (the first repeating past Hinder) and the Recycler's one.
      expect(drawnBy(s.events, "p1")).toHaveLength(4);
    });

    it("a targeting cost's discards count: C #89's two draw two", () => {
      const s = scenario({
        p1: { hand: ["classic-055", FILLER, FILLER], backrow: [RECYCLER], library: [VANILLA, VANILLA, VANILLA] },
        p2: { field: ["classic-089"] },
      });
      const [a, b] = s.hand("p1").filter((card) => card.defId === FILLER);
      const result = reduce(s.state, {
        type: "play",
        playerId: "p1",
        nonce: "c64-ghost",
        instanceId: s.card("classic-055").id,
        targets: [{ pick: "instance", instanceId: s.card("classic-089").id }],
        discards: [a?.id ?? "", b?.id ?? ""],
      });
      expect(result.error).toBeUndefined();
      expect(drawnBy(result.events, "p1")).toHaveLength(2);
    });

    it("an opponent's discard does not: their Zao Gao draws you nothing", () => {
      const s = scenario({
        p1: { hand: [FILLER], backrow: [RECYCLER], library: [VANILLA, VANILLA] },
        p2: { hand: [ZAO_GAO, MENACE, VANILLA, FILLER], library: [VANILLA] },
        active: "p2",
      });
      s.play(ZAO_GAO);
      expect(discardedBy(s.lastEvents, "p2")).toHaveLength(2);
      expect(drawnBy(s.lastEvents, "p1")).toHaveLength(0);
      expect(drawnBy(s.lastEvents, "p2")).toHaveLength(0);
    });

    it("§6.1 a Brittle crumble in your hand is no discard: it draws nothing", () => {
      const s = scenario({
        p1: { hand: [MENACE, FILLER], backrow: [RECYCLER], library: [VANILLA, VANILLA, VANILLA] },
        p2: { hand: [FILLER], library: [VANILLA, VANILLA] },
        active: "p2",
      });
      s.card(MENACE).brittle = { count: 1, since: 1 };
      s.endTurn();
      expect(s.state.active).toBe("p1");
      expect(s.lastEvents.some((event) => event.type === "crumbled")).toBe(true);
      expect(discardedBy(s.lastEvents, "p1")).toHaveLength(0);
      // Only the turn's own draw.
      expect(drawnBy(s.lastEvents, "p1")).toHaveLength(1);
    });

    it("R97 R81 its prompt's options and the cards drawn are never named in the opponent's view", () => {
      const s = recycling({ hand: [MENACE, VANILLA, FILLER] });
      s.endTurn();
      const asked = s.view("p2");
      // §10.6: the opponent sees that a prompt is open, never its options.
      expect(asked.pending).toEqual({ forYou: false, pendingFor: "p1" });
      expect(JSON.stringify(asked)).not.toContain(MENACE);
      s.answer(pick(s, MENACE, VANILLA));
      const drawnIds = drawnBy(s.lastEvents, "p1").map((event) => (event as { instanceId: string }).instanceId);
      const theirs = s.view("p2");
      for (const event of theirs.events.filter((e) => e.type === "drawn" && e.player === "p1")) {
        expect(drawnIds).not.toContain((event as { instanceId: string }).instanceId);
      }
      // The discards are public: the graveyard is.
      expect(theirs.opponent.graveyard.map((card) => card.defId)).toEqual([MENACE, VANILLA]);
    });
  });

  describe("radiant", () => {
    it("R58 its own end-of-turn discard draws your whole deck, as its size stands then", () => {
      const s = recycling({ radiant: true, hand: [MENACE, VANILLA], library: [VANILLA, VANILLA, VANILLA, VANILLA, VANILLA] });
      s.endTurn();
      s.answer(pick(s, MENACE, VANILLA));
      const events = s.lastEvents;
      const p2Starts = events.findIndex((event) => event.type === "turnStarted");
      expect(drawnBy(events.slice(0, p2Starts), "p1")).toHaveLength(5);
      expect(s.pile("p1", "library")).toHaveLength(0);
      // Each discard is answered: the second finds the deck empty and draws nothing, and no fatigue.
      expect(s.state.players.p1.fatigueCount).toBe(0);
    });

    it("R58 §2.4 a deck bigger than the hand's room burns past the hand cap", () => {
      const library = Array.from({ length: 12 }, () => VANILLA);
      const s = recycling({ radiant: true, hand: [MENACE], library });
      s.endTurn();
      s.answer(pick(s, MENACE));
      expect(s.hand("p1")).toHaveLength(10);
      expect(s.events.filter((event) => event.type === "burned")).toHaveLength(2);
    });

    it("a random discard of yours draws your whole deck too", () => {
      const s = recycling({ radiant: true, hand: [ZAO_GAO, MENACE, VANILLA], library: [VANILLA, VANILLA, VANILLA] });
      s.play(ZAO_GAO);
      expect(drawnBy(s.lastEvents, "p1")).toHaveLength(3);
      expect(s.pile("p1", "library")).toHaveLength(0);
    });

    it("an opponent's discard still draws you nothing", () => {
      const s = scenario({
        p1: { hand: [FILLER], backrow: [{ def: RECYCLER, radiant: true }], library: [VANILLA, VANILLA] },
        p2: { hand: [ZAO_GAO, MENACE, VANILLA, FILLER] },
        active: "p2",
      });
      s.play(ZAO_GAO);
      expect(drawnBy(s.lastEvents, "p1")).toHaveLength(0);
    });
  });
});
