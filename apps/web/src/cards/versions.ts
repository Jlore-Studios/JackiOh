// R2725000 (#2725): a card wears one version, its normal face or a `version-…` entry (the Full art version first),
// and the augments layered on it (`augment-…`: Animated, Signature), each owned card by card like a foil. R2725001:
// each is offered on the cards `COSMETIC_OFFERS` lists, the server's own reading of the entry's offer, and on no
// other. R2725002: a retired full art (R2655000, R2715000) counts as owning what it combines (`impliedBy`), and a
// card pick written before #2725 looks as it did (`lookOf`). Presentation only: nothing here reaches the engine,
// an action, `PlayerView` or the log (CLAUDE.md rule 7).

import { COSMETICS, COSMETIC_OFFERS } from "../wire/generated/cosmetics.ts";

/** R2715000: what an Animated art's registry id starts with, a `fullArt` entry too (migration 0075). */
export const ANIMATED_ART_ID_PREFIX = "animated-";

/** R2725000: what a version's registry id starts with, a card pick's `fullArt` text too (migration 0077). */
export const VERSION_ID_PREFIX = "version-";

/** R2725000: what an augment's registry id starts with, each one a card pick's `augments` lists. */
export const AUGMENT_ID_PREFIX = "augment-";

/** R2725000: the Full art version, Legendary: the card's art across its whole face. */
export const VERSION_FULL_ART_ID = "version-full-art";

/** R2725000: the Animated augment, Epic: the art's own elements move (R2715001) on whichever version is worn. */
export const AUGMENT_ANIMATED_ID = "augment-animated";

/** R2725000: the Signature augment, Mythic: the card's name signed across its art. */
export const AUGMENT_SIGNATURE_ID = "augment-signature";

/** R2725000: what a face wears: its version (null for the normal face) and the augments layered on it. */
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

/**
 * R2725000, R2725002: the look a card's pick names: `fullArt` false or absent is the normal face, `true` the Full art
 * version (a full-art card's still Epic), an id the version or retired full art it names, which a Legendary makes
 * animated and a Mythic animated and signed, as they printed before #2725; then each of `augments` layers on it. An
 * id this client does not know names the normal face, and an augment it does not know adds nothing.
 */
export function lookOf(fullArt: boolean | string | null | undefined, augments: readonly string[] | undefined = []): CardLook {
  const base = fullArt === true ? { version: VERSION_FULL_ART_ID, animated: false, signed: false } : typeof fullArt === "string" ? lookOfId(fullArt) : null;
  const look = base ?? NORMAL_LOOK;
  const animated = look.animated || augments.includes(AUGMENT_ANIMATED_ID);
  const signed = look.signed || augments.includes(AUGMENT_SIGNATURE_ID);
  return animated === look.animated && signed === look.signed ? look : { ...look, animated, signed };
}

/** R2725000: the augment ids a look layers on its version, in the registry's order: what a card pick's `augments` holds. */
export function augmentsOf(look: CardLook): string[] {
  return [look.animated && AUGMENT_ANIMATED_ID, look.signed && AUGMENT_SIGNATURE_ID].filter((id): id is string => typeof id === "string");
}

/** Whether two looks wear the same version and augments. */
export function sameLook(a: CardLook, b: CardLook): boolean {
  return a.version === b.version && a.animated === b.animated && a.signed === b.signed;
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
