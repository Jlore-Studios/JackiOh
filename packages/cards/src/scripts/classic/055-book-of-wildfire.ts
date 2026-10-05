// C #55 Book of Wildfire (SPEC §8.6 row 55, BUILD M9 Classic row C 55). (1) Spell, Book, Epic.
//   Base:    "Deal {damage} damage. / End of turn: Become a different Book." (4)
//   Radiant: "Deal {damage} damage. / End of turn: Become a different Radiant Book." (8)
//   Engine:  "One targeted hit, as C #16 Book of Flame. The designer's second 'Book of Flame' is
//            renamed so that the two names never collide and no text that names Book of Flame (C #23
//            Devil's Pact, C #29 Book of Vital Kill) finds it (R381). At the end of its owner's turn,
//            while in their hand, it is Replaced by a random other non-token Book of any set (not
//            Wildfire), on the same face, which keeps swapping each turn (R671). Tunes: damage 4 ↑."
//
// "Deal N damage" with no target named is targeted (§8 Conventions, as #68 Twisted Sorcerer's is): one
// declared pick (R81) of any Unit on top of its pile or either hero, either side, and one §4.4 damage
// instance from this Spell, so Spell Damage raises it (§4.4 step 0) and Armor and Divine Shield meet
// it. A Unit Immune to Spells is never offered (R81, E35). With no Unit, the heroes remain, so the
// Spell always has a target. The number is the declared one, `param(ctx, "damage")` (R386). It is a
// Book (its tag), which C #4 Palantir's base face answers; it names no card and no card names it.
//
// The swap (Patch v0.2.X, #271, R671) is the engine's one Book-swap hand trigger (`BOOK_SWAP_TRIGGER`):
// on its owner's `turnEnded`, while this is in their hand, it becomes a Book drawn with the match rng
// from §5.1's pool — every non-token Book (R380), Book of Flame included, but neither this card nor
// Wildfire — in the same place in the hand and on the same face, so a Radiant Wildfire becomes the
// Radiant face of that Book. The Book it becomes has that Book's own text and carries the swap as the
// `swapsBook` enchantment, so it keeps changing every end of its owner's turn while it stays in hand.

import type { Script } from "@jackioh/engine";
import { BOOK_SWAP_TRIGGER, param } from "@jackioh/engine";
import { damage } from "@jackioh/engine/effects";
import type { TargetDecl } from "@jackioh/shared";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classic-055");

const targets: TargetDecl[] = [{ kind: "target", min: 1, max: 1, filter: { side: "any", of: ["unit", "hero"] } }];

export const base: Script = {
  targets,
  cry: (ctx) => [damage({ to: { of: "chosen" }, amount: param(ctx, "damage") })],
  handTriggers: [BOOK_SWAP_TRIGGER],
};

// The same script: the Radiant face differs in its declared damage (8), read through `param`, and in
// the face of the Book it becomes, which the swap reads off the card.
export const radiant: Script = base;
