// The coach's advice for a lesson's quiet turns (tutorial/advice.ts), on hand-built views: R292. Every
// move it names is one `legalActions` already offered, and it never aims a play at the human's own side.
// The attacks it ranks are the engine's own forecast of them (R3100000), so the hand-built boards below
// are laid on a real engine state for the forecast to be asked of, and a second block holds the coach's
// trades to what the engine's combat really does.

import { describe, expect, it } from "vitest";

import { beginGame, combatForecast, createGame, legalActions, reduce, viewFor, type GameState } from "@jackioh/engine";
import { opponentOf, type Action, type ActionBody, type CombatForecast, type GameEvent, type PlayerId, type PlayerView, type UnitView } from "@jackioh/shared";

import { devDeck } from "../audio/test/realGame.ts";
import { baseView, card, emptySide, unit } from "../test/fixtures.ts";
import type { CoachCtx } from "./coach.ts";
import { lethalOnBoard, nextMove, trades } from "./advice.ts";

/** A unit with no text of its own, 4/4, which a laid board dresses with the stats and keywords of a view's unit. */
const BODY = "core-008";
const BODY_ATTACK = 4;
const BODY_HEALTH = 4;

/**
 * The engine's forecast on the board a hand-built view shows: a real game forced to the viewer's main phase,
 * each unit of the view laid in its lane as a vanilla body given the view's stats and keywords, each hero's
 * life and Armor set. Nothing is worked out here: the units are placed and the engine is asked.
 */
function forecastOn(view: PlayerView): CombatForecast {
  const deck = devDeck("cheap20");
  const state = createGame({ seed: "r3100000-laid", decks: [deck, deck] });
  state.turn = view.turn;
  state.active = view.viewer;
  state.phase = "main";
  for (const [side, seat] of [
    [view.you, view.viewer],
    [view.opponent, opponentOf(view.viewer)],
  ] as const) {
    const player = state.players[seat];
    player.hero.health = side.hero.health;
    player.hero.armor = side.hero.armor;
    side.units.forEach((each: UnitView | null, index: number) => {
      if (each === null) return;
      player.units[index] = [
        {
          id: each.instanceId,
          defId: BODY,
          owner: seat,
          controller: seat,
          radiant: false,
          zone: { z: "field", player: seat, row: "units", lane: index + 1 },
          position: each.position,
          damage: each.maxHealth - each.health,
          buffs: { attack: each.attack - BODY_ATTACK, health: each.maxHealth - BODY_HEALTH },
          grantedKeywords: each.keywords,
          vanilla: true,
          costMod: 0,
          counters: {},
          memory: {},
          exertion: { attacked: false, switched: false },
        },
      ];
    });
  }
  return combatForecast(state, view.viewer);
}

function ctx(view: PlayerView, legal: ActionBody[]): CoachCtx {
  return {
    view,
    legal,
    forecast: forecastOn(view),
    fresh: [],
    aiToAct: false,
    nameOf: (defId) => ({ "core-008": "Mr. Vanilla", "core-013": "Shredder" })[defId],
  };
}

const END: ActionBody = { type: "endTurn" };

describe("R292 the coach's advice", () => {
  it("R292 names the dearest legal play first, never one aimed at the human's own side", () => {
    const mine = unit("p1", { instanceId: "m1" });
    const view = baseView({
      you: emptySide("p1", {
        units: [mine, null, null, null, null],
        hand: [card({ instanceId: "h1", defId: "core-008", cost: 1 }), card({ instanceId: "h2", defId: "core-013", cost: 3 })],
      }),
    });
    const cheap: ActionBody = { type: "play", instanceId: "h1", zone: { row: "units", lane: 2 } };
    const dearAtOwn: ActionBody = { type: "play", instanceId: "h2", zone: { row: "units", lane: 2 }, targets: [{ pick: "instance", instanceId: "m1" }] };
    const dear: ActionBody = { type: "play", instanceId: "h2", zone: { row: "units", lane: 2 }, targets: [{ pick: "hero", player: "p2" }] };
    const move = nextMove(ctx(view, [cheap, dearAtOwn, dear, END]));
    expect(move?.action).toEqual(dear);
    expect(nextMove(ctx(view, [cheap, dearAtOwn, END]))?.action).toEqual(cheap);
  });

  it("R292 prefers a trade that kills and survives, then the hero, then a dent in a Taunt, then End turn", () => {
    const big = unit("p1", { instanceId: "a1", defId: "core-013", attack: 8, health: 10, maxHealth: 10 });
    const small = unit("p2", { instanceId: "e1", defId: "core-008", attack: 3, health: 3, maxHealth: 3 });
    const wall = unit("p2", { instanceId: "e2", defId: "core-019", attack: 9, health: 20, maxHealth: 20, keywords: [{ kind: "Taunt" }] });
    const view = (enemy: PlayerView["opponent"]["units"]): PlayerView =>
      baseView({ you: emptySide("p1", { units: [big, null, null, null, null] }), opponent: emptySide("p2", { units: enemy, hand: { count: 2 } }) });

    const trade: ActionBody = { type: "attack", attackerId: "a1", targetId: "e1" };
    const face: ActionBody = { type: "attack", attackerId: "a1", targetId: "hero-p2" };
    const both = view([small, null, null, null, null]);
    expect(nextMove(ctx(both, [trade, face, END]))?.kind).toBe("trade");
    expect(nextMove(ctx(both, [face, END]))?.kind).toBe("hero");

    const walled = view([wall, null, null, null, null]);
    const dent: ActionBody = { type: "attack", attackerId: "a1", targetId: "e2" };
    expect(nextMove(ctx(walled, [dent, END]))?.kind).toBe("chip");
    expect(nextMove(ctx(walled, [END]))?.kind).toBe("end");
    expect(nextMove(ctx(walled, []))).toBeUndefined();
  });

  it("R292 names the finishing blow before anything else, and clears a Taunt even at the attacker's cost", () => {
    const striker = unit("p1", { instanceId: "a1", defId: "core-013", attack: 8, health: 2, maxHealth: 2 });
    const face: ActionBody = { type: "attack", attackerId: "a1", targetId: "hero-p2" };
    const hand = [card({ instanceId: "h1", defId: "core-008", cost: 1 })];
    const play: ActionBody = { type: "play", instanceId: "h1", zone: { row: "units", lane: 2 } };
    const low = baseView({
      you: emptySide("p1", { units: [striker, null, null, null, null], hand }),
      opponent: emptySide("p2", { hand: { count: 1 }, hero: { health: 8, armor: 0, powers: [], power: null } }),
    });
    const lethal = nextMove(ctx(low, [play, face, END]));
    expect(lethal?.kind).toBe("hero");
    // Not lethal: the card comes first.
    const high = { ...low, opponent: { ...low.opponent, hero: { ...low.opponent.hero, health: 9 } } };
    expect(nextMove(ctx(high, [play, face, END]))?.kind).toBe("play");

    const guard = unit("p2", { instanceId: "e1", defId: "core-003", attack: 5, health: 3, maxHealth: 3, keywords: [{ kind: "Taunt" }] });
    const walled = baseView({
      you: emptySide("p1", { units: [striker, null, null, null, null] }),
      opponent: emptySide("p2", { units: [guard, null, null, null, null], hand: { count: 1 } }),
    });
    const clear: ActionBody = { type: "attack", attackerId: "a1", targetId: "e1" };
    const move = nextMove(ctx(walled, [clear, END]));
    expect(move?.kind).toBe("clear");
  });

  it("R292 breaks a Divine Shield on a Taunt when nothing better is on offer", () => {
    const small = unit("p1", { instanceId: "a1", defId: "core-008", attack: 3, health: 3, maxHealth: 3 });
    const shielded = unit("p2", { instanceId: "e1", defId: "core-003", attack: 1, health: 1, maxHealth: 1, keywords: [{ kind: "Taunt" }, { kind: "Divine Shield" }] });
    const view = baseView({
      you: emptySide("p1", { units: [small, null, null, null, null] }),
      opponent: emptySide("p2", { units: [shielded, null, null, null, null], hand: { count: 1 } }),
    });
    const pop: ActionBody = { type: "attack", attackerId: "a1", targetId: "e1" };
    const move = nextMove(ctx(view, [pop, END]));
    expect(move?.kind).toBe("chip");
  });
});

// A real game, laid with cards of the catalog by the practice sandbox (R923000), which places them as an
// effect does. Deft Duelist (4/3, Charge) attacks the turn it lands, so no turn has to pass.
const DUELIST = "core-045";
/** Training Grounds' Chosen by the Sun: 3/3, Divine Shield. */
const SUN = "traininggrounds-006";
/** Training Grounds' Chosen by the Moon: 3/3, Taunt, Armor 3. */
const MOON = "traininggrounds-007";
/** Core's 4-mana 7/7, Armor 7. */
const WALL = "core-025";

type Laid = { p1: string[]; p2: string[]; hero?: { health: number; armor: number } };

let nonce = 0;
function send(state: GameState, player: PlayerId, body: ActionBody): GameState {
  nonce += 1;
  const action = { ...body, playerId: player, nonce: `r3100000-${String(nonce)}` } as Action;
  const result = reduce(state, action);
  if (result.error !== undefined) throw new Error(`${player} ${body.type}: ${result.error}`);
  return result.state;
}

function sandbox(state: GameState, op: unknown): GameState {
  return send(state, "p1", { type: "sandbox", op } as unknown as ActionBody);
}

/** A sandbox game at p1's main phase with `laid` on the board, p1's units in lanes 1, 2, …, p2's in 1, 2, …. */
function laid(seed: string, { p1, p2, hero }: Laid): GameState {
  const deck = devDeck("cheap20");
  let state = beginGame(createGame({ seed, decks: [deck, deck], sandbox: true })).state;
  for (let step = 0; step < 40; step += 1) {
    const view = viewFor(state, "p1");
    if (view.active === "p1" && view.phase === "main" && view.pending === null) break;
    const actor = (["p1", "p2"] as const).find((seat) => legalActions(state, seat).some((a) => a.type !== "concede" && a.type !== "offerDraw"));
    if (actor === undefined) break;
    const options = legalActions(state, actor);
    const mulligan = options.find((a) => a.type === "mulligan");
    state = send(state, actor, mulligan ?? options.find((a) => a.type === "answer") ?? { type: "endTurn" });
  }
  p1.forEach((defId, index) => {
    state = sandbox(state, { kind: "spawn", player: "p1", defId, to: { zone: "field", row: "units", lane: index + 1 } });
  });
  p2.forEach((defId, index) => {
    state = sandbox(state, { kind: "spawn", player: "p2", defId, to: { zone: "field", row: "units", lane: index + 1 } });
  });
  if (hero !== undefined) state = sandbox(state, { kind: "setHero", player: "p2", ...hero });
  return state;
}

function coachCtx(state: GameState): CoachCtx {
  return { view: viewFor(state, "p1"), legal: legalActions(state, "p1"), forecast: combatForecast(state, "p1"), fresh: [], aiToAct: false };
}

/** What the engine's combat did to each of two cards in the events of one action. */
function damageTo(events: readonly GameEvent[], instanceId: string): number {
  return events.reduce((sum, event) => (event.type === "damage" && event.targetId === instanceId ? sum + event.amount : sum), 0);
}

describe("R3100000 the coach's trades are the engine's own combat", () => {
  /** Plays the coach's trade for real and returns the damage each side took and whether the shield broke. */
  function played(state: GameState, attackerId: string, targetId: string) {
    const result = reduce(state, { type: "attack", attackerId, targetId, playerId: "p1", nonce: `r3100000-play-${attackerId}-${targetId}` });
    if (result.error !== undefined) throw new Error(result.error);
    return {
      dealt: damageTo(result.events, targetId),
      taken: damageTo(result.events, attackerId),
      popped: result.events.some((event) => event.type === "divineShieldLost" && event.instanceId === targetId),
    };
  }

  it("R3100000 Armor takes its part of the blow and of the strike back, as the engine's damage events say", () => {
    const state = laid("r3100000-armor", { p1: [DUELIST], p2: [MOON] });
    const context = coachCtx(state);
    const [trade, ...rest] = trades(context);
    expect(rest).toEqual([]);
    expect(trade).toBeDefined();
    if (trade === undefined) return;
    // The Duelist's 4 less the Moon's Armor 3 is 1 into a 3-health Taunt; the Moon's 3 back kills the Duelist.
    expect([trade.dealt, trade.taken, trade.kills, trade.survives, trade.pops]).toEqual([1, 3, false, false, false]);
    const real = played(state, trade.action.attackerId, trade.action.targetId);
    expect([real.dealt, real.taken, real.popped]).toEqual([trade.dealt, trade.taken, trade.pops]);
  });

  it("R3100000 Armor that takes the whole blow leaves the target unhurt", () => {
    const state = laid("r3100000-wall", { p1: [DUELIST], p2: [WALL] });
    const [trade] = trades(coachCtx(state));
    expect([trade?.dealt, trade?.taken, trade?.kills]).toEqual([0, 7, false]);
    const real = played(state, trade?.action.attackerId ?? "", trade?.action.targetId ?? "");
    expect([real.dealt, real.taken]).toEqual([0, 7]);
  });

  it("R3100000 a Divine Shield takes the whole blow and breaks, as the engine's combat does", () => {
    const state = laid("r3100000-shield", { p1: [DUELIST], p2: [SUN] });
    const [trade] = trades(coachCtx(state));
    expect([trade?.dealt, trade?.taken, trade?.kills, trade?.pops]).toEqual([0, 3, false, true]);
    const real = played(state, trade?.action.attackerId ?? "", trade?.action.targetId ?? "");
    expect([real.dealt, real.taken, real.popped]).toEqual([0, 3, true]);
  });

  it("R3100000 the board is lethal when the engine's blows, less the hero's Armor, finish it", () => {
    // Two Duelists hit for 4 each less Armor 2: 2 + 2.
    const lethal = laid("r3100000-lethal", { p1: [DUELIST, DUELIST], p2: [], hero: { health: 4, armor: 2 } });
    expect(lethalOnBoard(coachCtx(lethal))).toBe(true);
    expect(nextMove(coachCtx(lethal))).toMatchObject({ kind: "hero", lethal: true });
    // Played for real, the two blows end the game.
    let state = lethal;
    for (let blow = 0; blow < 2; blow += 1) {
      const hit = legalActions(state, "p1").find((each) => each.type === "attack" && each.targetId === "hero-p2");
      expect(hit).toBeDefined();
      if (hit !== undefined) state = send(state, "p1", hit);
    }
    expect(viewFor(state, "p1").result?.winner).toBe("p1");

    // One more life and the same two blows leave the hero standing.
    const short = laid("r3100000-short", { p1: [DUELIST, DUELIST], p2: [], hero: { health: 5, armor: 2 } });
    expect(lethalOnBoard(coachCtx(short))).toBe(false);
  });

  it("R3100000 the forecast reads only the board: the opponent's hidden cards change none of it", () => {
    const bare = laid("r3100000-hidden", { p1: [DUELIST], p2: [MOON] });
    // A face-down card in the opponent's backrow and a card in its hand.
    let hidden = sandbox(bare, { kind: "spawn", player: "p2", defId: "core-067", to: { zone: "hand" } });
    hidden = sandbox(hidden, { kind: "spawn", player: "p2", defId: "core-041", to: { zone: "field", row: "backrow", lane: 3 } });
    expect(combatForecast(hidden, "p1")).toEqual(combatForecast(bare, "p1"));
  });
});
