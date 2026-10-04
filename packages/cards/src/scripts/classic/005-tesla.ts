// C #5 Tesla (SPEC §8.6 row 5). Field Trap, cost 2, Epic, 1/4 → 2/8 (its unit face).
//   Both faces: "Animated, Lifesteal
//                Reveals when your opponent summons a Unit: Deal {damage} damage to it. Then summon
//                this as a Unit in Defense Position." — damage 4 on the base face, 8 on the Radiant.
//
// A Field Trap answering each Unit that arrives on the opponent's side, however it is summoned (§6.3
// Summon: played, cast, a token, a Recruit, a Reborn body); a card already on the field that crosses
// to that side by a steal (R171) or steps into a unit zone by animating (R383) was not summoned and
// leaves it set, as does any Unit of its own controller's. A played (or cast) Unit is answered after it
// resolves (`cardResolved`), as #60 Bear Honeypot answers (R17, Hearthstone's Snipe), so its Cry
// happens first: the placement §10.5 step 4 reports as that play's `summoned` — the one summon that
// carries the play's stays (`exitsFrom`, R174) — is left to the `cardResolved` that follows, which
// answers only while the played card's stay on the field lasts (`permanent`). Every other `summoned`
// is answered at once.
//
// The hit's source is Tesla, whose printed Lifesteal heals its controller the amount actually dealt
// (§4.4 step 8; 0 into a Divine Shield). Then it animates in Defense Position (Animated, B3.1, R383):
// into the unit zone in its own lane when open, else the leftmost open, unlocked, unreserved one
// (R64); already a Unit, it stays put in its position; with no open unit zone it stays face-up in its
// backrow zone. A Field Trap is never consumed, so it keeps firing — from the backrow or animated, a
// turret. Face-down it is hidden like any trap until it first fires (R33). The condition lives in
// `when` (R99), so an event it declines leaves it set.

import type { GameEvent } from "@jackioh/shared";
import type { EffectContext, Script, TrapTrigger } from "@jackioh/engine";
import { defOf, findInstance, param } from "@jackioh/engine";
import { animate, damage } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classic-005");

/** The instance id of the opponent's Unit this event summons (or resolves, for a play), or null. */
function arrival(ctx: EffectContext & { event: GameEvent }): string | null {
  const event = ctx.event;
  if (event.type === "summoned") {
    if (event.player === ctx.controller || event.row !== "units") return null;
    // A play's own placement (§10.5 step 4) is answered when the play resolves, below.
    if (event.exitsFrom !== undefined) return null;
    return event.instanceId;
  }
  if (event.type === "cardResolved") {
    if (!event.permanent || defOf(ctx.state, event.defId).type !== "Unit") return null;
    const unit = findInstance(ctx.state, event.instanceId);
    if (unit === undefined || unit.zone.z !== "field" || unit.controller === ctx.controller) return null;
    return event.instanceId;
  }
  return null;
}

const zap: TrapTrigger = {
  id: "tesla-zap",
  on: ["summoned", "cardResolved"],
  when: (ctx) => arrival(ctx) !== null,
  run: (ctx) => {
    const target = arrival(ctx);
    if (target === null) return [];
    return [damage({ to: { of: "instance", instanceId: target }, amount: param(ctx, "damage") }), animate({ position: "DEF" })];
  },
};

export const base: Script = { triggers: [zap] };

// The same script: the Radiant face's 8 is its declared damage, which `param` reads off the face;
// its 2/8 body and keywords are printed on the catalog face.
export const radiant: Script = base;
