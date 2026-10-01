// C+ #62 KY's Papaya — SPEC §8.7 row 62, R422, BUILD M9 row C+ 62: the cells asked one board-cell
// prompt at a time (20, then every cell of the unused lanes and "done", at most 21 answers), x the lane
// and y the row from the caster's seat, the lowest-degree curve through the cells in exact rationals,
// every card on it exiled (the top of a pile, a card beneath resuming; face-down cards openly; tokens
// ceasing to exist; nothing between lanes); cells never cards (R177); a random cast's answers (R452);
// a pause that survives JSON and a game that replays; radiant only enemy cards. The engine's own
// proofs are `packages/engine/test/papaya.test.ts`.

import { describe, expect, it } from "vitest";
import {
  beginGame,
  createGame,
  createRng,
  fold,
  hashState,
  legalActions,
  makeContext,
  applyEffects,
  reduce,
  settle,
  type EngineSink,
  type GameState,
} from "@jackioh/engine";
import { castNew, lock } from "@jackioh/engine/effects";
import type { Action, ActionInput, PlayerId } from "@jackioh/shared";
import { base, def, radiant } from "../../src/scripts/classic-plus/062-kys-papaya";
import { scenario, type Scenario, type SideSetup } from "../_harness";

const PAPAYA = "classicplus-062";
const UNIT = "core-008"; // Mr. Vanilla, a plain 4/4.
const FIELD = "core-006"; // Mana Well, a Field Spell.
const TRAP = "core-041"; // Sheepish, set face-down.
const TOKEN = "core-t-rush";
const FILLER = "core-005";

/** A card on every cell: Mr. Vanilla in each unit zone, a Mana Well in each backrow zone. */
const FULL: SideSetup = {
  hand: [FILLER],
  field: Array.from({ length: 5 }, () => UNIT),
  backrow: Array.from({ length: 5 }, () => FIELD),
};

function full(opts: { radiant?: boolean } = {}): Scenario {
  return scenario({
    p1: { ...FULL, hand: [{ def: PAPAYA, radiant: opts.radiant === true }, FILLER] },
    p2: FULL,
  });
}

/** The option key of the cell (x, y) from p1's seat: y 0 p1's backrow, 1 p1's units, 2 p2's units, 3 p2's backrow. */
function cell(x: number, y: number, caster: PlayerId = "p1"): string {
  const other: PlayerId = caster === "p1" ? "p2" : "p1";
  const [player, row] = [
    [caster, "backrow"],
    [caster, "units"],
    [other, "units"],
    [other, "backrow"],
  ][y] as [PlayerId, string];
  return `zone:${player}:${row}:${x + 1}`;
}

function curve(s: Scenario, cells: readonly [number, number][]): Scenario {
  s.play(PAPAYA);
  for (const [x, y] of cells) s.answer(cell(x, y));
  if (cells.length < 4) s.answer("none");
  expect(s.state.pending).toBeNull();
  return s;
}

function exiled(s: Scenario): string[] {
  return [...s.pile("p1", "exile"), ...s.pile("p2", "exile")].map((card) => card.id);
}

function at(s: Scenario, player: PlayerId, row: "units" | "backrow", lane: number): string {
  const card = row === "units" ? s.unit(player, lane) : s.backrow(player, lane);
  if (card === null) throw new Error(`nothing in ${player} ${row} ${lane}`);
  return card.id;
}

describe("C+ #62 KY's Papaya", () => {
  it("is one script on both faces: the subsystem reads the running face", () => {
    expect(def.id).toBe(PAPAYA);
    expect(radiant).toBe(base);
    expect(base.resume).toBeDefined();
  });

  describe("base", () => {
    it("R422 the cells come one board-cell prompt at a time: 20 cells, then the unused lanes' cells and done, never more than 21 answers, every one in legalActions", () => {
      const s = full();
      s.play(PAPAYA);
      const sizes: number[] = [];
      for (const [x, y] of [
        [2, 1],
        [0, 3],
        [4, 0],
      ] as const) {
        const pending = s.state.pending;
        if (pending === null) throw new Error("a cell prompt");
        expect(pending.kind).toBe("cell");
        const answers = legalActions(s.state, "p1").filter((action) => action.type === "answer");
        expect(answers).toHaveLength(pending.options.length);
        sizes.push(pending.options.length);
        s.answer(cell(x, y));
      }
      sizes.push(s.state.pending?.options.length ?? 0);
      expect(sizes).toEqual([20, 17, 13, 9]);
      expect(Math.max(...sizes)).toBeLessThanOrEqual(21);
      expect(s.state.pending?.options.at(-1)?.selection).toEqual({ pick: "none" });
      expect(s.state.pending?.options.some((option) => option.key.endsWith(":3") || option.key.endsWith(":1"))).toBe(false);
    });

    it("R422 the cells are offered empty, occupied or Locked alike", () => {
      const s = scenario({ p1: { hand: [PAPAYA, FILLER] }, p2: { hand: [FILLER] } });
      const state = s.state;
      const sink: EngineSink = { state, events: [], rng: createRng(state.seed, state.rngCursor) };
      applyEffects([lock({ zone: { of: "lane", player: "enemy", row: "units", lane: 2 } })], makeContext(sink, null, { controller: "p1" }));
      state.rngCursor = sink.rng.cursor;
      s.play(PAPAYA);
      expect(s.state.pending?.options.map((option) => option.key)).toContain("zone:p2:units:2");
      expect(s.state.pending?.options).toHaveLength(20);
    });

    it("R422 x is the lane from your own lane 1 and y the row from your own backrow, on either seat", () => {
      const s = scenario({
        active: "p2",
        p1: { hand: [FILLER], field: [UNIT, UNIT], backrow: [FIELD] },
        p2: { hand: [PAPAYA, FILLER], field: [UNIT] },
      });
      s.play(PAPAYA);
      // p2's y = 2 row is p1's unit row: the constant through (1, 2) exiles p1's units in lanes 1 and 2.
      const lanes = [at(s, "p1", "units", 1), at(s, "p1", "units", 2)];
      s.answer(cell(1, 2, "p2"));
      s.answer("none");
      expect(exiled(s).sort()).toEqual(lanes.sort());
      expect(s.unit("p2", 1)).not.toBeNull();
      expect(s.backrow("p1", 1)).not.toBeNull();
    });

    it("R422 one cell is a constant: its whole row is exiled and nothing else", () => {
      const s = full();
      const row = [1, 2, 3, 4, 5].map((lane) => at(s, "p2", "backrow", lane));
      curve(s, [[2, 3]]);
      expect(exiled(s).sort()).toEqual(row.sort());
      expect(s.state.players.p1.exile).toHaveLength(0);
    });

    it("R422 two cells make a line, which meets a third cell (y = x through lanes 1 to 4)", () => {
      const s = full();
      const line = [at(s, "p1", "backrow", 1), at(s, "p1", "units", 2), at(s, "p2", "units", 3), at(s, "p2", "backrow", 4)];
      curve(s, [
        [0, 0],
        [1, 1],
      ]);
      expect(exiled(s).sort()).toEqual(line.sort());
      // Lane 5 would be row 4, off the board.
      expect(s.unit("p1", 5)).not.toBeNull();
    });

    it("R422 between lanes the curve touches nothing: y = x/2 exiles lanes 1, 3 and 5 only", () => {
      const s = full();
      const hits = [at(s, "p1", "backrow", 1), at(s, "p1", "units", 3), at(s, "p2", "units", 5)];
      curve(s, [
        [0, 0],
        [2, 1],
      ]);
      expect(exiled(s).sort()).toEqual(hits.sort());
    });

    it("R422 three cells in a line are that line, the lowest degree through them", () => {
      const s = full();
      // (0, 3), (1, 2), (3, 0) lie on y = 3 − x, which also meets (2, 1); a parabola would not.
      const line = [at(s, "p2", "backrow", 1), at(s, "p2", "units", 2), at(s, "p1", "units", 3), at(s, "p1", "backrow", 4)];
      curve(s, [
        [0, 3],
        [1, 2],
        [3, 0],
      ]);
      expect(exiled(s).sort()).toEqual(line.sort());
    });

    it("R422 four cells fix the cubic, the fourth ends the picking, and it may hit a cell of the fifth lane", () => {
      const s = full();
      // y = x³/2 − 3x² + 9x/2 through (0, 0), (1, 2), (2, 1), (3, 0) is 2 at x = 4: p2's unit in lane 5.
      const hits = [
        at(s, "p1", "backrow", 1),
        at(s, "p2", "units", 2),
        at(s, "p1", "units", 3),
        at(s, "p1", "backrow", 4),
        at(s, "p2", "units", 5),
      ];
      curve(s, [
        [0, 0],
        [1, 2],
        [2, 1],
        [3, 0],
      ]);
      expect(exiled(s).sort()).toEqual(hits.sort());
      s.expectEvents("cardPlayed", "promptOpened", "exiled", "cardResolved");
    });

    it("R422 face-down cards on the curve are exiled, and the exile names them openly", () => {
      const s = scenario({ p1: { hand: [PAPAYA, FILLER] }, p2: { hand: [FILLER], backrow: [TRAP, TRAP] } });
      const traps = [at(s, "p2", "backrow", 1), at(s, "p2", "backrow", 2)];
      expect(s.backrow("p2", 1)?.faceUp).not.toBe(true);
      curve(s, [[0, 3]]);
      expect(s.pile("p2", "exile").map((card) => card.id).sort()).toEqual(traps.sort());
      for (const viewer of ["p1", "p2"] as const) {
        const shown = s
          .view(viewer)
          .events.flatMap((event) => (event.type === "exiled" ? [[event.instanceId, event.defId]] : []));
        expect(shown.map(([id]) => id).sort()).toEqual(traps.sort());
        expect(shown.every(([, defId]) => defId === TRAP)).toBe(true);
      }
    });

    it("§6.3 Exile is no destroy: an Indestructible unit and a Reborn unit on the curve are exiled, and nothing comes back", () => {
      const s = scenario({ p1: { hand: [PAPAYA, FILLER] }, p2: { hand: [FILLER], field: ["core-066", "core-003"] } });
      const rock = s.unit("p2", 1);
      const defender = s.unit("p2", 2);
      curve(s, [[0, 2]]);
      s.expectInZone(rock!, "exile").expectInZone(defender!, "exile");
      expect(s.unit("p2", 1)).toBeNull();
      expect(s.unit("p2", 2)).toBeNull();
    });

    it("R11 tokens on the curve cease to exist", () => {
      const s = scenario({ p1: { hand: [PAPAYA, FILLER] }, p2: { hand: [FILLER], field: [TOKEN, UNIT] } });
      const token = s.unit("p2", 1);
      const unit = s.unit("p2", 2);
      curve(s, [[0, 2]]);
      s.expectInZone(token!, "gone").expectInZone(unit!, "exile");
    });

    it("§3.2 R13 the top of a Stack pile is exiled and the card beneath resumes, not exiled by the same curve", () => {
      const s = scenario({
        p1: { hand: [PAPAYA, FILLER] },
        p2: { hand: [FILLER], field: [{ def: "core-043", lane: 1 }, { def: "core-092", stack: true }] },
      });
      const top = s.unit("p2", 1);
      curve(s, [[0, 2]]);
      s.expectInZone(top!, "exile");
      expect(s.unit("p2", 1)?.defId).toBe("core-043");
    });

    it("R422 a curve over empty cells exiles nothing, and the Spell still resolves to the graveyard", () => {
      const s = scenario({ p1: { hand: [PAPAYA, FILLER] }, p2: { hand: [FILLER] } });
      curve(s, [
        [0, 1],
        [4, 2],
      ]);
      expect(exiled(s)).toEqual([]);
      s.expectInZone(PAPAYA, "graveyard");
    });

    it("R177 the prompts offer cells, never cards: the other seat sees only that a prompt is open, then the exiles", () => {
      const s = scenario({ p1: { hand: [PAPAYA, FILLER] }, p2: { hand: [FILLER], backrow: [TRAP] } });
      const trap = at(s, "p2", "backrow", 1);
      s.play(PAPAYA);
      expect(s.view("p2").pending).toEqual({ forYou: false, pendingFor: "p1" });
      expect(s.state.pending?.options.every((option) => option.selection.pick === "zone")).toBe(true);
      expect(JSON.stringify(s.view("p1").pending)).not.toContain(trap);
      expect(JSON.stringify(s.view("p2").pending)).not.toContain("zone");
      s.answer(cell(0, 3));
      expect(s.view("p2").pending).toEqual({ forYou: false, pendingFor: "p1" });
      s.answer("none");
      expect(s.view("p2").events.some((event) => event.type === "exiled" && event.instanceId === trap)).toBe(true);
    });

    it("R452 a random cast answers the same prompts at random: nothing pauses and the curve's cards are exiled", () => {
      for (let seed = 1; seed <= 4; seed += 1) {
        const s = scenario({ seed: `papaya-random-${seed}`, p1: FULL, p2: FULL });
        const laneOf = new Map<string, number>();
        for (const player of ["p1", "p2"] as const) {
          for (const lane of [1, 2, 3, 4, 5]) {
            laneOf.set(at(s, player, "units", lane), lane);
            laneOf.set(at(s, player, "backrow", lane), lane);
          }
        }
        const state = s.state;
        const sink: EngineSink = { state, events: [], rng: createRng(state.seed, state.rngCursor) };
        applyEffects([castNew({ def: PAPAYA, random: true })], makeContext(sink, null, { controller: "p1" }));
        settle(sink);
        state.rngCursor = sink.rng.cursor;
        expect(state.pending).toBeNull();
        expect(sink.events.some((event) => event.type === "promptOpened")).toBe(false);
        // A full board: at least the cell the curve was drawn through, and never two cards of one lane.
        const lanes = sink.events.flatMap((event) => (event.type === "exiled" ? [laneOf.get(event.instanceId)] : []));
        expect(lanes.length).toBeGreaterThanOrEqual(1);
        expect(new Set(lanes).size).toBe(lanes.length);
        expect(lanes.every((lane) => lane !== undefined)).toBe(true);
      }
    });

    it("R113 paused mid-answer, the cells so far survive JSON, and the copy answers to the same hash", () => {
      const s = full();
      s.play(PAPAYA);
      s.answer(cell(0, 0));
      s.answer(cell(1, 1));
      const paused = s.state;
      const copy = JSON.parse(JSON.stringify(paused)) as GameState;
      expect(copy).toEqual(paused);
      const answer = (state: GameState, selection: ActionInput & { type: "answer" }, nonce: string): GameState => {
        const result = reduce(state, { ...selection, nonce } as Action);
        if (result.error !== undefined) throw new Error(result.error);
        return result.state;
      };
      let live = paused;
      let revived = copy;
      for (const [index, key] of [cell(3, 2), "none"].entries()) {
        const pending = live.pending;
        if (pending === null) throw new Error("a prompt");
        const option = pending.options.find((held) => held.key === key);
        if (option === undefined) throw new Error(`option ${key}`);
        const body = { type: "answer" as const, playerId: "p1" as const, choiceId: pending.id, selection: [option.selection] };
        live = answer(live, body, `rt-${index}`);
        revived = answer(revived, body, `rt-${index}`);
      }
      expect(live.pending).toBeNull();
      expect(hashState(revived)).toBe(hashState(live));
    });

    it("§9.2 a game that draws a curve replays from its log to the same hash", () => {
      const units = ["core-001", "core-002", "core-004", "core-007", "core-008", "core-009", "core-011", "core-012"];
      const more = ["core-013", "core-015", "core-019", "core-020", "core-022", "core-025", "core-030", "core-032"];
      const deck = [...units, ...more, "core-037", "core-043", "core-045", PAPAYA];
      let found: { seed: string; state: GameState } | null = null;
      for (let n = 0; n < 300 && found === null; n += 1) {
        const seed = `papaya-replay-${n}`;
        const begun = beginGame(createGame({ seed, decks: [deck, deck] })).state;
        if (begun.players.p1.hand.some((card) => card.defId === PAPAYA) && begun.pending === null) found = { seed, state: begun };
      }
      if (found === null) throw new Error("no seed deals p1 the Papaya");
      const log: Action[] = [];
      let n = 0;
      const act = (state: GameState, body: ActionInput): GameState => {
        n += 1;
        const action = { ...body, nonce: `pr${n}` } as Action;
        const result = reduce(state, action);
        if (result.error !== undefined) throw new Error(result.error);
        log.push(action);
        return result.state;
      };
      let state = found.state;
      for (const player of ["p1", "p2"] as const) {
        state = act(state, { type: "mulligan", playerId: player, keep: state.players[player].hand.map((card) => card.id) });
      }
      const papaya = state.players.p1.hand.find((card) => card.defId === PAPAYA);
      state = act(state, { type: "play", playerId: "p1", instanceId: papaya?.id ?? "" });
      for (const key of [cell(0, 3), cell(2, 1), "none"]) {
        const pending = state.pending;
        const option = pending?.options.find((held) => held.key === key);
        if (pending === null || option === undefined) throw new Error(`option ${key}`);
        state = act(state, { type: "answer", playerId: "p1", choiceId: pending.id, selection: [option.selection] });
      }
      expect(state.pending).toBeNull();
      const replayed = fold({ seed: found.seed, decks: [deck, deck], log });
      expect(replayed.errors).toEqual([]);
      expect(hashState(replayed.state)).toBe(hashState(state));
    });
  });

  describe("radiant", () => {
    it("R422 only enemy cards on the curve are exiled: y = x takes their (2, 2) and (3, 3), not your (0, 0) and (1, 1)", () => {
      const s = full({ radiant: true });
      const enemy = [at(s, "p2", "units", 3), at(s, "p2", "backrow", 4)];
      curve(s, [
        [0, 0],
        [1, 1],
      ]);
      expect(exiled(s).sort()).toEqual(enemy.sort());
      expect(s.state.players.p1.exile).toHaveLength(0);
    });

    it("R422 a constant through your own row exiles nothing; through theirs, the whole row", () => {
      const own = full({ radiant: true });
      curve(own, [[1, 1]]);
      expect(exiled(own)).toEqual([]);
      const theirs = full({ radiant: true });
      const row = [1, 2, 3, 4, 5].map((lane) => at(theirs, "p2", "units", lane));
      curve(theirs, [[1, 2]]);
      expect(exiled(theirs).sort()).toEqual(row.sort());
    });

    it("R422 the Radiant face asks the same prompts", () => {
      const s = full({ radiant: true });
      s.play(PAPAYA);
      expect(s.state.pending?.options).toHaveLength(20);
      s.answer(cell(0, 0));
      expect(s.state.pending?.options).toHaveLength(17);
    });
  });
});
