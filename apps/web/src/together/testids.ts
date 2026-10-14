// Play together's chrome (#936, SPEC S8): the friends page, the chat panel and the in-match chat, the
// challenge toast, the watch screen, the replay share and the spectators setting. The SPEC fixes the
// first ids of each group; the rest are this slice's. `e2e/support/testids.ts` mirrors the strings.

export const togetherTestid = {
  root: "together",
  /** The landing page's Play together call to action (B54). */
  landingCta: "landing-play-together",
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
  chatOpen: "together-chat-open",
  unread: "together-unread",
  remove: "together-remove",
  block: "together-block",
  /** The inline "are you sure" of Remove and Block. */
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
  challengeSend: "together-challenge-send",
  challengeClose: "together-challenge-close",
  challengeError: "together-challenge-error",
  /** One per open challenge this account sent: `data-challenge-id`, `data-room-code`. */
  outgoingChallenge: "together-outgoing-challenge",
  cancelChallenge: "together-cancel-challenge",
  /** One per replay shared with this account: `data-match-id`. */
  sharedRow: "together-shared-row",
  sharedOpen: "together-shared-open",
  noShared: "together-no-shared",
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
  earlier: "chat-earlier",
  report: "chat-report",
  reportReason: "chat-report-reason",
  reportSend: "chat-report-send",
  reported: "chat-reported",
  /** The board rail's chat toggle (B58) and its list of online friends. */
  inMatchToggle: "in-match-chat-toggle",
  inMatchList: "in-match-chat-list",
  inMatchFriend: "in-match-chat-friend",
  inMatchUnread: "in-match-chat-unread",
} as const;

export const challengeToastTestid = {
  /** One per incoming challenge: `data-challenge-id`, `data-room-code`, `data-mode`. */
  toast: "challenge-toast",
  accept: "challenge-accept",
  decline: "challenge-decline",
  error: "challenge-toast-error",
} as const;

export const watchTestid = {
  /** `data-connection` (`connecting|open|closed|refused`). */
  screen: "watch-screen",
  waiting: "watch-waiting",
  refused: "watch-refused",
  status: "watch-status",
  back: "watch-back",
} as const;

export const shareTestid = {
  open: "share-friend-open",
  /** One per friend: `data-profile-id`, `data-shared`. */
  option: "share-friend-option",
  unshare: "share-friend-unshare",
  none: "share-friend-none",
  error: "share-friend-error",
} as const;

/** "Let friends watch my games" (`allowSpectators`), in the settings panel and on Play together. */
export const SPECTATORS_SETTING_TESTID = "setting-allowSpectators";
