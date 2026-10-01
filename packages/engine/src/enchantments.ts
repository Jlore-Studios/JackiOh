// Enchantments that ride a card (docs/classic-sets.md B5 E39): lasting instructions stored on the
// instance and kept in every zone, which R78's reset leaves alone. Each is read where its rule acts:
// `returnAfterResolve` by the play pipeline's step 7 and by the cost calculation (its floor, E15),
// `castOnDraw` by the draw (§2.4), `targetEnemies` by a random cast's target picks (E12).

import type { Enchantment } from "@jackioh/shared";
import type { CardInstance } from "./state";

export function enchantmentsOf(instance: Pick<CardInstance, "enchantments">): readonly Enchantment[] {
  return instance.enchantments ?? [];
}

/** The card's enchantments of one kind, in the order they were given. */
export function enchantmentsOfKind<K extends Enchantment["kind"]>(
  instance: Pick<CardInstance, "enchantments">,
  kind: K,
): Extract<Enchantment, { kind: K }>[] {
  return enchantmentsOf(instance).filter((entry): entry is Extract<Enchantment, { kind: K }> => entry.kind === kind);
}

export function hasEnchantment(instance: Pick<CardInstance, "enchantments">, kind: Enchantment["kind"]): boolean {
  return enchantmentsOf(instance).some((entry) => entry.kind === kind);
}

/** Two enchantments are one when every field agrees, so a card is never given the same one twice. */
function sameEnchantment(a: Enchantment, b: Enchantment): boolean {
  return JSON.stringify(a) === JSON.stringify(b);
}

/**
 * B5 E39: put an enchantment on a card. It rides the card through every zone from now on and R78's
 * reset never takes it (`zones.resetInstance`); one the card already carries is not added twice, while
 * two of a kind that differ (two floors) are both kept for their reader to combine. Returns whether
 * the card gained it. Also what the play pipeline stamps with (Classic+ #14 Forever&'s "the next Spell
 * you play gains …").
 */
export function addEnchantment(instance: Pick<CardInstance, "enchantments">, enchantment: Enchantment): boolean {
  const held = instance.enchantments ?? [];
  if (held.some((entry) => sameEnchantment(entry, enchantment))) return false;
  instance.enchantments = [...held, { ...enchantment }];
  return true;
}

/**
 * R443: what a copy or a Fuse carries — every enchantment of the cards it is made from, once each, in
 * their order. Undefined when none carries any, so the card stores nothing.
 */
export function unitedEnchantments(
  instances: readonly Pick<CardInstance, "enchantments">[],
): Enchantment[] | undefined {
  const out: Enchantment[] = [];
  for (const instance of instances) {
    for (const entry of enchantmentsOf(instance)) {
      if (!out.some((held) => sameEnchantment(held, entry))) out.push({ ...entry });
    }
  }
  return out.length === 0 ? undefined : out;
}
