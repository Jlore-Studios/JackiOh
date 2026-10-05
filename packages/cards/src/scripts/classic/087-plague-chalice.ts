// C #87 Plague Chalice (SPEC §8.6 row 87). (X) Field Spell, Epic.
//   Base:    "This enters with X Plague Tokens on it.
//             Aura: Counter every card played whose cost equals the number of Plague Tokens on this."
//   Radiant: "… Counter every card your opponent plays whose cost equals the number of Plague Tokens on this."
//   Engine:  "X is at least 1 (R348), and on the field it costs the X it was played for (R396). Counter
//            (§6.3), in §10.5's announce window, on every announce whose cost paid equals the current count
//            (both players'; Radiant: the opponent's). "Cost" is the cost paid, as #60 Bear Honeypot reads it
//            (R56), so a free cast (R70) is countered only at a count of 0. … It is not on the field during
//            its own announce, so it never counters itself. Tunes: none."
//
// "Enters with X" is one placement of X on itself as its Cry (`placePlague`). The Aura answers each
// `cardAnnounced` whose `costPaid` equals the tokens on it now — the count moves as tokens are placed and
// removed (C #78) — by countering that play to its owner's graveyard (`counterPlay`).
//
// Patch v0.2.12 (#126, R667): `wouldCounter` is the same match asked ahead of any play, so the engine
// can warn the viewer off a hand card the Chalice would counter (`counteredOnPlay`). Both halves call
// `counters`, so the warning and the counter cannot disagree.

import { plagueOn, type CardInstance, type EffectContext, type Script, type TriggerDef } from "@jackioh/engine";
import { counterPlay, placePlague } from "@jackioh/engine/effects";
import type { GameEvent, PlayerId } from "@jackioh/shared";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classic-087");

/** The match: a play `player` makes paying `costPaid`, against this Chalice's count and its face's reach. */
function counters(self: CardInstance, controller: PlayerId, player: PlayerId, costPaid: number, opponentOnly: boolean): boolean {
  if (opponentOnly && player === controller) return false;
  return costPaid === plagueOn(self);
}

function matches(ctx: EffectContext & { event: GameEvent }, opponentOnly: boolean): string | null {
  const event = ctx.event;
  if (event.type !== "cardAnnounced" || ctx.self === null) return null;
  return counters(ctx.self, ctx.controller, event.player, event.costPaid, opponentOnly) ? event.instanceId : null;
}

function chalice(opponentOnly: boolean): Script {
  const aura: TriggerDef = {
    id: "plague-chalice",
    on: ["cardAnnounced"],
    when: (ctx) => matches(ctx, opponentOnly) !== null,
    run: (ctx) => {
      const instanceId = matches(ctx, opponentOnly);
      return instanceId === null ? [] : [counterPlay({ target: { of: "instance", instanceId } })];
    },
  };
  return {
    cry: (ctx) => [placePlague({ target: { of: "self" }, amount: ctx.x })],
    triggers: [aura],
    wouldCounter: ({ self, controller, player, costPaid }) => counters(self, controller, player, costPaid, opponentOnly),
  };
}

export const base: Script = chalice(false);

export const radiant: Script = chalice(true);
