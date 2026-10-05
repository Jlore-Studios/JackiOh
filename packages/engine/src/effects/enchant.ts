// Enchantments that ride a card (docs/classic-sets.md B5 E39): the verb that puts one on. Classic+ #40
// Appropriations' Education shuffles in Books that "have Cast on draw and aim at enemies when they
// harm and at your side when they help" — `enchant({ instanceId, enchantment: { kind: "castOnDraw" } })`
// and `{ kind: "targetEnemies" }` on each Book it made — and #14 Forever&'s "after this resolves, return
// it to your hand; it can't cost
// less than (2)" is `{ kind: "returnAfterResolve", floor }` (which the play pipeline stamps on the next
// Spell played, `enchantments.addEnchantment`).
//
// An enchantment is kept in every zone and never reset (R78 leaves it alone): each is read where its
// rule acts, by the module that owns that rule (`enchantments.ts`). No event reports one: it names no
// change a board shows, and each viewer reads a card's enchantments off its view where they may read
// the card (`CardView.enchantments`).

import type { Enchantment } from "@jackioh/shared";
import { addEnchantment } from "../enchantments";
import type { Effect } from "../script";
import type { CardInstance } from "../state";
import { cardsInCardScope, type CardScope } from "./cardScope";
import { instanceOnItsStay, resolveTarget, type TargetSpec } from "./targets";

/** B5 E39: put `enchantment` on one named card (a spec or an id a script captured), or on a scope's cards. */
export function enchant(args: {
  target?: TargetSpec;
  instanceId?: string;
  scope?: CardScope;
  enchantment: Enchantment;
}): Effect {
  return {
    kind: "enchant",
    apply(ctx): void {
      let cards: CardInstance[] = [];
      if (args.scope !== undefined) {
        cards = cardsInCardScope(ctx, args.scope).map((entry) => entry.card);
      } else if (args.instanceId !== undefined) {
        // R174: a card named by id is aimed at the stay it had when the run began.
        const card = instanceOnItsStay(ctx, args.instanceId);
        cards = card === null ? [] : [card];
      } else if (args.target !== undefined) {
        const target = resolveTarget(ctx, args.target);
        cards = target?.kind === "unit" ? [target.instance] : [];
      }
      for (const card of cards) {
        // A card that has ceased to exist (R11, R86) is in no pile to carry anything.
        if (card.zone.z !== "gone") addEnchantment(card, args.enchantment);
      }
    },
  };
}
