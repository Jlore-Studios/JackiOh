// #93 Combo-Index (SPEC §8.4, R27, R60, R62, R85, R86, BUILD M4-T4 row 93).
//
// Base: a Field Spell carrying a grade counter that starts at E and rises at the end of its
// controller's turn whenever they played at least `grade` cards that turn, running every step from
// E up to the new grade. Radiant: "Start of turn: add a Combo-Fodder to your hand; same" — one
// clause ADDED ("same" keeps the whole base text, §8 Conventions), so the radiant face is the base
// script plus a `startOfTurn`.
//
// Almost none of that is in this file, and deliberately: `engine/src/subsystems/comboIndex.ts` owns
// the grade counter and the cascade, and its header names this file's shape — "the card file (M4)
// stays a list of effects: `endOfTurn: (ctx) => comboIndexEndOfTurn(ctx, ctx.self)`". Calling the
// subsystem instead of re-deriving the cascade is what keeps the counter, `viewFor` and a replay
// reading the same number (§10.1), and it is the only place `counters.grade` is written.
//
// The two hooks:
//   `cry`        — "Grade counter, starts at E": `startGrade()` writes the counter as the card
//                  arrives so the client has a grade to show before the first end of turn. `cry` is
//                  a Field Spell's on-resolve hook as well as a unit's Cry (§10.5 step 5, and
//                  `runHook`'s doc in `engine/src/resolve.ts`); #73 Anti-oneshot Armor is the other
//                  Field Spell that uses it.
//   `endOfTurn`  — the whole threshold-and-cascade check, at its R62 point (end-of-turn triggers,
//                  before the trap window, while `turnLog.cardsPlayed` is still this turn's count:
//                  `turn.ts` runs the hooks before `cleanup`).
//
// The rulings the subsystem implements, named here so a change has a test with its name on it:
// R27 (E adds a fresh copy keeping the radiant flag, D picks 2 DIFFERENT hand cards, steps run
// E→new grade in order, S is terminal), R60 (a random "becomes Radiant" pick only considers
// non-Radiant cards), R85 (grade A's 8 damage has Lifesteal of its own without #93 gaining the
// keyword) and R86 (grade E's pool skips cards that have ceased to exist).
//
// R195, the yellow glow: on the field, during its controller's turn, the card glows exactly when
// the end of this turn would raise the grade — the cards played reach the grade and it is not at S.
// `conditionMet` reads the subsystem's own `gradeRises`, the predicate `comboIndexEndOfTurn` checks,
// so the glow and the rise cannot disagree; `yourTurn` stands for "the end of turn that fires
// `endOfTurn` is this one" without the card reading `state.active`. In hand it never glows: the
// grade is a counter on a card in play.
//
// R372, the grade in play: the printed text starts "Grade (starts at E)" and names the threshold as
// "N = the grades from E to the current one", so a player reading the card in play needs the letter
// it has reached and the N it asks for now. `preview` returns both, on the field only (in hand the
// card has no grade yet, and the text already says it starts at E): "Grade {C}", the letter carried
// as the value's `display` because the text names a grade by its letter, and "N … {3}", which is
// the same number `gradeRises` compares the plays with. At S nothing rises (R27), so there is no N to
// show. Both read the counter through the subsystem, so the view, the glow and the rise agree.

import type { Script } from "@jackioh/engine";
import { subsystems } from "@jackioh/engine";
import { addToHand } from "@jackioh/engine/effects";
import { cardDef } from "../catalog-data";

export const def = cardDef("core-093");

/** §7, §8: the token the radiant face hands you each turn. `cardDef` is the only id source. */
const COMBO_FODDER = cardDef("core-093-1").id;

/** "Grade counter, starts at E" (§8, §10.1). */
const cry: Script["cry"] = () => [subsystems.startGrade()];

/**
 * "End of turn: if cards played this turn ≥ grade, grade +1 and run every step from E up to the new
 * grade" (§8, R27). The subsystem returns the whole cascade as one effect list, so the resolution
 * loop sees one trigger and R59's state check falls between whole steps, not inside them.
 */
const endOfTurn: Script["endOfTurn"] = (ctx) =>
  ctx.self === null ? [] : subsystems.comboIndexEndOfTurn(ctx, ctx.self);

/** R195: field only, on its controller's turn, when `endOfTurn` would raise the grade now. */
const conditionMet: Script["conditionMet"] = (ctx) =>
  ctx.zone === "field" && ctx.yourTurn && subsystems.gradeRises(ctx.state, ctx.self);

/** R372: the words of the printed text the two values follow, on both faces. */
const GRADE_LABEL = "Grade";
const THRESHOLD_LABEL = "N = the grades from E to the current one";

/**
 * R372: "Grade {C}" and "N = … {3}" on the field; nothing in hand, where the card has no grade yet.
 * The counter is public on the Field Spell (§10.8), so the values say nothing the board does not.
 */
const preview: Script["preview"] = (ctx) => {
  if (ctx.zone !== "field") return [];
  const grade = subsystems.gradeOf(ctx.self);
  const letter = { label: GRADE_LABEL, value: grade, display: subsystems.gradeName(grade) };
  return subsystems.isTerminalGrade(grade) ? [letter] : [letter, { label: THRESHOLD_LABEL, value: grade }];
};

export const base: Script = { cry, endOfTurn, conditionMet, preview };

export const radiant: Script = {
  cry,
  endOfTurn,
  conditionMet,
  preview,
  // "Start of turn: add a Combo-Fodder to your hand". A full hand burns it (§2.4, R4), which
  // `addToHand` already does.
  startOfTurn: () => [addToHand({ defId: COMBO_FODDER })],
};
