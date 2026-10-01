// C #39 Outbreak (SPEC §8.6 row 39). (1) Spell, Epic.
//   Base:    "Place {tokens|Plague Token|Plague Tokens} on a permanent. If it's an enemy permanent with
//            at least as many Plague Tokens as its cost, steal it. Otherwise, draw a card for each
//            Plague Token on it." — 1 token
//   Radiant: the same text — 2 tokens
//   Engine:  "One declared target (R81), either side, one placement of 1 (Radiant 2) (Plague Tokens,
//            §6.3; a C #27 Pestilent Slime multiplies it). Its cost is R65's on the field: an X-cost
//            card on the field costs the X it was played for … (R396) … Steal per §6.3 and R15;
//            otherwise draw N (the hand cap applies). Tunes: tokens 1 ↑."
//
// The target is any permanent on either side — the top of a unit pile or a backrow card, face-down
// ones included — declared with the play (R81); a face-down card its chooser may not read is offered
// by its id alone and the placement on it never names it to them (R177).
//
// One placement of {tokens} on it (`placePlague`, B5 E19, R471), multiplied by its own multiplier.
// Then the two branches, read once the placement has landed:
//   * an enemy permanent (its controller is not the caster) whose Plague Tokens are at least its cost
//     — R396's `costNow`: an X card on the field its X, 0 with none chosen; any other card R65's cost
//     where it stands — is stolen (§6.3, R15: the same lane if free, else the first free zone of its
//     row, an entry, R171); with no free zone it stays with them, and nothing is drawn, since the
//     text's "otherwise" is the condition's, not the steal's. A (0) Cost enemy permanent is always
//     stolen. A stolen face-down trap is read by its new controller from then on (R33).
//   * otherwise — your own permanent always — one draw per Plague Token on it (§2.4: the hand cap
//     burns what does not fit).
// A target that has left the field by then takes nothing and draws nothing.
//
// Both branches are read after the placement, as the list reaches them (`forEachCard`), because the
// placement is what they count. The number is the declared `tokens` (R386), read through `param`.

import { costNow, param, plagueOn, type EffectContext, type Script } from "@jackioh/engine";
import { draw, forEachCard, instanceOf, placePlague, steal } from "@jackioh/engine/effects";
import type { TargetDecl } from "@jackioh/shared";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classic-039");

/** "a permanent": the top of any unit pile or any backrow card, either side. */
const targets: TargetDecl[] = [{ kind: "target", min: 1, max: 1, filter: { side: "any", of: ["unit", "backrow"] } }];

/**
 * The chosen permanent if it is still on the field, and whether the steal branch holds for it: an
 * enemy permanent with at least its cost in Plague Tokens — or one this run's steal has just taken,
 * which the draw half reads as the steal branch too (R136: this script's own `controlChanged`).
 */
function outcome(ctx: EffectContext): { id: string; steals: boolean; tokens: number } | null {
  const card = instanceOf(ctx, { of: "chosen" });
  if (card === null || card.zone.z !== "field") return null;
  const tokens = plagueOn(card);
  const takenNow = ctx.events
    .slice(ctx.eventsFrom)
    .some((event) => event.type === "controlChanged" && event.instanceId === card.id);
  const steals = takenNow || (card.controller !== ctx.controller && tokens >= costNow(ctx.state, card));
  return { id: card.id, steals, tokens };
}

export const base: Script = {
  targets,
  cry: (ctx) => [
    placePlague({ target: { of: "chosen" }, amount: param(ctx, "tokens") }),
    forEachCard({
      cards: (c) => {
        const read = outcome(c);
        return read !== null && read.steals ? [read.id] : [];
      },
      each: (instanceId) => steal({ instanceId }),
    }),
    forEachCard({
      // One entry per token, each a draw of 1: "draw a card for each Plague Token on it".
      cards: (c) => {
        const read = outcome(c);
        return read === null || read.steals ? [] : Array.from({ length: read.tokens }, () => read.id);
      },
      each: () => draw({ count: 1 }),
    }),
  ],
};

// The same script: the Radiant face's 2 tokens are its declared `tokens`, which `param` reads off the running face.
export const radiant: Script = base;
