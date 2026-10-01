// C #53 Plague Crawler (SPEC §8.6 row 53). (1) Unit, Common, 2/2 → 4/4.
//   Base:    "Cry: Place {tokens|Plague Token|Plague Tokens} on another permanent.
//             Whenever Plague Tokens are placed on this, draw {draw}." — 1 token, draw 1
//   Radiant: the same text — 2 tokens, draw 2
//   Engine:  "Plague Tokens (§6.3): the Cry's declared target (R81) is another permanent on either
//            side, face-down cards included, and gets one placement of 1 (Radiant 2). The draw is a
//            trigger on a placement on this card, by any card of either player, once per placement
//            however many tokens it places. Tunes: tokens 1 ↑; draw 1 ↑."
//
// The Cry's target is declared with the play (R81): the top of any unit pile or any backrow card on
// either side, the Crawler itself never (it is still in hand as the play is chosen, and the filter
// says "another" besides). A face-down card its chooser may not read is offered by its id alone and
// the placement on it never names it to them (R177). With no other permanent on the field the target
// fizzles and the Crawler enters anyway (a target the board cannot satisfy is not a price).
//
// "Place N Plague Tokens on X" is ONE placement of N (`placePlague`), multiplied by what the card
// receiving it says (C #27 Pestilent Slime's ×2); it is reported once as `counterChanged` carrying
// `placed`.
//
// "Whenever Plague Tokens are placed on this" answers each such report naming this card: once per
// placement, however many tokens it put on (a C #27 doubling included), whoever made it — a removal
// (C #78's, C #74's spent tokens) carries no `placed` and draws nothing. The condition is the
// trigger's `when` (R99) and is checked again in `run`, so it holds whichever of the two the queue
// reads for a non-trap trigger.
//
// Both numbers are the declared `tokens` and `draw` (R386), read through `param` on the running face.

import { param, type EffectContext, type Script, type TriggerDef } from "@jackioh/engine";
import { draw, placePlague } from "@jackioh/engine/effects";
import type { GameEvent, TargetDecl } from "@jackioh/shared";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classic-053");

/** "another permanent": the top of any unit pile or any backrow card, either side, never this one. */
const targets: TargetDecl[] = [
  { kind: "target", min: 1, max: 1, filter: { side: "any", of: ["unit", "backrow"], excludeSelf: true } },
];

/** A placement of Plague Tokens on this card: `counterChanged` for `plague` with `placed`, naming it. */
function placedOnThis(ctx: EffectContext & { event: GameEvent }): boolean {
  const event = ctx.event;
  return (
    ctx.self !== null &&
    event.type === "counterChanged" &&
    event.counter === "plague" &&
    event.instanceId === ctx.self.id &&
    (event.placed ?? 0) > 0
  );
}

const drawOnPlacement: TriggerDef = {
  id: "plague-crawler-placed",
  on: ["counterChanged"],
  when: placedOnThis,
  run: (ctx) => (placedOnThis(ctx) ? [draw({ count: param(ctx, "draw") })] : []),
};

export const base: Script = {
  targets,
  cry: (ctx) => [placePlague({ target: { of: "chosen" }, amount: param(ctx, "tokens") })],
  triggers: [drawOnPlacement],
};

// The same script: the Radiant face's 2 tokens and draw 2 are its declared numbers, which `param`
// reads off the running face.
export const radiant: Script = base;
