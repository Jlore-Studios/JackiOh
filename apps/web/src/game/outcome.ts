// How a finished game went for one seat: a win, a loss or a draw (R1843004).
// Presentation only (CLAUDE.md rule 7): it reads the view's `result` and the engine's own list of
// the endings whose winner's opponent draws (`LOSER_DRAWS_REASONS`, generated from config.rs into
// `engineConfig.ts`), the same list the engine's `seat_outcome`, the ladder and the record read, so
// the client restates no rule. Every ending but those is the winner's win and the other seat's loss,
// or a draw for both; the other player's `fled` is a draw for the viewer.

import { LOSER_DRAWS_REASONS } from "@jackioh/engine/config";
import type { GameOverReason, PlayerId } from "@jackioh/shared";

export type SeatOutcome = "win" | "loss" | "draw";

export function seatOutcome(winner: PlayerId | "draw", reason: GameOverReason, viewer: PlayerId): SeatOutcome {
  if (winner === "draw") return "draw";
  if (winner === viewer) return "win";
  return LOSER_DRAWS_REASONS.includes(reason) ? "draw" : "loss";
}
