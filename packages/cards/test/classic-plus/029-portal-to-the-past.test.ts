// C+ #29 Portal to the Past — SPEC §8.7 row 29, R417, R564; BUILD M9 Classic+ row C+ 29. The last
// board is a setup input frozen into the match (B5 E30); `scenario({ lastBoards })` hands it to
// `createGame` as the server does. The engine's own proofs (the reader, the freeze, the fold) are in
// packages/engine/test/lastBoards.test.ts.

import { AI_TUTORIAL, beginGame, createGame, fold, hashState, reduce, stepParam, type GameState, type LastBoardInput } from "@jackioh/engine";
import type { Action, ActionInput, PlayerId } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { scenario, type Scenario } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic-plus/029-portal-to-the-past";

const PORTAL = "classicplus-029";
const FILLER = "core-005"; // keeps a hand from auto-ending the turn (§2.5)
const MENACE = "core-019";
const TOKEN = "core-t-rush"; // a unit token (R11)
const FUSED = "t-1:core-012+core-025"; // R179
type Entry = { defId: string; radiant: boolean };
const BOARD: Entry[] = [
  { defId: "core-012", radiant: false },
  { defId: MENACE, radiant: false },
  { defId: "core-025", radiant: true },
  { defId: TOKEN, radiant: false },
  { defId: "core-043", radiant: false },
];
const BOARD_IDS = BOARD.map((entry) => entry.defId);
const OPPONENT_BOARD: Entry[] = [{ defId: "core-092", radiant: false }];

function game(own: readonly Entry[], opts: { radiant?: boolean; fillers?: number } = {}): Scenario {
  return scenario({
    lastBoards: [own, OPPONENT_BOARD],
    p1: { hand: [{ def: PORTAL, radiant: opts.radiant === true }, ...Array.from({ length: opts.fillers ?? 1 }, () => FILLER)] },
    p2: { hand: [FILLER] },
  });
}

function offered(state: GameState): string[] {
  return (state.pending?.options ?? []).map((option) => (option.selection.pick === "mode" ? option.selection.option : ""));
}

/** The cards Portal made: everything in p1's hand but the fillers. */
function made(s: Scenario) {
  return s.hand("p1").filter((card) => card.defId !== FILLER && card.defId !== PORTAL);
}

describe("C+ #29 Portal to the Past", () => {
  it("is a (3) Spell whose Radiant count is the declared number `cards`", () => {
    expect(def).toMatchObject({ id: PORTAL, type: "Spell", cost: 3 });
    expect(def.params).toEqual([{ key: "cards", base: 3, radiant: 3, better: "up", step: 1, min: 1 }]);
    expect(base.resume).toBeDefined();
    expect(radiant.resume).toBeUndefined();
  });

  describe("base: Discover a card from the board your last game ended with; it costs (0)", () => {
    it("R417 offers 3 different cards of the caster's own last board, never the opponent's", () => {
      const s = game(BOARD).play(PORTAL);
      expect(s.state.pending).toMatchObject({ kind: "discover", playerId: "p1" });
      const options = offered(s.state);
      expect(options).toHaveLength(3);
      expect(new Set(options).size).toBe(3);
      for (const defId of options) expect(BOARD_IDS).toContain(defId);
    });

    it("R417 offers them all when the board holds fewer than 3 different cards", () => {
      const s = game([BOARD[0] as Entry, BOARD[0] as Entry, BOARD[1] as Entry]).play(PORTAL);
      expect(offered(s.state).sort()).toEqual(["core-012", MENACE]);
    });

    it("R417 the pick arrives as a new card the caster owns, on its entry's face, costing (0)", () => {
      const s = game([{ defId: "core-025", radiant: true }]).play(PORTAL);
      expect(s.state.pending?.options[0]?.radiant).toBe(true);
      s.answer("mode:core-025");
      expect(made(s)).toMatchObject([{ defId: "core-025", radiant: true, costOverride: 0, owner: "p1", controller: "p1" }]);
      const hand = s.view("p1").you.hand;
      if (!Array.isArray(hand)) throw new Error("§10.8: the viewer's own hand is a list of cards");
      expect(hand.find((card) => card.defId === "core-025")?.cost).toBe(0);
      expect(s.state.pending).toBeNull();
      s.expectInZone(PORTAL, "graveyard");
    });

    it("R417 only card and face come back: no stats, buffs or damage", () => {
      const s = game([{ defId: MENACE, radiant: false }]).play(PORTAL).answer(`mode:${MENACE}`);
      expect(made(s)).toMatchObject([{ defId: MENACE, radiant: false, damage: 0, buffs: { attack: 0, health: 0 } }]);
    });

    it("R11 a unit token on that board comes back as a card in hand", () => {
      const s = game([{ defId: TOKEN, radiant: false }]).play(PORTAL).answer(`mode:${TOKEN}`);
      expect(made(s)).toMatchObject([{ defId: TOKEN, costOverride: 0, zone: { z: "hand", player: "p1" } }]);
    });

    it("R179 a fused card on that board is offered by its id and rebuilt by the engine", () => {
      const s = game([{ defId: FUSED, radiant: false }]).play(PORTAL);
      expect(offered(s.state)).toEqual([FUSED]);
      const view = s.view("p1").pending;
      expect(view?.forYou === true ? view.options[0] : null).toMatchObject({ defId: FUSED });
      s.answer(`mode:${FUSED}`);
      expect(made(s)).toMatchObject([{ defId: FUSED, costOverride: 0 }]);
      expect(s.state.transientDefs[FUSED]?.ingredients).toEqual([{ defId: "core-012" }, { defId: "core-025" }]);
    });

    it("R564 two copies of a card are one option, on its Radiant face if either copy was Radiant", () => {
      const s = game([
        { defId: MENACE, radiant: false },
        { defId: MENACE, radiant: true },
      ]).play(PORTAL);
      expect(s.state.pending?.options).toHaveLength(1);
      expect(s.state.pending?.options[0]).toMatchObject({ key: `mode:${MENACE}`, radiant: true });
    });

    it("R387 never offers Portal to the Past itself", () => {
      const s = game([
        { defId: PORTAL, radiant: false },
        { defId: MENACE, radiant: false },
      ]).play(PORTAL);
      expect(offered(s.state)).toEqual([MENACE]);
    });

    it("R564 an entry the match's catalog lacks was dropped when the match was created", () => {
      const s = game([
        { defId: "core-999", radiant: false },
        { defId: MENACE, radiant: false },
      ]);
      expect(s.state.lastBoards?.p1).toEqual([{ defId: MENACE, radiant: false }]);
    });

    it("R129 an empty last board (hotseat, a first game) fizzles: no prompt, no random draw, the Spell still played", () => {
      const s = scenario({ p1: { hand: [PORTAL, FILLER] }, p2: { hand: [FILLER] } });
      const cursor = s.state.rngCursor;
      const played = s.state.counters.played;
      s.play(PORTAL);
      expect(s.state.pending).toBeNull();
      expect(s.state.rngCursor).toBe(cursor);
      expect(s.state.counters.played).toBe(played + 1);
      expect(made(s)).toEqual([]);
      s.expectInZone(PORTAL, "graveyard").expectEvents("cardPlayed");
    });

    it("R97, R177 the options reach the chooser only: the opponent sees a prompt open, then a card added under the sentinel", () => {
      const s = game(BOARD).play(PORTAL);
      expect(s.view("p2").pending).toEqual({ forYou: false, pendingFor: "p1" });
      for (const defId of BOARD_IDS) expect(JSON.stringify(s.view("p2"))).not.toContain(`"${defId}"`);
      const pick = offered(s.state)[0] ?? "";
      s.answer(`mode:${pick}`);
      const added = s.view("p2").events.filter((event) => event.type === "addedToHand");
      expect(added).toEqual([{ type: "addedToHand", player: "p1", instanceId: "hidden", defId: "hidden" }]);
      expect(JSON.stringify(s.view("p2"))).not.toContain(`"${pick}"`);
    });

    it("R317 the pick fills a hand to its cap of 10, and fits", () => {
      const s = game([{ defId: MENACE, radiant: false }], { fillers: 9 }).play(PORTAL).answer(`mode:${MENACE}`);
      expect(s.hand("p1")).toHaveLength(10);
      expect(made(s)).toHaveLength(1);
      expect(s.lastEvents.some((event) => event.type === "burned")).toBe(false);
    });

    it("§9.3 paused mid-prompt, the state survives JSON and resumes to the same card", () => {
      const s = game(BOARD).play(PORTAL);
      const revived = JSON.parse(JSON.stringify(s.state)) as GameState;
      expect(revived).toEqual(s.state);
      const option = offered(revived)[1] ?? "";
      const result = reduce(revived, {
        type: "answer",
        playerId: "p1",
        choiceId: revived.pending?.id ?? "",
        selection: [{ pick: "mode", option }],
        nonce: "portal-round-trip",
      });
      expect(result.error).toBeUndefined();
      expect(result.state.players.p1.hand.some((card) => card.defId === option && card.costOverride === 0)).toBe(true);
      expect(result.state.work).toEqual([]);
    });

    it("§9.3, R417 in a real game it folds from (seed, decks, handicaps, lastBoards, log) to the same hash, through a JSON pause", () => {
      const p1Deck = [PORTAL, "core-002", "core-005", "core-006", "core-008", "core-011", "core-012", "core-013", "core-015", "core-016",
        "core-019", "core-020", "core-025", "core-026", "core-032", "core-036", "core-043", "core-044", "core-053", "core-055"];
      const p2Deck = ["core-015", "core-011", "core-004", "core-002", "core-005", "core-006", "core-008", "core-012", "core-013", "core-016",
        "core-019", "core-020", "core-025", "core-026", "core-032", "core-036", "core-043", "core-044", "core-053", "core-055"];
      const lastBoards: LastBoardInput = [[...BOARD, { defId: FUSED, radiant: true }], OPPONENT_BOARD];
      // p2 plays under a practice handicap (R180), so every setup input travels together.
      const handicaps = { p2: AI_TUTORIAL };
      const decks: [string[], string[]] = [p1Deck, p2Deck.slice(0, AI_TUTORIAL.deckSize)];
      const log: Action[] = [];
      const act = (state: GameState, body: ActionInput): GameState => {
        const action = { ...body, nonce: `portal-fold-${log.length}` } as Action;
        const result = reduce(state, action);
        if (result.error !== undefined) throw new Error(`${body.type}: ${result.error}`);
        log.push(action);
        return result.state;
      };

      let seed = "";
      let state: GameState | null = null;
      for (let at = 0; at < 300 && state === null; at += 1) {
        seed = `portal-fold-${at}`;
        const begun = beginGame(createGame({ seed, decks, handicaps, lastBoards })).state;
        if (begun.pending === null && begun.players.p1.hand.some((card) => card.defId === PORTAL)) state = begun;
      }
      if (state === null) throw new Error("no seed deals p1 the Portal");
      for (const player of ["p1", "p2"] as PlayerId[]) {
        state = act(state, { type: "mulligan", keep: state.players[player].hand.map((card) => card.id), playerId: player });
      }
      // To p1's first turn with (3) mana, ending every turn on the way.
      while (!(state.active === "p1" && state.players.p1.mana.current >= 3)) {
        expect(state.pending).toBeNull();
        state = act(state, { type: "endTurn", playerId: state.active });
      }
      const portal = state.players.p1.hand.find((card) => card.defId === PORTAL);
      if (portal === undefined) throw new Error("p1 no longer holds the Portal");
      state = act(state, { type: "play", playerId: "p1", instanceId: portal.id });
      expect(state.pending?.kind).toBe("discover");

      const revived = JSON.parse(JSON.stringify(state)) as GameState;
      expect(revived).toEqual(state);
      const option = offered(revived).includes(FUSED) ? FUSED : (offered(revived)[0] ?? "");
      const done = act(revived, { type: "answer", playerId: "p1", choiceId: revived.pending?.id ?? "", selection: [{ pick: "mode", option }] });
      expect(done.players.p1.hand.some((card) => card.defId === option && card.costOverride === 0)).toBe(true);

      const replayed = fold({ seed, decks, handicaps, lastBoards, log });
      expect(replayed.errors).toEqual([]);
      expect(hashState(replayed.state)).toBe(hashState(done));
      // Folded without its last boards it is another game: the boards are part of the match's inputs.
      expect(hashState(fold({ seed, decks, handicaps, log }).state)).not.toBe(hashState(done));
    });
  });

  describe("radiant: add {cards} random cards from the board your last game ended with; each costs (0)", () => {
    it("R417 adds 3 different random cards of the board straight to hand, with no prompt, each costing (0)", () => {
      const s = game(BOARD, { radiant: true }).play(PORTAL);
      expect(s.state.pending).toBeNull();
      const cards = made(s);
      expect(cards).toHaveLength(3);
      expect(new Set(cards.map((card) => card.defId)).size).toBe(3);
      for (const card of cards) {
        expect(BOARD_IDS).toContain(card.defId);
        expect(card).toMatchObject({ costOverride: 0, owner: "p1", radiant: card.defId === "core-025" });
      }
    });

    it("R417 adds them all when the board holds fewer different cards than {cards}", () => {
      const s = game([{ defId: MENACE, radiant: false }], { radiant: true }).play(PORTAL);
      expect(made(s).map((card) => card.defId)).toEqual([MENACE]);
    });

    it("R129 an empty board adds nothing and draws no random number", () => {
      const s = scenario({ p1: { hand: [{ def: PORTAL, radiant: true }, FILLER] }, p2: { hand: [FILLER] } });
      const cursor = s.state.rngCursor;
      s.play(PORTAL);
      expect(s.state.rngCursor).toBe(cursor);
      expect(made(s)).toEqual([]);
      s.expectInZone(PORTAL, "graveyard");
    });

    it("R386 the count is param(ctx, \"cards\"): an Upgrade adds 4, a Degrade 2", () => {
      const up = game(BOARD, { radiant: true });
      stepParam(up.card(PORTAL), "cards", 1);
      up.play(PORTAL);
      expect(made(up)).toHaveLength(4);

      const down = game(BOARD, { radiant: true });
      stepParam(down.card(PORTAL), "cards", -1);
      down.play(PORTAL);
      expect(made(down)).toHaveLength(2);
    });

    it("R317 a full hand burns the overflow, which both players read", () => {
      const s = game(BOARD, { radiant: true, fillers: 9 }).play(PORTAL);
      expect(s.hand("p1")).toHaveLength(10);
      const burned = s.lastEvents.filter((event) => event.type === "burned");
      expect(burned).toHaveLength(2);
      for (const event of s.view("p2").events.filter((entry) => entry.type === "burned")) {
        expect(event).toMatchObject({ owner: "p1" });
        expect(BOARD_IDS).toContain((event as { defId: string }).defId);
      }
    });

    it("R97 the opponent reads each card added to hand as the sentinel", () => {
      const s = game(BOARD, { radiant: true }).play(PORTAL);
      const added = s.view("p2").events.filter((event) => event.type === "addedToHand");
      expect(added).toHaveLength(3);
      for (const event of added) expect(event).toMatchObject({ player: "p1", instanceId: "hidden", defId: "hidden" });
    });
  });
});
