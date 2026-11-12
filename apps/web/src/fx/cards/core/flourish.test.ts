// R3402001's first beat and burst cap (fx/cards/core/flourish.ts): what Core's signatures share —
// whether an event is their play's own first beat, and the cap on the follow-up beats one entry draws.
// R2722002: a Cloaked card's face-down set (its fresh id's `formerId`, R227) is no first beat, and its
// reveal, a `trapFired` like a Trap's, is. `flourishes.test.ts` runs the three flourishes end to end;
// here the two predicates' own boundaries, which no test names today.

import { describe, expect, it } from "vitest";

import type { GameEvent } from "@jackioh/shared";

import { FX_SIGNATURE_BURSTS_MAX } from "../../constants.ts";
import type { FxMemory, FxPlay } from "../../types.ts";
import type { CardFxPlan } from "../recipe.ts";
import { isFirstBeat, withinBursts } from "./flourish.ts";
import type { AnimationEntry } from "../../../game/animations.ts";
import { baseView } from "../../../test/fixtures.ts";

function play(over: Partial<FxPlay> = {}): FxPlay {
  return { player: "p1", instanceId: "c1", defId: "core-002", castOnDraw: false, step: 1, seen: {}, ...over };
}

function casted(id = "c1"): Extract<GameEvent, { type: "cardPlayed" }> {
  return { type: "cardPlayed", player: "p1", instanceId: id, defId: "core-002", costPaid: 1 };
}

function fired(
  id = "b5",
  over: Partial<Extract<GameEvent, { type: "trapFired" }>> = {},
): Extract<GameEvent, { type: "trapFired" }> {
  return { type: "trapFired", instanceId: id, defId: "core-084", controller: "p1", row: "backrow", lane: 3, ...over };
}

function plan(events: readonly GameEvent[]): CardFxPlan {
  const memory = { resolving: () => undefined } as FxMemory;
  return {
    entry: { events } as AnimationEntry,
    view: baseView(),
    env: { intensity: 1, card: () => undefined, memory },
    D: 1000,
  };
}

describe("R3402001 the first beat", () => {
  it("is its own cardPlayed: a Spell's cast, a Field Spell's play", () => {
    expect(isFirstBeat(casted(), play())).toBe(true);
  });

  it("is never another card's cardPlayed, and never a Cloaked set's (R227)", () => {
    expect(isFirstBeat(casted("c2"), play())).toBe(false);
    expect(isFirstBeat({ ...casted(), formerId: "c0" }, play())).toBe(false);
  });

  it("is never a cardPlayed under a hook: a hook's marker opens that entry, not the play", () => {
    expect(isFirstBeat(casted(), play({ hook: "cast" }))).toBe(false);
  });

  it("is a Trap's own first trapFired, never its face-down set", () => {
    expect(isFirstBeat(fired(), play({ instanceId: "b5", hook: "cast" }))).toBe(true);
    // The set is a cardPlayed with a formerId, not a firing.
    expect(isFirstBeat({ ...casted("b5"), formerId: "b4" }, play({ instanceId: "b5", hook: "cast" }))).toBe(false);
  });

  it("R2722002 a Cloaked card's reveal fires like a Trap's, and a repeat firing is no first beat", () => {
    expect(isFirstBeat(fired("k1"), play({ instanceId: "k1", hook: "cast" }))).toBe(true);
    expect(isFirstBeat(fired("b5", { again: true }), play({ hook: "cast" }))).toBe(false);
    expect(isFirstBeat(fired(), play({ hook: undefined }))).toBe(false);
    expect(isFirstBeat(fired("b6"), play({ hook: "cast" }))).toBe(false);
  });

  it("is never another event kind", () => {
    const damage: GameEvent = { type: "damage", sourceId: null, targetId: "u1", amount: 3, combat: true };
    expect(isFirstBeat(damage, play())).toBe(false);
  });
});

describe("R3402001 the burst cap", () => {
  it(`draws the first ${FX_SIGNATURE_BURSTS_MAX} of a type and no more`, () => {
    expect(FX_SIGNATURE_BURSTS_MAX).toBe(5);
    const events: GameEvent[] = Array.from({ length: 7 }, (_, i) => ({
      type: "damage",
      sourceId: null,
      targetId: `u${String(i)}`,
      amount: 1,
      combat: true,
    }));
    const p = plan(events);
    for (const [i, event] of events.entries()) expect(withinBursts(event, p), `beat ${String(i)}`).toBe(i < 5);
  });

  it("counts each type on its own: the sixth damage fails while the sixth heal of the same entry fails too, the fifths pass", () => {
    const damages: GameEvent[] = Array.from({ length: 6 }, (_, i) => ({
      type: "damage",
      sourceId: null,
      targetId: `u${String(i)}`,
      amount: 1,
      combat: true,
    }));
    const heals: GameEvent[] = Array.from({ length: 6 }, (_, i) => ({ type: "healed", targetId: `u${String(i)}`, amount: 1 }));
    const p = plan([...damages, ...heals]);
    // Identity, not shape: the entry's own objects are the beats.
    expect(withinBursts(damages[4] as GameEvent, p)).toBe(true);
    expect(withinBursts(damages[5] as GameEvent, p)).toBe(false);
    expect(withinBursts(heals[0] as GameEvent, p)).toBe(true);
    expect(withinBursts(heals[5] as GameEvent, p)).toBe(false);
  });
});
