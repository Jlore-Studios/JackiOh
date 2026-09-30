// Test ids of the Patch notes page and the History section (R388, R507). Every id starts with
// `patch-`, never `card-` or `hand-card-`, so no board selector can resolve to one of them.

export const patchTestid = {
  /** The page's root (routes/patch-notes.tsx). */
  screen: "patch-notes-screen",
  loading: "patch-notes-loading",
  error: "patch-notes-error",
  retry: "patch-notes-retry",
  /** "There are no patch notes yet." */
  empty: "patch-notes-empty",
  /** One patch, `data-version` its version. */
  patch: "patch-entry",
  /** A patch's Show / Hide the cards control. */
  toggle: "patch-toggle",
  /** A patch's cards, once shown. */
  cards: "patch-cards",
  cardsLoading: "patch-cards-loading",
  cardsError: "patch-cards-error",
  cardsRetry: "patch-cards-retry",
  /** The name filter over a patch's cards. */
  filter: "patch-filter",
  /** "No card in this patch matches …". */
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
  /** One changed field, `data-field` the field. */
  change: "patch-change",
  /** The History section in the card detail view. */
  history: "patch-history",
  historyToggle: "patch-history-toggle",
  historyLoading: "patch-history-loading",
  historyError: "patch-history-error",
  historyRetry: "patch-history-retry",
  historyEmpty: "patch-history-empty",
  /** "Unchanged since v0.2.0." */
  historyUnchanged: "patch-history-unchanged",
  /** One patch in a card's history, `data-version` and `data-kind`. */
  historyEntry: "patch-history-entry",
} as const;
