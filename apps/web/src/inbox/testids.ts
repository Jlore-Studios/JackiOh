// The inbox's browser-test contract (R3729002). It imports nothing, so `e2e/support/testids.ts` can mirror
// the strings. A row carries `data-kind` (`challenge|invite|dailyMove|dailyChallenge|request|replay`) and
// `data-key`; Accept, Decline and Open stand on more than one kind of row, told apart by that `data-kind`.
// R4039001: the Recent section's rows (`recentRow`) carry `data-kind` (the feed's kind: `dailyMove|dailyEnded|
// dailyChallenge|dailyAnswered|friendRequest|friendAccepted|lobbyInvite|replayShared`) and `data-unread`.

export const inboxTestid = {
  bell: "inbox-bell",
  /** The count of what needs the player; absent at zero. */
  badge: "inbox-badge",
  panel: "inbox-panel",
  close: "inbox-close",
  /** One per thing waiting: `data-kind`, `data-key`. */
  item: "inbox-item",
  empty: "inbox-empty",
  /** A live challenge's time left, ticking. */
  left: "inbox-left",
  /** A live challenge or a friend request. */
  accept: "inbox-accept",
  /** A live challenge's way into its room's lobby, when it cannot join from here. */
  lobby: "inbox-lobby",
  /** A live challenge, a daily challenge or a friend request. */
  decline: "inbox-decline",
  /** A lobby invite. */
  join: "inbox-join",
  dismiss: "inbox-dismiss",
  /** A daily game on the player's move. */
  play: "inbox-play",
  /** A daily challenge: it opens the Daily screen, where accepting asks for a deck. */
  open: "inbox-open",
  /** A shared replay. */
  watch: "inbox-watch",
  /** A refusal of an answer, in the server's words. */
  error: "inbox-error",
  /** R4039001: the bell's dot, when the feed holds unread rows and nothing is counted on the badge. */
  dot: "inbox-dot",
  /** The Recent section: the notification feed. */
  recent: "inbox-recent",
  /** One per row of the feed: `data-kind`, `data-unread`. */
  recentRow: "inbox-recent-row",
  /** A row the player had not read when the panel opened. */
  recentDot: "inbox-recent-dot",
  /** A row's Open: where it happened. */
  recentOpen: "inbox-recent-open",
  /** The feed holds no row. */
  recentEmpty: "inbox-recent-empty",
  /** A read of the feed that failed, in the server's words. */
  recentError: "inbox-recent-error",
  /** Reads the page of rows before the oldest shown. */
  older: "inbox-older",
  /** The one notification toast: `data-kind`. */
  toast: "notification-toast",
  toastOpen: "notification-toast-open",
  toastDismiss: "notification-toast-dismiss",
} as const;
