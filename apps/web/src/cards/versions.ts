// R2725000 (#2725): a card wears one version, its normal face or a `version-…` entry, each owned card by card like a
// foil. R2725001: each is offered on the cards `COSMETIC_OFFERS` lists, the server's own reading of the entry's offer,
// and on no other. R2725002: a retired full art (R2655000, R2715000) counts as owning what it maps to (`impliedBy`).
// R2940000: #2940 adds the Alt art, Sketch and Alt full art versions; R2940001: the full bleed (`isFullBleed`) and the
// picture each version draws (`artPictureOf`). R2942000: #2942 adds the Extended art, Pixel and Comic versions;
// R2942001: Extended art draws the normal picture past its window (CardFace), and Pixel and Comic restyle it
// (`artPictureOf`). R2943000: #2943 adds the Set showcase and Out of frame versions; R2943001: Set showcase restyles
// the picture in its set's own treatment (`artPictureOf` by the card's set), and Out of frame draws its subject apart
// from its backdrop (`artPartOf`, CardFace). R2944000: #2944 adds the Immersive and Prismatic versions; R2944001:
// Immersive draws its backdrop hazed under a near layer of its subject (`artPictureOf`, CardFace), and Prismatic its
// picture in mirror chrome under an iridescent sheen.
// R3015000 (#3015): each look is a variant, a ladder of levels (`COSMETIC_LEVELS`: its variant, its rank and what it
// adds: art, the full-art frame, its picture's motion, a signature), every card's Default first, whose first level is
// the normal face; the Animated and Signature augments are retired. R3015002: every level of a variant draws its first
// level's picture. R3015005: a card's pick names one level (`levelOfPick`, the server's `level_of_pick`), an older
// pick's shape mapped as the server maps it, and wears the highest owned and offered at or below it (`wornLevel`).
// R3017000 (#3017): #3017 adds six two-level variants, Night, Neon sign, Watercolor, Stained-glass window, Woodcut and
// Tarot card; R3017002: each draws its own look (`artPictureOf`), four a new picture of the card's subject under a
// treatment and Woodcut and Neon sign the normal one restyled.
// Presentation only: nothing here reaches the engine, an action, `PlayerView` or the log (CLAUDE.md rule 7).

import { COSMETICS, COSMETIC_LEVELS, COSMETIC_OFFERS } from "../wire/generated/cosmetics.ts";
import { showcaseFamilyOf, type ArtPart, type ArtPicture } from "./art/pictures.ts";

/** R2715000: what an Animated art's registry id starts with, a `fullArt` entry too (migration 0075). */
export const ANIMATED_ART_ID_PREFIX = "animated-";

/** R2655000: what every other full art's registry id starts with (migration 0072). */
export const FULL_ART_ID_PREFIX = "full-art-";

/** R2725000: what a version's registry id starts with, a card pick's `fullArt` text too (migration 0079). */
export const VERSION_ID_PREFIX = "version-";

/** R2725000: what an augment's registry id starts with, each one an older card pick's `augments` lists (R3015000: retired). */
export const AUGMENT_ID_PREFIX = "augment-";

/** R2725000: the Full art version, Legendary: the card's art across its whole face. R3015000: Full art's first level. */
export const VERSION_FULL_ART_ID = "version-full-art";

/** R2725000: the Animated augment. R3015000: retired; an older pick listing it steps up to an animated level (`levelOfPick`). */
export const AUGMENT_ANIMATED_ID = "augment-animated";

/** R2725000: the Signature augment. R3015000: retired; an older pick listing it steps up to a signed level (`levelOfPick`). */
export const AUGMENT_SIGNATURE_ID = "augment-signature";

/** R2940000: the Alt art version, Rare: a new picture of the card's subject in the normal frame. */
export const VERSION_ALT_ART_ID = "version-alt-art";

/** R2940000: the Sketch version, Rare: the card's picture in pencil and ink on paper, its frame a pencilled outline. */
export const VERSION_SKETCH_ID = "version-sketch";

/** R2940000: the Alt full art version, Legendary: Alt art's picture across the whole face. R3015000: Alt art's second level. */
export const VERSION_ALT_FULL_ART_ID = "version-alt-full-art";

/**
 * R2942000: the Extended art version, Epic: the card's own art past its window, behind a translucent rules box and
 * frame.
 */
export const VERSION_EXTENDED_ART_ID = "version-extended-art";

/** R2942000: the Pixel version, Epic: the card's picture as 8-bit pixel art in a retro frame, its name in a pixel face. */
export const VERSION_PIXEL_ID = "version-pixel";

/** R2942000: the Comic version, Epic: the card's picture as pop art, halftone dots and ink outlines, its name in a burst. */
export const VERSION_COMIC_ID = "version-comic";

/** R2943000: the Set showcase version, Legendary: the whole card restyled in its set's own look, frame, art treatment and lettering. */
export const VERSION_SET_SHOWCASE_ID = "version-set-showcase";

/**
 * R2943000: the Out of frame version, Legendary: the card's subject steps out of a smaller window over its frame, under
 * its gem, crest, name, stats and rules text.
 */
export const VERSION_OUT_OF_FRAME_ID = "version-out-of-frame";

/**
 * R2944000: the Immersive version, Mythic: a layered scene, its subject on a near layer over a hazed backdrop, the near
 * layer shifting with the pointer or the device's tilt and breathing slowly; still under reduced motion.
 */
export const VERSION_IMMERSIVE_ID = "version-immersive";

/** R2944000: the Prismatic version, Mythic: mirror chrome, rainbow-edged and iridescent, the top chase; never gold. */
export const VERSION_PRISMATIC_ID = "version-prismatic";

/** R3017000: the Night variant's first level, Rare: a new picture of the card's subject under a night sky, a moon and stars. */
export const VERSION_NIGHT_ID = "version-night";

/** R3017000: the Neon sign variant's first level, Epic: the card's own picture as glowing tubes on a dark wall. */
export const VERSION_NEON_ID = "version-neon";

/** R3017000: the Watercolor variant's first level, Rare: a new picture of the card's subject in washes on paper. */
export const VERSION_WATERCOLOR_ID = "version-watercolor";

/** R3017000: the Stained-glass window variant's first level, Epic: a new picture of the card's subject in leaded panes. */
export const VERSION_STAINED_GLASS_ID = "version-stained-glass";

/** R3017000: the Woodcut variant's first level, Rare: the card's own picture cut as a three-tone print. */
export const VERSION_WOODCUT_ID = "version-woodcut";

/** R3017000: the Tarot card variant's first level, Epic: a new picture of the card's subject as a fortune card's, under an arch. */
export const VERSION_TAROT_ID = "version-tarot";

/** R3015000: the Default variant's second level, Epic: the normal face, its picture moving (R2715001). */
export const VERSION_ANIMATED_ID = "version-animated";

/** R3015000: Full art's second level, Mythic: the full art, its picture moving. */
export const VERSION_FULL_ART_ANIMATED_ID = "version-full-art-animated";

/** R3015000: Full art's third level, Mythic: the full art moving and signed (R2949000's Signature look, R3015005). */
export const VERSION_FULL_ART_SIGNED_ID = "version-full-art-signed";

/** R3015000: Alt art's third level, Mythic: Alt full art, its picture moving. */
export const VERSION_ALT_FULL_ART_ANIMATED_ID = "version-alt-full-art-animated";

/** R3015000: Alt art's fourth level, Mythic: Alt full art moving and signed. */
export const VERSION_ALT_FULL_ART_SIGNED_ID = "version-alt-full-art-signed";

/** R3015000: the Sketch's second level, Epic: the sketch across the whole face. */
export const VERSION_SKETCH_FULL_ART_ID = "version-sketch-full-art";

/** R3015000: the Sketch's third level, Legendary: the sketch's full art, its picture moving. */
export const VERSION_SKETCH_FULL_ART_ANIMATED_ID = "version-sketch-full-art-animated";

/** R3015000: the Sketch's fourth level, Mythic: the sketch's full art moving and signed. */
export const VERSION_SKETCH_FULL_ART_SIGNED_ID = "version-sketch-full-art-signed";

/** R3015000: the variant every card has, whose first level is the normal face, which no registry entry names. */
export const DEFAULT_VARIANT = "default";

/** R3015000: the rank of a variant's first level, the Default variant's the normal face. */
export const FIRST_RANK = 1;

/** R3015000: what a level adds to the one below it (`COSMETIC_LEVELS`' `adds`). */
export type LevelAdds = (typeof COSMETIC_LEVELS)[string]["adds"];

/** R3015000: a level's place: its variant, its rank and what it adds. */
export type VariantLevel = (typeof COSMETIC_LEVELS)[string];

/** R3015000: the level `id` is, or null for any id that is no level. */
export function levelOf(id: string): VariantLevel | null {
  return Object.hasOwn(COSMETIC_LEVELS, id) ? (COSMETIC_LEVELS[id] ?? null) : null;
}

/** R3015000: each variant's levels, lowest first, the variants in the registry's order of their first levels. */
const LADDERS: ReadonlyMap<string, readonly string[]> = (() => {
  const out = new Map<string, string[]>();
  const ids = Object.keys(COSMETIC_LEVELS).sort((a, b) => (levelOf(a)?.rank ?? 0) - (levelOf(b)?.rank ?? 0));
  for (const id of ids) {
    const variant = levelOf(id)?.variant;
    if (variant === undefined) continue;
    out.set(variant, [...(out.get(variant) ?? []), id]);
  }
  return out;
})();

/** R3015000: every variant, the Default first, then the others in the registry's order of their first levels. */
export const VARIANTS: readonly string[] = [
  DEFAULT_VARIANT,
  ...Object.keys(COSMETIC_LEVELS)
    .map((id) => levelOf(id))
    .filter((level): level is VariantLevel => level !== null && level.rank === FIRST_RANK)
    .map((level) => level.variant),
];

/** R3015000: the levels of `variant`, lowest first: none for a variant this client does not know. */
export function ladderOf(variant: string): readonly string[] {
  return LADDERS.get(variant) ?? [];
}

/** R3015003: the levels of `id`'s variant below it, lowest first: none for its first level or an id that is no level. */
export function levelsBelow(id: string): readonly string[] {
  const at = levelOf(id);
  if (at === null) return [];
  return ladderOf(at.variant).filter((below) => (levelOf(below)?.rank ?? at.rank) < at.rank);
}

/**
 * R3015000: the variants a card `defId` has: the Default always (its first level the normal face), then each whose
 * first level it is offered, in `VARIANTS`' order.
 */
export function variantsOn(defId: string): readonly string[] {
  return VARIANTS.filter((variant) => {
    if (variant === DEFAULT_VARIANT) return true;
    const first = ladderOf(variant)[0];
    return first !== undefined && offeredOn(first, defId);
  });
}

/** R3015000: the levels of `variant` offered on `defId`, lowest first (a Legacy card's stop before animated, R2724000). */
export function levelsOn(defId: string, variant: string): readonly string[] {
  return ladderOf(variant).filter((id) => offeredOn(id, defId));
}

/**
 * R3015003, R3015004: whether `id` is a next level on `defId` where `owns` says what the viewer owns there: every level
 * below it is (the server's `is_next`). Anything but a level always is.
 */
export function isNextLevel(id: string, defId: string, owns: (defId: string, id: string) => boolean): boolean {
  return levelsBelow(id).every((below) => owns(defId, below));
}

/**
 * R2940001: whether a face wearing `id` lays its art across the whole face. R3015002: every level whose ladder has a
 * full-art level at or below it (Full art's from its first, Alt art's and the Sketch's from their second).
 */
export function isFullBleed(id: string | null): boolean {
  const at = id === null ? null : levelOf(id);
  if (at === null) return false;
  return ladderOf(at.variant).some((level) => {
    const each = levelOf(level);
    return each !== null && each.rank <= at.rank && each.adds === "fullArt";
  });
}

/** R3015002: the first level of `id`'s variant, whose picture every level of it draws; null for the Default variant and any other id. */
function firstLevelOf(id: string | null): string | null {
  const at = id === null ? null : levelOf(id);
  if (at === null || at.variant === DEFAULT_VARIANT) return null;
  return ladderOf(at.variant)[0] ?? null;
}

/** R3017002: the picture each of part 6's variants draws, by its first level. */
const PART_SIX_PICTURES: ReadonlyMap<string, ArtPicture> = new Map<string, ArtPicture>([
  [VERSION_NIGHT_ID, "night"],
  [VERSION_NEON_ID, "neon"],
  [VERSION_WATERCOLOR_ID, "watercolor"],
  [VERSION_STAINED_GLASS_ID, "stained-glass"],
  [VERSION_WOODCUT_ID, "woodcut"],
  [VERSION_TAROT_ID, "tarot"],
]);

/**
 * R2940001: the picture a face wearing `id` draws (pictures.ts): Alt art's for Alt art and Alt full art, the Sketch for
 * Sketch. R2942001: Pixel's and Comic's for those; Extended art draws the normal one. R2943001: Set showcase its set's
 * own treatment, a card of `set` (the normal picture for a set with none); Out of frame the normal one. R2944001:
 * Immersive its hazed depth (its window's backdrop, CardFace) and Prismatic its chrome. R3015002: every level of a
 * variant its first level's (the Sketch's full art the sketch, Alt art's full arts Alt art's, Full art's the normal).
 * R3017002: each of part 6's its own: Night's, Neon sign's, Watercolor's, Stained-glass window's, Woodcut's and Tarot
 * card's.
 */
export function artPictureOf(id: string | null, set: string | null = null): ArtPicture {
  const version = firstLevelOf(id);
  const partSix = version === null ? undefined : PART_SIX_PICTURES.get(version);
  if (partSix !== undefined) return partSix;
  if (version === VERSION_ALT_ART_ID) return "alt";
  if (version === VERSION_SKETCH_ID) return "sketch";
  if (version === VERSION_PIXEL_ID) return "pixel";
  if (version === VERSION_SET_SHOWCASE_ID) {
    const family = showcaseFamilyOf(set);
    return family === null ? "normal" : `showcase-${family}`;
  }
  if (version === VERSION_IMMERSIVE_ID) return "depth";
  if (version === VERSION_PRISMATIC_ID) return "prismatic";
  return version === VERSION_COMIC_ID ? "comic" : "normal";
}

/**
 * R2943001: the part of its picture a face wearing `id` draws in its window: Out of frame's subject (R3015002: at
 * every level), which a picker's tile shows alone; every other version the whole picture (R2944001: Immersive's tile
 * its whole picture in haze, its window drawing the backdrop under its near layer, CardFace).
 */
export function artPartOf(id: string | null): ArtPart {
  return firstLevelOf(id) === VERSION_OUT_OF_FRAME_ID ? "subject" : "whole";
}

/**
 * R3015000: what a face wears: its level (null for the normal face), the highest art or full-art level at or below it
 * (`version`, null on the Default variant), and whether a level at or below it adds its picture's motion or a signature.
 */
export type CardLook = {
  readonly level: string | null;
  readonly version: string | null;
  readonly animated: boolean;
  readonly signed: boolean;
};

/** R2725000: the normal face, the Default variant's first level. */
export const NORMAL_LOOK: CardLook = Object.freeze({ level: null, version: null, animated: false, signed: false });

/** R3015000: each level's look, one object per level. */
const LOOKS: ReadonlyMap<string, CardLook> = new Map(
  Object.keys(COSMETIC_LEVELS).map((id): [string, CardLook] => {
    const upTo = [...levelsBelow(id), id];
    const adds = (each: string): LevelAdds | undefined => levelOf(each)?.adds;
    const art = upTo.filter((each) => adds(each) === "art" || adds(each) === "fullArt");
    const look = { level: id, version: art.at(-1) ?? null, animated: upTo.some((each) => adds(each) === "animated"), signed: upTo.some((each) => adds(each) === "signed") };
    return [id, Object.freeze(look)];
  }),
);

/** R3015000: the look the level `id` wears: the normal face for null or an id that is no level. */
export function lookOfLevel(id: string | null): CardLook {
  return (id === null ? undefined : LOOKS.get(id)) ?? NORMAL_LOOK;
}

const OFFERS: ReadonlyMap<string, ReadonlySet<string>> = new Map(
  Object.entries(COSMETIC_OFFERS).map(([id, cards]): [string, ReadonlySet<string>] => [id, new Set(cards)]),
);

/** R2725001: whether the card `defId` is offered the level `cosmeticId`; false for any id with no offer (R3015000: a retired augment's among them). */
export function offeredOn(cosmeticId: string, defId: string): boolean {
  return OFFERS.get(cosmeticId)?.has(defId) === true;
}

/** R2725000: whether `cosmeticId` is a version's (R3015000: a level's), which only `COSMETIC_OFFERS` places. */
export function isOfferedId(cosmeticId: string): boolean {
  return OFFERS.has(cosmeticId);
}

/**
 * R2725002, R3015006: what owning the retired entry `id` counts as owning on its card beside itself, as the server's
 * `cosmetics::implied_by` reads it: an Animated art and the Animated augment the Default variant's animated level; a
 * still Epic full art Full art's first level; a Legendary that, its animated level and the Default one; a Mythic those
 * and its signed level. Nothing for Signature alone or any other id.
 */
export function impliedBy(id: string): readonly string[] {
  if (id === AUGMENT_ANIMATED_ID) return [VERSION_ANIMATED_ID];
  const entry = COSMETICS.find((cosmetic) => cosmetic.id === id);
  if (entry === undefined || entry.kind !== "fullArt") return [];
  if (id.startsWith(ANIMATED_ART_ID_PREFIX)) return [VERSION_ANIMATED_ID];
  switch (entry.rarity) {
    case "Epic":
      return [VERSION_FULL_ART_ID];
    case "Legendary":
      return [VERSION_FULL_ART_ID, VERSION_FULL_ART_ANIMATED_ID, VERSION_ANIMATED_ID];
    case "Mythic":
      return [VERSION_FULL_ART_ID, VERSION_FULL_ART_ANIMATED_ID, VERSION_FULL_ART_SIGNED_ID, VERSION_ANIMATED_ID];
    default:
      return [];
  }
}

/** R2655000: whether `defId` has a still Epic full art, what a pick of `true` names (the server's `full_art_of`). */
function hasStillFullArt(defId: string): boolean {
  return COSMETICS.some(
    (cosmetic) => cosmetic.kind === "fullArt" && cosmetic.card === defId && cosmetic.rarity === "Epic" && cosmetic.id.startsWith(FULL_ART_ID_PREFIX),
  );
}

/** R3015005: the level a pick's `fullArt` names on `defId` before its augments, owned or not. */
function baseLevelOf(defId: string, fullArt: boolean | string | null | undefined): string | null {
  if (fullArt === true) return hasStillFullArt(defId) ? VERSION_FULL_ART_ID : null;
  if (typeof fullArt !== "string") return null;
  if (levelOf(fullArt) !== null) return fullArt;
  const entry = COSMETICS.find((cosmetic) => cosmetic.id === fullArt);
  if (entry === undefined || entry.kind !== "fullArt" || entry.card !== defId) return null;
  // The highest level it maps to, a variant other than the Default first, then by rank.
  const above = (a: VariantLevel, b: VariantLevel): boolean =>
    a.variant === DEFAULT_VARIANT && b.variant !== DEFAULT_VARIANT ? false : b.variant === DEFAULT_VARIANT && a.variant !== DEFAULT_VARIANT ? true : a.rank > b.rank;
  let best: string | null = null;
  for (const id of impliedBy(fullArt)) {
    const at = levelOf(id);
    const top = best === null ? null : levelOf(best);
    if (at !== null && (top === null || above(at, top))) best = id;
  }
  return best;
}

/** R3015005: `level` stepped up one rank where the next of its variant adds `adds`; from none, Animated steps to the Default's animated level. */
function stepUp(level: string | null, adds: LevelAdds): string | null {
  const at = level === null ? null : levelOf(level);
  if (at === null) return adds === "animated" ? VERSION_ANIMATED_ID : level;
  const up = ladderOf(at.variant).find((id) => {
    const each = levelOf(id);
    return each !== null && each.rank === at.rank + 1 && each.adds === adds;
  });
  return up ?? level;
}

/**
 * R3015005: the level a card's pick of `fullArt` and `augments` names on `defId`, owned or not, as the server's
 * `cosmetics::level_of_pick` reads it: none for no pick or `false`; Full art's first level for `true` on a full-art
 * card (its still Epic, R2655000); a level by its id; a retired full art of the card the highest level it maps to
 * (`impliedBy`), a non-Default one first; nothing for any other id. Then each old augment steps up one rank where the
 * next adds it: Animated (from no level, the Default variant's animated level), then Signature. So R2949000's
 * Signature look is Full art's signed level, and Alt art or the Sketch with Animated stays at its first level.
 */
export function levelOfPick(defId: string, fullArt: boolean | string | null | undefined, augments: readonly string[] = []): string | null {
  let level = baseLevelOf(defId, fullArt);
  if (augments.includes(AUGMENT_ANIMATED_ID)) level = stepUp(level, "animated");
  if (augments.includes(AUGMENT_SIGNATURE_ID)) level = stepUp(level, "signed");
  return level;
}

/**
 * R3015005: what a card wears of the level `level`: it, where `wears` takes it (offered and owned there), else the
 * highest level below it that `wears` takes, so an older pick whose augment was not owned wears what it wore; none
 * when no level does (the server's `worn_level`).
 */
export function wornLevel(level: string | null, wears: (id: string) => boolean): string | null {
  if (level === null) return null;
  if (wears(level)) return level;
  return [...levelsBelow(level)].reverse().find(wears) ?? null;
}
