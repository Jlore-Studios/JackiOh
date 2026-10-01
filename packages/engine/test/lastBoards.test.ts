// B5 E30, last boards: a match setup input from outside the match (SPEC §8.7 C+ #29, §9.3, §10.1,
// R417, R564), driven through `fixtures/lastBoards.ts`' Portal, the smallest script with C+ #29's
// two faces. The real card's test (packages/cards/test/classic-plus/029-portal-to-the-past.test.ts)
// covers the same cases again with the real catalog.

import { describe, expect, it } from "vitest";
import type { Action, PlayerId, Selection } from "@jackioh/shared";
import { registeredCatalog } from "../src/catalog";
import { DECK_SIZE } from "../src/config";
import { fold, hashState } from "../src/replay";
import { registeredScripts } from "../src/scripts";
import { createGame, newInstance, type GameState, type LastBoardInput } from "../src/state";
import { freezeLastBoards, lastBoardCandidates, lastBoardFor, rebuildableFromId } from "../src/subsystems/lastBoards";
import { HIDDEN_ID, viewFor } from "../src/viewFor";
import { placeOnField } from "../src/zones";
import { vanillaDeck } from "./fixtures/catalog";
import { put, slot } from "./fixtures/harness";
import { FIELD_TRAP, LB_DECKS, PORTAL, PORTAL_RADIANT_CARDS, TRAP, act, portalGame, registerLastBoards } from "./fixtures/lastBoards";

const FUSED = "t-1:fx-1+fx-2";
const NESTED = "t-2:(t-1:fx-1+fx-2)+fx-3";

function decks(): [string[], string[]] {
  return [vanillaDeck(DECK_SIZE, 1), vanillaDeck(DECK_SIZE, 21)];
}

function created(lastBoards?: LastBoardInput): GameState {
  registerLastBoards();
  return createGame({ seed: "lb", decks: decks(), ...(lastBoards === undefined ? {} : { lastBoards }) });
}

function portalOf(state: GameState) {
  const card = state.players.p1.hand.find((entry) => entry.defId === PORTAL);
  if (card === undefined) throw new Error("p1 holds no Portal");
  return card;
}

function play(state: GameState, log: Action[]): GameState {
  return act(state, log, { type: "play", playerId: "p1", instanceId: portalOf(state).id });
}

function answerWith(state: GameState, log: Action[], option: string): GameState {
  const pending = state.pending;
  if (pending === null) throw new Error("no prompt is open");
  const selection: Selection[] = [{ pick: "mode", option }];
  return act(state, log, { type: "answer", playerId: pending.playerId, choiceId: pending.id, selection });
}

describe("B5 E30 last boards as a createGame input (R417)", () => {
  it("R417 each seat's board is frozen into the match as card and face only", () => {
    const state = created([
      [{ defId: "fx-3", radiant: true, damage: 4 } as never, { defId: "fx-4", radiant: false }],
      [{ defId: "fx-5" } as never],
    ]);
    expect(state.lastBoards).toEqual({
      p1: [
        { defId: "fx-3", radiant: true },
        { defId: "fx-4", radiant: false },
      ],
      p2: [{ defId: "fx-5", radiant: false }],
    });
  });

  it("R417 no last boards, or two empty ones, store nothing: the game hashes as one created without", () => {
    const without = created();
    expect(without.lastBoards).toBeUndefined();
    expect(hashState(created([[], []]))).toBe(hashState(without));
    expect(hashState(created([[{ defId: "nope", radiant: false }], []]))).toBe(hashState(without));
  });

  it("R564 an entry this match cannot rebuild from its id is dropped as the match is created", () => {
    const catalog = (registerLastBoards(), registeredCatalog());
    expect(rebuildableFromId("fx-1", catalog)).toBe(true);
    expect(rebuildableFromId("fx-token-rush", catalog)).toBe(true);
    expect(rebuildableFromId(FUSED, catalog)).toBe(true);
    expect(rebuildableFromId(NESTED, catalog)).toBe(true);
    expect(rebuildableFromId("core-999", catalog)).toBe(false);
    expect(rebuildableFromId("t-3", catalog)).toBe(false);
    expect(rebuildableFromId("t-1:fx-1+nope", catalog)).toBe(false);
    expect(rebuildableFromId("t-1:fx-1", catalog)).toBe(false);
    // R468: a digest names its list only to the process that minted it, so it never rebuilds.
    expect(rebuildableFromId("t-4:#0123456789abcdef", catalog)).toBe(false);
    expect(rebuildableFromId("t-5:(t-4:#0123456789abcdef)+fx-1", catalog)).toBe(false);

    expect(
      freezeLastBoards(
        [[{ defId: "nope", radiant: true }, { defId: FUSED, radiant: true }, null as never, { radiant: true } as never], [{ defId: "t-4:#0123456789abcdef", radiant: false }]],
        catalog,
      ),
    ).toEqual({ p1: [{ defId: FUSED, radiant: true }] });
  });

  it("R417 the frozen boards survive JSON", () => {
    const state = created([[{ defId: FUSED, radiant: false }], [{ defId: "fx-9", radiant: true }]]);
    const revived = JSON.parse(JSON.stringify(state)) as GameState;
    expect(revived).toEqual(state);
    expect(hashState(revived)).toBe(hashState(state));
  });

  it("R417 viewFor never sends a last board, to either seat", () => {
    // Two cards no deck holds, so nothing else in a view can name them.
    const state = created([[{ defId: TRAP, radiant: true }], [{ defId: FIELD_TRAP, radiant: true }]]);
    for (const viewer of ["p1", "p2"] as PlayerId[]) {
      const text = JSON.stringify(viewFor(state, viewer));
      expect(text).not.toContain(TRAP);
      expect(text).not.toContain(FIELD_TRAP);
      expect(text).not.toContain("lastBoard");
    }
  });
});

describe("B5 E30 the reader the server calls as a game ends (R417)", () => {
  function board(): GameState {
    const state = created();
    put(state, "fx-1", slot("p1", "units", 1), { radiant: true });
    put(state, "fx-2", slot("p1", "units", 2));
    // A Stack pile: the dormant card beneath is not on the field (R13).
    const top = newInstance(state, "fx-3", "p1", { z: "hand", player: "p1" });
    if (!placeOnField(state, top, slot("p1", "units", 2), { stack: true })) throw new Error("stack");
    put(state, TRAP, slot("p1", "backrow", 1));
    // A Unit carried on p1's trap (R446).
    const carried = newInstance(state, "fx-4", "p1", { z: "hand", player: "p1" });
    if (!placeOnField(state, carried, slot("p1", "backrow", 1), { stack: true })) throw new Error("carry");
    put(state, "fx-21", slot("p2", "units", 3));
    put(state, TRAP, slot("p2", "backrow", 2), { radiant: true });
    const fired = put(state, FIELD_TRAP, slot("p2", "backrow", 3));
    fired.faceUp = true;
    // p2's trap that p1 controls now (a steal, R33): p1 reads it, p2 no longer does.
    const stolen = newInstance(state, FIELD_TRAP, "p2", { z: "hand", player: "p2" });
    if (!placeOnField(state, stolen, slot("p1", "backrow", 4))) throw new Error("stolen");
    // Damage and buffs are the card's state, never its entry.
    const hurt = state.players.p2.units[2]?.[0];
    if (hurt === undefined) throw new Error("p2 unit");
    hurt.damage = 1;
    hurt.buffs = { attack: 3, health: 3 };
    return state;
  }

  it("R417 every card on the field, both sides, each seat minus the other side's face-down cards (R33)", () => {
    const state = board();
    expect(lastBoardFor(state, "p1")).toEqual([
      { defId: "fx-1", radiant: true },
      { defId: "fx-3", radiant: false },
      { defId: "fx-4", radiant: false },
      { defId: TRAP, radiant: false },
      { defId: FIELD_TRAP, radiant: false },
      { defId: "fx-21", radiant: false },
      { defId: FIELD_TRAP, radiant: false },
    ]);
    expect(lastBoardFor(state, "p2")).toEqual([
      { defId: "fx-1", radiant: true },
      { defId: "fx-3", radiant: false },
      { defId: "fx-4", radiant: false },
      { defId: "fx-21", radiant: false },
      { defId: TRAP, radiant: true },
      { defId: FIELD_TRAP, radiant: false },
    ]);
  });

  it("R417 a seat's board is exactly what its own view shows of the field", () => {
    const state = board();
    for (const seat of ["p1", "p2"] as PlayerId[]) {
      const view = viewFor(state, seat);
      const shown = [view.you, view.opponent]
        .sort((a, b) => (a.player < b.player ? -1 : 1))
        .flatMap((side) => [
          ...side.units.flatMap((unit) => (unit === null ? [] : [unit.defId])),
          ...side.backrow.flatMap((card, lane) => [
            ...(side.carried?.[lane] ? [side.carried[lane]?.defId ?? ""] : []),
            ...(card === null || card.faceDown ? [] : [card.defId]),
          ]),
        ]);
      expect(lastBoardFor(state, seat).map((entry) => entry.defId)).toEqual(shown);
    }
  });

  it("R417 an empty field is an empty board", () => {
    expect(lastBoardFor(created(), "p1")).toEqual([]);
  });
});

describe("B5 E30 C+ #29's verbs over the frozen board (R417, R564)", () => {
  // p2's deck holds fx-21 to fx-40, so none of these can reach p1's hand any other way.
  const P1_BOARD = [
    { defId: "fx-25", radiant: false },
    { defId: "fx-26", radiant: false },
    { defId: "fx-25", radiant: true },
    { defId: "fx-27", radiant: false },
    { defId: "fx-token-rush", radiant: false },
    { defId: PORTAL, radiant: false },
  ];
  const P1_CARDS = ["fx-25", "fx-26", "fx-27", "fx-token-rush"];
  const P2_BOARD = [{ defId: TRAP, radiant: false }];

  it("R564 the candidates are each different card once, Radiant if any entry was, never the generating card (R387)", () => {
    const state = created([P1_BOARD, P2_BOARD]);
    expect(lastBoardCandidates(state, "p1", [PORTAL])).toEqual([
      { defId: "fx-25", radiant: true },
      { defId: "fx-26", radiant: false },
      { defId: "fx-27", radiant: false },
      { defId: "fx-token-rush", radiant: false },
    ]);
    expect(lastBoardCandidates(state, "p2")).toEqual(P2_BOARD);
  });

  it("R417 base: Discover 3 different cards of the caster's own board, shown to the caster only (§10.8, R81)", () => {
    const run = portalGame("lb-discover", [P1_BOARD, P2_BOARD]);
    const state = play(run.state, run.log);
    const pending = state.pending;
    expect(pending?.kind).toBe("discover");
    expect(pending?.playerId).toBe("p1");
    const offered = pending?.options.map((option) => (option.selection.pick === "mode" ? option.selection.option : "")) ?? [];
    expect(offered).toHaveLength(3);
    expect(new Set(offered).size).toBe(3);
    for (const defId of offered) expect(P1_CARDS).toContain(defId);
    // The other seat learns that a prompt is open and whose, and nothing of its options (R81, R177).
    expect(viewFor(state, "p2").pending).toEqual({ forYou: false, pendingFor: "p1" });
    expect(JSON.stringify(viewFor(state, "p2"))).not.toContain("fx-token-rush");
  });

  it("R417 the pick arrives as a new card the caster owns, on its entry's face, costing (0); the other seat reads the sentinel (R97)", () => {
    const run = portalGame("lb-pick", [[{ defId: "fx-25", radiant: true }], P2_BOARD]);
    let state = play(run.state, run.log);
    expect(state.pending?.options.map((option) => option.radiant)).toEqual([true]);
    state = answerWith(state, run.log, "fx-25");
    const made = state.players.p1.hand.find((card) => card.defId === "fx-25");
    expect(made).toMatchObject({ owner: "p1", controller: "p1", radiant: true, costOverride: 0, damage: 0 });
    expect(state.pending).toBeNull();
    const seen = viewFor(state, "p2").events.filter((event) => event.type === "addedToHand");
    expect(seen.at(-1)).toMatchObject({ instanceId: HIDDEN_ID, defId: HIDDEN_ID });
  });

  it("R129 an empty last board fizzles: no prompt, no random draw, and the Spell still counts as played", () => {
    const run = portalGame("lb-empty");
    const before = run.state.rngCursor;
    const played = run.state.counters.played;
    const state = play(run.state, run.log);
    expect(state.pending).toBeNull();
    expect(state.rngCursor).toBe(before);
    expect(state.counters.played).toBe(played + 1);
    expect(state.players.p1.graveyard.map((card) => card.defId)).toContain(PORTAL);
  });

  it("R179 a fused entry is rebuilt from its id: definition and scripts, nested fusions included", () => {
    const run = portalGame("lb-fused", [[{ defId: NESTED, radiant: false }], []]);
    let state = play(run.state, run.log);
    state = answerWith(state, run.log, NESTED);
    const made = state.players.p1.hand.find((card) => card.defId === NESTED);
    expect(made?.costOverride).toBe(0);
    // Summed faces: fx-1, fx-2 and fx-3 are 2/2 each, so 6/6, and 12/12 Radiant.
    expect(state.transientDefs[NESTED]).toMatchObject({ base: { attack: 6, health: 6 }, radiant: { attack: 12, health: 12 }, type: "Unit" });
    expect(state.transientDefs[FUSED]).toMatchObject({ base: { attack: 4, health: 4 } });
    expect(registeredScripts()[NESTED]).toBeDefined();
  });

  it("R417 Radiant adds 3 different random cards straight to hand, each costing (0)", () => {
    const run = portalGame("lb-radiant", [P1_BOARD, P2_BOARD]);
    portalOf(run.state).radiant = true;
    const state = play(run.state, run.log);
    expect(state.pending).toBeNull();
    const added = state.players.p1.hand.filter((card) => card.costOverride === 0);
    expect(added).toHaveLength(PORTAL_RADIANT_CARDS);
    expect(new Set(added.map((card) => card.defId)).size).toBe(PORTAL_RADIANT_CARDS);
    for (const card of added) expect(P1_CARDS).toContain(card.defId);
  });

  it("R417 paused mid-prompt, the state survives JSON and folds from (seed, decks, lastBoards, log) to the same hash", () => {
    const boards: LastBoardInput = [[...P1_BOARD, { defId: FUSED, radiant: true }], P2_BOARD];
    const run = portalGame("lb-fold", boards);
    const paused = play(run.state, run.log);
    const revived = JSON.parse(JSON.stringify(paused)) as GameState;
    expect(revived).toEqual(paused);
    const option = revived.pending?.options[0]?.selection;
    if (option?.pick !== "mode") throw new Error("no mode option");
    const done = answerWith(revived, run.log, option.option);

    const replayed = fold({ seed: run.seed, decks: LB_DECKS, lastBoards: boards, log: run.log });
    expect(replayed.errors).toEqual([]);
    expect(hashState(replayed.state)).toBe(hashState(done));
    // Folded without them, it is another game.
    expect(hashState(fold({ seed: run.seed, decks: LB_DECKS, log: run.log }).state)).not.toBe(hashState(done));
  });
});
