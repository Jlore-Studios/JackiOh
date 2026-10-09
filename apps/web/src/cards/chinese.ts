// A card's words in Simplified Chinese (ME-CN, R1300–R1303), read from the two sidecars beside the
// catalog (`@jackioh/cards/chinese.json` and `@jackioh/cards/chinese-terms.json`, which the aliases
// resolve to crates/cards/), the way flavour.ts reads its own: the bundle carries the words, and no
// server is asked. Presentation only (CLAUDE.md rule 7): no rule reads them.
//
// The first table is keyed by catalog id: each card's and token's name and both faces' texts, each
// text with exactly the `{key}`s its English face has (an English `{key|singular|plural}` is a plain
// `{key}` there, a measure word beside it), and for a card that previews a formula (R280) its labels,
// English template to Chinese template (`previews`). The second holds the frame's words: the types,
// tags, rarities and keywords, the labels, "Radiant" and "Created", and the glossary's entries by
// GLOSSARY's ids (R1303). crates/cards' cross tests hold both to their contract; chinese.test.tsx
// proves the glossary names exactly GLOSSARY's entries.
//
// A card is drawn in Chinese only where the view says it is (`CardView.chinese`, R1301), so every
// face of it in a match, and only there: the collection's faces are the catalog's. A fused card
// (R468) has no entry of its own, and its Chinese is its ingredients', joined as the engine joins
// their English (crates/engine/src/subsystems/fuse.rs). A definition with no entry and no
// ingredients (the hidden sentinel, Craft a Card's) keeps its English, and so does Glitch, whose
// words stay corrupted in every language (glitch.ts).

import chineseJson from "@jackioh/cards/chinese.json";
import termsJson from "@jackioh/cards/chinese-terms.json";

import {
  fillParams,
  type CardDef,
  type CardType,
  type Keyword,
  type KeywordKind,
  type Rarity,
  type Tag,
} from "@jackioh/shared";

import { isGlitch } from "./glitch.ts";
import type { GlossaryTermId } from "./glossary.ts";

/** One card's entry: its name and both faces' texts, and its preview labels' templates (R280). */
export type ChineseEntry = {
  readonly name: string;
  readonly base: string;
  readonly radiant: string;
  /** English label template → Chinese label template, each written with the face's `{key}`s. */
  readonly previews?: Readonly<Record<string, string>>;
};

/** A glossary entry's words in Chinese (R1303): what its label and its rule read. */
export type ChineseGlossaryEntry = { readonly label: string; readonly rule: string };

/** The frame's words (R1303). */
export type ChineseTerms = {
  readonly types: Readonly<Record<CardType, string>>;
  readonly tags: Readonly<Record<Tag, string>>;
  readonly rarities: Readonly<Record<Rarity, string>>;
  readonly keywords: Readonly<Record<KeywordKind, string>>;
  /** A label as the English prints it ("Cry:") → as the Chinese does ("战吼："). */
  readonly labels: Readonly<Record<string, string>>;
  readonly radiant: string;
  readonly created: string;
  /** R1382: what the frame prints in place of the five tribal tags on a card with them all. */
  readonly allTribes: string;
  readonly glossary: Readonly<Record<GlossaryTermId, ChineseGlossaryEntry>>;
};

export const CHINESE: Readonly<Record<string, ChineseEntry>> = chineseJson;

export const CHINESE_TERMS: ChineseTerms = termsJson;

/** What joins a list of keywords on a Chinese face, as the table's texts join them ("嘲讽，圣盾，复生"). */
export const CHINESE_COMMA = "，";

/** R102, R468: what joins a fused definition's ingredients' texts (one per line) and names. */
const FUSED_LINE = "\n";
const FUSED_NAME = " + ";

function entryOf(defId: string): ChineseEntry | undefined {
  return Object.hasOwn(CHINESE, defId) ? CHINESE[defId] : undefined;
}

/** R468: a fused definition's ingredients' entries, in order, or null when one has none. */
function ingredientEntries(def: CardDef): { entry: ChineseEntry; radiant: boolean }[] | null {
  const ingredients = def.ingredients;
  if (ingredients === undefined || ingredients.length === 0) return null;
  const entries = ingredients.map((ingredient) => ({ entry: entryOf(ingredient.defId), radiant: ingredient.radiant === true }));
  return entries.every((item): item is { entry: ChineseEntry; radiant: boolean } => item.entry !== undefined) ? entries : null;
}

/**
 * R1301: `def` with its name and both faces' texts in Chinese, everything else as it is. A fused
 * definition's are its ingredients' joined as the engine joins the English: the names with " + ",
 * the base text their faces line by line (R469: an ingredient fused on its Radiant face puts that
 * face in the base form too) and the Radiant text their Radiant faces. Any other definition with
 * no entry, and Glitch, comes back as given.
 */
export function chineseDef(def: CardDef): CardDef {
  if (isGlitch(def.id)) return def;
  const own = entryOf(def.id);
  if (own !== undefined) {
    return { ...def, name: own.name, base: { ...def.base, text: own.base }, radiant: { ...def.radiant, text: own.radiant } };
  }
  const parts = ingredientEntries(def);
  if (parts === null) return def;
  return {
    ...def,
    name: parts.map((part) => part.entry.name).join(FUSED_NAME),
    base: { ...def.base, text: parts.map((part) => (part.radiant ? part.entry.radiant : part.entry.base)).join(FUSED_LINE) },
    radiant: { ...def.radiant, text: parts.map((part) => part.entry.radiant).join(FUSED_LINE) },
  };
}

/**
 * R1301: the Chinese name of the card `defId` names: its entry's, or, given the definition, a fused
 * card's (`chineseDef`); else `english`, as given.
 */
export function chineseName(defId: string, english: string, def?: CardDef): string {
  if (isGlitch(defId)) return english;
  const own = entryOf(defId);
  if (own !== undefined) return own.name;
  if (def === undefined || ingredientEntries(def) === null) return english;
  return chineseDef(def).name;
}

/** R1301: a keyword as a Chinese face prints it: its word, with its number stuck to it ("护甲7"). */
export function chineseKeyword(keyword: Keyword): string {
  const word = CHINESE_TERMS.keywords[keyword.kind];
  return "n" in keyword ? `${word}${String(keyword.n)}` : word;
}

/**
 * R280, R1301: a preview label in Chinese. The card's templates (its own, or a fused card's
 * ingredients') are filled with the numbers its face is filled with (`fillParams`, the values in
 * play over the printed ones): the one whose English fill is `label` gives its Chinese template,
 * filled the same way. A label no template fills comes back as it is.
 */
export function chinesePreviewLabel(
  def: CardDef,
  radiant: boolean,
  label: string,
  values?: Readonly<Record<string, number>>,
): string {
  const face = radiant ? "radiant" : "base";
  const fill = (template: string): string =>
    fillParams({ params: def.params, base: { ...def.base, text: template }, radiant: { ...def.radiant, text: template } }, face, values);
  const ids = [def.id, ...(def.ingredients ?? []).map((ingredient) => ingredient.defId)];
  for (const id of ids) {
    for (const [english, chinese] of Object.entries(entryOf(id)?.previews ?? {})) {
      if (fill(english) === label) return fill(chinese);
    }
  }
  return label;
}
