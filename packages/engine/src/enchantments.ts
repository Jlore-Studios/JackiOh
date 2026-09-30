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
