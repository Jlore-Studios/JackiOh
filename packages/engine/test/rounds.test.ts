// `castRoundsUntilDeath` (effects/rounds.ts; SPEC §8.7 C+ #32.3 Blade Storm's base face, R59, R652):
// round after round casts the named Spell — each round a real Spell cast (R70) with its own state
// check — until a round in which a Unit died, `rounds` rounds, or no Unit is left. The real card's
// test (packages/cards/test/classic-plus/032-3-blade-storm.test.ts) covers the card again.
//
// Fixtures are this file's own: defs are `pcr-`, indexed from 5960.

import type { Action, ActionInput, CardDef, CardFace, GameEvent, Keyword } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { registerCatalog, registeredCatalog } from "../src/catalog";
import { damageAll } from "../src/effects/damage";
import { castRoundsUntilDeath } from "../src/effects/rounds";
import { beginGame, reduce } from "../src/reduce";
import type { CardScripts, Script } from "../src/script";
import { registerScripts, registeredScripts } from "../src/scripts";
import type { CardInstance, GameState } from "../src/state";
import { inHand, newGame, put, slot } from "./fixtures/harness";

let nextIndex = 5960;

function def(name: string, type: CardDef["type"], extra: Partial<CardDef> = {}): CardDef {
  nextIndex += 1;
  const face: CardFace = { keywords: [], text: name };
  return {
    id: `pcr-${name}`,
    index: String(nextIndex),
    name: `PCR ${name}`,
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

/** "Deal 1 damage to all Units", the Whirlwind each round casts. */
const pingAll = def("ping-all", "Spell");
/** Cast it round after round, up to three. */
const castStorm = def("cast-storm", "Spell");
const SHORT_ROUNDS = 3;
/** Cast it round after round, up to thirty. */
const longStorm = def("long-storm", "Spell");
const LONG_ROUNDS = 30;
const body1 = unit("body-1", 1, 1);
const body5 = unit("body-5", 5, 5);
/** Survives every round of a short storm, so its hits count the rounds. */
const tank = unit("tank", 1, SHORT_ROUNDS + 1);
const plain = unit("plain", 1, 1);

const DEFS = [pingAll, castStorm, longStorm, body1, body5, tank, plain];

function both(script: Script): CardScripts {
  return { base: script, radiant: script };
}

const SCRIPTS: Record<string, CardScripts> = {
  [pingAll.id]: both({ cry: () => [damageAll({ amount: 1, side: "any" })] }),
  [castStorm.id]: both({ cry: () => [castRoundsUntilDeath({ def: pingAll.id, rounds: SHORT_ROUNDS })] }),
  [longStorm.id]: both({ cry: () => [castRoundsUntilDeath({ def: pingAll.id, rounds: LONG_ROUNDS })] }),
};

let nonce = 0;
function act(state: GameState, body: ActionInput): { state: GameState; events: GameEvent[] } {
  nonce += 1;
  const result = reduce(state, { ...body, nonce: `pcr${nonce}` } as Action);
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
function casted(seed: string, setup: (state: GameState) => void, defId: string = castStorm.id) {
  const state = playing(seed);
  setup(state);
  const [card] = inHand(state, defId, "p1");
  if (card === undefined) throw new Error("the storm");
  inHand(state, plain.id, "p1");
  return { before: state, ...act(state, { type: "play", instanceId: card.id, playerId: "p1" }) };
}

const casts = (events: readonly GameEvent[]): number =>
  events.filter((event) => event.type === "cardPlayed" && event.defId === pingAll.id).length;
const resolved = (events: readonly GameEvent[]): number =>
  events.filter((event) => event.type === "cardResolved" && event.defId === pingAll.id).length;
const deaths = (events: readonly GameEvent[]): string[] => events.flatMap((event) => (event.type === "destroyed" ? [event.defId] : []));

describe("R652 castRoundsUntilDeath: each round casts, and the storm stops after a round in which a Unit died", () => {
  it("R652 a round in which a Unit died ends it: a 1/1 dies in round 1 and only one cast happens", () => {
    const { events } = casted("pcr-basic", (s) => {
      put(s, body1.id, slot("p2", "units", 1));
      put(s, body5.id, slot("p1", "units", 1));
    }, longStorm.id);
    expect(casts(events)).toBe(1);
    expect(deaths(events)).toEqual([body1.id]);
  });

  it("R652 each cast is a real Spell cast (R70): every cast is announced, played and resolved", () => {
    const { events } = casted("pcr-real", (s) => {
      put(s, tank.id, slot("p2", "units", 1));
    });
    expect(casts(events)).toBe(SHORT_ROUNDS);
    expect(resolved(events)).toBe(SHORT_ROUNDS);
    expect(events.filter((event) => event.type === "cardAnnounced" && event.defId === pingAll.id)).toHaveLength(SHORT_ROUNDS);
  });

  it("R652 with no death the storm runs exactly the round cap and stops settled", () => {
    let counter: CardInstance | undefined;
    const { state, events } = casted("pcr-cap", (s) => {
      counter = put(s, tank.id, slot("p1", "units", 1));
    });
    if (counter === undefined) throw new Error("setup");
    expect(casts(events)).toBe(SHORT_ROUNDS);
    expect(deaths(events)).toEqual([]);
    expect(state.pending).toBeNull();
  });

  it("R652 with no Unit left it casts nothing", () => {
    const { events } = casted("pcr-empty", () => {});
    expect(casts(events)).toBe(0);
    expect(events.filter((event) => event.type === "damage")).toEqual([]);
    expect(deaths(events)).toEqual([]);
  });
});
