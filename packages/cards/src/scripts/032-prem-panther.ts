// #32 Prem Panther (SPEC §8.2 row 32): 5/4 "Rush / After this attacks and survives, draw 2 for each
// Unit that attack destroyed", radiant 10/8 "Rush, Cleave / the same" (patch v0.2.0, R426).
//
// Rush and Cleave are printed on the catalog faces (`def.base.keywords`, `def.radiant.keywords`),
// so §10.4 layer 1 already grants them and nothing here re-grants them. Both faces carry the same
// text; what changes is that a radiant Panther's Cleave destroys more Units in one attack, and each
// one draws 2.
//
// R426 reads the text as three conditions, and each is a fact the engine already states:
//   * "that attack destroyed": a death whose lethal hit this Panther dealt ATTACKING — its strike on
//     the Unit it attacked, or a Cleave hit on a neighbour of it (R42's killer, per Unit) — in the
//     combat of an attack it made, declared or forced (§4.2, R53). The check that closes that combat
//     marks such a death `destroyed.killerAttacking` (`stateCheck`'s `CombatCheck`), so a Unit the
//     Panther kills striking BACK while defending is not one: it is the attacker's attack, not the
//     Panther's.
//   * "survives": the trigger answers from the field, so a Panther that died in that combat — its
//     own check included, a Death hook's damage in the same check too — is in its graveyard when the
//     deaths are dispatched and answers none of them (R153), and a Reborn body that came back is a
//     new arrival that did not see them (R212, R83). It must still stand there when its trigger
//     resolves: a trap answering the death that removes it first leaves no Panther to draw.
//   * "after": the draws are queued triggers, so they come once the combat and its state check are
//     over, one trigger (2 cards) per Unit destroyed, in the order the Units died.
//
// `killerId` and `killerAttacking` are read off the event, not off the corpse: R89 runs every trigger
// but the Death hook after the instance has been reset, and `resetInstance` (zones.ts) deletes
// `lastDamagedBy` on the way to the graveyard. `stateCheck` fills both in just before the move.

import type { Script, TriggerDef } from "@jackioh/engine";
import { draw } from "@jackioh/engine/effects";
import { cardDef } from "../catalog-data";

export const def = cardDef("core-032");

/** §8 row 32: "draw 2 for each Unit that attack destroyed". */
const DRAW_PER_UNIT = 2;

/**
 * R426, per Unit: one `destroyed` event whose killer is this Panther, attacking, draws 2. Cleave
 * needs no special case — each cleaved Unit dies with its own event, so two kills queue two
 * triggers. A combat is fought only between Units in unit zones (§4.2), so every death an attack
 * dealt is a Unit's.
 */
const attackTrigger: TriggerDef = {
  id: "32-after-this-attacks-and-survives",
  on: ["destroyed"],
  run(ctx) {
    const self = ctx.self;
    const event = ctx.event;
    if (self === null || event.type !== "destroyed") return [];
    if (event.instanceId === self.id) return [];
    if (event.killerId !== self.id || event.killerAttacking !== true) return [];
    // "Survives": still standing on the field as its draws resolve.
    if (self.zone.z !== "field") return [];
    return [draw({ count: DRAW_PER_UNIT })];
  },
};

export const base: Script = { triggers: [attackTrigger] };

// "Rush, Cleave / the same": the keyword list is the radiant face's, the text is unchanged.
export const radiant: Script = { triggers: [attackTrigger] };
