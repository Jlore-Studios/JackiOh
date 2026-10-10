// The tall card (docs/polish/6-cards.md, Surface B "CardFace DOM"): cost gem, art window, name
// ribbon, rarity gem, type line, rules box, tag badges, and sword and drop for units.
//
// Every element is a span, strong or img, so a face can sit inside the deck builder's <button>.
// Nothing here carries `data-testid`, `data-attack`, `data-health`, `data-keyword` or the `card`
// class token: those belong to the board's root element (game/Card.tsx) and the minion form, and a
// face in a hover preview must never answer an e2e selector meant for the board (B12, B20).
//
// `layout="compact"` is a face-up backrow card: cost, art, name, rarity gem and type line, with no
// rules box, tags or stats.
//
// A face in play (FaceModel.inPlay, SPEC §10.10) prints the card as it stands: the rules box ends
// with the keywords it has gained since it was printed (`.cf-text-gained`), and a Vanilla unit's box
// says its text is gone. A fused card's text is a line per ingredient (R102), which the box keeps.
//
// The rules box prints the face's whole text (`.cf-text-base`, on both faces): a Radiant face its
// catalog Radiant text with what the base face lacks marked in gold (R277), the names its `refs`
// link as references (R279), and in play the values its formula comes to, "{7}" (R280) — all
// drawn by RulesText. A printed Radiant unit's attack and health that the Radiant face raised are
// marked too (`data-grew`).
//
// Patch v0.2.0 (SPEC §10.8): a face in play wears the states the view gives the card on a rail of
// badges (CardStates.tsx): its Brittle count (R385), its tuned mark (R386), its enchantments (E39) and
// a card standing as a Unit (R383). A tuned card also marks what changed where it shows: the numbers
// in its text that moved (RulesText's `.cf-tuned`), its stats (`data-tuned` on the sword and drop,
// a ▲ or ▼ pip, beside the usual tones) and its keywords (a "+" chip for each one a Buff added, a struck
// "−" chip for each one a Nerf removed, at the foot of the rules box). An Animated Field Spell or Trap
// shows the attack and health of the Unit it becomes (B3.1), on its full face.
//
// R503: every face with a set shows it as a small mark on the frame (`.cf-set[data-set]`, setMark.ts),
// and a token that prints a rarity (B2.5's `printedRarity`) wears that rarity's frame, gem, crest and
// foil rather than Token's, for display only. The art gets the card's name, which picks its motif.
//
// ME-CN, R1301: a Chinese face (`face.chinese`) prints its name and text as the model gives them (the
// Chinese table's) and its type line, tags and keywords in the table's words (chinese.ts); its art
// keeps the English name's motif, so the picture is the same card's.
//
// R1382: a card with every tribal tag (R1424, Meditative #87) prints "All Tribes" on its tags line in
// place of the five, its other tags after it; its tags themselves are unchanged.

import { useRef, type CSSProperties, type ReactElement } from "react";

import { keywordKey, type CardType, type Keyword, type KeywordKind, type Rarity, type Tag } from "@jackioh/shared";

import { CardArt, type ArtShape } from "./art/index.ts";
import { CardStates } from "./CardStates.tsx";
import { CHINESE_COMMA, CHINESE_TERMS, chineseKeyword } from "./chinese.ts";
import { FIT_FLOOR_PX, TIER_SCALE } from "./constants.ts";
import { nameTier, textTier, useFitText } from "./fit.ts";
import { Icon } from "./icons.tsx";
import { GLITCH_WORDS, isGlitch } from "./glitch.ts";
import { GlitchBlob } from "./GlitchBlob.tsx";
import { ALL_TRIBES, foilFor, frameRarity, frameTags, type FaceModel } from "./model.ts";
import { RulesText, printedValue } from "./RulesText.tsx";
import { setMarkOf } from "./setMark.ts";
import { useCardSettings } from "./settings.ts";
import { MINUS } from "./tuning.ts";

import "./cards.css";
import "./setmark.css";

export type CardFaceProps = {
  face: FaceModel;
  layout?: "full" | "compact";
  className?: string;
  /** One face of a long grid: its art is drawn once it has dwelt near the screen (CardArt `lazy`). */
  lazyArt?: boolean;
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

/** A frame tag in the face's words (R1301). */
function tagWords(face: FaceModel, tag: Tag | typeof ALL_TRIBES): string {
  if (tag === ALL_TRIBES) return face.chinese === true ? CHINESE_TERMS.allTribes : ALL_TRIBES;
  return face.chinese === true ? CHINESE_TERMS.tags[tag] : tag;
}

/** Everything the rules box prints, as one string: what `textTier` and `useFitText` measure. */
function printedText(face: FaceModel): string {
  const values = face.values.map((entry) => ` {${printedValue(entry)}}`).join("");
  return [`${face.text.full}${values}`, gainedLine(face), tuningLine(face)].filter((part) => part !== "").join(" ");
}

/** R386: the keywords a Buff added ("+") and a Nerf removed (struck "−"), at the foot of the rules box. */
function TuningKeywords({ face }: { face: FaceModel }): ReactElement | null {
  const tuning = face.tuning;
  if (tuning === undefined || tuning === null || (tuning.added.length === 0 && tuning.removed.length === 0)) return null;
  return (
    <span className="cf-text-tuning">
      {tuning.added.map((keyword) => (
        <span key={`+${keywordKey(keyword)}`} className="cf-kw-chip" data-tuned="added" title={`Gained ${keywordWords(face, keyword)}`}>
          <span className="cf-kw-sign" aria-hidden="true">
            +
          </span>
          {keywordWords(face, keyword)}
        </span>
      ))}
      {tuning.removed.map((kind) => (
        <span key={`-${kind}`} className="cf-kw-chip" data-tuned="removed" title={`Lost ${kindWords(face, kind)}`}>
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

/** Longer than two characters (Ceaseless Void's 100): cards.css steps the gem's number down. */
const LONG_COST_CHARS = 3;

export function costDigits(text: string): string | undefined {
  return text.length >= LONG_COST_CHARS ? String(LONG_COST_CHARS) : undefined;
}

/** The board minion's crest (MinionFace), by the rarity its frame shows (a token's printed one, B2.5). */
export function hasCrest(face: FaceModel): boolean {
  return crested(frameRarity(face));
}

export function CardFace({ face, layout = "full", className, lazyArt = false }: CardFaceProps): ReactElement {
  const settings = useCardSettings();
  const nameRef = useRef<HTMLSpanElement>(null);
  const textRef = useRef<HTMLSpanElement>(null);

  const full = layout === "full";
  const printed = printedText(face);
  const names = nameTier(face.name);
  const texts = textTier(printed);
  useFitText(nameRef, face.name);
  // The rules box only exists on a full face; keying on the layout refits it when one appears. Its
  // floor keeps dense cards readable: the long layout first, then a clamp (fit.ts).
  useFitText(textRef, full ? printed : "", { floorPx: FIT_FLOOR_PX });

  const scales = { "--cf-name-scale": String(TIER_SCALE[names]), "--cf-text-scale": String(TIER_SCALE[texts]) } as CSSProperties;
  // R503: the frame's rarity, a token's printed one included.
  const rarity = frameRarity(face);
  // Glitch is a blank card (glitch.ts): no picture, set mark or tags, its words corrupted, and its
  // blob spilling out over the frame, outside the clipped `.cf-scale`.
  const glitch = isGlitch(face.defId);

  return (
    <span
      className={join("cf", `cf--${layout}`, className)}
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
      style={scales}
    >
      <span className="cf-scale">
        <span className="cost-gem" data-cost={face.cost.value} data-tone={face.cost.tone} data-digits={costDigits(face.cost.text)}>
          {face.cost.text}
          {face.cost.alt !== null && <span className="cf-cost-alt">{face.cost.alt}</span>}
        </span>

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
              shape={ART_SHAPE[face.type]}
              lazy={lazyArt}
            />
          )}
        </span>

        <span className="card-name" ref={nameRef}>
          {face.name}
        </span>

        {hasRarityGem(rarity) && (
          <span className="cf-gem" data-rarity={rarity ?? undefined}>
            <Icon name="gem" />
          </span>
        )}

        {!glitch && <SetMarkBadge set={face.set} />}

        <CardStates face={face} />

        <span className="card-type">{glitch ? GLITCH_WORDS.type : face.chinese === true ? CHINESE_TERMS.types[face.type] : face.type}</span>

        {full && (
          <span className="card-text" ref={textRef}>
            <span className="cf-text-base">
              <RulesText
                text={face.text.full}
                marks={face.text.marks}
                refs={face.refs}
                values={face.values}
                {...(face.text.tuned === undefined ? {} : { tuned: face.text.tuned })}
                chinese={face.chinese === true}
              />
            </span>
            {face.gained.length > 0 && (
              <span className="cf-text-gained" data-gained={face.gained.map(keywordKey).join("|")}>
                <RulesText text={gainedLine(face)} />
              </span>
            )}
            <TuningKeywords face={face} />
          </span>
        )}

        {full && !glitch && face.tags.length > 0 && (
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
