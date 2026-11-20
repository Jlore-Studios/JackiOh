// A card's words in Simplified Chinese (ME-CN, R1300–R1303) for the lists and filters that name cards, read from the two sidecars beside the catalog
// (`crates/cards/chinese.json` and `crates/cards/chinese-terms.json`, loaded on demand by `loadCardTables`):
// the bundle carries the words, and no server is asked. Presentation only (CLAUDE.md rule 7): no rule reads them.
// #4180: the tables arrive through `import()`, so the English bundle carries none; an English reader never
// downloads them, and every lookup falls back to the English until they arrive.
//
// The first table is keyed by catalog id: each card's and token's name and both faces' texts, each text with
// exactly the `{key}`s its English face has (an English `{key|singular|plural}` is a plain `{key}` there, a
// measure word beside it), and its preview labels (`previews`). It is written in each card's `zh` in
// crates/cards/card-text.yaml (#2498), which crates/tools/tests/export_card_text.rs turns into
// `chinese.json`; its `"//"` entry says so and is dropped here. The second holds the frame's words.
// crates/cards' cross tests hold both to their contract; chinese.test.tsx proves the glossary names exactly
// GLOSSARY's entries.
//
// A card is drawn in Chinese where the view says it is (`CardView.chinese`, R1301), whatever the reader's
// language, and for a reader who reads Chinese (R711000) every card is drawn from these tables, in a match
// and in the collection alike.

import type { CardDef, CardType, KeywordKind, Rarity, Tag } from "@jackioh/shared";

import { getLocale, isZhCn, type Locale } from "../i18n/i18n.ts";
import { loadersByStem, type Loader } from "../patches/loaders.ts";
import { isRecord } from "../wire/json.ts";
import { DORMANT_STARSHIP_ID, holdsDormantStarship } from "./fused.ts";
import { isGlitch } from "./glitch.ts";

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
  readonly glossary: Readonly<Record<string, ChineseGlossaryEntry>>;
};

/**
 * #4180: the two sidecars' loaders, by stem (`loadersByStem`), through a lazy glob: Vite keeps both
 * JSON files out of every bundle until `loadCardTables()` asks for them. Keep the glob path-based:
 * Vite rejects package specifiers (patches/source.ts). The glob runs inside `loadCardTables()`,
 * never at import: the GitHub-releases script (under `tsx`, where `import.meta.glob` does not
 * exist) imports this module through `ChangeList.tsx` but never loads the tables.
 */
let tableLoaders: Record<string, Loader> | null = null;

function getTableLoaders(): Record<string, Loader> {
  if (tableLoaders === null) {
    tableLoaders = loadersByStem(import.meta.glob<unknown>("../../../../crates/cards/chinese*.json", { import: "default" }));
  }
  return tableLoaders;
}

/** The cards' table, filled in place by `loadCardTables()`: the same object `CARD_TABLES` reads. */
export const CHINESE: Record<string, ChineseEntry> = {};

/** The frame's table, filled in place by `loadCardTables()`: the same object `CARD_TABLES` reads. */
export const CHINESE_TERMS = {} as ChineseTerms;

let tablesReady = false;
let pendingTables: Promise<void> | null = null;

/** #4180, R711000: whether `loadCardTables()` has filled the tables: every lookup reads English until it has. */
export function cardTablesReady(): boolean {
  return tablesReady;
}

/**
 * #4180, R711000: loads the two sidecars once and fills `CHINESE` and `CHINESE_TERMS` in place, so the
 * 20+ call sites and the tests keep reading the same objects. A failed download leaves the tables
 * empty and clears the pending load, so the language stays as it was and the next call tries again.
 */
export function loadCardTables(): Promise<void> {
  if (tablesReady) return Promise.resolve();
  if (pendingTables !== null) return pendingTables;
  let loaders: Record<string, Loader>;
  try {
    loaders = getTableLoaders();
  } catch {
    return Promise.reject(new Error("the Chinese card tables did not load"));
  }
  const cards = loaders["chinese"];
  const terms = loaders["chinese-terms"];
  if (cards === undefined || terms === undefined) {
    return Promise.reject(new Error("the Chinese card tables did not load"));
  }
  pendingTables = Promise.all([cards(), terms()])
    .then(([cardsTable, termsTable]) => {
      if (!isRecord(cardsTable) || !isRecord(termsTable)) throw new Error("the Chinese card tables did not load");
      const { "//": _generated, ...entries } = cardsTable;
      Object.assign(CHINESE, entries);
      Object.assign(CHINESE_TERMS, termsTable);
      tablesReady = true;
      pendingTables = null;
    })
    .catch((cause: unknown) => {
      pendingTables = null;
      throw cause;
    });
  return pendingTables;
}

/** Empties the tables, back to before `loadCardTables()`. Tests only. */
export function resetCardTablesForTests(): void {
  for (const key of Object.keys(CHINESE)) delete CHINESE[key];
  for (const key of Object.keys(CHINESE_TERMS)) delete (CHINESE_TERMS as Record<string, unknown>)[key];
  tablesReady = false;
  pendingTables = null;
}

/** A language's card tables: the pair of sidecars beside the catalog (R1303). */
export type CardTable = { readonly cards: Readonly<Record<string, ChineseEntry>>; readonly terms: ChineseTerms };

/** The card tables by locale (R711000): a reader whose language has none reads the catalog's English. */
const CARD_TABLES: Readonly<Partial<Record<Locale, CardTable>>> = {
  "zh-CN": { cards: CHINESE, terms: CHINESE_TERMS },
};

/**
 * R711000: whether a card is drawn from the Chinese table: the view flagged it, or the reader reads
 * Chinese. Read where a face or a name is built, never cached across a language switch.
 */
export function drawnInChinese(flagged: boolean): boolean {
  return flagged || isZhCn();
}

/**
 * R711000: the frame's words in the reader's language, or undefined where it has no card table. Read
 * where a label is built, never cached across a language switch. #4180: undefined until the tables
 * arrive, as `t()` is the English for a missing entry.
 */
export function readerTerms(): ChineseTerms | undefined {
  return tablesReady ? CARD_TABLES[getLocale()]?.terms : undefined;
}

/** R711000: a catalog card's name as the reader reads it: its entry's in Chinese, else the catalog's. */
export function readerName(def: CardDef): string {
  return drawnInChinese(false) ? chineseName(def.id, def.name, def) : def.name;
}

/** R711000: what joins a list of phrases in the reader's language: "; ", or "；" for a reader of Chinese. */
export function listSeparator(): string {
  return isZhCn() ? "；" : "; ";
}

/** What joins a list of keywords on a Chinese face, as the table's texts join them ("嘲讽，圣盾，复生"). */
export const CHINESE_COMMA = "，";

/** R102, R468: what joins a fused definition's ingredients' texts (one per line) and names. */
const FUSED_LINE = "\n";
const FUSED_NAME = " + ";

function entryOf(defId: string): ChineseEntry | undefined {
  // #4180: no entry until the tables arrive: every lookup falls back to the English, and the gates
  // load them best-effort beside the module, so a failed chunk keeps reading English.
  return tablesReady && Object.hasOwn(CHINESE, defId) ? CHINESE[defId] : undefined;
}

/** R468: a fused definition's ingredients' entries, in order, or null when one has none. */
function ingredientEntries(def: CardDef): { entry: ChineseEntry; radiant: boolean }[] | null {
  const ingredients = def.ingredients;
  if (ingredients === undefined || ingredients.length === 0) return null;
  const entries = ingredients.map((ingredient) => ({ entry: entryOf(ingredient.defId), radiant: ingredient.radiant === true }));
  return entries.every((item): item is { entry: ChineseEntry; radiant: boolean } => item.entry !== undefined) ? entries : null;
}

/**
 * R3045000: the Dormant Starship's name in Chinese, for a fusion that holds its hull: a Starship keeps the hull's
 * name whatever was fused into it, as the English face does (`faceModel`); undefined for any other definition.
 */
function hullNameOf(def: CardDef): string | undefined {
  return holdsDormantStarship(def) ? entryOf(DORMANT_STARSHIP_ID)?.name : undefined;
}

/**
 * R1301: `def` with its name and both faces' texts in Chinese. A fused one's (R468) are its ingredients'
 * joined as crates/engine/src/subsystems/fuse.rs joins the English (R469: one fused on its Radiant face puts
 * that face in the base text too), but a fused Dormant Starship's name is the hull's (R3045000). Any other
 * with no entry, and Glitch (glitch.ts), comes back as given.
 */
export function chineseDef(def: CardDef): CardDef {
  if (isGlitch(def.id)) return def;
  const own = entryOf(def.id);
  if (own !== undefined) {
    return { ...def, name: own.name, base: { ...def.base, text: own.base }, radiant: { ...def.radiant, text: own.radiant } };
  }
  const hull = hullNameOf(def);
  const parts = ingredientEntries(def);
  if (parts === null) return hull === undefined ? def : { ...def, name: hull };
  return {
    ...def,
    name: hull ?? parts.map((part) => part.entry.name).join(FUSED_NAME),
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
  if (def === undefined || (ingredientEntries(def) === null && hullNameOf(def) === undefined)) return english;
  return chineseDef(def).name;
}

/**
 * R711000: a keyword or glossary term in a tuning change's English ("Armor 2") as the reader reads it
 * ("护甲2"); as given when the reader's language has no table or the text names no term.
 */
export function readerKeyword(text: string): string {
  const terms = readerTerms();
  if (terms === undefined) return text;
  const words: Record<string, string> = {};
  for (const [id, entry] of Object.entries(terms.glossary)) words[id] = entry.label;
  Object.assign(words, terms.keywords);
  const id = Object.keys(words)
    .filter((candidate) => text === candidate || text.startsWith(`${candidate} `))
    .sort((a, b) => b.length - a.length)[0];
  return id === undefined ? text : `${words[id] ?? id}${text.slice(id.length).trim()}`;
}
