// Animated (docs/classic-sets.md B3.1, R383, R445): a Field Spell, Trap or Field Trap that steps into a
// unit zone as a Unit — as the last step of a trap's firing, as an Animated Field Spell enters, and for an
// "Animated on your turn" card at its controller's start of turn and back at their cleanup — with its
// home zone held, the rule's exceptions, a pause inside the firing, a round trip, a replay, and what
// each seat's view shows. Fixtures: `fixtures/field.ts`.

import type { GameEvent } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import {
  animateAtTurnStart,
  animateCard,
  animatedKindOf,
  faceTypeOf,
  isAnimated,
  returnAtCleanup,
  returnHome,
} from "../src/animated";
import { attackTargets, canAttack } from "../src/combat";
import { steal } from "../src/effects/steal";
import { cardsInScope } from "../src/effects/targets";
import { cardTypeOf } from "../src/faces";
import { makeContext } from "../src/resolve";
import { hashState } from "../src/replay";
import { findInstance, type CardInstance, type GameState } from "../src/state";
import { isSpent } from "../src/traps";
import { runHooksInTriggerOrder, settle } from "../src/triggers";
import { viewFor, HIDDEN_ID } from "../src/viewFor";
import { activeUnitsOf, cardAt, homeOf, isReserved, lockZone, placeOnField } from "../src/zones";
import { plain, stacker } from "./fixtures/combat";
import {
  act,
  actResult,
  asker,
  banner,
  ears,
  flush,
  golem,
  listener,
  notesOf,
  playing,
  spatula,
  springer,
  tesla,
} from "./fixtures/field";
import { eventsOfType, inHand, put, sinkFor, slot } from "./fixtures/harness";

/** p1 plays a fresh plain 3/3 from hand into the unit zone of `lane`. */
function playPlain(state: GameState, lane: number): { state: GameState; events: GameEvent[]; unit: string } {
  const [card] = inHand(state, plain.id, "p1");
  if (card === undefined) throw new Error("no card");
  flush(state, "p1");
  const result = actResult(state, { type: "play", instanceId: card.id, zone: { row: "units", lane }, playerId: "p1" });
  if (result.error !== undefined) throw new Error(result.error);
  return { state: result.state, events: result.events, unit: card.id };
}

function byId(state: GameState, id: string): CardInstance {
  const card = findInstance(state, id);
  if (card === undefined) throw new Error(`no card ${id}`);
  return card;
}

function fillUnits(state: GameState, player: "p1" | "p2"): void {
  for (let lane = 1; lane <= 5; lane += 1) put(state, plain.id, slot(player, "units", lane));
}

describe("B3.1 Animated traps (R383)", () => {
  it("R383 an Animated Field Trap fires, then animates into its lane's unit zone in the position its text names", () => {
    const start = playing("animated-tesla");
    const zapper = put(start, tesla.id, slot("p2", "backrow", 2));
    const played = playPlain(start, 1);
    const state = played.state;
    const tesla2 = byId(state, zapper.id);

    // The arrival was zapped for 4 (a 3/3 dies), and then the trap stepped into p2's unit zone 2.
    expect(state.players.p1.graveyard.some((card) => card.id === played.unit)).toBe(true);
    expect(cardAt(state, slot("p2", "units", 2))?.id).toBe(zapper.id);
    expect(cardAt(state, slot("p2", "backrow", 2))).toBeNull();
    expect(tesla2.position).toBe("DEF");
    expect(tesla2.faceUp).toBe(true);
    expect(tesla2.summonedTurn).toBe(state.turn);
    expect(eventsOfType(played.events, "animated")).toEqual([
      { type: "animated", player: "p2", instanceId: zapper.id, defId: tesla.id, backrowLane: 2, unitLane: 2 },
    ]);
    // Firing ended by animating, so the trap was not consumed into the graveyard.
    expect(state.players.p2.graveyard).toEqual([]);
  });

  it("R383 an animated Field Trap keeps firing from its unit zone and, a Unit already, does not move or change position", () => {
    let state = playing("animated-tesla-turret");
    const zapper = put(state, tesla.id, slot("p2", "backrow", 2));
    state = playPlain(state, 1).state;
    byId(state, zapper.id).position = "ATK";
    const second = playPlain(state, 3);
    state = second.state;

    expect(eventsOfType(second.events, "trapFired").map((event) => event.instanceId)).toEqual([zapper.id]);
    expect(state.players.p1.graveyard.some((card) => card.id === second.unit)).toBe(true);
    // Rule 4: "A card that is already a Unit when it fires again does not move or change position."
    expect(cardAt(state, slot("p2", "units", 2))?.id).toBe(zapper.id);
    expect(byId(state, zapper.id).position).toBe("ATK");
    expect(eventsOfType(second.events, "animated")).toEqual([]);
  });

  it("R383 an animated Field Trap fires in R68's order among the unit lanes, before the backrow's traps", () => {
    let state = playing("animated-order");
    const turret = put(state, ears.id, slot("p2", "backrow", 4));
    const ear = put(state, listener.id, slot("p2", "backrow", 1));
    const first = playPlain(state, 1);
    state = first.state;
    // Both in the backrow at first: lane 1, then lane 4.
    expect(eventsOfType(first.events, "trapFired").map((event) => event.instanceId)).toEqual([ear.id, turret.id]);
    expect(cardAt(state, slot("p2", "units", 4))?.id).toBe(turret.id);
    const fired = playPlain(state, 2);
    // p2 is the opponent here: its unit lanes (the animated turret) come before its backrow (the listener).
    expect(eventsOfType(fired.events, "trapFired").map((event) => event.instanceId)).toEqual([turret.id, ear.id]);
    expect(notesOf(byId(fired.state, ear.id))).toEqual(["heard", "heard"]);
    expect(notesOf(byId(fired.state, turret.id))).toEqual(["heard", "heard"]);
  });

  it("R383 a plain Animated Trap animates on firing, is spent, and never reaches the graveyard", () => {
    let state = playing("animated-springer");
    const trap = put(state, springer.id, slot("p2", "backrow", 5));
    state = playPlain(state, 1).state;
    const card = byId(state, trap.id);
    expect(isAnimated(state, card)).toBe(true);
    expect(cardAt(state, slot("p2", "units", 5))?.id).toBe(trap.id);
    expect(isSpent(state, card)).toBe(true);
    // A Trap fires once: the next play finds it spent.
    const again = playPlain(state, 2);
    expect(eventsOfType(again.events, "trapFired")).toEqual([]);
    expect(notesOf(byId(again.state, trap.id))).toEqual(["sprang"]);
  });

  it("R383 with no open unit zone an Animated Trap stays where it is, face-up, spent, out of the graveyard", () => {
    let state = playing("animated-no-room");
    fillUnits(state, "p2");
    const trap = put(state, springer.id, slot("p2", "backrow", 3));
    const played = playPlain(state, 1);
    state = played.state;
    const card = byId(state, trap.id);
    expect(cardAt(state, slot("p2", "backrow", 3))?.id).toBe(trap.id);
    expect(card.faceUp).toBe(true);
    expect(isSpent(state, card)).toBe(true);
    expect(state.players.p2.graveyard).toEqual([]);
    expect(eventsOfType(played.events, "animated")).toEqual([]);
    // Both players read it now, as a fired Field Trap is read (R33).
    expect(viewFor(state, "p1").opponent.backrow[2]).toMatchObject({ faceDown: false, defId: springer.id });
  });

  it("R445 animating is not a summon: it emits `animated`, never `summoned`, and a Tesla does not answer it", () => {
    let state = playing("animated-not-summon");
    const zapper = put(state, tesla.id, slot("p1", "backrow", 1));
    const trap = put(state, springer.id, slot("p2", "backrow", 2));
    const played = playPlain(state, 3);
    state = played.state;
    expect(isAnimated(state, byId(state, trap.id))).toBe(true);
    const summons = eventsOfType(played.events, "summoned").map((event) => event.instanceId);
    expect(summons).toEqual([played.unit]);
    expect(summons).not.toContain(trap.id);
    // p1's Tesla watches p2's summons, and p2's trap stepping into a unit zone was none.
    expect(eventsOfType(played.events, "trapFired").map((event) => event.instanceId)).toEqual([trap.id]);
    expect(byId(state, zapper.id).faceUp).toBeUndefined();
    expect(cardAt(state, slot("p1", "backrow", 1))?.id).toBe(zapper.id);
  });
});

describe("B3.1 an animated card is a Unit for every rule (R383)", () => {
  it("R383 answers 'Unit' for its type where it stands, joins the unit walks, fights, and dies like a unit", () => {
    let state = playing("animated-unit-rules");
    const trap = put(state, springer.id, slot("p2", "backrow", 4));
    state = playPlain(state, 1).state;
    const card = byId(state, trap.id);

    // Its face is still a Trap; where it stands, it is a Unit (B2.7's reader, R383).
    expect(faceTypeOf(state, card)).toBe("Trap");
    expect(cardTypeOf(state, card)).toBe("Unit");
    expect(activeUnitsOf(state, "p2").map((unit) => unit.id)).toContain(trap.id);
    const ctx = makeContext(sinkFor(state), null, { controller: "p1" });
    expect(cardsInScope(ctx, { side: "enemy" }).map((unit) => unit.id)).toContain(trap.id);
    expect(cardsInScope(ctx, { side: "enemy", rows: ["backrow"] }).map((unit) => unit.id)).not.toContain(trap.id);

    // An enemy unit may attack it on the next turn, and the combat kills it: to the graveyard.
    state = act(state, { type: "endTurn", playerId: "p1" });
    state = act(state, { type: "endTurn", playerId: "p2" });
    const attacker = cardAt(state, slot("p1", "units", 1));
    expect(attacker).not.toBeNull();
    if (attacker === null) return;
    byId(state, trap.id).damage = 2;
    expect(canAttack(state, attacker, { kind: "unit", instance: byId(state, trap.id) })).toBe(true);
    state = act(state, { type: "attack", attackerId: attacker.id, targetId: trap.id, playerId: "p1" });
    expect(state.players.p2.graveyard.map((c) => c.id)).toContain(trap.id);
    expect(cardAt(state, slot("p2", "units", 4))).toBeNull();
  });

  it("R383 entering the unit zone is entering it on that turn: summoning sick, a fresh exertion, and it cannot attack yet", () => {
    const state = playing("animated-sick");
    const card = put(state, springer.id, slot("p1", "backrow", 2));
    card.faceUp = true;
    card.exertion = { attacked: true, switched: true };
    const sink = sinkFor(state);
    expect(animateCard(sink, card)).toBe(true);
    expect(card.summonedTurn).toBe(state.turn);
    expect(card.exertion).toEqual({ attacked: false, switched: false });
    expect(attackTargets(state, card)).toEqual([]);
  });
});

describe("B3.1 Animated Field Spells and 'Animated on your turn' (R383)", () => {
  it("R383 an Animated Field Spell animates as it enters the field, and holds no home", () => {
    const state = playing("animated-golem");
    const [card] = inHand(state, golem.id, "p1");
    if (card === undefined) return;
    flush(state, "p1");
    const result = actResult(state, { type: "play", instanceId: card.id, zone: { row: "backrow", lane: 3 }, playerId: "p1" });
    expect(result.error).toBeUndefined();
    const next = result.state;
    expect(cardAt(next, slot("p1", "units", 3))?.id).toBe(card.id);
    expect(cardAt(next, slot("p1", "backrow", 3))).toBeNull();
    expect(isReserved(next, slot("p1", "backrow", 3))).toBe(false);
    const order = result.events.filter((e) => e.type === "summoned" || e.type === "animated").map((e) => e.type);
    expect(order).toEqual(["summoned", "animated"]);
    expect(viewFor(next, "p2").opponent.units[2]).toMatchObject({ defId: golem.id, animated: {} });
  });

  it("R383 an 'on your turn' card played on its controller's turn animates at once and holds its backrow zone for its return", () => {
    const state = playing("animated-spatula-play");
    const [card] = inHand(state, spatula.id, "p1");
    if (card === undefined) return;
    flush(state, "p1");
    const next = act(state, { type: "play", instanceId: card.id, zone: { row: "backrow", lane: 2 }, playerId: "p1" });
    expect(cardAt(next, slot("p1", "units", 2))?.id).toBe(card.id);
    expect(homeOf(next, card.id)).toEqual({ instanceId: card.id, zone: { player: "p1", row: "backrow", lane: 2 } });
    expect(isReserved(next, slot("p1", "backrow", 2))).toBe(true);
    // Both seats see the held zone and the home lane on the unit.
    for (const viewer of ["p1", "p2"] as const) {
      const side = viewer === "p1" ? viewFor(next, viewer).you : viewFor(next, viewer).opponent;
      expect(side.reserved.backrow).toEqual([false, true, false, false, false]);
      expect(side.units[1]).toMatchObject({ defId: spatula.id, animated: { home: 2 } });
    }
    // Nothing else may enter the held zone.
    const [other] = inHand(next, golem.id, "p1");
    if (other === undefined) return;
    flush(next, "p1");
    const refused = actResult(next, { type: "play", instanceId: other.id, zone: { row: "backrow", lane: 2 }, playerId: "p1" });
    expect(refused.error).toBeDefined();
  });

  it("R383 its end-of-turn text runs while it is a Unit, and cleanup sends it home; its next start of turn animates it again", () => {
    const state = playing("animated-spatula-cycle");
    const card = put(state, spatula.id, slot("p1", "backrow", 4));
    const sink = sinkFor(state);
    animateAtTurnStart(sink, "p1");
    expect(cardAt(state, slot("p1", "units", 4))?.id).toBe(card.id);

    // §2.2: the end-of-turn hooks, then cleanup's return (the order `turn.ts` runs them in).
    runHooksInTriggerOrder(sink, "endOfTurn", "p1");
    returnAtCleanup(sink, "p1");
    expect(notesOf(card)).toEqual(["units"]);
    expect(cardAt(state, slot("p1", "backrow", 4))?.id).toBe(card.id);
    expect(card.position).toBeUndefined();
    expect(homeOf(state, card.id)).toBeUndefined();
    expect(eventsOfType(sink.events, "deanimated")).toEqual([
      { type: "deanimated", player: "p1", instanceId: card.id, defId: spatula.id, unitLane: 4, backrowLane: 4 },
    ]);

    // On the opponent's turn it sits in the backrow, where no attack can reach it: no unit walk finds
    // it, and a backrow effect does (rule 7).
    expect(activeUnitsOf(state, "p1").map((unit) => unit.id)).not.toContain(card.id);
    const ctx = makeContext(sink, null, { controller: "p2" });
    expect(cardsInScope(ctx, { side: "enemy" }).map((unit) => unit.id)).not.toContain(card.id);
    expect(cardsInScope(ctx, { side: "enemy", rows: ["backrow"] }).map((unit) => unit.id)).toContain(card.id);
    expect(cardTypeOf(state, card)).toBe("Field Spell");
    state.turn += 2;
    animateAtTurnStart(sink, "p1");
    expect(cardAt(state, slot("p1", "units", 4))?.id).toBe(card.id);
    expect(card.summonedTurn).toBe(state.turn);
  });

  it("R383 moving is not leaving the field: damage, buffs, counters and memory stay, and no departure is counted", () => {
    const state = playing("animated-keeps");
    const card = put(state, spatula.id, slot("p1", "backrow", 1));
    card.damage = 1;
    card.buffs = { attack: 2, health: 2 };
    card.memory.kept = "yes";
    card.counters.plague = 2;
    const sink = sinkFor(state);
    const exits = state.fieldExits?.count ?? 0;
    animateAtTurnStart(sink, "p1");
    returnAtCleanup(sink, "p1");
    animateAtTurnStart(sink, "p1");
    expect(card.damage).toBe(1);
    expect(card.buffs).toEqual({ attack: 2, health: 2 });
    expect(card.memory.kept).toBe("yes");
    expect(card.counters.plague).toBe(2);
    expect(state.fieldExits?.count ?? 0).toBe(exits);
  });

  it("R383 with no open unit zone an 'on your turn' card stays in the backrow, and a face-down one never animates at turn start", () => {
    const state = playing("animated-turn-start-limits");
    fillUnits(state, "p1");
    const card = put(state, spatula.id, slot("p1", "backrow", 1));
    const sink = sinkFor(state);
    animateAtTurnStart(sink, "p1");
    expect(cardAt(state, slot("p1", "backrow", 1))?.id).toBe(card.id);
    expect(homeOf(state, card.id)).toBeUndefined();
    expect(eventsOfType(sink.events, "animated")).toEqual([]);
  });

  it("R688 rule 6: a Lock on its home since no longer stops the return — the return is a move, not a play", () => {
    const state = playing("animated-home-locked");
    const card = put(state, spatula.id, slot("p1", "backrow", 3));
    const sink = sinkFor(state);
    animateAtTurnStart(sink, "p1");
    lockZone(state, slot("p1", "backrow", 3));
    returnAtCleanup(sink, "p1");
    expect(cardAt(state, slot("p1", "backrow", 3))?.id).toBe(card.id);
    expect(homeOf(state, card.id)).toBeUndefined();
  });

  it("R383 rule 6: a card that changed sides has no home on the new side, going to its controller's leftmost open backrow zone", () => {
    const state = playing("animated-stolen");
    const card = put(state, spatula.id, slot("p1", "backrow", 2));
    const sink = sinkFor(state);
    animateAtTurnStart(sink, "p1");
    put(state, banner.id, slot("p2", "backrow", 1));
    steal({ target: { of: "instance", instanceId: card.id } }).apply(makeContext(sink, null, { controller: "p2" }));
    expect(card.controller).toBe("p2");
    returnAtCleanup(sink, "p2");
    // p2's backrow lane 1 is taken, so lane 2; the old home on p1's side is let go.
    expect(cardAt(state, slot("p2", "backrow", 2))?.id).toBe(card.id);
    expect(isReserved(state, slot("p1", "backrow", 2))).toBe(false);
    expect(homeOf(state, card.id)).toBeUndefined();
  });

  it("R383 rule 6: a card dormant under a Stack does not return, and its home stays held", () => {
    const state = playing("animated-buried");
    const card = put(state, spatula.id, slot("p1", "backrow", 5));
    const sink = sinkFor(state);
    animateAtTurnStart(sink, "p1");
    const top = put(state, stacker.id, slot("p1", "units", 1));
    placeOnField(state, top, slot("p1", "units", 5), { stack: true });
    returnAtCleanup(sink, "p1");
    expect(state.players.p1.units[4]?.map((c) => c.id)).toEqual([top.id, card.id]);
    expect(isReserved(state, slot("p1", "backrow", 5))).toBe(true);
    expect(returnHome(sink, card)).toBe(false);
  });

  it("R383 a Vanilla card has lost Animated where it stands: it stays a Unit and its home is let go", () => {
    const state = playing("animated-vanilla");
    const card = put(state, spatula.id, slot("p1", "backrow", 1));
    const sink = sinkFor(state);
    animateAtTurnStart(sink, "p1");
    card.vanilla = true;
    expect(animatedKindOf(state, card)).toBeNull();
    returnAtCleanup(sink, "p1");
    expect(cardAt(state, slot("p1", "units", 1))?.id).toBe(card.id);
    expect(homeOf(state, card.id)).toBeUndefined();
    expect(isReserved(state, slot("p1", "backrow", 1))).toBe(false);
  });

  it("R383 a card leaving the field from its unit zone lets its home go", () => {
    const state = playing("animated-dies");
    const card = put(state, spatula.id, slot("p2", "backrow", 1));
    state.turn += 1;
    state.active = "p2";
    const sink = sinkFor(state);
    animateAtTurnStart(sink, "p2");
    card.damage = 5;
    settle(sink);
    expect(state.players.p2.graveyard.map((c) => c.id)).toContain(card.id);
    expect(homeOf(state, card.id)).toBeUndefined();
    expect(isReserved(state, slot("p2", "backrow", 1))).toBe(false);
  });
});

describe("B3.1 a pause inside an Animated trap's firing (R113, R383)", () => {
  function paused(seed: string): { state: GameState; trap: string } {
    const start = playing(seed);
    const trap = put(start, asker.id, slot("p2", "backrow", 1));
    const played = playPlain(start, 2);
    return { state: played.state, trap: trap.id };
  }

  it("R383 a trap whose list asks animates only once the answer has run the rest of it", () => {
    const { state, trap } = paused("animated-pause");
    expect(state.pending?.playerId).toBe("p2");
    expect(cardAt(state, slot("p2", "backrow", 1))?.id).toBe(trap);
    expect(state.work.length).toBeGreaterThan(0);
    const next = act(state, {
      type: "answer",
      choiceId: state.pending?.id ?? "",
      selection: [{ pick: "hero", player: "p1" }],
      playerId: "p2",
    });
    expect(notesOf(findInstance(next, trap))).toEqual(["answered", "tail"]);
    expect(cardAt(next, slot("p2", "units", 1))?.id).toBe(trap);
    expect(next.work).toEqual([]);
  });

  it("R383 the paused firing survives a JSON round trip and a replay to the same state", () => {
    const run = (seed: string, roundTrip: boolean): { pausedHash: string; doneHash: string } => {
      const { state } = paused(seed);
      const pausedHash = hashState(state);
      const from = roundTrip ? (JSON.parse(JSON.stringify(state)) as GameState) : state;
      expect(hashState(from)).toBe(pausedHash);
      const next = act(from, {
        type: "answer",
        choiceId: from.pending?.id ?? "",
        selection: [{ pick: "hero", player: "p1" }],
        playerId: "p2",
      });
      return { pausedHash, doneHash: hashState(next) };
    };
    const live = run("animated-pause-replay", false);
    const revived = run("animated-pause-replay", true);
    const replayed = run("animated-pause-replay", false);
    expect(revived).toEqual(live);
    expect(replayed).toEqual(live);
  });

  it("R383 hidden information: the opponent reads the flip's zone only, then the animated card in full; the prompt is its controller's alone", () => {
    const { state, trap } = paused("animated-pause-view");
    const opponentView = viewFor(state, "p1");
    const fired = eventsOfType(opponentView.events, "trapFired");
    expect(fired.at(-1)).toMatchObject({ instanceId: HIDDEN_ID, defId: HIDDEN_ID, row: "backrow", lane: 1 });
    expect(opponentView.pending).toEqual({ forYou: false, pendingFor: "p2" });
    expect(viewFor(state, "p2").pending).toMatchObject({ forYou: true });

    const next = act(state, {
      type: "answer",
      choiceId: state.pending?.id ?? "",
      selection: [{ pick: "hero", player: "p1" }],
      playerId: "p2",
    });
    for (const viewer of ["p1", "p2"] as const) {
      const view = viewFor(next, viewer);
      expect(eventsOfType(view.events, "animated")).toEqual([
        { type: "animated", player: "p2", instanceId: trap, defId: asker.id, backrowLane: 1, unitLane: 1 },
      ]);
      const side = viewer === "p2" ? view.you : view.opponent;
      expect(side.units[0]).toMatchObject({ instanceId: trap, defId: asker.id, animated: {} });
    }
  });
});
