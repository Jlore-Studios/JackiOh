// C+ #73.1 Classic Golem (SPEC §8.7 row 73.1, B7). (4) Unit, Token (printed Legendary). 10/10 → 20/20.
//   Base:    "Rush, First Strike, Trample / After this attacks a Unit, it transforms into a random
//            Classic or Classic+ Unit. If this destroyed that Unit, the new Unit may attack again
//            this turn."
//   Radiant: "Rush, Trample, Divine Shield / After this attacks a Unit, it transforms into a random
//            Radiant Classic or Classic+ Unit. If this destroyed that Unit, the new Unit may attack
//            again this turn." (balance patch 1: no First Strike, a Radiant transform)
//   Engine:  "R424: the transform comes after the combat of an attack the Golem declared on a Unit, so
//            the Golem's own stats fight (First Strike, Trample's excess to the hero, §4.4); a Golem that
//            left the field in that combat transforms into nothing. It is transformed (Transform, §6.3)
//            in place into a random non-token Unit of the Classic or Classic+ set (the text names the
//            sets, R380), on its base face, with no Cry (R1); a Transform on the field keeps the type
//            (R35), and an Immutable Golem is not transformed. If the Golem was the defender's killer
//            (R42), 'may attack again' passes to the new Unit: its exertion is fresh and it is not
//            summoning sick this turn. Tunes: none."
//
// The engine's "after this attacks" hook (`afterAttack`) runs once the check that closes the Golem's
// combat has run, with the combat's facts (`afterAttackOf`): the target, the Units whose lethal hit was
// the Golem's (R42), whether it survived, and whether the attack was forced (R53) — "an attack it
// declared" is not a forced one. E24's `transformRandom` draws the Unit (R129: no draw when it cannot
// land) and `readyToAttack` lifts the new body's sickness (its exertion is a new instance's, fresh). The
// keywords are the catalog's faces, so both faces run this one script.

import type { Effect, EffectContext, Script } from "@jackioh/engine";
import { afterAttackOf } from "@jackioh/engine";
import { transformRandom } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classicplus-073-1");

/** §8.7: "a random Classic or Classic+ Unit" — non-token Units of the two sets the text names (R380). */
const CLASSIC_UNITS = { type: "Unit" as const, set: ["Classic" as const, "Classic+" as const] };

/** A hero target is `hero-<player>` in the combat's facts (`combat.targetIdOf`). */
const HERO_TARGET = "hero-";

function afterItAttacks(radiant: boolean): (ctx: EffectContext) => Effect[] {
  return (ctx) => {
    const combat = afterAttackOf(ctx);
    if (combat === null || combat.forced || !combat.survived || combat.targetId.startsWith(HERO_TARGET)) return [];
    const destroyed = combat.destroyedIds.includes(combat.targetId);
    return [
      transformRandom({
        target: { of: "self" },
        query: CLASSIC_UNITS,
        ...(radiant ? { radiant: true } : {}),
        readyToAttack: destroyed,
      }),
    ];
  };
}

export const base: Script = { afterAttack: afterItAttacks(false) };

// "Rush, Trample, Divine Shield; a random Radiant Unit": the Radiant face transforms into a Radiant
// Unit (balance patch 1); its 20/20 and keywords are its catalog face.
export const radiant: Script = { afterAttack: afterItAttacks(true) };
