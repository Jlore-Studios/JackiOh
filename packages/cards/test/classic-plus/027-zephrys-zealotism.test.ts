// C+ #27 Zephrys Zealotism — SPEC §8.7 row 27, B5 E34, R29, R364, R387, R416. BUILD M9 Classic+ row
// C+ 27: "Each other card in your hand goes to your graveyard (not a discard) and as many cards arrive:
// the scorer's top distinct picks (R29's scorer) among the non-token Classic and Classic+ cards but this
// one (R387), ranked for the current state, never a Core card on either face (R416); a card that enables
// lethal ranks first when lethal exists; the same state always gives the same hand; with only this card
// in hand nothing arrives; then a Refresh (R364) gives back up to max mana, its own 4 included, never
// past max; the replaced cards are public in the graveyard and the new ones hidden from the opponent
// (R97); the state survives JSON and replays to the same hash; radiant the new cards are Radiant".
//
// The ranking itself is pinned against a fixed pool in the engine (`packages/engine/test/perfectHand.test.ts`).
// Here it runs over the real catalog, so each case compares the hand that arrives with the subsystem's
// own ranking of the state the card resolves in: the same board and hand without this card, at the mana
// left once its (4) is paid (`resolvingState`).

import type { Action } from "@jackioh/shared";
import { hashState, reduce, subsystems, type GameState } from "@jackioh/engine";
import { describe, expect, it } from "vitest";
import { scenario, type Scenario, type SideSetup } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic-plus/027-zephrys-zealotism";

const ZEALOTISM = "classicplus-027";
/** Plain hand cards to be replaced: #5 Stockpile, #19 Midrange Menace, #11 Tempo Timmy. */
const STOCKPILE = "core-005";
const MENACE = "core-019";
const TIMMY = "core-011";
/** A unit-token card, which ceases to exist rather than reach a graveyard (R11). */
const RUSH_TOKEN = "core-t-rush";
const COST = 4;
const HIDDEN = "hidden";

type Setup = { p1?: SideSetup; p2?: SideSetup; seed?: string };

function play(setup: Setup, options: { radiant?: boolean } = {}): Scenario {
  const s = scenario({
    ...(setup.seed === undefined ? {} : { seed: setup.seed }),
    p1: { ...setup.p1, hand: [{ def: ZEALOTISM, radiant: options.radiant === true }, ...(setup.p1?.hand ?? [])] },
    ...(setup.p2 === undefined ? {} : { p2: setup.p2 }),
  });
  s.play(ZEALOTISM);
  return s;
}

/** The state Zealotism resolves in: the same game with this card gone from hand and its (4) paid. */
function resolvingState(setup: Setup): GameState {
  const mana = (setup.p1?.mana ?? COST) - COST;
  return scenario({ ...setup, p1: { ...setup.p1, mana } }).state;
}

function ranked(setup: Setup, options: { radiant?: boolean } = {}) {
  return subsystems.rankPerfectHand(resolvingState(setup), "p1", { selfDefId: ZEALOTISM, ...options });
}

const handDefs = (s: Scenario): string[] => s.hand("p1").map((card) => card.defId);

describe("C+ #27 Zephrys Zealotism", () => {
  it("is the card it says, and the faces differ only in the face they rank and hand over", () => {
    expect(def.id).toBe(ZEALOTISM);
    expect(base).not.toBe(radiant);
  });

  describe("base", () => {
    it("R416 each other card goes to your graveyard, not a discard, and as many cards arrive", () => {
      const setup = { p1: { hand: [STOCKPILE, MENACE, TIMMY] } };
      const s = play(setup);
      expect(s.hand("p1")).toHaveLength(3);
      for (const old of [STOCKPILE, MENACE, TIMMY]) s.expectInZone(old, "graveyard");
      expect(s.events.some((event) => event.type === "discarded")).toBe(false);
      expect(s.events.filter((event) => event.type === "enteredGraveyard").map((event) => event.defId)).toEqual(
        expect.arrayContaining([STOCKPILE, MENACE, TIMMY]),
      );
      s.expectInZone(ZEALOTISM, "graveyard");
    });

    it("R416 the new hand is the scorer's top distinct picks in rank order, for the state as it resolves", () => {
      const setup = { p1: { hand: [STOCKPILE, MENACE, TIMMY] } };
      const s = play(setup);
      expect(handDefs(s)).toEqual(ranked(setup).slice(0, 3).map((scored) => scored.def.id));
      expect(new Set(handDefs(s)).size).toBe(3);
    });

    it("R387 R416 never a Core card, never a token, never Zephrys Zealotism itself", () => {
      const s = play({ p1: { hand: [STOCKPILE, MENACE, TIMMY, STOCKPILE, MENACE, TIMMY, STOCKPILE, MENACE, TIMMY] } });
      expect(s.hand("p1")).toHaveLength(9);
      for (const card of s.hand("p1")) {
        const entry = subsystems.rankPerfectHand(s.state, "p1", { selfDefId: ZEALOTISM }).find((scored) => scored.def.id === card.defId);
        expect(entry?.def.set === "Classic" || entry?.def.set === "Classic+").toBe(true);
        expect(entry?.def.token).toBe(false);
        expect(card.defId).not.toBe(ZEALOTISM);
      }
      const pool = ranked({}).map((scored) => scored.def);
      expect(pool.every((entry) => (entry.set === "Classic" || entry.set === "Classic+") && !entry.token)).toBe(true);
      expect(pool.map((entry) => entry.id)).not.toContain(ZEALOTISM);
    });

    it("R29 a card that enables lethal ranks first when lethal exists", () => {
      // p2 at 4: a 4-attack Charge body, played with the 4 mana left after this card's (4), is lethal.
      const setup = { p1: { hand: [STOCKPILE, MENACE], mana: 8 }, p2: { health: 4 } };
      const top = ranked(setup)[0];
      expect(top?.priority).toBe("lethal");
      const s = play(setup);
      expect(handDefs(s)[0]).toBe(top?.def.id);
      // With no lethal on the board, nothing ranks for it.
      expect(ranked({ p1: { hand: [STOCKPILE, MENACE], mana: 8 } }).every((scored) => scored.priority !== "lethal")).toBe(true);
    });

    it("§10.7 the same state always gives the same hand, whatever the seed, and draws nothing", () => {
      const setup = { p1: { hand: [STOCKPILE, MENACE, TIMMY], field: [MENACE] }, p2: { field: [TIMMY] } };
      const a = play({ ...setup, seed: "zealotism-a" });
      const b = play({ ...setup, seed: "zealotism-b" });
      expect(handDefs(a)).toEqual(handDefs(b));
      const before = scenario({ ...setup, p1: { ...setup.p1, hand: [ZEALOTISM, ...setup.p1.hand] } });
      const cursor = before.state.rngCursor;
      before.play(ZEALOTISM);
      expect(before.state.rngCursor).toBe(cursor);
    });

    it("R416 with only this card in hand nothing arrives, and the Refresh still happens", () => {
      const s = play({ p1: { hand: [] } });
      expect(s.hand("p1")).toHaveLength(0);
      s.expectMana("p1", 4);
    });

    it("R11 a unit-token card in the replaced hand ceases to exist", () => {
      const s = play({ p1: { hand: [RUSH_TOKEN, STOCKPILE] } });
      expect(s.hand("p1")).toHaveLength(2);
      expect(s.pile("p1", "graveyard").map((card) => card.defId)).not.toContain(RUSH_TOKEN);
      expect(s.pile("p1", "graveyard").map((card) => card.defId)).toContain(STOCKPILE);
    });

    it("R364 the Refresh gives back up to max mana, this card's 4 included", () => {
      play({ p1: { hand: [STOCKPILE] } }).expectMana("p1", 4);
      // 6 current of 4 max: 2 left after paying, the Refresh gives back 2 more, up to max.
      play({ p1: { hand: [STOCKPILE], mana: 6 } }).expectMana("p1", 4);
    });

    it("R364 and never past max: a player still above max after paying gains nothing", () => {
      const s = play({ p1: { hand: [STOCKPILE], mana: 9 } });
      s.expectMana("p1", 5);
      expect(s.events.filter((event) => event.type === "manaChanged")).toHaveLength(1);
    });

    it("R97 the replaced cards are public in the graveyard; the new cards are hidden from the opponent", () => {
      const s = play({ p1: { hand: [STOCKPILE, MENACE] } });
      const theirs = s.view("p2");
      const added = theirs.events.filter((event) => event.type === "addedToHand");
      expect(added).toHaveLength(2);
      for (const event of added) {
        if (event.type !== "addedToHand") continue;
        expect(event.defId).toBe(HIDDEN);
        expect(event.instanceId).toBe(HIDDEN);
      }
      expect(theirs.players.p1.graveyard.map((card) => card.defId)).toEqual(expect.arrayContaining([STOCKPILE, MENACE]));
      expect(JSON.stringify(theirs)).not.toContain(s.hand("p1")[0]?.id ?? "no card");
    });

    it("§9.3 the state survives JSON and the play replays to the same hash", () => {
      const s = scenario({ p1: { hand: [ZEALOTISM, STOCKPILE, MENACE] }, p2: { field: [TIMMY] } });
      const thawed = JSON.parse(JSON.stringify(s.state)) as GameState;
      const action = {
        type: "play",
        instanceId: s.card(ZEALOTISM).id,
        playerId: "p1",
        nonce: "zealotism-replay",
      } as Action;
      const live = reduce(s.state, action);
      const again = reduce(thawed, action);
      expect(live.error).toBeUndefined();
      expect(hashState(again.state)).toBe(hashState(live.state));
      expect(again.events).toEqual(live.events);
    });
  });

  describe("radiant", () => {
    it("R416 the perfect Radiant hand: ranked on the Radiant faces, and the new cards arrive Radiant", () => {
      const setup = { p1: { hand: [STOCKPILE, MENACE, TIMMY] } };
      const s = play(setup, { radiant: true });
      expect(handDefs(s)).toEqual(ranked(setup, { radiant: true }).slice(0, 3).map((scored) => scored.def.id));
      expect(s.hand("p1").every((card) => card.radiant)).toBe(true);
      s.expectMana("p1", 4);
    });

    it("R416 the Radiant face draws on Classic and Classic+ only too", () => {
      const s = play({ p1: { hand: [STOCKPILE, MENACE] } }, { radiant: true });
      for (const card of s.hand("p1")) expect(card.defId.startsWith("classic")).toBe(true);
    });
  });
});
