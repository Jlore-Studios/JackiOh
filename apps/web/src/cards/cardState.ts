// The per-card states a face draws beside its printed text (SPEC §10.8, patch v0.2.0): a Brittle count
// (B3.3, R385), the enchantments riding the card (B5 E39), a card standing in a unit zone as a Unit
// (B3.1, R383) and a card Degrade or Upgrade changed (B3.4, R386, tuning.ts). Each is a badge on the
// face — a glyph with the count where there is one, its words as the tooltip and the accessible name —
// and a line in the inspect overlays, in the words below (R373, R432: "(N) Cost" the noun, "costs (N)"
// the verb, so a floor is "can't cost less than (2)").
//
// `stateBadges` lists a face's badges in the order the rail draws them: the Brittle count first (it is
// the one that ends the card), then the tuned mark, the enchantments and the animated mark. Everything
// is read off the FaceModel, which read it off the view (CLAUDE.md rule 7); a face with none of them
// has no badge at all.

import type { CardType, Enchantment } from "@jackioh/shared";

import type { IconName } from "./icons.tsx";
import type { FaceModel } from "./model.ts";
import { VERDICT_GLYPH, VERDICT_WORD, tuningSummary } from "./tuning.ts";

export type StateBadgeKind = "brittle" | "tuned" | Enchantment["kind"] | "animated";

export type StateBadge = {
  kind: StateBadgeKind;
  /** What the badge shows: a count or a glyph; null for an icon alone. */
  text: string | null;
  /** The icon drawn under the text, when the badge has one. */
  icon: IconName | null;
  /** The tooltip, the accessible name and the inspect overlay's line. */
  words: string;
  /** `data-*` values the badge carries for tests and styles. */
  data: Readonly<Record<string, string>>;
};

/** E21: what a pile's depth badge says, a backrow pile's as a unit pile's (game/Card.tsx). */
export const PILE_WORDS = "Cards buried under this pile";

/** R385: "Brittle 2: crumbles at 0". */
export function brittleWords(count: number): string {
  return `Brittle ${String(count)}: crumbles at 0`;
}

/** E39, R432: an enchantment in a player's words. */
export function enchantmentWords(enchantment: Enchantment): string {
  switch (enchantment.kind) {
    case "returnAfterResolve":
      return `Returns to hand · can't cost less than (${String(enchantment.floor)})`;
    case "castOnDraw":
      return "Cast on draw";
    case "targetEnemies":
      return "Targets enemies";
  }
}

const ENCHANTMENT_ICON: Readonly<Record<Enchantment["kind"], IconName>> = {
  returnAfterResolve: "returnHand",
  castOnDraw: "castOnDraw",
  targetEnemies: "target",
};

/**
 * R383: a Field Spell, Trap or Field Trap standing in a unit zone as a Unit, and where an "Animated on
 * your turn" card goes back to (`UnitView.animated.home`, a backrow lane).
 */
export function animatedWords(animated: { home?: number }, type: CardType): string {
  if (animated.home === undefined) return `Animated: this ${type} stands in a unit zone as a Unit`;
  return `Animated on your turn: back to its backrow zone (lane ${String(animated.home)}) at the end of its controller's turn`;
}

/** The enchantments a face shows, each distinct one once, in the order the view lists them. */
export function distinctEnchantments(list: readonly Enchantment[] | undefined): Enchantment[] {
  const seen = new Set<string>();
  const out: Enchantment[] = [];
  for (const entry of list ?? []) {
    const key = JSON.stringify(entry);
    if (seen.has(key)) continue;
    seen.add(key);
    out.push(entry);
  }
  return out;
}

/** Every state badge a face draws, in rail order; empty for a face with none. */
export function stateBadges(face: FaceModel): StateBadge[] {
  const badges: StateBadge[] = [];
  const brittle = face.brittle;
  if (brittle !== undefined && brittle !== null) {
    badges.push({
      kind: "brittle",
      text: String(brittle),
      icon: "brittle",
      words: brittleWords(brittle),
      data: { "data-brittle": String(brittle) },
    });
  }
  const tuning = face.tuning;
  if (tuning !== undefined && tuning !== null) {
    badges.push({
      kind: "tuned",
      text: VERDICT_GLYPH[tuning.verdict],
      icon: null,
      words: tuningSummary(tuning),
      data: { "data-tuned": tuning.verdict, "data-tuned-word": VERDICT_WORD[tuning.verdict] },
    });
  }
  for (const enchantment of distinctEnchantments(face.enchantments)) {
    badges.push({
      kind: enchantment.kind,
      text: null,
      icon: ENCHANTMENT_ICON[enchantment.kind],
      words: enchantmentWords(enchantment),
      data: {
        "data-enchantment": enchantment.kind,
        ...(enchantment.kind === "returnAfterResolve" ? { "data-floor": String(enchantment.floor) } : {}),
      },
    });
  }
  const animated = face.animated;
  if (animated !== undefined && animated !== null) {
    badges.push({
      kind: "animated",
      text: null,
      icon: "cog",
      words: animatedWords(animated, face.type),
      data: { "data-animated": animated.home === undefined ? "true" : String(animated.home) },
    });
  }
  return badges;
}
