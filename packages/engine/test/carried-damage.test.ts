// R418, R446: a Unit an Ivory Tower carries is a Unit for every rule, so a hit lands on it — targeted
// or swept — as on any acting Unit (`zones.actsOnField`, which knows carried Units; the damage
// pipeline asks it rather than "the top of its unit zone"). Fixtures: `fixtures/field.ts`.

import { describe, expect, it } from "vitest";
import { dealDamage } from "../src/damage";
import { damageAll } from "../src/effects/damage";
import { unitView } from "../src/layers";
import { makeContext } from "../src/resolve";
import { findInstance, type CardInstance, type GameState } from "../src/state";
import { plain } from "./fixtures/combat";
import { actResult, flush, playing, tower } from "./fixtures/field";
import { inHand, put, sinkFor, slot } from "./fixtures/harness";

function carried(seed: string): { state: GameState; rider: CardInstance } {
  const state = playing(seed);
  put(state, tower.id, slot("p1", "backrow", 2));
  const [card] = inHand(state, plain.id, "p1");
  if (card === undefined) throw new Error("no plain body in hand");
  flush(state, "p1");
  const result = actResult(state, { type: "play", instanceId: card.id, zone: { row: "backrow", lane: 2 }, playerId: "p1" });
  if (result.error !== undefined) throw new Error(result.error);
  const rider = findInstance(result.state, card.id);
  if (rider === undefined) throw new Error("the rider is gone");
  return { state: result.state, rider };
}

describe("R446 a carried Unit takes damage", () => {
  it("R418 a hit aimed at it lands", () => {
    const { state, rider } = carried("carried-hit");
    expect(dealDamage(sinkFor(state), { source: null, target: { kind: "unit", instance: rider }, amount: 2 })).toBe(2);
    expect(unitView(state, rider).health).toBe(unitView(state, rider).maxHealth - 2);
  });

  it("R418 a sweep over 'all Units' hits it", () => {
    const { state, rider } = carried("carried-sweep");
    const sink = sinkFor(state);
    damageAll({ amount: 1, side: "enemy" }).apply(makeContext(sink, null, { controller: "p2" }));
    expect(unitView(state, rider).health).toBe(unitView(state, rider).maxHealth - 1);
    expect(sink.events.some((event) => event.type === "damage" && event.targetId === rider.id)).toBe(true);
  });
});
