// Face numbers (CLAUDE.md rule 9: named, never inline). cards.css mirrors the two that CSS has to
// know as literals: the 5:7 aspect ratio and the 149px container-query breakpoint.

import type { LengthTier } from "./fit.ts";

/** A full card is a tall 5:7 rectangle. */
export const FACE_ASPECT = 5 / 7;

/** Name length ceilings for each tier: `nameTier` returns the first tier whose ceiling holds. */
export const NAME_TIER_MAX = { s: 12, m: 18, l: 24, xl: 30 } as const;

/** Rules-text length ceilings (base text plus the radiant clause) for each tier. */
export const TEXT_TIER_MAX = { s: 40, m: 90, l: 160, xl: 260 } as const;

/** The font scale each tier starts from, before `useFitText` shrinks it further. */
export const TIER_SCALE: Readonly<Record<LengthTier, number>> = { s: 1, m: 0.92, l: 0.82, xl: 0.72, xxl: 0.62 };

/** The smallest `--cf-fit` factor `useFitText` tries before it clamps with an ellipsis. */
export const FIT_MIN = 0.55;

/** Binary-search steps between FIT_MIN and 1. */
export const FIT_STEPS = 6;

/**
 * The rules text's reading floor, in CSS pixels: text that would shrink below it takes the long
 * layout (`data-long`), then clamps with an ellipsis. The detail view and hover preview print it whole.
 */
export const FIT_FLOOR_PX = 9;

/**
 * Issue #1017: the most text a face may print, measured in Chromium at the reading floor on a 170 px
 * face (the Almanac's and deck builder's tiles are 164 to 220 px) before the art would give way.
 * Every catalog face fits it but the short forms (shortText.ts). `faceLines` counts by words.
 */
export const FACE_TEXT_LIMIT = { charsPerLine: 28, lines: 6 } as const;

/**
 * The same for the hover preview's face (271 px wide, PREVIEW_HEIGHT_PX tall). A face in play prints
 * only there, so a Summit's next Ascent line and a fused or Buffed card are held to it.
 */
export const PREVIEW_TEXT_LIMIT = { charsPerLine: 45, lines: 10 } as const;

/** Below this face height the rules box, tags and type line hide. Mirrored in cards.css's @container rule. */
export const FACE_TEXT_MIN_HEIGHT_PX = 150;

/** R279: a mouse or pen resting this long on a reference in a card's text opens the card it names. */
export const REF_HOVER_DELAY_MS = 250;

/** R2098000: a mouse or pen resting this long on a glossary term in a card's text shows its rule. */
export const TERM_HOVER_DELAY_MS = 250;

/** R279: the height of the card face a reference's tooltip shows. */
export const REF_TOOLTIP_HEIGHT_PX = 300;

/** R279: the height of each named card's face in a hover preview's references column. */
export const REF_PANEL_FACE_HEIGHT_PX = 190;

/**
 * The state rail (CardStates.tsx): the badges a small face (below FACE_TEXT_MIN_HEIGHT_PX) shows, the
 * rest folded into a "+n" chip. Mirrored in cardstate.css.
 */
export const STATE_BADGES_SMALL_MAX = 2;

/** A grid's lazy art (CardArt `lazy`) is drawn once its window is within this many px of its scrolling box. */
export const ART_NEAR_MARGIN_PX = 600;

/**
 * How long a lazy art window must stay near the screen before it draws (art/near.ts). A card
 * flicked straight past never dwells this long; the first screen waits it too, so it stays short.
 */
export const ART_DWELL_MS = 150;

/**
 * R1295000: the most faces whose cosmetic foil moves at once (cosmetics.ts); a face past it wears its
 * still version until a moving one leaves, so a full hand and board never animate a dozen sheens.
 */
export const COSMETIC_FOIL_ANIMATED_MAX = 6;

/**
 * R1295000: the Radiant gold, as a window in HSL that holds every color cards.css's Radiant face
 * declares. No border's colors may fall inside it (cosmetics.test.tsx), so no border passes for
 * a Radiant trim.
 */
export const RADIANT_GOLD_HUE_MIN_DEG = 38;
/** The Radiant gold window's upper hue (RADIANT_GOLD_HUE_MIN_DEG). */
export const RADIANT_GOLD_HUE_MAX_DEG = 56;
/** The Radiant gold window's least saturation, 0 to 1: a greyer color reads as no gold. */
export const RADIANT_GOLD_SATURATION_MIN = 0.5;
/** The Radiant gold window's darkest lightness, 0 to 1: below it a gold reads as brown. */
export const RADIANT_GOLD_LIGHTNESS_MIN = 0.3;
/** The Radiant gold window's lightest lightness, 0 to 1: above it a gold reads as cream. */
export const RADIANT_GOLD_LIGHTNESS_MAX = 0.9;

/**
 * #2435, R1839000: how far a Dormant card on the field is greyed and dimmed (the veil's grayscale() and
 * brightness(), dormant.css), well past the "Can't attack" portrait's (keywords.css).
 */
export const DORMANT_GREY = 0.92;
export const DORMANT_DIM = 0.6;

/** #2435: one beat of the Dormant seal's pulse, runes and crescent (dormant.css). */
export const DORMANT_PULSE_MS = 3200;
