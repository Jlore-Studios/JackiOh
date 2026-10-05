// Plague Tokens, extended (docs/classic-sets.md B5 E19, R471, R669): "Place N Plague Tokens" as N
// placements all on the one permanent a single prompt names, "Place N on X" as one placement,
// placement multipliers, the "placed on this" trigger, stats per token through the layers, removal,
// and what each player sees.

import type { GameEvent } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { castNew, consumePlague, placePlague, placePlagueEach, placePlagueRandom, placePlagueTokens, plague } from "../src/effects";
import { fuse } from "../src/subsystems/fuse";
import { unitView } from "../src/layers";
import { permanentsOnField, placePlagueOn, plagueMultiplierOf, plagueOn, plagueOnField, removePlague } from "../src/plague";
import { makeContext, type EngineSink } from "../src/resolve";
import { settle } from "../src/triggers";
import { hashState } from "../src/replay";
import type { Effect } from "../src/script";
import { newInstance, type CardInstance, type GameState } from "../src/state";
import { placeOnField } from "../src/zones";
import { HIDDEN_ID, HIDDEN_OPTION_LABEL, viewFor } from "../src/viewFor";
import {
  act,
  answer,
  bigBody,
  body,
  charger,
  crawler,
  dusting,
  frozen,
  handCard,
  outbreak,
  pick,
  plagueBook,
  playing,
  quietTrap,
  replayed,
  scatter,
  slime,
  toxins,
  type Run,
} from "./fixtures/generation";
import { eventsOfType, put, sinkFor, slot } from "./fixtures/harness";

function run(sink: EngineSink, effect: Effect, self: CardInstance | null = null, targets: CardInstance[] = []): void {
  effect.apply(
    makeContext(sink, self, { controller: "p1", targets: targets.map((card) => ({ pick: "instance", instanceId: card.id })) }),
  );
}

function placements(events: readonly GameEvent[]): { id: string; value: number; placed?: number }[] {
  return eventsOfType(events, "counterChanged")
    .filter((event) => event.counter === "plague")
    .map((event) => ({ id: event.instanceId, value: event.value, ...(event.placed === undefined ? {} : { placed: event.placed }) }));
}

/** A board with a permanent in each kind of place: p1's unit, p2's unit, p2's face-down trap. */
function board(seed: string): { run: Run; mine: CardInstance; theirs: CardInstance; trap: CardInstance } {
  const started = playing(seed);
  const state = started.state;
  const mine = put(state, body.id, slot("p1", "units", 1));
  const theirs = put(state, bigBody.id, slot("p2", "units", 2));
  const trap = put(state, quietTrap.id, slot("p2", "backrow", 3));
  return { run: started, mine, theirs, trap };
}

function heroHealth(state: GameState, player: "p1" | "p2"): number {
  return state.players[player].hero.health;
}

describe("E19 Place N Plague Tokens: one prompt naming the single target (R471, R669)", () => {
  it("R669 places every token on the one permanent the single prompt names, over every permanent on either side, face-down included", () => {
    const { run: start, mine, theirs, trap } = board("plague-prompts");
    const book = handCard(start.state, plagueBook.id);
    let run = frozen(start);
    const enemyBefore = heroHealth(run.state, "p2");

    run = act(run, { type: "play", instanceId: book.id, playerId: "p1" });
    const first = run.state.pending;
    expect(first?.playerId).toBe("p1");
    expect(first?.kind).toBe("target");
    // R68's order, the placer's side first: p1's unit, then p2's unit, then p2's face-down trap.
    expect(first?.options.map((option) => option.selection)).toEqual([pick(mine), pick(theirs), pick(trap)]);
    // The rest of the Cry waits for the placements (R113).
    expect(heroHealth(run.state, "p2")).toBe(enemyBefore);

    // One answer puts both placements on the same face-down card: no second prompt opens.
    run = answer(run, pick(trap));
    // `reduce` works on a copy, so the card is read back out of the state the answer returned.
    const trapNow = (): CardInstance => run.state.players.p2.backrow[2] as CardInstance;
    expect(plagueOn(trapNow())).toBe(2);
    expect(run.state.pending).toBeNull();
    expect(heroHealth(run.state, "p2")).toBe(enemyBefore - 1);

    const events = run.state.applied.slice(-2).flatMap((entry) => entry.events);
    expect(placements(events)).toEqual([
      { id: trap.id, value: 1, placed: 1 },
      { id: trap.id, value: 2, placed: 1 },
    ]);
  });

  it("R669 a Radiant face's larger count lands on the one pick, and the paused prompt survives JSON and replays", () => {
    const { run: start, theirs } = board("plague-json");
    const book = handCard(start.state, plagueBook.id);
    book.radiant = true;
    let run = frozen(start);
    run = act(run, { type: "play", instanceId: book.id, playerId: "p1" });

    // Paused on the one prompt, the Spell's damage parked behind it (R113).
    const paused = run.state;
    const round = JSON.parse(JSON.stringify(paused)) as GameState;
    expect(round).toEqual(paused);
    expect(round.pending?.resume.data).toMatchObject({ count: 3, amount: 1 });

    const live = answer(run, pick(theirs));
    const fromJson = answer({ ...run, state: round }, pick(theirs));
    expect(hashState(fromJson.state)).toBe(hashState(live.state));
    expect(hashState(replayed(live))).toBe(hashState(live.state));
    expect(plagueOn(live.state.players.p2.units[1]?.[0] as CardInstance)).toBe(3);
    expect(plagueOn(live.state.players.p1.units[0]?.[0] as CardInstance)).toBe(0);
  });

  it("R471 with no permanent on the field nothing is asked and the rest of the list resolves at once", () => {
    const start = playing("plague-empty");
    const book = handCard(start.state, plagueBook.id);
    let run = frozen(start);
    const before = heroHealth(run.state, "p2");
    const cursor = run.state.rngCursor;
    run = act(run, { type: "play", instanceId: book.id, playerId: "p1" });
    expect(run.state.pending).toBeNull();
    expect(heroHealth(run.state, "p2")).toBe(before - 1);
    // R129: a placement with nowhere to go draws nothing.
    expect(run.state.rngCursor).toBe(cursor);
  });

  it("R471 an answer that names no offered permanent is refused, and the prompt stays", () => {
    const { run: start, mine } = board("plague-refused");
    const book = handCard(start.state, plagueBook.id);
    const inHand = handCard(start.state, body.id);
    let run = frozen(start);
    run = act(run, { type: "play", instanceId: book.id, playerId: "p1" });
    const pending = run.state.pending;
    if (pending === null) throw new Error("expected a prompt");
    const refused = run.state;
    expect(
      () => act(run, { type: "answer", choiceId: pending.id, selection: [pick(inHand)], playerId: "p1" }),
    ).toThrow(/not one of the options/);
    expect(
      () => act(run, { type: "answer", choiceId: pending.id, selection: [pick(mine)], playerId: "p2" }),
    ).toThrow(/other player/);
    expect(run.state).toBe(refused);
  });

  it("R471 the other player sees only that a prompt is open; the placer sees an enemy face-down card by id alone", () => {
    const { run: start, trap } = board("plague-view");
    const book = handCard(start.state, plagueBook.id);
    let run = frozen(start);
    run = act(run, { type: "play", instanceId: book.id, playerId: "p1" });

    expect(viewFor(run.state, "p2").pending).toEqual({ forYou: false, pendingFor: "p1" });
    const mine = viewFor(run.state, "p1").pending;
    if (mine === null || mine.forYou !== true) throw new Error("expected p1's own prompt");
    const trapOption = mine.options.find((option) => option.instanceId === trap.id);
    expect(trapOption).toEqual({ key: `instance:${trap.id}`, label: HIDDEN_OPTION_LABEL, instanceId: trap.id });
    expect(JSON.stringify(mine)).not.toContain(quietTrap.id);

    // The one answer lands both placements on the trap (R669).
    run = answer(run, pick(trap));
    // The event names the trap to its controller and hides it from the placer (R97); the count on the
    // card's back is public to both (R471).
    const placerEvents = viewFor(run.state, "p1").events.filter((event) => event.type === "counterChanged");
    const ownerEvents = viewFor(run.state, "p2").events.filter((event) => event.type === "counterChanged");
    expect(placerEvents.at(-1)).toMatchObject({ instanceId: HIDDEN_ID, value: 2, placed: 1 });
    expect(ownerEvents.at(-1)).toMatchObject({ instanceId: trap.id, value: 2, placed: 1 });
    expect(viewFor(run.state, "p1").opponent.backrow[2]).toEqual({ faceDown: true, cost: 1, plague: 2 });
    const ownerZone = viewFor(run.state, "p2").you.backrow[2];
    expect(ownerZone).toMatchObject({ faceDown: false, counters: { plague: 2 } });
  });
});

describe("E19 a placement prompt held by the player who is not taking the turn", () => {
  it("R471 the placer is the placing card's controller, whose prompt it is on either player's turn (C #90's reward D)", () => {
    const { run: start, theirs } = board("plague-off-turn");
    const state = start.state;
    const sink = sinkFor(state);
    // p2's card places on p1's turn: the prompt is p2's, and p1 cannot answer it.
    placePlagueTokens({ count: 1, amount: 2 }).apply(makeContext(sink, null, { controller: "p2" }));
    expect(state.active).toBe("p1");
    expect(state.pending?.playerId).toBe("p2");
    let run1 = frozen(start);
    expect(() => answer(run1, pick(theirs), "p1")).toThrow(/other player/);
    run1 = answer(run1, pick(theirs), "p2");
    expect(run1.state.pending).toBeNull();
    expect(plagueOn(run1.state.players.p2.units[1]?.[0] as CardInstance)).toBe(2);
    expect(hashState(replayed(run1))).toBe(hashState(run1.state));
  });
});

describe("E19 one placement, multipliers and the placed trigger (R471)", () => {
  it("R471 'Place N on X' is one placement of N on the declared card, Radiant 2", () => {
    const { run: start, theirs } = board("plague-outbreak");
    const spell = handCard(start.state, outbreak.id);
    const radiant = handCard(start.state, outbreak.id);
    radiant.radiant = true;
    let run = frozen(start);
    run = act(run, { type: "play", instanceId: spell.id, targets: [pick(theirs)], playerId: "p1" });
    run = act(run, { type: "play", instanceId: radiant.id, targets: [pick(theirs)], playerId: "p1" });
    expect(run.state.pending).toBeNull();
    const events = run.state.applied.slice(-2).flatMap((entry) => entry.events);
    expect(placements(events)).toEqual([
      { id: theirs.id, value: 1, placed: 1 },
      { id: theirs.id, value: 3, placed: 2 },
    ]);
  });

  it("R471 a placement multiplier on the card receiving it: doubled, tripled on its Radiant face", () => {
    const { run: start } = board("plague-slime");
    const state = start.state;
    const base = put(state, slime.id, slot("p1", "units", 3));
    const radiant = put(state, slime.id, slot("p1", "units", 4), { radiant: true });
    const sink = sinkFor(state);
    expect(plagueMultiplierOf(state, base)).toBe(2);
    expect(plagueMultiplierOf(state, radiant)).toBe(3);

    run(sink, placePlague({ target: { of: "chosen" }, amount: 1 }), null, [base]);
    run(sink, placePlague({ amount: 2 }), radiant);
    expect(plagueOn(base)).toBe(2);
    expect(plagueOn(radiant)).toBe(6);
    expect(placements(sink.events)).toEqual([
      { id: base.id, value: 2, placed: 2 },
      { id: radiant.id, value: 6, placed: 6 },
    ]);

    // A Vanilla slime carries no text, so nothing multiplies (§6.3 Vanilla).
    base.vanilla = true;
    expect(plagueMultiplierOf(state, base)).toBe(1);
  });

  it("R471 a fused card's multipliers multiply: a Slime fused onto a Slime quadruples", () => {
    const { run: start } = board("plague-fused-slime");
    const state = start.state;
    const kept = put(state, slime.id, slot("p1", "units", 3));
    const other = put(state, slime.id, slot("p1", "units", 4));
    const sink = sinkFor(state);
    const fused = fuse(sink, { ingredients: [other], target: kept });
    if (fused === null) throw new Error("expected a fusion");
    expect(plagueMultiplierOf(state, fused)).toBe(4);
    expect(placePlagueOn(sink, fused, 1)).toBe(4);
  });

  it("R471 'whenever Plague Tokens are placed on this' answers once per placement, never a removal", () => {
    const { run: start } = board("plague-crawler");
    const state = start.state;
    const worm = put(state, crawler.id, slot("p1", "units", 3));
    const book = handCard(state, plagueBook.id);
    const spell = handCard(state, outbreak.id);
    spell.radiant = true;
    let run = frozen(start);
    const before = heroHealth(run.state, "p2");

    // One placement of 2: one answer.
    run = act(run, { type: "play", instanceId: spell.id, targets: [pick(worm)], playerId: "p1" });
    expect(heroHealth(run.state, "p2")).toBe(before - 1);

    // Two placements on it: one answer (R669); all three triggers still wait for the whole effect and
    // the Spell (R59, R68), so the health lands where two answers put it.
    run = act(run, { type: "play", instanceId: book.id, playerId: "p1" });
    run = answer(run, pick(worm));
    expect(run.state.pending).toBeNull();
    expect(heroHealth(run.state, "p2")).toBe(before - 1 - 1 - 2);

    // Taking tokens off is no placement.
    const sink = sinkFor(run.state);
    const live = run.state.players.p1.units[2]?.[0] as CardInstance;
    expect(removePlague(sink, live, 1)).toBe(1);
    expect(placements(sink.events)).toEqual([{ id: live.id, value: 3 }]);
  });

  it("R471 every gain is a placement: Core #91's '+1 Plague Token' (`plague`) reports `placed` and is multiplied", () => {
    const { run: start } = board("plague-gain");
    const state = start.state;
    const onSlime = put(state, slime.id, slot("p1", "units", 3));
    const sink = sinkFor(state);
    run(sink, plague({ amount: 1 }), onSlime);
    run(sink, plague({ amount: -1 }), onSlime);
    expect(placements(sink.events)).toEqual([
      { id: onSlime.id, value: 2, placed: 2 },
      { id: onSlime.id, value: 1 },
    ]);
  });

  it("R471 a placement lands only on a permanent on the field: never a hand card or one dormant under a Stack (R13)", () => {
    const { run: start, mine } = board("plague-off-field");
    const state = start.state;
    const inHand = handCard(state, body.id);
    // A Stack card played on p1's lane 1, so `mine` lies dormant under it.
    const top = newInstance(state, bigBody.id, "p1", { z: "hand", player: "p1" });
    if (!placeOnField(state, top, slot("p1", "units", 1), { stack: true })) throw new Error("expected a pile");
    const sink = sinkFor(state);
    expect(placePlagueOn(sink, inHand, 1)).toBe(0);
    expect(placePlagueOn(sink, mine, 1)).toBe(0);
    expect(placePlagueOn(sink, top, 0)).toBe(0);
    expect(sink.events).toEqual([]);
    expect(permanentsOnField(state).map((card) => card.id)).not.toContain(mine.id);
  });
});

describe("E19 placements over a scope, at random, and removals", () => {
  it("R471 'on each permanent' is one placement on each, both sides, face-down included (Classic #63)", () => {
    const { run: start, mine, theirs, trap } = board("plague-each");
    const spell = handCard(start.state, dusting.id);
    let run = frozen(start);
    run = act(run, { type: "play", instanceId: spell.id, playerId: "p1" });
    const events = run.state.applied.at(-1)?.events ?? [];
    expect(placements(events)).toEqual([
      { id: mine.id, value: 1, placed: 1 },
      { id: theirs.id, value: 1, placed: 1 },
      { id: trap.id, value: 1, placed: 1 },
    ]);
    expect(plagueOnField(run.state)).toBe(3);
    expect(plagueOnField(run.state, "p2")).toBe(2);
  });

  it("R60 a random placement on N Units picks N different ones, all of them when fewer, and nothing on an empty board", () => {
    const { run: start, mine, theirs } = board("plague-random");
    const state = start.state;
    const third = put(state, body.id, slot("p2", "units", 4));
    const sink = sinkFor(state);
    run(sink, placePlagueRandom({ count: 2, amount: 1 }));
    const hit = placements(sink.events).map((entry) => entry.id);
    expect(hit).toHaveLength(2);
    expect(new Set(hit).size).toBe(2);
    expect(hit.every((id) => [mine.id, theirs.id, third.id].includes(id))).toBe(true);

    const lone = playing("plague-random-lone");
    const only = put(lone.state, body.id, slot("p2", "units", 1));
    const loneSink = sinkFor(lone.state);
    run(loneSink, placePlagueRandom({ count: 2, amount: 1 }));
    expect(placements(loneSink.events)).toEqual([{ id: only.id, value: 1, placed: 1 }]);

    const empty = playing("plague-random-empty");
    const emptySink = sinkFor(empty.state);
    const cursor = emptySink.rng.cursor;
    run(emptySink, placePlagueRandom({ count: 2, amount: 1 }));
    expect(emptySink.rng.cursor).toBe(cursor);
    expect(emptySink.events).toEqual([]);
  });

  it("R471 a random placement played as a Spell replays the same picks", () => {
    const { run: start } = board("plague-random-replay");
    put(start.state, body.id, slot("p2", "units", 4));
    const spell = handCard(start.state, scatter.id);
    let run = frozen(start);
    run = act(run, { type: "play", instanceId: spell.id, playerId: "p1" });
    expect(hashState(replayed(run))).toBe(hashState(run.state));
  });

  it("R471 consumePlague takes tokens off, floors at 0, and reports no placement; the payment helper says how many came off", () => {
    const { run: start, theirs } = board("plague-consume");
    const state = start.state;
    const sink = sinkFor(state);
    placePlagueOn(sink, theirs, 3);
    run(sink, consumePlague({ target: { of: "chosen" } }), null, [theirs]);
    expect(plagueOn(theirs)).toBe(2);
    expect(removePlague(sink, theirs, 5)).toBe(2);
    expect(theirs.counters.plague).toBeUndefined();
    expect(removePlague(sink, theirs, 1)).toBe(0);
    expect(placements(sink.events)).toEqual([
      { id: theirs.id, value: 3, placed: 3 },
      { id: theirs.id, value: 2 },
      { id: theirs.id, value: 0 },
    ]);
  });
});

describe("E19 stats per token (auras and self layers)", () => {
  it("R471 R669 an aura reading each unit's tokens buffs its side and shrinks the other: both placements land on the one pick", () => {
    const { run: start, mine, theirs } = board("plague-aura");
    const state = start.state;
    put(state, toxins.id, slot("p1", "backrow", 1));
    put(state, body.id, slot("p2", "units", 4)); // 1/1
    const book = handCard(state, plagueBook.id);
    let run = frozen(start);

    run = act(run, { type: "play", instanceId: book.id, playerId: "p1" });
    run = answer(run, pick(mine));
    expect(run.state.pending).toBeNull();
    const ally = run.state.players.p1.units[0]?.[0] as CardInstance;
    expect(unitView(run.state, ally)).toMatchObject({ attack: 3, maxHealth: 3 });
    const enemy = run.state.players.p2.units[1]?.[0] as CardInstance;
    expect(theirs.id).toBe(enemy.id);
    expect(unitView(run.state, enemy)).toMatchObject({ attack: 3, maxHealth: 5 });
  });

  it("R669 a −X/−X from the placements kills at the state check after the whole effect", () => {
    const { run: start } = board("plague-aura-death");
    const state = start.state;
    put(state, toxins.id, slot("p1", "backrow", 1));
    const frail = put(state, body.id, slot("p2", "units", 4)); // 1/1
    const book = handCard(state, plagueBook.id);
    let run = frozen(start);

    run = act(run, { type: "play", instanceId: book.id, playerId: "p1" });
    run = answer(run, pick(frail));
    // Both placements landed before any state check, so the 1/1 is gone only now that the effect ended.
    expect(run.state.pending).toBeNull();
    expect(run.state.players.p2.units[3]).toBeNull();
    expect(eventsOfType(run.state.applied.at(-1)?.events ?? [], "destroyed").map((event) => event.instanceId)).toEqual([
      frail.id,
    ]);
  });

  it("R471 a self layer reads the card's own tokens (Classic #69: +2 attack per token)", () => {
    const { run: start } = board("plague-self-layer");
    const state = start.state;
    const unit = put(state, charger.id, slot("p1", "units", 3));
    expect(unitView(state, unit).attack).toBe(4);
    placePlagueOn(sinkFor(state), unit, 2);
    expect(unitView(state, unit).attack).toBe(8);
  });
});

describe("E19 tokens spent as mana: the removal the play pipeline pays with", () => {
  it("R471 removePlague is the one way tokens come off a card, and placePlagueOn the one way they go on", () => {
    const { run: start } = board("plague-payment");
    const state = start.state;
    const field = put(state, toxins.id, slot("p1", "backrow", 2));
    const sink = sinkFor(state);
    run(sink, placePlagueEach({ scope: { side: "self", rows: ["backrow"] }, amount: 4 }));
    expect(plagueOn(field)).toBe(4);
    expect(removePlague(sink, field, 3)).toBe(3);
    expect(plagueOn(field)).toBe(1);
  });

  it("R471 a placement prompt effect with a count below 1 asks nothing", () => {
    const { run: start } = board("plague-zero");
    const sink = sinkFor(start.state);
    run(sink, placePlagueTokens({ count: 0 }));
    expect(start.state.pending).toBeNull();
    expect(sink.events).toEqual([]);
  });
});

describe("E19 placements under a random cast (B5 E12, R452)", () => {
  it("R452 R471 a random cast places each token on a random permanent itself, and asks nothing", () => {
    const { run: start } = board("plague-random-cast");
    const state = start.state;
    const sink = sinkFor(state);
    run(sink, castNew({ def: plagueBook.id, random: true }));
    settle(sink);
    expect(state.pending).toBeNull();
    expect(eventsOfType(sink.events, "promptOpened")).toEqual([]);
    // Its two placements both landed, and the rest of its text ran after them.
    expect(placements(sink.events)).toHaveLength(2);
    expect(heroHealth(state, "p2")).toBe(29);
  });

  it("R452 a cast that targets enemies places on enemy permanents whenever there is one", () => {
    for (const seed of ["plague-enemies-1", "plague-enemies-2", "plague-enemies-3"]) {
      const { run: start, theirs, trap } = board(seed);
      const sink = sinkFor(start.state);
      run(sink, castNew({ def: plagueBook.id, random: true, targetEnemies: true }));
      settle(sink);
      for (const placed of placements(sink.events)) expect([theirs.id, trap.id]).toContain(placed.id);
    }
  });
});
