// C #23 Devil's Pact — SPEC §8.6 row 23, BUILD M9 Classic row C 23: "Cry: discard 666 cards, which is
// your whole hand; Activate, once per turn (R384): for the rest of this turn each card you play is
// replaced at §10.5 step 3 by a new Book of Flame (C #16, base face), which resolves as that play,
// counts as a Book of Flame play and asks its target then; the old card ceases to exist (R35) and the
// price paid was the old card's; a Unit or a trap replaced this way takes no zone; a cast (R70) and
// a play from the graveyard (R454) are replaced too; a card played before activating is not; the
// modifier expires at cleanup; activating is not a play; the opponent's view never names a replaced
// card (it ceased to exist unread, R177);
// radiant: discard 6 cards at random (R682; all if fewer), and each replacement is a Radiant Book
// of Flame; its tuned number (discards) reads through `param()` (R386)".

import { describe, expect, it } from "vitest";
import { findInstance, heroOf, legalActions, reduce, stepParam, zoneCards, type GameState } from "@jackioh/engine";
import type { Action, GameEvent, Selection } from "@jackioh/shared";
import { scenario, type Scenario, type SideSetup } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic/023-devils-pact";

const PACT = "classic-023";
const BOOK = "classic-016"; // Book of Flame: Deal 4 damage (Radiant 8).
const VANILLA = "core-008"; // (1) Unit 4/4.
const TIMMY = "core-011"; // (1) Unit 3/3.
const STOCKPILE = "core-005"; // (1) Spell: Draw 2. Heal 2.
const BEAR = "core-060"; // (1) Trap.
const PANTHER = "core-032"; // Unit 5/4 Rush: after it attacks and survives, draw 2 per Unit it destroyed.
const HINDER = "core-021"; // (0) Spell, cast on draw.
const FILLER = "core-010"; // (0) Spell Rapid Replenish.
const MENACE = "core-019"; // (3) Unit 9/9.
const WIND = "classic-028"; // Second Wind: Radiant Aura lets you play cards costing (1) or more from your graveyard.

const AT_P2: readonly Selection[] = [{ pick: "hero", player: "p2" }];

function onField(p1: SideSetup = {}, radiantFace = false, p2: SideSetup = {}): Scenario {
  return scenario({
    p1: { hand: [VANILLA, STOCKPILE, FILLER], backrow: [{ def: PACT, radiant: radiantFace, lane: 1 }], library: [TIMMY, TIMMY, TIMMY], ...p1 },
    p2: { hand: [FILLER], library: [FILLER, FILLER], ...p2 },
  });
}

function count(events: readonly GameEvent[], type: GameEvent["type"]): number {
  return events.filter((event) => event.type === type).length;
}

describe("C #23 Devil's Pact", () => {
  it("declares a Cry and one Activate, once per turn, on each face", () => {
    expect(def.id).toBe(PACT);
    expect(def.refs).toContain(BOOK);
    for (const face of [base, radiant]) {
      expect(face.cry).toBeTypeOf("function");
      expect(face.activations?.[0]?.uses).toBe(1);
    }
  });

  describe("base", () => {
    it("Cry: discard 666 cards — the whole hand, with no prompt", () => {
      const s = scenario({
        p1: { hand: [PACT, VANILLA, TIMMY, STOCKPILE], library: [TIMMY] },
        p2: { hand: [FILLER] },
      });

      s.play(PACT, { zone: 1 });

      expect(s.hand("p1")).toHaveLength(0);
      expect(s.state.pending).toBeNull();
      expect(count(s.lastEvents, "discarded")).toBe(3);
    });

    it("Cry with an empty hand discards nothing", () => {
      const s = scenario({ p1: { hand: [PACT], field: [VANILLA] }, p2: { hand: [FILLER] } });

      s.play(PACT, { zone: 1 });

      expect(count(s.lastEvents, "discarded")).toBe(0);
      s.expectInZone(PACT, "field");
    });

    it("R449 after the activation a played card is replaced by a Book of Flame, which asks its target then", () => {
      const s = onField();
      const vanilla = s.card(VANILLA);

      s.activate(PACT);
      s.play(vanilla, { zone: 2 });

      expect(s.state.pending?.playerId).toBe("p1");
      s.answer(AT_P2);

      s.expectInZone(vanilla, "gone");
      expect(s.unit("p1", 2)).toBeNull();
      s.expectHealth("p2", 26);
      s.expectEvents("activated", "transformed", "cardAnnounced", "cardPlayed", "damage");
    });

    it("§9.3 the replaced play, paused on the Book of Flame's target, survives a JSON round trip", () => {
      const s = onField();
      s.activate(PACT);
      s.play(VANILLA, { zone: 2 });
      const paused = s.state;
      const revived = JSON.parse(JSON.stringify(paused)) as GameState;
      expect(revived).toEqual(paused);

      const result = reduce(revived, {
        type: "answer",
        playerId: "p1",
        choiceId: revived.pending?.id ?? "",
        selection: [...AT_P2],
        nonce: "pact-book-round-trip",
      });

      expect(result.error).toBeUndefined();
      expect(result.state.pending).toBeNull();
      expect(result.state.work).toEqual([]);
      expect(heroOf(result.state, "p2").health).toBe(26);
    });

    it("R449 the price paid is the old card's, not the Book's: a (3) Cost Unit replaced costs 3", () => {
      const s = onField({ hand: [MENACE, FILLER] });

      s.activate(PACT);
      s.play(MENACE, { zone: 2 }).answer(AT_P2);

      const played = s.events.find((event) => event.type === "cardPlayed");
      expect(played).toMatchObject({ defId: BOOK, costPaid: 3 });
      s.expectMana("p1", 1);
      s.expectHealth("p2", 26);
    });

    it("R449 the replacement is played as a Book of Flame, counted as one play; the price was the old card's", () => {
      const s = onField();

      s.activate(PACT);
      s.play(VANILLA, { zone: 2 }).answer(AT_P2);

      const played = s.events.filter((event) => event.type === "cardPlayed");
      expect(played).toHaveLength(1);
      expect(played[0]).toMatchObject({ defId: BOOK, costPaid: 1 });
      expect(s.state.players.p1.turnLog.cardsPlayed).toBe(1);
      s.expectMana("p1", 3);
      expect(s.pile("p1", "graveyard").map((card) => card.defId)).toContain(BOOK);
    });

    it("R449 a trap replaced this way takes no zone", () => {
      const s = onField({ hand: [BEAR, FILLER] });

      s.activate(PACT);
      s.play(BEAR, { zone: 3 }).answer(AT_P2);

      expect(s.backrow("p1", 3)).toBeNull();
      s.expectHealth("p2", 26);
    });

    it("R70 a cast is replaced too: a card cast on draw becomes a Book of Flame", () => {
      const s = onField(
        { hand: [FILLER], field: [PANTHER], library: [{ def: HINDER, radiant: true }, TIMMY, TIMMY] },
        false,
        { field: [TIMMY] },
      );
      const hinder = s.card(HINDER);
      const prey = s.unit("p2", 1);
      if (prey === null) throw new Error("fixture");

      s.activate(PACT);
      // Prem Panther kills the Timmy and survives, so it draws 2: the first draw finds Hinder, cast.
      s.attack(PANTHER, prey);
      expect(s.state.pending?.playerId).toBe("p1");
      s.answer(AT_P2);

      s.expectInZone(hinder, "gone");
      s.expectHealth("p2", 26);
      expect(s.state.players.p2.mana.nextTurnMod).toBe(0);
    });

    it("R449 R454 a play from the graveyard is replaced too: it plays as from hand, so its card becomes a Book of Flame", () => {
      const s = onField({ hand: [FILLER], backrow: [{ def: PACT, lane: 1 }, { def: WIND, radiant: true, lane: 2 }], graveyard: [VANILLA] });
      const vanilla = s.card(VANILLA);
      s.activate(PACT);
      const play = legalActions(s.state, "p1").find((action) => action.type === "play" && action.instanceId === vanilla.id);
      if (play === undefined) throw new Error("Second Wind offers no graveyard play");

      const played = reduce(s.state, { ...play, playerId: "p1", nonce: "pact-graveyard-play" } as Action);
      expect(played.error).toBeUndefined();
      expect(played.events.map((event) => event.type)).toContain("transformed");
      expect(played.state.pending?.playerId).toBe("p1");
      const answered = reduce(played.state, {
        type: "answer",
        playerId: "p1",
        choiceId: played.state.pending?.id ?? "",
        selection: [...AT_P2],
        nonce: "pact-graveyard-answer",
      });

      expect(answered.error).toBeUndefined();
      // R35: the graveyard card ceased to exist.
      expect(findInstance(answered.state, vanilla.id)).toBeUndefined();
      expect([...played.events, ...answered.events].map((event) => event.type)).not.toContain("summoned");
      expect(heroOf(answered.state, "p2").health).toBe(26);
      expect(answered.events.find((event) => event.type === "cardPlayed")).toMatchObject({ defId: BOOK, costPaid: 1 });
      expect(zoneCards(answered.state, "p1", "graveyard").map((card) => card.defId)).toEqual([BOOK]);
    });

    it("R449 R454 two live Pacts replace a graveyard play in turn: the Radiant one's Book of Flame resolves", () => {
      const s = onField({
        hand: [FILLER],
        backrow: [{ def: PACT, lane: 1 }, { def: PACT, radiant: true, lane: 2 }, { def: WIND, radiant: true, lane: 3 }],
        graveyard: [VANILLA],
      });
      const [first, second] = s.state.players.p1.backrow.filter((card) => card?.defId === PACT);
      if (first === undefined || first === null || second === undefined || second === null) throw new Error("fixture");
      s.activate(first).activate(second);
      const vanilla = s.card(VANILLA);
      const play = legalActions(s.state, "p1").find((action) => action.type === "play" && action.instanceId === vanilla.id);
      if (play === undefined) throw new Error("Second Wind offers no graveyard play");

      const played = reduce(s.state, { ...play, playerId: "p1", nonce: "pacts-graveyard-play" } as Action);
      expect(played.events.filter((event) => event.type === "transformed")).toHaveLength(2);
      const answered = reduce(played.state, {
        type: "answer",
        playerId: "p1",
        choiceId: played.state.pending?.id ?? "",
        selection: [...AT_P2],
        nonce: "pacts-graveyard-answer",
      });

      expect(answered.error).toBeUndefined();
      expect(heroOf(answered.state, "p2").health).toBe(22);
      // The Book came from no graveyard: its play does not say it did.
      expect(answered.events.find((event) => event.type === "cardPlayed")).not.toHaveProperty("from");
    });

    it("a card played before the activation is not replaced", () => {
      const s = onField();

      s.play(VANILLA, { zone: 2 });
      s.activate(PACT);

      s.expectInZone(VANILLA, "field");
      expect(s.unit("p1", 2)?.defId).toBe(VANILLA);
    });

    it("the modifier expires at cleanup: next turn a play is itself again", () => {
      const s = onField();

      s.activate(PACT);
      s.endTurn().endTurn();
      expect(s.state.active).toBe("p1");
      s.play(VANILLA, { zone: 2 });

      expect(s.unit("p1", 2)?.defId).toBe(VANILLA);
      expect(s.state.pending).toBeNull();
    });

    it("R384 activating is not a play, and is once per turn", () => {
      const s = onField();
      const played = s.state.players.p1.turnLog.cardsPlayed;

      s.activate(PACT);

      expect(s.state.players.p1.turnLog.cardsPlayed).toBe(played);
      expect(count(s.events, "cardPlayed")).toBe(0);
      expect(() => s.activate(PACT)).toThrow();
    });

    it("R177 the opponent's view never names a card replaced out of your hand", () => {
      const s = onField();

      s.activate(PACT);
      s.play(VANILLA, { zone: 2 }).answer(AT_P2);

      expect(JSON.stringify(s.view("p2"))).not.toContain(VANILLA);
    });

    it("R386 a Degrade of its discards stays the whole hand; an Upgrade (499) is still more than any hand", () => {
      const s = scenario({ p1: { hand: [PACT, VANILLA, TIMMY], library: [TIMMY] }, p2: { hand: [FILLER] } });
      stepParam(s.card(PACT), "discards", -1);

      s.play(PACT, { zone: 1 });

      expect(s.hand("p1")).toHaveLength(0);
    });
  });

  describe("radiant", () => {
    it("R682 Cry: discard 6 cards at random, with no prompt", () => {
      const eight = [VANILLA, VANILLA, TIMMY, TIMMY, STOCKPILE, STOCKPILE, FILLER, BEAR];
      const s = scenario({ p1: { hand: [{ def: PACT, radiant: true }, ...eight] }, p2: { hand: [FILLER] } });

      s.play(PACT, { zone: 1 });

      expect(s.state.pending).toBeNull();
      expect(s.hand("p1")).toHaveLength(2);
      expect(count(s.lastEvents, "discarded")).toBe(6);
    });

    it("R682 the random discards come from the match rng: the same game discards the same cards", () => {
      const eight = [VANILLA, VANILLA, TIMMY, TIMMY, STOCKPILE, STOCKPILE, FILLER, BEAR];
      const mk = (): Scenario =>
        scenario({ p1: { hand: [{ def: PACT, radiant: true }, ...eight] }, p2: { hand: [FILLER] } });
      const first = mk();
      first.play(PACT, { zone: 1 });
      const second = mk();
      second.play(PACT, { zone: 1 });
      const ids = (s: Scenario): string[] =>
        s.lastEvents.flatMap((event) => (event.type === "discarded" ? [event.instanceId] : []));
      expect(ids(first)).toEqual(ids(second));
    });

    it("R682 with 6 or fewer in hand it discards all of them, asking nothing", () => {
      const s = scenario({ p1: { hand: [{ def: PACT, radiant: true }, VANILLA, TIMMY] }, p2: { hand: [FILLER] } });

      s.play(PACT, { zone: 1 });

      expect(s.state.pending).toBeNull();
      expect(s.hand("p1")).toHaveLength(0);
    });

    it("each replacement is a Radiant Book of Flame: 8 damage", () => {
      const s = onField({}, true);

      s.activate(PACT);
      s.play(VANILLA, { zone: 2 }).answer(AT_P2);

      s.expectHealth("p2", 22);
      const played = s.events.find((event) => event.type === "cardPlayed");
      expect(played).toMatchObject({ defId: BOOK });
    });

    it("R386 an Upgrade of its discards takes one step of 167, the base face's, so the Radiant 6 goes to its floor of 1", () => {
      const eight = [VANILLA, VANILLA, TIMMY, TIMMY, STOCKPILE, STOCKPILE, FILLER, BEAR];
      const s = scenario({ p1: { hand: [{ def: PACT, radiant: true }, ...eight] }, p2: { hand: [FILLER] } });
      stepParam(s.card(PACT), "discards", -1);

      s.play(PACT, { zone: 1 });

      expect(s.state.pending).toBeNull();
      expect(count(s.lastEvents, "discarded")).toBe(1);
      expect(s.hand("p1")).toHaveLength(7);
    });
  });
});
