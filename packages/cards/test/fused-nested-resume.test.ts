// R102, B3.4 rule 5: a fusion several levels deep names the ingredient whose text is running by a path
// of indices from the outermost card in (`work.PART_KEY`), and `param` walks that path down the fused
// ids. The path is built by the hooks of each level, so every level has to add its index.
//
// Found by the fuzz gate (seed 598 of fuzz-handicap, once the pool changed by a card): a Final Gambit
// four fusions down resumed its step ("heal your hero, draw") against the wrong ingredient and threw
// `declares no number "heal"`. A table of steps (`Script.resume`) held by only one ingredient of a
// fusion was passed up as it stood, so no hook at that level wrapped its steps; the one that did sat
// at the first level with two such tables, and its path began there. A Final Gambit set under a card
// with no table of its own read its numbers off the outer card's first ingredient. Fusion ids stay
// short enough to spell out here, so the digest ids of the seed (R468) are not needed to show it.

import { describe, expect, it } from "vitest";
import { createRng, subsystems, type CardInstance } from "@jackioh/engine";
import { scenario, type Scenario } from "./_harness";

const GAMBIT = "classic-052"; // (2) Trap: a lethal hit is re-aimed; then heal {heal} 10 and draw {draw} 3 (resume step).
const INCOME_TAX = "classic-009"; // (2) Trap with a table of steps of its own, and no "heal".
const COUNTERSPELL = "classic-017"; // (2) Trap with no table of steps: the outer ingredient of the fusion.
const FILLER = "core-005";
const ATTACKER = "core-008"; // 4/4

function craft(s: Scenario, ingredients: [CardInstance, CardInstance]): CardInstance {
  const sink = { state: s.state, events: [], rng: createRng(s.state.seed, s.state.rngCursor) };
  const fused = subsystems.fuse(sink, { ingredients, toHand: "p1" });
  if (fused === null) throw new Error("the fusion did not happen");
  return fused;
}

/** p1 sets `Counterspell + (Final Gambit + Income Tax)` face-down at 4 health; p2 attacks its hero for 4. */
function lethalAgainstNested(): Scenario {
  const s = scenario({
    seed: "fused-nested-resume",
    p1: { hand: [GAMBIT, INCOME_TAX, COUNTERSPELL, FILLER], library: [FILLER, FILLER, FILLER, FILLER, FILLER], health: 4, mana: 10 },
    p2: { hand: [FILLER], field: [ATTACKER], library: [FILLER, FILLER, FILLER, FILLER, FILLER] },
  });
  const inner = craft(s, [s.card(GAMBIT), s.card(INCOME_TAX)]);
  const outer = craft(s, [s.card(COUNTERSPELL), inner]);
  s.play(outer.id);
  s.endTurn();
  s.attack(ATTACKER, "hero");
  return s;
}

describe("R102 a Final Gambit under a fusion whose other ingredients hold no table of steps", () => {
  it("R102 resumes its own step: the hit is re-aimed, the hero heals 10 and draws 3, as a Final Gambit standing alone does", () => {
    const s = lethalAgainstNested();

    s.expectEvents("attackDeclared", "trapFired", "redirected", "damage", "healed");
    s.expectHealth("p1", 14).expectHealth("p2", 26);
    expect(s.hand("p1")).toHaveLength(4);
  });
});
