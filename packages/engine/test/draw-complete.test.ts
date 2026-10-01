// The draw-complete point of a cast-on-draw draw (SPEC §2.4, R58, R70; Classic #9 Income Tax's shape).
//
// R58: a cast-on-draw card is cast as it is drawn, and the draw is complete once that cast has
// resolved, so a trap or trigger answering the draw answers it after the cast. The cast's own windows
// (its announce, R448, and its step 4) offer every event so far to the traps, which used to hand them
// the `drawn` that found the card mid-cast; the draw now holds that event back until its cast's
// pipeline has finished, across a question the cast asks too.

import type { Action, CardDef, GameEvent } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { registerCatalog, registeredCatalog } from "../src/catalog";
import { HERO_HEALTH } from "../src/config";
import { draw } from "../src/draw";
import { damage } from "../src/effects";
import { reduce } from "../src/reduce";
import { hashState } from "../src/replay";
import type { CardScripts, Script } from "../src/script";
import { registerScripts, registeredScripts } from "../src/scripts";
import { newInstance, type CardInstance, type GameState } from "../src/state";
import { settle } from "../src/triggers";
import { newGame, put, sinkFor, slot } from "./fixtures/harness";

let nextIndex = 5600;

function def(name: string, type: CardDef["type"]): CardDef {
  nextIndex += 1;
  return {
    id: `dc-${name}`,
    index: String(nextIndex),
    name: `${name} (draw complete)`,
    set: "Core",
    type,
    tags: [],
    rarity: "Common",
    token: false,
    cost: 1,
    base: { keywords: [], text: name },
    radiant: { keywords: [], text: name },
  };
}

/** A cast-on-draw Spell whose Cry hits the enemy hero for 1. */
const bolt = def("bolt", "Spell");
/** A cast-on-draw Spell that declares a target, so its cast asks its caster (R70, R81). */
const aimed = def("aimed", "Spell");
/** p2's trap on the opponent's draws (Classic #9's moment): it hits the drawing player's hero for 2. */
const taxTrap = def("tax", "Trap");
/** p2's Field Spell with an ordinary trigger on the opponent's draws: it hits that hero for 3. */
const watcher = def("watcher", "Field Spell");

function both(script: Script): CardScripts {
  return { base: script, radiant: script };
}

const opponentsDraw = (ctx: { event: GameEvent; controller: string }): boolean =>
  ctx.event.type === "drawn" && ctx.event.player !== ctx.controller;

const SCRIPTS: Record<string, CardScripts> = {
  [bolt.id]: both({ staticFlags: { castOnDraw: true }, cry: () => [damage({ to: { of: "enemyHero" }, amount: 1 })] }),
  [aimed.id]: both({
    staticFlags: { castOnDraw: true },
    targets: [{ kind: "target", min: 1, max: 1, filter: { of: ["hero"] } }],
    cry: () => [damage({ to: { of: "chosen" }, amount: 1 })],
  }),
  [taxTrap.id]: both({
    triggers: [{ id: "tax", on: ["drawn"], when: opponentsDraw, run: () => [damage({ to: { of: "enemyHero" }, amount: 2 })] }],
  }),
  [watcher.id]: both({
    triggers: [
      {
        id: "watch",
        on: ["drawn"],
        run: (ctx) => (opponentsDraw(ctx) ? [damage({ to: { of: "enemyHero" }, amount: 3 })] : []),
      },
    ],
  }),
};

function game(seed: string): GameState {
  const state = newGame(seed);
  registerCatalog({ ...registeredCatalog(), ...Object.fromEntries([bolt, aimed, taxTrap, watcher].map((d) => [d.id, d])) });
  registerScripts({ ...registeredScripts(), ...SCRIPTS });
  state.turn = 3;
  state.phase = "main";
  return state;
}

function onTop(state: GameState, defId: string): CardInstance {
  const card = newInstance(state, defId, "p1", { z: "library", player: "p1" });
  state.players.p1.library.unshift(card);
  return card;
}

function indexOf(events: readonly GameEvent[], match: (event: GameEvent) => boolean): number {
  return events.findIndex(match);
}

describe("R58 a draw that casts is complete once its cast has resolved", () => {
  it("R58 a trap answering a cast-on-draw draw fires after the cast resolves, not inside it", () => {
    const state = game("r58-complete");
    const trap = put(state, taxTrap.id, slot("p2", "backrow", 1));
    const card = onTop(state, bolt.id);

    const sink = sinkFor(state);
    draw(sink, "p1", 1);
    settle(sink);

    const events = sink.events;
    const resolved = indexOf(events, (event) => event.type === "cardResolved" && event.instanceId === card.id);
    const fired = indexOf(events, (event) => event.type === "trapFired" && event.instanceId === trap.id);
    expect(resolved).toBeGreaterThanOrEqual(0);
    expect(fired).toBeGreaterThan(resolved);
    // Both happened: the cast's 1 to p2, the trap's 2 to p1.
    expect(state.players.p2.hero.health).toBe(HERO_HEALTH - 1);
    expect(state.players.p1.hero.health).toBe(HERO_HEALTH - 2);
    expect(state.heldDraws).toBeUndefined();
  });

  it("R58 an ordinary trigger on the draw waits for the cast too", () => {
    const state = game("r58-trigger");
    put(state, watcher.id, slot("p2", "backrow", 1));
    const card = onTop(state, bolt.id);

    const sink = sinkFor(state);
    draw(sink, "p1", 1);
    settle(sink);

    const resolved = indexOf(sink.events, (event) => event.type === "cardResolved" && event.instanceId === card.id);
    const hit = indexOf(sink.events, (event) => event.type === "damage" && event.targetId === "hero-p1");
    expect(hit).toBeGreaterThan(resolved);
    // The draw repeats once the cast has resolved (R58), so the watcher answers both draws.
    expect(sink.events.filter((event) => event.type === "drawn")).toHaveLength(2);
    expect(state.players.p1.hero.health).toBe(HERO_HEALTH - 6);
  });

  it("R58 a plain draw is answered at once, as before", () => {
    const state = game("r58-plain");
    const trap = put(state, taxTrap.id, slot("p2", "backrow", 1));
    onTop(state, "fx-1");
    const sink = sinkFor(state);
    draw(sink, "p1", 1);
    settle(sink);
    expect(sink.events.some((event) => event.type === "trapFired" && event.instanceId === trap.id)).toBe(true);
    expect(state.heldDraws).toBeUndefined();
  });

  it("R58 a cast that asks keeps its draw held across the answer, JSON round trip included", () => {
    const state = game("r58-pause");
    const trap = put(state, taxTrap.id, slot("p2", "backrow", 1));
    const card = onTop(state, aimed.id);

    const sink = sinkFor(state);
    draw(sink, "p1", 1);
    settle(sink);
    expect(state.pending?.playerId).toBe("p1");
    expect(state.heldDraws).toEqual([card.id]);
    expect(sink.events.some((event) => event.type === "trapFired")).toBe(false);

    const round = JSON.parse(JSON.stringify(state)) as GameState;
    expect(hashState(round)).toBe(hashState(state));
    const answer = {
      type: "answer",
      choiceId: state.pending?.id ?? "",
      selection: [{ pick: "hero", player: "p2" }],
      playerId: "p1",
      nonce: "r58-answer",
    } as Action;
    const live = reduce(state, answer);
    const revived = reduce(round, answer);
    expect(live.error).toBeUndefined();
    expect(hashState(revived.state)).toBe(hashState(live.state));

    const resolved = indexOf(live.events, (event) => event.type === "cardResolved" && event.instanceId === card.id);
    const fired = indexOf(live.events, (event) => event.type === "trapFired" && event.instanceId === trap.id);
    expect(resolved).toBeGreaterThanOrEqual(0);
    expect(fired).toBeGreaterThan(resolved);
    expect(live.state.players.p1.hero.health).toBe(HERO_HEALTH - 2);
    expect(live.state.heldDraws).toBeUndefined();
  });
});
