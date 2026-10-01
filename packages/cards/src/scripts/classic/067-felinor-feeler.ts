// C #67 Felinor Feeler (SPEC §8.6 row 67). (1) Unit, Human, Common, 2/4 → 4/8.
//   Base:    "Pierce\nCry: Switch every enemy Unit to Defense Position."
//   Radiant: "Pierce, Rush\nCry: Switch every enemy Unit to Defense Position."
//   Engine:  "An effect's switch, which spends no exertion (R20); units already in Defense stay (R91);
//            #65.1 Spikey Pillow never enters Defense. Tagged Human, not Felinor, as the designer
//            tagged it. Tunes: none."
//
// Pierce and Rush are catalog keywords (§6.1, R346: its hits skip Armor). The Cry switches each enemy
// Unit acting on the field — the top of each pile (R13), an animated card in the unit row included
// (R383) — to Defense Position as an effect (`switchPositionOf` with `to: "DEF"`): no exertion is spent
// (R20), a unit already in Defense is asked for the position it holds and nothing happens (R91), and a
// unit that can't be in Defense Position (Spikey Pillow's `neverDefense`, §4.1) is refused and stays in
// Attack. The set of units is read once, as the Cry reaches it (`forEachCard`, R113). The Radiant face
// differs only in its catalog stats and Rush, so it runs the same script.
//
// Rulings: R20, R91, R13, R346. Its proof: `test/classic/067-felinor-feeler.test.ts`.

import { activeUnitsOf, type Script } from "@jackioh/engine";
import { forEachCard, switchPositionOf } from "@jackioh/engine/effects";
import { opponentOf } from "@jackioh/shared";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classic-067");

export const base: Script = {
  cry: () => [
    forEachCard({
      cards: (ctx) => activeUnitsOf(ctx.state, opponentOf(ctx.controller)),
      each: (instanceId) => switchPositionOf({ to: "DEF", target: { of: "instance", instanceId } }),
    }),
  ],
};

// The same script: the Radiant face's 4/8 and its added Rush are catalog data.
export const radiant: Script = base;
