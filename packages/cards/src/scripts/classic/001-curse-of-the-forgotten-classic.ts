// C #1 Curse of the Forgotten Classic (SPEC §8.6 row 1, §6.3 Damage, Draw, Recruit, Forced attack;
// R12, R33, R53, R63, R280). Spell, cost 1, Rare.
//   Base:    "Deal {damage} damage to the enemy hero for each card in their exile. Draw {draw}."
//   Radiant: "Deal {damage} damage to the enemy hero for each card in their exile. Draw {draw}.
//            Recruit a card from their exile. If it's a Unit, it attacks the enemy hero at once."
//   Engine:  "One hit of N on the enemy hero, N = the opponent's exile size as it resolves: "for each
//            card" in one sentence is one hit, so Armor applies once, as Hearthstone reads it, and
//            N = 0 is no hit (R63); then the draw. The Radiant face keeps the draw. Recruit (§6.3) from
//            the opponent's exile: exile is chronological (§3), so Recruit's top-down scan is newest
//            first, and the most recently exiled permanent card there is summoned under your control,
//            no Cry (R1), becoming yours as it reaches your field, so it goes to your piles when it
//            leaves the field (R12); a Unit then makes one forced attack on the enemy hero (R53), summoning sickness
//            ignored; with no permanent in their exile nothing is recruited. Tunes: damage per card 1
//            ↑; draw 1 ↑."
//
// ONE HIT: the damage per card (`param(ctx, "damage")`) times the opponent's exile size, read as the
// Spell resolves, is a single `damage` on the enemy hero, so Armor and a hit cap meet it once. A total
// of 0 is no damage instance at all (R63), so nothing is dealt. Then the draw (`param(ctx, "draw")`).
//
// THE PREVIEW (R280) is that total, computed by the same function the Cry deals with, under the label
// "for each card in their exile" (the formula as both faces print it). It reads the opponent's exile
// size and the card's own number, both public.
//
// THE RADIANT RECRUIT is the engine's E25 `recruit({ from: "exile", whose: "enemy" })`: their exile
// scanned newest first for a permanent (never a Spell), summoned on your side under your control and
// yours from then on (R12, R669) (a Unit to your leftmost open unit zone, a Trap face-down to your backrow, read by
// you alone, R33); with no open zone for it, or no permanent there, nothing is recruited. A Unit it
// recruited then makes one forced attack on the enemy hero (`forcedAttacks` over the units of its
// definition this list summoned, R53): no Taunt, position or summoning sickness stops it, and it spends
// no exertion. Which card the Recruit takes is known before it happens (the newest permanent in their
// exile), so the attacker is named by that definition as well as by "summoned by this list": a Unit a
// card cast on the draw summoned (R58) is this list's too, and it does not attack.

import type { CardInstance, ConditionContext, Effect, EffectContext, Script } from "@jackioh/engine";
import { defOf, param, zoneCards, zoneCount } from "@jackioh/engine";
import type { PlayerId } from "@jackioh/shared";
import { opponentOf } from "@jackioh/shared";
import { damage, draw, forcedAttacks, recruit } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classic-001");

/** The label both faces print the formula under (R280). */
const FORMULA = "for each card in their exile";

/** N: the damage per card times the opponent's exile size, now. */
function curseDamage(ctx: EffectContext | ConditionContext, controller: PlayerId): number {
  return param(ctx, "damage") * zoneCount(ctx.state, opponentOf(controller), "exile");
}

function hitAndDraw(ctx: EffectContext): Effect[] {
  return [damage({ to: { of: "enemyHero" }, amount: curseDamage(ctx, ctx.controller) }), draw({ count: param(ctx, "draw") })];
}

const preview: Script["preview"] = (ctx) => [{ label: FORMULA, value: curseDamage(ctx, ctx.controller) }];

export const base: Script = {
  cry: (ctx) => hitAndDraw(ctx),
  preview,
};

/**
 * The card the Recruit will take: the newest permanent in the opponent's exile (a Spell is never
 * recruited, and a unit token is never in an exile, R11), read as the Spell resolves.
 */
function newestPermanentOfTheirs(ctx: EffectContext): CardInstance | null {
  const exile = zoneCards(ctx.state, opponentOf(ctx.controller), "exile");
  return [...exile].reverse().find((card) => defOf(ctx.state, card.defId).type !== "Spell") ?? null;
}

export const radiant: Script = {
  cry: (ctx) => {
    const recruited = newestPermanentOfTheirs(ctx);
    return [
      ...hitAndDraw(ctx),
      recruit({ from: "exile", whose: "enemy" }),
      // "If it's a Unit, it attacks": the unit this list summoned of that definition — not a Unit a
      // card cast on the draw summoned (C+ #26 Tommy Tempo), which is summoned by this list too.
      ...(recruited !== null && defOf(ctx.state, recruited.defId).type === "Unit"
        ? [
            forcedAttacks({
              attackers: { side: "self", defId: recruited.defId, summonedThisScript: true },
              target: { spec: { of: "enemyHero" } },
            }),
          ]
        : []),
    ];
  },
  preview,
};
