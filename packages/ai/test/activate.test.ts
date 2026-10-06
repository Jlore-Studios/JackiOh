// Activate abilities in the AI's search (docs/classic-sets.md B3.2, R384). `candidateActions` puts an
// `activate` in the same tier as a play and Heroic Power's `activatePower` (R43), round-robin by source
// instance and ordered by the ability's mana price, so the beam and the lethal solver try abilities as
// early as plays. Activate N and ♾️ need nothing of their own: the AI re-plans after every action, so a
// second use is simply the next decision's candidate. Every board below is built from the real Classic
// cards (`packages/cards/src/scripts/classic/`); one test-only card carries a mana price, which no
// Classic ability has yet.

import { describe, expect, it } from "vitest";
import type { ActionBody, CardDef } from "@jackioh/shared";
import {
  catalogVersion,
  createRng,
  effects,
  registerCatalog,
  registerScripts,
  registeredCatalog,
  registeredScripts,
  subsystems,
  type GameState,
  type Script,
} from "@jackioh/engine";
import { actionKey, candidateActions, decide } from "../src/index";
import { AI, HUMAN, inGraveyard, isLegal, onField, runPuzzle, scenario, trace } from "./_support";

const PUZZLE_TIMEOUT = 120_000;

const PUNISH = "classic-020"; // Field Spell; Activate: deal 2 damage, or a discard, or a delayed destroy
const TURTINATOR = "classic-021"; // Unit 5/4; Activate ♾️: Tribute a Unit, deal its Attack to any target
const HEROIC = "core-098";

/** A test-only Field Spell token whose one ability costs (3): the price `sourceCost` orders by. */
const PRICEY = "ai-test-pricey";
const priceyDef: CardDef = {
  id: PRICEY,
  index: "ai-test-1",
  name: "Pricey Ability (AI test)",
  set: "Core",
  type: "Field Spell",
  tags: [],
  rarity: "Token",
  token: true,
  cost: 1,
  base: { keywords: [], text: "Activate: Spend (3): deal 1 damage to the enemy hero." },
  radiant: { keywords: [], text: "Activate: Spend (3): deal 1 damage to the enemy hero." },
};
const pricey: Script = {
  activations: [
    {
      id: "zap",
      label: "Deal 1 damage to the enemy hero",
      uses: 1,
      cost: { mana: 3 },
      run: () => [effects.damage({ to: { of: "enemyHero" }, amount: 1 })],
    },
  ],
};
// A token, so no deck, pool or determinization ever deals it; registered for this file only.
registerCatalog({ ...registeredCatalog(), [PRICEY]: priceyDef }, catalogVersion());
registerScripts({ ...registeredScripts(), [PRICEY]: { base: pricey, radiant: pricey } });

function indexWhere(candidates: readonly ActionBody[], test: (action: ActionBody) => boolean): number {
  return candidates.findIndex(test);
}

function sourceOf(action: ActionBody | undefined): string | null {
  if (action === undefined) return null;
  return action.type === "play" || action.type === "activate" || action.type === "activatePower" ? action.instanceId : null;
}

describe("B3.2: activations are first-class candidates", () => {
  it("B3.2: candidateActions lists activations with the plays, round-robin by source, before other attacks and switches", () => {
    // p1: Mr. Vanilla (1) in hand, Tempo Timmy on the field, The Power to Punish (free ability) in the
    // backrow. p2's Midrange Menace (9/9 Taunt) makes Timmy's only attack a losing one (the late tier).
    const state = scenario({
      seed: "activate-order",
      p1: { hand: ["core-008"], field: ["core-011"], backrow: [PUNISH] },
      p2: { field: ["core-019"], hand: ["core-005"] },
    }).state;
    const candidates = candidateActions(state, AI);
    for (const action of candidates) expect(isLegal(state, AI, action), JSON.stringify(action)).toBe(true);

    const firstActivate = indexWhere(candidates, (action) => action.type === "activate");
    const firstAttack = indexWhere(candidates, (action) => action.type === "attack");
    const firstSwitch = indexWhere(candidates, (action) => action.type === "switchPosition");
    expect(firstActivate).toBeGreaterThanOrEqual(0);
    expect(firstAttack).toBeGreaterThan(firstActivate);
    expect(firstSwitch).toBeGreaterThan(firstActivate);

    // Round-robin: Mr. Vanilla's first lane, Punish's first choice, Mr. Vanilla's second lane, …
    expect(candidates.slice(0, 3).map((action) => action.type)).toEqual(["play", "activate", "play"]);
    // Every activation sits in the plays' tier: none after the first attack or switch.
    const lastActivate = candidates.map((action) => action.type).lastIndexOf("activate");
    expect(lastActivate).toBeLessThan(firstAttack);
    expect(lastActivate).toBeLessThan(firstSwitch);
  });

  it("B3.2: a source's place in each round is its price, an ability's being its mana cost", () => {
    // Pricey's ability costs (3), Mr. Vanilla (1), Punish's ability nothing: that order, highest first.
    const s = scenario({
      seed: "activate-price",
      p1: { hand: ["core-008"], backrow: [PRICEY, PUNISH] },
      p2: { hand: ["core-005"] },
    });
    const pricey = s.backrow(AI, 1);
    const punish = s.backrow(AI, 2);
    const vanilla = s.hand(AI)[0];
    if (pricey === null || punish === null || vanilla === undefined) throw new Error("setup");
    expect(subsystems.abilitiesOf(s.state, pricey)[0]?.cost?.mana).toBe(3);

    const candidates = candidateActions(s.state, AI);
    expect(candidates.slice(0, 3).map(sourceOf)).toEqual([pricey.id, vanilla.id, punish.id]);
  });

  it("B3.2: the AI spends a free ability on the kill it is clearly best for", { timeout: PUZZLE_TIMEOUT }, () => {
    // Punish's 2 damage kills p2's Pointmaster (7/1, First Strike), which nothing else of p1's can.
    const run = runPuzzle("activate-punish", {
      p1: { hand: ["core-008"], backrow: [PUNISH] },
      p2: { field: ["core-020"], hand: ["core-005"] },
    });
    const activations = run.turn.actions.filter((action) => action.type === "activate");
    expect(activations.length, trace(run.turn)).toBe(1);
    expect(inGraveyard(run.end, HUMAN, "core-020"), trace(run.turn)).toBe(true);
    expect(run.turn.decisions.every((decision) => decision.reason !== "fallback")).toBe(true);
  });

  it("B3.2: the lethal solver finds a lethal that only an activation reaches", { timeout: PUZZLE_TIMEOUT }, () => {
    // p2 stands at 5 behind Midrange Menace (9/9 Taunt), so no attack reaches the face; a Turtinator
    // tributes the other Turtinator (never itself, R683) and deals its 5 Attack to p2's hero.
    const state = scenario({
      seed: "activate-lethal",
      p1: { field: [TURTINATOR, TURTINATOR] },
      p2: { field: ["core-019"], health: 5, hand: ["core-005"] },
    }).state;
    const decision = decide(state, AI, { rng: createRng("activate-lethal") });
    expect(decision?.reason).toBe("lethal");
    expect(decision?.action.type).toBe("activate");
    expect(decision?.line.some((action) => action.type === "attack")).toBe(false);

    const run = runPuzzle("activate-lethal", {
      p1: { field: [TURTINATOR, TURTINATOR] },
      p2: { field: ["core-019"], health: 5, hand: ["core-005"] },
    });
    expect(run.end.result?.winner, trace(run.turn)).toBe(AI);
  });

  it("B3.2: an Activate ♾️ ability is used again in the same turn while each use is good, and the turn still ends", { timeout: PUZZLE_TIMEOUT }, () => {
    // Two Pointmasters (7/1, First Strike) face Turtinator, Gary the Gambler and Jewelosco Scarab
    // (both 1/1). A 1/1 attacking a Pointmaster dies to First Strike first; tributing it to
    // Turtinator pings one dead. Two uses, then the turn ends. Both libraries hold cards, so no
    // fatigue race (R82's auto-ended turns of an empty side) makes throwing Turtinator away a win.
    const run = runPuzzle("activate-infinite", {
      p1: { field: [TURTINATOR, "core-004", "core-007"], library: ["core-053", "core-030", "core-037"] },
      p2: { field: ["core-020", "core-020"], hand: ["core-005"], library: ["core-053", "core-030", "core-037"] },
    });
    const turtinator = run.start.players.p1.units[0]?.[0];
    if (turtinator === undefined) throw new Error("setup");
    const uses = run.turn.actions.filter((action) => action.type === "activate" && action.instanceId === turtinator.id);
    expect(uses.length, trace(run.turn)).toBeGreaterThanOrEqual(2);
    expect(run.end.players.p2.graveyard.filter((card) => card.defId === "core-020"), trace(run.turn)).toHaveLength(2);
    // The turn ended: p2's turn has begun (or the game is over), well inside playAiTurn's ceiling.
    expect(run.end.result !== null || run.end.turn > run.start.turn, trace(run.turn)).toBe(true);
    expect(onField(run.end, AI, TURTINATOR), trace(run.turn)).toBe(true);
  });
});

describe("B3.2 rule 10: Heroic Power's power is an Activate the AI decides like any other (R752)", () => {
  function heroicBoard(seed: string, p2Health: number): GameState {
    const s = scenario({
      seed,
      p1: { backrow: [HEROIC], hand: ["core-053"] },
      p2: { health: p2Health, hand: ["core-005"] },
    });
    const power = s.backrow(AI, 1);
    if (power === null) throw new Error("setup");
    // R754: the rolled power lives on the instance; "burn" is Steady Shot, 2 to the enemy hero for (1).
    power.memory[subsystems.POWER_KEY] = "burn";
    return s.state;
  }

  it("R752 the power is listed in the plays' tier as its activate, and is the lethal when it is one", { timeout: PUZZLE_TIMEOUT }, () => {
    const state = heroicBoard("activate-heroic", 2);
    const power = state.players.p1.backrow[0];
    const reno = state.players.p1.hand[0];
    if (power === null || power === undefined || reno === undefined) throw new Error("setup");
    const shot: ActionBody = { type: "activate", instanceId: power.id, ability: "burn" };
    // Round-robin as ever: Reno's first lane (3), the power (its X, 1), Reno's second lane.
    const candidates = candidateActions(state, AI);
    expect(candidates.slice(0, 3).map(sourceOf)).toEqual([reno.id, power.id, reno.id]);
    expect(actionKey(candidates[1] as ActionBody)).toBe(actionKey(shot));
    // R752: the alias is no longer listed; the power is the card's Activate ability.
    expect(candidates.some((action) => action.type === "activatePower")).toBe(false);

    // The solver's first lethal may play Reno first; the power is in it either way, and legal now.
    const decision = decide(state, AI, { rng: createRng("activate-heroic") });
    expect(decision?.reason).toBe("lethal");
    expect(decision?.line.map(actionKey)).toContain(actionKey(shot));
    expect(isLegal(state, AI, shot)).toBe(true);
  });
});
