// R752: the AI samples an unseen face-down trap at the cost the board shows.
//
// Since R351 every face-down Trap shows its cost to both players, and `redact` keeps that number
// on the placeholder — but `determinize` filled each placeholder from every Trap and Field Trap,
// so a sampled world could put a cost-1 trap where the board shows 3. The lethal solver accepts a
// line that wins on all 3 determinizations, so it swung at the face into a shown cost-3 trap most
// of the time, where a human reading the same cost would not.

import { describe, expect, it } from "vitest";
import {
  createRng,
  defOf,
  effectiveCost,
  type CardInstance,
  type GameState,
} from "@jackioh/engine";
import { decide, determinize, redact } from "../src/index";
import { AI, HUMAN, cardById, isLegal, scenario } from "./_support";

const DOOM_SHROOM = "classicplus-001";
const GROOM_SHROOM = "classicplus-002";

function oneOfEachCost(): GameState {
  return scenario({
    seed: "shown-cost-lanes",
    active: AI,
    turn: 9,
    p1: { field: ["core-008"], library: ["core-020"] },
    p2: {
      backrow: [{ def: "core-041", faceUp: false }, { def: "classic-017", faceUp: false }, { def: DOOM_SHROOM, faceUp: false }],
      library: ["core-005"],
    },
  }).state;
}

/**
 * Vanilla 4 + Timmy 3 is exactly 7, so the two swings are lethal. Doom Shroom exiles both attackers
 * on the first swing. Groom Shroom lets the first swing hit (R405) and walls the second behind
 * Taunts. So no world at (3) has a lethal. Do not cut this to one attacker: a Groom world would
 * then still be lethal.
 */
function shroomed(trap: boolean): GameState {
  return scenario({
    seed: "shown-cost-doom",
    active: AI,
    turn: 9,
    p1: { field: ["core-008", "core-011"], library: ["core-020", "core-053"] },
    p2: {
      health: 7,
      backrow: trap ? [{ def: DOOM_SHROOM, faceUp: false }] : [],
      library: ["core-005", "core-016"],
    },
  }).state;
}

function faceDown(state: GameState): CardInstance[] {
  return state.players[HUMAN].backrow.flatMap((card) => (card === null ? [] : [card]));
}

describe("determinize at the shown cost (R752)", () => {
  it("R752 every face-down card showing a cost is sampled as a trap of that cost, over 100 seeds", () => {
    const state = oneOfEachCost();
    const lanes = faceDown(state);
    const shown = lanes.map((card) => effectiveCost(state, card));
    expect(shown).toEqual([1, 2, 3]);
    const picked = new Set<string>();
    for (let k = 0; k < 100; k += 1) {
      const world = determinize(redact(state, AI), AI, createRng(`shown-cost-lanes:${k}`));
      lanes.forEach((lane, at) => {
        const defId = cardById(world, lane.id)?.defId as string;
        expect(defOf(world, defId).cost, `seed ${k}, lane ${at + 1}: ${defId}`).toBe(shown[at]);
        picked.add(defId);
      });
    }
    expect(picked.has(DOOM_SHROOM)).toBe(true);
    expect(picked.has(GROOM_SHROOM)).toBe(true);
  });

  it("R752 the greedy baseline's sampler (matchShownCost: false) still fills a back with a trap of any cost", () => {
    const state = oneOfEachCost();
    const lanes = faceDown(state);
    const seen = redact(state, AI);
    const costs = new Set<unknown>();
    for (let k = 0; k < 100; k += 1) {
      const world = determinize(seen, AI, createRng(`shown-cost-greedy:${k}`), { matchShownCost: false });
      const defId = cardById(world, (lanes[2] as CardInstance).id)?.defId as string;
      costs.add(defOf(world, defId).cost);
    }
    expect(costs.size).toBeGreaterThan(1);
  });

  it("R752 with no face-down card the two swings are a lethal the AI takes", () => {
    expect(decide(shroomed(false), AI, { rng: createRng("shown-cost-doom:control") })?.reason).toBe("lethal");
  });

  it("R752 never returns the swing into a face-down Doom Shroom showing (3) as lethal, on AI seeds 1-20", { timeout: 60_000 }, () => {
    const state = shroomed(true);
    for (let seed = 1; seed <= 20; seed += 1) {
      const decision = decide(state, AI, { rng: createRng(`shown-cost-doom:${seed}`) });
      expect(decision, `seed ${seed}`).not.toBeNull();
      expect(decision?.reason, `seed ${seed}: ${JSON.stringify(decision?.line)}`).not.toBe("lethal");
      expect(isLegal(state, AI, decision?.action ?? { type: "endTurn" }), `seed ${seed}`).toBe(true);
    }
  });
});
