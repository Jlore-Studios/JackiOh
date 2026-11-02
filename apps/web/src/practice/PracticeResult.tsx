// The end of a practice game: Victory, Defeat or Draw, why, and what to do next (SPEC §9.9). It reads
// only the human's `PlayerView` result (CLAUDE.md rule 7); `Game.tsx`'s `result-overlay` still renders
// underneath, unchanged, for every other consumer of the board.

import { useEffect, useId, useRef, type CSSProperties, type ReactElement } from "react";

import type { Difficulty } from "@jackioh/engine/config";
import type { GameOverReason, PlayerId, PlayerView } from "@jackioh/shared";

import XpReveal from "../economy/XpReveal.tsx";
import { seatOutcome } from "../game/outcome.ts";
import { t } from "../i18n/i18n.ts";
import { PRACTICE_RESULT_PARTICLES } from "./config.ts";
import { earningText, type PracticeEarning } from "./earning.ts";
import { practiceTestid } from "./testids.ts";
import { DIFFICULTY_LABEL, TierCrest } from "./Tier.tsx";
import "./practice.css";

export type PracticeOutcome = "win" | "loss" | "draw";

type Result = NonNullable<PlayerView["result"]>;

/** R1843004: the AI's `fled` is a draw for the human (`game/outcome.ts`). */
export function outcomeOf(result: Result, viewer: PlayerId): PracticeOutcome {
  return seatOutcome(result.winner, result.reason, viewer);
}

export const OUTCOME_TITLE: Readonly<Record<PracticeOutcome, string>> = {
  win: "Victory",
  loss: "Defeat",
  draw: "Draw",
};

/** Why the game ended, from the human's side of the table. `winner` tells two fleers' draw apart. */
function reasonText(outcome: PracticeOutcome, reason: GameOverReason, winner: PlayerId | "draw"): string {
  switch (reason) {
    case "hero-death":
      return outcome === "win" ? "The AI's hero has fallen." : "Your hero has fallen.";
    case "both-heroes-dead":
      return "Both heroes fell together.";
    case "concede":
      return outcome === "win" ? "The AI conceded." : "You conceded.";
    case "turn-cap":
      return "The turn limit was reached.";
    case "draw-accepted":
      return "Game drawn by agreement.";
    case "disconnect":
    case "match-ceiling":
      return "The game has ended.";
    case "voided":
      // R679: a Glitch voided the game.
      return "This match never happened.";
    case "alt-win":
      if (outcome === "draw") return "Both players met a winning condition.";
      return outcome === "win"
        ? "You met your winning condition."
        : "The AI met its winning condition.";
    case "won-by-effect":
      // R850: an effect won the game outright.
      return outcome === "win" ? "Your card won the game." : "The AI's card won the game.";
    case "voted-out":
      // R1843000: a player's tally filled, and the cards that voted removed them.
      return outcome === "win" ? "The AI was voted out of the game." : "You were voted out of the game.";
    case "both-voted-out":
      // R1843001: both tallies filled at once.
      return "Both players were voted out together.";
    case "fled":
      // R1843002, R1843004: a win for the fleer and a draw for the other player; two fleers draw.
      if (outcome === "win") return "You escaped the universe. The AI draws.";
      return winner === "draw" ? "Both players escaped the universe." : "The AI escaped the universe. You draw.";
    case "fled-decisive":
      // R1843002: the fleer chose a loss for the other player.
      return outcome === "win"
        ? "You escaped the universe and left the AI behind."
        : "The AI escaped the universe and left you behind.";
  }
}

type PracticeResultProps = {
  result: Result;
  viewer: PlayerId;
  difficulty: Difficulty;
  onPlayAgain(): void;
  onChangeSetup(): void;
  onViewBoard(): void;
  /** R768000: the kept game's replay from step 0; absent for a game that was not kept. */
  onWatchReplay?: () => void;
  /** R768000: the kept game's replay at its last step, making its last turn's GIF. */
  onShareLastTurn?: () => void;
  /** R1303000: what a game on a server seed earned, or why not; absent for any other game. #1664: a paid one plays its tracks and quests. */
  earning?: PracticeEarning;
};

export function PracticeResult({
  result,
  viewer,
  difficulty,
  onPlayAgain,
  onChangeSetup,
  onViewBoard,
  onWatchReplay,
  onShareLastTurn,
  earning,
}: PracticeResultProps): ReactElement {
  const outcome = outcomeOf(result, viewer);
  const titleId = useId();
  const playAgain = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    playAgain.current?.focus();
  }, []);

  useEffect(() => {
    function onKey(event: KeyboardEvent): void {
      if (event.key === "Escape") onViewBoard();
    }
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("keydown", onKey);
    };
  }, [onViewBoard]);

  return (
    <div className="practice-result-scrim" data-outcome={outcome}>
      <section
        className="practice-result"
        data-testid={practiceTestid.result}
        data-outcome={outcome}
        role="dialog"
        aria-modal="true"
        aria-labelledby={titleId}
      >
        <div className="practice-result__emblem" aria-hidden="true">
          <span className="practice-result__rays" />
          {/* Sparks burst from the crest on a Victory; embers drift off it on a Defeat. Each particle
              reads its index as `--i`, and practice.css spreads them round the circle. */}
          {outcome === "draw" ? null : (
            <span className="practice-result__particles">
              {Array.from({ length: PRACTICE_RESULT_PARTICLES }, (_unused, index) => (
                <i key={index} style={{ "--i": index, "--n": PRACTICE_RESULT_PARTICLES } as CSSProperties} />
              ))}
            </span>
          )}
          <TierCrest tier={difficulty} size="lg" />
        </div>
        <h2 className="practice-result__title" id={titleId}>
          {t(OUTCOME_TITLE[outcome])}
        </h2>
        <p className="practice-result__reason">{t(reasonText(outcome, result.reason, result.winner))}</p>
        <p className="practice-result__meta">{t("Practice")} · {t(DIFFICULTY_LABEL[difficulty])}</p>
        {earning === undefined ? null : (
          <p
            className="practice-result__meta"
            data-testid={practiceTestid.resultEarning}
            data-earning={earning.kind}
            {...(earning.kind === "refused" ? { "data-reason": earning.reason } : {})}
            role="status"
          >
            {earningText(earning)}
          </p>
        )}
        {earning?.kind === "paid" ? (
          <XpReveal economy={{ tracks: earning.tracks ?? [], quests: earning.quests ?? [] }} />
        ) : null}
        <div className="practice-result__actions">
          <button
            ref={playAgain}
            type="button"
            className="practice-play"
            data-testid={practiceTestid.playAgain}
            onClick={onPlayAgain}
          >
            {t("Play again")}
          </button>
          <button
            type="button"
            className="practice-result__secondary"
            data-testid={practiceTestid.changeSetup}
            onClick={onChangeSetup}
          >
            {t("Change setup")}
          </button>
        </div>
        {onWatchReplay === undefined && onShareLastTurn === undefined ? null : (
          <div className="practice-result__replay">
            {onWatchReplay === undefined ? null : (
              <button type="button" className="practice-result__secondary" data-testid={practiceTestid.watchReplay} onClick={onWatchReplay}>
                {t("Watch the replay")}
              </button>
            )}
            {onShareLastTurn === undefined ? null : (
              <button type="button" className="practice-result__secondary" data-testid={practiceTestid.shareLastTurn} onClick={onShareLastTurn}>
                {t("Share the last turn")}
              </button>
            )}
          </div>
        )}
        <button type="button" className="link-button practice-result__view" data-testid={practiceTestid.viewBoard} onClick={onViewBoard}>
          {t("View the board")}
        </button>
      </section>
    </div>
  );
}
