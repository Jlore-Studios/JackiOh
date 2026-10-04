// C #63 Crop Dusting (SPEC §8.6 row 63). (2) Trap, Common.
//   Base:    "Start of turn: Reveal. Place {tokens|Plague Counter|Plague Counters} on each
//            permanent. Draw {draw}." — 1 counter, draw 1
//   Radiant: the same text — 3 counters, draw 3 (patch v0.2.2's face, R275, R276)
//   Engine:  "A Trap whose condition is its controller's start of turn, fired with the start-of-turn
//            triggers (§2.2, R62); it fires once and goes to the graveyard. Each permanent on the field,
//            both sides, face-down ones included, gets one placement (Plague Counters, §6.3) of 1
//            (Radiant 3). The designer's Radiant face (patch v0.2.2, R652) triples both numbers.
//            Tunes: tokens 1 ↑; draw 1 ↑."
//
// A Trap is set face-down (R33) and fires by answering an event (§5.1, R99): this one answers its own
// controller's `turnStarted`, so it stays set through the opponent's turn and fires at the start of its
// controller's next one. The engine dispatches `turnStarted` to the traps at the first settle of the
// turn (after the refresh and the Brittle tick), so it fires before the turn's draw, as R62 has every
// start-of-turn trigger do. Firing turns it face-up and spends it to its owner's graveyard.
//
// "Each permanent" is one placement of {tokens} on every permanent on the field (`placePlagueEach`):
// the top of each unit pile and every backrow card, both sides, face-down ones included, in R68's
// order — each multiplied by the card that receives it (C #27) and each its own placement for "whenever
// Plague Counters are placed on this" (C #53). The firing trap is one of them too (R550); it is spent to
// the graveyard as its firing ends, and its tokens go with it (R78). A placement on a card a player may not
// read never names it to them (R97). Then the draw.
//
// Both numbers are the declared `tokens` and `draw` (R386), read through `param`.

import { param, type EffectContext, type Script, type TrapTrigger } from "@jackioh/engine";
import { draw, placePlagueEach } from "@jackioh/engine/effects";
import type { GameEvent } from "@jackioh/shared";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classic-063");

/** "At the start of your turn": its controller's `turnStarted`. */
function yourTurnStarts(ctx: EffectContext & { event: GameEvent }): boolean {
  return ctx.event.type === "turnStarted" && ctx.event.player === ctx.controller;
}

const dusting: TrapTrigger = {
  id: "crop-dusting",
  on: ["turnStarted"],
  when: yourTurnStarts,
  run: (ctx) => [
    placePlagueEach({ scope: { side: "any", rows: ["units", "backrow"] }, amount: param(ctx, "tokens") }),
    draw({ count: param(ctx, "draw") }),
  ],
};

export const base: Script = { triggers: [dusting] };

// The same script: the Radiant face's 3 counters and draw 3 are its declared numbers, which `param`
// reads off the running face.
export const radiant: Script = base;
