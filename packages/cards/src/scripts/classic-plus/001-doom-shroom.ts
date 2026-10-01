// C+ #1 Doom Shroom (SPEC §8.7 row 1). (3) Trap, Epic.
// Fires in §4.2 step 4's trap window (as Core #96 My Pawn) when an enemy Unit declares an attack on
// your hero: exiles every Unit (Radiant: every enemy Unit), tops of piles only, so the attacker is gone
// and no combat happens (R44, R220); then Locks its own backrow zone (E20) and is consumed.

import type { Script, TrapTrigger } from "@jackioh/engine";
import { attackTargetOf, findInstance } from "@jackioh/engine";
import { exileAll, lockOwnZone } from "@jackioh/engine/effects";
import { opponentOf } from "@jackioh/shared";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classicplus-001");

/** A declared attack (a forced one opens no window, R121) by an enemy Unit on this trap's controller's hero. */
const attacksYourHero: NonNullable<TrapTrigger["when"]> = ({ event, state, controller }) => {
  if (event.type !== "attackDeclared" || event.forced) return false;
  const target = attackTargetOf(state, event.targetId);
  return findInstance(state, event.attackerId)?.controller === opponentOf(controller) && target?.kind === "hero" && target.player === controller;
};

function doomShroom(side: "any" | "enemy"): Script {
  return { triggers: [{ id: "doom-shroom", on: ["attackDeclared"], when: attacksYourHero, run: () => [exileAll({ side }), lockOwnZone()] }] };
}

export const base: Script = doomShroom("any");

export const radiant: Script = doomShroom("enemy");
