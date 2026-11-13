// The Duos screens' testids (#936 S8), mirrored in `e2e/support/testids.ts`. The first block is S8's
// frozen list; the rest are this slice's own chrome. The co-op panel's are the lobby's now
// (`lobby/testids.ts`, R3730000).

import type { DuosSeat } from "../wire/generated/DuosSeat.ts";
import type { PingKind } from "../net/duos.ts";

export const duosTestid = {
  // S8: the board (B61–B64).
  portal: "duos-portal",
  pass: "duos-pass",
  pool: "duos-pool",
  ownHp: "duos-own-hp",
  teammateStrip: "duos-teammate-strip",
  pingWheel: "duos-ping-wheel",
  ping: (kind: PingKind): string => `duos-ping-${kind}`,
  pingMarker: "duos-ping-marker",

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
  /** R2881000: how many friends watch the game, while any do. */
  watchers: "duos-watchers",
  reconnect: "duos-reconnect",
  /** R3767000: the sentence for a game another tab or device took the seat of, before any view. */
  elsewhere: "duos-elsewhere",
  /** R3767000: the button that takes the seat back, there and in the connection line. */
  playHere: "duos-play-here",
  clock: "duos-clock",
  // The co-op replay's own (#965): its root; its controls carry the replay player's ids (replayTestid).
  replay: "duos-replay",
  // R1242000: the co-op replay's Download JSON.
  replayDownload: "duos-replay-download",
  // `/dev/duos` (B65): the seat switcher.
  devSeat: (seat: DuosSeat): string => `duos-dev-seat-${seat}`,
} as const;
