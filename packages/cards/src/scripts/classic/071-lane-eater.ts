// C #71 Lane Eater (SPEC §8.6 row 71). (3) Unit, Common, 4/4 → 8/8.
//   Base:    "Cry: Destroy every other card in this lane. Lock this lane."
//   Radiant: "Cry: Destroy the enemy cards in this lane. Lock the enemy side of this lane."
//   Engine:  "The lane's four zones (§3.1): Lane Eater's own, your backrow zone and both of the
//            opponent's. The top card of each other zone is destroyed (a dormant card under a Stack is
//            not on the field, R13, and resumes); then all four are Locked (Lock / Unlock, §6.3). Lock
//            evicts nothing, so Lane Eater stays and its zone stays Locked after it leaves. Radiant: the
//            opponent's two zones only. Unlabelled one-time text on a Unit is its Cry. Tunes: none."
//
// "This lane" is the column Lane Eater stands in (§3.1: your lane N faces the opponent's lane N), read
// as the Cry resolves. The destroy marks the card acting in each other zone — the top of a unit pile or
// of a backrow pile (R13), a face-down trap included — and the state check after the Cry collects them
// together (R59); Indestructible ones stay (R46), and a card beneath a destroyed top resumes there.
// Then `lockLane` Locks the zones (both sides on the base face, the enemy side on the Radiant face)
// before that check, so a destroyed Reborn Unit finds its zone Locked and does not return (R47). A
// Lock evicts nothing: Lane Eater, the survivors and the resumed cards stay, later summons and plays
// into those zones are refused (§3.2), and the zones stay Locked after Lane Eater leaves.
//
// Rulings: R13, R46, R47, R59; §3.1, §3.2. Its proof: `test/classic/071-lane-eater.test.ts`.

import { cardAt, slotOf, type EffectContext, type Effect, type Script } from "@jackioh/engine";
import { destroy, lockLane } from "@jackioh/engine/effects";
import { opponentOf, type PlayerId, type Row } from "@jackioh/shared";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classic-071");

/** The zones of this lane the face destroys in: the base face's three others, the Radiant's two enemy ones. */
function laneZones(me: PlayerId, enemyOnly: boolean): { player: PlayerId; row: Row }[] {
  const enemy = opponentOf(me);
  const theirs: { player: PlayerId; row: Row }[] = [
    { player: enemy, row: "units" },
    { player: enemy, row: "backrow" },
  ];
  return enemyOnly ? theirs : [{ player: me, row: "backrow" }, ...theirs];
}

/** Destroy the card acting in each of those zones of Lane Eater's lane, as the Cry resolves. */
function destroyInLane(ctx: EffectContext, enemyOnly: boolean): Effect[] {
  const at = ctx.self === null ? null : slotOf(ctx.state, ctx.self);
  if (at === null || at.row !== "units") return [];
  return laneZones(ctx.controller, enemyOnly).flatMap(({ player, row }) => {
    const card = cardAt(ctx.state, { player, row, lane: at.lane });
    return card === null ? [] : [destroy({ target: { of: "instance", instanceId: card.id } })];
  });
}

export const base: Script = {
  cry: (ctx) => [...destroyInLane(ctx, false), lockLane()],
};

export const radiant: Script = {
  cry: (ctx) => [...destroyInLane(ctx, true), lockLane({ side: "enemy" })],
};
