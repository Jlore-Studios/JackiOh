// What a card face shows, computed once from a catalog def and the live view (docs/polish/6-cards.md,
// Surface B). Pure: no React, no DOM, no settings, except the reader's language (R711000). Every
// component that draws a card — the board's Card.tsx, the inspect overlays, the deck builder —
// builds a FaceModel here and hands it down.
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
// stats — so a buffed unit reads green and a damaged one red, the way Hearthstone colors them.
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
// An Animated Field Spell or Trap that prints attack and health prints those of the Unit it becomes
// (B3.1 rule 1; R2280000: it may print none), so its face carries them as a Unit's does, wherever it is.
//
// A face in play carries a quest line as the view gives it (`quest`, B5 E33, R404: In Too Deep's open
// quests, their progress and rewards, its auras), which cardState.ts draws as badges. A copier (Classic
// #57 Echo, B5 E14, R399, R511) prints the Spell text the view says it has (`InPlay.copies`), filled
// with the numbers it reads on the card, in place of its own copying sentence; it keeps its own name,
// cost, type and art, and `copying` names the card it copies for the inspect notes.
//
// ME-CN, R1301, R711000: a card the view says is Chinese (`FaceSource.chinese`), or any card when the
// reader reads Chinese (`drawnInChinese`), prints its name and both faces' texts from the Chinese
// table (chinese.ts), a copier's copied text and a fused card's
// ingredients' included, filled with the same numbers; its preview labels are the table's too. A
// Radiant face's gold diff compares the two Chinese faces (R1302). The English face's name and text
// ride along (`englishName`, `englishText`), never printed: the art's motif and the glossary's terms
// are read from them.
//
// R2521009: a Resource (its definition carries `resource`, R1840000) prints what it sells for on the
// face shown, `credits` (the face's `sellMin` and `sellMax`), as one line under its text,
// "Credits: 2–5" (`creditsLine`). Read off the definition alone, in play or out; no other card has it.

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
import { TRIBAL_TAGS } from "@jackioh/engine/config";

import {
  HEROIC_POWER_ID,
  VANILLA_TEXT,
  concealedInPlay,
  concealedText,
  powerText,
  type RolledPower,
} from "./inPlay.ts";
import { isZhCn, t } from "../i18n/i18n.ts";
import { CHINESE_COMMA, chineseDef, chineseKeyword, chinesePreviewLabel, drawnInChinese } from "./chinese.ts";
import { GLITCH_WORDS, isGlitch } from "./glitch.ts";
import { radiantMarks, type TextRange } from "./radiantDiff.ts";
import { temporaryLeft } from "./keywordVisuals.ts";
import { faceTuning, filledText, type FaceTuning, type TunedRange } from "./tuning.ts";
import type { NamedTally } from "./tallies.ts";

export type StatTone = "base" | "buffed" | "reduced" | "damaged";
export type FaceCost = {
  /** What the gem shows (costOf). */
  text: string;
  /** `data-cost`: the live number when there is one (CardView.cost), else `text`. */
  value: string;
  tone: "base" | "down" | "up";
  /** The embiggen price, shown small beside the gem while the gem shows the base price; null otherwise. */
  alt: string | null;
};
export type FaceStats = {
  attack: number;
  health: number;
  maxHealth: number;
  attackTone: StatTone;
  healthTone: StatTone;
  /** R277: on a printed Radiant face, which stats it raised over the base face's, marked as its text is. Absent elsewhere. */
  grew?: { attack: boolean; health: boolean };
};
/**
 * What the rules box prints: the whole text, and on a Radiant face the stretches of it the base
 * face's text does not have (R277); `marks` is empty wherever play prints something else.
 */
export type FaceText = {
  full: string;
  marks: readonly TextRange[];
  /** B3.4, R386: in play, the numbers in `full` the view moved off their printed values, each marked better or worse (tuning.ts). */
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
  /** B2.5, R503: the rarity a token prints (`CardDef.printedRarity`), which its frame shows in place of Token's. Display only. */
  printedRarity?: PrintedRarity | null;
  /**
   * R1177001: the card is a token (`CardDef.token`), which its full face says on its type line
   * ("Token Unit"); no face wears a mark of it (R2647000). On every catalog card the flag agrees
   * with rarity Token, the Token tag and §7's row, and on a fused card with its tag (R1177000).
   * Absent (false) on a face built by hand and on a card with no definition.
   */
  token?: boolean;
  index: string | null;
  set: SetName | null;
  radiant: boolean;
  cost: FaceCost;
  /** Units only: live when `live` was given, else the printed face; null for non-units and unknown stats. */
  stats: FaceStats | null;
  /** On a fused definition, its ingredients' texts line by line (R102); in play, what the card in play says (inPlay.ts). */
  text: FaceText;
  /** R279: the cards and tokens this card's text names (`CardDef.refs`), linked where their names stand in `text.full`. */
  refs: readonly string[];
  /** R280: in play, what the card's formula comes to now (`CardView.preview`), each printed in braces after its label. */
  values: readonly PreviewValue[];
  /** Live keywords when `live` was given, a hand card's as the view gives them (`InPlay.handKeywords`), else the printed face's. */
  keywords: readonly Keyword[];
  inPlay: boolean;
  /** R243, §6.3 Vanilla: the unit's text is gone, and the rules box says so. */
  vanilla: boolean;
  /** Keywords the card has now that its printed face does not print (gainedKeywords), printed after its text; none outside play. */
  gained: readonly Keyword[];
  /** ME-GRANT, R1107: the Death abilities other cards granted the unit, each on its own line after the gained keywords. Empty outside play. */
  grants: readonly string[];
  /**
   * The collection's text for this face when the face in play prints something else, so the inspect
   * overlays can show both. Null when the two agree, outside play, and for a "???" card.
   */
  printed: FaceText | null;
  /** B3.3, R385: in play, the card's Brittle count (`CardView.brittle`). Optional, as every field below is, for a face built by hand. */
  brittle?: number | null;
  /** #2437, R637, R2179001: in a hand, the count the card's Temporary X has left as its keywords give it (`temporaryLeft`); null elsewhere and on a card with none. */
  temporary?: number | null;
  /** R980: the card's element, while a Feng Shui acts (`CardView.element`); null otherwise. */
  element?: string | null;
  /** B3.4, R386: in play, what Degrade, Upgrade and KY's Constant changed on the card; null when nothing did. */
  tuning?: FaceTuning | null;
  /** B5 E39: in play, the enchantments riding the card (`CardView.enchantments`). */
  enchantments?: readonly Enchantment[];
  /** B3.1, R383: a backrow card standing in a unit zone as a Unit (`UnitView.animated`); null otherwise. */
  animated?: { home?: number } | null;
  /** B5 E35: in play, the unit has gone Berserk (`UnitView.berserk`). */
  berserk?: boolean;
  /** R1174000: in play, the unit is Frozen (`UnitView.frozen`). */
  frozen?: boolean;
  /**
   * R1839000, R1839001: in play, the card is Dormant (`UnitView.slumber`): `turns` is how many of its
   * controller's turns must start before it awakens, null when an effect must awaken it; null when awake.
   */
  dormant?: { turns: number | null } | null;
  /** R1848000: in play, the unit is Frozen forever, so it never thaws (`UnitView.frozenForever`). */
  frozenForever?: boolean;
  /**
   * R1852000, R1852005: in play, the card is a Starship (`UnitView.starship`): how many pieces are fused
   * into it, whether it has launched, and (R2249000) the pieces, oldest first, printed in place of its text.
   */
  starship?: { pieces: number; launched: boolean; list: readonly StarshipPieceRef[] } | null;
  /** MD-B6, R943: in play, the card was minted after the decks were built (`CardView.created`). */
  created?: boolean;
  /** R1956001: in play, the catalog id of the card that generated the viewer's own Created card, whose foil, finish and border it wears (cosmetics.ts). */
  createdBy?: string;
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
  /** R711000: the view flagged the card Chinese (R1301), as opposed to a Chinese reader's own language. */
  flagged?: boolean;
  /** R1301: on a Chinese face, the name the English face prints, which picks the art's motif (R503). */
  englishName?: string;
  /** R1301: on a Chinese face, the text the English face prints, whose terms the glossary explains. */
  englishText?: string;
  /** R849000: in play, why the viewer's own unit cannot attack now (`UnitView.attackReason`), which the inspect notes spell out. */
  attackReason?: string;
  /** R2321000: in play, the counts cards on the field keep on the unit (`UnitView.tallies`), which the inspect notes spell out. */
  tallies?: readonly NamedTally[];
  /** R979000: in play, why a card in the viewer's own hand (or, R1571000, graveyard) cannot be played now (`CardView.playReason`), which the inspect notes spell out. */
  playReason?: string;
  /**
   * R2521009: a Resource's sale prices on the face shown (`CardDef.resource`'s `base` or `radiant`
   * `sellMin` and `sellMax`), which the face prints under its text; null (or absent, on a face built
   * by hand) on every card that is no Resource.
   */
  credits?: FaceCredits | null;
};
/** R2521009: what a Resource sells for on one face, its least and its most. */
export type FaceCredits = { min: number; max: number };
/**
 * What a game adds to a face (R243, SPEC §10.10); its presence is what makes a face one in play.
 * `live` (a unit on the field) and `liveCost` stay where they were, beside it.
 */
/** R1852005, R2249000: one Starship Piece fused into a Starship, on the face it fused on. */
export type StarshipPieceRef = { defId: string; radiant: boolean };

/** What a game adds to a face (R243); its presence is what makes a face one in play. */
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
  /** R2247000: the word the card shows in place of its text, Anomaly Found!'s board code (`CardView.label`). */
  label?: string;
  /** R280: what the card's formula comes to now (`CardView.preview`). */
  preview?: readonly PreviewValue[];
  /** B3.4, R386: the card's declared numbers as they stand now (`CardView.params`), which fill its text's `{key}`s. */
  params?: Readonly<Record<string, number>>;
  /** B3.3, R385: the card's Brittle count (`CardView.brittle`). */
  brittle?: number;
  /** #2437: the card lies in a hand (game/Card.tsx's `inHandOf`), the one zone its Temporary counts in (R637). */
  inHand?: true;
  /** R980: the card's element, while a Feng Shui acts (`CardView.element`). */
  element?: string;
  /** B3.4, R386: what Degrade and Upgrade changed on the card (`CardView.tuning`). */
  tuning?: Tuning;
  /** B5 E39: the enchantments riding the card (`CardView.enchantments`). */
  enchantments?: readonly Enchantment[];
  /** B3.1, R383: the card stands in a unit zone as a Unit (`UnitView.animated`). */
  animated?: { home?: number };
  /** B5 E35: the unit has gone Berserk (`UnitView.berserk`). */
  berserk?: true;
  /** R1174000: the unit is Frozen (`UnitView.frozen`). */
  frozen?: true;
  /** R1839000, R1839001: the card is Dormant (`UnitView.slumber`), `turns` its countdown or null for none. */
  dormant?: { turns: number | null };
  /** R1848000: the unit is Frozen forever (`UnitView.frozenForever`). */
  frozenForever?: true;
  /**
   * R1852000, R1852004: the card is a Starship (`UnitView.starship`): its piece count, the name it took
   * as it launched (null while Dormant), printed in place of its own, and its pieces, oldest first.
   */
  starship?: { pieces: number; name: string | null; list?: readonly StarshipPieceRef[] };
  /** MD-B6, R943: the card was minted after the decks were built (`CardView.created`). */
  created?: true;
  /** R1956001: the card that generated the viewer's own Created card (`CardView.createdBy`). */
  createdBy?: string;
  /** R437: the marks on the card (`CardView.marks`). */
  marks?: readonly CardMark[];
  /** B5 E33, R404: the card's quest line (`CardView.quest`). */
  quest?: QuestView;
  /** B5 E14, R399: the Spell a copier has the text of (`CardView.copies`), with its definition. */
  copies?: { def: CardDef; radiant: boolean; params?: Readonly<Record<string, number>> };
  /** ME-GRANT, R1107: the Death abilities other cards granted the unit (`UnitView.grants`). */
  grants?: readonly string[];
  /** R849000: why the viewer's own unit cannot attack now (`UnitView.attackReason`). */
  attackReason?: string;
  /** R2321000: the counts cards on the field keep on the unit (`UnitView.tallies`), each by its keeper's name. */
  tallies?: readonly NamedTally[];
  /** R979000: why a card in the viewer's own hand (or, R1571000, graveyard) cannot be played now (`CardView.playReason`). */
  playReason?: string;
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
  /** R1181003: CardView.embiggenCost, what a play of the Embiggened version costs now. */
  liveEmbiggenCost?: number;
  /** R1848003: CardView.fixedX, the X an X-cost card is set at, which the gem prints in place of X. */
  fixedX?: number;
  /** UnitView's current numbers. */
  live?: { attack: number; health: number; maxHealth: number; keywords: readonly Keyword[] };
  inPlay?: InPlay;
  /** ME-CN, R1301: the view says the card is Chinese (`CardView.chinese`), whatever the reader's language. */
  chinese?: boolean;
  /** MD-B15, R923: the view's tags where they differ from the definition's (a granted tag). */
  tags?: readonly Tag[];
};

/** The gem of a card nobody can name: no catalog, no live cost. */
const UNKNOWN_COST = "?";

/** A card with no def is drawn as a Unit, exactly as `unknownCard` in game/catalog.ts reports it. */
const UNKNOWN_TYPE: CardType = "Unit";

export function faceModel(source: FaceSource): FaceModel {
  return faceIn(source, drawnInChinese(source.chinese === true));
}

function faceIn(source: FaceSource, chinese: boolean): FaceModel {
  const filled = withViewParams(source.def, source.inPlay?.params);
  // R1301: a Chinese card's name and texts are the table's; all else stays the definition's.
  const def = chinese && filled !== undefined ? chineseDef(filled) : filled;
  const printed = def === undefined ? undefined : source.radiant ? def.radiant : def.base;
  // B2.7: a face may carry its own type (Classic+ #22 Blood Moon's Radiant face is a Field Trap), and
  // the card's type is its face's (§5.2); in play the view's word for it comes first.
  const viewType = source.inPlay === undefined ? undefined : source.type;
  const type: CardType = viewType ?? printed?.type ?? def?.type ?? source.type ?? UNKNOWN_TYPE;
  const inPlay = source.inPlay;
  const vanilla = inPlay?.vanilla === true;
  const printedText = textOf(def, source.radiant);
  const ownText = inPlay?.params === undefined ? printedText : textOf(def, source.radiant, inPlay.params);
  // B5 E14, R511: a copier's own words are the copied Spell's text (in Chinese on a Chinese copier, R1301).
  const viewCopies = inPlay?.copies;
  const copies = viewCopies === undefined || !chinese ? viewCopies : { ...viewCopies, def: chineseDef(viewCopies.def) };
  const liveText = copies === undefined ? ownText : copiedText(ownText, def, textOf(copies.def, copies.radiant, copies.params));
  const text = inPlay === undefined ? printedText : textInPlay(def, source.radiant, liveText, inPlay, chinese);
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
  const english = chinese ? faceIn(source, false) : null;

  return {
    defId: source.defId,
    known: def !== undefined,
    name: inPlay?.starship?.name ?? def?.name ?? source.name ?? source.defId,
    type,
    // MD-B15, R923: in play a face lists the view's tags; outside play the definition's.
    tags: source.inPlay !== undefined && source.tags !== undefined ? source.tags : (def?.tags ?? []),
    rarity: def?.rarity ?? null,
    printedRarity: def?.printedRarity ?? null,
    token: def?.token === true,
    index: def?.index ?? null,
    set: def?.set ?? null,
    radiant: source.radiant,
    // Glitch's gem shows glyphs, never a number (glitch.ts); `data-cost` still carries the view's.
    cost: isGlitch(source.defId)
      ? { ...costOf(def, source.liveCost, source.liveEmbiggenCost, source.fixedX), text: GLITCH_WORDS.cost, tone: "base", alt: null }
      : costOf(def, source.liveCost, source.liveEmbiggenCost, source.fixedX),
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
    grants: inPlay?.grants ?? [],
    printed: inPlay === undefined || sameText(text, printedText) || concealed(def) ? null : printedText,
    brittle: inPlay?.brittle ?? null,
    temporary: inPlay?.inHand === true ? temporaryLeft(keywords) : null,
    element: inPlay?.element ?? null,
    tuning,
    enchantments: inPlay?.enchantments ?? [],
    animated: inPlay?.animated ?? null,
    berserk: inPlay?.berserk === true,
    frozen: inPlay?.frozen === true,
    dormant: inPlay?.dormant ?? null,
    frozenForever: inPlay?.frozenForever === true,
    starship:
      inPlay?.starship === undefined
        ? null
        : { pieces: inPlay.starship.pieces, launched: inPlay.starship.name !== null, list: inPlay.starship.list ?? [] },
    created: inPlay?.created === true,
    loc: def?.loc ?? null,
    credits: creditsOf(source.def, source.radiant),
    marks: inPlay?.marks ?? [],
    quest: inPlay?.quest ?? null,
    copying: copies === undefined ? null : { defId: copies.def.id, name: copies.def.name, radiant: copies.radiant },
    ...(inPlay?.createdBy === undefined ? {} : { createdBy: inPlay.createdBy }),
    ...(inPlay?.attackReason === undefined ? {} : { attackReason: inPlay.attackReason }),
    ...(inPlay?.tallies === undefined ? {} : { tallies: inPlay.tallies }),
    ...(inPlay?.playReason === undefined ? {} : { playReason: inPlay.playReason }),
    ...(english === null ? {} : { chinese: true, englishName: english.name, englishText: english.text.full }),
    ...(source.chinese === true ? { flagged: true } : {}),
  };
}

/**
 * B5 E14, R511: a copier's text in play (Classic #57 Echo): the copied text where its own face prints its
 * copying sentence (its base text), so whatever else its face prints stays (a Radiant Echo still reads
 * "Echo 1"); a face that does not print that sentence prints the copied text alone.
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
 * B3.4: a fused definition (R77, R102) carries no `params`, though its text writes its ingredients'
 * `{key}`s: in play the view's values fill them and count as printed, so none is marked as moved.
 */
function withViewParams(def: CardDef | undefined, values: Readonly<Record<string, number>> | undefined): CardDef | undefined {
  if (def === undefined || def.params !== undefined || values === undefined) return def;
  const params: Param[] = Object.entries(values).map(([key, value]) => ({ key, base: value, radiant: value, better: "up" }));
  return params.length === 0 ? def : { ...def, params };
}

/**
 * R2521009: a Resource's sale prices on the face shown, as its definition prints them
 * (`CardDef.resource`, R1840000); null for a card that is no Resource. Nothing is worked out: the
 * numbers are the definition's (CLAUDE.md rule 7).
 */
function creditsOf(def: CardDef | undefined, radiant: boolean): FaceCredits | null {
  const resource = def?.resource;
  if (resource === undefined) return null;
  const price = radiant ? resource.radiant : resource.base;
  return { min: price.sellMin, max: price.sellMax };
}

/** R2521009: the line a Resource's face prints under its text, "Credits: 2–5"; "" on any other face. */
export function creditsLine(face: Pick<FaceModel, "credits">): string {
  const credits = face.credits;
  if (credits === undefined || credits === null) return "";
  return t("Credits: {min}–{max}", { min: credits.min, max: credits.max });
}

/** E36: "27 lines of code", "1 line of code". */
export function locWords(loc: number): string {
  return loc === 1 ? t("{n} line of code", { n: loc }) : t("{n} lines of code", { n: loc });
}

/** R243: a hand card's stats are its face plus what it gained in hand, with nothing on the field's layers. */
function handLive(stats: InPlay["handStats"], printed: PrintedFace | undefined): FaceSource["live"] {
  if (stats === undefined) return undefined;
  return { attack: stats.attack, health: stats.health, maxHealth: stats.health, keywords: printed?.keywords ?? [] };
}

/**
 * The keywords a card has now that its printed face does not print, each once, in order. Lucky X stacks
 * and the view keeps each entry (layers.rs), so Lucky is gained when its sum is not the printed sum, as
 * one entry at the sum where its first entry stands (R1438).
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

/** What the rules box prints in play (inPlay.ts): a #98 Heroic Power prints the power it rolled. */
function textInPlay(def: CardDef | undefined, radiant: boolean, printedText: FaceText, inPlay: InPlay, chinese: boolean): FaceText {
  // R711000: a face drawn in Chinese for a reader of Chinese says it in the reader's words; the English
  // face (`englishText`, what the glossary's terms are found in) keeps the English.
  const reader = chinese && isZhCn();
  if (inPlay.vanilla === true) return { full: reader ? t(VANILLA_TEXT) : VANILLA_TEXT, marks: [] };
  if (def === undefined) return printedText;
  if (concealed(def)) return { full: concealedText(def.id, radiant), marks: [] };
  // R2247000: a card that holds a label (Anomaly Found!'s "FO") prints it as its text.
  if (inPlay.label !== undefined) return { full: inPlay.label, marks: [] };
  // R2249000: a Starship prints no text; its face lists the pieces fused into it instead (CardFace.tsx).
  if (inPlay.starship !== undefined) return { full: "", marks: [] };
  if (inPlay.power !== undefined && def.id === HEROIC_POWER_ID) {
    const face = radiant ? def.radiant : def.base;
    // B3.4: Steady Shot's {shot} is the card's number as it stands (the view's), else as printed.
    const values = inPlay.params ?? printedValues(def, radiant);
    const kinds = (list: readonly Keyword[]): string =>
      reader ? list.map(chineseKeyword).join(CHINESE_COMMA) : list.map(keywordKey).join(", ");
    const words = powerText(inPlay.power, radiant, kinds(face.keywords), values, reader);
    if (words !== null) {
      // R277: a Radiant power is marked against the same power's base words.
      const baseWords = radiant
        ? powerText(inPlay.power, false, kinds(def.base.keywords), printedValues(def, false), reader)
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

/** R503: the rarity a face's frame is drawn in. Display only; nothing reads it as the card's rarity. */
export function frameRarity(face: Pick<FaceModel, "rarity" | "printedRarity">): Rarity | null {
  return face.printedRarity ?? face.rarity;
}

/** R1382, R1162 (Meditative #87 Tatches the Totem): what the frame prints for a card with every tribal tag (R1424), in place of the five. */
export const ALL_TRIBES = "All Tribes";

/**
 * The tags the frame prints, `ALL_TRIBES` in place of every tribal tag. The Token tag is never a chip
 * (R1177001): a token's type line says it ("Token Unit"), where no crowded tag row can cut it.
 */
export function frameTags(tags: readonly Tag[]): readonly (Tag | typeof ALL_TRIBES)[] {
  const printed: readonly Tag[] = tags.filter((tag) => tag !== "Token");
  if (!TRIBAL_TAGS.every((tribe) => printed.includes(tribe))) return printed;
  return [ALL_TRIBES, ...printed.filter((tag) => !TRIBAL_TAGS.includes(tag))];
}

/** `data-foil` on `.cf`. The CSS never animates it under `prefers-reduced-motion: reduce` whatever this says. */
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

function costOf(def: CardDef | undefined, liveCost: number | undefined, liveEmbiggenCost: number | undefined, fixedX?: number): FaceCost {
  const live = liveCost === undefined ? undefined : String(liveCost);

  if (def === undefined) {
    const text = live ?? UNKNOWN_COST;
    return { text, value: text, tone: "base", alt: null };
  }

  const printed = def.cost;
  if (printed === "X") {
    // R1848003: an X card whose X is set costs that X, and the gem prints it.
    if (fixedX !== undefined) return { text: String(fixedX), value: live ?? String(fixedX), tone: "base", alt: null };
    // An X card costs exactly X (R65): the gem says X, and `data-cost` carries whatever the view says.
    return { text: "X", value: live ?? "X", tone: "base", alt: null };
  }
  if (typeof printed === "number") {
    const text = live ?? String(printed);
    return { text, value: text, tone: costTone(liveCost, printed), alt: null };
  }
  // "A embiggen B": with no live number, or the live number at A, the gem shows A with B (the view's
  // `embiggenCost` where it has one, R1181003) beside it. Otherwise it shows the view's number and drops
  // B: the client never works out a discounted embiggen price (CLAUDE.md rule 7). A card whose live
  // cost is B was paid B, its printed price, so the tone is base.
  const base = String(printed.base);
  if (liveCost === undefined || liveCost === printed.base) {
    return { text: base, value: live ?? base, tone: "base", alt: String(liveEmbiggenCost ?? printed.embiggen) };
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
    // A unit nobody can name has no printed face to compare against, so nothing is colored.
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

  // B3.1 rule 1: an Animated Field Spell or Trap that prints stats prints those of the Unit it becomes
  // (R2280000: it may print none); no other card that is not a Unit prints any.
  const attack = printed?.attack;
  const health = printed?.health;
  if (attack === undefined || health === undefined) return null;
  if (type !== "Unit" && !printsUnitStats(printed)) return null;
  return { attack, health, maxHealth: health, attackTone: "base", healthTone: "base", ...(grew === undefined ? {} : { grew }) };
}

/** B3.1: an Animated face (either kind) may print a Unit's attack and health (R2280000). */
function printsUnitStats(printed: PrintedFace | undefined): boolean {
  return (printed?.keywords ?? []).some((keyword) => keyword.kind === "Animated" || keyword.kind === "Animated on your turn");
}

/**
 * A face's text with its `{key}` numbers filled in (`fillParams`, B3.4 rule 5): its printed values, or
 * `values` in play. A Radiant face marks what the base face's text, as printed, does not have (R277); a
 * fused definition's lines are marked against the same ingredients' base lines (radiantDiff.ts).
 */
function textOf(
  def: CardDef | undefined,
  radiant: boolean,
  values?: Readonly<Record<string, number>>,
): FaceText {
  if (def === undefined) return { full: "", marks: [] };
  const { text: full, tuned } = filledText(def, radiant ? "radiant" : "base", values);
  const moved = tuned.length === 0 ? {} : { tuned };
  if (!radiant) return { full, marks: [], ...moved };
  return { full, marks: radiantMarks(fillParams(def, "base"), full), ...moved };
}
