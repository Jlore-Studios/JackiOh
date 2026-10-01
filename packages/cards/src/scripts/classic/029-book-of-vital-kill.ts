// C #29 Book of Vital Kill (SPEC §8.6 row 29, §6.3 Set health; R18, R81, R317). Spell, Book, cost 1,
// Epic.
//   Base:    "Set a hero's health to 13."
//   Radiant: "Set a hero's health to 13. Add a Book of Flame to your hand."
//   Engine:  "Set health (§6.3) on either hero, a declared target (R81): no pipeline, not damage and
//            not a heal, like R18's lose health. Radiant: adds C #16's base face (a card named without
//            "Radiant" is its base face); the hand cap applies. Tunes: none."
//
// THE TARGET is declared (R81): one hero, either side, so it travels in the play and `legalActions`
// offers both heroes. `setHealth` writes the number and emits `healthSet`; it runs no §4.4 step, so
// Armor, a hit cap, Lifesteal, a lethal-hit replacement and every "takes damage" or "is healed"
// trigger never see it, and the hero's Armor is left as it was.
//
// 13 is the card's own printed number, and the entry declares no `params` for it (a Degrade has
// nothing to move: 13 is good or bad depending on whose hero it is), so it is written here.
//
// THE RADIANT BOOK OF FLAME is C #16 named without "Radiant", so its base face: `addToHand` makes a
// fresh one (R57's radiant flag left off) through §2.4's pipeline, where a full hand burns it (R4,
// R317). Once in the hand it follows R97 like any hand card.

import type { Script } from "@jackioh/engine";
import { addToHand, setHealth } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classic-029");

/** The printed health the Book sets a hero to. */
const VITAL_HEALTH = 13;

/** C #16 Book of Flame, which the Radiant face adds on its base face. */
const BOOK_OF_FLAME = "classic-016";

export const base: Script = {
  targets: [{ kind: "target", min: 1, max: 1, filter: { side: "any", of: ["hero"] } }],
  cry: () => [setHealth({ to: { of: "chosen" }, value: VITAL_HEALTH })],
};

export const radiant: Script = {
  targets: base.targets,
  cry: () => [setHealth({ to: { of: "chosen" }, value: VITAL_HEALTH }), addToHand({ defId: BOOK_OF_FLAME })],
};
