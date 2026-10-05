// C #64 Malzahar's Recycler — SPEC §8.6 row 64, BUILD M9 Classic row C 64: "End of your turn: discard
// 2 cards at random (R661; fewer → all, none → nothing); whenever you discard, draw as many as that
// effect discarded, one answer per discarding effect (its own end-of-turn discard draws 2); every
// discard of yours counts (C #15, C #26, C #37, C #8 played against you), an opponent's discard does
// not, and a Brittle crumble is no discard; the drawn cards are never named in the opponent's view;
// radiant: each discarding effect draws your whole deck instead (R58); no tuned numbers".
//
// Every discard of yours is shown with the cards the row names — C #15 Nose Hunter's random one, C #26
// Rapid Draw's random four, C #37 Last Hurrah's whole hand, C #8 Pickle's played against you — and with
// Core #80 Zao Gao ("Discard 2 random cards"), Core #21 Hinder's cast-on-draw discard and C #89 Paul
// Allen's Ghost's targeting cost.

import { reduce } from "@jackioh/engine";
import type { GameEvent, PlayerId, Selection } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { base, def, radiant } from "../../src/scripts/classic/064-malzahars-recycler";
import { scenario, type Scenario } from "../_harness";
import { expectAnimated } from "../_animated";

const RECYCLER = "classic-064";
const ZAO_GAO = "core-080"; // (2) Spell: Discard 2 random cards. Summon 2 Rush Tokens …
const HINDER = "core-021"; // (0) Spell, Cast on draw: … Discard 1.
const STOCKPILE = "core-005"; // (1) Spell: Draw 2.
const VANILLA = "core-008"; // (1) Unit 4/4
const MENACE = "core-019"; // (3) Unit 9/9
const FILLER = "core-010"; // (0) Spell
const NOSE_HUNTER = "classic-015"; // Activate: Discard a random card. Exile the bottom card of their deck.
const RAPID_DRAW = "classic-026"; // (0) Spell: Draw 4. Then discard 4 cards.
const LAST_HURRAH = "classic-037"; // (0) Spell: Draw your deck. At the end of this turn, discard your hand.
const PICKLE = "classic-008"; // (1) Spell: your opponent chooses 3 times: discard 1, exile 1, or you draw 1.
const INCOME_TAX = "classic-009"; // Trap: when the cards your opponent has drawn in a turn reach 2 …

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
    it("R661 at the end of your turn 2 random cards are discarded, and its own discard draws 2", () => {
      const s = recycling({ hand: [MENACE, VANILLA, FILLER] });
      s.endTurn();
      // R661: no prompt opens — the two discards land at once, at random.
      expect(s.events.some((event) => event.type === "promptOpened" && (event as { player?: string }).player === "p1")).toBe(false);
      const discarded = discardedBy(s.lastEvents, "p1").map((event) => (event as { defId: string }).defId);
      expect(discarded).toHaveLength(2);
      expect([MENACE, VANILLA, FILLER]).toEqual(expect.arrayContaining(discarded));
      expect(s.state.active).toBe("p2");
      // The two draws happen at p1's end of turn, before p2's turn starts.
      const events = s.lastEvents;
      const p2Starts = events.findIndex((event) => event.type === "turnStarted");
      expect(drawnBy(events.slice(0, p2Starts), "p1")).toHaveLength(2);
      expect(s.hand("p1")).toHaveLength(3);
    });

    it("R661 the random discards come from the match rng: the same game discards the same cards", () => {
      const first = recycling({ hand: [MENACE, VANILLA, FILLER] });
      first.endTurn();
      const second = recycling({ hand: [MENACE, VANILLA, FILLER] });
      second.endTurn();
      const ids = (s: Scenario): string[] =>
        discardedBy(s.lastEvents, "p1").map((event) => (event as { instanceId: string }).instanceId);
      expect(ids(first)).toEqual(ids(second));
    });

    it("R661 with fewer than 2 cards it discards them all, and draws that many", () => {
      const s = recycling({ hand: [MENACE] });
      s.endTurn();
      expect(s.state.pending).toBeNull();
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

    it("a random discard of yours counts: Zao Gao's 2 random discards draw 2", () => {
      const s = recycling({ hand: [ZAO_GAO, MENACE, VANILLA, FILLER] });
      s.play(ZAO_GAO);
      expect(discardedBy(s.lastEvents, "p1")).toHaveLength(2);
      expect(drawnBy(s.lastEvents, "p1")).toHaveLength(2);
    });

    it("R661 a random discard of yours counts: Hinder cast on draw discards 1 at random, and you draw 1", () => {
      const s = recycling({ hand: [STOCKPILE, MENACE], library: [HINDER, VANILLA, VANILLA, VANILLA, VANILLA] });
      s.play(STOCKPILE);
      // Hinder is cast on the first draw and discards at random (R661): no prompt opens.
      expect(s.state.pending).toBeNull();
      expect(discardedBy(s.lastEvents, "p1")).toHaveLength(1);
      // Stockpile's two draws (the first repeating past Hinder) and the Recycler's one.
      expect(drawnBy(s.events, "p1")).toHaveLength(4);
    });

    it("a targeting cost's discards count: C #89's two random ones draw two", () => {
      const s = scenario({
        p1: { hand: ["classic-055", FILLER, FILLER], backrow: [RECYCLER], library: [VANILLA, VANILLA, VANILLA] },
        p2: { field: ["classic-089"] },
      });
      const result = reduce(s.state, {
        type: "play",
        playerId: "p1",
        nonce: "c64-ghost",
        instanceId: s.card("classic-055").id,
        targets: [{ pick: "instance", instanceId: s.card("classic-089").id }],
      });
      expect(result.error).toBeUndefined();
      expect(drawnBy(result.events, "p1")).toHaveLength(2);
    });

    it("C #15 Nose Hunter's random discard, its Activate's cost, draws 1", () => {
      const s = scenario({
        p1: { hand: [MENACE, FILLER], field: [NOSE_HUNTER], backrow: [RECYCLER], library: [VANILLA, VANILLA] },
        p2: { hand: [FILLER], library: [VANILLA, VANILLA] },
      });
      s.activate(NOSE_HUNTER);
      expect(discardedBy(s.lastEvents, "p1")).toHaveLength(1);
      expect(drawnBy(s.lastEvents, "p1")).toHaveLength(1);
      expect(s.hand("p1")).toHaveLength(2);
    });

    it("C #26 Rapid Draw's random four are one effect that discards 4, and draw 4", () => {
      const library = Array.from({ length: 8 }, () => VANILLA);
      const s = recycling({ hand: [RAPID_DRAW, FILLER], library });
      s.play(RAPID_DRAW);
      // R661: no prompt opens — four random cards go at once.
      expect(s.state.pending).toBeNull();
      expect(discardedBy(s.lastEvents, "p1")).toHaveLength(4);
      expect(drawnBy(s.lastEvents, "p1")).toHaveLength(8);
      expect(s.hand("p1")).toHaveLength(5);
      expect(s.pile("p1", "library")).toHaveLength(0);
    });

    it("C #37 Last Hurrah's end-of-turn discard of your hand draws that many: from the deck it emptied, fatigue", () => {
      const s = recycling({ hand: [LAST_HURRAH], library: [VANILLA, VANILLA, VANILLA, VANILLA, VANILLA] });
      s.play(LAST_HURRAH);
      expect(s.hand("p1")).toHaveLength(5);
      s.endTurn();
      // The Recycler's own end-of-turn discard comes first (two random discards, two fatigue draws) …
      // … then, after `turnEnded`, Last Hurrah discards the other three, and each is answered (R62).
      const events = s.lastEvents;
      const after = events.slice(events.findIndex((event) => event.type === "turnEnded"));
      const beforeNextTurn = after.slice(0, after.findIndex((event) => event.type === "turnStarted"));
      expect(discardedBy(beforeNextTurn, "p1")).toHaveLength(3);
      expect(beforeNextTurn.filter((event) => event.type === "fatigue" && event.player === "p1")).toHaveLength(3);
      expect(s.state.players.p1.fatigueCount).toBe(5);
    });

    it("C #8 Pickle played against you: the random discard is yours, and draws 1", () => {
      const s = scenario({
        p1: { hand: [MENACE, FILLER], backrow: [RECYCLER], library: [VANILLA, VANILLA, VANILLA, VANILLA] },
        p2: { hand: [PICKLE, FILLER], library: [VANILLA, VANILLA] },
        active: "p2",
      });
      s.play(PICKLE);
      expect(s.state.pending?.playerId).toBe("p1");
      s.answer("discard");
      // R661: the discard lands at once, at random — no second answer.
      expect(discardedBy(s.lastEvents, "p1")).toHaveLength(1);
      // The answer is a queued trigger (§10.3): it draws once Pickle has finished asking.
      s.answer("exile").answer("exile");
      expect(s.state.pending).toBeNull();
      expect(drawnBy(s.events, "p1")).toHaveLength(1);
      expect(s.hand("p1")).toHaveLength(2);
      expect(s.hand("p1").map((card) => card.defId)).toContain(VANILLA);
    });

    it("its draws are your draws: the second sets off the opponent's C #9 Income Tax", () => {
      const s = scenario({
        p1: { hand: [FILLER, FILLER, FILLER], backrow: [RECYCLER], library: [STOCKPILE, STOCKPILE, STOCKPILE] },
        p2: { hand: [FILLER], backrow: [{ def: INCOME_TAX, faceUp: false }], library: [VANILLA, VANILLA] },
      });
      s.endTurn();
      // R661: the Recycler's two discards are random, so no prompt opens — and the two draws set off the tax.
      expect(s.events.some((event) => event.type === "trapFired")).toBe(true);
      expect(s.state.pending).toMatchObject({ playerId: "p1", kind: "hand" });
      s.answer(pick(s, FILLER));
      expect(s.hand("p1").map((card) => card.defId)).toEqual([FILLER]);
      expect(s.hand("p2").filter((card) => card.defId === STOCKPILE && card.owner === "p2")).toHaveLength(2);
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

    it("§6.1 R638 a Brittle crumble is no discard: it draws nothing (and a card in your hand never crumbles)", () => {
      const s = scenario({
        p1: { hand: [FILLER], field: [{ def: MENACE, lane: 1 }], backrow: [RECYCLER], library: [VANILLA, VANILLA, VANILLA] },
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

    it("R97 R661 no prompt opens, and the cards drawn are never named in the opponent's view", () => {
      const s = recycling({ hand: [MENACE, VANILLA, FILLER] });
      s.endTurn();
      // R661: no prompt opens at all — nothing to redact options from.
      expect(s.state.pending).toBeNull();
      expect(s.view("p2").pending).toBeNull();
      const drawnIds = drawnBy(s.lastEvents, "p1").map((event) => (event as { instanceId: string }).instanceId);
      expect(drawnIds).toHaveLength(2);
      const theirs = s.view("p2");
      for (const event of theirs.events.filter((e) => e.type === "drawn" && e.player === "p1")) {
        expect(drawnIds).not.toContain((event as { instanceId: string }).instanceId);
      }
      // The discards are public: the graveyard is.
      const grave = theirs.opponent.graveyard.map((card) => card.defId);
      expect(grave).toHaveLength(2);
      expect([MENACE, VANILLA, FILLER]).toEqual(expect.arrayContaining(grave));
    });
  });

  describe("radiant", () => {
    it("R58 its own end-of-turn discard draws your whole deck, as its size stands then", () => {
      const s = recycling({ radiant: true, hand: [MENACE, VANILLA], library: [VANILLA, VANILLA, VANILLA, VANILLA, VANILLA] });
      s.endTurn();
      // R661: no prompt — and the whole two-card hand goes, so both discards are certain.
      expect(s.state.pending).toBeNull();
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
      // R661: the one-card hand goes with no prompt.
      expect(s.state.pending).toBeNull();
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

describe("C #64 Malzahar's Recycler: Animated (patch v0.2.10)", () => {
  it("R383 played, it animates into its lane's unit zone, else the leftmost open one, a 2/3 Unit; with none open it stays a Field Spell", () => {
    expectAnimated({ def: "classic-064", stats: { attack: 2, health: 3 } });
  });

  it("R383 radiant: a 4/6 Unit", () => {
    expectAnimated({ def: "classic-064", radiant: true, stats: { attack: 4, health: 6 } });
  });

  it("R383 played, it keeps its text as a Unit: its end-of-turn discard of 2 draws 2", () => {
    const s = scenario({
      p1: { hand: [RECYCLER, MENACE, VANILLA, FILLER], library: [VANILLA, VANILLA, VANILLA, VANILLA] },
      p2: { hand: [FILLER], library: [VANILLA, VANILLA, VANILLA] },
    });

    s.play(RECYCLER, { zone: 3 });
    expect(s.unit("p1", 3)?.defId).toBe(RECYCLER);
    s.endTurn();
    // R661: the two discards land at once, at random — no prompt opens.
    expect(s.state.pending).toBeNull();
    expect(s.state.active).toBe("p2");

    const events = s.lastEvents;
    const p2Starts = events.findIndex((event) => event.type === "turnStarted");
    const discarded = discardedBy(events, "p1").map((event) => (event as { defId: string }).defId);
    expect(discarded).toHaveLength(2);
    expect([MENACE, VANILLA, FILLER]).toEqual(expect.arrayContaining(discarded));
    expect(drawnBy(events.slice(0, p2Starts), "p1")).toHaveLength(2);
    expect(s.hand("p1")).toHaveLength(3);
  });
});
