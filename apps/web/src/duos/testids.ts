// The Duos screens' testids (#936 S8), mirrored in `e2e/support/testids.ts`. The first block is S8's
// frozen list; the rest are this slice's own chrome, each named for what it holds.

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

  // The panel's own: the deck choice, the lean, the open room's controls, its status and refusals.
  coopDeck: "coop-deck",
  coopLeanNewest: "coop-lean-newest",
  coopCancel: "coop-cancel",
  coopInviteFriend: "coop-invite-friend",
  coopInvite: "coop-invite",
  coopStatus: "coop-status",
  coopError: "coop-error",
  coopRejoin: "coop-rejoin",
  // The board's own: the rail's pieces, the concede offer and the result.
  board: "duos-board",
  lifeStrip: "duos-life-strip",
  round: "duos-round",
  waiting: "duos-waiting",
  concede: "duos-concede",
  concedeStatus: "duos-concede-status",
  cardActions: "duos-card-actions",
  result: "duos-result",
  resultBack: "duos-result-back",
  resultBoard: "duos-result-board",
  // The route's own: before the first view, and its clock and connection line.
  connecting: "duos-connecting",
  refused: "duos-refused",
  connection: "duos-connection",
  clock: "duos-clock",
  // `/dev/duos` (B65): the seat switcher.
  devSeat: (seat: DuosSeat): string => `duos-dev-seat-${seat}`,
} as const;
