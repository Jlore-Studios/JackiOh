// C+ #35 Rollback — SPEC §8.7 row 35, BUILD M9 "C+ 35", R419, and this card's rulings R562 (too little
// history), R563 (held zones) and R566 (what a put-back card keeps of the present). The history is
// E29's (`packages/engine/src/subsystems/boardHistory.ts`, engine-tested in `boardHistory.test.ts`).
//
// The harness starts mid-game on turn 9 without running a turn start, so a test's history begins with
// its first `endTurn()`: two of them reach p1's turn 11 holding the snapshots of turns 10 and 11.
// Libraries hold Mr. Vanilla fillers for the draws, and each side keeps a Unit, so no turn auto-ends
// (R82). Props: #8 Mr. Vanilla, #11 Tempo Timmy, #15 Me and Mr Token (a Cry that would show), #16 Hit
// Job, #17 Flood, #34 Collateral Damage, #36 Magic Jammed, #41 Sheepish (a face-down Trap), #43 Big
// Felinor under #92 Felinor Fiender (a Stack pile), #3 Right-house defender (Reborn), #49 Snom Bunny Mind
// Control, #66 The Rock (Radiant: Indestructible, Immutable), #83 Transmogulate, #85 Unlicensed
// Experimentation, the Rush Token and C+ #12.8 Frostspatula ("Animated on your turn").

import { describe, expect, it } from "vitest";
import type { Action, ActionInput, GameEvent, PlayerId } from "@jackioh/shared";
import {
  BOARD_HISTORY_DEPTH,
  beginGame,
  createGame,
  createRng,
  fold,
  hashState,
  legalActions,
  reduce,
  seatToAct,
  subsystems,
  type GameState,
} from "@jackioh/engine";
import { scenario, type Scenario, type ScenarioOptions } from "../_harness";
import { createInvariantMonitor } from "../_invariants";
import { base, def, radiant } from "../../src/scripts/classic-plus/035-rollback";

const ROLLBACK = "classicplus-035";
const RADIANT_ROLLBACK = { def: ROLLBACK, radiant: true } as const;
const VANILLA = "core-008";
const TIMMY = "core-011";
const TOKEN_MAKER = "core-015";
const HIT_JOB = "core-016";
const FLOOD = "core-017";
const COLLATERAL = "core-034";
const MAGIC_JAMMED = "core-036";
const SHEEPISH = "core-041";
const BIG_FELINOR = "core-043";
const FIENDER = "core-092";
const DEFENDER = "core-003";
const MIND_CONTROL = "core-049";
const ROCK = "core-066";
const TRANSMOGULATE = "core-083";
const UNLICENSED = "core-085";
const RUSH_TOKEN = "core-t-rush";
const SPATULA = "classicplus-012-8";
const FILLER = Array.from({ length: 8 }, () => VANILLA);

/** The board at p1's turn 11, with the snapshots of turns 10 and 11 recorded. */
function onTurn11(opts: ScenarioOptions): Scenario {
  const s = scenario({
    ...opts,
    p1: { library: FILLER, ...opts.p1 },
    p2: { library: FILLER, ...opts.p2 },
  });
  return s.endTurn().endTurn();
}

/** Two more turns: p1's turn 13, the snapshots of turns 10 to 13 recorded. */
function toTurn13(s: Scenario): Scenario {
  s.endTurn().endTurn();
  expect(s.state.turn).toBe(13);
  return s;
}

function ofType<T extends GameEvent["type"]>(events: readonly GameEvent[], type: T): Extract<GameEvent, { type: T }>[] {
  return events.filter((event): event is Extract<GameEvent, { type: T }> => event.type === type);
}

const target = (instanceId: string) => [{ pick: "instance" as const, instanceId }];

describe("C+ #35 Rollback — the script", () => {
  it("def is the catalog's, and N is declared with the play (R81): 1, 2 or 3; the Radiant face adds the side", () => {
    expect(def.id).toBe(ROLLBACK);
    expect(base.modes).toEqual([{ kind: "number", options: ["1", "2", "3"] }]);
    expect(radiant.modes).toEqual([
      { kind: "number", options: ["1", "2", "3"] },
      { kind: "mode", options: ["your side", "your opponent's side", "both sides"] },
    ]);
  });

  it("R81 legalActions offers one play per N on the base face and per N and side on the Radiant face", () => {
    const s = scenario({ p1: { hand: [ROLLBACK, RADIANT_ROLLBACK] } });
    const [plain, shiny] = s.hand("p1");
    const plays = (id: string | undefined) =>
      legalActions(s.state, "p1").flatMap((action) => (action.type === "play" && action.instanceId === id ? [action.modes] : []));
    expect(plays(plain?.id)).toEqual([["1"], ["2"], ["3"]]);
    expect(plays(shiny?.id)).toHaveLength(9);
    expect(plays(shiny?.id)).toContainEqual(["2", "your opponent's side"]);
  });
});

describe("C+ #35 Rollback — the history (R419)", () => {
  it("R419 each turn's start records both sides' piles (dormant cards too), backrow zones and Locks before anything else, and keeps BOARD_HISTORY_DEPTH", () => {
    const s = scenario({
      p1: { field: [BIG_FELINOR, { def: FIENDER, stack: true }], backrow: [SHEEPISH, SPATULA], library: FILLER },
      p2: { field: [TIMMY], backrow: [SHEEPISH], library: FILLER },
    });
    expect(s.state.boardHistory).toBeUndefined();
    s.endTurn().endTurn();
    const history = () => s.state.boardHistory ?? [];
    expect(history().map((snapshot) => snapshot.turn)).toEqual([10, 11]);
    const mine = history()[1]?.sides.p1;
    // The pile top first, its dormant Big Felinor included, as whole instances.
    expect(mine?.units[0]?.map((card) => card.defId)).toEqual([FIENDER, BIG_FELINOR]);
    expect(mine?.units[0]?.[1]).toEqual(s.state.players.p1.units[0]?.[1]);
    expect(mine?.backrow[0]?.defId).toBe(SHEEPISH);
    expect(history()[1]?.sides.p2.backrow[0]?.defId).toBe(SHEEPISH);
    expect(mine?.locks).toEqual({ units: [false, false, false, false, false], backrow: [false, false, false, false, false] });
    // Frostspatula animated at this turn's start, after the refresh: the snapshot came first.
    expect(mine?.backrow[1]?.defId).toBe(SPATULA);
    expect(s.unit("p1", 2)?.defId).toBe(SPATULA);

    s.endTurn().endTurn().endTurn().endTurn();
    expect(BOARD_HISTORY_DEPTH).toBe(4);
    expect(history().map((snapshot) => snapshot.turn)).toEqual([12, 13, 14, 15]);
  });

  it("R419 viewFor never carries a snapshot, nor the face-down card it holds", () => {
    const s = onTurn11({ p1: { hand: [ROLLBACK], field: [VANILLA], backrow: [SHEEPISH] }, p2: { field: [TIMMY] } });
    expect(s.state.boardHistory?.[0]?.sides.p1.backrow[0]?.defId).toBe(SHEEPISH);
    for (const viewer of ["p1", "p2"] as PlayerId[]) expect(JSON.stringify(s.view(viewer))).not.toContain("boardHistory");
    expect(JSON.stringify(s.view("p2"))).not.toContain(SHEEPISH);
  });

  it("R419 the history survives JSON: a round-tripped state hashes the same and rolls back the same", () => {
    const s = onTurn11({ p1: { hand: [ROLLBACK, HIT_JOB], field: [VANILLA] }, p2: { field: [TIMMY] } });
    s.play(HIT_JOB, { targets: target(s.unit("p2", 1)?.id ?? "") });
    toTurn13(s);
    const copy = JSON.parse(JSON.stringify(s.state)) as GameState;
    expect(hashState(copy)).toBe(hashState(s.state));
    const rollback = s.hand("p1").find((card) => card.defId === ROLLBACK)?.id ?? "";
    const action = { type: "play", playerId: "p1", instanceId: rollback, modes: ["2"], nonce: "json" } as Action;
    const live = reduce(s.state, action);
    const again = reduce(copy, action);
    expect(live.error).toBeUndefined();
    expect(hashState(again.state)).toBe(hashState(live.state));
    expect(again.events).toEqual(live.events);
  });
});

describe("C+ #35 Rollback — base: both sides", () => {
  it("R419 N names the snapshot of the start of the player-turn N before this one", () => {
    const run = (n: string): Scenario => {
      const s = scenario({
        p1: { hand: [ROLLBACK, TIMMY], field: [VANILLA], library: FILLER },
        p2: { hand: [TIMMY, TOKEN_MAKER], field: [VANILLA], library: FILLER },
      });
      s.endTurn(); // turn 10, p2's: snapshot, then p2 plays a Tempo Timmy
      s.play(TIMMY, { zone: 2 });
      s.endTurn(); // turn 11, p1's: snapshot, then p1 plays one
      s.play(TIMMY, { zone: 2 });
      s.endTurn(); // turn 12, p2's: snapshot, then Me and Mr Token
      s.play(TOKEN_MAKER, { zone: 3 });
      s.endTurn(); // turn 13
      s.play(ROLLBACK, { modes: [n] });
      return s;
    };
    const lanes = (s: Scenario, player: PlayerId) => [1, 2, 3, 4].map((lane) => s.unit(player, lane)?.defId ?? null);
    // 1: the start of turn 12 — both Tempo Timmys, no Me and Mr Token yet.
    const one = run("1");
    expect(lanes(one, "p1")).toEqual([VANILLA, TIMMY, null, null]);
    expect(lanes(one, "p2")).toEqual([VANILLA, TIMMY, null, null]);
    // 2: the start of turn 11 — p2's Timmy, not p1's.
    const two = run("2");
    expect(lanes(two, "p1")).toEqual([VANILLA, null, null, null]);
    expect(lanes(two, "p2")).toEqual([VANILLA, TIMMY, null, null]);
    // 3: the start of turn 10 — neither.
    const three = run("3");
    expect(lanes(three, "p1")).toEqual([VANILLA, null, null, null]);
    expect(lanes(three, "p2")).toEqual([VANILLA, null, null, null]);
    expect(ofType(three.lastEvents, "rolledBack")).toEqual([{ type: "rolledBack", player: "p1", turnsAgo: 3, sides: ["p1", "p2"] }]);
  });

  it("R419 step 1: a card the snapshot lacks goes to its owner's hand, reset (R78); a unit token ceases to exist (R11); no Death", () => {
    const s = onTurn11({
      p1: { hand: [ROLLBACK, TOKEN_MAKER, { def: DEFENDER, radiant: true }], field: [VANILLA] },
      p2: { field: [TIMMY] },
    });
    s.play(TOKEN_MAKER, { zone: 2 });
    s.play({ ...s.hand("p1").find((card) => card.defId === DEFENDER)! }, { zone: 4 });
    const maker = s.unit("p1", 2)!;
    const token = s.unit("p1", 3)!;
    const defender = s.unit("p1", 4)!;
    expect(token.defId).toBe(RUSH_TOKEN);
    s.endTurn(); // p2's Timmy hits the defender's Divine Shield away
    s.attack(s.unit("p2", 1)!, defender);
    s.endTurn();

    s.play(ROLLBACK, { modes: ["2"] });
    s.expectInZone(maker, "hand").expectInZone(defender, "hand").expectInZone(token, "gone");
    expect(s.card(defender).divineShieldSpent).toBeUndefined();
    expect(s.card(defender).position).toBeUndefined();
    s.expectEvents("rolledBack", "bounced", "bounced", "bounced");
    // A move to a hand is no death: the Radiant defender's Death summons nothing, and none dies.
    expect(ofType(s.lastEvents, "destroyed")).toEqual([]);
    expect(ofType(s.lastEvents, "summoned")).toEqual([]);
    expect([1, 2, 3, 4, 5].map((lane) => s.unit("p1", lane)?.defId ?? null)).toEqual([VANILLA, null, null, null, null]);
  });

  it("R419 step 1: a full hand burns the card (§2.4, R4)", () => {
    const s = scenario({
      p1: { hand: [ROLLBACK], field: [VANILLA], library: FILLER },
      p2: { hand: Array.from({ length: 10 }, () => TIMMY), field: [VANILLA], library: FILLER },
    });
    s.endTurn(); // turn 10: p2's draw burns at 10; p2 plays two Timmys
    s.play(TIMMY, { zone: 2 }).play(TIMMY, { zone: 3 });
    s.endTurn().endTurn(); // p2's draw on turn 12 brings it to 9
    s.endTurn();
    expect(s.hand("p2")).toHaveLength(9);
    const [first, second] = [s.unit("p2", 2)!, s.unit("p2", 3)!];

    s.play(ROLLBACK, { modes: ["3"] });
    s.expectInZone(first, "hand").expectInZone(second, "graveyard");
    expect(ofType(s.lastEvents, "burned").map((event) => event.instanceId)).toEqual([second.id]);
    expect(s.hand("p2")).toHaveLength(10);
  });

  it("R419 step 2: a snapshot card goes back exactly as it was — damage, buffs, position, counters — from the field, its pile rebuilt", () => {
    const s = onTurn11({
      p1: {
        hand: [ROLLBACK, HIT_JOB],
        field: [BIG_FELINOR, { def: FIENDER, stack: true }, { def: VANILLA, damage: 1, position: "DEF", counters: { plague: 2 } }],
      },
      p2: { field: [TIMMY] },
    });
    const fiender = s.unit("p1", 1)!;
    const felinor = s.state.players.p1.units[0]?.[1];
    const vanilla = s.unit("p1", 2)!;
    s.play(HIT_JOB, { targets: target(fiender.id) }); // the pile's top dies; Big Felinor resumes
    s.switchPosition(vanilla);
    expect(s.unit("p1", 1)?.id).toBe(felinor?.id);
    s.endTurn();
    s.attack(s.unit("p2", 1)!, s.unit("p1", 2)!);
    s.endTurn();

    s.play(ROLLBACK, { modes: ["2"] });
    expect(s.state.players.p1.units[0]?.map((card) => card.id)).toEqual([fiender.id, felinor?.id]);
    expect(s.card(vanilla)).toMatchObject({ damage: 1, position: "DEF", counters: { plague: 2 }, buffs: { attack: 0, health: 0 } });
    expect(s.state.players.p1.graveyard.map((card) => card.id)).not.toContain(fiender.id);
  });

  it("R419 step 2: from a hand, a graveyard and exile — no Cry for a card that comes back (R1)", () => {
    const s = onTurn11({
      p1: { hand: [ROLLBACK, HIT_JOB], field: [TOKEN_MAKER, { def: VANILLA, lane: 4 }] },
      p2: { hand: [COLLATERAL, FLOOD], field: [TIMMY, VANILLA] },
    });
    const maker = s.unit("p1", 1)!;
    const vanilla = s.unit("p1", 4)!;
    const timmy = s.unit("p2", 1)!;
    const theirs = s.unit("p2", 2)!;
    s.play(HIT_JOB, { targets: target(timmy.id) }); // to p2's graveyard
    s.endTurn();
    s.play(COLLATERAL, { targets: target(vanilla.id) }); // to p1's exile, nothing beside it
    s.endTurn().endTurn();
    s.play(FLOOD); // p2's turn 14: every Unit to its owner's hand, and the turn has nothing left (R82)
    expect(s.state.turn).toBe(15);
    s.expectInZone(maker, "hand").expectInZone(timmy, "graveyard").expectInZone(vanilla, "exile").expectInZone(theirs, "hand");

    s.play(ROLLBACK, { modes: ["3"] }); // turn 15 → the start of turn 12, before the exile and the Flood
    expect(s.unit("p1", 1)?.id).toBe(maker.id);
    expect(s.unit("p1", 4)?.id).toBe(vanilla.id);
    expect(s.unit("p2", 2)?.id).toBe(theirs.id);
    // Timmy died on turn 11, before the snapshot of turn 12: it stays where it is.
    s.expectInZone(timmy, "graveyard");
    expect(ofType(s.lastEvents, "summoned")).toEqual([]);
    expect(s.state.players.p1.units.flat().filter((card) => card?.defId === RUSH_TOKEN)).toEqual([]);
    expect(ofType(s.lastEvents, "controlChanged").map((event) => event.instanceId)).toEqual([maker.id, vanilla.id, theirs.id]);
  });

  it("R419 step 2: a stolen card goes back to its side, under its owner's control (R12, R171)", () => {
    const s = onTurn11({ p1: { hand: [ROLLBACK, MIND_CONTROL], field: [VANILLA] }, p2: { field: [VANILLA, TIMMY] } });
    const timmy = s.unit("p2", 2)!;
    s.play(MIND_CONTROL, { targets: target(timmy.id) });
    expect(s.unit("p1", 2)?.id).toBe(timmy.id);
    toTurn13(s);

    s.play(ROLLBACK, { modes: ["2"] });
    expect(s.unit("p2", 2)?.id).toBe(timmy.id);
    expect(s.unit("p1", 2)).toBeNull();
    expect(s.card(timmy)).toMatchObject({ controller: "p2", owner: "p2", summonedTurn: 13 });
    expect(ofType(s.lastEvents, "controlChanged")).toEqual([
      { type: "controlChanged", instanceId: timmy.id, controller: "p2", row: "units", lane: 2 },
    ]);
  });

  it("R419 step 2: a card that left for the opponent's hand comes back out of it", () => {
    const s = onTurn11({ p1: { hand: [ROLLBACK, MIND_CONTROL], field: [VANILLA] }, p2: { hand: [FLOOD], field: [TIMMY] } });
    const vanilla = s.unit("p1", 1)!;
    const timmy = s.unit("p2", 1)!;
    s.play(MIND_CONTROL, { targets: target(timmy.id) });
    s.endTurn(); // turn 12's snapshot holds Timmy on p1's side
    s.play(FLOOD); // back to its owner, p2; the turn has nothing left (R82)
    s.expectInZone(timmy, "hand");
    expect(s.hand("p2").map((card) => card.id)).toContain(timmy.id);
    expect(s.state.turn).toBe(13);

    s.play(ROLLBACK, { modes: ["1"] });
    expect(s.unit("p1", 1)?.id).toBe(vanilla.id);
    expect(s.unit("p1", 2)?.id).toBe(timmy.id);
    expect(s.hand("p2").map((card) => card.id)).not.toContain(timmy.id);
    expect(s.card(timmy)).toMatchObject({ controller: "p1", owner: "p2" });
    // It is public on the field again, so both views name it.
    expect(ofType(s.view("p2").events, "controlChanged").map((event) => event.instanceId)).toContain(timmy.id);
  });

  it("R419 step 2: a card transformed since is recreated as it was, and what replaced it leaves (R35)", () => {
    const s = onTurn11({ p1: { hand: [ROLLBACK], field: [VANILLA] }, p2: { hand: [TRANSMOGULATE], field: [TIMMY] } });
    const timmy = s.unit("p2", 1)!;
    s.endTurn();
    s.play(TRANSMOGULATE);
    const legend = s.unit("p2", 1)!;
    expect(legend.id).not.toBe(timmy.id);
    s.expectInZone(timmy, "gone");
    s.endTurn();

    s.play(ROLLBACK, { modes: ["1"] });
    expect(s.unit("p2", 1)).toMatchObject({ id: timmy.id, defId: TIMMY, controller: "p2" });
    expect(s.hand("p2").map((card) => card.id)).toContain(legend.id);
  });

  it("R419 step 2: a token that ceased to exist is recreated from the snapshot (R11, R175)", () => {
    const s = onTurn11({ p1: { hand: [ROLLBACK, HIT_JOB], field: [VANILLA, RUSH_TOKEN] }, p2: { field: [TIMMY] } });
    const token = s.unit("p1", 2)!;
    s.play(HIT_JOB, { targets: target(token.id) });
    s.expectInZone(token, "gone");
    toTurn13(s);

    s.play(ROLLBACK, { modes: ["2"] });
    expect(s.unit("p1", 2)).toMatchObject({ id: token.id, defId: RUSH_TOKEN, summonedTurn: 13 });
    s.expectStats(token, { attack: 3, health: 3 });
  });

  it("R419 step 2: a card fused since goes back to its own definition; a fired Trap goes back face-down (R77)", () => {
    const s = onTurn11({
      p1: { hand: [ROLLBACK], field: [TIMMY], backrow: [{ def: UNLICENSED, lane: 3 }] },
      p2: { hand: [VANILLA], field: [TIMMY] },
    });
    const mine = s.unit("p1", 1)!;
    const trap = s.backrow("p1", 3)!;
    s.endTurn();
    s.play(VANILLA, { zone: 2 }); // the Trap fuses it onto p1's Timmy
    expect(s.card(mine).defId).not.toBe(TIMMY);
    s.expectInZone(trap, "graveyard");
    s.endTurn();

    s.play(ROLLBACK, { modes: ["1"] });
    expect(s.card(mine).defId).toBe(TIMMY);
    s.expectStats(mine, { attack: 3, health: 3 });
    const back = s.backrow("p1", 3)!;
    expect(back.defId).toBe(UNLICENSED);
    expect(back.faceUp).not.toBe(true);
    expect(s.state.players.p1.graveyard.map((card) => card.defId)).not.toContain(UNLICENSED);
  });

  it("R419 step 3: the Locks become the snapshot's; a card put back face-down shows the opponent only a face-down card, under a fresh id (R33, R97, R227)", () => {
    const s = onTurn11({ p1: { hand: [ROLLBACK, MAGIC_JAMMED], field: [VANILLA] }, p2: { field: [TIMMY], backrow: [SHEEPISH] } });
    const trap = s.backrow("p2", 1)!;
    s.play(MAGIC_JAMMED, { targets: target(trap.id) });
    s.expectInZone(trap, "graveyard");
    expect(s.state.players.p2.locks.backrow[0]).toBe(true);
    toTurn13(s);

    s.play(ROLLBACK, { modes: ["2"] });
    const back = s.backrow("p2", 1)!;
    expect(back.defId).toBe(SHEEPISH);
    expect(back.id).not.toBe(trap.id);
    expect(s.state.players.p2.locks.backrow[0]).toBe(false);
    expect(ofType(s.lastEvents, "unlocked")).toEqual([{ type: "unlocked", player: "p2", row: "backrow", lane: 1 }]);
    // p1 sees a face-down card in that zone and nothing that names it.
    expect(s.view("p1").opponent.backrow[0]).toMatchObject({ faceDown: true });
    expect(JSON.stringify(s.view("p1").opponent)).not.toContain(SHEEPISH);
    const seen = ofType(s.view("p1").events, "controlChanged").find((event) => event.row === "backrow");
    expect(seen).toEqual({ type: "controlChanged", instanceId: "hidden", controller: "p2", row: "backrow", lane: 1 });
    // Its controller reads it, and finds the card it was by `formerId` (R227).
    const own = ofType(s.view("p2").events, "controlChanged").find((event) => event.row === "backrow");
    expect(own).toEqual({ type: "controlChanged", instanceId: back.id, controller: "p2", row: "backrow", lane: 1, formerId: trap.id });
  });

  it("R419 health, mana, decks and graveyards change only by the cards that moved; one rolledBack, then the moves", () => {
    const s = onTurn11({ p1: { hand: [ROLLBACK, HIT_JOB], field: [VANILLA] }, p2: { field: [TIMMY, VANILLA] } });
    const timmy = s.unit("p2", 1)!;
    s.play(HIT_JOB, { targets: target(timmy.id) });
    s.endTurn();
    s.attack(s.unit("p2", 2)!, "hero");
    s.endTurn();
    const before = {
      health: [s.state.players.p1.hero.health, s.state.players.p2.hero.health],
      libraries: [s.state.players.p1.library.length, s.state.players.p2.library.length],
      graveyards: [s.state.players.p1.graveyard.map((card) => card.id), s.state.players.p2.graveyard.map((card) => card.id)],
    };

    s.play(ROLLBACK, { modes: ["2"] });
    expect([s.state.players.p1.hero.health, s.state.players.p2.hero.health]).toEqual(before.health);
    expect([s.state.players.p1.library.length, s.state.players.p2.library.length]).toEqual(before.libraries);
    s.expectMana("p1", 0);
    expect(s.state.players.p1.graveyard.map((card) => card.id)).toEqual([...(before.graveyards[0] ?? []), s.card(ROLLBACK).id]);
    expect(s.state.players.p2.graveyard.map((card) => card.id)).toEqual((before.graveyards[1] ?? []).filter((id) => id !== timmy.id));
    expect(ofType(s.lastEvents, "rolledBack")).toEqual([{ type: "rolledBack", player: "p1", turnsAgo: 2, sides: ["p1", "p2"] }]);
    const types = s.lastEvents.map((event) => event.type);
    expect(types.indexOf("rolledBack")).toBeLessThan(types.indexOf("controlChanged"));
    expect(types).not.toContain("destroyed");
  });

  it("R419 Indestructible and Immutable change nothing: a move is no destroy and no Transform", () => {
    const s = onTurn11({ p1: { hand: [ROLLBACK], field: [VANILLA] }, p2: { hand: [{ def: ROCK, radiant: true }], field: [TIMMY] } });
    const timmy = s.unit("p2", 1)!;
    s.endTurn();
    s.play(ROCK, { zone: 2, tributes: [timmy.id] });
    const rock = s.unit("p2", 2)!;
    s.endTurn();

    s.play(ROLLBACK, { modes: ["1"] });
    s.expectInZone(rock, "hand");
    expect(s.unit("p2", 1)?.id).toBe(timmy.id);
    expect(ofType(s.lastEvents, "destroyed")).toEqual([]);
  });

  it("R419 Reborn: a unit that came back through Reborn goes back to the snapshot's instance, Reborn and Divine Shield again", () => {
    const s = onTurn11({ p1: { hand: [ROLLBACK, HIT_JOB], field: [VANILLA, DEFENDER] }, p2: { field: [TIMMY] } });
    const defender = s.unit("p1", 2)!;
    s.play(HIT_JOB, { targets: target(defender.id) });
    expect(s.card(defender).rebornSpent).toBe(true);
    toTurn13(s);

    s.play(ROLLBACK, { modes: ["2"] });
    expect(s.unit("p1", 2)?.id).toBe(defender.id);
    expect(s.card(defender).rebornSpent).toBeUndefined();
    expect(s.stats(defender).keywords.map((keyword) => keyword.kind)).toEqual(expect.arrayContaining(["Reborn", "Divine Shield"]));
    s.expectStats(defender, { health: 1, maxHealth: 1 });
  });
});

describe("C+ #35 Rollback — what a put-back card keeps (R566)", () => {
  it("R566 a card that stood on its side keeps its exertion and sickness; one from anywhere else entered this turn", () => {
    const s = onTurn11({ p1: { hand: [ROLLBACK, HIT_JOB], field: [VANILLA, TIMMY] }, p2: { field: [VANILLA] } });
    const timmy = s.unit("p1", 2)!;
    s.play(HIT_JOB, { targets: target(timmy.id) });
    toTurn13(s);
    const vanilla = s.unit("p1", 1)!;
    s.attack(vanilla, "hero");

    s.play(ROLLBACK, { modes: ["2"] });
    expect(s.card(vanilla).exertion.attacked).toBe(true);
    expect(s.card(vanilla).summonedTurn).toBeUndefined();
    expect(s.card(timmy).summonedTurn).toBe(13);
    const attackers = legalActions(s.state, "p1").flatMap((action) => (action.type === "attack" ? [action.attackerId] : []));
    expect(attackers).not.toContain(vanilla.id);
    // Tempo Timmy has Rush: back on the field this turn, it may attack a Unit, never the hero (§6.1).
    expect(legalActions(s.state, "p1").some((action) => action.type === "attack" && action.attackerId === timmy.id && action.targetId === "hero-p2")).toBe(false);
  });
});

describe("C+ #35 Rollback — too little history (R562)", () => {
  it("R562 with fewer turns recorded than N it goes back as far as the history goes", () => {
    const s = scenario({
      p1: { hand: [ROLLBACK], field: [VANILLA], library: FILLER },
      p2: { hand: [TIMMY], field: [VANILLA], library: FILLER },
    });
    s.endTurn(); // turn 10, p2's: the first snapshot, then p2 plays a Tempo Timmy
    const timmy = s.play(TIMMY, { zone: 2 }).unit("p2", 2)!;
    s.endTurn();
    s.play(ROLLBACK, { modes: ["3"] });
    // Turn 8 was never recorded; the oldest snapshot, turn 10's, is where it goes.
    expect(ofType(s.lastEvents, "rolledBack")).toEqual([{ type: "rolledBack", player: "p1", turnsAgo: 1, sides: ["p1", "p2"] }]);
    s.expectInZone(timmy, "hand");
  });

  it("R562 a game's first turns: the oldest snapshot is the start of turn 1, the empty board", () => {
    const deck = [ROLLBACK, ...["core-002", "core-005", "core-006", "core-008", "core-011", "core-012", "core-013", "core-015", "core-016", "core-019",
      "core-020", "core-025", "core-026", "core-032", "core-036", "core-043", "core-044", "core-053", "core-055"]];
    let state = beginGame(createGame({ seed: "rollback-first-turn", decks: [deck, deck] })).state;
    let n = 0;
    const act = (body: ActionInput): void => {
      n += 1;
      const result = reduce(state, { ...body, nonce: `first-${n}` } as Action);
      if (result.error !== undefined) throw new Error(result.error);
      state = result.state;
    };
    for (const player of ["p1", "p2"] as PlayerId[]) act({ type: "mulligan", keep: state.players[player].hand.map((card) => card.id), playerId: player });
    expect(state.boardHistory?.map((snapshot) => snapshot.turn)).toEqual([1]);
    expect(subsystems.snapshotFor(state, 3)?.turn).toBe(1);
    expect(state.boardHistory?.[0]?.sides.p1.units.every((pile) => pile === null)).toBe(true);
  });

  it("R562 with no snapshot at all nothing is restored and nothing is reported", () => {
    const s = scenario({ p1: { hand: [ROLLBACK], field: [VANILLA] }, p2: { field: [TIMMY] } });
    s.play(ROLLBACK, { modes: ["1"] });
    expect(ofType(s.lastEvents, "rolledBack")).toEqual([]);
    expect(s.unit("p1", 1)?.defId).toBe(VANILLA);
    expect(s.unit("p2", 1)?.defId).toBe(TIMMY);
  });
});

describe("C+ #35 Rollback — held zones (R563)", () => {
  it("R563 an animated card's home is let go, and the card stands in it again as the snapshot had it", () => {
    const s = onTurn11({ p1: { hand: [ROLLBACK], field: [VANILLA], backrow: [{ def: SPATULA, lane: 2 }] }, p2: { field: [TIMMY] } });
    const spatula = s.unit("p1", 2)!;
    expect(spatula.defId).toBe(SPATULA);
    expect(s.state.homes).toEqual([{ instanceId: spatula.id, zone: { player: "p1", row: "backrow", lane: 2 } }]);

    s.play(ROLLBACK, { modes: ["1"] });
    expect(s.backrow("p1", 2)?.id).toBe(spatula.id);
    expect(s.unit("p1", 2)).toBeNull();
    expect(s.state.homes).toBeUndefined();
    // It moved along its own side: nothing but `rolledBack` reports it (R171, R566).
    expect(ofType(s.lastEvents, "controlChanged")).toEqual([]);
  });
});

describe("C+ #35 Rollback — Radiant: your side, your opponent's or both", () => {
  const board = (): { s: Scenario; mine: string; theirs: string } => {
    const s = onTurn11({ p1: { hand: [RADIANT_ROLLBACK, TIMMY], field: [VANILLA] }, p2: { hand: [TIMMY], field: [VANILLA] } });
    s.play(TIMMY, { zone: 2 });
    const mine = s.unit("p1", 2)!.id;
    s.endTurn();
    s.play(TIMMY, { zone: 2 });
    const theirs = s.unit("p2", 2)!.id;
    s.endTurn();
    return { s, mine, theirs };
  };

  it("your side: only your side goes back", () => {
    const { s, mine, theirs } = board();
    s.play(ROLLBACK, { modes: ["2", "your side"] });
    s.expectInZone(mine, "hand");
    expect(s.unit("p2", 2)?.id).toBe(theirs);
    expect(ofType(s.lastEvents, "rolledBack")).toEqual([{ type: "rolledBack", player: "p1", turnsAgo: 2, sides: ["p1"] }]);
  });

  it("your opponent's side: only theirs goes back", () => {
    const { s, mine, theirs } = board();
    s.play(ROLLBACK, { modes: ["2", "your opponent's side"] });
    expect(s.unit("p1", 2)?.id).toBe(mine);
    s.expectInZone(theirs, "hand");
    expect(ofType(s.lastEvents, "rolledBack")).toEqual([{ type: "rolledBack", player: "p1", turnsAgo: 2, sides: ["p2"] }]);
  });

  it("both sides: the whole board, as the base face", () => {
    const { s, mine, theirs } = board();
    s.play(ROLLBACK, { modes: ["2", "both sides"] });
    s.expectInZone(mine, "hand").expectInZone(theirs, "hand");
  });

  it("R419 a card on the restored side that the other side's snapshot holds still leaves: only that part goes back", () => {
    const s = onTurn11({ p1: { hand: [RADIANT_ROLLBACK, MIND_CONTROL], field: [VANILLA] }, p2: { field: [VANILLA, TIMMY] } });
    const timmy = s.unit("p2", 2)!;
    s.play(MIND_CONTROL, { targets: target(timmy.id) });
    toTurn13(s);
    s.play(ROLLBACK, { modes: ["2", "your side"] });
    // p1's side as it was holds no Timmy, and p2's side is not restored: it goes to its owner's hand.
    expect(s.hand("p2").map((card) => card.id)).toContain(timmy.id);
  });
});

describe("C+ #35 Rollback — a whole game (§9.3)", () => {
  it("R419 a game using it replays to the same hash, and its views match", () => {
    const deck = [ROLLBACK, "core-008", "core-011", "core-015", "core-002", "core-005", "core-006", "core-012", "core-013", "core-016",
      "core-019", "core-020", "core-025", "core-026", "core-032", "core-036", "core-043", "core-044", "core-053", "core-055"];
    const other = ["core-008", "core-011", "core-015", "core-002", "core-005", "core-006", "core-012", "core-013", "core-016", "core-019",
      "core-020", "core-025", "core-026", "core-032", "core-036", "core-043", "core-044", "core-053", "core-055", "core-004"];
    let found: { seed: string; state: GameState } | null = null;
    for (let at = 0; at < 300 && found === null; at += 1) {
      const seed = `rollback-replay-${at}`;
      const begun = beginGame(createGame({ seed, decks: [deck, other] })).state;
      if (begun.players.p1.hand.some((card) => card.defId === ROLLBACK)) found = { seed, state: begun };
    }
    if (found === null) throw new Error("no seed deals Rollback to p1");
    const log: Action[] = [];
    let state = found.state;
    const act = (body: ActionInput): void => {
      const action = { ...body, nonce: `rb-${log.length}` } as Action;
      const result = reduce(state, action);
      if (result.error !== undefined) throw new Error(result.error);
      log.push(action);
      state = result.state;
    };
    for (const player of ["p1", "p2"] as PlayerId[]) act({ type: "mulligan", keep: state.players[player].hand.map((card) => card.id), playerId: player });
    // Each side plays its cheapest Unit each turn until p1's fourth turn, when Rollback is affordable.
    while (!(state.active === "p1" && state.players.p1.mana.current >= 4)) {
      const player = state.active;
      const play = legalActions(state, player).find((action) => action.type === "play" && action.zone?.row === "units");
      if (play !== undefined) act({ ...play, playerId: player } as ActionInput);
      act({ type: "endTurn", playerId: player });
    }
    const rollback = state.players.p1.hand.find((card) => card.defId === ROLLBACK)?.id ?? "";
    act({ type: "play", playerId: "p1", instanceId: rollback, modes: ["2"] } as ActionInput);
    expect(state.players.p1.graveyard.some((card) => card.defId === ROLLBACK)).toBe(true);

    const replayed = fold({ seed: found.seed, decks: [deck, other], log });
    expect(replayed.errors).toEqual([]);
    expect(hashState(replayed.state)).toBe(hashState(state));
    expect(replayed.state.boardHistory).toEqual(state.boardHistory);
  });

  it("R419 random-policy games with Rollback in both decks keep the fuzz invariants and replay (§9.3, R171)", { timeout: 120_000 }, () => {
    const deck = (offset: number): string[] => [ROLLBACK, ...["core-008", "core-011", "core-015", "core-002", "core-012", "core-016", "core-017",
      "core-019", "core-020", "core-025", "core-026", "core-036", "core-041", "core-043", "core-049", "core-053", "core-055", "core-060", "core-092",
      "core-003", "core-018", "core-071"].slice(offset, offset + 19)];
    let rolledBack = 0;
    for (let seed = 1; seed <= 8; seed += 1) {
      const decks: [string[], string[]] = [deck(0), deck(3)];
      let state = beginGame(createGame({ seed: `rollback-fuzz-${seed}`, decks })).state;
      const policy = createRng(`rollback-policy-${seed}`);
      const monitor = createInvariantMonitor(state);
      const log: Action[] = [];
      for (let step = 0; state.result === null && step < 5000; step += 1) {
        const seat = seatToAct(state);
        const chosen = subsystems.chooseAction(state, seat, policy);
        if (chosen === null) throw new Error(`no action for ${seat}`);
        expect(monitor.before(state, seat, chosen)).toEqual([]);
        const action = { ...chosen, playerId: seat, nonce: `rf-${seed}-${step}` } as Action;
        const result = reduce(state, action);
        if (result.error !== undefined) throw new Error(result.error);
        log.push(action);
        state = result.state;
        expect(monitor.after(result.events, state)).toEqual([]);
        rolledBack += result.events.filter((event) => event.type === "rolledBack").length;
      }
      expect(state.result).not.toBeNull();
      const replayed = fold({ seed: `rollback-fuzz-${seed}`, decks, log });
      expect(replayed.errors).toEqual([]);
      expect(hashState(replayed.state)).toBe(hashState(state));
    }
    // The games did cast it: the property is about Rollback, not about games that never drew it.
    expect(rolledBack).toBeGreaterThan(0);
  });
});
