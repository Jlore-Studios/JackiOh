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
 * Every catalog face fits it but the short forms (jackioh-shared's short-forms.json). `faceLines` counts by words.
 */
export const FACE_TEXT_LIMIT = { charsPerLine: 28, lines: 6 } as const;

/**
 * The same for the hover preview's face (271 px wide, 380 px tall, as it was measured: #3341 draws it larger, which
 * only gives its lines room). A face in play prints only there, so a Summit's next Ascent line and a fused or Buffed
 * card are held to it.
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
 * R2720000: the most a finish covers a full-art face's art: its band round the art's edge has this
 * opacity at the rim and fades to nothing toward the middle. Mirrored in cosmetics.css.
 */
export const FULL_ART_FINISH_OPACITY = 0.6;

/**
 * R2942001: the most the Extended art version's translucent frame covers the art it runs over: the panel lies over the
 * picture at this opacity at the rim and fades to nothing toward the middle, as R2720000's band. Mirrored in cards.css.
 */
export const EXTENDED_ART_FRAME_OPACITY = 0.55;

/**
 * R2942001: the share, in percent, of a base Extended art face's rules box that is its parchment, the rest the art
 * under it: enough that over the darkest art, the watermark on it, every ink reads as well as over the normal box's
 * edge (R2718000). Mirrored in cards.css.
 */
export const EXTENDED_ART_BOX_PCT = 90;

/**
 * R2942001: the side of the Extended art version's square picture, in percent of the face's height. Standing on the
 * face's foot, its middle sits a quarter of the way down, where the normal window's does. Mirrored in cards.css.
 */
export const EXTENDED_ART_PICTURE_PCT = 150;

/**
 * R2943001: the Out of frame version's window, inset inside the art frame's box this far from its top and from each
 * side, in percent of the box, its foot kept: the frame's own colour shows round it, where the subject steps out.
 * Mirrored in cards.css.
 */
export const OUT_OF_FRAME_WINDOW_TOP_PCT = 30;
export const OUT_OF_FRAME_WINDOW_SIDE_PCT = 18;

/**
 * R2943001: the side of the Out of frame version's cut-out subject, a square, in percent of the art frame's height, and
 * how far its foot sinks below the frame's, under the name banner: it rises above the window and is wider than it, and
 * stays inside the card's face (a Unit's frame sits 3.5cqh down its face, 40cqh tall, so the square's 46cqh rises to
 * 1.5cqh). Mirrored in cards.css.
 */
export const OUT_OF_FRAME_SUBJECT_PCT = 115;
export const OUT_OF_FRAME_SUBJECT_SINK_PCT = 10;

/**
 * R2944002: how far the Immersive version's near layer shifts from its rest at the pointer's or the tilt's far edge, in
 * percent of the window, each way. Mirrored in cards.css.
 */
export const IMMERSIVE_SHIFT_PCT = 6;

/**
 * R2944002: how long the Immersive version's near layer takes to swell, and again to settle, in seconds: a slow breath.
 * Mirrored in cards.css.
 */
export const IMMERSIVE_BREATH_S = 7;

/** R2944002: how far the Immersive version's near layer swells at the top of a breath, its scale. Mirrored in cards.css. */
export const IMMERSIVE_BREATH_SCALE = 1.04;

/**
 * R2944002: how far a device tilts, in degrees, from its rest to the Immersive version's far edge: left and right
 * (`gamma`) from flat, and forward and back (`beta`) from IMMERSIVE_TILT_REST_DEG, the way a phone is held to be read.
 */
export const IMMERSIVE_TILT_RANGE_DEG = 30;
export const IMMERSIVE_TILT_REST_DEG = 45;

/**
 * R3344001: how far a face with depth tilts toward the pointer, the device or a drag at its far edge, in degrees, each
 * way. tilt.ts writes the angle (`--cf-tilt-rx`, `--cf-tilt-ry`), which depth.css reads as it is.
 */
export const CARD_TILT_MAX_DEG = 12;

/**
 * R3344001: how far each depth plane (`--cf-plane`, 1 for the middle, 3 for the nearest) shifts against the face per
 * tilt step, in px, at full tilt on a face CARD_DEPTH_FACE_PX wide: the planes parallax by their depth. CardFace writes
 * it as `--cf-depth-step`, in cqw so a smaller face shifts less.
 */
export const CARD_DEPTH_STEP_PX = 3;

/** R3344001: the face width, in px, that CARD_DEPTH_STEP_PX is measured at (a hand card on a phone). */
export const CARD_DEPTH_FACE_PX = 170;

/** R3344001: CARD_DEPTH_STEP_PX as a share of the face's width, in percent: what `--cf-depth-step` is written in, as cqw. */
export const CARD_DEPTH_STEP_CQW = (CARD_DEPTH_STEP_PX / CARD_DEPTH_FACE_PX) * 100;

/**
 * R3344000: how far a Legendary or Mythic face's near plane (its subject) may rise over the face's top edge, in px,
 * clipped by the face's own edge as Out of frame's subject is. CardFace writes it as `--cf-frame-break`.
 */
export const CARD_FRAME_BREAK_PX = 8;

/** R3344000: how far that near plane is scaled up from its foot, so its subject stands taller than its window. */
export const CARD_FRAME_BREAK_SCALE = 1.12;

/**
 * R3344001: the perspective the face's parent gives a tilting face, in px. Mirrored in depth.css's `perspective`
 * (depth.test.tsx holds the two equal).
 */
export const CARD_PERSPECTIVE_PX = 900;

/**
 * R3344000: how far a glare travels from the middle of its layer at full tilt, in percent of the layer, each way; it
 * moves opposite the tilt (tilt.ts `glareOf`).
 */
export const CARD_GLARE_TRAVEL_PCT = 60;

/** R3344000: how far a glare's gradient reaches before it is clear, in percent of its layer. CardFace writes it as `--cf-glare-size`. */
export const CARD_GLARE_SIZE_PCT = 55;

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

/** R2801000: the most hull plates a Dormant Starship's face bolts on, one a piece (StarshipHull.tsx). */
export const STARSHIP_HULL_PLATES_MAX = 8;

/**
 * #3341: how much a small face's art (below FACE_TEXT_MIN_HEIGHT_PX, a card as small as the board draws it) is lifted
 * so a picture holds its colour at that size: `filter: saturate() contrast()`, a factor each, 1 being none. Never on a
 * Radiant face's gold window (R2725004). Mirrored in cards.css's @container rule.
 */
export const ART_SMALL_FACE_LIFT = { saturate: 1.15, contrast: 1.06 } as const;

/**
 * #3341: how far a normal board unit's art runs down its face, as a share of one gem's height (`--mf-stat`) kept
 * clear of the foot: the art stops half a gem above the face's bottom edge, so the attack and health gems straddle its
 * edge and the name plate lies on it. Mirrored in cards.css (`--mf-art-foot`).
 */
export const MINION_ART_FOOT_SHARE = 0.5;

/**
 * #3341: how far a small hand face's art window grows downward (in cqh, percent of the face's height), the name
 * band and the rest following it down by as much (below FACE_TEXT_MIN_HEIGHT_PX the rules box is hidden, so the
 * band gives up the room). Mirrored in cards.css's @container rule.
 */
export const HAND_ART_GROW_CQH = 4;

/**
 * #3341: the least a hand face's rules box shrinks to, in cqh (percent of the face's height), however short its text
 * (fit.ts measures the text and cards.css gives the room it does not need to the art above). Two lines at the
 * reading floor fit it.
 */
export const HAND_TEXT_BOX_MIN_CQH = 12;
