// The Glitch Easter egg (issue #170; subsystems/glitch.ts, catalog.pickGenerated; R672–R678). Through a
// fixture Glitch registered under the real id; the real card's test (packages/cards/test/classic/
// t-glitch-glitch.test.ts) covers the same cases again on the real catalog.

import type { Action, CardDef, GameEvent } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { findDef, pickGenerated, query, registerCatalog, registeredCatalog } from "../src/catalog";
import { GLITCH_DEF_ID, GLITCH_ODDS_DENOMINATOR, GLITCH_OUTCOMES, SETUP_TURN, SYSTEM_CARD_DEF_IDS } from "../src/config";
import { glitch } from "../src/effects";
import { playCost } from "../src/mana";
import { reduce } from "../src/reduce";
import { hashState } from "../src/replay";
import { makeContext } from "../src/resolve";
import { createRng } from "../src/rng";
import { registerScripts, registeredScripts } from "../src/scripts";
import { cloneState, createGame, newInstance, type GameState } from "../src/state";
import { countSystemPlay, seatPlayedBy, seatsSwapped } from "../src/subsystems/glitch";
import { cardAt, slotsOf } from "../src/zones";
import { spellDef } from "./fixtures/catalog";
import { inHand, newGame, put, sinkFor, slot } from "./fixtures/harness";

const GLITCH: CardDef = {
  ...spellDef(9001, { id: GLITCH_DEF_ID, index: "T-glitch", name: "Glitch", set: "Classic", token: true, rarity: "Token", tags: ["Token"] }),
  cost: 0,
  base: { keywords: [], text: "" },
  radiant: { keywords: [], text: "" },
};

/** A game at p1's turn 3 in its main phase, Glitch registered beside the fixture catalog. */
function game(seed: string): GameState {
  const state = newGame(seed);
  registerCatalog({ ...registeredCatalog(), [GLITCH.id]: GLITCH });
  registerScripts({ ...registeredScripts(), [GLITCH.id]: { base: { cry: () => [glitch()] }, radiant: { cry: () => [glitch()] } } });
  state.phase = "main";
  state.turn = 3;
  state.active = "p1";
  state.players.p1.turnsStarted = 2;
  state.players.p2.turnsStarted = 1;
  state.players.p1.mana = { ...state.players.p1.mana, current: 0, max: 2 };
  return state;
}

/** The first seed whose Glitch draws `outcome` when p1 plays it (each outcome is one rng draw, R675). */
function seedFor(outcome: (typeof GLITCH_OUTCOMES)[number]): string {
  for (let n = 0; n < 200; n += 1) {
    const seed = `glitch-${outcome}-${n}`;
    const rng = createRng(seed, game(seed).rngCursor);
    if (GLITCH_OUTCOMES[rng.int(GLITCH_OUTCOMES.length)] === outcome) return seed;
  }
  throw new Error(`no seed draws ${outcome}`);
}

function playGlitch(state: GameState): { state: GameState; events: GameEvent[] } {
  const [card] = inHand(state, GLITCH_DEF_ID, "p1");
  const action: Action = { type: "play", playerId: "p1", instanceId: card?.id ?? "", nonce: "glitch" };
  const result = reduce(state, action);
  expect(result.error).toBeUndefined();
  return result;
}

describe("R672 Glitch's odds", () => {
  it("R672 with no System play a pick for a hand draws exactly as before: one draw, the same card", () => {
    game("odds-none");
    const pool = query({ type: "Unit" });
    const a = createRng("odds-none");
    const b = createRng("odds-none");
    expect(pickGenerated(a, pool, { systemPlays: 0 })).toBe(b.pick(pool));
    expect(pickGenerated(a, pool, {})).toBe(b.pick(pool));
    expect(a.cursor).toBe(b.cursor);
  });

  it("R672 after n System plays one more draw makes the pick Glitch when it falls under n/10000", () => {
    game("odds-some");
    const pool = query({ type: "Unit" });
    let glitched = 0;
    let kept = 0;
    for (let n = 0; n < 4000 && (glitched === 0 || kept === 0); n += 1) {
      const probe = createRng(`odds-${n}`);
      probe.pick(pool);
      const roll = probe.int(GLITCH_ODDS_DENOMINATOR);
      const rng = createRng(`odds-${n}`);
      const def = pickGenerated(rng, pool, { systemPlays: 3 });
      expect(rng.cursor).toBe(2);
      if (roll < 3) {
        expect(def?.id).toBe(GLITCH_DEF_ID);
        glitched += 1;
      } else {
        expect(def?.id).not.toBe(GLITCH_DEF_ID);
        kept += 1;
      }
    }
    expect(kept).toBeGreaterThan(0);
    // 10000 out of 10000: every pick is Glitch.
    expect(pickGenerated(createRng("all"), pool, { systemPlays: GLITCH_ODDS_DENOMINATOR })?.id).toBe(GLITCH_DEF_ID);
  });

  it("R672 counts every play of a … in the System card, by either player, and nothing else", () => {
    const state = game("count");
    for (const defId of [...SYSTEM_CARD_DEF_IDS, "fx-1"]) {
      countSystemPlay(state, newInstance(state, defId, "p2", { z: "resolving", player: "p2" }));
    }
    expect(state.systemPlays).toBe(SYSTEM_CARD_DEF_IDS.length);
  });

  it("R673 no pool holds Glitch, not even one that takes every token; naming it by id still finds it", () => {
    game("pools");
    expect(query({ withTokens: true }).map((def) => def.id)).not.toContain(GLITCH_DEF_ID);
    expect(query({ tags: ["Token"] }).map((def) => def.id)).not.toContain(GLITCH_DEF_ID);
    expect(query({ defId: GLITCH_DEF_ID }).map((def) => def.id)).toEqual([GLITCH_DEF_ID]);
  });
});

describe("R674 Glitch is always playable on its owner's turn", () => {
  it("R674 costs (0) whatever modifies it, so it is played with no mana", () => {
    const state = game("price");
    const [card] = inHand(state, GLITCH_DEF_ID, "p1");
    if (card === undefined) throw new Error("no Glitch");
    card.costMod = 5;
    card.costOverride = 3;
    expect(playCost(state, card)).toBe(0);
    expect(reduce(state, { type: "play", playerId: "p1", instanceId: card.id, nonce: "free" }).error).toBeUndefined();
  });

  it("R674 is not playable on the other player's turn", () => {
    const state = game("their-turn");
    const [card] = inHand(state, GLITCH_DEF_ID, "p2");
    expect(reduce(state, { type: "play", playerId: "p2", instanceId: card?.id ?? "", nonce: "no" }).error).toBeDefined();
  });
});

describe("R675 Glitch's outcome and the reset", () => {
  it("R675 draws its outcome from the match rng and says which, publicly", () => {
    for (const outcome of GLITCH_OUTCOMES) {
      const { events } = playGlitch(game(seedFor(outcome)));
      expect(events.filter((event) => event.type === "glitched")).toEqual([{ type: "glitched", player: "p1", outcome }]);
    }
  });

  it("R675 a reset deals the match again from its decks: setup, mulligans open, fresh ids, the nonce log kept", () => {
    const before = game(seedFor("reset"));
    before.systemPlays = 2;
    const oldIds = new Set([...before.players.p1.library, ...before.players.p2.library].map((card) => card.id));
    const { state, events } = playGlitch(before);
    expect(state.turn).toBe(SETUP_TURN);
    expect(state.phase).toBe("mulligan");
    expect(state.mulligan?.p1).toBeDefined();
    expect(state.systemPlays).toBeUndefined();
    expect(state.resetOwed).toBeUndefined();
    expect(state.resets).toBe(1);
    expect(state.opening).toEqual(before.opening);
    expect(state.applied.map((entry) => entry.nonce)).toContain("glitch");
    const dealt = [...state.players.p1.hand, ...state.players.p1.library];
    expect(dealt).toHaveLength(before.opening?.decks[0].length ?? 0);
    expect(dealt.some((card) => oldIds.has(card.id))).toBe(false);
    expect(events.some((event) => event.type === "glitched")).toBe(true);
  });

  it("R675 a reset is pure: the same state and action give the same state, and a JSON copy plays the same", () => {
    const start = game(seedFor("reset"));
    inHand(start, GLITCH_DEF_ID, "p1");
    const card = start.players.p1.hand.at(-1);
    const action: Action = { type: "play", playerId: "p1", instanceId: card?.id ?? "", nonce: "pure" };
    const a = reduce(cloneState(start), action).state;
    const b = reduce(JSON.parse(JSON.stringify(start)) as GameState, action).state;
    expect(hashState(a)).toBe(hashState(b));
  });
});

describe("R676 the seat swap", () => {
  it("R676 swaps which seat each account plays, and only that: an even number of swaps is none", () => {
    const { state } = playGlitch(game(seedFor("swap")));
    expect(state.seatSwaps).toBe(1);
    expect(seatsSwapped(state)).toBe(true);
    expect(seatPlayedBy(state, "p1")).toBe("p2");
    expect(seatPlayedBy(state, "p2")).toBe("p1");
    expect(seatPlayedBy({ seatSwaps: 2 }, "p1")).toBe("p1");
  });
});

describe("R677 other games' boards", () => {
  it("R677 replaces both fields with the frozen boards, Units to the unit zones and the rest to the backrow", () => {
    const seed = seedFor("boards");
    const state = game(seed);
    put(state, "fx-1", slot("p1", "units", 1));
    put(state, "fx-2", slot("p2", "units", 3));
    const frozen = createGame({
      seed,
      decks: [state.opening?.decks[0] ?? [], state.opening?.decks[1] ?? []],
      glitchBoards: [[{ defId: "fx-7", radiant: true }, { defId: "fx-8", radiant: false }], [{ defId: "not-a-card", radiant: false }]],
    });
    state.glitchBoards = frozen.glitchBoards;
    const { state: after } = playGlitch(state);
    const units = (player: "p1" | "p2") => slotsOf(player, "units").flatMap((ref) => cardAt(after, ref) ?? []);
    expect(units("p1").map((card) => [card.defId, card.radiant, card.owner])).toEqual([
      ["fx-7", true, "p1"],
      ["fx-8", false, "p1"],
    ]);
    // An entry this match cannot rebuild was dropped when it was frozen (R564), so p2's field is empty.
    expect(units("p2")).toEqual([]);
    expect(after.players.p1.graveyard.map((card) => card.defId)).toEqual([GLITCH_DEF_ID]);
  });
});

describe("R678 the void", () => {
  it("R678 ends the game with no winner and reason voided", () => {
    const { state, events } = playGlitch(game(seedFor("void")));
    expect(state.result).toEqual({ winner: "draw", reason: "voided" });
    expect(events.at(-1)).toEqual({ type: "gameOver", winner: "draw", reason: "voided" });
  });

  it("R678 the effect alone, from any context, ends the game the same way", () => {
    const state = game(seedFor("void"));
    const sink = sinkFor(state);
    glitch().apply(makeContext(sink, null, { controller: "p1" }));
    expect(state.result?.reason).toBe("voided");
    expect(findDef(null, GLITCH_DEF_ID)).toBe(GLITCH);
  });
});
