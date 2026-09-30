// Instance data end to end (docs/classic-sets.md B5 E38, B3.4, R386, R440): buffs and granted keywords
// given in a hand or a deck ride the card onto the field; the E38 scope verbs report only what both
// players read; KY's Constant and a Degrade after a target prompt pause, survive a JSON round trip and
// resume identically; and whole games dealt the instance-data fixtures fold back from their logs to
// the same state (§9.3), with a JSON round trip before every action.

import type { Action, ActionBody, ActionInput, GameEvent, PlayerId } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { registeredCatalog } from "../src/catalog";
import { AI_END_TURN_PROBABILITY } from "../src/config";
import { draw as drawCards } from "../src/draw";
import { buff, buffCards, grantKeyword, grantKeywordCards, recruit } from "../src/effects";
import { unitView } from "../src/layers";
import { makeContext, type HookOptions } from "../src/resolve";
import { beginGame, legalActions, reduce, seatToAct } from "../src/reduce";
import { fold, hashState } from "../src/replay";
import { createRng } from "../src/rng";
import type { Effect } from "../src/script";
import { createGame, type CardInstance, type GameState } from "../src/state";
import { viewFor } from "../src/viewFor";
import { moveToZone } from "../src/zones";
import { plain } from "./fixtures/combat";
import { eventsOfType, inHand, put, setLibrary, sinkFor, slot } from "./fixtures/harness";
import {
  body,
  constant,
  CONSTANT_STEP,
  instanceDeck,
  instanceGame,
  military,
  nerfer,
  numbered,
  registerInstanceFixtures,
} from "./fixtures/instanceData";

let nonce = 0;
function act(state: GameState, body: ActionInput, fixed?: string): GameState {
  nonce += 1;
  const result = reduce(state, { ...body, nonce: fixed ?? `id${nonce}` } as Action);
  if (result.error !== undefined) throw new Error(result.error);
  return result.state;
}

/** p1's main phase on turn 1, past the mulligans, with mana to spend. */
function playing(seed: string): GameState {
  let state = beginGame(instanceGame(seed)).state;
  state = act(state, { type: "mulligan", keep: state.players.p1.hand.map((c) => c.id), playerId: "p1" });
  state = act(state, { type: "mulligan", keep: state.players.p2.hand.map((c) => c.id), playerId: "p2" });
  state.players.p1.mana = { ...state.players.p1.mana, current: 4, max: 4 };
  return state;
}

function run(state: GameState, effect: Effect, options: HookOptions & { self?: CardInstance | null } = {}): GameEvent[] {
  const { self = null, ...hook } = options;
  const sink = sinkFor(state);
  effect.apply(makeContext(sink, self, { controller: "p1", ...hook }));
  state.rngCursor = sink.rng.cursor;
  return sink.events;
}

function playOf(state: GameState, card: CardInstance, pick?: (action: ActionBody) => boolean): ActionInput {
  const play = legalActions(state, "p1").find(
    (action) => action.type === "play" && action.instanceId === card.id && (pick === undefined || pick(action)),
  );
  if (play === undefined) throw new Error(`no play for ${card.defId}`);
  return { ...play, playerId: "p1" } as ActionInput;
}

function onField(state: GameState, id: string): CardInstance {
  const found = state.players.p1.units.flatMap((pile) => pile ?? []).find((card) => card.id === id);
  if (found === undefined) throw new Error("expected the card on the field");
  return found;
}

describe("B5 E38: buffs and keywords in a hand or a deck ride onto the field", () => {
  it("E38 a buff and a keyword given to a hand card are on it when it is played", () => {
    let state = playing("e38-hand");
    const [card] = inHand(state, plain.id, "p1");
    if (card === undefined) throw new Error("no card");
    run(state, buff({ target: { of: "instance", instanceId: card.id }, attack: 2, health: 1 }));
    run(state, grantKeyword({ target: { of: "instance", instanceId: card.id }, keyword: { kind: "Rush" } }));
    const hand = viewFor(state, "p1").you.hand;
    if (!Array.isArray(hand)) throw new Error("own hand is a list");
    // R243: its owner sees what it is made of now.
    expect(hand.find((c) => c.instanceId === card.id)).toMatchObject({ attack: 5, health: 4, keywords: [{ kind: "Rush" }] });
    state = act(state, playOf(state, card));
    const unit = onField(state, card.id);
    expect(unitView(state, unit)).toMatchObject({ attack: 5, maxHealth: 4 });
    expect(unitView(state, unit).keywords).toContainEqual({ kind: "Rush" });
  });

  it("E38 given in a deck, they ride the draw into the hand and a Recruit onto the field", () => {
    const state = playing("e38-deck");
    const [first, second] = setLibrary(state, "p1", [plain.id, body.id]);
    if (first === undefined || second === undefined) throw new Error("no card");
    const events = run(state, buffCards({ scope: { zones: ["library"] }, attack: 1 }));
    run(state, grantKeywordCards({ scope: { zones: ["library"] }, keyword: { kind: "Pierce" } }));
    // R440: the deck changed silently.
    expect(events).toEqual([]);
    drawCards(sinkFor(state), "p1", 1);
    expect(first.zone.z).toBe("hand");
    expect(first.buffs).toEqual({ attack: 1, health: 0 });
    expect(first.grantedKeywords).toEqual([{ kind: "Pierce" }]);
    run(state, recruit({}));
    const recruited = onField(state, second.id);
    expect(unitView(state, recruited).attack).toBe(4);
    expect(unitView(state, recruited).keywords).toContainEqual({ kind: "Pierce" });
  });

  it("E38 they go when the card leaves the field (R78) and when a hand card reaches a graveyard (R215)", () => {
    const state = playing("e38-reset");
    const unit = put(state, plain.id, slot("p1", "units", 1));
    const [held] = inHand(state, plain.id, "p1");
    if (held === undefined) throw new Error("no card");
    for (const card of [unit, held]) {
      run(state, buff({ target: { of: "instance", instanceId: card.id }, attack: 2 }));
      run(state, grantKeyword({ target: { of: "instance", instanceId: card.id }, keyword: { kind: "Taunt" } }));
    }
    moveToZone(state, unit, "hand");
    moveToZone(state, held, "graveyard");
    for (const card of [unit, held]) {
      expect(card.buffs).toEqual({ attack: 0, health: 0 });
      expect(card.grantedKeywords).toEqual([]);
    }
  });

  it("R440 a scope over the field, a hand and a deck reports its public cards only", () => {
    let state = playing("e38-military");
    const unit = put(state, plain.id, slot("p1", "units", 1));
    const handUnits = inHand(state, body.id, "p1", 2);
    const [spell] = inHand(state, military.id, "p1");
    if (spell === undefined) throw new Error("no card");
    const before = state.applied.length;
    state = act(state, playOf(state, spell));
    const events = state.applied.slice(before).flatMap((entry) => entry.events);
    expect(eventsOfType(events, "buffed").map((e) => e.instanceId)).toEqual([unit.id]);
    expect(eventsOfType(events, "keywordGranted").map((e) => e.instanceId)).toEqual([unit.id]);
    for (const card of handUnits) {
      const held = state.players.p1.hand.find((c) => c.id === card.id);
      expect(held?.buffs).toEqual({ attack: 2, health: 0 });
      expect(held?.grantedKeywords).toEqual([{ kind: "Rush" }]);
    }
    for (const card of state.players.p1.library) {
      const isUnit = registeredCatalog()[card.defId]?.type === "Unit";
      expect(card.buffs.attack).toBe(isUnit ? 2 : 0);
    }
  });
});

describe("Classic+ #41 and a Degrade after a prompt: pauses and replays (R113, §9.3, R386)", () => {
  it("R386 KY's Constant's base face sets a random number on the chosen hand card to 3", () => {
    let state = playing("constant-base");
    const [spell] = inHand(state, constant.id, "p1");
    const [target] = inHand(state, numbered.id, "p1");
    if (spell === undefined || target === undefined) throw new Error("no card");
    const before = state.applied.length;
    state = act(state, playOf(state, spell, (a) => a.type === "play" && JSON.stringify(a.targets).includes(target.id)));
    const changed = eventsOfType(state.applied.slice(before).flatMap((e) => e.events), "numberChanged");
    expect(changed).toHaveLength(1);
    expect(changed[0]).toMatchObject({ instanceId: target.id, value: 3, hiddenFrom: ["p2"] });
  });

  it("R386 the Radiant face's Discover pauses, survives JSON, and resumes to the same state in the live game and a copy", () => {
    let state = playing("constant-radiant");
    const [spell] = inHand(state, constant.id, "p1");
    const [target] = inHand(state, numbered.id, "p1");
    if (spell === undefined || target === undefined) throw new Error("no card");
    spell.radiant = true;
    state = act(state, playOf(state, spell, (a) => a.type === "play" && JSON.stringify(a.targets).includes(target.id)));
    const pending = state.pending;
    if (pending === null) throw new Error("expected the Discover");
    expect(pending.kind).toBe("discover");
    expect(pending.options).toHaveLength(3);
    expect(pending.resume.step).toBe(CONSTANT_STEP);
    // Labels are words and numbers, never a raw `{key}` placeholder.
    for (const option of pending.options) expect(option.label).not.toMatch(/[{}]/);
    const other = viewFor(state, "p2");
    expect(other.pending).toEqual({ forYou: false, pendingFor: "p1" });

    const copy = JSON.parse(JSON.stringify(state)) as GameState;
    const option = pending.options[0];
    if (option === undefined) throw new Error("no option");
    const answer = { type: "answer" as const, choiceId: pending.id, selection: [option.selection], playerId: "p1" as const };
    const live = act(state, answer, "a");
    const resumed = act(copy, answer, "a");
    expect(resumed).toEqual(live);
    const held = live.players.p1.hand.find((c) => c.id === target.id);
    expect(held).toBeDefined();
    expect(live.work).toEqual([]);
    const changed = eventsOfType(live.applied.at(-1)?.events ?? [], "numberChanged");
    expect(changed).toHaveLength(1);
    expect(changed[0]?.value).toBe(3);
  });

  it("R386 a Degrade after a target prompt resumes the same from a JSON copy", () => {
    let state = playing("nerf-pause");
    const victim = put(state, body.id, slot("p2", "units", 2));
    const [spell] = inHand(state, nerfer.id, "p1");
    if (spell === undefined) throw new Error("no card");
    state = act(state, playOf(state, spell));
    const pending = state.pending;
    if (pending === null) throw new Error("expected the target prompt");
    const option = pending.options.find((o) => o.selection.pick === "instance" && o.selection.instanceId === victim.id);
    if (option === undefined) throw new Error("expected the victim offered");
    const copy = JSON.parse(JSON.stringify(state)) as GameState;
    const answer = { type: "answer" as const, choiceId: pending.id, selection: [option.selection], playerId: "p1" as const };
    const live = act(state, answer, "b");
    expect(act(copy, answer, "b")).toEqual(live);
    const degraded = eventsOfType(live.applied.at(-1)?.events ?? [], "degraded");
    expect(degraded).toHaveLength(3);
    expect(degraded.every((e) => e.instanceId === victim.id && e.hiddenFrom === undefined)).toBe(true);
  });
});

/**
 * §10.7's random policy over the instance-data decks, with a JSON round trip of the state before every
 * action, so every pause the fixtures open — Discovers, target prompts, Deaths that ask — is crossed
 * as plain data. Returns the log and the final state.
 */
function playOut(seed: string): { state: GameState; log: Action[]; decks: [string[], string[]]; tuned: number } {
  registerInstanceFixtures();
  const decks: [string[], string[]] = [instanceDeck("p1"), instanceDeck("p2")];
  let state = beginGame(createGame({ seed, decks })).state;
  const policy = createRng(`policy-${seed}`);
  const log: Action[] = [];
  let tuned = 0;
  for (let step = 0; state.result === null; step += 1) {
    if (step > 3000) throw new Error(`game ${seed} did not finish`);
    state = JSON.parse(JSON.stringify(state)) as GameState;
    const player: PlayerId = seatToAct(state);
    const actions = legalActions(state, player).filter(
      (action) => action.type !== "concede" && action.type !== "offerDraw" && action.type !== "answerDraw",
    );
    const others = actions.filter((action) => action.type !== "endTurn");
    const endTurn = actions.find((action) => action.type === "endTurn");
    const chosen =
      others.length === 0 || (endTurn !== undefined && policy.chance(AI_END_TURN_PROBABILITY))
        ? (endTurn ?? (others[policy.int(others.length)] as ActionBody))
        : (others[policy.int(others.length)] as ActionBody);
    const action = { ...chosen, playerId: player, nonce: `g${log.length}` } as Action;
    const result = reduce(state, action);
    if (result.error !== undefined) throw new Error(`${action.type} rejected in ${seed}: ${result.error}`);
    log.push(action);
    tuned += result.events.filter((e) => e.type === "degraded" || e.type === "upgraded" || e.type === "numberChanged").length;
    state = result.state;
  }
  return { state, log, decks, tuned };
}

describe("§9.3 games dealt the instance-data fixtures replay exactly (R386, R385)", () => {
  it("R386 six seeded games fold back from their logs to the same state, and they did change cards", () => {
    let tuned = 0;
    for (const seed of ["id-1", "id-2", "id-3", "id-4", "id-5", "id-6"]) {
      const played = playOut(seed);
      const folded = fold({ seed, decks: played.decks, log: played.log, catalog: registeredCatalog() });
      expect(folded.errors).toEqual([]);
      expect(hashState(folded.state)).toBe(hashState(played.state));
      tuned += played.tuned;
    }
    expect(tuned).toBeGreaterThan(0);
  });
});
