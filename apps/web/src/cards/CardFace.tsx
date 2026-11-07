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
// and inside cosmetics.css's motion gate. R2715000: an Animated art (tier "Animated", no `data-full-art`) prints
// the normal face. R2715001: on an Animated, Legendary or Mythic full face the art's own layers move
// (`data-art-motion`, CardArt's `motion`), taking a foil's slot (`useArtMotion`), and hold still under reduced
// motion, with Animated foil off and on a lazy face.
//
// R2725000: the face wears its look (fullArt.ts `useCardLook`): its version (`data-version`; the Full art version prints
// the full-art layout), its `data-full-art-tier` the tier the look printed as before #2725 (`tierOfLook`). R2725003:
// Animated moves the art on either face. R2725004: a Radiant face keeps every Radiant mark under every look.
// R3015002: the look is one level (`data-level`): every level of a variant draws its first level's picture
// (`artPictureOf`), a level adding only the full-art frame, its picture's own motion (`artMotionOf`) and a signature;
// a signed level is its variant's full art, moving, with the signed name (FullArtSignature.tsx), and no picture or
// motion of its own (R2949000's Signature look is Full art's signed level, R3015005).
// R2940001: Alt full art prints the full-art layout as Full art does (versions.ts `isFullBleed`); Alt art and Alt full
// art draw Alt art's picture and the Sketch the normal one in pencil (`artPictureOf`, CardArt's `picture`), and the
// Sketch's base frame is a pencilled outline (cosmetics.css), its full art's too (R3015002). R2940002: Animated, a
// foil, a finish and a border layer on each.
// R2942001: Extended art lays the card's own picture across the whole face (CardArt's `shape` "full") and keeps every
// part of the normal face over it, its frame and rules box translucent (cards.css); Pixel and Comic restyle the normal
// picture (`artPictureOf`), Pixel setting the name in its pixel face (pixelName.ts, `data-pixel-name`, the words
// kept unseen) and Comic in a burst (cards.css). R2942002: Animated, a foil, a finish and a border layer on each.
// R2943001: Set showcase draws the card in its set's own look (`data-showcase`, pictures.ts `showcaseFamilyOf`): the
// picture in its set's treatment (`artPictureOf` by the face's set), and on a base face the set's edge, bezel and name
// banner (cards.css). Out of frame draws the picture's backdrop in a smaller window and its subject cut out over the
// frame (`.cf-art--cutout`, CardArt's `part`), inside the art frame, so under the cost gem, crest, name, stats and rules
// box and inside the card's rim; a Starship in play and a Legacy card, whose pictures have no parts, keep the normal
// window (CardArt `hasArtParts`). R2943002: Animated, a foil, a finish and a border layer on each.
// R2944001: Immersive draws its window's backdrop hazed as far off (`artPictureOf`'s "depth") and its subject on a near
// layer over it (`.cf-immersive`, the window's shape, CardArt's `part`), inside the window; a Starship in play and a
// Legacy card draw their one picture hazed. Prismatic draws its picture in chrome under a sheen, and on a base face a
// rainbow edge, a chrome bezel and a chrome banner (cards.css). R2944002: Animated, a foil, a finish and a border layer
// on each. Immersive's near layer follows the pointer and the device's tilt and breathes
// (`data-immersive-motion`, immersive.ts `useImmersiveShift`) while it holds a motion slot, the one Animated's motion
// shares on that face (`useArtMotion`), and holds still under reduced motion, with Animated foil off and on a lazy face.
//
// R1295000: the viewer's own card in a game (`worn`) wears the device's card cosmetics, each on its own attribute
// apart from the rarity foil (cosmetics.ts). R2521009: a Resource's full face prints what it sells for ("Credits:
// 2–5") under its text.
//
// R3046000: a compact face (a face-up backrow card) prints full art too: the art across the face, still, and its name in
// a band below the art, with the cost gem, the rarity gem and set mark, the crest and the states over it and no type line
// (`data-full-art` on a compact face, cards.css; fullArt.ts `boardLookOf`). A compact face that is not full art keeps its window.
// R3340000: a compact face in play (`onBoard`) moves its art when its card wears an animated or signed level or is a Mythic, as
// a full face does, drawing no signature, sweep or drift, taking a motion slot ranked by the pointer, focus and rarity
// (`useArtMotion`, motionSlots.ts), and answers the pointer and a press (artLive.ts, `data-art-hover`): a hovered one claims a
// slot to answer in. Its art is in layers while it moves (CardArt `live`).
//
// R2723000: a full-art face in the viewer's own hand (`ownHand`, which Card.tsx sets) is its art alone
// (`data-full-art-hand`), with its cost gem, a Unit's stats, its states, its motion and its cosmetics:
// it has no name banner, signature, rarity gem, set mark or crest. Its name stays in its words,
// visually hidden (`.cf-hand-name`), for a screen reader and the by-name finders.
//
// R2718000: a face with a rules box, blank ones included, prints its set's watermark behind the words: its set
// mark (R503) large, faint and in one ink (`.cf-watermark`, an `img` with no text, setMark.ts `watermarkOf`).
// Glitch, a face with no box (compact, full art), a set with no symbol and the Set watermarks setting off print
// none. It takes no pointer, so the text keeps its contrast and tooltip, and hides with the box.
//
// R3013002: the face wears its card's proficiency stamp (stamps.ts): the account's pick on the viewer's own copy
// (`worn`), the seat's frozen one below a `TheirCopy`; ProficiencyStamp.tsx draws it in stamps.css's slot, clear
// of every part, base and Radiant alike.
//
// R3338002: given `gemDrain` (Card.tsx, a Ceaseless Void in its owner's own hand), the cost gem's level falls as its
// cost does (`data-gem-drain`, `--cost-drain`, cards.css); every other face's gem is drawn whole.
//
// R3344000: a face that is Epic or rarer, or prints full art, draws its picture as depth planes by the painter's roles
// (`data-depth`, depth.ts `hasDepth`, CardArt's `part` a plane): the base window's picture is the far plane and
// `.cf-planes` stacks the middle, near and nearest over it, with a glare layer over the art and one over the frame
// (`.cf-glare`, depth.css). A delivered file, a Starship in play, a Legacy card, a restyled picture, Immersive, Out of
// frame, Extended art and Glitch draw none; nor does any face under reduced motion, the system's setting or the fx
// intensity off (`useDepthMotion`), which also hold such a face's foil still. A Legendary or Mythic face in the viewer's
// own hand or in inspect lets its subject break the window's top edge (`data-frame-break`). R3344001: given `tilt`, the face
// tilts with the pointer, the device or a drag (tilt.ts `useCardTilt`), one face at a time.

import { useRef, type CSSProperties, type ReactElement } from "react";

import { keywordKey, type CardType, type Rarity } from "@jackioh/shared";

import { CardArt, type ArtShape } from "./art/index.ts";
import { hasArtParts, hasArtPlanes } from "./art/CardArt.tsx";
import { ART_PLANES, showcaseFamilyOf } from "./art/pictures.ts";
import { CardStates } from "./CardStates.tsx";
import { useArtLive } from "./artLive.ts";
import { useImmersiveShift } from "./immersive.ts";
import { slotRarityOf } from "./motionSlots.ts";
import { t, useLocale } from "../i18n/i18n.ts";
import { useArtMotion, useCosmeticAttributes } from "./cosmetics.ts";
import {
  CARD_DEPTH_STEP_CQW,
  CARD_FRAME_BREAK_PX,
  CARD_FRAME_BREAK_SCALE,
  CARD_GLARE_SIZE_PCT,
  FIT_FLOOR_PX,
  PREVIEW_TEXT_LIMIT,
  TIER_SCALE,
} from "./constants.ts";
import { breaksFrame, glareFor, hasDepth, useDepthMotion } from "./depth.ts";
import { nameTier, overLimit, textTier, useFitText } from "./fit.ts";
import { Icon } from "./icons.tsx";
import { isGlitch } from "./glitch.ts";
import { GlitchBlob } from "./GlitchBlob.tsx";
import { artMotionOf, boardLookOf, boardNameOf, showsFullArt, tierOfLook, useCardLook } from "./fullArt.ts";
import { FullArtSignature } from "./FullArtSignature.tsx";
import { foilFor, frameRarity } from "./frame.ts";
import type { FaceModel, RichText } from "./model.ts";
import { RulesText, printedValue } from "./RulesText.tsx";
import { StarshipPieces } from "./StarshipPieces.tsx";
import { setMarkOf, watermarkOf } from "./setMark.ts";
import { useCardSettings } from "./settings.ts";
import { pixelNameUri } from "./pixelName.ts";
import { useCardTilt, type TiltMode } from "./tilt.ts";
import {
  NORMAL_LOOK,
  VERSION_EXTENDED_ART_ID,
  VERSION_IMMERSIVE_ID,
  VERSION_OUT_OF_FRAME_ID,
  VERSION_PIXEL_ID,
  VERSION_SET_SHOWCASE_ID,
  artPictureOf,
  isFullBleed,
} from "./versions.ts";
import { ProficiencyStamp } from "./ProficiencyStamp.tsx";
import { useCardStamp } from "./stamps.ts";

import "./cards.css";
import "./setmark.css";
import "./cosmetics.css";
import "./depth.css";

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
  /** R2723000: a card in the viewer's own hand on a board they play (Card.tsx): a full-art face there draws its art alone. */
  ownHand?: boolean;
  /**
   * R3339000: a compact face that is a card in play on the board (Card.tsx's backrow, the Collection's board view): its name
   * and type line follow "Names on the board" and an art-first card drops them. Any other compact face (a picker's tile,
   * a notice, a cosmetic's sample) prints its name as it always has.
   */
  onBoard?: boolean;
  /**
   * R3338002: how far the cost gem has drained, 0 to 1 (gemDrain.ts `gemDrainOf`), which Card.tsx gives only a card
   * in its owner's own hand whose gem drains; absent or null, the gem is drawn whole.
   */
  gemDrain?: number | null;
  /**
   * R3344001: where this face listens for what tilts it, when it has depth (R3344000): its parent ("host": a card in hand,
   * on the board or in the Collection), the window ("window": the hover preview) or a drag on it ("drag": inspect);
   * "off" by default, so a face in a list or a picker never tilts.
   */
  tilt?: TiltMode;
};

/** The art window's shape follows the card type, as Hearthstone's minion oval and spell frame do. */
const ART_SHAPE: Readonly<Record<CardType, ArtShape>> = {
  Unit: "portrait",
  Spell: "window",
  "Field Spell": "arch",
  Trap: "notched",
  "Field Trap": "notched",
};

/** R3344000: the planes `.cf-planes` stacks over the base window's far one, nearest last. */
const OVER_PLANES = ART_PLANES.slice(1);

/** The digits `--cf-depth-step` is written with, in cqw. */
const DEPTH_STEP_DIGITS = 2;

/** A class list from the parts that are non-empty strings, in order. */
export function cx(...parts: (string | false | null | undefined)[]): string {
  return parts.filter((part): part is string => typeof part === "string" && part.length > 0).join(" ");
}

/** The keyword chips a tuned face prints, as words: "+Rush −Taunt". Empty when none. */
export function tuningLine(face: FaceModel): string {
  return face.tuningChips.map((chip) => `${chip.sign}${chip.text}`).join(" ");
}

/** Everything the rules box prints, as one string: what `textTier` and `useFitText` measure. */
function printedText(face: FaceModel, text: string): string {
  const values = face.values.map((entry) => ` {${printedValue(entry)}}`).join("");
  return [`${text}${values}`, face.creditsLine, face.gainedLine.full, ...face.grants.map((grant) => grant.full), tuningLine(face)]
    .filter((part) => part !== "")
    .join(" ");
}

/** What a face's rules box prints (issues #814, #1017). */
export type RulesBox = {
  /** The words the rules box prints: the whole text, or a short form ending in "…". */
  shown: RichText;
  /** The face prints a short form (`data-short`). */
  short: boolean;
  /** The face prints less than its whole text: its rules box clamps, never shrinking the art. */
  less: boolean;
};

/**
 * Prints less than the whole text, which the hover preview prints on the face's left: a Summit in play given
 * its level prints its header and the next Ascent line; a Quest card (R1837000) the stage it stands at; a
 * face printing its catalog text its short form; a face in play past the hover preview's limit clamps. The
 * face model decided every one of those (`face.short`, `.summit`, `.questBox`, `.clamp`); the box's size is
 * the layout's. `fullArtOn` is the caller's `useFullArt(defId)` (R1108000, R1313000); without it, the
 * setting decides. `ascentLevel` is the Summit's level where the surface gives it.
 */
export function faceText(face: FaceModel, ascentLevel?: number | null, fullArtOn?: boolean): RulesBox {
  if (showsFullArt(face, fullArtOn)) return { shown: EMPTY_TEXT, short: false, less: printedText(face, face.text.full) !== "" };
  // R2450000: a staged Quest prints one stage, a short form of its own; In Too Deep in play also clamps.
  const quest = face.questBox;
  if (quest !== null) {
    const less = quest.less || (face.inPlay && overLimit(printedText(face, quest.text.full), PREVIEW_TEXT_LIMIT));
    return { shown: quest.text, short: quest.less, less };
  }
  const summit = ascentLevel === null || ascentLevel === undefined ? undefined : face.summit;
  const short = summit ?? face.short;
  const shown = short ?? face.text;
  const less = short !== undefined || face.clamp || (face.inPlay && overLimit(printedText(face, shown.full), PREVIEW_TEXT_LIMIT));
  return { shown, short: short !== undefined, less };
}

const EMPTY_TEXT: RichText = { full: "", spans: [] };

/** R386: the keywords a Buff added ("+") and a Nerf removed (struck "−"), at the foot of the rules box. */
function TuningKeywords({ face }: { face: FaceModel }): ReactElement | null {
  if (face.tuningChips.length === 0) return null;
  return (
    <span className="cf-text-tuning">
      {face.tuningChips.map((chip) => (
        <span key={`${chip.sign}${chip.text}`} className="cf-kw-chip" data-tuned={chip.added ? "added" : "removed"} title={chip.title}>
          <span className="cf-kw-sign" aria-hidden="true">
            {chip.sign}
          </span>
          {chip.text}
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

export function CardFace({
  face,
  layout = "full",
  className,
  lazyArt = false,
  ascentLevel,
  worn = false,
  ownHand = false, onBoard = false, gemDrain = null,
  tilt = "off", }: CardFaceProps): ReactElement {
  useLocale();
  const settings = useCardSettings();
  // R1313000: a full art prints on the viewer's own copy (`worn`) once they own it, where ownership decides;
  // R2725000, R3015005: the look its pick names, one level (R2655000's tiers among them).
  const cardLook = useCardLook(face.defId, worn);
  // R2715000: the Animated art prints the normal face; R2725000: only the Full art version prints full art, and
  // R2940001 Alt full art.
  const fullArtOn = isFullBleed(cardLook.version);
  const nameRef = useRef<HTMLSpanElement>(null);
  const textRef = useRef<HTMLSpanElement>(null);
  const faceRef = useRef<HTMLSpanElement>(null);

  const full = layout === "full";
  // R1017001: a full-art card fills its full face with its art, and prints no words. R3046000: a compact face (a face-up
  // backrow card) fills itself with its art too, still, and prints its name in a band below the art.
  const fullArt = showsFullArt(face, fullArtOn);
  // R2723000: in the viewer's own hand a full-art face is its art alone: no name banner, signature,
  // rarity gem, set mark or crest. Its cost gem, a Unit's stats, its states and its motion stay.
  const bare = fullArt && ownHand;
  // R3339000: a card in play in the backrow draws its name as "Names on the board" says; an art-first one drops it and its type line.
  const { artFirst, staticName: showStaticName } =
    !full && onBoard ? boardNameOf(face, cardLook, settings.boardNames) : { artFirst: false, staticName: true };
  // Issues #814, #1017: the face model gives the box's words whole, as a short form, or as a Quest's stage
  // (R2450000), each with its own spans; the box's size is the layout's.
  const { shown, short, less } = faceText(face, ascentLevel, fullArtOn);
  // #912: the hover preview shows the whole card, so the native tooltip only stands in for it
  // while hover previews are off, as Card.tsx's root title does (B19).
  const tooltip = settings.hoverPreviews || face.text.full === "" ? undefined : face.text.full;
  const printed = printedText(face, shown.full);
  const names = nameTier(face.name);
  const texts = textTier(printed);
  useFitText(nameRef, bare ? "" : showStaticName ? face.name : `hover ${face.name}`);
  // The rules box only exists on a full face; keying on the layout refits it when one appears. A face
  // that prints less than its text clamps at once, its art whole (#1017; fit.ts).
  useFitText(textRef, full ? printed : "", { floorPx: FIT_FLOOR_PX, long: !less });

  const scales = { "--cf-name-scale": String(TIER_SCALE[names]), "--cf-text-scale": String(TIER_SCALE[texts]) } as CSSProperties;
  const rarity = frameRarity(face);
  const stamp = useCardStamp(face, worn);
  // Glitch is a blank card (glitch.ts): no picture, set mark or tags, its words corrupted, and its
  // blob spilling out over the frame, outside the clipped `.cf-scale`.
  const glitch = isGlitch(face.defId);
  // R2718000: the set's watermark behind the rules text, unless Glitch's blank, or the setting is off.
  const watermark = settings.setWatermarks && !glitch ? watermarkOf(face.set) : null;
  // R2715000, R2725000: any full layout keeps its look, the Default's animated level too; a compact face wears none but
  // (R3046000) the version a full-art one prints, still, and no tier.
  const look = full ? cardLook : (boardLookOf(face, cardLook) ?? NORMAL_LOOK);
  const tier = full ? tierOfLook(look) : null;
  // R2725003: an animated full art lays the band over it and drifts it. R3015002: a signed level signs the name
  // across its full art, but in the viewer's own hand.
  // R3340000: a compact face's look carries the flags, its art moving by them (`artMoves`); only a full face sweeps and signs.
  const moving = full && fullArt && look.animated;
  const signed = full && fullArt && look.signed && !bare;
  // R2943001: a Starship in play's and a Legacy card's pictures have no parts.
  const starship = face.inPlay ? face.starship : null;
  // R2944001: Immersive lays its subject on a near layer over its hazed backdrop where the picture has parts.
  const immersive = look.version === VERSION_IMMERSIVE_ID && !fullArt && !glitch && hasArtParts(face.defId, starship);
  // R2715001: an Animated look moves its art's own layers while a slot is theirs. R2944002: Immersive's depth moves in
  // the same slot, one per face, Animated or not. R3340000: a card in play moves its art for an animated or signed level and
  // for a Mythic; a hovered or focused one claims a slot to answer in, ranked first (motionSlots.ts).
  const board = !full && onBoard;
  const artMoves = full ? look.animated : board && (cardLook.animated || rarity === "Mythic");
  const live = useArtLive(faceRef, artMoves || board);
  const engaged = board && live.hovered;
  const artMotion = useArtMotion((artMoves || immersive || engaged) && settings.animatedFoil && !lazyArt && live.onScreen, {
    hovered: live.hovered,
    rarity: slotRarityOf(rarity),
  });
  useImmersiveShift(faceRef, immersive && artMotion);
  // R2942001: Extended art's picture runs past its window across the face; Pixel sets the name in its pixel face, when
  // the face has every character of it and it is short enough to read (`PIXEL_NAME_MAX_GLYPHS`; a small face prints
  // it in the normal face, cards.css).
  const extended = look.version === VERSION_EXTENDED_ART_ID;
  const pixelName = look.version === VERSION_PIXEL_ID ? pixelNameUri(face.name) : null;
  // R2943001: Set showcase wears its set's look; Out of frame cuts its subject out over the frame where the picture has
  // parts (a Starship in play's and a Legacy card's have none).
  const showcase = look.version === VERSION_SET_SHOWCASE_ID ? showcaseFamilyOf(face.set) : null;
  const outOfFrame = look.version === VERSION_OUT_OF_FRAME_ID && !fullArt && !glitch && hasArtParts(face.defId, starship);
  // R3015002: one picture per variant at every level, and its own motion at every animated one.
  const picture = artPictureOf(look.version, face.set);
  const motion = artMotion && (artMoves || engaged) ? artMotionOf(face.defId) : null;
  // R3344000: an Epic or rarer face, or a full-art one, draws its picture as depth planes unless the viewer's motion
  // settings ask for the flat picture, which also holds its foil still under the fx intensity off.
  const { flat, fxOff } = useDepthMotion();
  const wantsDepth = hasDepth({ rarity, fullArt, version: look.version, glitch, planed: hasArtPlanes(face.defId, starship, picture, face.radiant) });
  const depth = wantsDepth && !flat;
  const shape: ArtShape = fullArt || extended || artFirst ? "full" : ART_SHAPE[face.type];
  const breaks = depth && breaksFrame(rarity, ownHand, tilt);
  useCardTilt(faceRef, depth ? tilt : "off", !flat);
  // R1295000: what the face wears, judged by the rarity its frame shows as its rarity foil is.
  // R1956000: a grid's faces hold theirs still, so the slots stay with the card a player opens.
  const foilMoves = settings.animatedFoil && !(fxOff && wantsDepth);
  const cosmetics = useCosmeticAttributes({ ...face, rarity }, worn, foilMoves && !lazyArt);
  const glare = glareFor(cosmetics["data-cosmetic-foil"], face.radiant);
  const style = depth
    ? ({
        ...scales,
        "--cf-depth-step": `${Number(CARD_DEPTH_STEP_CQW.toFixed(DEPTH_STEP_DIGITS))}cqw`,
        "--cf-glare-art": String(glare.art),
        "--cf-glare-frame": String(glare.frame),
        "--cf-glare-size": `${CARD_GLARE_SIZE_PCT}%`,
        ...(breaks ? { "--cf-frame-break": `${CARD_FRAME_BREAK_PX}px`, "--cf-frame-break-scale": String(CARD_FRAME_BREAK_SCALE) } : {}),
      } as CSSProperties)
    : scales;

  return (
    <span
      ref={faceRef}
      className={cx("cf", `cf--${layout}`, className, moving && "cf-full-art--animated", signed && "cf-full-art--signed")}
      data-layout={layout}
      data-card-type={face.type}
      data-rarity={rarity ?? undefined}
      data-printed-rarity={face.printedRarity ?? undefined}
      data-name-tier={names}
      data-text-tier={texts}
      data-foil={foilFor({ ...face, rarity }, foilMoves, settings.rarityFoil)}
      data-radiant-face={face.radiant ? "true" : undefined}
      data-in-play={face.inPlay ? "true" : undefined}
      data-vanilla={face.vanilla ? "true" : undefined}
      data-tuned={face.tuning?.verdict}
      data-glitch={glitch ? "true" : undefined}
      data-full-art={fullArt ? "true" : undefined}
      data-art-first={artFirst ? "true" : undefined}
      data-full-art-hand={bare ? "true" : undefined}
      data-full-art-tier={tier ?? undefined}
      data-version={look.version ?? undefined}
      data-level={look.level ?? undefined}
      data-showcase={showcase ?? undefined}
      data-full-art-motion={moving || signed ? (settings.animatedFoil ? "animated" : "static") : undefined}
      data-art-motion={artMoves || engaged ? (artMotion ? "animated" : "static") : undefined}
      data-art-hover={board ? (live.hovered ? "true" : "false") : undefined}
      data-immersive-motion={immersive ? (artMotion ? "animated" : "static") : undefined}
      data-depth={depth ? "true" : undefined}
      data-frame-break={breaks ? "true" : undefined}
      {...cosmetics}
      style={style}
    >
      <span className="cf-scale">
        {depth && <span className="cf-glare cf-glare--frame" aria-hidden="true" />}
        <span
          className="cost-gem"
          data-cost={face.cost.value}
          data-tone={face.cost.tone}
          data-digits={costDigits(face.cost.text)}
          data-gem-drain={gemDrain === null ? undefined : "true"}
          style={gemDrain === null ? undefined : ({ "--cost-drain": String(gemDrain) } as CSSProperties)}
        >
          {/* R3338002: the gem's level, falling as the Void's cost does (cards.css). */}
          {gemDrain !== null && <span className="cf-cost-drain" aria-hidden="true" />}
          {face.cost.text}
          {face.cost.alt !== null && <EmbiggenedCost cost={face.cost.alt} />}
        </span>

        {face.element != null && (
          <span className="cf-element" data-testid="card-element" title={t("Element {element}", { element: t(face.element) })}>
            {t(face.element)}
          </span>
        )}

        {crested(rarity) && !bare && (
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
              shape={shape}
              lazy={lazyArt}
              starship={starship}
              picture={picture}
              motion={motion}
              part={depth ? "far" : outOfFrame || immersive ? "backdrop" : "whole"}
              live={board && motion !== null}
            />
          )}
          {outOfFrame && (
            <CardArt
              defId={face.defId}
              radiant={face.radiant}
              tags={face.tags}
              type={face.type}
              name={face.englishName ?? face.name}
              shape="full"
              className="cf-art--cutout"
              lazy={lazyArt}
              starship={starship}
              picture={picture}
              motion={motion}
              part="subject"
            />
          )}
          {immersive && (
            <span className={`cf-immersive cf-art--${ART_SHAPE[face.type]}`} aria-hidden="true">
              <CardArt
                defId={face.defId}
                radiant={face.radiant}
                tags={face.tags}
                type={face.type}
                name={face.englishName ?? face.name}
                shape="full"
                className="cf-art--near"
                lazy={lazyArt}
                starship={starship}
                picture="normal"
                motion={motion}
                part="subject"
              />
            </span>
          )}
          {depth && (
            <>
              <span className="cf-planes" aria-hidden="true">
                {OVER_PLANES.map((plane, index) => (
                  <span key={plane} className={`cf-plane cf-art--${shape}`} data-plane={plane} style={{ "--cf-plane": String(index + 1) } as CSSProperties}>
                    <CardArt
                      defId={face.defId}
                      radiant={face.radiant}
                      tags={face.tags}
                      type={face.type}
                      name={face.englishName ?? face.name}
                      shape="plane"
                      lazy={lazyArt}
                      starship={starship}
                      picture={picture}
                      motion={motion}
                      part={plane}
                    />
                  </span>
                ))}
              </span>
              <span className={`cf-glare cf-glare--art cf-art--${shape}`} aria-hidden="true" />
            </>
          )}
          {moving && <span className="cf-full-art-sweep" aria-hidden="true" />}
        </span>

        {bare ? (
          <span className="cf-hand-name">{face.name}</span>
        ) : showStaticName ? (
          <span
            className="card-name"
            ref={nameRef}
            data-pixel-name={pixelName !== null ? "true" : undefined}
            style={pixelName !== null ? ({ "--cf-pixel-name": `url("${pixelName}")` } as CSSProperties) : undefined}
          >
            {face.name}
          </span>
        ) : (
          // R3339000: transparent until hover or focus, and in the accessibility tree all the while: the name is read as it was.
          <span className="card-name card-name-hover" ref={nameRef}>
            {face.name}
          </span>
        )}

        {signed && <FullArtSignature name={face.englishName ?? face.name} defId={face.defId} />}

        {hasRarityGem(rarity) && !bare && (
          <span className="cf-gem" data-rarity={rarity ?? undefined}>
            <Icon name="gem" />
          </span>
        )}

        {!glitch && !bare && <SetMarkBadge set={face.set} />}

        <CardStates face={face} />

        {stamp !== null && <ProficiencyStamp level={stamp} />}

        {!fullArt && !artFirst && <span className="card-type">{face.typeLine}</span>}

        {full && !fullArt && (
          <span
            className="card-text"
            ref={textRef}
            title={tooltip}
            data-short={short ? "true" : undefined}
            data-less={less ? "true" : undefined}
          >
            {watermark !== null && (
              <img className="cf-watermark" data-watermark={watermark.kind} src={watermark.src} alt="" aria-hidden="true" draggable={false} />
            )}
            {face.inPlay && face.starship != null ? (
              // R2249000: a Starship lists its pieces in place of its text.
              <StarshipPieces starship={face.starship} />
            ) : (
              <span className="cf-text-base">
                <RulesText rich={shown} />
              </span>
            )}
            {face.credits != null && (
              <span className="cf-text-credits" data-credits-min={face.credits.min} data-credits-max={face.credits.max}>
                {face.creditsLine}
              </span>
            )}
            {face.gained.length > 0 && (
              <span className="cf-text-gained" data-gained={face.gained.map(keywordKey).join("|")}>
                <RulesText rich={face.gainedLine} />
              </span>
            )}
            {face.grants.map((grant) => (
              <span key={grant.full} className="cf-text-granted">
                <RulesText rich={grant} />
              </span>
            ))}
            <TuningKeywords face={face} />
          </span>
        )}

        {full && !fullArt && !glitch && face.tagChips.length > 0 && (
          <span className="cf-tags">
            {face.tagChips.map((chip) => (
              <span key={chip.tag} className="cf-tag" data-tag={chip.tag}>
                {chip.text}
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
