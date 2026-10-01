// C #75 Argusland (SPEC §8.6 row 75; §4.4 step 2; R18, R44, R125, R386). Field Spell, cost 1, Rare.
//   Base:    "Aura: Damage to your hero is divided by {divisor}, rounded up." (2: halved)
//   Radiant: the same with divisor 4 (quartered).
//   Engine:  a hero damage multiplier after Armor and before the hit caps; several multiply. Fatigue is
//            damage and is reduced too (R125); losing health is not (R18).
//
// The engine's hero guard (`Script.heroGuard`): the pipeline, R44's lethal projection and the lethal
// window all read the same divided amount. Both faces run one script; `param` reads the face's divisor.

import { param, type Script } from "@jackioh/engine";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classic-075");

export const base: Script = {
  heroGuard: (args) => [{ divisor: param(args, "divisor") }],
};

// The same script: the Radiant face's 4 is its declared `divisor`, which `param` reads off the running face.
export const radiant: Script = base;
