// §10.6 A target prompt narrowed by a card's own condition — `chooseTargetWhere`
// (effects/chooseWhere.ts), the prompt Classic #32 Felinor Feelings' Radiant face asks after its token
// lands. The real card's test (packages/cards/test/classic/032-felinor-feelings.test.ts) covers the
// same cases again through the card.

import type { GameEvent } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { chooseTargetWhere } from "../src/effects/chooseWhere";
import { makeContext } from "../src/resolve";
import type { GameState } from "../src/state";
import { newGame, put, sinkFor, slot } from "./fixtures/harness";

function run(state: GameState, effect: ReturnType<typeof chooseTargetWhere>): GameEvent[] {
  const events: GameEvent[] = [];
  const sink = sinkFor(state, events);
  effect.apply(makeContext(sink, null, { controller: "p1" }));
  return events;
}

describe("§10.6 chooseTargetWhere (C #32 Felinor Feelings)", () => {
  it("offers the scope's cards that the condition admits, in the scope's order, to the controller", () => {
    const state = newGame("choose-where");
    const a = put(state, "fx-1", slot("p2", "units", 1));
    put(state, "fx-2", slot("p2", "units", 2));
    const c = put(state, "fx-3", slot("p2", "units", 3));

    const events = run(
      state,
      chooseTargetWhere({ step: "picked", scope: { side: "enemy", of: ["unit"] }, where: (_ctx, card) => card !== null && card.zone.z === "field" && card.zone.lane !== 2 }),
    );

    expect(state.pending?.playerId).toBe("p1");
    expect(state.pending?.kind).toBe("target");
    expect(state.pending?.options.map((option) => option.selection)).toEqual([
      { pick: "instance", instanceId: a.id },
      { pick: "instance", instanceId: c.id },
    ]);
    expect(state.pending?.options.map((option) => option.key)).toEqual([`instance:${a.id}`, `instance:${c.id}`]);
    expect(events.map((event) => event.type)).toEqual(["promptOpened"]);
  });

  it("asks nothing when the condition admits no card: the effect fizzles", () => {
    const state = newGame("choose-where-none");
    put(state, "fx-1", slot("p2", "units", 1));

    const events = run(state, chooseTargetWhere({ step: "picked", scope: { side: "enemy" }, where: () => false }));

    expect(state.pending).toBeNull();
    expect(events).toEqual([]);
  });

  it("§9.3 the open prompt is plain data: it survives a JSON round trip", () => {
    const state = newGame("choose-where-json");
    put(state, "fx-1", slot("p2", "units", 1));

    run(state, chooseTargetWhere({ step: "picked", scope: { side: "enemy" }, where: () => true }));

    expect(JSON.parse(JSON.stringify(state))).toEqual(state);
  });
});
