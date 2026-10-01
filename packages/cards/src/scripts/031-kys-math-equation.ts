// #31 KY's Math Equation (SPEC §8.2 row 31), patch v0.2.0 (R429): "Deal Fib(times played + 1) damage
// to a target. End of turn: Return this to your hand. It costs (1) more, to a maximum of (4).",
// radiant "Deal Fib(times played + 3) damage to a target. …" (R275 raised the offset).
//
// Four rulings drive the whole card:
//   R429 the Fib index is the times this card has been played, THIS play included, + 1 (radiant + 3).
//        The engine counts the plays on the instance at §10.5 step 4 (`StaticFlags.countsPlays`,
//        `timesPlayedOf`), casts included (R70) and countered plays never, and the count rides the
//        card through every zone like `costMod` (R78). So the 1st play deals Fib(2) = 1, the 2nd
//        Fib(3) = 2, the 3rd 3, the 4th 5, the 5th 8 (radiant 3, 5, 8, 13, 21).
//   R67  its cost plays no part in the damage any more: `costMod`, player discounts and the cost
//        paid change its price, never what it deals.
//   R25  Fib = 0,1,1,2,3,5,8,13,21,34,55,89 and the index clamps at 11, which is what `fib` in
//        engine/src/config.ts already does — nothing here re-derives Fibonacci.
//   R280 the damage it would deal now is the card's `preview`, labelled with the running face's
//        formula as its catalog text prints it, off the same `damageFor` its Cry deals. In hand that
//        is the play it would be (one more than it has had); it reads the card's own count and
//        nothing else, so it shows wherever the card may be read (§10.8).
//
// The return is an `endOfTurn` hook on a Spell that is sitting in its owner's graveyard: §5.1's
// "add this back to your hand" spells are found there by `triggerHoldersWithHook` (triggers.ts,
// R68), which unlike `turn.ts`'s `triggerOrder` reaches the hand and the graveyard. The hook is
// one-shot by reading the turn log: the return happens on the turn the card was played, not at
// every end of turn for the rest of the game. R429: the return raises the card's own cost (R65:
// `costOverride` or printed, plus `costMod`) by 1, but never above (4) — at (4) it adds nothing.

import type { CardInstance, Effect, EffectContext, GameState, Script } from "@jackioh/engine";
import { fib, printedCost, timesPlayedOf, wasPlayedThisTurn } from "@jackioh/engine";
import { bounce, damage, setCostMod } from "@jackioh/engine/effects";
import { cardDef } from "../catalog-data";

export const def = cardDef("core-031");

/** §8 row 31: what each face adds to the times played before taking the Fibonacci number. */
const FIB_OFFSET = { base: 1, radiant: 3 } as const;

/** R429: "It costs (1) more, to a maximum of (4)." */
const RETURN_COST_STEP = 1;
const RETURN_COST_CAP = 4;

/**
 * R280: the formula as each face prints it — "Fib(times played + 1)" — which the preview labels its
 * number with. Read off the catalog text, so the label is always a substring of the face it names.
 */
const FORMULA = {
  base: formulaIn(def.base.text),
  radiant: formulaIn(def.radiant.text),
} as const;

function formulaIn(text: string): string {
  const found = /Fib\([^)]*\)/.exec(text);
  if (found === null) throw new Error(`#31's text names no Fib(…) formula: ${text}`);
  return found[0];
}

/**
 * The plays the Fib index counts: the ones the card has had, this one included. While the card is
 * resolving its play has been counted already (§10.5 step 4); anywhere else — in hand, for the
 * preview — "if it resolved now" is one more play than it has had.
 */
function playsIfResolvedNow(self: CardInstance): number {
  const counted = timesPlayedOf(self);
  return self.zone.z === "resolving" ? counted : counted + 1;
}

/**
 * The damage the card deals if it resolves now — the Cry's amount and the preview's value, one
 * function so the two cannot disagree (R280). R25: `fib` clamps the index at 11, so it tops out at 89.
 */
function damageFor(self: CardInstance, radiant: boolean): number {
  return fib(playsIfResolvedNow(self) + FIB_OFFSET[radiant ? "radiant" : "base"]);
}

function blast(ctx: EffectContext): Effect[] {
  const self = ctx.self;
  return [damage({ to: { of: "chosen" }, amount: self === null ? fib(0) : damageFor(self, ctx.radiant) })];
}

/** R65 outside a hand: the card's own cost, `costOverride` or printed, plus `costMod`, floored at 0. */
function ownCost(state: GameState, self: CardInstance): number {
  return Math.max(0, (self.costOverride ?? printedCost(state, self)) + self.costMod);
}

/**
 * "End of turn: Return this to your hand. It costs (1) more, to a maximum of (4)." Only on the turn
 * it was played: the spell stays in the graveyard when its hand is full (`bounce` burns it back,
 * §2.4), and a card that is still there on a later turn is no longer returning, so the turn log —
 * which `startTurn` clears — is the gate.
 */
function returnToHand(ctx: EffectContext): Effect[] {
  const self = ctx.self;
  if (self === null) return [];
  if (self.zone.z !== "graveyard") return [];
  if (!wasPlayedThisTurn(ctx.state, self.owner, self)) return [];
  // R78: the +1 rides on the instance in every zone. It is the price of the return, so it lands only
  // on a card that reached the hand: a full hand burns the card back to the graveyard (§2.4, R4),
  // which is no return at all. R429: never above (4), so at (4) or more it adds nothing.
  const raise = Math.max(0, Math.min(RETURN_COST_STEP, RETURN_COST_CAP - ownCost(ctx.state, self)));
  return [bounce({ target: { of: "self" } }), ...(raise > 0 ? [setCostMod({ amount: raise, inHandOnly: true })] : [])];
}

/** R81: the target travels in the `play` action; "target" is any unit or hero (§8 Conventions). */
const targets: Script["targets"] = [
  { kind: "target", min: 1, max: 1, filter: { side: "any", of: ["unit", "hero"] } },
];

/**
 * R280: "Fib(times played + 1) {n}" — the damage it deals if played now, in hand, and as the card
 * stands wherever else it may be read. It reads its own count of plays, which is the card's (§9.1):
 * `viewFor` shows it only where the card itself is shown, never on the opponent's hand.
 */
const preview: Script["preview"] = (ctx) => [
  { label: FORMULA[ctx.radiant ? "radiant" : "base"], value: damageFor(ctx.self, ctx.radiant) },
];

/** R429: §10.5 step 4 counts this card's plays on its instance. */
const staticFlags: Script["staticFlags"] = { countsPlays: true };

export const base: Script = {
  staticFlags,
  targets,
  cry: blast,
  endOfTurn: returnToHand,
  preview,
};

// The radiant face changes only the Fibonacci offset (R275), so the return clause is kept.
export const radiant: Script = {
  staticFlags,
  targets,
  cry: blast,
  endOfTurn: returnToHand,
  preview,
};
