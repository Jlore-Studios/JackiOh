// The inbox's browser-test contract (R3729002). It imports nothing, so `e2e/support/testids.ts` can mirror
// the strings. A row carries `data-kind` (`challenge|invite|dailyMove|dailyChallenge|request|replay`) and
// `data-key`; Accept, Decline and Open stand on more than one kind of row, told apart by that `data-kind`.

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
} as const;
