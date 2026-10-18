// The per-card states a face draws beside its printed text (SPEC §10.8, patch v0.2.0): a Brittle count
// (B3.3, R385), the enchantments riding the card (B5 E39), a card standing in a unit zone as a Unit
// (B3.1, R383), a card Degrade or Upgrade changed (B3.4, R386, tuning.ts) and each open quest of a
// quest line with its progress, "1/2" (Classic #90 In Too Deep, B5 E33, R404). Each is a badge on the
// face — a glyph with the count where there is one, its words as the tooltip and the accessible name —
// and a line in the inspect overlays, in the words below (R373, R432: "(N) Cost" the noun, "costs (N)"
// the verb, so a floor is "can't cost less than (2)").
//
// `stateBadges` lists a face's badges in the order the rail draws them: the Brittle count first (it is
// the one that ends the card), then the open quests (what the card is doing), the tuned mark, the
// enchantments and the animated mark. Everything
// is read off the FaceModel, which read it off the view (CLAUDE.md rule 7); a face with none of them
// has no badge at all.

import type { CardType, Enchantment, QuestView } from "@jackioh/shared";

import { t } from "../i18n/i18n.ts";
import type { IconName } from "./icons.tsx";
import type { FaceModel } from "./model.ts";
import { CHINESE_TERMS, readerTerms } from "./chinese.ts";
import { VERDICT_GLYPH, VERDICT_WORD, tuningSummary } from "./tuning.ts";

export type StateBadgeKind = "brittle" | "quest" | "tuned" | Enchantment["kind"] | "animated" | "berserk" | "frozen" | "created";

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
  return t("Brittle {n}: crumbles at 0", { n: count });
}

/** R404: an open quest's progress against its goal, "1/2". */
export function questProgress(quest: QuestView["open"][number]): string {
  return `${String(quest.progress)}/${String(quest.goal)}`;
}

/** R404: an open quest in words, "Quest: Draw 2 cards (1/2)". */
export function questWords(quest: QuestView["open"][number]): string {
  // `quest.text` is the engine's English (the view's), looked up as it is displayed.
  return t("Quest: {text} ({progress})", { text: t(quest.text), progress: questProgress(quest) });
}

/**
 * E39, R432: an enchantment in a player's words. `radiant` is the face it rides, which Classic #55's
 * swap reads: a Radiant card becomes a Radiant Book (R671).
 */
export function enchantmentWords(enchantment: Enchantment, radiant = false): string {
  switch (enchantment.kind) {
    case "returnAfterResolve":
      return t("Returns to hand · can't cost less than ({floor})", { floor: enchantment.floor });
    case "castOnDraw":
      return t("Cast on draw");
    case "targetEnemies":
      return t("Targets enemies");
    case "swapsBook":
      return radiant ? t("End of turn: Become a random other Radiant Book") : t("End of turn: Become a random other Book");
  }
}

/** B5 E35, §6.1: what a Berserk unit's badge says. */
export const BERSERK_WORDS = "Berserk: at the start and end of its controller's turn it attacks its own hero";

/** R1174000, §6.1: what a Frozen unit's badge says. */
export const FROZEN_WORDS = "Frozen: can't attack or switch position; it thaws at the end of the turn it could have acted in";

/** R1174000: the frost badge's glyph, drawn in text presentation (U+FE0E) so it takes the badge's colour. */
export const FROZEN_GLYPH = "\u2744\uFE0E";

const ENCHANTMENT_ICON: Readonly<Record<Enchantment["kind"], IconName>> = {
  returnAfterResolve: "returnHand",
  castOnDraw: "castOnDraw",
  targetEnemies: "target",
  swapsBook: "cog",
};

/**
 * R383: a Field Spell, Trap or Field Trap standing in a unit zone as a Unit, and where an "Animated on
 * your turn" card goes back to (`UnitView.animated.home`, a backrow lane).
 */
export function animatedWords(animated: { home?: number }, type: CardType): string {
  if (animated.home === undefined) {
    return t("Animated: this {type} stands in a unit zone as a Unit", { type: readerTerms()?.types[type] ?? type });
  }
  return t("Animated on your turn: back to its backrow (lane {lane}) at the end of its controller's turn", { lane: animated.home });
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
  for (const quest of face.quest?.open ?? []) {
    badges.push({
      kind: "quest",
      text: questProgress(quest),
      icon: null,
      words: questWords(quest),
      data: { "data-quest": quest.id, "data-progress": String(quest.progress), "data-goal": String(quest.goal) },
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
      words: enchantmentWords(enchantment, face.radiant),
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
  // B5 E35: a Berserk unit, in its glossary row's words (§6.1).
  if (face.berserk === true) {
    badges.push({ kind: "berserk", text: null, icon: "sword", words: t(BERSERK_WORDS), data: { "data-berserk": "true" } });
  }
  // R1174000: a Frozen unit, the frost badge.
  if (face.frozen === true) {
    badges.push({ kind: "frozen", text: FROZEN_GLYPH, icon: null, words: t(FROZEN_WORDS), data: { "data-frozen": "true" } });
  }
  // MD-B6, R943: a Created card, made during the game.
  if (face.created === true) {
    badges.push({
      kind: "created",
      text: face.chinese === true ? CHINESE_TERMS.created : "Created",
      icon: null,
      words: t("Created: made during the game"),
      data: { "data-created": "true" },
    });
  }
  return badges;
}
