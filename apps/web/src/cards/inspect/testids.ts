// Test ids of the inspect overlays. Every id starts with `inspect-`, never `card-` or `hand-card-`,
// so `cy.fieldCardByName` and `cy.handCardByName` cannot resolve to an overlay (B20).
// e2e/support/testids.ts block A14 mirrors these name for name.

export const INSPECT_HOVER = "inspect-hover";
export const INSPECT_SHEET = "inspect-sheet";
export const INSPECT_DETAIL = "inspect-detail";
export const INSPECT_SCRIM = "inspect-scrim";
export const INSPECT_CLOSE = "inspect-close";
export const INSPECT_FACE = "inspect-face";
export const INSPECT_FACE_BASE = "inspect-face-base";
export const INSPECT_FACE_RADIANT = "inspect-face-radiant";
export const INSPECT_GLOSSARY = "inspect-glossary";
/** A face in play's printed text, where the two differ (SPEC §10.10). */
export const INSPECT_PRINTED = "inspect-printed";
/** R279: the hover preview's column of the cards a face's text names. */
export const INSPECT_REFS = "inspect-refs";
/** R660: a card's flavour line and artist credit, in the preview, the sheet and the detail view. */
export const INSPECT_FLAVOUR = "inspect-flavour";
/** R660: the artist credit inside it, where the sidecar names one. */
export const INSPECT_ARTIST = "inspect-artist";

// A list of cards (a graveyard or an exile pile): the hover preview, the sheet, and inside them.
export const INSPECT_LIST_HOVER = "inspect-list-hover";
export const INSPECT_LIST_SHEET = "inspect-list-sheet";
export const INSPECT_LIST_COUNT = "inspect-list-count";
export const INSPECT_LIST_CARD = "inspect-list-card";
export const INSPECT_LIST_MORE = "inspect-list-more";
export const INSPECT_LIST_DETAIL = "inspect-list-detail";
export const INSPECT_LIST_BACK = "inspect-list-back";

// R370, R371: a face-down backrow card's overlay (FaceDown.tsx), the cost it states, and the note a
// preview or a sheet of your own face-down trap carries.
export const INSPECT_FACE_DOWN = "inspect-face-down";
export const INSPECT_FACE_DOWN_COST = "inspect-face-down-cost";
export const INSPECT_NOTE = "inspect-note";

// Patch v0.2.0 (SPEC §10.8): a face in play's states spelled out beside it (StateNotes.tsx) — the
// tuned ribbon (R386), the list of the other states (Brittle R385, enchantments E39, animated R383) —
// and the lines-of-code meta line (E36).
export const INSPECT_TUNED = "inspect-tuned";
export const INSPECT_STATES = "inspect-states";
export const INSPECT_LOC = "inspect-loc";
export const INSPECT_STATS = "inspect-stats";

