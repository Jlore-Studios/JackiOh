// The per-card states a face draws beside its printed text (SPEC §10.8, patch v0.2.0): a Brittle count
// (B3.3, R385), a hand card's Temporary count (#2437, R637, R2179001), the enchantments riding the card (B5 E39), a card standing in a unit zone as a Unit
// (B3.1, R383), a card Degrade or Upgrade changed (B3.4, R386, tuning.ts) and each open quest of a
// quest line with its progress, "1/2" (Classic #90 In Too Deep, B5 E33, R404), with a staged quest's
// stage, "2/3" (Stinkyscape's staged Quests, R1854000). Each is a badge on the
// face — a glyph with the count where there is one, its words as the tooltip and the accessible name —
// and a line in the inspect overlays, in the words below (R373, R432: "(N) Cost" the noun, "costs (N)"
// the verb, so a floor is "can't cost less than (2)"). A Quest card's rules box prints its stage and
// its open quests in the same words (questFace.ts, R2450000).
//
// `stateBadges` lists a face's badges in the order the rail draws them: the Brittle count first, then a
// hand card's Temporary count (both end the card), then the open quests and the stage (what the card is doing), the tuned mark, the
// enchantments and the animated mark. Everything
// is read off the FaceModel, which read it off the view (CLAUDE.md rule 7); a face with none of them
// has no badge at all.

import type { CardType, Enchantment, QuestView } from "@jackioh/shared";

import { t } from "../i18n/i18n.ts";
import type { IconName } from "./icons.tsx";
import { TEMPORARY_LAST_COUNT } from "./keywordVisuals.ts";
import type { FaceModel } from "./model.ts";
import { CHINESE_TERMS, readerTerms } from "./chinese.ts";
import { VERDICT_GLYPH, VERDICT_WORD, tuningSummary } from "./tuning.ts";

export type StateBadgeKind =
  | "brittle"
  | "temporary"
  | "quest"
  | "questStage"
  | "tuned"
  | Enchantment["kind"]
  | "animated"
  | "berserk"
  | "frozen"
  | "dormant"
  | "starship"
  | "created";

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

/** #2437: the Temporary badge's glyph, an hourglass in text presentation (U+FE0E), like DORMANT_GLYPH. */
export const TEMPORARY_GLYPH = "\u231B\uFE0E";

/** #2437, R637, R2179001: a Temporary card's badge words, with the count it has left in its owner's hand. */
export function temporaryWords(count: number): string {
  if (count > TEMPORARY_LAST_COUNT) return t("Temporary {n}: discarded after {n} of its owner's ends of turn", { n: count });
  return t("Temporary 1: discarded at the end of its owner's turn");
}

/** R404: an open quest's progress against its goal, "1/2". */
export function questProgress(quest: QuestView["open"][number]): string {
  return `${String(quest.progress)}/${String(quest.goal)}`;
}

/** R404: an open quest in words, "Quest: Draw 2 cards (1/2)"; R1854002: with the cost its play must have. */
export function questWords(quest: QuestView["open"][number]): string {
  // `quest.text` is the engine's English (the view's), looked up as it is displayed.
  const text = t(quest.text);
  const progress = questProgress(quest);
  return quest.cost === undefined
    ? t("Quest: {text} ({progress})", { text, progress })
    : t("Quest: {text}, a card that costs ({cost}) ({progress})", { text, cost: quest.cost, progress });
}

type QuestStageView = NonNullable<QuestView["stages"]>[number];

/** R1854000: the stage a staged line stands at — the one open, else the one it hears back with, else the last done — and its number. */
export function currentStage(stages: readonly QuestStageView[]): { stage: QuestStageView; number: number } | null {
  const at =
    [stages.findIndex((stage) => stage.state === "current"), stages.findIndex((stage) => stage.hearBack !== undefined)].find(
      (index) => index >= 0,
    ) ?? stages.map((stage) => stage.state).lastIndexOf("done");
  const stage = stages[at];
  return stage === undefined ? null : { stage, number: at + 1 };
}

/** R1854000, R1854004, R2450000: a stage named, "Stage 2 of 3, Comradery", with the turns left while the line waits to hear back with it. */
export function stageTitle(number: number, of: number, name: string, wait?: number): string {
  const words = { n: number, of, name };
  if (wait === undefined) return t("Stage {n} of {of}, {name}", words);
  if (wait <= 1) return t("Stage {n} of {of}, {name}, hearing back next turn", words);
  return t("Stage {n} of {of}, {name}, hearing back in {turns} turns", { ...words, turns: wait });
}

/**
 * R1854000, R1854004: a staged line in words — "Stage 2 of 3, Comradery", "Stage 2 of 3, The Interview,
 * hearing back in 2 turns", "Quest complete" — off the view's stages alone.
 */
export function stageWords(stages: readonly QuestStageView[]): string {
  if (stages.length > 0 && stages.every((stage) => stage.state === "done")) return t("Quest complete");
  const found = currentStage(stages);
  if (found === null) return "";
  return stageTitle(found.number, stages.length, t(found.stage.name), found.stage.hearBack);
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

/** R1174000, R2246000, §6.1: what a Frozen unit's badge says. */
export const FROZEN_WORDS = "Frozen: can't attack, switch position or use its Activate; it thaws at the end of the turn it could have acted in";

/** R1848000, R2246000, §6.1: what the badge of a unit Frozen forever says. */
export const FROZEN_FOREVER_WORDS = "Frozen forever: can't attack, switch position or use its Activate, and never thaws";

/** R1174000: the frost badge's glyph, drawn in text presentation (U+FE0E) so it takes the badge's color. */
export const FROZEN_GLYPH = "\u2744\uFE0E";

/** R1839000, §6.1: what a Dormant card's badge says when no countdown will wake it. */
export const DORMANT_WORDS = "Dormant: it can't attack or be attacked and nothing reaches it, until an effect awakens it";

/** R1839001: what a Dormant card's badge says on the last turn of its countdown. */
export const DORMANT_NEXT_TURN_WORDS =
  "Dormant: it awakens at the start of its controller's next turn; until then nothing reaches it";

/** R1839000: the Dormant badge's glyph, a crescent moon in text presentation (U+FE0E). */
export const DORMANT_GLYPH = "\u263E\uFE0E";

/** R1852000: the Starship badge's glyph, a comet in text presentation (U+FE0E). */
export const STARSHIP_GLYPH = "\u2604\uFE0E";

/** R1852000, R1852003, R1852005: a Starship's badge words — the pieces it holds, and whether it has launched. */
export function starshipWords(pieces: number, launched: boolean): string {
  if (launched) return t("Starship: launched with {n} Starship Pieces fused into it", { n: pieces });
  return t("Dormant Starship: {n} Starship Pieces fused into it; its Activate launches it", { n: pieces });
}

/** R1839000, R1839001: a Dormant card's badge words, with its countdown when it has one. */
export function dormantWords(turns: number | null): string {
  if (turns === null) return t(DORMANT_WORDS);
  if (turns <= 1) return t(DORMANT_NEXT_TURN_WORDS);
  return t("Dormant: it awakens {n} of its controller's turns from now; until then nothing reaches it", { n: turns });
}

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
  // #2437: a hand card's Temporary count, beside the moon's pattern (R1839000); both it and Brittle end the card.
  const temporary = face.temporary;
  if (temporary !== undefined && temporary !== null) {
    badges.push({
      kind: "temporary",
      text: `${TEMPORARY_GLYPH}${String(temporary)}`,
      icon: null,
      words: temporaryWords(temporary),
      data: { "data-temporary": String(temporary) },
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
  // R1854000: a staged quest's stage, "2/3", and the turns left while it waits to hear back (R1854004).
  const stages = face.quest?.stages ?? [];
  const staged = currentStage(stages);
  if (staged !== null) {
    const waiting = staged.stage.hearBack;
    badges.push({
      kind: "questStage",
      text: `${String(staged.number)}/${String(stages.length)}`,
      icon: null,
      words: stageWords(stages),
      data: {
        "data-stage": String(staged.number),
        "data-stages": String(stages.length),
        "data-stage-state": staged.stage.state,
        ...(waiting === undefined ? {} : { "data-hear-back": String(waiting) }),
      },
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
    // R1848000: one Frozen forever says it never thaws.
    const words = face.frozenForever === true ? FROZEN_FOREVER_WORDS : FROZEN_WORDS;
    badges.push({ kind: "frozen", text: FROZEN_GLYPH, icon: null, words: t(words), data: { "data-frozen": face.frozenForever === true ? "forever" : "true" } });
  }
  // R1839000, R1839001: a Dormant card, its countdown beside the moon when it has one.
  const dormant = face.dormant;
  if (dormant !== undefined && dormant !== null) {
    badges.push({
      kind: "dormant",
      text: dormant.turns === null ? DORMANT_GLYPH : `${DORMANT_GLYPH}${String(dormant.turns)}`,
      icon: null,
      words: dormantWords(dormant.turns),
      data: { "data-dormant": dormant.turns === null ? "true" : String(dormant.turns) },
    });
  }
  // R1852000, R1852005: a Starship, the count of the pieces fused into it beside the comet.
  const starship = face.starship;
  if (starship !== undefined && starship !== null) {
    badges.push({
      kind: "starship",
      text: `${STARSHIP_GLYPH}${String(starship.pieces)}`,
      icon: null,
      words: starshipWords(starship.pieces, starship.launched),
      data: {
        "data-starship": starship.launched ? "launched" : "dormant",
        "data-starship-pieces": String(starship.pieces),
      },
    });
  }
  // MD-B6, R943: a Created card, made during the game. R2647000: only the inspect state notes list it (CardStates.tsx draws it on no face).
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
