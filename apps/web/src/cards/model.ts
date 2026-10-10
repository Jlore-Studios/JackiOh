// What a card face shows, computed once from a catalog def and the live view (docs/polish/6-cards.md,
// Surface B). Pure: no React, no DOM, no settings. Every component that draws a card — the board's
// Card.tsx, the inspect overlays, the deck builder — builds a FaceModel here and hands it down.
//
// Two kinds of face come out of it (SPEC §10.10). A face with no `inPlay` is the card as printed:
// the collection's, both faces of it in the catalog's words. A face with `inPlay` is the card as the
// view says it stands in a game (R243): the cost the view gives it, the stats of a Unit in its
// owner's hand with what it gained there (#89 Corpse Eater), a hand card's keywords where the view
// gives them (a Lucky given in hand, R1438), the numbers and keywords of a unit on the field, the
// Vanilla marker, and inPlay.ts's words where play and print part ways — a #98 Heroic Power's rolled
// power, "???" for Call to Chaos. A match-made definition (a Fuse's, R77) is just the `def` the
// caller found in the view's `defs`, and prints its own name, text and stats.
//
// Nothing here is a rule (CLAUDE.md rule 7). The tones compare two numbers the view and the catalog
// already carry — the live cost against the printed price, the live stats against the printed
// stats — so a buffed unit reads green and a damaged one red, the way Hearthstone colours them.
//
// Three marks ride on the text (SPEC §10.10). A Radiant face prints its whole catalog text with the
// stretches its base face does not have marked (`text.marks`, radiantDiff.ts, R277); the names its
// `refs` link are references (refs.ts, R279), which the renderer finds in the text; and in play the
// numbers the view's `preview` carries are printed after their formulas (`values`, R280).
//
// Patch v0.2.0's per-card states ride on a face in play as the view gives them (cardState.ts draws
// them): the Brittle count (B3.3, R385), the enchantments (B5 E39), the card standing in a unit zone
// as a Unit (B3.1, R383), and what Degrade and Upgrade changed (B3.4, R386, tuning.ts), whose numbers
// that moved are marked where the text prints them (`text.tuned`). The collection shows none of
// them. Every face carries its definition's lines of code (`loc`, E36) for the inspect overlays.
//
// An Animated Field Spell or Trap prints the attack and health of the Unit it becomes (B3.1 rule 1),
// so its face carries them as a Unit's does, wherever it is.
//
// A face in play carries a quest line as the view gives it (`quest`, B5 E33, R404: In Too Deep's open
// quests, their progress and rewards, its auras), which cardState.ts draws as badges. A copier (Classic
// #57 Echo, B5 E14, R399, R511) prints the Spell text the view says it has (`InPlay.copies`), filled
// with the numbers it reads on the card, in place of its own copying sentence; it keeps its own name,
// cost, type and art, and `copying` names the card it copies for the inspect notes.
//
// ME-CN, R1301: a card the view says is Chinese (`FaceSource.chinese`) prints its name and both
// faces' texts from the Chinese table (chinese.ts), a copier's copied text and a fused card's
// ingredients' included, filled with the same numbers; its preview labels are the table's too. A
// Radiant face's gold diff compares the two Chinese faces (R1302). The English face's name and text
// ride along (`englishName`, `englishText`), never printed: the art's motif and the glossary's terms
// are read from them.

import {
  fillParams,
  keywordKey,
  type CardDef,
  type CardMark,
  type CardFace as PrintedFace,
  type CardType,
  type Enchantment,
  type Keyword,
  type Param,
  type QuestView,
  type Rarity,
  type PreviewValue,
  type PrintedRarity,
  type SetName,
  type Tag,
  type Tuning,
} from "@jackioh/shared";

import {
  HEROIC_POWER_ID,
  VANILLA_TEXT,
  concealedInPlay,
  concealedText,
  powerText,
  type RolledPower,
} from "./inPlay.ts";
import { chineseDef, chinesePreviewLabel } from "./chinese.ts";
import { GLITCH_WORDS, isGlitch } from "./glitch.ts";
import { radiantMarks, type TextRange } from "./radiantDiff.ts";
import { faceTuning, filledText, type FaceTuning, type TunedRange } from "./tuning.ts";

export type FaceLayout = "full" | "compact" | "minion";
export type StatTone = "base" | "buffed" | "reduced" | "damaged";
export type FaceCost = {
  /**
   * What the gem shows: the view's live number whenever there is one (SPEC §10.10: the client
   * renders `viewFor`), "X" for an X card, else the printed price (an embiggen card's base).
   */
  text: string;
  /** `data-cost`: the live number when there is one (CardView.cost), else `text`. */
  value: string;
  tone: "base" | "down" | "up";
  /**
   * The embiggen price, shown small beside the gem while the gem shows the base price; null
   * otherwise, including once the live cost has moved off the base price (a discount, or a card
   * on the field that was paid its embiggen price).
   */
  alt: string | null;
};
export type FaceStats = {
  attack: number;
  health: number;
  maxHealth: number;
  attackTone: StatTone;
  healthTone: StatTone;
  /**
   * R277: on a printed Radiant face (no live numbers), which stats the Radiant face raised over the
   * base face's, so the face can mark them as it marks its text. Absent elsewhere.
   */
  grew?: { attack: boolean; health: boolean };
};
/**
 * What the rules box prints: the whole text, and on a Radiant face the stretches of it the base
 * face's text does not have (R277). `marks` is empty on a base face and wherever play prints
 * something else (a Vanilla unit, "???", a Heroic Power's power on its base face).
 */
export type FaceText = {
  full: string;
  marks: readonly TextRange[];
  /**
   * B3.4, R386: in play, the numbers in `full` the view moved off their printed values, each marked
   * better or worse (tuning.ts). Absent or empty everywhere else.
   */
  tuned?: readonly TunedRange[];
};
export type FaceModel = {
  defId: string;
  /** False when no catalog def was available (the `unknownCard` fallback). */
  known: boolean;
  name: string;
  type: CardType;
  tags: readonly Tag[];
  rarity: Rarity | null;
  /**
   * B2.5, R503: the rarity a token prints (`CardDef.printedRarity`), which its frame shows in place
   * of Token's; null (or absent, on a face built by hand) for every other card. Display only:
   * `rarity` stays "Token" for everything else.
   */
  printedRarity?: PrintedRarity | null;
  index: string | null;
  set: SetName | null;
  radiant: boolean;
  cost: FaceCost;
  /** Units only: live when `live` was given, else the printed face; null for non-units and unknown stats. */
  stats: FaceStats | null;
  /**
   * What the rules box prints: the face's catalog text whole, with a Radiant face's changes marked
   * (R277). A fused definition's text is its ingredients' texts line by line (R102), each Radiant
   * line marked against its own base line. In play it is what the card in play says (inPlay.ts),
   * which may differ from the printed text.
   */
  text: FaceText;
  /**
   * R279: the cards and tokens this card's text names (`CardDef.refs`), which the renderer links
   * where their names stand in `text.full`. Empty where the text in play names none ("???", Vanilla).
   */
  refs: readonly string[];
  /**
   * R280: in play, what the card's formula comes to now (`CardView.preview`), each value printed in
   * braces after its label. Always empty in the collection, and wherever play prints other words.
   */
  values: readonly PreviewValue[];
  /**
   * Live keywords when `live` was given, a hand card's as the view gives them (`InPlay.handKeywords`),
   * else the printed face's keywords.
   */
  keywords: readonly Keyword[];
  /** A face in a game (`FaceSource.inPlay` given) rather than the collection's. */
  inPlay: boolean;
  /** R243, §6.3 Vanilla: the unit's text is gone, and the rules box says so. */
  vanilla: boolean;
  /**
   * Keywords the card has now that its printed face does not print (a Plastic Surgery's, an aura's,
   * Defense Position's Taunt, every keyword a Vanilla unit still has, a Lucky given in hand): the
   * face prints them after its text, since the text no longer says them. Lucky is one entry at its
   * sum whenever the sum is not the printed one (R1438). Empty outside play.
   */
  gained: readonly Keyword[];
  /**
   * The collection's text for this face, when the face in play prints something else — a Heroic
   * Power's rolled power, a Vanilla unit — so the inspect overlays can show both. Null when the two
   * agree, outside play, and for a card whose text play keeps a mystery ("???").
   */
  printed: FaceText | null;
  /**
   * B3.3, R385: in play, the card's Brittle count as the view gives it (`CardView.brittle`). Absent or
   * null elsewhere, and on a card with none. Optional, as every field below is, for a face built by hand.
   */
  brittle?: number | null;
  /** B3.4, R386: in play, what Degrade, Upgrade and KY's Constant changed on the card; null when nothing did. */
  tuning?: FaceTuning | null;
  /** B5 E39: in play, the enchantments riding the card (`CardView.enchantments`). */
  enchantments?: readonly Enchantment[];
  /** B3.1, R383: a backrow card standing in a unit zone as a Unit (`UnitView.animated`); null otherwise. */
  animated?: { home?: number } | null;
  /** B5 E35: in play, the unit has gone Berserk (`UnitView.berserk`). */
  berserk?: boolean;
  /** MD-B6, R943: in play, the card was minted after the decks were built (`CardView.created`). */
  created?: boolean;
  /** E36: the lines of code of the card's script (`CardDef.loc`); a fused card's definition carries its ingredients' sum. */
  loc?: number | null;
  /** R437: in play, the marks on the card (`CardView.marks`), which the inspect overlays spell out. */
  marks?: readonly CardMark[];
  /** B5 E33, R404: in play, the card's quest line (`CardView.quest`); null for a card with none. */
  quest?: QuestView | null;
  /** B5 E14, R511: in play, the Spell a copier's text is now (`CardView.copies`); null when it copies none. */
  copying?: { defId: string; name: string; radiant: boolean } | null;
  /** ME-CN, R1301: the face prints its words in Chinese (`FaceSource.chinese`). Absent otherwise. */
  chinese?: boolean;
  /** R1301: on a Chinese face, the name the English face prints, which picks the art's motif (R503). */
  englishName?: string;
  /** R1301: on a Chinese face, the text the English face prints, whose terms the glossary explains. */
  englishText?: string;
};
/**
 * What a game adds to a face (R243, SPEC §10.10); its presence is what makes a face one in play.
 * `live` (a unit on the field) and `liveCost` stay where they were, beside it.
 */
export type InPlay = {
  /** R243: a Unit card's stats in its owner's hand, as they stand (`CardView.attack`, `.health`). */
  handStats?: { attack: number; health: number };
  /**
   * B5 E38, R1438: a hand card's keywords as the view gives them (`CardView.keywords`): its printed
   * ones as Degrade and Upgrade left them, and those it was given in the hand or the deck (a Lucky).
   */
  handKeywords?: readonly Keyword[];
  /** R243: the unit's text is gone (`UnitView.vanilla`). */
  vanilla?: boolean;
  /** R43, R243: the power a #98 Heroic Power rolled (R752). */
  power?: RolledPower;
  /** R280: what the card's formula comes to now (`CardView.preview`). */
  preview?: readonly PreviewValue[];
  /**
   * B3.4, R386: the card's declared numbers as they stand now (`CardView.params`), which fill its
   * text's `{key}`s in place of the printed values.
   */
  params?: Readonly<Record<string, number>>;
  /** B3.3, R385: the card's Brittle count (`CardView.brittle`). */
  brittle?: number;
  /** B3.4, R386: what Degrade and Upgrade changed on the card (`CardView.tuning`). */
  tuning?: Tuning;
  /** B5 E39: the enchantments riding the card (`CardView.enchantments`). */
  enchantments?: readonly Enchantment[];
  /** B3.1, R383: the card stands in a unit zone as a Unit (`UnitView.animated`). */
  animated?: { home?: number };
  /** B5 E35: the unit has gone Berserk (`UnitView.berserk`). */
  berserk?: true;
  /** MD-B6, R943: the card was minted after the decks were built (`CardView.created`). */
  created?: true;
  /** R437: the marks on the card (`CardView.marks`). */
  marks?: readonly CardMark[];
  /** B5 E33, R404: the card's quest line (`CardView.quest`). */
  quest?: QuestView;
  /** B5 E14, R399: the Spell a copier has the text of (`CardView.copies`), with its definition. */
  copies?: { def: CardDef; radiant: boolean; params?: Readonly<Record<string, number>> };
};
export type FaceSource = {
  defId: string;
  def?: CardDef;
  /** Fallbacks when `def` is absent (CardInfo.name, BackrowView.type). */
  name?: string;
  type?: CardType;
  radiant: boolean;
  /** CardView.cost. */
  liveCost?: number;
  /** UnitView's current numbers. */
  live?: { attack: number; health: number; maxHealth: number; keywords: readonly Keyword[] };
  /** Set on every face drawn in a game, absent in the collection (see the header). */
  inPlay?: InPlay;
  /** ME-CN, R1301: the view says the card is Chinese (`CardView.chinese`); its words are the table's. */
  chinese?: boolean;
  /** MD-B15, R923: the view's tags where they differ from the definition's (a granted tag). */
  tags?: readonly Tag[];
};

/** The gem of a card nobody can name: no catalog, no live cost. */
const UNKNOWN_COST = "?";

/** A card with no def is drawn as a Unit, exactly as `unknownCard` in game/catalog.ts reports it. */
const UNKNOWN_TYPE: CardType = "Unit";

export function faceModel(source: FaceSource): FaceModel {
  // R102, B3.4: a fused definition declares no numbers of its own, and its text still writes its
  // ingredients' `{key}`s; in play the view's numbers fill them (and are, for it, the printed ones).
  const filled = withViewParams(source.def, source.inPlay?.params);
  // ME-CN, R1301: a Chinese card's name and texts are the table's; its numbers, keywords and all else
  // stay the definition's.
  const chinese = source.chinese === true;
  const def = chinese && filled !== undefined ? chineseDef(filled) : filled;
  const printed = def === undefined ? undefined : source.radiant ? def.radiant : def.base;
  // B2.7: a face may carry its own type (Classic+ #22 Blood Moon's Radiant face is a Field Trap), and
  // the card's type is its face's (§5.2); in play the view's word for it comes first.
  const viewType = source.inPlay === undefined ? undefined : source.type;
  const type: CardType = viewType ?? printed?.type ?? def?.type ?? source.type ?? UNKNOWN_TYPE;
  const inPlay = source.inPlay;
  const vanilla = inPlay?.vanilla === true;
  const printedText = textOf(def, source.radiant);
  // B3.4: in play a card's numbers are the ones the view says it has now (a Degrade, an Upgrade).
  const ownText = inPlay?.params === undefined ? printedText : textOf(def, source.radiant, inPlay.params);
  // B5 E14, R511: a copier's own words are the copied Spell's text, filled with the numbers it reads
  // (in Chinese on a Chinese copier, R1301).
  const viewCopies = inPlay?.copies;
  const copies = viewCopies === undefined || !chinese ? viewCopies : { ...viewCopies, def: chineseDef(viewCopies.def) };
  const liveText = copies === undefined ? ownText : copiedText(ownText, def, textOf(copies.def, copies.radiant, copies.params));
  const text = inPlay === undefined ? printedText : textInPlay(def, source.radiant, liveText, inPlay);
  // B5 E38, R1438: a hand card's keywords are the view's where it gives them (a Lucky given in hand).
  const keywords = source.live?.keywords ?? inPlay?.handKeywords ?? printed?.keywords ?? [];
  // The values belong to the card's own words, its numbers as they stand included: a formula play
  // does not print (Vanilla, "???", a rolled power) has no value to show.
  const printsItsText = text.full === liveText.full;
  // B3.4, R386: what Degrade and Upgrade changed; the keywords they added are marked as such, so the
  // line of keywords gained since printing leaves them out.
  const tuning = inPlay === undefined ? null : faceTuning(def, source.radiant, inPlay.tuning, inPlay.params);
  const tunedKeys = new Set((tuning?.added ?? []).map(keywordKey));
  // R280, R1301: a Chinese card's preview labels are the table's, filled with the numbers its face is.
  const preview = inPlay?.preview ?? [];
  const values =
    chinese && filled !== undefined
      ? preview.map((entry) => ({ ...entry, label: chinesePreviewLabel(filled, source.radiant, entry.label, inPlay?.params) }))
      : preview;
  // R1301: the same card's English face, for what is read off its words and never printed.
  const english = chinese ? faceModel({ ...source, chinese: false }) : null;

  return {
    defId: source.defId,
    known: def !== undefined,
    name: def?.name ?? source.name ?? source.defId,
    type,
    // MD-B15, R923: in play a face lists the view's tags; outside play the definition's.
    tags: source.inPlay !== undefined && source.tags !== undefined ? source.tags : (def?.tags ?? []),
    rarity: def?.rarity ?? null,
    printedRarity: def?.printedRarity ?? null,
    index: def?.index ?? null,
    set: def?.set ?? null,
    radiant: source.radiant,
    // Glitch's gem shows glyphs, never a number (glitch.ts); `data-cost` still carries the view's.
    cost: isGlitch(source.defId) ? { ...costOf(def, source.liveCost), text: GLITCH_WORDS.cost, tone: "base", alt: null } : costOf(def, source.liveCost),
    stats: statsOf(type, def !== undefined, printed, source.live ?? handLive(inPlay?.handStats, printed), grewOf(def, source)),
    text,
    // The renderer links only the names that stand in the text, so play's own words link what they name.
    refs: [...(copies?.def.refs ?? []), ...(def?.refs ?? [])],
    values: printsItsText ? values : [],
    keywords,
    inPlay: inPlay !== undefined,
    vanilla,
    // A Vanilla unit prints no keyword of its own any more (§6.3), so every one it still has is gained.
    gained:
      inPlay === undefined || (source.live === undefined && inPlay.handKeywords === undefined)
        ? []
        : gainedKeywords(keywords, vanilla ? [] : (printed?.keywords ?? [])).filter((keyword) => !tunedKeys.has(keywordKey(keyword))),
    printed: inPlay === undefined || sameText(text, printedText) || concealed(def) ? null : printedText,
    brittle: inPlay?.brittle ?? null,
    tuning,
    enchantments: inPlay?.enchantments ?? [],
    animated: inPlay?.animated ?? null,
    berserk: inPlay?.berserk === true,
    created: inPlay?.created === true,
    loc: def?.loc ?? null,
    marks: inPlay?.marks ?? [],
    quest: inPlay?.quest ?? null,
    copying: copies === undefined ? null : { defId: copies.def.id, name: copies.def.name, radiant: copies.radiant },
    ...(english === null ? {} : { chinese: true, englishName: english.name, englishText: english.text.full }),
  };
}

/**
 * B5 E14, R511: a copier's text in play — the copied text where its own face prints its copying
 * sentence (its base text), so whatever else its face prints stays: a Radiant Echo still reads "Echo 1"
 * over the text it copies, marked as its Radiant face marks it. A face that does not print that
 * sentence prints the copied text alone.
 */
function copiedText(own: FaceText, def: CardDef | undefined, copied: FaceText): FaceText {
  const sentence = def === undefined ? "" : fillParams(def, "base");
  const at = sentence === "" ? -1 : own.full.indexOf(sentence);
  if (at < 0) return copied;
  const after = at + sentence.length;
  const by = (offset: number) => (range: TextRange): TextRange => ({ start: range.start + offset, end: range.end + offset });
  const tuned = (copied.tuned ?? []).map((range) => ({ ...range, ...by(at)(range) }));
  return {
    full: own.full.slice(0, at) + copied.full + own.full.slice(after),
    marks: [
      ...own.marks.filter((range) => range.end <= at),
      ...copied.marks.map(by(at)),
      ...own.marks.filter((range) => range.start >= after).map(by(copied.full.length - sentence.length)),
    ],
    ...(tuned.length === 0 ? {} : { tuned }),
  };
}

/**
 * A fused definition (R77, R102) carries no `params`, though its text writes its ingredients'
 * `{key}`s; the view gives their values (`CardView.params`), so in play each value fills its key and
 * counts as printed (no number of a fused card is marked as moved). Any other definition is as given.
 */
function withViewParams(def: CardDef | undefined, values: Readonly<Record<string, number>> | undefined): CardDef | undefined {
  if (def === undefined || def.params !== undefined || values === undefined) return def;
  const params: Param[] = Object.entries(values).map(([key, value]) => ({ key, base: value, radiant: value, better: "up" }));
  return params.length === 0 ? def : { ...def, params };
}

/** E36: "27 lines of code", "1 line of code". */
export function locWords(loc: number): string {
  return `${String(loc)} ${loc === 1 ? "line" : "lines"} of code`;
}

/** R243: a hand card's stats are its face plus what it gained in hand, with nothing on the field's layers. */
function handLive(stats: InPlay["handStats"], printed: PrintedFace | undefined): FaceSource["live"] {
  if (stats === undefined) return undefined;
  return { attack: stats.attack, health: stats.health, maxHealth: stats.health, keywords: printed?.keywords ?? [] };
}

/**
 * The keywords a card has now that its printed face does not print, each once, in the order it has
 * them. Lucky X stacks, and the view keeps each entry for the sum (layers.rs), so Lucky is gained when
 * its sum is not the printed sum, as one entry at the sum where its first entry stands: a Lucky 1 card
 * given Lucky 1 has gained Lucky 2 (R1438).
 */
function gainedKeywords(live: readonly Keyword[], printed: readonly Keyword[]): Keyword[] {
  const printedKeys = new Set(printed.map(keywordKey));
  const luck = luckOf(live);
  let luckGained = luck !== luckOf(printed);
  const seen = new Set<string>();
  const gained: Keyword[] = [];
  for (const keyword of live) {
    if (keyword.kind === "Lucky") {
      if (luckGained) gained.push({ kind: "Lucky", n: luck });
      luckGained = false;
      continue;
    }
    const key = keywordKey(keyword);
    if (printedKeys.has(key) || seen.has(key)) continue;
    seen.add(key);
    gained.push(keyword);
  }
  return gained;
}

/** Lucky X's sum over a list of keywords (§6.1: Lucky stacks). */
function luckOf(keywords: readonly Keyword[]): number {
  return keywords.reduce((sum, keyword) => (keyword.kind === "Lucky" ? sum + keyword.n : sum), 0);
}

function sameText(a: FaceText, b: FaceText): boolean {
  return a.full === b.full;
}

/** R277: which stats a printed Radiant face raised over its base face's; none in play or on a base face. */
function grewOf(def: CardDef | undefined, source: FaceSource): FaceStats["grew"] {
  if (def === undefined || !source.radiant || source.live !== undefined || source.inPlay?.handStats !== undefined) {
    return undefined;
  }
  return {
    attack: (def.radiant.attack ?? 0) > (def.base.attack ?? 0),
    health: (def.radiant.health ?? 0) > (def.base.health ?? 0),
  };
}

/** inPlay.ts: a card with the Call to Chaos tag keeps its text a mystery in play. */
function concealed(def: CardDef | undefined): boolean {
  return def !== undefined && concealedInPlay(def.tags);
}

/**
 * What the rules box prints in play (inPlay.ts). A Vanilla unit's text is gone; a Call to Chaos is
 * "???"; a #98 Heroic Power is the power it rolled. Anything else prints its printed text.
 */
function textInPlay(def: CardDef | undefined, radiant: boolean, printedText: FaceText, inPlay: InPlay): FaceText {
  if (inPlay.vanilla === true) return { full: VANILLA_TEXT, marks: [] };
  if (def === undefined) return printedText;
  if (concealed(def)) return { full: concealedText(def.id, radiant), marks: [] };
  if (inPlay.power !== undefined && def.id === HEROIC_POWER_ID) {
    const face = radiant ? def.radiant : def.base;
    // B3.4: Steady Shot's {shot} is the card's number as it stands (the view's), else as printed.
    const values = inPlay.params ?? printedValues(def, radiant);
    const words = powerText(inPlay.power, radiant, face.keywords.map(keywordKey).join(", "), values);
    if (words !== null) {
      // R277: a Radiant power is marked against the same power's base words.
      const baseWords = radiant
        ? powerText(inPlay.power, false, def.base.keywords.map(keywordKey).join(", "), printedValues(def, false))
        : null;
      return { full: words, marks: baseWords === null ? [] : radiantMarks(baseWords, words) };
    }
  }
  return printedText;
}

/** A definition's declared numbers as printed on a face (B3.4 rule 5), by key. */
function printedValues(def: CardDef, radiant: boolean): Record<string, number> {
  return Object.fromEntries((def.params ?? []).map((param) => [param.key, radiant ? param.radiant : param.base]));
}

/**
 * The rarity a face's frame is drawn in (R503): a token's printed rarity when it has one, else the
 * card's own. Display only; nothing reads it as the card's rarity.
 */
export function frameRarity(face: Pick<FaceModel, "rarity" | "printedRarity">): Rarity | null {
  return face.printedRarity ?? face.rarity;
}

/**
 * `data-foil` on `.cf`: Mythic cards and radiant faces carry foil, animated while the player's
 * `animatedFoil` setting is on and still while it is off. The CSS never animates it under
 * `prefers-reduced-motion: reduce` whatever this says.
 */
export function foilFor(face: FaceModel, animatedFoil: boolean): "animated" | "static" | "none" {
  if (face.rarity !== "Mythic" && !face.radiant) return "none";
  return animatedFoil ? "animated" : "static";
}

function costTone(live: number | undefined, printed: number): FaceCost["tone"] {
  if (live === undefined) return "base";
  if (live < printed) return "down";
  if (live > printed) return "up";
  return "base";
}

function costOf(def: CardDef | undefined, liveCost: number | undefined): FaceCost {
  const live = liveCost === undefined ? undefined : String(liveCost);

  if (def === undefined) {
    const text = live ?? UNKNOWN_COST;
    return { text, value: text, tone: "base", alt: null };
  }

  const printed = def.cost;
  if (printed === "X") {
    // An X card costs exactly X (R65): the gem says X, and `data-cost` carries whatever the view says.
    return { text: "X", value: live ?? "X", tone: "base", alt: null };
  }
  if (typeof printed === "number") {
    const text = live ?? String(printed);
    return { text, value: text, tone: costTone(liveCost, printed), alt: null };
  }
  // "A embiggen B". With no live number, or the live number at A, the gem shows A with the bigger
  // price B beside it. Otherwise it shows the view's number and drops B, which it can no longer
  // vouch for (the client never works out a discounted embiggen price, CLAUDE.md rule 7). A card
  // whose live cost is B was paid B, on the field: that is its printed price, so the tone is base.
  const base = String(printed.base);
  if (liveCost === undefined || liveCost === printed.base) {
    return { text: base, value: live ?? base, tone: "base", alt: String(printed.embiggen) };
  }
  const text = String(liveCost);
  const tone = liveCost === printed.embiggen ? "base" : costTone(liveCost, printed.base);
  return { text, value: text, tone, alt: null };
}

function statTone(value: number, printed: number | undefined): StatTone {
  if (printed === undefined) return "base";
  if (value > printed) return "buffed";
  if (value < printed) return "reduced";
  return "base";
}

function statsOf(
  type: CardType,
  known: boolean,
  printed: PrintedFace | undefined,
  live: FaceSource["live"],
  grew: FaceStats["grew"],
): FaceStats | null {
  if (live !== undefined) {
    // A unit nobody can name has no printed face to compare against, so nothing is coloured.
    if (!known) {
      return { attack: live.attack, health: live.health, maxHealth: live.maxHealth, attackTone: "base", healthTone: "base" };
    }
    return {
      attack: live.attack,
      health: live.health,
      maxHealth: live.maxHealth,
      attackTone: statTone(live.attack, printed?.attack),
      healthTone: live.health < live.maxHealth ? "damaged" : statTone(live.maxHealth, printed?.health),
    };
  }

  // B3.1 rule 1: an Animated Field Spell or Trap prints the stats of the Unit it becomes; no other
  // card that is not a Unit prints any.
  const attack = printed?.attack;
  const health = printed?.health;
  if (attack === undefined || health === undefined) return null;
  if (type !== "Unit" && !printsUnitStats(printed)) return null;
  return { attack, health, maxHealth: health, attackTone: "base", healthTone: "base", ...(grew === undefined ? {} : { grew }) };
}

/** B3.1: an Animated face (either kind) prints a Unit's attack and health. */
function printsUnitStats(printed: PrintedFace | undefined): boolean {
  return (printed?.keywords ?? []).some((keyword) => keyword.kind === "Animated" || keyword.kind === "Animated on your turn");
}

/**
 * A base face prints its catalog text. A Radiant face prints its catalog text whole, the Radiant
 * cell written out (R277), with what the base face's text does not have marked; a fused definition's
 * lines are marked line by line against the base lines of the same ingredients (radiantDiff.ts).
 */
/**
 * A face's text with its `{key}` numbers filled in (`fillParams`, B3.4 rule 5): the face's own printed
 * values, or `values` for a card in play whose numbers have moved. R277's diff compares the Radiant
 * face so filled with the base face as printed.
 */
function textOf(
  def: CardDef | undefined,
  radiant: boolean,
  values?: Readonly<Record<string, number>>,
): FaceText {
  if (def === undefined) return { full: "", marks: [] };
  // B3.4, R386: in play, the numbers that moved off their printed values are marked where they stand.
  const { text: full, tuned } = filledText(def, radiant ? "radiant" : "base", values);
  const moved = tuned.length === 0 ? {} : { tuned };
  if (!radiant) return { full, marks: [], ...moved };
  return { full, marks: radiantMarks(fillParams(def, "base"), full), ...moved };
}
