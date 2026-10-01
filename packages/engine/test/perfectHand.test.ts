// B5 E34, the perfect-hand scorer (`subsystems/perfectHand.ts`; SPEC §8.7 C+ #27, §10.7, R29, R364,
// R387, R416), through a fixture card of Classic+ #27's shape: "replace your hand with the perfect
// hand, then refresh your mana". The real card's test (packages/cards/test/classic-plus/
// 027-zephrys-zealotism.test.ts) covers the card again over the real catalog.
//
// The pool is pinned: this file registers seven Classic and Classic+ cards on top of the (Core)
// fixture catalog and nothing else of those sets, so each ranking is a statement about a fixed state
// and pool, never about the scorer's weights. Core #97's scorer is reused unchanged. Defs are `ph-`,
// indexed from 4901.

import type { Action, ActionInput, CardDef, CardFace, GameEvent, Keyword, SetName } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { registerCatalog, registeredCatalog } from "../src/catalog";
import { refreshMana } from "../src/effects/mana";
import { beginGame, reduce } from "../src/reduce";
import { hashState } from "../src/replay";
import type { CardScripts } from "../src/script";
import { registerScripts, registeredScripts } from "../src/scripts";
import { findInstance, type GameState } from "../src/state";
import { compareScored } from "../src/subsystems/scorer";
import { rankPerfectHand, replaceHandWithPerfect } from "../src/subsystems/perfectHand";
import { viewFor } from "../src/viewFor";
import { inHand, newGame } from "./fixtures/harness";

let nextIndex = 4900;

function face(text: string, extra: Partial<CardFace> = {}): CardFace {
  return { keywords: [], text, ...extra };
}

function def(name: string, type: CardDef["type"], set: SetName, extra: Partial<CardDef> = {}): CardDef {
  nextIndex += 1;
  return {
    id: `ph-${name}`,
    index: String(nextIndex),
    name: `PH ${name}`,
    set,
    type,
    tags: [],
    rarity: "Common",
    token: false,
    cost: 1,
    base: face(name),
    radiant: face(name),
    ...extra,
  };
}

function unit(name: string, set: SetName, attack: number, health: number, extra: Partial<CardDef> & { keywords?: Keyword[] } = {}): CardDef {
  const { keywords = [], ...rest } = extra;
  return def(name, "Unit", set, {
    base: face(name, { attack, health, keywords }),
    radiant: face(name, { attack: attack * 2, health: health * 2, keywords }),
    ...rest,
  });
}

/** C+ #27's shape. Classic+, so it would be in its own pool but for R387. */
const zealot = def("zealot", "Spell", "Classic+", { cost: 4 });
/** A 4/4 Charge for 1: the one card that enables lethal. */
const charger = unit("charger", "Classic+", 4, 4, { keywords: [{ kind: "Charge" }] });
/** The best stats per mana in the pool, 9/9 for 2. */
const bigBody = unit("big-body", "Classic", 9, 9, { cost: 2 });
/** Always tied; "ph-tie-a" sorts first by id but sits at a later index than "ph-tie-b" (index 1). */
const tieA = unit("tie-a", "Classic+", 3, 3, { index: "4999" });
const tieB = unit("tie-b", "Classic", 3, 3, { index: "1" });
/** A 0/1 whose Radiant face is a 30/30: low on the base face, first on the Radiant face. */
const sleeper = def("sleeper", "Unit", "Classic", {
  base: face("sleeper", { attack: 0, health: 1 }),
  radiant: face("sleeper radiant", { attack: 30, health: 30 }),
});
const quietSpell = def("quiet-spell", "Spell", "Classic", { cost: 0 });
const quietTrap = def("quiet-trap", "Trap", "Classic+");
/** Never in the pool: a Core card and a Classic+ token, each better than anything above. */
const coreGiant = unit("core-giant", "Core", 20, 20);
const plusToken = unit("plus-token", "Classic+", 20, 20, { token: true, tags: ["Token"], rarity: "Token" });

const POOL = [charger, bigBody, tieA, tieB, sleeper, quietSpell, quietTrap];
const DEFS = [zealot, ...POOL, coreGiant, plusToken];

const ALL_MANA = Number.POSITIVE_INFINITY;
const SCRIPTS: Record<string, CardScripts> = {
  [zealot.id]: {
    base: { cry: () => [replaceHandWithPerfect(), refreshMana({ amount: ALL_MANA })] },
    radiant: { cry: () => [replaceHandWithPerfect({ radiant: true }), refreshMana({ amount: ALL_MANA })] },
  },
};

let nonce = 0;
function act(state: GameState, body: ActionInput): { state: GameState; events: GameEvent[] } {
  nonce += 1;
  const result = reduce(state, { ...body, nonce: `ph${nonce}` } as Action);
  if (result.error !== undefined) throw new Error(result.error);
  return result;
}

/** p1's main phase, both hands empty, 4 of 4 mana, this file's cards registered. */
function playing(seed: string): GameState {
  let state = beginGame(newGame(seed)).state;
  for (const playerId of ["p1", "p2"] as const) {
    state = act(state, { type: "mulligan", keep: state.players[playerId].hand.map((c) => c.id), playerId }).state;
  }
  registerCatalog({ ...registeredCatalog(), ...Object.fromEntries(DEFS.map((d) => [d.id, d])) });
  registerScripts({ ...registeredScripts(), ...SCRIPTS });
  for (const playerId of ["p1", "p2"] as const) state.players[playerId].hand = [];
  state.players.p1.mana = { ...state.players.p1.mana, current: 4, max: 4 };
  return state;
}

/** p1 holds the zealot and `others`; returns the state and the zealot's id. */
function holding(seed: string, others: string[], options: { radiant?: boolean; mana?: number } = {}) {
  const state = playing(seed);
  const [card] = inHand(state, zealot.id, "p1");
  if (card === undefined) throw new Error("the zealot");
  card.radiant = options.radiant === true;
  const held = others.flatMap((defId) => inHand(state, defId, "p1"));
  if (options.mana !== undefined) state.players.p1.mana.current = options.mana;
  return { state, zealotId: card.id, held };
}

const cast = (state: GameState, instanceId: string) => act(state, { type: "play", instanceId, playerId: "p1" });
const handIds = (state: GameState): string[] => state.players.p1.hand.map((card) => card.defId);
const rankIds = (state: GameState, radiant = false): string[] =>
  rankPerfectHand(state, "p1", { selfDefId: zealot.id, radiant }).map((scored) => scored.def.id);

describe("E34 the perfect-hand ranking (R29, R387, R416)", () => {
  it("R416 ranks every non-token Classic and Classic+ card but the asking card: no Core card, no token", () => {
    const { state } = holding("ph-pool", []);
    expect([...rankIds(state)].sort()).toEqual(POOL.map((d) => d.id).sort());
    // Asked by nobody in particular, the zealot is a Classic+ card like any other.
    expect(rankPerfectHand(state, "p1").map((s) => s.def.id)).toContain(zealot.id);
  });

  it("R387 a fused asking card leaves out every one of its ingredients", () => {
    const { state } = holding("ph-fused", []);
    const ranked = rankPerfectHand(state, "p1", { selfDefId: `t-1:${bigBody.id}+${zealot.id}` }).map((s) => s.def.id);
    expect(ranked).not.toContain(zealot.id);
    expect(ranked).not.toContain(bigBody.id);
    expect(ranked).toContain(charger.id);
  });

  it("R416 ties go by card id, never by index (Core #97's order would put tie-b first)", () => {
    const { state } = holding("ph-ties", []);
    const ranked = rankPerfectHand(state, "p1", { selfDefId: zealot.id });
    const a = ranked.find((s) => s.def.id === tieA.id);
    const b = ranked.find((s) => s.def.id === tieB.id);
    if (a === undefined || b === undefined) throw new Error("both tied cards rank");
    expect(a.score).toBe(b.score);
    const order = ranked.map((s) => s.def.id);
    expect(order.indexOf(tieA.id)).toBe(order.indexOf(tieB.id) - 1);
    expect(compareScored(b, a)).toBeLessThan(0);
  });

  it("R29 the card that enables lethal ranks first when lethal exists, and only then", () => {
    const { state } = holding("ph-lethal", []);
    state.players.p2.hero.health = 4;
    const lethal = rankPerfectHand(state, "p1", { selfDefId: zealot.id });
    expect(lethal[0]?.def.id).toBe(charger.id);
    expect(lethal[0]?.priority).toBe("lethal");
    state.players.p2.hero.health = 30;
    const value = rankPerfectHand(state, "p1", { selfDefId: zealot.id });
    expect(value[0]?.def.id).toBe(bigBody.id);
    expect(value.every((s) => s.priority !== "lethal")).toBe(true);
  });

  it("R416 the Radiant face scores each candidate on its Radiant face", () => {
    const { state } = holding("ph-radiant-rank", []);
    expect(rankIds(state).indexOf(sleeper.id)).toBeGreaterThan(2);
    expect(rankIds(state, true)[0]).toBe(sleeper.id);
  });

  it("§10.7 the same state ranks the same, a JSON copy included, and ranking draws nothing", () => {
    const { state } = holding("ph-determinism", []);
    const cursor = state.rngCursor;
    const first = rankIds(state);
    expect(rankIds(JSON.parse(JSON.stringify(state)) as GameState)).toEqual(first);
    expect(state.rngCursor).toBe(cursor);
  });
});

describe("E34 replaceHandWithPerfect, then a Refresh (C+ #27's shape)", () => {
  it("R416 the hand keeps its size: each card goes to the graveyard, not a discard, and the top N arrive in rank order", () => {
    const { state, zealotId, held } = holding("ph-replace", [coreGiant.id, coreGiant.id, quietSpell.id]);
    const expected = rankIds(state).slice(0, 3);
    const after = cast(state, zealotId);
    expect(handIds(after.state)).toEqual(expected);
    for (const old of held) expect(findInstance(after.state, old.id)?.zone).toEqual({ z: "graveyard", player: "p1" });
    expect(after.events.some((e) => e.type === "discarded")).toBe(false);
    expect(after.events.filter((e) => e.type === "enteredGraveyard").map((e) => ("defId" in e ? e.defId : ""))).toEqual(
      expect.arrayContaining([coreGiant.id, quietSpell.id]),
    );
    for (const fresh of after.state.players.p1.hand) {
      expect(fresh.owner).toBe("p1");
      expect(fresh.radiant).toBe(false);
      expect(fresh.costOverride).toBeUndefined();
      expect(fresh.costMod).toBe(0);
    }
  });

  it("R416 with no other card in hand nothing arrives, and the Refresh still gives back the 4", () => {
    const { state, zealotId } = holding("ph-alone", []);
    const after = cast(state, zealotId).state;
    expect(after.players.p1.hand).toHaveLength(0);
    expect(after.players.p1.mana.current).toBe(4);
  });

  it("R11 a unit-token card in the replaced hand ceases to exist instead of reaching the graveyard", () => {
    const { state, zealotId, held } = holding("ph-token", [plusToken.id, quietSpell.id]);
    const after = cast(state, zealotId).state;
    const [token, spell] = held;
    expect(findInstance(after, token?.id ?? "")).toBeUndefined();
    expect(after.players.p1.graveyard.map((c) => c.id)).toContain(spell?.id);
    expect(after.players.p1.hand).toHaveLength(2);
  });

  it("R29 with lethal on the board the lethal-enabling card arrives first", () => {
    const { state, zealotId } = holding("ph-lethal-hand", [quietSpell.id, quietSpell.id], { mana: 8 });
    state.players.p2.hero.health = 4;
    expect(handIds(cast(state, zealotId).state)[0]).toBe(charger.id);
  });

  it("R416 the Radiant face: the perfect Radiant hand arrives Radiant", () => {
    const { state, zealotId } = holding("ph-radiant", [quietSpell.id, quietTrap.id], { radiant: true });
    const after = cast(state, zealotId).state;
    expect(handIds(after)[0]).toBe(sleeper.id);
    expect(after.players.p1.hand.every((card) => card.radiant)).toBe(true);
  });

  it("R364 the Refresh gives back up to max mana and never past it", () => {
    const spent = holding("ph-refresh", [quietSpell.id]);
    expect(cast(spent.state, spent.zealotId).state.players.p1.mana.current).toBe(4);
    const above = holding("ph-refresh-above", [quietSpell.id], { mana: 9 });
    expect(cast(above.state, above.zealotId).state.players.p1.mana.current).toBe(5);
  });

  it("§9.3 the play draws nothing from the match rng and a JSON copy replays to the same hash and events", () => {
    const { state, zealotId } = holding("ph-replay", [quietSpell.id, coreGiant.id]);
    const thawed = JSON.parse(JSON.stringify(state)) as GameState;
    const action = { type: "play", instanceId: zealotId, playerId: "p1", nonce: "ph-replay" } as Action;
    const live = reduce(state, action);
    const again = reduce(thawed, action);
    expect(live.state.rngCursor).toBe(state.rngCursor);
    expect(hashState(again.state)).toBe(hashState(live.state));
    expect(again.events).toEqual(live.events);
  });

  it("R97 the opponent sees the replaced cards reach the graveyard and never the new cards", () => {
    const { state, zealotId } = holding("ph-hidden", [quietSpell.id, coreGiant.id]);
    const after = cast(state, zealotId).state;
    const theirs = viewFor(after, "p2");
    const added = theirs.events.filter((e) => e.type === "addedToHand");
    expect(added.length).toBeGreaterThanOrEqual(2);
    for (const event of added) expect(JSON.stringify(event)).not.toMatch(/ph-(charger|big-body|tie|sleeper|quiet)/);
    const landed = theirs.events.filter((e) => e.type === "enteredGraveyard").map((e) => ("defId" in e ? e.defId : ""));
    expect(landed).toEqual(expect.arrayContaining([quietSpell.id, coreGiant.id]));
  });
});
