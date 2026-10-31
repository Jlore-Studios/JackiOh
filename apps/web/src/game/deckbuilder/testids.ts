// The deck workshop's `data-testid` vocabulary (SPEC §9.4, R250–R256).
// Builder half of the e2e/support/testids.ts contract.

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

/** Saved deck in list: count, status, practice-only (R915005), unsynced, aria-current. */
export function deckRowId(deckId: string): string {
  return `deck-row-${deckId}`;
}

/** R915005: "Practice only" on deck row and verdict when custom card is present. */
export const DECK_PRACTICE_ONLY = "deck-practice-only";

/** R2097000 badge on deck row and editor. */
export const DECK_PATCH_BADGE = "deck-patch-badge";

/** R2097000's list of the cards the newest patch changed of a deck. */
export const DECK_PATCH_CARDS = "deck-patch-cards";

/** R2097000: one card the newest patch changed of a deck, `data-card` its id. */
export function deckPatchCardId(cardId: string): string {
  return `deck-patch-card-${cardId}`;
}

/** #920: beside the set chips, the way to the card creator (`paths.creator`). */
export const DECKS_OPEN_CREATOR = "decks-open-creator";

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

/** Card tile in open deck; data-conflict when compared deck holds it. */
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

/** R2119000: deletes the deck at once, behind the undo toast (`undo/UndoToast.tsx`). */
export const DECK_DELETE = "deck-delete";

/** #2096: Duplicate action and cap reason; per-row deck-duplicate-<deckId>. */
export const DECK_DUPLICATE = "deck-duplicate";
export const DECK_DUPLICATE_REASON = "deck-duplicate-reason";

export function deckDuplicateId(deckId: string): string {
  return `deck-duplicate-${deckId}`;
}

/** #1991: Practice with complete saved deck. */
export const DECK_TO_PRACTICE = "deck-to-practice";

/** "Compare with" (R251): select trio/deck and chip removals. */
export const DECK_COMPARE_SELECT = "deck-compare-select";

/** R1297000: deck cosmetics button in top bar (BUILD M5-T1). */
export const DECK_COSMETICS_BUTTON = "deck-cosmetics-button";

/** R2100000: open deck history button, panel, rows, and restore. */
export const DECK_HISTORY_BUTTON = "deck-history-button";
export const DECK_HISTORY_PANEL = "deck-history-panel";
export const DECK_HISTORY_ROW = "deck-history-row";

export function deckHistoryRestoreId(versionId: string): string {
  return `deck-history-restore-${versionId}`;
}

/** #2099: the open deck's Undo and Redo, in the top bar; each is off with nothing to move. */
export const DECK_UNDO = "deck-undo";
export const DECK_REDO = "deck-redo";

export function deckCompareChipId(deckId: string): string {
  return `deck-compare-${deckId}`;
}

export const DECK_CONFLICTS = "deck-conflicts";

export const DECK_VERDICT = "deck-verdict";

/** The list every L1–L6 sentence is rendered into, in a deck's verdict and in a trio's. */
export const LOADOUT_ERRORS = "loadout-errors";

/** One marker per validator failure; carries data-rule, data-deck, and data-card. */
export function loadoutErrorId(rule: LoadoutRule): string {
  return `loadout-error-${rule}`;
}

export const DECKBUILDER_LOADING = "deckbuilder-loading";

export const DECKBUILDER_ERROR = "deckbuilder-error";

/** The decks read's Try again, beside `DECKBUILDER_ERROR` (#1485). */
export const DECKBUILDER_RETRY = "deckbuilder-retry";

/** MIME for pool catalog id on drag start; keeps parity with e2e/support/testids.ts. */
export const DECK_DRAG_MIME = "application/x-jackioh-card";

// The trio editor

export const TRIO_EDITOR = "trio-editor";

/** The open trio's name (T1 met gently: an emptied name saves as "Untitled trio"). */
export const TRIO_NAME_INPUT = "trio-name-input";

/** Surfacing of the L4 shared-card rule before queue (SPEC §9.4, R250). */
export const TRIO_L4_HINT = "trio-l4-hint";

export function trioSlotId(slot: number): string {
  return `trio-slot-${String(slot)}`;
}

export function trioOpenDeckId(slot: number): string {
  return `trio-open-${String(slot)}`;
}

/** R253's Conquest verdict (`data-ready`): "Ready for Conquest", or `loadout-errors`. */
export const TRIO_VERDICT = "trio-verdict";

export const TRIO_COMPARE = "trio-compare";

/** Card in trio slot column with data-conflict status. */
export function trioCardId(slot: number, cardId: string): string {
  return `trio-card-${String(slot)}-${cardId}`;
}

/** R2119000: as `DECK_DELETE`, for the trio. */
export const TRIO_DELETE = "trio-delete";

/** R339: "Copy trio code", and the read-only field the code is shown in once copied. */
export const TRIO_COPY_CODE = "trio-copy-code";
export const TRIO_CODE_OUTPUT = "trio-code-output";

// Import (R255)

export const DECK_IMPORT_OPEN = "deck-import-open";

export const DECK_IMPORT = "deck-import";

export const DECK_IMPORT_INPUT = "deck-import-input";

/** Live preview of pasted deck code. */
export const DECK_IMPORT_PREVIEW = "deck-import-preview";

/** Import deck submit button. */
export const DECK_IMPORT_SUBMIT = "deck-import-submit";
export const DECK_IMPORT_CAP_REASON = "deck-import-cap-reason";
export const DECK_IMPORT_CANCEL = "deck-import-cancel";

// Import a trio (R339–R341)

export const TRIO_IMPORT_OPEN = "trio-import-open";

export const TRIO_IMPORT = "trio-import";

export const TRIO_IMPORT_INPUT = "trio-import-input";

/** Live preview of pasted trio code. */
export const TRIO_IMPORT_PREVIEW = "trio-import-preview";

export function trioImportSlotId(slot: number): string {
  return `trio-import-slot-${String(slot)}`;
}

export const TRIO_IMPORT_SHARED = "trio-import-shared";

/** R340: Import trio submit button. */
export const TRIO_IMPORT_SUBMIT = "trio-import-submit";
export const TRIO_IMPORT_CAP_REASON = "trio-import-cap-reason";
export const TRIO_IMPORT_CANCEL = "trio-import-cancel";

export const TRIO_IMPORT_ERROR = "trio-import-error";

// Browse: filters, sort, pool grid and inspect control. Prefixes avoid hijacking e2e selectors.

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

/** The win-rate sort's notice over the pool, `data-state="loading|failed"` (useCardWinRates.ts). */
export const DB_WINRATE_NOTICE = "db-winrate-notice";

/** The notice's retry, shown after a failed fetch. */
export const DB_WINRATE_RETRY = "db-winrate-retry";

/** Card pool container and entries (R251). */
export const CARD_POOL = "card-pool";

export function poolCardId(cardId: string): string {
  return `${CARD_POOL}-${cardId}`;
}

export function addPoolId(cardId: string): string {
  return `db-add-${cardId}`;
}

export const DB_DETAIL_ADD = "db-detail-add";

export const DB_SIDEBAR = "db-sidebar";

