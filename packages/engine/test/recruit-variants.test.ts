// The Recruit extensions of patch v0.2.0 (docs/classic-sets.md B5 E25; §6.3 Recruit, R11, R12, R64,
// R218): from the opponent's exile newest first under your control, N at once, with filters, and
// "your entire deck" — each permanent until its row is full, Spells staying.

import { describe, expect, it } from "vitest";
import { forcedAttacks, recruit, recruitAll } from "../src/effects";
import { hashState } from "../src/replay";
import { makeContext, type EngineSink } from "../src/resolve";
import type { Effect } from "../src/script";
import { findInstance, newInstance, type CardInstance, type GameState } from "../src/state";
import { viewFor } from "../src/viewFor";
import { moveToZone } from "../src/zones";
import {
  act,
  answer,
  asker,
  askerAnswers,
  body,
  cheapUnit,
  deckSpell,
  fieldTrap,
  frozen,
  handCard,
  pileOn,
  plainTrap,
  playing,
  pricyUnit,
  replayed,
} from "./fixtures/generation";
import { eventsOfType, put, setLibrary, sinkFor, slot } from "./fixtures/harness";

function run(sink: EngineSink, effects: readonly Effect[]): void {
  const ctx = makeContext(sink, null, { controller: "p1" });
  for (const effect of effects) effect.apply(ctx);
}

/** The ids of a pile, read now (a Recruit splices the very array `setLibrary` returned). */
function ids(cards: readonly CardInstance[]): string[] {
  return cards.map((card) => card.id);
}

function must<T>(value: T | null | undefined, what: string): T {
  if (value === null || value === undefined) throw new Error(`expected ${what}`);
  return value;
}

/** A card put into a player's exile, as the last one exiled. */
function exiled(state: GameState, defId: string, owner: "p1" | "p2"): CardInstance {
  const card = newInstance(state, defId, owner, { z: "exile", player: owner });
  state.players[owner].exile.push(card);
  return card;
}

describe("E25 Recruit from the opponent's exile (Classic #1 Radiant)", () => {
  it("R659 the newest permanent of their exile is summoned on your side, under your control and now your card", () => {
    const start = playing("recruit-exile");
    const state = start.state;
    const older = exiled(state, pricyUnit.id, "p2");
    exiled(state, deckSpell.id, "p2");
    const newest = exiled(state, cheapUnit.id, "p2");
    exiled(state, deckSpell.id, "p2"); // a Spell exiled last is passed over
    const sink = sinkFor(state);
    run(sink, [recruit({ from: "exile", whose: "enemy" })]);

    const card = must(state.players.p1.units[0]?.[0], "the recruited card");
    expect(card.id).toBe(newest.id);
    expect(card.owner).toBe("p1");
    expect(card.controller).toBe("p1");
    expect(state.players.p2.exile.map((entry) => entry.id)).not.toContain(newest.id);
    expect(state.players.p2.exile.map((entry) => entry.id)).toContain(older.id);
    expect(eventsOfType(sink.events, "summoned")).toMatchObject([{ player: "p1", instanceId: newest.id }]);

    // It goes to its current owner's piles, yours, when it leaves the field (R12).
    moveToZone(state, card, "graveyard");
    expect(state.players.p1.graveyard.map((entry) => entry.id)).toContain(newest.id);
    expect(state.players.p2.graveyard.map((entry) => entry.id)).not.toContain(newest.id);
  });

  it("R53 'if it's a Unit, it attacks them at once': the recruit is one of the units this list summoned", () => {
    const start = playing("recruit-exile-attack");
    const state = start.state;
    exiled(state, cheapUnit.id, "p2");
    const before = state.players.p2.hero.health;
    run(sinkFor(state), [
      recruit({ from: "exile", whose: "enemy" }),
      forcedAttacks({ attackers: { summonedThisScript: true }, target: { spec: { of: "enemyHero" } } }),
    ]);
    expect(state.players.p2.hero.health).toBe(before - 1);
  });

  it("E25 an exile with no permanent recruits nothing", () => {
    const start = playing("recruit-exile-empty");
    const state = start.state;
    exiled(state, deckSpell.id, "p2");
    const sink = sinkFor(state);
    run(sink, [recruit({ from: "exile", whose: "enemy" })]);
    expect(sink.events).toEqual([]);
  });
});

describe("E25 Recruit N, with filters (Classic #31, #65)", () => {
  it("R64 N scans, each the first matching permanent from the top: Cost (2) or less Units, three of them", () => {
    const start = playing("recruit-count");
    const state = start.state;
    const library = ids(setLibrary(state, "p1", [pricyUnit.id, cheapUnit.id, deckSpell.id, body.id, plainTrap.id, cheapUnit.id]));
    run(sinkFor(state), [recruit({ count: 3, filter: { type: "Unit", costRange: { max: 2 } } })]);
    const units = state.players.p1.units.flatMap((pile) => (pile === null ? [] : [pile[0]?.id]));
    expect(units).toEqual([library[1], library[3], library[5]]);
    expect(ids(state.players.p1.library)).toEqual([library[0], library[2], library[4]]);
  });

  it("R64 a scan whose card finds no zone fizzles, and the next finds the same card (Core #69's shape)", () => {
    const start = playing("recruit-count-full");
    const state = start.state;
    for (let lane = 1; lane <= 5; lane += 1) put(state, body.id, slot("p1", "units", lane));
    const library = ids(setLibrary(state, "p1", [cheapUnit.id, plainTrap.id]));
    const sink = sinkFor(state);
    run(sink, [recruit({ count: 2 })]);
    expect(eventsOfType(sink.events, "summoned")).toEqual([]);
    expect(ids(state.players.p1.library)).toEqual(library);
  });
});

describe("E25 Recruit your entire deck (Classic #60)", () => {
  it("R64 top down, each permanent while its row has room — Units to units, the rest to the backrow, Traps face-down; Spells stay", () => {
    const start = playing("recruit-all");
    const state = start.state;
    for (let lane = 1; lane <= 3; lane += 1) put(state, body.id, slot("p1", "units", lane));
    const library = ids(setLibrary(state, "p1", [
      cheapUnit.id,
      deckSpell.id,
      plainTrap.id,
      pricyUnit.id,
      body.id, // the unit row is full by now: this one stays
      fieldTrap.id,
    ]));
    const spell = handCard(state, pileOn.id);
    state.players.p1.mana.current = 5;
    let run1 = frozen(start);
    run1 = act(run1, { type: "play", instanceId: spell.id, playerId: "p1" });
    const after = run1.state.players.p1;
    expect(after.units.map((pile) => pile?.[0]?.defId ?? null)).toEqual([body.id, body.id, body.id, cheapUnit.id, pricyUnit.id]);
    const traps = after.backrow.flatMap((card) => (card === null ? [] : [card]));
    expect(traps.map((card) => card.defId)).toEqual([plainTrap.id, fieldTrap.id]);
    expect(traps.every((card) => card.faceUp !== true)).toBe(true);
    expect(ids(after.library)).toEqual([library[1], library[4]]);
    expect(hashState(replayed(run1))).toBe(hashState(run1.state));
    // A Trap it set is face-down to the other player (R33, R227).
    expect(viewFor(run1.state, "p2").opponent.backrow.filter((zone) => zone !== null)).toEqual([
      { faceDown: true, cost: 1 },
      { faceDown: true, cost: 1 },
    ]);
  });

  it("R113 a card that asks a question as it arrives pauses the rest, which resumes over the same cards after the answer", () => {
    const start = playing("recruit-all-pause");
    const state = start.state;
    const library = ids(setLibrary(state, "p1", [asker.id, cheapUnit.id, deckSpell.id, body.id]));
    const spell = handCard(state, pileOn.id);
    state.players.p1.mana.current = 5;
    askerAnswers.length = 0;
    let run1 = frozen(start);
    run1 = act(run1, { type: "play", instanceId: spell.id, playerId: "p1" });

    // The Field Spell arrived and asked; the two Units behind it wait (R151, R113).
    const pending = must(run1.state.pending, "the arrival's question");
    expect(pending.playerId).toBe("p1");
    expect(run1.state.players.p1.units.every((pile) => pile === null)).toBe(true);
    const round = JSON.parse(JSON.stringify(run1.state)) as GameState;
    expect(round).toEqual(run1.state);

    const live = answer(run1, { pick: "mode", option: "right" });
    const fromJson = answer({ ...run1, state: round }, { pick: "mode", option: "right" });
    expect(hashState(fromJson.state)).toBe(hashState(live.state));
    expect(hashState(replayed(live))).toBe(hashState(live.state));
    expect(askerAnswers).toContain("right");
    expect(live.state.players.p1.units.flatMap((pile) => (pile === null ? [] : [pile[0]?.id]))).toEqual([
      library[1],
      library[3],
    ]);
    expect(ids(live.state.players.p1.library)).toEqual([library[2]]);
    expect(findInstance(live.state, must(library[0], "the asker"))?.zone).toMatchObject({ z: "field", row: "backrow" });
  });

  it("E25 recruitAll reaches the opponent's exile as well, newest first, and a filter narrows it", () => {
    const start = playing("recruit-all-exile");
    const state = start.state;
    const first = exiled(state, pricyUnit.id, "p2");
    const second = exiled(state, cheapUnit.id, "p2");
    exiled(state, deckSpell.id, "p2");
    run(sinkFor(state), [recruitAll({ from: "exile", whose: "enemy", filter: { costRange: { max: 1 } } })]);
    expect(state.players.p1.units[0]?.[0]?.id).toBe(second.id);
    expect(state.players.p1.units[1]).toBeNull();
    expect(state.players.p2.exile.map((card) => card.id)).toContain(first.id);
  });
});
