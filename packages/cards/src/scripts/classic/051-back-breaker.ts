// C #51 Back Breaker (SPEC §8.6 row 51). (1) Unit, Common, 3/2 → 6/4.
//   Base:    "Stack\nDeath: Destroy every backrow card."
//   Radiant: "Stack\nDeath: Destroy every enemy backrow card."
//   Engine:  "Both backrows (Radiant: the enemy's), face-down cards included: the top card of each
//            backrow zone (a dormant card under a backrow pile is not on the field, R13, §3.2).
//            Indestructible cards stay (#98 Heroic Power, C #84 Lockdown, C #90 In Too Deep). An
//            animated card (Animated, §6.1, R383) is in the unit row and is not backrow. Tunes: none."
//
// Stack is the catalog keyword (§6.2, §3.2): it may be played onto an occupied unit zone, burying the
// card beneath (R13), which resumes when it leaves. The Death is §4.5 step 3's hook, so it fires on any
// death — a destroy, a Tribute (a Sacrifice counts as one, §6.3), a combat — and never on a bounce or an
// exile, which are no deaths. It is a board sweep (`destroyAll` over the backrow row): every card that
// acts in a backrow zone is marked — face-down ones too, since a backrow scope reaches them — and only
// the top of a backrow pile, since a dormant card is not on the field (R13); the next state check
// collects them together (R59), the one beneath a destroyed top resuming. Indestructible ones keep
// their zones (R46). An animated card stands in the unit row (R383) and a carried Unit is no backrow
// card (R446), so neither is reached. Radiant: `side: "enemy"`, relative to the controller it died
// under (the Death hook runs as the snapshot's controller, R89).
//
// Rulings: R13, R46, R59, R383, R89. Its proof: `test/classic/051-back-breaker.test.ts`.

import type { Script } from "@jackioh/engine";
import { destroyAll } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classic-051");

export const base: Script = {
  death: () => [destroyAll({ side: "any", rows: ["backrow"] })],
};

export const radiant: Script = {
  death: () => [destroyAll({ side: "enemy", rows: ["backrow"] })],
};
