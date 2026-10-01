// C #10 Exile (SPEC §8.6 row 10). Trap, cost 2, Common.
//   Base:    "Activates when your opponent plays a card that costs ({threshold}) or less: Counter and
//             exile it."
//   Radiant: "Activates when your opponent plays a card that costs ({threshold}) or less: Counter and
//             exile it. Then exile random enemy permanents that together cost up to ({threshold})
//             minus its cost."
//
// Counter (§6.3, B5 E1, R448), in §10.5's announce window (`cardAnnounced`), before the card moves:
// the countered card never resolves or enters the field and goes to exile, not the graveyard. It is
// treated as never played. "Costs" is the cost paid (the announce's `costPaid`), as #60 Bear Honeypot
// reads it (R56), so a card cast for free (R70) always qualifies. A card set face-down is announced to
// the opponent by its zone and cost only (§10.5 step 3a), so this trap reads nothing it may not; the
// exile then shows the card (exile is public). The condition lives in `when` (R99, R61), so a play
// that costs more, or its controller's own play, leaves it set. A Trap is consumed when it fires.
//
// Radiant: a budget of the threshold ((3) on the Radiant face, tuned with it) minus the countered
// card's cost paid, as the designer's "until the difference in cost is made up (but never exceeded)"
// reads. Then, one at a time: pick a random enemy permanent (the top of a unit pile or a backrow card,
// face-down ones included) whose cost now is no more than the budget left — R396: R65's cost where it
// stands, an X card at the X it was played for (0 with none chosen), read by `costNow` — exile it and
// take its cost off the budget; stop when the budget is 0 or nothing fits. A (0) Cost permanent always
// fits while the budget is above 0. The picks are drawn from the match rng as the clause resolves and
// kept (`forEachCard`), so a pause could never re-roll them (R113). The name is also a rules word,
// which the reference proof never reads as this card unless `refs` lists it (R381).

import type { GameEvent } from "@jackioh/shared";
import { opponentOf } from "@jackioh/shared";
import type { CardInstance, Effect, EffectContext, Script, TrapTrigger } from "@jackioh/engine";
import { activeUnitsOf, cardAt, costNow, param, slotsOf } from "@jackioh/engine";
import { counterPlay, exile, forEachCard } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classic-010");

type Announced = Extract<GameEvent, { type: "cardAnnounced" }>;

/** "When your opponent plays a card that costs ({threshold}) or less": the cost paid (R56, R70). */
function cheapPlay(ctx: EffectContext & { event: GameEvent }): Announced | null {
  const event = ctx.event;
  if (event.type !== "cardAnnounced" || event.player === ctx.controller) return null;
  return event.costPaid <= param(ctx, "threshold") ? event : null;
}

/** The enemy permanents on the field: the top of each unit pile, then each backrow card (§3.2, R13). */
function enemyPermanents(ctx: EffectContext): CardInstance[] {
  const enemy = opponentOf(ctx.controller);
  const backrow = slotsOf(enemy, "backrow").flatMap((slot) => {
    const card = cardAt(ctx.state, slot);
    return card === null ? [] : [card];
  });
  return [...activeUnitsOf(ctx.state, enemy), ...backrow];
}

/**
 * The Radiant face's picks, drawn as the clause resolves: random enemy permanents, each costing no
 * more than the budget left (R396), until the budget is spent or nothing fits.
 */
function budgetPicks(ctx: EffectContext, budget: number): CardInstance[] {
  const picked: CardInstance[] = [];
  let left = budget;
  while (left > 0) {
    const fits = enemyPermanents(ctx).filter(
      (card) => !picked.includes(card) && costNow(ctx.state, card) <= left,
    );
    const card = ctx.rng.pick(fits);
    if (card === undefined) break;
    picked.push(card);
    left -= costNow(ctx.state, card);
  }
  return picked;
}

function exileTrap(radiantFace: boolean): TrapTrigger {
  return {
    id: "exile",
    on: ["cardAnnounced"],
    when: (ctx) => cheapPlay(ctx) !== null,
    run: (ctx) => {
      const played = cheapPlay(ctx);
      if (played === null) return [];
      const countered: Effect = counterPlay({ to: "exile", target: { of: "instance", instanceId: played.instanceId } });
      if (!radiantFace) return [countered];
      const budget = param(ctx, "threshold") - played.costPaid;
      return [
        countered,
        forEachCard({
          cards: (at) => budgetPicks(at, budget),
          each: (instanceId) => exile({ target: { of: "instance", instanceId } }),
        }),
      ];
    },
  };
}

export const base: Script = { triggers: [exileTrap(false)] };

export const radiant: Script = { triggers: [exileTrap(true)] };
