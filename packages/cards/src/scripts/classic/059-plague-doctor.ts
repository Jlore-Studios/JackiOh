// C #59 Plague Doctor (SPEC §8.6 row 59). (1) Unit, Human, Common, 2/3 → 4/6.
//   Base:    "Cry: Deal damage equal to the number of Plague Counters on the field."
//   Radiant: "Cry: Place {tokens|Plague Counter|Plague Counters} on this. Then deal damage equal to the
//            number of Plague Counters on the field." — 2 tokens
//   Engine:  "A declared target (R81); one hit of N, N = every Plague Counter on both sides, counted as it
//            resolves, after the Radiant's own placement (N = 0 is no hit, R63). A `preview` (R280)
//            shows N. Tunes: Radiant tokens 2 ↑."
//
// The target is declared with the play (R81): a Unit — the top of a unit pile, either side — or a hero.
// N is every Plague Counter on the field, both sides, face-down cards included, as the Cry resolves. On the Radiant face the Cry first makes one placement of {tokens} on the Doctor itself
// (`placePlague`, multiplied by the Doctor's own multiplier, as a placement on any card is), and N
// counts those too. One hit of N on the target; N = 0 is no hit at all (R63).
//
// The Cry builds its list once, so N is read as it begins and the Radiant's own placement is added to
// it: `tokensItPlaces` is exactly what `placePlague` puts on the Doctor (the declared number times its
// multiplier, nothing when it is not on the field), and nothing else in the list moves a token between
// the placement and the hit — a trigger the placement wakes waits for the whole Cry.
//
// R280: the preview is N — `damageNow`, the same function the Cry deals with: the tokens on the field
// now plus, on the Radiant face, the ones its own placement would add. Plague Counters are public on
// every permanent, a face-down one's included (§10.8), so the number reveals nothing. The label is
// the phrase both faces print. Its proofs are in `test/preview.test.ts`.
//
// The number is the declared `tokens` (R386), read through `param`; the base face declares it too (the
// entry's params are per card) but never reads it.

import {
  param,
  permanentsOnField,
  plagueMultiplierOf,
  plagueOn,
  type CardInstance,
  type Effect,
  type EffectContext,
  type GameState,
  type Script,
} from "@jackioh/engine";
import { damage, placePlague } from "@jackioh/engine/effects";
import type { PlayerId, PreviewValue, TargetDecl } from "@jackioh/shared";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classic-059");

/** R280: the formula as both faces print it. */
export const DOCTOR_LABEL = "the number of Plague Counters on the field";

/** "a target": a Unit on either side, or either hero (R81). */
const targets: TargetDecl[] = [{ kind: "target", min: 1, max: 1, filter: { side: "any", of: ["unit", "hero"] } }];

type Read = { state: GameState; self: CardInstance | null; radiant: boolean; controller: PlayerId };

/**
 * Every Plague Counter on the field: each permanent on both sides, face-down ones included, the top of
 * each pile only (R13). Walked from the controller's side, so the read never asks whose turn it is.
 */
function tokensOnField(read: Read): number {
  return permanentsOnField(read.state, read.controller).reduce((sum, card) => sum + plagueOn(card), 0);
}

/**
 * The Plague Counters the Radiant face's own placement puts on the Doctor: the declared number times its
 * own multiplier (a Doctor fused onto a C #27 Pestilent Slime doubles it), in hand as it would once
 * played. A preview is asked only of a Doctor acting on the field or in a hand (R13), and its Cry runs
 * as it arrives on top of its zone, so the Doctor always carries the placement.
 */
function tokensItPlaces(read: Read): number {
  if (!read.radiant || read.self === null) return 0;
  return Math.max(0, param(read, "tokens")) * plagueMultiplierOf(read.state, read.self);
}

/** N: every Plague Counter on the field, plus the Radiant face's own placement, as the Cry resolves now. */
export function damageNow(read: Read): number {
  return tokensOnField(read) + tokensItPlaces(read);
}

function cry(ctx: EffectContext): Effect[] {
  const amount = damageNow(ctx);
  const hit = amount > 0 ? [damage({ to: { of: "chosen" }, amount })] : [];
  if (!ctx.radiant) return hit;
  return [placePlague({ target: { of: "self" }, amount: param(ctx, "tokens") }), ...hit];
}

function preview(read: Read): PreviewValue[] {
  return [{ label: DOCTOR_LABEL, value: damageNow(read) }];
}

export const base: Script = { targets, cry, preview };

// The same script: the Radiant face's placement is `ctx.radiant`'s branch, and its 2 is the declared
// `tokens`, which `param` reads off the running face.
export const radiant: Script = base;
