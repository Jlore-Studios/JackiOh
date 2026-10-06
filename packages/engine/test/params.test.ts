// Declared numbers (docs/classic-sets.md B3.4 rule 5, R386): `param(ctx, key)` in a card script, the
// pure `paramValue` the view and a `preview` hook read, the default steps and the bounds, a number
// KY's Constant set and the steps after it, a fused card's ingredients each reading their own
// declaration (R102), a number tuned on the Radiant face only (R749), and a card resolving with the
// number as it stands.

import type { Action, ActionInput } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { defOf } from "../src/catalog";
import { PARAM_DEFAULT_STEP, TUNE_MIN_AMOUNT } from "../src/config";
import { numbersOn } from "../src/numbers";
import { param, paramStep, paramValue, paramsOf, paramsView, setParam, stepParam, steppableParams } from "../src/params";
import { makeContext } from "../src/resolve";
import { beginGame, legalActions, reduce } from "../src/reduce";
import type { GameState } from "../src/state";
import { fuse } from "../src/subsystems/fuse";
import { viewFor } from "../src/viewFor";
import { PART_KEY } from "../src/work";
import { inHand, put, sinkFor, slot } from "./fixtures/harness";
import { body, instanceGame, nerfer, numbered, radiantNumber } from "./fixtures/instanceData";

function game(): GameState {
  const state = instanceGame("params");
  state.turn = 3;
  state.active = "p1";
  return state;
}

describe("B3.4 rule 5: declared numbers (R386)", () => {
  it("R386 a number reads its face's printed value until the card's tuning moves it", () => {
    const state = game();
    const [card] = inHand(state, numbered.id, "p1");
    if (card === undefined) throw new Error("no card");
    expect(paramValue(state, card, "damage")).toBe(2);
    card.radiant = true;
    expect(paramValue(state, card, "damage")).toBe(4);
    expect(paramValue(state, card, "damage", { radiant: false })).toBe(2);
    stepParam(card, "damage", 1);
    expect(card.tuning).toEqual({ numbers: { damage: 1 } });
    expect(paramValue(state, card, "damage")).toBe(5);
    stepParam(card, "damage", -1);
    // Steps that cancel out leave the card untuned (§9.3: it hashes as a card never touched).
    expect(card.tuning).toBeUndefined();
  });

  it("R749 a number tunedOn radiant reads its printed value on the base face whatever its steps, and steps on the Radiant face", () => {
    const state = game();
    const [card] = inHand(state, radiantNumber.id, "p1");
    if (card === undefined) throw new Error("no card");
    // The base face prints no number: no change finds one to move, KY's Constant lists none, and a
    // step recorded anyway leaves it at its printed 1.
    expect(steppableParams(state, card, "upgrade")).toEqual([]);
    expect(steppableParams(state, card, "degrade")).toEqual([]);
    expect(numbersOn(state, card).some((entry) => entry.id === "param:times")).toBe(false);
    stepParam(card, "times", 1);
    expect(paramValue(state, card, "times")).toBe(1);
    setParam(card, "times", 3);
    expect(paramValue(state, card, "times")).toBe(1);
    // The Radiant face prints it, so it is tuned as any number is.
    const [shining] = inHand(state, radiantNumber.id, "p1");
    if (shining === undefined) throw new Error("no card");
    shining.radiant = true;
    expect(paramValue(state, shining, "times")).toBe(2);
    expect(steppableParams(state, shining, "upgrade").map((entry) => entry.delta)).toEqual([1]);
    expect(numbersOn(state, shining).find((entry) => entry.id === "param:times")?.value).toBe(2);
    stepParam(shining, "times", 1);
    expect(paramValue(state, shining, "times")).toBe(3);
  });

  it("R386 the default step is 1 up to 5, 2 from 6 to 12 and a quarter (rounded) above; a declared step wins", () => {
    const def = defOf(game(), numbered.id);
    const byKey = (key: string) => {
      const found = def.params?.find((p) => p.key === key);
      if (found === undefined) throw new Error(key);
      return found;
    };
    expect(PARAM_DEFAULT_STEP.small.step).toBe(1);
    expect(paramStep(byKey("damage"), 2)).toBe(1);
    expect(paramStep(byKey("big"), 8)).toBe(2);
    expect(paramStep(byKey("big"), 16)).toBe(4);
    expect(paramStep(byKey("huge"), 20)).toBe(5);
    expect(paramStep({ ...byKey("huge"), step: 3 }, 20)).toBe(3);
  });

  it("R386 an amount never drops below 1 (or its declared min) and never rises above its max", () => {
    const state = game();
    const [card] = inHand(state, numbered.id, "p1");
    if (card === undefined) throw new Error("no card");
    stepParam(card, "damage", -10);
    expect(paramValue(state, card, "damage")).toBe(TUNE_MIN_AMOUNT);
    stepParam(card, "threshold", -10);
    expect(paramValue(state, card, "threshold")).toBe(2);
    stepParam(card, "huge", 100);
    expect(paramValue(state, card, "huge")).toBe(44);
  });

  it("R386 a number set outright is the value from then on, and a later step counts from it", () => {
    const state = game();
    const [card] = inHand(state, numbered.id, "p1");
    if (card === undefined) throw new Error("no card");
    stepParam(card, "big", 2);
    setParam(card, "big", 3);
    expect(paramValue(state, card, "big")).toBe(3);
    expect(card.tuning).toEqual({ set: { big: 3 } });
    stepParam(card, "big", 1);
    // The step is read off the face's printed number (8: 2), so one step from 3 is 5.
    expect(paramValue(state, card, "big")).toBe(5);
  });

  it("R386 param(ctx, key) reads the running card on the face that runs; a card that has ceased to exist reads its printed value", () => {
    const state = game();
    const [card] = inHand(state, numbered.id, "p1");
    if (card === undefined) throw new Error("no card");
    stepParam(card, "damage", 2);
    const ctx = makeContext(sinkFor(state), card);
    expect(param(ctx, "damage")).toBe(4);
    expect(param({ ...ctx, radiant: true }, "damage")).toBe(6);
    expect(param({ state, self: null, radiant: true, defId: numbered.id }, "damage")).toBe(4);
    expect(() => param(ctx, "nope")).toThrow(/declares no number "nope"/);
    expect(() => param({ state, self: null, radiant: false }, "damage")).toThrow(/no card/);
  });

  it("R102 on a fused card each ingredient's text reads its own declaration, and the card declares the union", () => {
    const state = game();
    const kept = put(state, body.id, slot("p1", "units", 1));
    const [a] = inHand(state, numbered.id, "p1");
    const [b] = inHand(state, nerfer.id, "p1");
    if (a === undefined || b === undefined) throw new Error("no card");
    const fused = fuse(sinkFor(state), { ingredients: [a, b], target: kept });
    if (fused === null) throw new Error("expected a fusion");
    expect(paramsOf(state, fused.defId).map((p) => p.key)).toEqual(["damage", "threshold", "big", "huge", "times"]);
    const ctx = makeContext(sinkFor(state), fused);
    // Ingredient 0 is the numbered spell, 1 Book of Nerf's shape, and the kept body comes last (R77).
    expect(param({ ...ctx, data: { [PART_KEY]: [0] } }, "damage")).toBe(2);
    expect(param({ ...ctx, data: { [PART_KEY]: [1] } }, "times")).toBe(3);
    expect(() => param({ ...ctx, data: { [PART_KEY]: [1] } }, "damage")).toThrow(/declares no number/);
    expect(paramsView(state, fused)).toEqual({ damage: 2, threshold: 3, big: 8, huge: 20, times: 3 });
  });

  it("R102 a part path that names no ingredient is an error, not the numbers of the card it stopped at", () => {
    const state = game();
    const kept = put(state, body.id, slot("p1", "units", 1));
    const [a] = inHand(state, numbered.id, "p1");
    const [b] = inHand(state, nerfer.id, "p1");
    if (a === undefined || b === undefined) throw new Error("no card");
    const fused = fuse(sinkFor(state), { ingredients: [a, b], target: kept });
    if (fused === null) throw new Error("expected a fusion");
    const ctx = makeContext(sinkFor(state), fused);
    // Three ingredients: index 3 is past them, and ingredient 0 is a catalog card with no ingredients of its own.
    expect(() => param({ ...ctx, data: { [PART_KEY]: [3] } }, "damage")).toThrow(/no ingredient 3 on the part path 3/);
    expect(() => param({ ...ctx, data: { [PART_KEY]: [0, 0] } }, "damage")).toThrow(/no ingredient 0 on the part path 0\.0/);
  });

  it("R386 a card that declares no number carries no `params` in its view", () => {
    const state = game();
    put(state, body.id, slot("p1", "units", 1));
    const [card] = inHand(state, numbered.id, "p1");
    if (card === undefined) throw new Error("no card");
    const view = viewFor(state, "p1");
    expect(view.you.units[0]?.params).toBeUndefined();
    const hand = view.you.hand;
    if (!Array.isArray(hand)) throw new Error("own hand is a list");
    expect(hand.find((c) => c.instanceId === card.id)?.params).toEqual({ damage: 2, threshold: 3, big: 8, huge: 20 });
  });
});

let nonce = 0;
function act(state: GameState, body: ActionInput): GameState {
  nonce += 1;
  const result = reduce(state, { ...body, nonce: `pm${nonce}` } as Action);
  if (result.error !== undefined) throw new Error(result.error);
  return result.state;
}

describe("B3.4 rule 5: a card resolves with its number as it stands (R386)", () => {
  it("R386 a Spell that reads param(ctx, 'damage') deals its tuned amount", () => {
    let state = beginGame(instanceGame("param-play")).state;
    state = act(state, { type: "mulligan", keep: state.players.p1.hand.map((c) => c.id), playerId: "p1" });
    state = act(state, { type: "mulligan", keep: state.players.p2.hand.map((c) => c.id), playerId: "p2" });
    const [card] = inHand(state, numbered.id, "p1");
    if (card === undefined) throw new Error("no card");
    stepParam(card, "damage", 3);
    state.players.p1.mana = { ...state.players.p1.mana, current: 4, max: 4 };
    const before = state.players.p2.hero.health;
    const play = legalActions(state, "p1").find((action) => action.type === "play" && action.instanceId === card.id);
    if (play === undefined) throw new Error("expected a play");
    state = act(state, { ...play, playerId: "p1" } as ActionInput);
    expect(state.players.p2.hero.health).toBe(before - 5);
  });
});
