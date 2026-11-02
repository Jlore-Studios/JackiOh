// The Duos screens' testids (#936 S8), mirrored in `e2e/support/testids.ts`. The first block is S8's
// frozen list; the rest are this slice's own chrome.

import type { DuosSeat } from "../wire/generated/DuosSeat.ts";
import type { PingKind } from "../net/duos.ts";

export const duosTestid = {
  // S8: the co-op panel (B60).
  coopCreate: "coop-create",
  coopDifficulty: "coop-difficulty",
  coopCode: "coop-code",
  coopJoinInput: "coop-join-input",
  coopJoinSubmit: "coop-join-submit",
  // S8: the board (B61–B64).
  portal: "duos-portal",
  pass: "duos-pass",
  pool: "duos-pool",
  ownHp: "duos-own-hp",
  teammateStrip: "duos-teammate-strip",
  pingWheel: "duos-ping-wheel",
  ping: (kind: PingKind): string => `duos-ping-${kind}`,
  pingMarker: "duos-ping-marker",

  // The panel's own.
  coopDeck: "coop-deck",
  /** R2031000: beside the Random deck only, "More cards from" (R924000), a select. */
  coopLeanNewest: "coop-lean-newest",
  /** R915005: under the deck picker when it lists one naming a custom card: why it is off. */
  coopPracticeOnly: "coop-practice-only",
  coopCancel: "coop-cancel",
  coopInviteFriend: "coop-invite-friend",
  coopInvite: "coop-invite",
  coopLink: "duos-coop-link",
  coopStatus: "coop-status",
  coopError: "coop-error",
  coopRejoin: "coop-rejoin",
  /** #1568: a failed deck read offers Try again beside the alert. */
  coopDecksRetry: "coop-decks-retry",
  /** #1900: a failed read of the live co-op game offers Try again beside the alert. */
  coopLiveRetry: "coop-live-retry",
  // The board's own.
  board: "duos-board",
  /** R2095000: the rail's Both boards toggle, and each board's own wrapper. */
  both: "duos-both",
  boardOwn: "duos-board-own",
  boardTeammate: "duos-board-teammate",
  lifeStrip: "duos-life-strip",
  round: "duos-round",
  waiting: "duos-waiting",
  concede: "duos-concede",
  concedeStatus: "duos-concede-status",
  cardActions: "duos-card-actions",
  result: "duos-result",
  resultBack: "duos-result-back",
  resultBoard: "duos-result-board",
  resultReplay: "duos-result-replay",
  resultJlore: "duos-result-jlore",
  /** #1755: the rail's "Result", while "View the board" has folded the result panel. */
  resultReopen: "duos-result-reopen",
  /** R2092000: the result panel's Play again, and its status line. */
  resultAgain: "duos-result-again",
  resultAgainStatus: "duos-result-again-status",
  // The route's own.
  connecting: "duos-connecting",
  refused: "duos-refused",
  /** #1599: the sentence that stands in for a socket closed for good before any view (a voided match). */
  gone: "duos-gone",
  connection: "duos-connection",
  reconnect: "duos-reconnect",
  clock: "duos-clock",
  // The co-op replay's own (#965): its root; its controls carry the replay player's ids (replayTestid).
  replay: "duos-replay",
  // R1242000: the co-op replay's Download JSON.
  replayDownload: "duos-replay-download",
  // `/dev/duos` (B65): the seat switcher.
  devSeat: (seat: DuosSeat): string => `duos-dev-seat-${seat}`,
} as const;
