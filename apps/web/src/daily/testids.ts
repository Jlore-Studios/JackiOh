// Daily games' chrome (#3727): the Daily screen, its dialogs and a daily game's board. `e2e/support/testids.ts`
// mirrors the strings its specs use.

export const dailyTestid = {
  root: "daily",
  /** Opens the New daily game dialog. */
  newGame: "daily-new-game",
  error: "daily-error",
  listError: "daily-list-error",
  listRetry: "daily-list-retry",
  /** No game, challenge or seek at all. */
  empty: "daily-empty",
  /** One per section: `data-section` (`yourMove|theirMove|incoming|outgoing|seeks|finished`). */
  section: "daily-section",
  /** One per game: `data-match-id`, `data-has-move`, `data-over`. */
  gameRow: "daily-game-row",
  /** The row's time left: `data-urgent`. */
  timeLeft: "daily-time-left",
  /** One per challenge or seek: `data-invite-id`, `data-kind`, `data-direction` (`incoming|outgoing|seek`). */
  inviteRow: "daily-invite-row",
  accept: "daily-accept",
  decline: "daily-decline",
  cancel: "daily-cancel",
  /** The notification settings. */
  settings: "daily-settings",
  email: "daily-email",
  noEmail: "daily-no-email",
  push: "daily-push",
  pushNote: "daily-push-note",
  /** iOS and iPadOS Safari, not from the Home Screen: web push needs the Home Screen there. */
  homeScreenNote: "daily-home-screen-note",
  /** The New daily game dialog. */
  dialog: "daily-new-dialog",
  dialogKind: "daily-new-kind",
  friend: "daily-friend",
  noFriends: "daily-no-friends",
  mode: "daily-mode",
  deck: "daily-deck",
  decksRetry: "daily-decks-retry",
  lean: "daily-lean",
  perMove: "daily-per-move",
  send: "daily-send",
  close: "daily-dialog-close",
  dialogError: "daily-dialog-error",
  /** The accept dialog of a challenge. */
  acceptDialog: "daily-accept-dialog",
  acceptSend: "daily-accept-send",
  acceptClose: "daily-accept-close",
  /** A daily game's board. */
  board: "daily-board",
  boardError: "daily-board-error",
  boardRetry: "daily-board-retry",
  /** Whoever holds the move, and its time left: `data-holder` (`you|opponent|none`), `data-urgent`. */
  clock: "daily-clock",
  nextGame: "daily-next-game",
  /** A finished game's Rematch: a new daily challenge to its opponent at its mode and time. */
  rematch: "daily-rematch",
  olderCatalog: "daily-older-catalog",
  backToList: "daily-back",
} as const;

/** One mode's option in the dialog's segmented control. */
export function dailyModeTestid(mode: "bo1" | "random"): string {
  return `daily-mode-${mode}`;
}

/** One time per move's option, in days. */
export function dailyPerMoveTestid(days: number): string {
  return `daily-per-move-${String(days)}`;
}

/** The dialog's tabs: a friend's challenge or the daily queue. */
export function dailyKindTestid(kind: "friend" | "queue"): string {
  return `daily-kind-${kind}`;
}
