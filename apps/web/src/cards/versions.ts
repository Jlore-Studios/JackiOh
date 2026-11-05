// R2725000 (#2725): a card wears one version, its normal face or a `version-…` entry (the Full art version first),
// and the augments layered on it (`augment-…`: Animated, Signature), each owned card by card like a foil. R2725001:
// each is offered on the cards `COSMETIC_OFFERS` lists, the server's own reading of the entry's offer, and on no
// other. R2725002: a retired full art (R2655000, R2715000) counts as owning what it combines (`impliedBy`), and a
// card pick written before #2725 looks as it did (`lookOf`). R2949000: Signature is worn only beside the Full art
// version and Animated (`signatureFits`), and a face wearing it shows the Signature look. R2940000: #2940 adds the Alt
// art, Sketch and Alt full art versions; R2940001: the full bleed of Full art and Alt full art (`isFullBleed`) and the
// picture each version draws (`artPictureOf`). R2942000: #2942 adds the Extended art, Pixel and Comic versions;
// R2942001: Extended art draws the normal picture past its window (CardFace), and Pixel and Comic restyle it
// (`artPictureOf`). R2943000: #2943 adds the Set showcase and Out of frame versions; R2943001: Set showcase restyles
// the picture in its set's own treatment (`artPictureOf` by the card's set), and Out of frame draws its subject apart
// from its backdrop (`artPartOf`, CardFace). R2944000: #2944 adds the Immersive and Prismatic versions; R2944001:
// Immersive draws its backdrop hazed under a near layer of its subject (`artPictureOf`, CardFace), and Prismatic its
// picture in mirror chrome under an iridescent sheen. Presentation only: nothing here reaches the engine, an action,
// `PlayerView` or the log (CLAUDE.md rule 7).

import { COSMETICS, COSMETIC_OFFERS } from "../wire/generated/cosmetics.ts";
import { showcaseFamilyOf, type ArtPart, type ArtPicture } from "./art/pictures.ts";

/** R2715000: what an Animated art's registry id starts with, a `fullArt` entry too (migration 0075). */
export const ANIMATED_ART_ID_PREFIX = "animated-";

/** R2725000: what a version's registry id starts with, a card pick's `fullArt` text too (migration 0079). */
export const VERSION_ID_PREFIX = "version-";

/** R2725000: what an augment's registry id starts with, each one a card pick's `augments` lists. */
export const AUGMENT_ID_PREFIX = "augment-";

/** R2725000: the Full art version, Legendary: the card's art across its whole face. */
export const VERSION_FULL_ART_ID = "version-full-art";

/** R2725000: the Animated augment, Epic: the art's own elements move (R2715001) on whichever version is worn. */
export const AUGMENT_ANIMATED_ID = "augment-animated";

/** R2725000: the Signature augment, Mythic: the card's name signed across its art. */
export const AUGMENT_SIGNATURE_ID = "augment-signature";

/** R2940000: the Alt art version, Rare: a new picture of the card's subject in the normal frame. */
export const VERSION_ALT_ART_ID = "version-alt-art";

/** R2940000: the Sketch version, Rare: the card's picture in pencil and ink on paper, its frame a pencilled outline. */
export const VERSION_SKETCH_ID = "version-sketch";

/** R2940000: the Alt full art version, Legendary: Alt art's picture across the whole face. */
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

/** R2940001: whether a face wearing `version` lays its art across the whole face: Full art and Alt full art. */
export function isFullBleed(version: string | null): boolean {
  return version === VERSION_FULL_ART_ID || version === VERSION_ALT_FULL_ART_ID;
}

/**
 * R2940001: the picture a face wearing `version` draws (pictures.ts): Alt art's for Alt art and Alt full art, the
 * Sketch for Sketch. R2942001: Pixel's and Comic's for those; Extended art draws the normal one. R2943001: Set showcase
 * its set's own treatment, a card of `set` (the normal picture for a set with none); Out of frame the normal one.
 * R2944001: Immersive its hazed depth (its window's backdrop, CardFace) and Prismatic its chrome.
 */
export function artPictureOf(version: string | null, set: string | null = null): ArtPicture {
  if (version === VERSION_ALT_ART_ID || version === VERSION_ALT_FULL_ART_ID) return "alt";
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
 * R2943001: the part of its picture a face wearing `version` draws in its window: Out of frame's subject, which a
 * picker's tile shows alone; every other version the whole picture (R2944001: Immersive's tile its whole picture in
 * haze, its window drawing the backdrop under its near layer, CardFace).
 */
export function artPartOf(version: string | null): ArtPart {
  return version === VERSION_OUT_OF_FRAME_ID ? "subject" : "whole";
}

/**
 * R2725000: what a face wears: its version (null for the normal face) and the augments layered on it. R2949000:
 * `signed` holds only beside the Full art version and `animated`.
 */
export type CardLook = { readonly version: string | null; readonly animated: boolean; readonly signed: boolean };

/** R2725000: the normal face, with no augment. */
export const NORMAL_LOOK: CardLook = Object.freeze({ version: null, animated: false, signed: false });

const OFFERS: ReadonlyMap<string, ReadonlySet<string>> = new Map(
  Object.entries(COSMETIC_OFFERS).map(([id, cards]): [string, ReadonlySet<string>] => [id, new Set(cards)]),
);

/** R2725001: whether the card `defId` is offered the version or augment `cosmeticId`; false for any id with no offer. */
export function offeredOn(cosmeticId: string, defId: string): boolean {
  return OFFERS.get(cosmeticId)?.has(defId) === true;
}

/** R2725000: whether `cosmeticId` is a version's or an augment's, which only `COSMETIC_OFFERS` places. */
export function isOfferedId(cosmeticId: string): boolean {
  return OFFERS.has(cosmeticId);
}

/**
 * R2725002: what owning the retired full art `id` counts as owning on its card beside itself, as the server's
 * `cosmetics::implied_by` reads it: an Animated art the Animated augment; a still Epic the Full art version; a
 * Legendary that and Animated; a Mythic those and Signature. Nothing for any other id.
 */
export function impliedBy(id: string): readonly string[] {
  const entry = COSMETICS.find((cosmetic) => cosmetic.id === id);
  if (entry === undefined || entry.kind !== "fullArt") return [];
  if (id.startsWith(ANIMATED_ART_ID_PREFIX)) return [AUGMENT_ANIMATED_ID];
  switch (entry.rarity) {
    case "Epic":
      return [VERSION_FULL_ART_ID];
    case "Legendary":
      return [VERSION_FULL_ART_ID, AUGMENT_ANIMATED_ID];
    case "Mythic":
      return [VERSION_FULL_ART_ID, AUGMENT_ANIMATED_ID, AUGMENT_SIGNATURE_ID];
    default:
      return [];
  }
}

/** The look a retired full art or a version names on its own, before any augment: null for an id that names neither. */
function lookOfId(id: string): CardLook | null {
  if (id.startsWith(VERSION_ID_PREFIX)) return isOfferedId(id) ? { version: id, animated: false, signed: false } : null;
  const implied = impliedBy(id);
  if (implied.length === 0) return null;
  return {
    version: implied.includes(VERSION_FULL_ART_ID) ? VERSION_FULL_ART_ID : null,
    animated: implied.includes(AUGMENT_ANIMATED_ID),
    signed: implied.includes(AUGMENT_SIGNATURE_ID),
  };
}

/** The look `fullArt` names on its own, before any augment: the normal face where it names none. */
function baseLookOf(fullArt: boolean | string | null | undefined): CardLook {
  const base = fullArt === true ? { version: VERSION_FULL_ART_ID, animated: false, signed: false } : typeof fullArt === "string" ? lookOfId(fullArt) : null;
  return base ?? NORMAL_LOOK;
}

/**
 * R2949000: whether a pick of `fullArt` and `augments` wears the Full art version and Animated, as Signature needs, as
 * the server's `cosmetics::signature_fits` reads it: `fullArt` the version, `true` or a retired full art that implies
 * it, and Animated listed or implied by it. Reads no ownership and no offer.
 */
export function signatureFits(fullArt: boolean | string | null | undefined, augments: readonly string[] | undefined = []): boolean {
  const base = baseLookOf(fullArt);
  return base.version === VERSION_FULL_ART_ID && (base.animated || augments.includes(AUGMENT_ANIMATED_ID));
}

/**
 * R2725000, R2725002: the look a card's pick names: `fullArt` false or absent is the normal face, `true` the Full art
 * version (a full-art card's still Epic), an id the version or retired full art it names, which a Legendary makes
 * animated and a Mythic animated and signed, as they printed before #2725; then each of `augments` layers on it. An
 * id this client does not know names the normal face, and an augment it does not know adds nothing. R2949000:
 * Signature layers only where the pick fits it (`signatureFits`).
 */
export function lookOf(fullArt: boolean | string | null | undefined, augments: readonly string[] | undefined = []): CardLook {
  const look = baseLookOf(fullArt);
  const animated = look.animated || augments.includes(AUGMENT_ANIMATED_ID);
  const signed = look.signed || (augments.includes(AUGMENT_SIGNATURE_ID) && signatureFits(fullArt, augments));
  return animated === look.animated && signed === look.signed ? look : { ...look, animated, signed };
}

/** R2725000: the augment ids a look layers on its version, in the registry's order: what a card pick's `augments` holds. */
export function augmentsOf(look: CardLook): string[] {
  return [look.animated && AUGMENT_ANIMATED_ID, look.signed && AUGMENT_SIGNATURE_ID].filter((id): id is string => typeof id === "string");
}

/**
 * R2725000: the `fullArt` a write keeps when it rewrites only a card's augments: null (no pick, the setting's) where
 * the card has none, `true` and `false` as they are, a retired full art's id as the version its look wears (its
 * augments go to `augmentPicksOf`), and a version's id, or an id this client does not know, as written.
 */
export function versionPickOf(fullArt: boolean | string | undefined): boolean | string | null {
  if (fullArt === undefined) return null;
  if (typeof fullArt === "boolean" || fullArt.startsWith(VERSION_ID_PREFIX)) return fullArt;
  const look = lookOfId(fullArt);
  return look === null ? fullArt : (look.version ?? false);
}

/**
 * R2725000: the augment ids a card's pick layers on its version, those a retired full art's pick combines among them:
 * what a rewrite of the pick keeps (`versionPickOf`), an id this client does not know too.
 */
export function augmentPicksOf(fullArt: boolean | string | undefined, augments: readonly string[] = []): string[] {
  const out = typeof fullArt === "string" ? augmentsOf(lookOf(fullArt)) : [];
  for (const id of augments) if (!out.includes(id)) out.push(id);
  return out;
}
