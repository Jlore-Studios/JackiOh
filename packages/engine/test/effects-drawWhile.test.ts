// `drawWhile` (effects/drawWhile.ts; Classic #46 Divine Favor's "draw until"): one draw at a time while
// a condition holds, read again before each draw, ending at the first draw that adds no card to the
// hand — fatigue, a burn at the hand cap, a cast on draw, a draw a draw limit stops (§2.4, R58, B5 E3)
// — or that leaves a question open.

import type { CardDef } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { registerCatalog, registeredCatalog } from "../src/catalog";
import { HAND_CAP } from "../src/config";
import { damage, drawWhile } from "../src/effects";
import { makeContext } from "../src/resolve";
import type { EffectContext, Script } from "../src/script";
import { registerScripts, registeredScripts } from "../src/scripts";
import type { GameState } from "../src/state";
import { eventsOfType, inHand, newGame, put, setLibrary, sinkFor, slot } from "./fixtures/harness";

function spell(id: string, cost = 1): CardDef {
  return {
    id: `dw-${id}`,
    index: `dw-${id}`,
    name: id,
    set: "Core",
    type: "Spell",
    tags: [],
    rarity: "Common",
    token: false,
    cost,
    base: { keywords: [], text: id },
    radiant: { keywords: [], text: id },
  };
}

const LIMITER: CardDef = { ...spell("limiter"), type: "Field Spell" };
const CAST: CardDef = spell("cast");
const PLAIN = "dw-plain";

function setup(seed: string): GameState {
  const state = newGame(seed);
  registerCatalog({
    ...registeredCatalog(),
    [LIMITER.id]: LIMITER,
    [CAST.id]: CAST,
    [PLAIN]: spell("plain"),
  });
  const scripts: Record<string, Script> = {
    [LIMITER.id]: { drawLimit: () => [{ player: "both", count: 1 }] },
    [CAST.id]: { staticFlags: { castOnDraw: true }, cry: () => [damage({ to: { of: "enemyHero" }, amount: 1 })] },
    [PLAIN]: {},
  };
  registerScripts({
    ...registeredScripts(),
    ...Object.fromEntries(Object.entries(scripts).map(([id, script]) => [id, { base: script, radiant: script }])),
  });
  state.players.p1.hand = [];
  state.players.p2.hand = [];
  // A main phase of p1's (a draw during setup is no player's turn and counts toward no limit, B5 E3).
  state.turn = 3;
  state.active = "p1";
  state.phase = "main";
  return state;
}

/** Run `drawWhile` for p1 while its hand holds fewer than `mark` cards; returns the sink's events. */
function drawTo(state: GameState, mark: number | ((ctx: EffectContext) => boolean)) {
  const sink = sinkFor(state);
  const ctx = makeContext(sink, null, { controller: "p1" });
  const more = typeof mark === "number" ? (c: EffectContext) => c.state.players.p1.hand.length < mark : mark;
  drawWhile({ more }).apply(ctx);
  return sink.events;
}

describe("drawWhile (Classic #46 Divine Favor's draw until)", () => {
  it("draws one card at a time, asking again before each draw, until the condition fails", () => {
    const state = setup("dw-basic");
    setLibrary(state, "p1", [PLAIN, PLAIN, PLAIN, PLAIN, PLAIN]);
    inHand(state, PLAIN, "p1");
    const events = drawTo(state, 4);
    expect(eventsOfType(events, "drawn")).toHaveLength(3);
    expect(state.players.p1.hand).toHaveLength(4);
    expect(state.players.p1.library).toHaveLength(2);
  });

  it("draws nothing when the condition already fails", () => {
    const state = setup("dw-none");
    setLibrary(state, "p1", [PLAIN]);
    inHand(state, PLAIN, "p1", 3);
    expect(eventsOfType(drawTo(state, 3), "drawn")).toHaveLength(0);
    expect(state.players.p1.library).toHaveLength(1);
  });

  it("a fatigue hit adds no card and ends it: one hit, not one per missing card", () => {
    const state = setup("dw-fatigue");
    setLibrary(state, "p1", [PLAIN]);
    const events = drawTo(state, 5);
    expect(eventsOfType(events, "drawn")).toHaveLength(1);
    expect(eventsOfType(events, "fatigue")).toHaveLength(1);
    expect(state.players.p1.fatigueCount).toBe(1);
  });

  it("a burn at the hand cap adds no card and ends it", () => {
    const state = setup("dw-burn");
    setLibrary(state, "p1", [PLAIN, PLAIN, PLAIN]);
    inHand(state, PLAIN, "p1", HAND_CAP);
    const events = drawTo(state, () => true);
    expect(eventsOfType(events, "burned")).toHaveLength(1);
    expect(state.players.p1.library).toHaveLength(2);
  });

  it("B5 E3 a draw a draw limit stops adds no card and ends it", () => {
    const state = setup("dw-limit");
    put(state, LIMITER.id, slot("p2", "backrow", 1));
    setLibrary(state, "p1", [PLAIN, PLAIN, PLAIN]);
    const events = drawTo(state, 3);
    expect(eventsOfType(events, "drawn")).toHaveLength(1);
    expect(eventsOfType(events, "drawLimited")).toHaveLength(1);
    expect(state.players.p1.library).toHaveLength(2);
  });

  it("R58 a card cast on draw adds no card and ends it, even when its chain repeats into a card", () => {
    const state = setup("dw-cast");
    setLibrary(state, "p1", [CAST.id, PLAIN, PLAIN, PLAIN]);
    const events = drawTo(state, 3);
    expect(eventsOfType(events, "damage").filter((event) => event.targetId === "hero-p2")).toHaveLength(1);
    // The chain's repeat drew one card (§2.4), and the draws stop there.
    expect(state.players.p1.hand).toHaveLength(1);
    expect(state.players.p1.library).toHaveLength(2);
  });
});
