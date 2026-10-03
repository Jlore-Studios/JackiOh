// R102, §10.6: a fused card carries every ingredient's declared targets, and a declaration's filter
// may name a predicate (`TargetFilter.check`) in its card's `targetChecks`. Those are pure reads asked
// with the candidate, not hooks that return lists, so the fused card must answer them with a boolean.
//
// Found by the fuzz gate: a fused card of two ingredients that each define `targetChecks` (here Classic
// #48 Hired Shrimp and a fusion holding C+ #41 KY's Constant) made `legalActions` throw "Cannot read
// properties of undefined (reading '__partDepth')" for as long as it sat in a hand, and `reduce` with
// it, because every action ends by asking whether the turn is over (R82). The combiner wrapped each
// predicate as a fused Cry; one ingredient defining the object was never wrapped, so no test saw it.

import { describe, expect, it } from "vitest";
import { createRng, legalActions, subsystems, type CardInstance } from "@jackioh/engine";
import { scenario, type Scenario } from "./_harness";

const KYS_CONSTANT = "classicplus-041"; // (1) Spell: Cry, pick a hand card (check: "number").
const REWIND = "classic-054"; // (1) Spell: Cry, pick an ally Unit or graveyard card with a Cry (check: "hasCry").
const SHRIMP = "classic-048"; // (2) Unit with a Cry: a card Rewind's predicate admits.
const HIT_JOB = "core-016"; // a hand card for KY's Constant's pick to name.
const ANCHOR = "core-010"; // a free Spell that keeps the turn open; never played.

function craft(s: Scenario, ingredients: [CardInstance, CardInstance]): CardInstance {
  const sink = { state: s.state, events: [], rng: createRng(s.state.seed, s.state.rngCursor) };
  const fused = subsystems.fuse(sink, { ingredients, toHand: "p1" });
  if (fused === null) throw new Error("the fusion did not happen");
  return fused;
}

function playsOf(s: Scenario, card: CardInstance) {
  return legalActions(s.state, "p1").flatMap((action) =>
    action.type === "play" && action.instanceId === card.id ? [action] : [],
  );
}

describe("R102 a fused card's declared targets keep working when two ingredients define targetChecks", () => {
  it("R102 a crafted Rewind + KY's Constant offers plays that pick for both declarations, instead of throwing from legalActions", () => {
    const s = scenario({
      seed: "fused-target-checks",
      p1: { hand: [REWIND, KYS_CONSTANT, HIT_JOB, ANCHOR], field: [{ def: SHRIMP, lane: 1 }], mana: 10 },
    });
    const fused = craft(s, [s.card(REWIND), s.card(KYS_CONSTANT)]);

    expect(() => legalActions(s.state, "p1")).not.toThrow();
    // Rewind's pick (an ally with a Cry) and KY's Constant's (a hand card), in ingredient order.
    expect(playsOf(s, fused).some((play) => play.targets?.length === 2)).toBe(true);
  });

  it("R102 ingredients that name the same predicate must each admit the candidate: a crafted KY's Constant + KY's Constant", () => {
    const s = scenario({
      seed: "fused-target-checks",
      p1: { hand: [KYS_CONSTANT, KYS_CONSTANT, HIT_JOB, ANCHOR], mana: 10 },
    });
    const [first, second] = s.hand("p1").filter((card) => card.defId === KYS_CONSTANT);
    if (first === undefined || second === undefined) throw new Error("setup: two KY's Constant in hand");
    const fused = craft(s, [first, second]);

    expect(() => legalActions(s.state, "p1")).not.toThrow();
    expect(playsOf(s, fused).length).toBeGreaterThan(0);
  });
});
