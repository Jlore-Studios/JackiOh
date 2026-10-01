// What `evaluate` sees of patch v0.2.0's mechanics (SPEC §9.9 "Evaluation"; docs/classic-sets.md B3.1,
// B3.3, B3.4, B5 E6, E35). Each term reads the engine's own helper — `activeBrittleCount`,
// `animatedKindOf`, `cannotAttack`, `ownCost` and the layers' `unitView` — and is a named weight in
// AI_EVAL that GREEDY_EVAL, frozen with the gates, sets to nothing. As in evaluate.test.ts, nothing
// here pins a weight's value: the tests pin orderings, and where a term is exactly one weight, that
// the weight is all it adds.

import { describe, expect, it } from "vitest";
import type { CardDef, Keyword } from "@jackioh/shared";
import {
  activeUnitsOf,
  animateCard,
  cannotAttack,
  catalogVersion,
  giveBrittleCount,
  registerCatalog,
  registerScripts,
  registeredCatalog,
  registeredScripts,
  tuningOf,
  unitView,
  type CardInstance,
  type GameState,
  type Script,
} from "@jackioh/engine";
import { AI_EVAL, GREEDY_EVAL, evaluate, faceThreat, unitWorth, type EvalWeights } from "../src/index";
import { AI, HUMAN, clone, scenario, type ScenarioOptions } from "./_support";

const TESLA = "classic-005"; // Field Trap (2), Animated, its unit face 1/4 Lifesteal
const FROSTSPATULA = "classicplus-012-8"; // Field Spell token (2), Animated on your turn, 10/3 Rush
const SAME_COST_TRAP = "core-085"; // Unlicensed Experimentation: a Trap (2) with no unit face
const SOLARIUS = "classicplus-038"; // Unit 3/2, Spell Damage +2 (Radiant +5)
const TOP_LOSER = "classicplus-019-1"; // Unit token; only its Radiant face prints Immune to Spells
const VANILLA = "core-008"; // Mr. Vanilla, 4/4, no keywords

/** A test-only 4/4 whose text bars it from attacking: E35's static flag, no keyword. */
const IDLE = "ai-test-idle";
const idleDef: CardDef = {
  id: IDLE,
  index: "ai-test-2",
  name: "Idle Wall (AI test)",
  set: "Core",
  type: "Unit",
  tags: [],
  rarity: "Token",
  token: true,
  cost: 1,
  base: { attack: 4, health: 4, keywords: [], text: "Can't attack or be attacked." },
  radiant: { attack: 8, health: 8, keywords: [], text: "Can't attack or be attacked." },
};
const idle: Script = { staticFlags: { cantAttackOrBeAttacked: true } };
// A token, so no deck, pool or determinization ever deals it; registered for this file only.
registerCatalog({ ...registeredCatalog(), [IDLE]: idleDef }, catalogVersion());
registerScripts({ ...registeredScripts(), [IDLE]: { base: idle, radiant: idle } });

function board(opts: ScenarioOptions): GameState {
  return scenario({ seed: "evaluate-v020", ...opts }).state;
}

function unitsOf(state: GameState, player: "p1" | "p2"): CardInstance[] {
  return activeUnitsOf(state, player);
}

function first<T>(list: readonly (T | null | undefined)[]): T {
  const found = list.find((item): item is T => item !== null && item !== undefined);
  if (found === undefined) throw new Error("setup: nothing there");
  return found;
}

/** A clone of `state` with `change` applied to it. */
function changed(state: GameState, change: (out: GameState) => void): GameState {
  const out = clone(state);
  change(out);
  return out;
}

function withKeywordWeight(kind: string, weight: number): EvalWeights {
  return { ...AI_EVAL, keyword: { ...AI_EVAL.keyword, [kind]: weight } };
}

describe("evaluate: patch v0.2.0's mechanics", () => {
  it("B3.3: a Brittle card is worth less the nearer it is to crumbling — on the field, in the backrow and in hand", () => {
    const base = board({
      p1: { field: [VANILLA], backrow: ["core-006"], hand: ["core-053"] },
      p2: { hand: ["core-005"] },
    });
    const picks: Record<string, (state: GameState) => CardInstance> = {
      unit: (state) => first(unitsOf(state, AI)),
      backrow: (state) => first(state.players.p1.backrow),
      hand: (state) => first(state.players.p1.hand),
    };
    for (const [where, pick] of Object.entries(picks)) {
      const brittle = (count: number) => changed(base, (out) => giveBrittleCount(out, pick(out), count));
      const one = evaluate(brittle(1), AI);
      const three = evaluate(brittle(3), AI);
      expect(one, where).toBeLessThan(three);
      expect(three, where).toBeLessThan(evaluate(base, AI));
    }
    // On the field the count is the unit's own: unitWorth carries it.
    const unit = (count: number) => {
      const out = changed(base, (state) => giveBrittleCount(state, first(unitsOf(state, AI)), count));
      return unitWorth(out, first(unitsOf(out, AI)));
    };
    expect(unit(1)).toBeLessThan(unit(3));
    expect(unit(3)).toBeLessThan(unitWorth(base, first(unitsOf(base, AI))));
  });

  it("B3.1: a readable Animated backrow card is partly a unit already, worth animatedShare of its unit face", () => {
    const noShare: EvalWeights = { ...AI_EVAL, animatedShare: 0 };
    expect(AI_EVAL.animatedShare).toBeGreaterThan(0);
    for (const defId of [TESLA, FROSTSPATULA]) {
      const state = board({ p1: { backrow: [defId], hand: ["core-053"] }, p2: { hand: ["core-005"] } });
      const card = first(state.players.p1.backrow);
      const gain = evaluate(state, AI) - evaluate(state, AI, "enemy", noShare);
      expect(gain, defId).toBeCloseTo(AI_EVAL.animatedShare * unitWorth(state, card), 10);
      expect(gain, defId).toBeGreaterThan(0);
    }
    // Tesla against a Trap of the same cost with no unit face: the same backrow value, plus the share.
    const tesla = board({ p1: { backrow: [TESLA], hand: ["core-053"] }, p2: { hand: ["core-005"] } });
    const trap = board({ p1: { backrow: [SAME_COST_TRAP], hand: ["core-053"] }, p2: { hand: ["core-005"] } });
    expect(evaluate(tesla, AI, "enemy", noShare)).toBeCloseTo(evaluate(trap, AI, "enemy", noShare), 10);
    expect(evaluate(tesla, AI)).toBeGreaterThan(evaluate(trap, AI));
  });

  it("B3.1: an animated card standing in a unit zone is a unit, counted once", () => {
    const state = changed(board({ p1: { backrow: [TESLA], hand: ["core-053"] }, p2: { hand: ["core-005"] } }), (out) => {
      const tesla = first(out.players.p1.backrow);
      expect(animateCard({ state: out, events: [] }, tesla, { position: "DEF" })).toBe(true);
    });
    const tesla = first(unitsOf(state, AI));
    expect(tesla.defId).toBe(TESLA);
    expect(state.players.p1.backrow.every((card) => card === null)).toBe(true);
    // The backrow term is gone with it: the share no longer moves the score.
    expect(evaluate(state, AI)).toBe(evaluate(state, AI, "enemy", { ...AI_EVAL, animatedShare: 0 }));
  });

  it("E6: Spell Damage is worth AI_EVAL.spellDamage per point, Radiant's +5 too", () => {
    const state = board({ p1: { field: [SOLARIUS, { def: SOLARIUS, radiant: true }] } });
    const [base, radiant] = unitsOf(state, AI);
    if (base === undefined || radiant === undefined) throw new Error("setup");
    const none: EvalWeights = { ...AI_EVAL, spellDamage: 0 };
    expect(AI_EVAL.spellDamage).toBeGreaterThan(0);
    expect(unitWorth(state, base) - unitWorth(state, base, none)).toBeCloseTo(2 * AI_EVAL.spellDamage, 10);
    expect(unitWorth(state, radiant) - unitWorth(state, radiant, none)).toBeCloseTo(5 * AI_EVAL.spellDamage, 10);
  });

  it("E35: Immune to Spells is worth its keyword weight", () => {
    const state = board({ p1: { field: [{ def: TOP_LOSER, radiant: true }, TOP_LOSER] } });
    const [radiant, base] = unitsOf(state, AI);
    if (radiant === undefined || base === undefined) throw new Error("setup");
    const weight = AI_EVAL.keyword["Immune to Spells"];
    const none = withKeywordWeight("Immune to Spells", 0);
    expect(weight).toBeGreaterThan(0);
    expect(unitWorth(state, radiant) - unitWorth(state, radiant, none)).toBeCloseTo(weight, 10);
    // The base face prints no immunity.
    expect(unitWorth(state, base)).toBe(unitWorth(state, base, none));
  });

  it("E35: a unit a status bars from attacking counts no attack and threatens nothing, exactly as Can't attack", () => {
    const barred = board({ p1: { hand: ["core-053"] }, p2: { field: [IDLE] } });
    const plain = board({ p1: { hand: ["core-053"] }, p2: { field: [VANILLA] } });
    const idleUnit = first(unitsOf(barred, HUMAN));
    const vanilla = first(unitsOf(plain, HUMAN));
    expect(cannotAttack(barred, idleUnit)).toBe(true);
    expect(unitView(barred, idleUnit).keywords).toEqual([]);

    expect(faceThreat(plain, HUMAN)).toBe(4);
    expect(faceThreat(barred, HUMAN)).toBe(0);
    expect(unitWorth(barred, idleUnit)).toBeCloseTo(unitWorth(plain, vanilla) - AI_EVAL.attack * 4, 10);

    // The same 4/4 with the "Can't attack" keyword instead: the same worth, the same threat.
    const keyworded = changed(plain, (out) => {
      const unit = first(unitsOf(out, HUMAN));
      unit.grantedKeywords = [...unit.grantedKeywords, { kind: "Can't attack" }];
    });
    expect(unitWorth(keyworded, first(unitsOf(keyworded, HUMAN)))).toBeCloseTo(unitWorth(barred, idleUnit), 10);
    expect(faceThreat(keyworded, HUMAN)).toBe(0);
  });

  it("B3.4: a Degrade or Upgrade on the field reaches the evaluation through the layers", () => {
    // Mr. Vanilla 4/4 and Midrange Menace 9/9 Taunt on p1's field.
    const base = board({ p1: { field: [VANILLA, "core-019"] }, p2: { hand: ["core-005"] } });
    const tune = (change: (vanilla: CardInstance, menace: CardInstance) => void) =>
      changed(base, (out) => {
        const [vanilla, menace] = unitsOf(out, AI);
        if (vanilla === undefined || menace === undefined) throw new Error("setup");
        change(vanilla, menace);
      });
    const shield: Keyword = { kind: "Divine Shield" };
    const degraded = tune((vanilla) => Object.assign(tuningOf(vanilla), { attack: -1, health: -3 }));
    const upgraded = tune((vanilla) => Object.assign(tuningOf(vanilla), { attack: 2, health: 2 }));
    const stripped = tune((_, menace) => Object.assign(tuningOf(menace), { removeKeywords: ["Taunt"] }));
    const shielded = tune((vanilla) => Object.assign(tuningOf(vanilla), { addKeywords: [shield] }));

    expect(unitView(degraded, first(unitsOf(degraded, AI))).attack).toBe(3);
    const score = evaluate(base, AI);
    expect(evaluate(degraded, AI)).toBeLessThan(score);
    expect(evaluate(stripped, AI)).toBeLessThan(score);
    expect(evaluate(upgraded, AI)).toBeGreaterThan(score);
    expect(evaluate(shielded, AI)).toBeGreaterThan(score);

    // A numbered keyword's step (B3.4 rule 3's X row) moves Spell Damage's worth by one point.
    const solarius = board({ p1: { field: [SOLARIUS] } });
    const stepped = changed(solarius, (out) => Object.assign(tuningOf(first(unitsOf(out, AI))), { x: { "Spell Damage": 1 } }));
    expect(unitWorth(stepped, first(unitsOf(stepped, AI))) - unitWorth(solarius, first(unitsOf(solarius, AI)))).toBeCloseTo(
      AI_EVAL.spellDamage,
      10,
    );
  });

  it("B3.4, R65: an own hand card made dearer scores lower and one made cheaper higher; X-cost and enemy cards do not move", () => {
    const hand = (costMod: number, extra: ScenarioOptions["p2"] = {}) =>
      board({ p1: { hand: [{ def: "core-053", costMod }] }, p2: { hand: ["core-005"], ...extra } });
    const plain = evaluate(hand(0), AI);
    expect(AI_EVAL.handCostDelta).toBeGreaterThan(0);
    expect(plain - evaluate(hand(1), AI)).toBeCloseTo(AI_EVAL.handCostDelta, 10);
    expect(evaluate(hand(-1), AI) - plain).toBeCloseTo(AI_EVAL.handCostDelta, 10);

    // A price floors at 0: Mr. Vanilla (1) two crystals cheaper is one crystal cheaper.
    const vanilla = (costMod: number) => evaluate(board({ p1: { hand: [{ def: VANILLA, costMod }] }, p2: { hand: ["core-005"] } }), AI);
    expect(vanilla(-2)).toBeCloseTo(vanilla(-1), 10);
    // R65: no modifier reaches an X-cost card, so its costMod is no change.
    const dividend = (costMod: number) =>
      evaluate(board({ p1: { hand: [{ def: "core-024", costMod }] }, p2: { hand: ["core-005"] } }), AI);
    expect(dividend(1)).toBe(dividend(0));
    // The opponent's hand is placeholders to the AI: its cards' costs are no term.
    const enemy = (costMod: number) => evaluate(board({ p1: { hand: ["core-053"] }, p2: { hand: [{ def: "core-005", costMod }] } }), AI);
    expect(enemy(1)).toBe(enemy(0));
  });

  it("GREEDY_EVAL gives every v0.2.0 term nothing, so the frozen baseline scores these boards as before", () => {
    const greedy = (state: GameState) => evaluate(state, AI, "enemy", GREEDY_EVAL);
    const field = board({ p1: { field: [VANILLA, SOLARIUS, { def: TOP_LOSER, radiant: true }], hand: ["core-053"] } });
    const plainField = changed(field, (out) => {
      const [, solarius, topLoser] = unitsOf(out, AI);
      if (solarius === undefined || topLoser === undefined) throw new Error("setup");
      tuningOf(solarius).removeKeywords = ["Spell Damage"];
      tuningOf(topLoser).removeKeywords = ["Immune to Spells"];
    });
    expect(greedy(field)).toBe(greedy(plainField));
    expect(evaluate(field, AI)).toBeGreaterThan(evaluate(plainField, AI));

    const brittle = changed(field, (out) => {
      giveBrittleCount(out, first(unitsOf(out, AI)), 1);
      giveBrittleCount(out, first(out.players.p1.hand), 1);
    });
    expect(greedy(brittle)).toBe(greedy(field));

    const dearer = changed(field, (out) => {
      first(out.players.p1.hand).costMod = 1;
    });
    expect(greedy(dearer)).toBe(greedy(field));

    const tesla = board({ p1: { backrow: [TESLA], hand: ["core-053"] }, p2: { hand: ["core-005"] } });
    const trap = board({ p1: { backrow: [SAME_COST_TRAP], hand: ["core-053"] }, p2: { hand: ["core-005"] } });
    expect(greedy(tesla)).toBe(greedy(trap));
  });
});
