// C+ #73 Call to Chaos (Classic+ Edition) (SPEC §8.7 row 73, B7, R28, R87, R423, R436). (4) Spell, Call
// to Chaos, Legendary.
//   Base:    "One random effect: Add 5 random Fruits to your hand, which cost (0); add 3 random Books to
//            your hand, which cost (0); destroy all enemy permanents; add 3 random Classic cards to your
//            hand, which cost (0); Upgrade every card in your hand and deck twice; fuse a random card
//            into each card in your deck, each keeping its cost; Degrade every card on your opponent's
//            field and in their hand three times; summon a Classic Golem; replace your deck with random
//            Call to Chaos cards, which cost (0); cast a random Call to Chaos."
//   Radiant: "Three different random effects, resolved in the order listed: …" (the same ten).
//
// Core #95's subsystem (`subsystems/callToChaos.ts`) with this edition's table (`CHAOS_PLUS_EFFECTS`,
// `subsystems/callToChaosPlus.ts`): the roll, the announcement both players read (R436), the chain cap
// counting casts of either edition (R28) and the order the Radiant's three resolve in (R423) are the one
// rule both editions share. The face is passed explicitly, as #95's file does.

import type { Script } from "@jackioh/engine";
import { subsystems } from "@jackioh/engine";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classicplus-073");

/** §8.7: "One random effect" of the ten. */
export const base: Script = { cry: () => [subsystems.callToChaos({ radiant: false, table: subsystems.CHAOS_PLUS_EFFECTS })] };

/** §8.7, R423: three different random effects of the ten, resolved in the order the list writes them. */
export const radiant: Script = { cry: () => [subsystems.callToChaos({ radiant: true, table: subsystems.CHAOS_PLUS_EFFECTS })] };
