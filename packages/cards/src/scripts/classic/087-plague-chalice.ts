// C #87 Plague Chalice (SPEC §8.6 row 87). (X) Field Spell, Epic.
//   Base:    "This enters with X Plague Counters on it.
//             Aura: Counter every card played whose cost equals the number of Plague Counters on this."
//   Radiant: "… Counter every card your opponent plays whose cost equals the number of Plague Counters on this."
//   Engine:  "X is at least 1 (R348), and on the field it costs the X it was played for (R396). Counter
//            (§6.3), in §10.5's announce window, on every announce whose cost paid equals the current count
//            (both players'; Radiant: the opponent's). "Cost" is the cost paid, as #60 Bear Honeypot reads it
//            (R56), so a free cast (R70) is countered only at a count of 0. … It is not on the field during
//            its own announce, so it never counters itself. Tunes: none."
//
// "Enters with X" is one placement of X on itself as its Cry (`placePlague`). The Aura answers each
// `cardAnnounced` whose `costPaid` equals the tokens on it now — the count moves as tokens are placed and
// removed (C #78) — by countering that play to its owner's graveyard (`counterPlay`).

import { plagueOn, type EffectContext, type Script, type TriggerDef } from "@jackioh/engine";
import { counterPlay, placePlague } from "@jackioh/engine/effects";
import type { GameEvent } from "@jackioh/shared";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classic-087");

function matches(ctx: EffectContext & { event: GameEvent }, opponentOnly: boolean): string | null {
  const event = ctx.event;
  if (event.type !== "cardAnnounced" || ctx.self === null) return null;
  if (opponentOnly && event.player === ctx.controller) return null;
  return event.costPaid === plagueOn(ctx.self) ? event.instanceId : null;
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
  return { cry: (ctx) => [placePlague({ target: { of: "self" }, amount: ctx.x })], triggers: [aura] };
}

export const base: Script = chalice(false);

export const radiant: Script = chalice(true);
