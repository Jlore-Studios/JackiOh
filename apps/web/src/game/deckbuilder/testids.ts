// The deck workshop's `data-testid` vocabulary, in one file (SPEC §9.4, R250–R256).
//
// BUILD M5-T1 fixes the board's testids and `e2e/support/testids.ts` mirrors them; this file is the
// builder's half of that contract. One deck is open at a time, so its elements carry no number.
//
// `data-legal="false"` is not a new word: `e2e/support/testids.ts` exports it as `ILLEGAL` for the
// board's M5-T2 highlighting, and a pool card the open deck will not take is the same statement.

import type { CardType, Rarity, Tag } from "@jackioh/shared";
import type { LoadoutRule } from "@jackioh/validator";

import type { CostBucket } from "./filters.ts";

// The workshop: the screen, the save status, the rail of decks and trios

/** The workshop's root: the whole screen, whether a deck, a trio, the import or nothing is open.
 *  `data-view="list|editor"` says which half a phone shows. */
export const WORKSHOP = "workshop";

/** The save status line, always on screen: `data-state="saved|saving|offline|error"` (R256). */
export const SYNC_STATUS = "sync-status";

export const WORKSHOP_BACK = "workshop-back";

export const WORKSHOP_EMPTY = "workshop-empty";

export const DECK_LIST = "deck-list";
export const DECK_CAP = "deck-cap";

/**
 * One saved deck in the list: `data-count` (its cards), `data-status` (`ready`, `complete`,
 * `incomplete`, `unowned` or `invalid`, the chip it wears), `data-unsynced="true"` while a save is
 * pending, and `aria-current="true"` when it is open.
 */
export function deckRowId(deckId: string): string {
  return `deck-row-${deckId}`;
}

export const DECK_NEW = "deck-new";
export const DECK_CAP_REASON = "deck-cap-reason";

export const TRIO_LIST = "trio-list";
export const TRIO_CAP = "trio-cap";
export const TRIO_NEW = "trio-new";
export const TRIO_CAP_REASON = "trio-cap-reason";

/** One saved trio in the list: `data-ready="true|false"` (R253's Conquest verdict). */
export function trioRowId(trioId: string): string {
  return `trio-row-${trioId}`;
}

// The deck editor

export const DECK_EDITOR = "deck-editor";

/** The open deck's name (D1 met gently: an emptied name saves as "Untitled deck"). */
export const DECK_NAME_INPUT = "deck-name-input";

export const DECK_COUNT = "deck-count";

export const DECK_DROP = "deck-drop";

export const DECK_CARDS = "deck-cards";

/**
 * One card in the open deck: a tile whose click takes it out. `data-conflict="true"` and
 * `data-conflict-with="<deck name>"` when a compared deck holds it too (shown, never removed).
 */
export function deckCardId(cardId: string): string {
  return `deck-card-${cardId}`;
}

export const DECK_CURVE = "deck-curve";

export const DECK_FOLD = "deck-fold";

export const DECK_STATUS = "deck-status";

/** The server's refusal of this deck's last save, verbatim (R256). */
export const DECK_SAVE_ERROR = "deck-save-error";

/** Copies the deck's code (R255), and shows it in `deck-code-output`, a read-only field. */
export const DECK_COPY_CODE = "deck-copy-code";
export const DECK_CODE_OUTPUT = "deck-code-output";

export const DECK_DELETE = "deck-delete";
export const DECK_DELETE_CONFIRM = "deck-delete-confirm";
export const DECK_DELETE_CANCEL = "deck-delete-cancel";

/**
 * "Compare with" (R251): a select whose options are `trio:<trioId>` (a trio this deck is in: its
 * other decks), `deck:<deckId>` (another deck, up to two at once) and `none`. Each compared deck is
 * then a chip, `deck-compare-<deckId>`, whose press stops comparing with it.
 */
export const DECK_COMPARE_SELECT = "deck-compare-select";

export function deckCompareChipId(deckId: string): string {
  return `deck-compare-${deckId}`;
}

export const DECK_CONFLICTS = "deck-conflicts";

export const DECK_VERDICT = "deck-verdict";

/** The list every L1–L6 sentence is rendered into, in a deck's verdict and in a trio's. */
export const LOADOUT_ERRORS = "loadout-errors";

/**
 * One marker per failure, carrying the validator's sentence and nothing else. Several elements
 * may share a testid (the validator reports every failure), which is why each also carries
 * `data-rule` and, where the validator named them, `data-deck` and `data-card`.
 */
export function loadoutErrorId(rule: LoadoutRule): string {
  return `loadout-error-${rule}`;
}

export const DECKBUILDER_LOADING = "deckbuilder-loading";

export const DECKBUILDER_ERROR = "deckbuilder-error";

/**
 * The MIME the pool puts a catalog id on when a drag starts, alongside a `text/plain` copy.
 * `e2e/support/testids.ts` `DECK_DRAG_MIME` is the same string. A drop reads both, and prefers the
 * id the component recorded on `dragstart`, because Cypress and jsdom synthesise drag events
 * without a `DataTransfer` at all.
 */
export const DECK_DRAG_MIME = "application/x-jackioh-card";

// The trio editor

export const TRIO_EDITOR = "trio-editor";

/** The open trio's name (T1 met gently: an emptied name saves as "Untitled trio"). */
export const TRIO_NAME_INPUT = "trio-name-input";

export function trioSlotId(slot: number): string {
  return `trio-slot-${String(slot)}`;
}

export function trioOpenDeckId(slot: number): string {
  return `trio-open-${String(slot)}`;
}

/** R253's Conquest verdict (`data-ready`): "Ready for Conquest", or `loadout-errors`. */
export const TRIO_VERDICT = "trio-verdict";

export const TRIO_COMPARE = "trio-compare";

/**
 * Card `cardId` in slot `n`'s column (1-based): `data-conflict="true|false"`, and, for a card
 * another slot's deck holds too, `data-conflict-with="<deck name>"` (names joined with ", ").
 */
export function trioCardId(slot: number, cardId: string): string {
  return `trio-card-${String(slot)}-${cardId}`;
}

export const TRIO_DELETE = "trio-delete";
export const TRIO_DELETE_CONFIRM = "trio-delete-confirm";
export const TRIO_DELETE_CANCEL = "trio-delete-cancel";

/** R339: "Copy trio code", and the read-only field the code is shown in once copied. */
export const TRIO_COPY_CODE = "trio-copy-code";
export const TRIO_CODE_OUTPUT = "trio-code-output";

// Import (R255)

export const DECK_IMPORT_OPEN = "deck-import-open";

export const DECK_IMPORT = "deck-import";

export const DECK_IMPORT_INPUT = "deck-import-input";

/**
 * The live read of the pasted code, `data-ok="true|false"`: the name, the count, what was left
 * out and which cards are not owned, or the reason the code cannot be read.
 */
export const DECK_IMPORT_PREVIEW = "deck-import-preview";

/** "Import as new deck": off until the code reads, and at the deck cap (`deck-import-cap-reason`
 *  says why). */
export const DECK_IMPORT_SUBMIT = "deck-import-submit";
export const DECK_IMPORT_CAP_REASON = "deck-import-cap-reason";
export const DECK_IMPORT_CANCEL = "deck-import-cancel";

// Import a trio (R339–R341)

export const TRIO_IMPORT_OPEN = "trio-import-open";

export const TRIO_IMPORT = "trio-import";

export const TRIO_IMPORT_INPUT = "trio-import-input";

/**
 * The live read of the pasted trio code, `data-ok="true|false"`: the trio's name, each slot's deck
 * (`trio-import-slot-<n>`), the cards the decks share (`trio-import-shared`), or the reason the
 * code cannot be read.
 */
export const TRIO_IMPORT_PREVIEW = "trio-import-preview";

export function trioImportSlotId(slot: number): string {
  return `trio-import-slot-${String(slot)}`;
}

export const TRIO_IMPORT_SHARED = "trio-import-shared";

/**
 * "Import as new trio": off until the code reads, and while the caps leave too little room, with
 * `trio-import-cap-reason` saying exactly how many deck and trio slots it needs (R340;
 * `data-decks-short`, `data-trios-short`).
 */
export const TRIO_IMPORT_SUBMIT = "trio-import-submit";
export const TRIO_IMPORT_CAP_REASON = "trio-import-cap-reason";
export const TRIO_IMPORT_CANCEL = "trio-import-cancel";

export const TRIO_IMPORT_ERROR = "trio-import-error";

// Browse: filters, sort, the pool grid and its inspect control (docs/polish/6-cards.md, Surface D).
// `e2e/support/testids.ts` block A14 mirrors these name for name.
//
// No new name starts with `card-` or `hand-card-`: `cy.fieldCardByName` and `cy.handCardByName`
// select on those prefixes, and a new id carrying one would hijack them.

/** Lower-case, every run of characters outside `[a-z0-9]` becomes one "-", trimmed of "-". */
export function slugOf(value: string): string {
  return value
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "");
}

export const DB_FILTERS = "db-filters";

export const DB_SEARCH = "db-search";

export function filterCostId(bucket: CostBucket): string {
  return `db-filter-cost-${bucket}`;
}

export function filterTypeId(type: CardType): string {
  return `db-filter-type-${slugOf(type)}`;
}

export function filterTagId(tag: Tag): string {
  return `db-filter-tag-${slugOf(tag)}`;
}

export function filterSetId(set: string): string {
  return `db-filter-set-${slugOf(set.replace(/\+/g, " plus"))}`;
}

export function filterRarityId(rarity: Rarity): string {
  return `db-filter-rarity-${slugOf(rarity)}`;
}

export const DB_FILTER_OWNED = "db-filter-owned";

export const DB_FILTER_CLEAR = "db-filter-clear";
export const DB_FILTER_TOGGLE = "db-filter-toggle";

export const DB_SORT = "db-sort";

export const DB_SORT_DIR = "db-sort-dir";

export const DB_RESULT_COUNT = "db-result-count";

export const DB_EMPTY = "db-empty";

/** The card pool, and one entry per card in it: `card-pool-<id>` carries `data-legal`,
 *  `data-in-deck="true"` when the open deck holds it, and `data-unavailable="true"` with
 *  `data-held-by="<deck name>"` when a compared deck does (R251). */
export const CARD_POOL = "card-pool";

export function poolCardId(cardId: string): string {
  return `${CARD_POOL}-${cardId}`;
}

export function addPoolId(cardId: string): string {
  return `db-add-${cardId}`;
}

export const DB_DETAIL_ADD = "db-detail-add";

export const DB_SIDEBAR = "db-sidebar";

