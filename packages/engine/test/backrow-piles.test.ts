// Backrow piles and carriers (docs/classic-sets.md B5 E21; R446, R447): a backrow card with Stack tops
// an occupied backrow zone, only the top acts (a face-down trap under a pile never fires, an aura under
// one is off), a pile travels whole; and a carrier (Classic+ #33 Ivory Tower) holds one Unit played on
// top of it — a Unit for every rule that can neither attack nor be attacked, stepping down into a unit
// zone when its zone stops carrying it. Pauses, a round trip, a replay and each seat's view included.
// Fixtures: `fixtures/field.ts`.

import { describe, expect, it } from "vitest";
import { attackTargets, canAttack } from "../src/combat";
import { destroyAll } from "../src/effects/destroy";
import { swapBoard } from "../src/effects/swap";
import { cardsInScope } from "../src/effects/targets";
import { legalZonesFor, playsOnStack } from "../src/playChoices";
import { legalActions } from "../src/reduce";
import { hashState } from "../src/replay";
import { makeContext } from "../src/resolve";
import { findInstance, newInstance, type CardInstance, type GameState } from "../src/state";
import { whyCannotActivate } from "../src/subsystems/heroPower";
import { settle } from "../src/triggers";
import { viewFor } from "../src/viewFor";
import {
  activeUnitsOf,
  beneathAt,
  cardAt,
  carriedAt,
  isBuried,
  isCarried,
  lockZone,
  placeOnField,
  reserveZone,
} from "../src/zones";
import { indestructible, plain, taunter } from "./fixtures/combat";
import {
  act,
  actResult,
  banner,
  cover,
  flush,
  mourner,
  notesOf,
  playing,
  tower,
  watcher,
  wrecker,
} from "./fixtures/field";
import { eventsOfType, inHand, put, sinkFor, slot } from "./fixtures/harness";
import { heroicPower } from "./fixtures/scripts";

function byId(state: GameState, id: string): CardInstance {
  const card = findInstance(state, id);
  if (card === undefined) throw new Error(`no card ${id}`);
  return card;
}

/** Stack a new card of `topDef` onto the zone `under` stands in, as a Stack play or summon would (B5 E21). */
function stackOnto(state: GameState, topDef: string, under: CardInstance): CardInstance {
  const at = under.zone;
  if (at.z !== "field") throw new Error("not on the field");
  const top = newInstance(state, topDef, at.player, { z: "hand", player: at.player });
  if (!placeOnField(state, top, slot(at.player, "backrow", at.lane), { stack: true })) throw new Error("no stack");
  return top;
}

function playFrom(
  state: GameState,
  defId: string,
  zone?: { row: "units" | "backrow"; lane: number },
): ReturnType<typeof actResult> {
  const [card] = inHand(state, defId, "p1");
  if (card === undefined) throw new Error("no card");
  flush(state, "p1");
  return actResult(state, { type: "play", instanceId: card.id, ...(zone === undefined ? {} : { zone }), playerId: "p1" });
}

describe("B5 E21 backrow piles (R447)", () => {
  it("R447 a backrow card with Stack may be played onto an occupied backrow zone; the card beneath goes dormant", () => {
    const state = playing("piles-play");
    const flag = put(state, banner.id, slot("p1", "backrow", 1));
    const [card] = inHand(state, cover.id, "p1");
    if (card === undefined) return;
    flush(state, "p1");
    expect(playsOnStack(state, card)).toBe(true);
    expect(legalZonesFor(state, "p1", card)).toContainEqual({ row: "backrow", lane: 1 });
    const next = act(state, { type: "play", instanceId: card.id, zone: { row: "backrow", lane: 1 }, playerId: "p1" });
    expect(cardAt(next, slot("p1", "backrow", 1))?.id).toBe(card.id);
    expect(beneathAt(next, slot("p1", "backrow", 1)).map((c) => c.id)).toEqual([flag.id]);
    expect(isBuried(next, byId(next, flag.id))).toBe(true);
    for (const viewer of ["p1", "p2"] as const) {
      const side = viewer === "p1" ? viewFor(next, viewer).you : viewFor(next, viewer).opponent;
      expect(side.backrow[0]).toMatchObject({ defId: cover.id, buried: 1 });
    }
    // A card without Stack still needs an empty zone.
    const [plainCard] = inHand(next, banner.id, "p1");
    if (plainCard === undefined) return;
    expect(legalZonesFor(next, "p1", plainCard)).not.toContainEqual({ row: "backrow", lane: 1 });
  });

  it("R447 an aura under a pile is off, and on again once the top leaves", () => {
    const state = playing("piles-aura");
    const flag = put(state, banner.id, slot("p1", "backrow", 2));
    const unit = put(state, plain.id, slot("p1", "units", 1));
    expect(viewFor(state, "p1").you.units[0]?.attack).toBe(5);
    const top = stackOnto(state, cover.id, flag);
    expect(viewFor(state, "p1").you.units[0]?.attack).toBe(3);
    // The top leaves: the banner resumes acting.
    const sink = sinkFor(state);
    destroyAll({ side: "self", rows: ["backrow"] }).apply(makeContext(sink, null, { controller: "p1" }));
    settle(sink);
    expect(state.players.p1.graveyard.map((c) => c.id)).toContain(top.id);
    expect(cardAt(state, slot("p1", "backrow", 2))?.id).toBe(flag.id);
    expect(viewFor(state, "p1").you.units[0]?.attack).toBe(5);
    expect(unit.zone.z).toBe("field");
  });

  it("R447 a face-down trap under a pile never fires; once uncovered it answers only what happens after", () => {
    let state = playing("piles-trap");
    state.active = "p2";
    const trap = put(state, watcher.id, slot("p2", "backrow", 3));
    const top = stackOnto(state, cover.id, trap);
    state.active = "p1";
    const first = playFrom(state, plain.id, { row: "units", lane: 1 });
    state = first.state;
    expect(eventsOfType(first.events, "trapFired")).toEqual([]);
    expect(notesOf(byId(state, trap.id))).toEqual([]);

    // Take the top off: the trap resumes, face-down, and answers the next play.
    const sink = sinkFor(state);
    destroyAll({ side: "enemy", rows: ["backrow"] }).apply(makeContext(sink, null, { controller: "p1" }));
    settle(sink);
    expect(cardAt(state, slot("p2", "backrow", 3))?.id).toBe(trap.id);
    expect(state.players.p2.graveyard.map((c) => c.id)).toContain(top.id);
    const second = playFrom(state, plain.id, { row: "units", lane: 2 });
    expect(eventsOfType(second.events, "trapFired").map((e) => e.instanceId)).toEqual([trap.id]);
  });

  it("R447 a card that resumes answers nothing of the removal that uncovered it, and what comes after (R212)", () => {
    const state = playing("piles-uncovered");
    const under = put(state, mourner.id, slot("p1", "backrow", 4));
    const top = stackOnto(state, cover.id, under);
    const unit = put(state, plain.id, slot("p2", "units", 1));
    const sink = sinkFor(state);
    destroyAll({ side: "self", rows: ["backrow"] }).apply(makeContext(sink, null, { controller: "p1" }));
    settle(sink);
    expect(cardAt(state, slot("p1", "backrow", 4))?.id).toBe(under.id);
    expect(state.players.p1.graveyard.map((c) => c.id)).toContain(top.id);
    expect(notesOf(under)).toEqual([]);
    unit.damage = 3;
    settle(sink);
    expect(notesOf(under)).toEqual(["mourned"]);
  });

  it("R447 hidden information: a face-down card beneath is a count to both seats, never an identity", () => {
    const state = playing("piles-hidden");
    const trap = put(state, watcher.id, slot("p2", "backrow", 1));
    stackOnto(state, cover.id, trap);
    const mine = viewFor(state, "p2");
    const theirs = viewFor(state, "p1");
    expect(theirs.opponent.backrow[0]).toMatchObject({ faceDown: false, defId: cover.id, buried: 1 });
    expect(mine.you.backrow[0]).toMatchObject({ defId: cover.id, buried: 1 });
    expect(JSON.stringify(theirs)).not.toContain(trap.id);
    expect(JSON.stringify(theirs)).not.toContain(watcher.id);
    // A face-down top shows its back and the count.
    const deep = put(state, watcher.id, slot("p2", "backrow", 4));
    const upper = stackOnto(state, watcher.id, deep);
    expect(viewFor(state, "p1").opponent.backrow[3]).toEqual({ faceDown: true, cost: 1, buried: 1 });
    expect(upper.id).not.toBe(deep.id);
  });

  it("R447 a Locked zone, a held zone and a zone carrying a Unit take no Stack card", () => {
    const state = playing("piles-refusals");
    put(state, banner.id, slot("p1", "backrow", 1));
    put(state, banner.id, slot("p1", "backrow", 2));
    put(state, tower.id, slot("p1", "backrow", 3));
    lockZone(state, slot("p1", "backrow", 1));
    reserveZone(state, slot("p1", "backrow", 2));
    const rider = put(state, plain.id, slot("p1", "units", 1));
    state.players.p1.units[0] = null;
    expect(placeOnField(state, rider, slot("p1", "backrow", 3))).toBe(true);
    const [card] = inHand(state, cover.id, "p1");
    if (card === undefined) return;
    const zones = legalZonesFor(state, "p1", card);
    expect(zones).not.toContainEqual({ row: "backrow", lane: 1 });
    expect(zones).not.toContainEqual({ row: "backrow", lane: 2 });
    expect(zones).not.toContainEqual({ row: "backrow", lane: 3 });
    expect(zones).toContainEqual({ row: "backrow", lane: 4 });
  });

  it("R447 a board swap carries a backrow pile whole, top on top, to the other side", () => {
    const state = playing("piles-swap");
    const flag = put(state, banner.id, slot("p1", "backrow", 5));
    const top = stackOnto(state, cover.id, flag);
    const sink = sinkFor(state);
    swapBoard().apply(makeContext(sink, null, { controller: "p1" }));
    expect(cardAt(state, slot("p2", "backrow", 5))?.id).toBe(top.id);
    expect(beneathAt(state, slot("p2", "backrow", 5)).map((c) => c.id)).toEqual([flag.id]);
    expect(byId(state, flag.id).controller).toBe("p2");
    expect(state.players.p1.backrowPiles).toBeUndefined();
  });

  it("R447 a card dormant under a backrow pile does not act: a buried Heroic Power cannot be used", () => {
    const state = playing("piles-power");
    const power = put(state, heroicPower.id, slot("p1", "backrow", 3));
    power.memory.power = "ping";
    flush(state, "p1");
    expect(whyCannotActivate(state, "p1", power.id)).toBeNull();
    stackOnto(state, cover.id, power);
    expect(whyCannotActivate(state, "p1", power.id)).toBe("that card is not on the field");
    expect(legalActions(state, "p1").some((action) => action.type === "activatePower")).toBe(false);
  });

  it("R447 a JSON round trip keeps the pile, and a game with no pile carries no pile field", () => {
    const plainState = playing("piles-json-none");
    expect(plainState.players.p1.backrowPiles).toBeUndefined();
    expect(plainState.players.p1.carried).toBeUndefined();
    const state = playing("piles-json");
    const flag = put(state, banner.id, slot("p1", "backrow", 1));
    stackOnto(state, cover.id, flag);
    const round = JSON.parse(JSON.stringify(state)) as GameState;
    expect(hashState(round)).toBe(hashState(state));
    expect(isBuried(round, byId(round, flag.id))).toBe(true);
  });
});

describe("B5 E21 a carrier and the Unit it holds (R446)", () => {
  function towerGame(seed: string): { state: GameState; tower: CardInstance; rider: string } {
    const state = playing(seed);
    const holder = put(state, tower.id, slot("p1", "backrow", 2));
    const result = playFrom(state, plain.id, { row: "backrow", lane: 2 });
    if (result.error !== undefined) throw new Error(result.error);
    const rider = eventsOfType(result.events, "cardPlayed")[0]?.instanceId ?? "";
    return { state: result.state, tower: holder, rider };
  }

  it("R446 a Unit may be played on top of a carrier: offered, placed there, with the carrier acting beneath", () => {
    const state = playing("carrier-play");
    put(state, tower.id, slot("p1", "backrow", 2));
    const [card] = inHand(state, plain.id, "p1");
    if (card === undefined) return;
    flush(state, "p1");
    expect(legalZonesFor(state, "p1", card)).toContainEqual({ row: "backrow", lane: 2 });
    expect(legalActions(state, "p1")).toContainEqual({
      type: "play",
      instanceId: card.id,
      zone: { row: "backrow", lane: 2 },
    });
    const result = actResult(state, { type: "play", instanceId: card.id, zone: { row: "backrow", lane: 2 }, playerId: "p1" });
    expect(result.error).toBeUndefined();
    const next = result.state;
    expect(carriedAt(next, slot("p1", "backrow", 2))?.id).toBe(card.id);
    expect(cardAt(next, slot("p1", "backrow", 2))?.defId).toBe(tower.id);
    expect(isCarried(next, byId(next, card.id))).toBe(true);
    expect(activeUnitsOf(next, "p1").map((u) => u.id)).toContain(card.id);
    expect(eventsOfType(result.events, "summoned")).toMatchObject([{ instanceId: card.id, row: "backrow", lane: 2 }]);
    // The carrier's aura keeps working: the Unit in hand has Stack.
    const [held] = inHand(next, plain.id, "p1");
    if (held === undefined) return;
    expect(playsOnStack(next, held)).toBe(true);
    // Only a carrier's zone: a plain backrow card takes no Unit.
    put(next, banner.id, slot("p1", "backrow", 4));
    const refused = actResult(next, { type: "play", instanceId: held.id, zone: { row: "backrow", lane: 4 }, playerId: "p1" });
    expect(refused.error).toBeDefined();
  });

  it("R446 a carried Unit can neither attack nor be attacked, and its Taunt binds no attacker", () => {
    const { state, rider } = towerGame("carrier-combat");
    const unit = byId(state, rider);
    state.turn += 2;
    expect(attackTargets(state, unit)).toEqual([]);
    const enemy = put(state, plain.id, slot("p2", "units", 1));
    enemy.summonedTurn = 0;
    state.active = "p2";
    expect(canAttack(state, enemy, { kind: "unit", instance: unit })).toBe(false);
    unit.grantedKeywords.push(taunter.base.keywords[0] ?? { kind: "Taunt" });
    expect(canAttack(state, enemy, { kind: "hero", player: "p1" })).toBe(true);
    expect(attackTargets(state, enemy).map((t) => (t.kind === "hero" ? "hero" : t.instance.id))).toEqual(["hero"]);
    // It may still switch position, a unit's own action.
    state.active = "p1";
    state.phase = "main";
    expect(legalActions(state, "p1")).toContainEqual({ type: "switchPosition", instanceId: rider });
    const switched = act(state, { type: "switchPosition", instanceId: rider, playerId: "p1" });
    expect(byId(switched, rider).position).toBe("DEF");
  });

  it("R446 it is a Unit for every rule: 'all Units' reach it, and backrow effects find the carrier instead", () => {
    const { state, tower: holder, rider } = towerGame("carrier-scopes");
    const ctx = makeContext(sinkFor(state), null, { controller: "p2" });
    expect(cardsInScope(ctx, { side: "enemy" }).map((c) => c.id)).toContain(rider);
    expect(cardsInScope(ctx, { side: "enemy", rows: ["backrow"] }).map((c) => c.id)).toEqual([holder.id]);
    const view = viewFor(state, "p2").opponent;
    expect(view.carried?.[1]).toMatchObject({ instanceId: rider, defId: plain.id });
    expect(view.backrow[1]).toMatchObject({ defId: tower.id });
  });

  it("R446 when its carrier leaves, the Unit steps down into its lane's unit zone without leaving the field", () => {
    const { state, rider } = towerGame("carrier-step-down");
    const unit = byId(state, rider);
    unit.damage = 1;
    const exits = state.fieldExits?.last[rider];
    const sink = sinkFor(state);
    destroyAll({ side: "self", rows: ["backrow"] }).apply(makeContext(sink, null, { controller: "p1" }));
    settle(sink);
    expect(cardAt(state, slot("p1", "units", 2))?.id).toBe(rider);
    expect(isCarried(state, unit)).toBe(false);
    expect(unit.damage).toBe(1);
    expect(state.fieldExits?.last[rider]).toBe(exits);
    expect(eventsOfType(sink.events, "animated")).toEqual([
      { type: "animated", player: "p1", instanceId: rider, defId: plain.id, backrowLane: 2, unitLane: 2, carried: true },
    ]);
    expect(state.players.p1.carried).toBeUndefined();
  });

  it("R446 its lane taken, it goes to the leftmost open unit zone; with none it is destroyed, and an Indestructible one waits", () => {
    const { state, rider } = towerGame("carrier-no-room");
    put(state, plain.id, slot("p1", "units", 2));
    const sink = sinkFor(state);
    destroyAll({ side: "self", rows: ["backrow"] }).apply(makeContext(sink, null, { controller: "p1" }));
    settle(sink);
    expect(cardAt(state, slot("p1", "units", 1))?.id).toBe(rider);

    const full = towerGame("carrier-full");
    for (let lane = 1; lane <= 5; lane += 1) put(full.state, plain.id, slot("p1", "units", lane));
    const fullSink = sinkFor(full.state);
    destroyAll({ side: "self", rows: ["backrow"] }).apply(makeContext(fullSink, null, { controller: "p1" }));
    settle(fullSink);
    expect(full.state.players.p1.graveyard.map((c) => c.id)).toContain(full.rider);

    const tough = towerGame("carrier-indestructible");
    for (let lane = 1; lane <= 5; lane += 1) put(tough.state, plain.id, slot("p1", "units", lane));
    const body = byId(tough.state, tough.rider);
    body.defId = indestructible.id;
    const toughSink = sinkFor(tough.state);
    destroyAll({ side: "self", rows: ["backrow"] }).apply(makeContext(toughSink, null, { controller: "p1" }));
    settle(toughSink);
    expect(isCarried(tough.state, body)).toBe(true);
    // It is destroyed once, not again at every check while it waits.
    const destroyedBefore = tough.state.counters.destroyed;
    settle(toughSink);
    expect(tough.state.counters.destroyed).toBe(destroyedBefore);
    // A unit zone opens: it steps down at the next check.
    tough.state.players.p1.units[3] = null;
    settle(toughSink);
    expect(cardAt(tough.state, slot("p1", "units", 4))?.id).toBe(tough.rider);
  });

  it("R446 a Vanilla carrier carries no more: its Unit steps down", () => {
    const { state, tower: placed, rider } = towerGame("carrier-vanilla");
    const holder = byId(state, placed.id);
    holder.vanilla = true;
    const sink = sinkFor(state);
    settle(sink);
    expect(cardAt(state, slot("p1", "units", 2))?.id).toBe(rider);
    expect(cardAt(state, slot("p1", "backrow", 2))?.id).toBe(holder.id);
  });

  it("R446 a pause after the carrier left keeps the Unit carried until the list is whole, through a round trip and a replay", () => {
    const run = (seed: string, roundTrip: boolean): { paused: string; done: string } => {
      const { state, rider } = towerGame(seed);
      const cast = playFrom(state, wrecker.id);
      if (cast.error !== undefined) throw new Error(cast.error);
      let paused = cast.state;
      expect(paused.pending?.playerId).toBe("p1");
      // The Tower is marked; the check waits for the whole list, so the Unit is still where it was.
      expect(isCarried(paused, byId(paused, rider))).toBe(true);
      const pausedHash = hashState(paused);
      if (roundTrip) paused = JSON.parse(JSON.stringify(paused)) as GameState;
      const done = act(paused, {
        type: "answer",
        choiceId: paused.pending?.id ?? "",
        selection: [{ pick: "hero", player: "p2" }],
        playerId: "p1",
      });
      expect(cardAt(done, slot("p1", "units", 2))?.id).toBe(rider);
      expect(done.players.p1.graveyard.map((c) => c.defId)).toContain(tower.id);
      return { paused: pausedHash, done: hashState(done) };
    };
    const live = run("carrier-pause", false);
    expect(run("carrier-pause", true)).toEqual(live);
    expect(run("carrier-pause", false)).toEqual(live);
  });
});
