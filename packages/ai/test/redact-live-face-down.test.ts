// R602 and R403: a face-down trap with no reveal condition is live (C #88 Siphon Squad's aura
// shrinks the enemy's Attack from the moment it is set), and the units it shrinks show it to both
// players. `redact` hides the card (R185, R33), so without more the AI's determinizations would give
// its units their Attack back and it would declare attacks the engine refuses. It keeps every unit's
// shown Attack and Health instead, and the card stays a placeholder.

import { describe, expect, it } from "vitest";
import { createRng, legalActions, registeredScripts, unitView, type CardInstance, type GameState } from "@jackioh/engine";
import { HIDDEN_DEF_ID, actionKey, candidateActions, decide, determinize, redact } from "../src/index";
import { AI, HUMAN, cardById, isLegal, scenario, trapPool } from "./_support";

/** p1 (the AI) swings into p2, whose face-down Siphon Squad sets p1's units' Attack to 0 (Radiant). */
function siphoned(radiant: boolean): GameState {
  return scenario({
    seed: "redact-live-face-down",
    active: AI,
    turn: 9,
    p1: { field: ["core-019", "core-011"], hand: ["core-008"], mana: 3, library: ["core-020", "core-053"] },
    p2: { backrow: [{ def: "classic-088", faceUp: false, radiant }], library: ["core-005", "core-016"] },
  }).state;
}

describe("R602 the AI's view keeps what a live face-down card visibly does", () => {
  it("R602 R403 redact keeps each unit's shown Attack and Health and still hides the card", () => {
    for (const radiant of [false, true]) {
      const state = siphoned(radiant);
      const trap = state.players[HUMAN].backrow.find((card) => card !== null) as CardInstance;
      const seen = redact(state, AI);
      expect(cardById(seen, trap.id)?.defId).toBe(HIDDEN_DEF_ID);
      for (const unit of state.players[AI].units.flatMap((pile) => (pile === null ? [] : [pile[pile.length - 1] as CardInstance]))) {
        const truth = unitView(state, unit);
        const shown = unitView(seen, cardById(seen, unit.id) as CardInstance);
        expect({ attack: shown.attack, health: shown.health }, `${unit.defId}, radiant ${String(radiant)}`).toEqual({
          attack: truth.attack,
          health: truth.health,
        });
      }
    }
    // The Radiant face's "0 Attack" leaves the AI's units nothing to swing with, and its view says so.
    const zero = siphoned(true);
    for (const unit of zero.players[AI].units.flatMap((pile) => pile ?? [])) expect(unitView(zero, unit).attack).toBe(0);
  });

  it("R602 every move the AI's determinizations offer is legal on the true board, and so is its decision", () => {
    const state = siphoned(true);
    const legal = new Set(legalActions(state, AI).map(actionKey));
    expect([...legal].some((key) => key.includes('"attack"'))).toBe(false);
    const seen = redact(state, AI);
    for (let k = 0; k < 8; k += 1) {
      const world = determinize(seen, AI, createRng(`redact-live-face-down:${k}`));
      for (const move of candidateActions(world, AI)) expect(legal.has(actionKey(move)), actionKey(move)).toBe(true);
    }
    const decision = decide(state, AI, { rng: createRng("redact-live-face-down-decide") });
    expect(decision).not.toBeNull();
    expect(decision?.reason).not.toBe("fallback");
    expect(isLegal(state, AI, decision?.action ?? { type: "endTurn" })).toBe(true);
  });
});

describe("R602 a determinization agrees with the board it was dealt from", () => {
  /** The same board as `siphoned`, but p2's face-down trap is a plain one: no unit is shrunk. */
  function plain(): GameState {
    return scenario({
      seed: "determinize-live-face-down",
      active: AI,
      turn: 9,
      p1: { field: ["core-019", "core-011"], hand: ["core-008"], mana: 3, library: ["core-020", "core-053"] },
      p2: { backrow: [{ def: "core-041", faceUp: false }], library: ["core-005", "core-016"] },
    }).state;
  }

  const shownBy = (state: GameState): string[] =>
    state.players[AI].units.flatMap((pile) => (pile === null ? [] : [pile[pile.length - 1] as CardInstance])).map((unit) => {
      const view = unitView(state, unit);
      return `${unit.id}:${view.attack}/${view.maxHealth}`;
    });

  it("R602 never puts a hidden trap with a live aura where the board shows none", () => {
    const auras = trapPool().filter((id) => registeredScripts()[id]?.base.aura !== undefined || registeredScripts()[id]?.radiant.aura !== undefined);
    expect(auras, "the pool has a live-aura trap to rule out").toContain("classic-088");

    const state = plain();
    const seen = redact(state, AI);
    const truth = shownBy(state);
    const picked = new Set<string>();
    for (let k = 0; k < 300; k += 1) {
      const world = determinize(seen, AI, createRng(`determinize-live-face-down:${k}`));
      const hidden = world.players[HUMAN].backrow.find((card) => card !== null) as CardInstance;
      picked.add(hidden.defId);
      expect(shownBy(world), `seed ${k}, ${hidden.defId}`).toEqual(truth);
    }
    for (const id of auras) expect(picked.has(id), id).toBe(false);
    // The rest of the pool is still sampled: only the traps the board rules out are gone.
    expect(picked.size).toBeGreaterThan(trapPool().length - auras.length - 6);
  });
});
