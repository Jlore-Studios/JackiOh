// The tall card (docs/polish/6-cards.md, Surface B "CardFace DOM"): cost gem, art window, name ribbon, rarity gem,
// type line, rules box, tag badges, and sword and drop for units. `layout="compact"` is a face-up backrow card:
// cost, art, name, rarity gem and type line only.
//
// Every element is a span, strong or img, so a face can sit inside the deck builder's <button>. Nothing here
// carries `data-testid`, `data-attack`, `data-health`, `data-keyword` or the `card` class token: those belong to
// the board's root element (game/Card.tsx) and the minion form, and a face in a hover preview must never answer
// an e2e selector meant for the board (B12, B20).
//
// A face in play (FaceModel.inPlay, SPEC §10.10) prints the card as it stands: its rules box ends with the
// keywords it has gained (`.cf-text-gained`), and a Vanilla unit's box says its text is gone. A fused card's
// text is a line per ingredient (R102). The box prints the face's whole text (`.cf-text-base`), drawn by
// RulesText: a Radiant face its Radiant text with what the base face lacks in gold (R277), `refs` names as
// references (R279), and in play the values its formula comes to (R280).
//
// SPEC §10.8: a face in play wears the view's states as badges (CardStates.tsx): Brittle (R385), tuned (R386),
// enchantments (E39), a card standing as a Unit (R383). An Animated Field Spell or Trap shows the Unit's attack
// and health (B3.1). R503: a token that prints a rarity (B2.5's `printedRarity`) wears that rarity's frame, gem,
// crest and foil rather than Token's, for display only; the art gets the card's name, which picks its motif.
//
// ME-CN, R1301: a Chinese face (`face.chinese`) prints its name and text as the model gives them and its type
// line, tags and keywords in chinese.ts's words; its art keeps the English name's motif. R1382: a card with every
// tribal tag (R1424, Meditative #87) prints "All Tribes" on its tags line.
//
// R1177001: a token's type line reads "Token Unit" and its tag row prints no Token chip. R2647000: where the type
// line hides (a face under FACE_TEXT_MIN_HEIGHT_PX, a full-art face) and on the board minion nothing marks it.
// Glitch stays blank (R674).
//
// R1017001: a full-art card (fullArt.ts; R2655001: one the cosmetics registry has a full art of, R2655002 a token
// listed under one) prints its art across the whole face and no words (`data-full-art`), its whole text in the
// hover preview and the detail view. The Full-art cards setting turns it off, and so does a tutorial lesson's
// board (R1108000). R2655000: the face wears its pick's tier (`data-full-art-tier`), a Legendary's art animated,
// a Mythic's animated and signed (FullArtSignature.tsx). R2655003: the motion runs only while Animated foil is on
// and inside cosmetics.css's motion gate.
//
// R1295000: the viewer's own card in a game (`worn`) wears the device's card cosmetics, each on its own attribute
// apart from the rarity foil (cosmetics.ts). R2521009: a Resource's full face prints what it sells for ("Credits:
// 2–5") under its text.

import { useRef, type CSSProperties, type ReactElement } from "react";

import { keywordKey, type CardType, type Keyword, type KeywordKind, type Rarity, type Tag } from "@jackioh/shared";

import { CardArt, type ArtShape } from "./art/index.ts";
import { CardStates } from "./CardStates.tsx";
import { t, useLocale } from "../i18n/i18n.ts";
import { CHINESE_COMMA, CHINESE_TERMS, chineseKeyword } from "./chinese.ts";
import { useCosmeticAttributes } from "./cosmetics.ts";
import { FIT_FLOOR_PX, PREVIEW_TEXT_LIMIT, TIER_SCALE } from "./constants.ts";
import { nameTier, textTier, useFitText } from "./fit.ts";
import { Icon } from "./icons.tsx";
import { GLITCH_WORDS, isGlitch } from "./glitch.ts";
import { GlitchBlob } from "./GlitchBlob.tsx";
import { showsFullArt, useFullArtTier } from "./fullArt.ts";
import { FullArtSignature } from "./FullArtSignature.tsx";
import { ALL_TRIBES, creditsLine, foilFor, frameRarity, frameTags, type FaceModel, type FaceText } from "./model.ts";
import { RulesText, printedValue } from "./RulesText.tsx";
import { StarshipPieces } from "./StarshipPieces.tsx";
import { setMarkOf } from "./setMark.ts";
import { useCardSettings } from "./settings.ts";
import { cutText, overLimit, shortFormOf, summitShortText } from "./shortText.ts";
import { questFaceText } from "./questFace.ts";
import { MINUS } from "./tuning.ts";

import "./cards.css";
import "./setmark.css";
import "./cosmetics.css";

export type CardFaceProps = {
  face: FaceModel;
  layout?: "full" | "compact";
  className?: string;
  /**
   * One face of a long grid: its art is drawn once it has dwelt near the screen (CardArt `lazy`),
   * and (R1956000) a cosmetic it wears holds still, taking none of the moving slots.
   */
  lazyArt?: boolean;
  /**
   * Issue #814: the Summit's Ascent level as the view gives it, in a game (faceText); a Summit without
   * one (the Almanac, the deck builder, the inspect overlays) prints the whole text.
   */
  ascentLevel?: number | null;
  worn?: boolean;
};

/** The art window's shape follows the card type, as Hearthstone's minion oval and spell frame do. */
const ART_SHAPE: Readonly<Record<CardType, ArtShape>> = {
  Unit: "portrait",
  Spell: "window",
  "Field Spell": "arch",
  Trap: "notched",
  "Field Trap": "notched",
};

function join(...parts: (string | false | undefined)[]): string {
  return parts.filter((part): part is string => typeof part === "string" && part.length > 0).join(" ");
}

/** A keyword in the face's words: "Armor 2", or on a Chinese face "护甲2" (R1301). */
function keywordWords(face: FaceModel, keyword: Keyword): string {
  return face.chinese === true ? chineseKeyword(keyword) : keywordKey(keyword);
}

/** A keyword kind in the face's words (a Degrade's removed one, R386), Chinese on a Chinese face. */
function kindWords(face: FaceModel, kind: KeywordKind): string {
  return face.chinese === true ? CHINESE_TERMS.keywords[kind] : kind;
}

/** The keywords a face in play has gained, as the rules box prints them. Empty when none. */
export function gainedLine(face: FaceModel): string {
  return face.gained.map((keyword) => keywordWords(face, keyword)).join(face.chinese === true ? CHINESE_COMMA : ", ");
}

/** R386: the keyword chips a tuned face prints, as words: "+Rush −Taunt". Empty when none. */
export function tuningLine(face: FaceModel): string {
  const tuning = face.tuning;
  if (tuning === undefined || tuning === null) return "";
  return [
    ...tuning.added.map((keyword) => `+${keywordWords(face, keyword)}`),
    ...tuning.removed.map((kind) => `${MINUS}${kindWords(face, kind)}`),
  ].join(" ");
}

/** R1177001: the word a token's type line prints. */
export const TOKEN_WORD = "Token";

/** R1177001: the type line in the face's words: the card type, after "Token" on a token (R1301: 衍生物 on a Chinese face). */
export function typeLine(face: FaceModel): string {
  if (face.chinese === true) {
    const type = CHINESE_TERMS.types[face.type];
    return face.token === true ? `${CHINESE_TERMS.tags.Token}${type}` : type;
  }
  return face.token === true ? `${TOKEN_WORD} ${face.type}` : face.type;
}

/** A frame tag in the face's words (R1301). */
function tagWords(face: FaceModel, tag: Tag | typeof ALL_TRIBES): string {
  if (tag === ALL_TRIBES) return face.chinese === true ? CHINESE_TERMS.allTribes : ALL_TRIBES;
  return face.chinese === true ? CHINESE_TERMS.tags[tag] : tag;
}

/** Everything the rules box prints, as one string: what `textTier` and `useFitText` measure. */
function printedText(face: FaceModel, text: string): string {
  const values = face.values.map((entry) => ` {${printedValue(entry)}}`).join("");
  return [`${text}${values}`, creditsLine(face), gainedLine(face), ...face.grants, tuningLine(face)].filter((part) => part !== "").join(" ");
}

/** What a face's rules box prints (issues #814, #1017). */
export type RulesBox = {
  /** The words the rules box prints: the whole text, or a short form ending in "…". */
  shown: string;
  /** The short form, when the face prints one (`data-short`). */
  short: string | null;
  /** The face prints less than its whole text: its rules box clamps, never shrinking the art. */
  less: boolean;
  /**
   * R2450000: a Quest card's `shown` with its own marks and tuned numbers, where they stand in it
   * (questFace.ts). Absent wherever `shown` is the whole text or its start.
   */
  text?: FaceText;
};

/**
 * Prints less than the whole text, which the hover preview prints on the face's left: a Summit in play given
 * its level prints its header and the next Ascent line; a Quest card (R1837000) the stage it stands at
 * (questFace.ts, R2450000); a face printing its catalog text its short form (shortText.ts SHORT_FORMS); a
 * face in play past the hover preview's limit clamps. `fullArtOn` is the caller's `useFullArt(defId)`
 * (R1108000, R1313000); without it, the setting decides.
 */
export function faceText(face: FaceModel, ascentLevel?: number | null, fullArtOn?: boolean): RulesBox {
  const full = face.text.full;
  if (showsFullArt(face, fullArtOn)) return { shown: "", short: null, less: printedText(face, full) !== "" };
  // R2450000: a staged Quest prints one stage, a short form of its own; In Too Deep in play also clamps.
  const quest = questFaceText(face);
  if (quest !== null) {
    const less = quest.less || (face.inPlay && overLimit(printedText(face, quest.full), PREVIEW_TEXT_LIMIT));
    return { shown: quest.full, short: quest.less ? quest.full : null, less, text: quest };
  }
  const summit = summitShortText(face.defId, full, ascentLevel);
  // A face in play that prints other than its catalog text (a #98 Heroic Power's rolled power, a
  // Vanilla unit's lost text, inPlay.ts) takes no short form; a Call to Chaos's "???" has no colon to cut at.
  const form = full === "" || face.printed !== null ? null : shortFormOf(face.defId, face.radiant);
  const short = summit ?? (form === null ? null : cutText(full, form));
  const shown = short ?? full;
  const less = short !== null || form === "clamp" || (face.inPlay && overLimit(printedText(face, shown), PREVIEW_TEXT_LIMIT));
  return { shown, short, less };
}

/** R386: the keywords a Buff added ("+") and a Nerf removed (struck "−"), at the foot of the rules box. */
function TuningKeywords({ face }: { face: FaceModel }): ReactElement | null {
  const tuning = face.tuning;
  if (tuning === undefined || tuning === null || (tuning.added.length === 0 && tuning.removed.length === 0)) return null;
  return (
    <span className="cf-text-tuning">
      {tuning.added.map((keyword) => (
        <span key={`+${keywordKey(keyword)}`} className="cf-kw-chip" data-tuned="added" title={t("Gained {keyword}", { keyword: keywordWords(face, keyword) })}>
          <span className="cf-kw-sign" aria-hidden="true">
            +
          </span>
          {keywordWords(face, keyword)}
        </span>
      ))}
      {tuning.removed.map((kind) => (
        <span key={`-${kind}`} className="cf-kw-chip" data-tuned="removed" title={t("Lost {keyword}", { keyword: kindWords(face, kind) })}>
          <span className="cf-kw-sign" aria-hidden="true">
            {MINUS}
          </span>
          {kindWords(face, kind)}
        </span>
      ))}
    </span>
  );
}

/**
 * Hearthstone draws no gem on Free and Core cards; tokens and unknown cards get none here, except a
 * token that prints a rarity (R503), whose frame shows that rarity.
 */
function hasRarityGem(rarity: Rarity | null): boolean {
  return rarity !== null && rarity !== "Token";
}

function crested(rarity: Rarity | null): boolean {
  return rarity === "Legendary" || rarity === "Mythic";
}

/** R503: the set mark, on every face whose set is known; nothing is guessed for an unknown card. */
function SetMarkBadge({ set }: { set: string | null }): ReactElement | null {
  if (set === null) return null;
  const mark = setMarkOf(set);
  return (
    <span className="cf-set" data-set={set} data-set-mark={mark.kind} role="img" aria-label={mark.label} title={mark.label}>
      <img className="cf-set-glyph" src={mark.src} alt="" aria-hidden="true" draggable={false} />
    </span>
  );
}

/**
 * B8, R1175000: an embiggen card's Embiggened cost, small beside the gem, named "Embiggened (4)" for a
 * screen reader and on hover, as the card's text labels that version.
 */
function EmbiggenedCost({ cost }: { cost: string }): ReactElement {
  const label = t("Embiggened ({cost})", { cost });
  return (
    <span className="cf-cost-alt" role="img" aria-label={label} title={label}>
      {cost}
    </span>
  );
}

/** Longer than two characters (Ceaseless Void's 100): cards.css steps the gem's number down. */
const LONG_COST_CHARS = 3;

export function costDigits(text: string): string | undefined {
  return text.length >= LONG_COST_CHARS ? String(LONG_COST_CHARS) : undefined;
}

/** The board minion's crest (MinionFace), by the rarity its frame shows (a token's printed one, B2.5). */
export function hasCrest(face: FaceModel): boolean {
  return crested(frameRarity(face));
}

export function CardFace({ face, layout = "full", className, lazyArt = false, ascentLevel, worn = false }: CardFaceProps): ReactElement {
  useLocale();
  const settings = useCardSettings();
  // R1313000: a full art prints on the viewer's own copy (`worn`) once they own it, where ownership decides;
  // R2655000: at the tier its pick names.
  const fullArtTier = useFullArtTier(face.defId, worn);
  const fullArtOn = fullArtTier !== null;
  const nameRef = useRef<HTMLSpanElement>(null);
  const textRef = useRef<HTMLSpanElement>(null);

  const full = layout === "full";
  // Issues #814, #1017: a short form is the full text's prefix, so its values (R280), marks and tuned
  // numbers keep their offsets, and RulesText drops marks past the printed words. A Quest card's
  // stage (R2450000) is no prefix, so it brings its marks and tuned numbers with it (`text`).
  const { shown, short, less, text } = faceText(face, ascentLevel, fullArtOn);
  const words = text ?? face.text;
  // #912: the hover preview shows the whole card, so the native tooltip only stands in for it
  // while hover previews are off, as Card.tsx's root title does (B19).
  const tooltip = settings.hoverPreviews || face.text.full === "" ? undefined : face.text.full;
  const printed = printedText(face, shown);
  const names = nameTier(face.name);
  const texts = textTier(printed);
  useFitText(nameRef, face.name);
  // The rules box only exists on a full face; keying on the layout refits it when one appears. A face
  // that prints less than its text clamps at once, its art whole (#1017; fit.ts).
  useFitText(textRef, full ? printed : "", { floorPx: FIT_FLOOR_PX, long: !less });

  const scales = { "--cf-name-scale": String(TIER_SCALE[names]), "--cf-text-scale": String(TIER_SCALE[texts]) } as CSSProperties;
  const rarity = frameRarity(face);
  // R1295000: what the face wears, judged by the rarity its frame shows as its rarity foil is.
  // R1956000: a grid's faces hold theirs still, so the slots stay with the card a player opens.
  const cosmetics = useCosmeticAttributes({ ...face, rarity }, worn, settings.animatedFoil && !lazyArt);
  // Glitch is a blank card (glitch.ts): no picture, set mark or tags, its words corrupted, and its
  // blob spilling out over the frame, outside the clipped `.cf-scale`.
  const glitch = isGlitch(face.defId);
  const fullArt = full && showsFullArt(face, fullArtOn);
  const tier = fullArt ? fullArtTier : null;
  const moving = tier === "Legendary" || tier === "Mythic";

  return (
    <span
      className={join("cf", `cf--${layout}`, className, moving && "cf-full-art--animated", tier === "Mythic" && "cf-full-art--signed")}
      data-layout={layout}
      data-card-type={face.type}
      data-rarity={rarity ?? undefined}
      data-printed-rarity={face.printedRarity ?? undefined}
      data-name-tier={names}
      data-text-tier={texts}
      data-foil={foilFor({ ...face, rarity }, settings.animatedFoil)}
      data-radiant-face={face.radiant ? "true" : undefined}
      data-in-play={face.inPlay ? "true" : undefined}
      data-vanilla={face.vanilla ? "true" : undefined}
      data-tuned={face.tuning?.verdict}
      data-glitch={glitch ? "true" : undefined}
      data-full-art={fullArt ? "true" : undefined}
      data-full-art-tier={tier ?? undefined}
      data-full-art-motion={moving ? (settings.animatedFoil ? "animated" : "static") : undefined}
      {...cosmetics}
      style={scales}
    >
      <span className="cf-scale">
        <span className="cost-gem" data-cost={face.cost.value} data-tone={face.cost.tone} data-digits={costDigits(face.cost.text)}>
          {face.cost.text}
          {face.cost.alt !== null && <EmbiggenedCost cost={face.cost.alt} />}
        </span>

        {face.element != null && (
          <span className="cf-element" data-testid="card-element" title={t("Element {element}", { element: t(face.element) })}>
            {t(face.element)}
          </span>
        )}

        {crested(rarity) && (
          <span className="cf-crest">
            <Icon name="crest" />
          </span>
        )}

        <span className="cf-art-frame">
          {glitch ? (
            <span className={`cf-art cf-art--${ART_SHAPE[face.type]} cf-glitch-void`} aria-hidden="true" />
          ) : (
            <CardArt
              defId={face.defId}
              radiant={face.radiant}
              tags={face.tags}
              type={face.type}
              name={face.englishName ?? face.name}
              shape={fullArt ? "full" : ART_SHAPE[face.type]}
              lazy={lazyArt}
              starship={face.inPlay ? face.starship : null}
            />
          )}
          {moving && <span className="cf-full-art-sweep" aria-hidden="true" />}
        </span>

        <span className="card-name" ref={nameRef}>
          {face.name}
        </span>

        {tier === "Mythic" && <FullArtSignature name={face.englishName ?? face.name} />}

        {hasRarityGem(rarity) && (
          <span className="cf-gem" data-rarity={rarity ?? undefined}>
            <Icon name="gem" />
          </span>
        )}

        {!glitch && <SetMarkBadge set={face.set} />}

        <CardStates face={face} />

        {!fullArt && <span className="card-type">{glitch ? GLITCH_WORDS.type : typeLine(face)}</span>}

        {full && !fullArt && (
          <span
            className="card-text"
            ref={textRef}
            title={tooltip}
            data-short={short !== null ? "true" : undefined}
            data-less={less ? "true" : undefined}
          >
            {face.inPlay && face.starship != null ? (
              // R2249000: a Starship lists its pieces in place of its text.
              <StarshipPieces list={face.starship.list} />
            ) : (
              <span className="cf-text-base">
                <RulesText
                  text={shown}
                  marks={words.marks}
                  refs={face.refs}
                  values={face.values}
                  {...(words.tuned === undefined ? {} : { tuned: words.tuned })}
                  chinese={face.chinese === true}
                />
              </span>
            )}
            {face.credits != null && (
              <span className="cf-text-credits" data-credits-min={face.credits.min} data-credits-max={face.credits.max}>
                {creditsLine(face)}
              </span>
            )}
            {face.gained.length > 0 && (
              <span className="cf-text-gained" data-gained={face.gained.map(keywordKey).join("|")}>
                <RulesText text={gainedLine(face)} />
              </span>
            )}
            {face.grants.map((grant) => (
              <span key={grant} className="cf-text-granted">
                <RulesText text={grant} />
              </span>
            ))}
            <TuningKeywords face={face} />
          </span>
        )}

        {full && !fullArt && !glitch && frameTags(face.tags).length > 0 && (
          <span className="cf-tags">
            {frameTags(face.tags).map((tag) => (
              <span key={tag} className="cf-tag" data-tag={tag}>
                {tagWords(face, tag)}
              </span>
            ))}
          </span>
        )}

        {/* A Unit's, and an Animated card's (B3.1): faceModel gives no other card stats. */}
        {full && face.stats !== null && (
          <span className="cf-stats">
            <span
              className="cf-atk"
              data-face-attack={face.stats.attack}
              data-tone={face.stats.attackTone}
              data-grew={face.stats.grew?.attack === true ? "true" : undefined}
              data-tuned={face.tuning?.attack}
            >
              <Icon name="sword" />
              <span className="cf-num">{face.stats.attack}</span>
            </span>
            <span
              className="cf-hp"
              data-face-health={face.stats.health}
              data-tone={face.stats.healthTone}
              data-grew={face.stats.grew?.health === true ? "true" : undefined}
              data-tuned={face.tuning?.health}
            >
              <Icon name="drop" />
              <span className="cf-num">{face.stats.health}</span>
            </span>
          </span>
        )}
      </span>
      {glitch && <GlitchBlob />}
    </span>
  );
}
