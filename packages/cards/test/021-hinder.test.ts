// #21 Hinder — SPEC §8.2, BUILD M4-T4 row 21: "Auto-casts on draw and draws again; opponent's next
// refresh −1 floored at 0; counts as played (R40, R70); radiant −2", and patch v0.2.0's discard
// (R431): the base face's caster discards 1 at random (R662), with no prompt; nothing with an empty
// hand; the Radiant face is unchanged and discards nothing. R635 keeps it out of the opening deal
// and the mulligan, so the "real game" below proves the deal, not a question.
//
// The harness default board is turn 9 with p1 active, so both sides sit at MAX_MANA (4/4) and the
// refresh Hinder lowers is a concrete number: 4 − 1 = 3 base, 4 − 2 = 2 radiant. The floor needs a
// refresh smaller than 2, which only the opening turns have, so that fixture starts at turn 1 and
// uses `startTurn()` to take p1's draw before p2 has ever refreshed: p2's first refresh is 1, and
// 1 − 2 floors at 0 rather than going negative (§2.3 `refreshMana`). Hinder lowers the refresh, not
// max mana: §2.3's max is min(turns, 4) plus persistent modifiers, and the one-shot rider is not one.
//
// Both sides keep a unit on the board and a card in hand throughout, or the engine's "nothing
// meaningful left" rule would auto-end turns the fixture means to take (harness header).

import { describe, expect, it } from "vitest";
import type { Action, ActionInput, PlayerId } from "@jackioh/shared";
import { beginGame, createGame, fold, hashState, reduce, type GameState } from "@jackioh/engine";
import { scenario, type Scenario } from "./_harness";
import { base, radiant } from "../src/scripts/021-hinder";

const HINDER = "core-021";
/** Tempo Timmy: drawn into a hand it does nothing, and on the board it keeps the turn alive. */
const CONTROL = "core-011";
const P1_FILLER = "core-016"; // Hit Job, a spell with no hand trigger.
const P1_SECOND = "core-044"; // True Strike, the other card a choice would weigh.
const P2_FILLER = "core-005"; // Stockpile, likewise.

function hinderOnTop(seed: string, isRadiant: boolean, turn?: number, hand: readonly string[] = [P1_FILLER]): Scenario {
  return scenario({
    seed,
    ...(turn === undefined ? {} : { turn }),
    p1: {
      hand,
      field: [CONTROL],
      library: [{ def: HINDER, radiant: isRadiant }, CONTROL, CONTROL, CONTROL],
    },
    p2: { hand: [P2_FILLER], field: [CONTROL], library: [P2_FILLER, P2_FILLER, P2_FILLER] },
  });
}

/** Take p1's turn draw (two `endTurn`s from p1's main phase), which casts the Hinder on top. */
function drawHinder(s: Scenario): Scenario {
  s.endTurn(); // p2's turn.
  s.endTurn(); // p1's turn: the draw that casts Hinder.
  return s;
}

describe("#21 Hinder", () => {
  describe("base", () => {
    it("R431, R662 the cast asks nothing: no prompt opens and the draw goes on", () => {
      const s = drawHinder(hinderOnTop("hinder-asks", false));

      // No question stops the draw: the random discard landed and the draw repeated.
      expect(s.state.pending).toBeNull();
      expect(s.view("p2").pending).toBeNull();
      // The filler went to the graveyard as a discard, and the next card is in the hand.
      s.expectInZone(P1_FILLER, "graveyard");
      expect(s.hand("p1").map((card) => card.defId)).toEqual([CONTROL]);
      s.expectInZone(HINDER, "graveyard");
    });

    it("R58, R70 it discards a random card, draws again, and never reaches the hand", () => {
      const s = drawHinder(hinderOnTop("hinder-cast", false));

      // R662: the random card went to the graveyard as a discard.
      s.expectInZone(P1_FILLER, "graveyard");
      s.expectEvents("drawn", "cardPlayed", "discarded", "drawn");
      // R58: the cast-on-draw card is cast, the draw repeats and the next card goes to the hand.
      expect(s.hand("p1").map((card) => card.defId)).toEqual([CONTROL]);
      // §5.1: a Spell that has resolved is in the graveyard.
      s.expectInZone(HINDER, "graveyard");
    });

    it("R662, R431 the discard is random: of two cards, one goes and one stays", () => {
      const s = drawHinder(hinderOnTop("hinder-choice", false, undefined, [P1_FILLER, P1_SECOND]));

      expect(s.state.pending).toBeNull();
      // The draw repeated past the cast, so the hand holds the survivor and the next card.
      expect(s.hand("p1")).toHaveLength(2);
      expect(s.hand("p1").map((card) => card.defId)).toContain(CONTROL);
      // The graveyard holds the resolved Hinder and the one random discard.
      const grave = s.pile("p1", "graveyard").map((card) => card.defId);
      expect(grave).toContain(HINDER);
      const discarded = grave.filter((defId) => defId !== HINDER);
      expect(discarded).toHaveLength(1);
      expect([P1_FILLER, P1_SECOND]).toContain(discarded[0]);
      const survivor = s.hand("p1").find((card) => card.defId !== CONTROL)?.defId;
      expect([P1_FILLER, P1_SECOND]).toContain(survivor);
      expect(survivor).not.toBe(discarded[0]);
    });

    it("R662 the random discard comes from the match rng: the same game discards the same card", () => {
      const first = drawHinder(hinderOnTop("hinder-roundtrip", false, undefined, [P1_FILLER, P1_SECOND]));
      const second = drawHinder(hinderOnTop("hinder-roundtrip", false, undefined, [P1_FILLER, P1_SECOND]));
      const ids = (s: Scenario): string[] =>
        s.events.flatMap((event) => (event.type === "discarded" ? [event.instanceId] : []));
      expect(ids(first)).toEqual(ids(second));
    });

    it("R431, R90 with an empty hand there is nothing to discard: no prompt, and the rest still lands", () => {
      const s = scenario({
        seed: "hinder-empty",
        p1: { field: [CONTROL], library: [HINDER, CONTROL, CONTROL, CONTROL] },
        p2: { hand: [P2_FILLER], field: [CONTROL], library: [P2_FILLER, P2_FILLER, P2_FILLER] },
      });
      s.startTurn(); // p1's draw, with nothing in hand.

      expect(s.state.pending).toBeNull();
      expect(s.events.some((event) => event.type === "discarded")).toBe(false);
      expect(s.state.players.p2.mana.nextTurnMod).toBe(-1);
      // The draw repeated: the next card is in the empty hand.
      expect(s.hand("p1").map((card) => card.defId)).toEqual([CONTROL]);
      s.expectInZone(HINDER, "graveyard");
    });

    it("R40, R70 counts as a card played this turn, at cost 0", () => {
      const s = drawHinder(hinderOnTop("hinder-played", false));

      const played = s.events.filter((event) => event.type === "cardPlayed" && event.defId === HINDER);
      expect(played).toHaveLength(1);
      expect(played[0]).toMatchObject({ player: "p1", costPaid: 0 });
      // R40: the cast is in the turn log every Combo card counts.
      expect(s.state.players.p1.turnLog.playedIds).toContain(s.card(HINDER).id);
    });

    it("the opponent has 1 less mana next turn", () => {
      const s = drawHinder(hinderOnTop("hinder-refresh", false));
      s.endTurn(); // p2's turn: the lowered refresh.

      expect(s.state.active).toBe("p2");
      s.expectMana("p2", 3);
      expect(s.view("p2").you.mana).toEqual({ current: 3, max: 4 });
    });

    it("the modifier is one-shot: the refresh after that is back to 4 (§2.3)", () => {
      const s = drawHinder(hinderOnTop("hinder-oneshot", false));
      s.endTurn(); // p2's lowered refresh.
      s.expectMana("p2", 3);
      s.endTurn(); // p1.
      s.endTurn(); // p2 again, with nothing owed.

      s.expectMana("p2", 4);
      expect(s.state.players.p2.mana.nextTurnMod).toBe(0);
    });
  });

  describe("radiant", () => {
    it("the opponent has 2 less mana next turn", () => {
      const s = drawHinder(hinderOnTop("hinder-radiant", true));
      s.endTurn();

      s.expectMana("p2", 2);
      expect(s.view("p2").you.mana).toEqual({ current: 2, max: 4 });
    });

    it("R431 discards nothing and asks nothing: casts itself on draw and draws again", () => {
      const s = drawHinder(hinderOnTop("hinder-radiant-cast", true));

      expect(s.state.pending).toBeNull();
      expect(s.events.some((event) => event.type === "discarded")).toBe(false);
      s.expectInZone(P1_FILLER, "hand");
      expect(s.hand("p1").map((card) => card.defId)).not.toContain(HINDER);
      s.expectInZone(HINDER, "graveyard");
      s.expectEvents("drawn", "cardPlayed", "drawn");
    });

    it("§2.3 the refresh floors at 0 rather than going negative", () => {
      // Turn 1: p2 has started no turn yet, so their first refresh is 1 and −2 would be −1.
      const s = hinderOnTop("hinder-floor", true, 1);
      s.startTurn(); // p1's draw, which casts Hinder; the turn does not change hands.
      expect(s.state.players.p2.mana.nextTurnMod).toBe(-2);

      s.endTurn(); // p2's first turn: the refresh.
      expect(s.state.active).toBe("p2");
      s.expectMana("p2", 0);
      expect(s.view("p2").you.mana).toEqual({ current: 0, max: 1 });
    });
  });

  it("R431 both faces are Cast on draw; neither declares a discard pick (R662: random)", () => {
    expect(base.staticFlags?.castOnDraw).toBe(true);
    expect(radiant.staticFlags?.castOnDraw).toBe(true);
    expect(base.targets).toBeUndefined();
    // The repeat draw and the 0 floor belong to `drawOne` and `refreshMana`, not to this card.
    expect(base.modes).toBeUndefined();
    expect(radiant.targets).toBeUndefined();
  });
});

// ---------------------------------------------------------------------------
// A real game: setup deals no Hinder (R635), so both mulligans open at once over hands that
// hold none; turn 1's draw is what meets it, discarding at random with no prompt (R662), and
// the log folds back to the same game (§9.3).
// ---------------------------------------------------------------------------

/** Twenty legal Core cards with no other cast-on-draw card among them, Hinder first. */
const DECK = [
  HINDER,
  ...["core-002", "core-005", "core-006", "core-008", "core-011", "core-012", "core-013", "core-015", "core-016"],
  ...["core-019", "core-020", "core-025", "core-026", "core-032", "core-036", "core-043", "core-044", "core-053", "core-055"],
];

let nonce = 0;
function act(state: GameState, body: ActionInput, log: Action[]): GameState {
  nonce += 1;
  const action = { ...body, nonce: `hinder-game-${nonce}` } as Action;
  const result = reduce(state, action);
  if (result.error !== undefined) throw new Error(result.error);
  log.push(action);
  return result.state;
}

describe("#21 Hinder in a real game (R635, R662, §9.3)", () => {
  it("R431, R635, R662 setup neither deals nor casts a Hinder; turn 1's draw casts it with a random discard and no prompt, and the log replays to the same state", () => {
    // A seed whose shuffle-in puts the Hinder on top of p1's library, so turn 1's draw casts it.
    let found: { seed: string; state: GameState; log: Action[] } | null = null;
    for (let at = 0; at < 300 && found === null; at += 1) {
      const seed = `hinder-deal-${at}`;
      const log: Action[] = [];
      let state = beginGame(createGame({ seed, decks: [DECK, DECK] })).state;

      // §2.1, R635: setup casts nothing and asks nothing, so the mulligans open at once, over a hand
      // that holds no Hinder, with the Hinder waiting in the library.
      expect(state.pending).toBeNull();
      expect(state.mulligan).toBeDefined();
      expect(state.players.p1.hand.map((card) => card.defId)).not.toContain(HINDER);
      expect(state.players.p1.graveyard).toEqual([]);
      expect(state.players.p1.library.map((card) => card.defId)).toContain(HINDER);

      for (const player of ["p1", "p2"] as PlayerId[]) {
        state = act(state, { type: "mulligan", keep: state.players[player].hand.map((card) => card.id), playerId: player }, log);
      }
      expect(state.turn).toBe(1);
      // R662: the cast discards at random and asks nothing, so a turn-1 cast leaves the Hinder and
      // one discard in the graveyard with no prompt open.
      if (state.players.p1.graveyard.map((card) => card.defId).includes(HINDER)) found = { seed, state, log };
    }
    expect(found).not.toBeNull();
    if (found === null) return;
    const state = found.state;

    expect(state.pending).toBeNull();
    const grave = state.players.p1.graveyard.map((card) => card.defId);
    expect(grave.filter((defId) => defId === HINDER)).toHaveLength(1);
    expect(grave).toHaveLength(2);
    // The draw repeated past the cast: the hand is full again.
    expect(state.players.p1.hand).toHaveLength(3);
    expect(state.turn).toBe(1);

    const replayed = fold({ seed: found.seed, decks: [DECK, DECK], log: found.log });
    expect(replayed.errors).toEqual([]);
    expect(hashState(replayed.state)).toBe(hashState(state));
  });
});
