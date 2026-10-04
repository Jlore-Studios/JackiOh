// C+ #74 Twice Forward One Step Backwards (SPEC §8.7 row 74, B7). (2) Field Trap, Mythic.
//   Base:    "Every {plays|card|cards} your opponent plays, the last one is fused into this after it
//            resolves, and this gains +{brittleGain} Brittle." (Brittle 2, balance patch 1)
//   Radiant: "Every {plays|card|cards} your opponent plays, a Radiant copy of the last one is fused
//            into this, and this gains +{brittleGain} Brittle." (Brittle 4)
//   Engine:  "A Field Trap (R425): a Trap is consumed when it fires, which would leave nothing to gain
//            Brittle. No Brittle while it is unrevealed: its printed Brittle never starts face-down
//            (R659); it reveals the first time it activates, and the count starts then. It counts the
//            opponent's plays since it was set (`memory.plays`; casts count, R70; a countered card was
//            never played). On each even count, once that card has resolved (§10.5 step 7), the card, if
//            it still exists (a Unit on the field, a Spell in the graveyard, a trap in the backrow), is
//            fused into this (Fuse, §6.3, R77, R102): this is the kept instance and stays a Field Trap,
//            and the opponent's card ceases to exist. The Radiant fuses in a Radiant copy of the card's
//            definition instead and leaves the card where it is. Either way this then gains +1 Brittle,
//            on every even count. The texts fused in work for you where they can … a fused Cry never
//            runs, since this is already on the field. It turns face-up, public, the first time it
//            activates (R33, R659); until then its play count is read by its controller only, and a card
//            destroyed while unrevealed reads public in its graveyard (R97). Tunes: Brittle 2 ↑ (its
//            X); every 2 ↓ (never below 2); Brittle gained 1 ↑."
//
// The whole card is its subsystem's trap trigger (`subsystems/twiceForward.ts`): the count on the
// instance, the fuse after the play resolves, the reveal with the first activation and the Brittle
// from then on. The printed Brittle is the catalog face's keyword (no start while face-down, R659;
// B3.4's X change tunes it), and "every N" and the Brittle gained are its declared `params`.

import type { PreviewHook, Script } from "@jackioh/engine";
import { subsystems } from "@jackioh/engine";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classicplus-074");

/**
 * SPEC §8.7 row 74: "until then its Brittle count and its play count are read by its controller only".
 * The play count rides the card's preview (R280), which a face-down card shows its controller alone
 * (R33); in play only, since the count starts as it is set.
 */
const preview: PreviewHook = ({ self, zone }) =>
  zone === "field" ? [{ label: "your opponent plays", value: subsystems.twiceForwardPlays(self) }] : [];

export const base: Script = { triggers: [subsystems.twiceForwardTrigger({ radiantCopy: false })], preview };

export const radiant: Script = { triggers: [subsystems.twiceForwardTrigger({ radiantCopy: true })], preview };
