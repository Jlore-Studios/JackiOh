// Inspect timings and preview geometry (docs/polish/6-cards.md, Surface C). CLAUDE.md rule 9: every
// number the inspect overlays use is named here, never inline.

/** A mouse or pen pointer resting this long on a face-up card opens the hover preview (B22). */
export const HOVER_DELAY_MS = 300;
/** A touch held this long opens the inspect sheet, or calls `onLongPress` (B24). */
export const LONG_PRESS_MS = 450;
/** A touch that moves further than this from where it started is a drag, not a long-press. */
export const LONG_PRESS_SLOP_PX = 10;
/** After a long-press fires, the next click on the trigger is swallowed for at most this long. */
export const CLICK_SUPPRESS_MS = 600;
/**
 * The hover preview's card height, at the least. inspect.css caps it at 80vh. #3341: 500, up from 380, the most that
 * still fits above a hand card on a 1280x720 screen after the card lifts (placement.ts tries "above" first for a hand),
 * where a taller one would cover its neighbours from beside them; a taller screen draws it taller still
 * (PREVIEW_HEIGHT_SHARE, `previewHeight` in placement.ts).
 */
export const PREVIEW_HEIGHT_PX = 500;
/**
 * #3341: the share of the viewport's height the preview card takes when that is more than PREVIEW_HEIGHT_PX: a board
 * face grows with its screen, and at this share a desktop's preview art stays over twice as wide as a unit's whole face
 * on it (previewSize.test.ts).
 */
export const PREVIEW_HEIGHT_SHARE = 0.56;
/** Space between the anchor card and the preview. */
export const PREVIEW_GAP_PX = 12;
/** The preview always stays this far inside the viewport. */
export const PREVIEW_MARGIN_PX = 8;

/** #3341: the phone sheet's card height, at most (`.inspect-face--sheet` in inspect.css, which mirrors it). */
export const PREVIEW_SHEET_HEIGHT_PX = 480;
/** #3341: the share of the viewport's height the phone sheet's card may use. Mirrors inspect.css's `60dvh`. */
export const PREVIEW_SHEET_SHARE = 0.6;

/** The share of the viewport height the preview card may use. Mirrors inspect.css's `80vh`. */
export const PREVIEW_MAX_VIEWPORT_SHARE = 0.8;
/** The glossary column beside the preview card. Mirrors `.inspect-hover .inspect-glossary`'s width. */
export const PREVIEW_GLOSSARY_WIDTH_PX = 240;
/** Issue #1017: the extended description on the preview card's left. Mirrors `.inspect-hover .inspect-extended`. */
export const PREVIEW_EXTENDED_WIDTH_PX = 280;
/** Space between the preview card and its glossary. Mirrors `.inspect-hover`'s `gap`. */
export const PREVIEW_GLOSSARY_GAP_PX = 10;
/**
 * R279: the preview's column of the cards a face's text names, one face under another at
 * REF_PANEL_FACE_HEIGHT_PX, with the panel's padding. Mirrors `.inspect-hover .inspect-refs`.
 */
export const PREVIEW_REFS_WIDTH_PX = 170;
/**
 * #3602: viewports this wide or narrower draw the phone hover-preview layout, with the full-text
 * panel stacked across the card's width instead of beside it. Mirrors inspect.css's
 * `@media (max-width: 600px)` hover block.
 */
export const PREVIEW_PHONE_WIDTH_PX = 600;
/**
 * #3602: the stacked full-text panel's max height on a phone. Longer words scroll inside it; the
 * estimate counts it whole. 200 keeps card, one gap and panel inside 390x844 (500 + 10 + 200 = 710)
 * and 430x932. Mirrors `.inspect-hover .inspect-extended`'s `max-height` in inspect.css's phone block.
 */
export const PREVIEW_EXTENDED_STACKED_MAX_PX = 200;

/** A pile's hover preview (CardList.tsx) shows at most this many faces, newest first; the sheet shows them all. */
export const LIST_PREVIEW_MAX = 8;
/** Faces per row in a pile's hover preview. Mirrors `.inspect-list-hover .inspect-list-grid`. */
export const LIST_PREVIEW_COLUMNS = 4;
/** The height of one face in a pile's hover preview. Mirrors `--list-face-h` in inspect.css. */
export const LIST_PREVIEW_FACE_HEIGHT_PX = 150;
/** Space between faces, and the panel's padding, in a pile's hover preview. Mirrors inspect.css. */
export const LIST_PREVIEW_GAP_PX = 8;
export const LIST_PREVIEW_PADDING_PX = 10;
/** The header row (title and count) and the "more" line of a pile's hover preview. */
export const LIST_PREVIEW_HEADER_PX = 26;

/** R370: the height of the back a face-down backrow card's hover preview draws (FaceDown.tsx). */
export const FACE_DOWN_PREVIEW_HEIGHT_PX = 220;

/** Issue #2121 (R2121000): a sheet let go further down than this share of its height closes. */
export const SHEET_DISMISS_DISTANCE = 0.25;
/** A sheet let go moving down faster than this, in px per ms, closes however far it went. */
export const SHEET_DISMISS_VELOCITY = 0.5;
/** A touch on a sheet moving further than this is a drag, a scroll or nothing; until then, a tap. */
export const SHEET_DRAG_SLOP_PX = 10;
/** A lift this long after the last move is a finger that stopped, not a flick. */
export const SHEET_FLICK_WINDOW_MS = 100;
