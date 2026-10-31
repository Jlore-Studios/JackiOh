// Play together's chrome (#936, SPEC S8): the friends page, the chat panel and the Friends button's
// panel, the challenge toast, the watch screen and the replay share. The SPEC fixes the
// first ids of each group; the rest are this slice's. `e2e/support/testids.ts` mirrors the strings.

export const togetherTestid = {
  root: "together",
  addInput: "together-add-input",
  addSubmit: "together-add-submit",
  /** The server's refusal of an add (B55), in its own words. */
  addError: "together-add-error",
  addSent: "together-add-sent",
  /** A refusal of any other action on the page. */
  error: "together-error",
  /** One per friend: `data-profile-id`, `data-status` (`offline|online|inMatch`), `data-watchable`. */
  friendRow: "together-friend-row",
  noFriends: "together-no-friends",
  challenge: "together-challenge",
  watch: "together-watch",
  /** R2615000: the Watching panel's note that friends may always watch, and how to stop one. */
  watchingNote: "together-watching-note",
  chatOpen: "together-chat-open",
  unread: "together-unread",
  remove: "together-remove",
  block: "together-block",
  /** The inline "are you sure" of Block; Remove asks nothing and offers Undo (R2119000). */
  confirm: "together-confirm",
  keep: "together-keep",
  /** One per request: `data-direction` (`incoming|outgoing`), `data-request-id`. */
  requestRow: "together-request-row",
  requestAccept: "together-request-accept",
  requestDecline: "together-request-decline",
  requestCancel: "together-request-cancel",
  requestBlock: "together-request-block",
  blockedRow: "together-blocked-row",
  unblock: "together-unblock",
  challengeForm: "together-challenge-form",
  challengeDeck: "together-challenge-deck",
  challengeTrio: "together-challenge-trio",
  /** R2031000: under All Random only, "More cards from" (R924000), a select. */
  challengeLean: "together-challenge-lean",
  challengeSend: "together-challenge-send",
  challengeClose: "together-challenge-close",
  challengeError: "together-challenge-error",
  /** #1568: a failed deck read offers Try again beside the alert. */
  challengeDecksRetry: "together-challenge-decks-retry",
  /** R915005: under a deck or trio picker that lists one naming a custom card: why it is off. */
  challengePracticeOnly: "together-challenge-practice-only",
  /** One per open challenge this account sent: `data-challenge-id`, `data-room-code`. */
  outgoingChallenge: "together-outgoing-challenge",
  cancelChallenge: "together-cancel-challenge",
  /** One per replay shared with this account: `data-match-id`. */
  sharedRow: "together-shared-row",
  sharedOpen: "together-shared-open",
  noShared: "together-no-shared",
  /** #1568: a failed shared list offers Try again beside the alert. */
  sharedRetry: "together-shared-retry",
  /** #1691: a failed friends list says why, and Try again reads it again. */
  friendsError: "together-friends-error",
  friendsRetry: "together-friends-retry",
} as const;

/** A mode's radio in the challenge form: `together-challenge-mode-bo1|bo3|random`. */
export function challengeModeTestid(mode: "bo1" | "bo3" | "random"): string {
  return `together-challenge-mode-${mode}`;
}

export const chatTestid = {
  /** `data-friend-id`. */
  panel: "chat-panel",
  input: "chat-input",
  send: "chat-send",
  /** One per message: `data-message-id`, `data-from` (`you|friend`). */
  message: "chat-message",
  close: "chat-close",
  error: "chat-error",
  join: "chat-join",
  /** #1898: a failed read of the conversation says why, and Try again reads it again. */
  readError: "chat-read-error",
  retry: "chat-retry",
  earlier: "chat-earlier",
  report: "chat-report",
  reportReason: "chat-report-reason",
  reportSend: "chat-report-send",
  reported: "chat-reported",
  /** R2120000: one per message sent from this panel that no answer has replaced yet: `data-state` (`sending|failed`), `data-from="you"`. */
  pending: "chat-pending",
  pendingRetry: "chat-pending-retry",
  pendingDiscard: "chat-pending-discard",
} as const;

/** #1831: the Friends button every signed-in screen has at the top right, and its panel. */
export const friendsTestid = {
  button: "friends-button",
  /** What needs the player: incoming requests, incoming challenges and unread chats. */
  badge: "friends-badge",
  panel: "friends-panel",
  close: "friends-close",
  copy: "friends-copy",
  /** A refusal of a request answered from the panel. */
  error: "friends-error",
  /** #1691: a failed friends list says why, and Try again reads it again. */
  listError: "friends-list-error",
  listRetry: "friends-list-retry",
  /** The link to all of Play together. */
  all: "friends-all",
} as const;

export const challengeToastTestid = {
  /** One per incoming challenge: `data-challenge-id`, `data-room-code`, `data-mode`. */
  toast: "challenge-toast",
  accept: "challenge-toast-accept",
  decline: "challenge-decline",
  /** The deck or trio the toast joins with. */
  deck: "challenge-toast-deck",
  /** R915005: under a deck or trio picker that lists one naming a custom card: why it is off. */
  practiceOnly: "challenge-toast-practice-only",
  /** R2031000: an All Random challenge's "More cards from" (R924000), a select. */
  lean: "challenge-toast-lean",
  /** R767's way in: `/play` with the room's code filled in, offered after a refused join. */
  lobby: "challenge-toast-lobby",
  error: "challenge-toast-error",
} as const;

export const watchTestid = {
  /** `data-connection` (`connecting|open|closed|refused`). */
  screen: "watch-screen",
  /** The panel before the first view: the wait, or (#1599) the notice that the reopens ran out. */
  waiting: "watch-waiting",
  refused: "watch-refused",
  /** #1599: the sentence that stands in for the wait once the reopens ran out before any view. */
  gone: "watch-gone",
  status: "watch-status",
  back: "watch-back",
  /** #1570: "Reconnect now", once the screen's own reopens have run out on a game still running. */
  reconnect: "watch-reconnect",
} as const;

export const shareTestid = {
  open: "share-friend-open",
  /** One per friend: `data-profile-id`, `data-shared`. */
  option: "share-friend-option",
  unshare: "share-friend-unshare",
  none: "share-friend-none",
  error: "share-friend-error",
  /** #1691: after the first read of the friends failed, Try again reads them again. */
  retry: "share-friend-retry",
} as const;

