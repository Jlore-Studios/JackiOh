// Test ids of the Patch notes page and the History section (R388, R507, R724000). Every id starts
// with `patch-`, never `card-` or `hand-card-`, so no board selector can resolve to one of them.

export const patchTestid = {
  /** The page's root (routes/patch-notes.tsx). */
  screen: "patch-notes-screen",
  loading: "patch-notes-loading",
  error: "patch-notes-error",
  retry: "patch-notes-retry",
  empty: "patch-notes-empty",
  /** One release line, `data-set` its id (R724000). */
  set: "patch-set",
  setToggle: "patch-set-toggle",
  technical: "patch-technical",
  /** A patch's lines from the technical log (R1369000). */
  technicalLog: "patch-technical-log",
  /** One patch, `data-version` its version. */
  patch: "patch-entry",
  /** A patch's player note (R724001). */
  note: "patch-note",
  /** One heading of a player note, `data-section` its key. */
  noteSection: "patch-note-section",
  /** A patch's Show / Hide the cards control. */
  toggle: "patch-toggle",
  cards: "patch-cards",
  cardsLoading: "patch-cards-loading",
  cardsError: "patch-cards-error",
  cardsRetry: "patch-cards-retry",
  filter: "patch-filter",
  noMatch: "patch-no-match",
  /** A changed card shown as a face, `data-card` its id. */
  changedCard: "patch-changed-card",
  /** A changed card whose changes a face does not print, `data-card` its id. */
  dataOnly: "patch-data-only",
  /** A set's added cards, `data-group` its label. */
  addedGroup: "patch-added-group",
  /** A removed card, `data-card` its id. */
  removed: "patch-removed",
  /** Any control that opens a card's history, `data-card` its id. */
  openCard: "patch-open-card",
  /** #2052: the dialog in the detail's place while a card opened from a patch loads, or after it failed. */
  opening: "patch-opening",
  openingLoading: "patch-opening-loading",
  openingError: "patch-opening-error",
  openingRetry: "patch-opening-retry",
  openingClose: "patch-opening-close",
  /** One changed field, `data-field` the field. */
  change: "patch-change",
  /** The History section in the card detail view. */
  history: "patch-history",
  historyToggle: "patch-history-toggle",
  historyLoading: "patch-history-loading",
  historyError: "patch-history-error",
  historyRetry: "patch-history-retry",
  historyEmpty: "patch-history-empty",
  historyUnchanged: "patch-history-unchanged",
  /** One patch in a card's history, `data-version` and `data-kind`. */
  historyEntry: "patch-history-entry",
} as const;
