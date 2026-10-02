// Classic+ #62 KY's Papaya's curve targeting (SPEC §8.7 row 62, R422; docs/classic-sets.md B5 E32),
// through the fixture Spell in `fixtures/papaya.ts`: the grid from the caster's seat, the curve in
// exact rationals, the cells on it, the one-cell-at-a-time prompts (E18's `cell` kind), the exile of
// the top card at each cell, the Radiant face's enemy rows, what the other seat sees (R97, R177), a
// random cast's answers (R452), and a pause that survives JSON and a game that replays (§9.2, §9.3).

import type { Action, ActionInput, PlayerId, Selection } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { DECK_SIZE, PAPAYA_MAX_CELLS } from "../src/config";
import { applyEffects, makeContext } from "../src/resolve";
import { MAX_PROMPT_ANSWERS, promptAnswers } from "../src/prompts";
import { beginGame, legalActions, reduce } from "../src/reduce";
import { fold, hashState } from "../src/replay";
import { createGame, newInstance, type CardInstance, type GameState } from "../src/state";
import {
  PAPAYA_LANES,
  cardsOnCurve,
  cellsOnCurve,
  curveAt,
  pointOfZone,
  zoneOfPoint,
  type PapayaPoint,
} from "../src/subsystems/papaya";
import { settle } from "../src/triggers";
import { viewFor } from "../src/viewFor";
import { cardAt, placeOnField, slotsOf } from "../src/zones";
import { vanillaDeck } from "./fixtures/catalog";
import { eventsOfType, put, setupCatalog, sinkFor, slot } from "./fixtures/harness";
import { body, curve, curveQuickdraw, field, registerPapayaFixtures, snare, token } from "./fixtures/papaya";
import { answerKeys, board, castNow, must, openAs, roundTrip } from "./fixtures/promptHarness";
import { registerPromptFixtures } from "./fixtures/prompts";
import { castNew } from "../src/effects";

function game(seed: string): GameState {
  const state = board(seed);
  registerPapayaFixtures();
  return state;
}

/** The option key of a grid cell, from p1's seat (the caster in every test here). */
function key(point: PapayaPoint, caster: PlayerId = "p1"): string {
  const zone = zoneOfPoint(caster, point);
  return `zone:${zone.player}:${zone.row}:${zone.lane}`;
}

/** Cast the curve for p1 and answer these cells, then "done" when fewer than four were given. */
function draw(state: GameState, points: readonly PapayaPoint[], radiant = false): void {
  castNow(state, curve.id, "p1", radiant);
  for (const point of points) answerKeys(state, key(point));
  if (points.length < PAPAYA_MAX_CELLS) answerKeys(state, "none");
  expect(state.pending).toBeNull();
}

function zoneOf(selection: Selection): { player: PlayerId; row: "units" | "backrow"; lane: number } {
  if (selection.pick !== "zone") throw new Error("expected a cell");
  return { player: selection.player, row: selection.row, lane: selection.lane };
}

function exiledIds(state: GameState): string[] {
  return [...state.players.p1.exile, ...state.players.p2.exile].map((card) => card.id);
}

/** A card on every one of the 20 cells: units on both unit rows, Field Spells on both backrows. */
function fullBoard(state: GameState): CardInstance[][] {
  return (["p1", "p2"] as const).map((player) => [
    ...slotsOf(player, "units").map((ref) => put(state, body.id, ref)),
    ...slotsOf(player, "backrow").map((ref) => put(state, field.id, ref)),
  ]);
}

describe("R422 the grid, from the caster's seat", () => {
  it("R422 x is the lane less one on both sides, y 0 your backrow, 1 your units, 2 their units, 3 their backrow", () => {
    expect(zoneOfPoint("p1", { x: 0, y: 0 })).toEqual({ player: "p1", row: "backrow", lane: 1 });
    expect(zoneOfPoint("p1", { x: 4, y: 1 })).toEqual({ player: "p1", row: "units", lane: 5 });
    expect(zoneOfPoint("p1", { x: 2, y: 2 })).toEqual({ player: "p2", row: "units", lane: 3 });
    expect(zoneOfPoint("p1", { x: 4, y: 3 })).toEqual({ player: "p2", row: "backrow", lane: 5 });
    // The other seat's grid is its own: its backrow is its row 0.
    expect(zoneOfPoint("p2", { x: 0, y: 0 })).toEqual({ player: "p2", row: "backrow", lane: 1 });
    expect(zoneOfPoint("p2", { x: 1, y: 2 })).toEqual({ player: "p1", row: "units", lane: 2 });
    for (const caster of ["p1", "p2"] as const) {
      for (let x = 0; x < PAPAYA_LANES; x += 1) {
        for (let y = 0; y < 4; y += 1) expect(pointOfZone(caster, zoneOfPoint(caster, { x, y }))).toEqual({ x, y });
      }
    }
    expect(() => zoneOfPoint("p1", { x: 5, y: 0 })).toThrow(/not a cell/);
    expect(() => zoneOfPoint("p1", { x: 0, y: 4 })).toThrow(/not a cell/);
  });
});

describe("R422 the curve, in exact rationals", () => {
  it("R422 one cell is a constant: the curve crosses its whole row", () => {
    expect(cellsOnCurve([{ x: 2, y: 1 }])).toEqual([0, 1, 2, 3, 4].map((x) => ({ x, y: 1 })));
  });

  it("R422 two cells make a line, which may meet a third cell and more", () => {
    const line = [
      { x: 0, y: 0 },
      { x: 1, y: 1 },
    ];
    // x = 4 would be row 4, off the grid.
    expect(cellsOnCurve(line)).toEqual([0, 1, 2, 3].map((x) => ({ x, y: x })));
  });

  it("R422 between lanes the curve touches nothing: y = x/2 meets lanes 1, 3 and 5 and passes 2 and 4 by", () => {
    const half = [
      { x: 0, y: 0 },
      { x: 2, y: 1 },
    ];
    expect(curveAt(half, 1)).toEqual({ num: 1, den: 2 });
    expect(curveAt(half, 3)).toEqual({ num: 3, den: 2 });
    expect(cellsOnCurve(half)).toEqual([
      { x: 0, y: 0 },
      { x: 2, y: 1 },
      { x: 4, y: 2 },
    ]);
  });

  it("R422 the lowest degree: three cells in a line make that line, not a parabola", () => {
    const collinear = [
      { x: 0, y: 3 },
      { x: 1, y: 2 },
      { x: 3, y: 0 },
    ];
    expect(cellsOnCurve(collinear)).toEqual([
      { x: 0, y: 3 },
      { x: 1, y: 2 },
      { x: 2, y: 1 },
      { x: 3, y: 0 },
    ]);
  });

  it("R422 three cells make a parabola in exact fractions, which may meet a fourth cell", () => {
    const arch = [
      { x: 0, y: 0 },
      { x: 1, y: 1 },
      { x: 3, y: 0 },
    ];
    expect(curveAt(arch, 2)).toEqual({ num: 1, den: 1 });
    // y = −x²/2 + 3x/2: at lane 5 it is −2, off the grid.
    expect(curveAt(arch, 4)).toEqual({ num: -2, den: 1 });
    expect(cellsOnCurve(arch)).toEqual([
      { x: 0, y: 0 },
      { x: 1, y: 1 },
      { x: 2, y: 1 },
      { x: 3, y: 0 },
    ]);
  });

  it("R422 four cells fix a cubic, which may hit a cell of the fifth lane", () => {
    const cubic = [
      { x: 0, y: 0 },
      { x: 1, y: 2 },
      { x: 2, y: 1 },
      { x: 3, y: 0 },
    ];
    // y = x³/2 − 3x² + 9x/2, which is 2 at x = 4.
    expect(curveAt(cubic, 4)).toEqual({ num: 2, den: 1 });
    expect(cellsOnCurve(cubic)).toEqual([...cubic, { x: 4, y: 2 }]);
  });

  it("R422 a cubic that leaves the grid in the fifth lane meets nothing there", () => {
    const zigzag = [
      { x: 0, y: 0 },
      { x: 1, y: 1 },
      { x: 2, y: 0 },
      { x: 3, y: 1 },
    ];
    expect(curveAt(zigzag, 4)).toEqual({ num: 8, den: 1 });
    expect(cellsOnCurve(zigzag)).toEqual(zigzag);
  });

  it("R422 the cells are 1 to 4 grid cells in different lanes", () => {
    expect(() => cellsOnCurve([])).toThrow(/1 to 4 grid cells in different lanes/);
    expect(() =>
      cellsOnCurve([
        { x: 1, y: 0 },
        { x: 1, y: 2 },
      ]),
    ).toThrow(/different lanes/);
    expect(() => cellsOnCurve([{ x: 0, y: 4 }])).toThrow(/grid cells/);
  });
});

describe("R422 the cells are asked one board-cell prompt at a time", () => {
  it("R422 the first prompt offers all 20 cells and no done, and legalActions lists each of them", () => {
    const state = game("papaya-first");
    castNow(state, curve.id);
    const first = openAs(state, "cell", "p1");
    expect(first.options).toHaveLength(20);
    expect(first.options.every((option) => option.selection.pick === "zone")).toBe(true);
    const answers = legalActions(state, "p1").filter((action) => action.type === "answer");
    expect(answers).toHaveLength(20);
    // The chooser's own rows from the hero outward, then the enemy's: the grid's rows in order.
    expect(first.options.map((option) => pointOfZone("p1", zoneOf(option.selection)).y)).toEqual(
      [0, 1, 2, 3].flatMap((y) => Array.from({ length: 5 }, () => y)),
    );
  });

  it("R422 each later prompt offers the 4 cells of every unused lane and done, never more than 21 answers", () => {
    const state = game("papaya-later");
    castNow(state, curve.id);
    const picks = [
      { x: 1, y: 2 },
      { x: 3, y: 0 },
      { x: 0, y: 3 },
    ];
    for (const [at, point] of picks.entries()) {
      answerKeys(state, key(point));
      const pending = openAs(state, "cell", "p1");
      const lanesLeft = PAPAYA_LANES - (at + 1);
      expect(pending.options).toHaveLength(4 * lanesLeft + 1);
      expect(pending.options.at(-1)?.selection).toEqual({ pick: "none" });
      const used = picks.slice(0, at + 1).map((p) => p.x + 1);
      expect(
        pending.options.some((option) => option.selection.pick === "zone" && used.includes(option.selection.lane)),
      ).toBe(false);
      expect(promptAnswers(pending).length).toBeLessThanOrEqual(21);
      expect(promptAnswers(pending).length).toBeLessThan(MAX_PROMPT_ANSWERS);
      expect(legalActions(state, "p1").filter((action) => action.type === "answer")).toHaveLength(4 * lanesLeft + 1);
    }
  });

  it("R422 the fourth cell ends the picking (PAPAYA_MAX_CELLS) and draws the cubic", () => {
    const state = game("papaya-four");
    const [, enemy] = fullBoard(state);
    castNow(state, curve.id);
    const cubic = [
      { x: 0, y: 0 },
      { x: 1, y: 2 },
      { x: 2, y: 1 },
      { x: 3, y: 0 },
    ];
    for (const point of cubic) answerKeys(state, key(point));
    expect(state.pending).toBeNull();
    // The fifth lane's cell (4, 2) is the enemy's unit in lane 5.
    expect(exiledIds(state)).toContain(enemy?.[4]?.id);
    expect(exiledIds(state)).toHaveLength(5);
  });

  it("R97 R177 the other seat sees only that a prompt is open; the options are cells, never cards", () => {
    const state = game("papaya-hidden");
    const trap = put(state, snare.id, slot("p2", "backrow", 2));
    castNow(state, curve.id);
    expect(viewFor(state, "p2").pending).toEqual({ forYou: false, pendingFor: "p1" });
    const pending = openAs(state, "cell", "p1");
    expect(JSON.stringify(pending.options)).not.toContain(trap.id);
    expect(JSON.stringify(viewFor(state, "p1").pending)).not.toContain(trap.id);
    for (const opened of eventsOfType(viewFor(state, "p2").events, "promptOpened")) {
      expect(Object.keys(opened).sort()).toEqual(["choiceId", "kind", "player", "type"]);
    }
  });
});

describe("R422 every card on the curve is exiled", () => {
  it("R422 one cell's constant exiles its whole row and nothing else", () => {
    const state = game("papaya-row");
    const [own, enemy] = fullBoard(state);
    draw(state, [{ x: 3, y: 2 }]);
    expect(exiledIds(state).sort()).toEqual((enemy ?? []).slice(0, 5).map((card) => card.id).sort());
    expect((own ?? []).every((card) => card.zone.z === "field")).toBe(true);
  });

  it("R422 a line exiles the third cell it meets, and nothing between lanes", () => {
    const state = game("papaya-line");
    const [own, enemy] = fullBoard(state);
    // y = x/2: (0, 0) your backrow lane 1, (2, 1) your units lane 3, (4, 2) their units lane 5.
    draw(state, [
      { x: 0, y: 0 },
      { x: 2, y: 1 },
    ]);
    expect(exiledIds(state).sort()).toEqual([own?.[5]?.id, own?.[2]?.id, enemy?.[4]?.id].sort());
  });

  it("R422 a face-down trap on the curve is exiled, and the exile names it openly to both players", () => {
    const state = game("papaya-trap");
    const trap = put(state, snare.id, slot("p2", "backrow", 4));
    expect(trap.faceUp).not.toBe(true);
    castNow(state, curve.id);
    answerKeys(state, key({ x: 0, y: 3 }));
    const { sink } = answerKeys(state, "none");
    expect(state.players.p2.exile.map((card) => card.id)).toEqual([trap.id]);
    expect(eventsOfType(sink.events, "exiled").map((event) => event.instanceId)).toEqual([trap.id]);
    state.applied = [{ nonce: "pp", events: sink.events }];
    for (const viewer of ["p1", "p2"] as const) {
      const exiled = eventsOfType(viewFor(state, viewer).events, "exiled");
      expect(exiled.map((event) => [event.instanceId, event.defId])).toEqual([[trap.id, snare.id]]);
    }
  });

  it("R11 a unit token on the curve ceases to exist instead of reaching the exile pile", () => {
    const state = game("papaya-token");
    const made = put(state, token.id, slot("p2", "units", 1));
    draw(state, [{ x: 0, y: 2 }]);
    expect(made.zone.z).toBe("gone");
    expect(state.players.p2.exile).toHaveLength(0);
  });

  it("§3.2 R13 the top of a Stack pile is exiled and the card beneath resumes, not exiled by the same curve", () => {
    const state = game("papaya-stack");
    const beneath = put(state, body.id, slot("p2", "units", 2));
    const top = newInstance(state, body.id, "p2", { z: "hand", player: "p2" });
    expect(placeOnField(state, top, slot("p2", "units", 2), { stack: true })).toBe(true);
    draw(state, [{ x: 1, y: 2 }]);
    expect(top.zone.z).toBe("exile");
    expect(cardAt(state, slot("p2", "units", 2))?.id).toBe(beneath.id);
    expect(beneath.zone.z).toBe("field");
  });

  it("R422 the cells are offered empty or not, and a curve over empty cells exiles nothing", () => {
    const state = game("papaya-empty");
    draw(state, [
      { x: 0, y: 1 },
      { x: 4, y: 3 },
    ]);
    expect(exiledIds(state)).toEqual([]);
  });

  it("R422 Radiant exiles only the enemy's cards on the curve, rows 2 and 3", () => {
    const state = game("papaya-radiant");
    const [own, enemy] = fullBoard(state);
    // y = x: (0, 0) and (1, 1) are yours, (2, 2) and (3, 3) the enemy's.
    draw(
      state,
      [
        { x: 0, y: 0 },
        { x: 1, y: 1 },
      ],
      true,
    );
    expect(exiledIds(state).sort()).toEqual([enemy?.[2]?.id, enemy?.[8]?.id].sort());
    expect((own ?? []).every((card) => card.zone.z === "field")).toBe(true);
  });

  it("R422 cardsOnCurve reads the board once, the tops of the piles in lane order", () => {
    const state = game("papaya-read");
    const [own] = fullBoard(state);
    const ids = cardsOnCurve({ state, controller: "p1" }, [{ x: 0, y: 1 }], false);
    expect(ids).toEqual((own ?? []).slice(0, 5).map((card) => card.id));
    expect(cardsOnCurve({ state, controller: "p1" }, [{ x: 0, y: 1 }], true)).toEqual([]);
    expect(cardsOnCurve({ state, controller: "p1" }, [], false)).toEqual([]);
  });
});

describe("R452 R113 §9.3 random casts, pauses and replays", () => {
  it("R452 a random cast answers every cell prompt at random: nothing pauses, and its curve exiles what lies on it", () => {
    for (let seed = 1; seed <= 6; seed += 1) {
      const state = game(`papaya-random-${seed}`);
      const lanes = new Map(fullBoard(state).flatMap((cards) => cards.map((card, at) => [card.id, (at % 5) + 1] as const)));
      const sink = sinkFor(state);
      applyEffects([castNew({ def: curve.id, random: true })], makeContext(sink, null, { controller: "p1" }));
      settle(sink);
      state.rngCursor = sink.rng.cursor;
      expect(state.pending).toBeNull();
      expect(eventsOfType(sink.events, "promptOpened")).toEqual([]);
      const exiled = eventsOfType(sink.events, "exiled");
      // A full board: the curve passes through at least the cell it was drawn through.
      expect(exiled.length).toBeGreaterThanOrEqual(1);
      expect(exiled.length).toBeLessThanOrEqual(PAPAYA_LANES);
      // A curve is a function of the lane: never two cards of one lane.
      const hit = exiled.map((event) => must(lanes.get(event.instanceId), "an exiled card's lane"));
      expect(new Set(hit).size).toBe(hit.length);
    }
  });

  it("R113 a pause between cells survives JSON, and the copy answers to the very same state", () => {
    const state = game("papaya-json");
    fullBoard(state);
    castNow(state, curve.id);
    answerKeys(state, key({ x: 0, y: 3 }));
    answerKeys(state, key({ x: 2, y: 2 }));
    const copy = roundTrip(state);
    expect(copy).toEqual(state);
    for (const target of [state, copy]) {
      answerKeys(target, key({ x: 4, y: 1 }));
      answerKeys(target, "none");
    }
    expect(copy.pending).toBeNull();
    expect(hashState(copy)).toBe(hashState(state));
    expect(exiledIds(copy)).toHaveLength(exiledIds(state).length);
  });

  it("§9.2 a game that draws a curve replays from its log to the same hash", () => {
    setupCatalog();
    registerPromptFixtures();
    registerPapayaFixtures();
    const seed = "papaya-replay";
    const decks: [string[], string[]] = [
      [...vanillaDeck(DECK_SIZE - 1, 1), curveQuickdraw.id],
      [...vanillaDeck(DECK_SIZE, 21)],
    ];
    const log: Action[] = [];
    let n = 0;
    const act = (current: GameState, input: ActionInput): GameState => {
      n += 1;
      const action = { ...input, nonce: `pp${n}` } as Action;
      const result = reduce(current, action);
      if (result.error !== undefined) throw new Error(result.error);
      log.push(action);
      return result.state;
    };
    let state = beginGame(createGame({ seed, decks })).state;
    for (const player of ["p1", "p2"] as const) {
      state = act(state, { type: "mulligan", playerId: player, keep: state.players[player].hand.map((card) => card.id) });
    }
    const papaya = must(state.players.p1.hand.find((card) => card.defId === curveQuickdraw.id), "the quickdraw curve");
    state = act(state, { type: "play", playerId: "p1", instanceId: papaya.id });
    for (const selection of [
      { pick: "zone" as const, player: "p1" as const, row: "units" as const, lane: 1 },
      { pick: "zone" as const, player: "p2" as const, row: "units" as const, lane: 3 },
      { pick: "none" as const },
    ]) {
      const pending = must(state.pending, "a cell prompt");
      state = act(state, { type: "answer", playerId: "p1", choiceId: pending.id, selection: [selection] });
    }
    expect(state.pending).toBeNull();
    const folded = fold({ seed, decks, log });
    expect(folded.errors).toEqual([]);
    expect(hashState(folded.state)).toBe(hashState(state));
  });
});
