// When the prompt on the held-back view has had its say (#37). Pure: the burst's entries in, a flag out.

import type { PlayerView } from "@jackioh/shared";

import type { AnimationEntry } from "./animations.ts";

/**
 * #37: whether the prompt on the held-back view has already had its say. The view is held back
 * until every entry of the burst has played, so a prompt that was answered would otherwise fade out
 * (promptAnswered) and then come back over the animations that follow it, a mulligan's modal over
 * the first turn's banner and draws. It goes once its own fade-out is past; the waiting panel of a
 * seat that answered first goes as the first turn starts.
 */
export function promptOver(shown: PlayerView, burst: readonly AnimationEntry[], inFlight: AnimationEntry | null): boolean {
  const answered = burst.some(
    (entry) =>
      entry !== inFlight && entry.events.some((e) => e.type === "promptAnswered" && e.player === shown.viewer),
  );
  const firstTurn =
    shown.phase === "mulligan" && burst.some((entry) => entry.events.some((e) => e.type === "turnStarted"));
  return answered || firstTurn;
}
