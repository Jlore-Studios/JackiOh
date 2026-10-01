// Two verbs of the Classic+ #1–#39 workstream: `damageRoundsUntilDeath` (effects/rounds.ts; SPEC
// §8.7 C+ #32.3 Blade Storm, §4.5, R59, R113, R283) and `chooseFromHand`'s `where` (effects/choose.ts;
// C+ #31 Fusion Lab's end of turn, R23). The real cards' tests (packages/cards/test/classic-plus/
// 032-3-blade-storm.test.ts, 031-fusion-lab.test.ts) cover the cards again.
//
// Fixtures are this file's own: defs are `pcv-`, indexed from 5900.

import type { Action, ActionInput, CardDef, CardFace, GameEvent, Keyword, Selection } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { registerCatalog, registeredCatalog } from "../src/catalog";
import { BLADE_STORM_ROUNDS } from "../src/config";
import { chooseFromHand, chooseMode } from "../src/effects/choose";
import { damageRoundsUntilDeath } from "../src/effects/rounds";
import { unitHas } from "../src/layers";
import { beginGame, reduce } from "../src/reduce";
import { hashState } from "../src/replay";
import type { CardScripts, Script } from "../src/script";
import { registerScripts, registeredScripts } from "../src/scripts";
import type { CardInstance, GameState } from "../src/state";
import { viewFor } from "../src/viewFor";
import { inHand, newGame, put, slot } from "./fixtures/harness";

let nextIndex = 5900;

function def(name: string, type: CardDef["type"], extra: Partial<CardDef> = {}): CardDef {
  nextIndex += 1;
  const face: CardFace = { keywords: [], text: name };
  return {
    id: `pcv-${name}`,
    index: String(nextIndex),
    name: `PCV ${name}`,
    set: "Core",
    type,
    tags: [],
    rarity: "Common",
    token: false,
    cost: 0,
    base: face,
    radiant: face,
    ...extra,
  };
}

function unit(name: string, attack: number, health: number, keywords: Keyword[] = []): CardDef {
  const face: CardFace = { attack, health, keywords, text: name };
  return def(name, "Unit", { base: face, radiant: face });
}

const storm = def("storm", "Spell");
const shortStorm = def("short-storm", "Spell");
const SHORT_ROUNDS = 3;
const body3 = unit("body-3", 3, 3);
const body5 = unit("body-5", 5, 5);
const shielded = unit("shielded", 1, 1, [{ kind: "Divine Shield" }]);
const armored = unit("armored", 1, 1, [{ kind: "Armor", n: 1 }]);
const unbreakable = unit("unbreakable", 1, 2, [{ kind: "Indestructible" }]);
/** Survives every round of a 30-round storm, so its hits count the rounds. */
const tank = unit("tank", 1, BLADE_STORM_ROUNDS + 1);
const reborn = unit("reborn", 1, 2, [{ kind: "Reborn" }]);
const spellproof = unit("spellproof", 1, 1, [{ kind: "Immune to Spells" }]);
const spellDamage = unit("spell-damage", 1, 9, [{ kind: "Spell Damage", n: 1 }]);
/** Death: its controller chooses "left" or "right" — a Death hook that asks inside a round's check. */
const askOnDeath = unit("ask-on-death", 1, 1);
/** End of turn: choose a hand card that is not Immutable; remembers the pick. */
const picker = def("picker", "Field Spell");
const immutable = unit("immutable", 1, 1, [{ kind: "Immutable" }]);
const plain = unit("plain", 1, 1);

const DEFS = [storm, shortStorm, body3, body5, shielded, armored, unbreakable, tank, reborn, spellproof, spellDamage, askOnDeath, picker, immutable, plain];

function both(script: Script): CardScripts {
  return { base: script, radiant: script };
}

const SCRIPTS: Record<string, CardScripts> = {
  [storm.id]: {
    base: { cry: () => [damageRoundsUntilDeath({ amount: 1, rounds: BLADE_STORM_ROUNDS, side: "any" })] },
    radiant: { cry: () => [damageRoundsUntilDeath({ amount: 1, rounds: BLADE_STORM_ROUNDS, side: "enemy" })] },
  },
  [shortStorm.id]: both({ cry: () => [damageRoundsUntilDeath({ amount: 1, rounds: SHORT_ROUNDS, side: "any" })] }),
  [askOnDeath.id]: both({ death: () => [chooseMode({ options: ["left", "right"], step: "asked" })], resume: { asked: () => [] } }),
  [picker.id]: both({
    endOfTurn: () => [chooseFromHand({ step: "picked", where: (ctx, card) => !unitHas(ctx.state, card, "Immutable") })],
    resume: { picked: () => [] },
  }),
};

let nonce = 0;
function act(state: GameState, body: ActionInput): { state: GameState; events: GameEvent[] } {
  nonce += 1;
  const result = reduce(state, { ...body, nonce: `pcv${nonce}` } as Action);
  if (result.error !== undefined) throw new Error(result.error);
  return result;
}

/** p1's main phase, hands empty, this file's cards registered. */
function playing(seed: string): GameState {
  let state = beginGame(newGame(seed)).state;
  for (const playerId of ["p1", "p2"] as const) {
    state = act(state, { type: "mulligan", keep: state.players[playerId].hand.map((c) => c.id), playerId }).state;
  }
  registerCatalog({ ...registeredCatalog(), ...Object.fromEntries(DEFS.map((d) => [d.id, d])) });
  registerScripts({ ...registeredScripts(), ...SCRIPTS });
  for (const playerId of ["p1", "p2"] as const) state.players[playerId].hand = [];
  return state;
}

/** p1 casts a storm onto the board `setup` builds (a spare hand card keeps the turn going). */
function stormed(seed: string, setup: (state: GameState) => void, options: { defId?: string; radiant?: boolean } = {}) {
  const state = playing(seed);
  setup(state);
  const [card] = inHand(state, options.defId ?? storm.id, "p1");
  if (card === undefined) throw new Error("the storm");
  card.radiant = options.radiant === true;
  inHand(state, plain.id, "p1");
  return { before: state, ...act(state, { type: "play", instanceId: card.id, playerId: "p1" }) };
}

const hitsTo = (events: readonly GameEvent[], card: CardInstance): number[] =>
  events.flatMap((e) => (e.type === "damage" && e.targetId === card.id ? [e.amount] : []));
const deaths = (events: readonly GameEvent[]): string[] => events.flatMap((e) => (e.type === "destroyed" ? [e.defId] : []));
const roundsRun = (events: readonly GameEvent[], card: CardInstance): number => hitsTo(events, card).length;

describe("R59 damageRoundsUntilDeath: each round checks state, and the storm stops after a round in which a Unit died", () => {
  it("R59 1 damage to every Unit a round until one dies: a 3/3 dies in round 3 and the 5/5 keeps 3 damage", () => {
    let small: CardInstance | undefined;
    let big: CardInstance | undefined;
    const { state, events } = stormed("pcv-basic", (s) => {
      small = put(s, body3.id, slot("p2", "units", 1));
      big = put(s, body5.id, slot("p1", "units", 1));
    });
    if (small === undefined || big === undefined) throw new Error("setup");
    expect(hitsTo(events, small)).toEqual([1, 1, 1]);
    expect(hitsTo(events, big)).toEqual([1, 1, 1]);
    expect(deaths(events)).toEqual([body3.id]);
    expect(state.players.p1.units[0]?.[0]?.damage).toBe(3);
  });

  it("R59 each round's check comes before the next round's hits: the death sits between round 3 and nothing after", () => {
    let small: CardInstance | undefined;
    const { events } = stormed("pcv-order", (s) => {
      small = put(s, body3.id, slot("p2", "units", 1));
      put(s, body5.id, slot("p1", "units", 1));
    });
    const kinds = events.map((e) => e.type);
    expect(kinds.lastIndexOf("damage")).toBeLessThan(kinds.indexOf("destroyed"));
    expect(small).toBeDefined();
  });

  it("R59 Divine Shield pops in the first round and the storm goes on", () => {
    let shield: CardInstance | undefined;
    const { events } = stormed("pcv-shield", (s) => {
      shield = put(s, shielded.id, slot("p2", "units", 1));
      put(s, body5.id, slot("p1", "units", 1));
    });
    expect(events.filter((e) => e.type === "divineShieldLost")).toHaveLength(1);
    expect(deaths(events)).toEqual([shielded.id]);
    if (shield === undefined) throw new Error("setup");
    expect(hitsTo(events, shield)).toEqual([1]);
  });

  it("R59 a board no round can kill (Armor 1, Indestructible) runs out its rounds and stops: no hit lands, nothing dies", () => {
    const { state, events } = stormed("pcv-walls", (s) => {
      put(s, armored.id, slot("p2", "units", 1));
      put(s, unbreakable.id, slot("p1", "units", 1));
    });
    expect(events.filter((e) => e.type === "damage")).toEqual([]);
    expect(deaths(events)).toEqual([]);
    expect(state.pending).toBeNull();
  });

  it("R59 with no death the storm runs exactly the round cap, BLADE_STORM_ROUNDS", () => {
    let counter: CardInstance | undefined;
    const { state, events } = stormed("pcv-cap", (s) => {
      put(s, unbreakable.id, slot("p2", "units", 1));
      counter = put(s, tank.id, slot("p1", "units", 1));
    });
    if (counter === undefined) throw new Error("setup");
    expect(roundsRun(events, counter)).toBe(BLADE_STORM_ROUNDS);
    expect(state.players.p1.units[0]?.[0]?.damage).toBe(BLADE_STORM_ROUNDS);
    expect(deaths(events)).toEqual([]);
  });

  it("R59 the cap is the caller's: a three-round storm stops after three", () => {
    let counter: CardInstance | undefined;
    const { events } = stormed("pcv-short", (s) => {
      counter = put(s, tank.id, slot("p1", "units", 1));
    }, { defId: shortStorm.id });
    if (counter === undefined) throw new Error("setup");
    expect(roundsRun(events, counter)).toBe(SHORT_ROUNDS);
  });

  it("R59 a Reborn death counts: the storm stops, and the unit comes back", () => {
    let back: CardInstance | undefined;
    let wall: CardInstance | undefined;
    const { state, events } = stormed("pcv-reborn", (s) => {
      back = put(s, reborn.id, slot("p2", "units", 1));
      wall = put(s, tank.id, slot("p1", "units", 1));
    });
    if (back === undefined || wall === undefined) throw new Error("setup");
    expect(deaths(events)).toEqual([reborn.id]);
    expect(roundsRun(events, wall)).toBe(2);
    expect(state.players.p2.units[0]?.[0]?.defId).toBe(reborn.id);
  });

  it("R59 with no Unit to hit nothing happens; an Immune to Spells Unit is never hit and never counts", () => {
    expect(stormed("pcv-empty", () => undefined).events.some((e) => e.type === "damage")).toBe(false);
    let proof: CardInstance | undefined;
    const { events } = stormed("pcv-proof", (s) => {
      proof = put(s, spellproof.id, slot("p2", "units", 1));
    });
    if (proof === undefined) throw new Error("setup");
    expect(hitsTo(events, proof)).toEqual([]);
    expect(events.some((e) => e.type === "damage")).toBe(false);
  });

  it("§4.4 Spell Damage raises every round's hit", () => {
    let target: CardInstance | undefined;
    const { events } = stormed("pcv-spell-damage", (s) => {
      put(s, spellDamage.id, slot("p1", "units", 1));
      target = put(s, body5.id, slot("p2", "units", 1));
    });
    if (target === undefined) throw new Error("setup");
    expect(hitsTo(events, target)).toEqual([2, 2, 2]);
  });

  it("R59 the Radiant storm hits enemy Units only, and a death there stops it", () => {
    let mine: CardInstance | undefined;
    const { events } = stormed("pcv-radiant", (s) => {
      mine = put(s, body5.id, slot("p1", "units", 1));
      put(s, body3.id, slot("p2", "units", 1));
    }, { radiant: true });
    if (mine === undefined) throw new Error("setup");
    expect(hitsTo(events, mine)).toEqual([]);
    expect(deaths(events)).toEqual([body3.id]);
  });

  it("R113 a Death hook asking inside a round's check pauses the storm; the pause survives JSON and nothing more hits after the answer", () => {
    let wall: CardInstance | undefined;
    const { state, events } = stormed("pcv-ask", (s) => {
      put(s, askOnDeath.id, slot("p2", "units", 1));
      wall = put(s, tank.id, slot("p1", "units", 1));
    });
    if (wall === undefined) throw new Error("setup");
    expect(state.pending?.kind).toBe("mode");
    expect(roundsRun(events, wall)).toBe(1);
    const thawed = JSON.parse(JSON.stringify(state)) as GameState;
    expect(thawed).toEqual(state);
    const pending = thawed.pending;
    if (pending === null) throw new Error("a prompt");
    const answer = (s: GameState) =>
      act(s, { type: "answer", choiceId: pending.id, selection: [{ pick: "mode", option: "left" } as Selection], playerId: pending.playerId });
    const live = answer(state);
    const again = answer(thawed);
    expect(hashState(again.state)).toBe(hashState(live.state));
    expect(live.state.pending).toBeNull();
    expect(roundsRun(live.events, wall)).toBe(0);
  });

  it("§9.3 a storm replays to the same hash from a JSON copy", () => {
    const state = playing("pcv-replay");
    put(state, body3.id, slot("p2", "units", 1));
    put(state, armored.id, slot("p1", "units", 1));
    const [card] = inHand(state, storm.id, "p1");
    const action = { type: "play", instanceId: card?.id ?? "", playerId: "p1", nonce: "pcv-replay" } as Action;
    const thawed = JSON.parse(JSON.stringify(state)) as GameState;
    expect(hashState(reduce(thawed, action).state)).toBe(hashState(reduce(state, action).state));
  });
});

describe("R23 chooseFromHand's `where`: only the cards it admits are offered", () => {
  function atEndOfTurn(seed: string, hand: string[]) {
    const state = playing(seed);
    put(state, picker.id, slot("p1", "backrow", 1));
    const held = hand.flatMap((defId) => inHand(state, defId, "p1"));
    return { held, ...act(state, { type: "endTurn", playerId: "p1" }) };
  }

  it("R23 offers the hand cards the filter admits, to the chooser alone", () => {
    const { state, held } = atEndOfTurn("pcv-pick", [immutable.id, plain.id, plain.id]);
    const pending = state.pending;
    expect(pending?.kind).toBe("hand");
    expect(pending?.options.map((o) => o.key)).toEqual(held.slice(1).map((card) => `instance:${card.id}`));
    expect(JSON.stringify(viewFor(state, "p2").pending ?? null)).not.toContain(held[1]?.id ?? "none");
  });

  it("R23 with no admitted card it asks nothing", () => {
    expect(atEndOfTurn("pcv-none", [immutable.id]).state.pending).toBeNull();
  });
});
