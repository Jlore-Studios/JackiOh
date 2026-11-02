// The board's game-over state (BUILD M5-T4 `gameOver`), in two forms:
//  - `panel`: a dimmed board and a panel with the outcome, why, and the route's own ways on
//    (`actions`), plus "View the board", which folds it to the chip;
//  - `chip`: one line at the foot of the screen, with "Result" to open the panel again. Practice asks
//    for this form from the start, because its own result dialog (PracticeResult) is the panel there.
// Both wait for the effects layer's Victory / Defeat sequence (`revealMs`) instead of sitting under
// its word, and neither is a rule (CLAUDE.md rule 7): it reads `PlayerView.result`, and the
// opponent's hand once the view reveals it. `data-reason` keeps the engine's own reason for tests.
//
// R434: the game's end reveals both hands. When the view shows the opponent's (`reveal.ts`), the
// panel lists it as "Their hand", and the chip's button opens it in the list dialog (TheirHand.tsx).

import { createContext, useContext, useEffect, useRef, useState, type CSSProperties, type ReactElement, type ReactNode } from "react";

import type { GameOverReason, PlayerId, PlayerView } from "@jackioh/shared";

import { prefersReducedMotion } from "./animations.ts";
import { testid } from "./contract.ts";
import { seatOutcome } from "./outcome.ts";
import { revealedOpponentHand } from "./reveal.ts";
import { TheirHand, TheirHandButton, type TheirHandProps } from "./TheirHand.tsx";
import { FX_LETHAL_LEAD_MAX_MS, FX_RESULT_MS } from "../fx/constants.ts";
import { getFxSettings } from "../fx/settings.ts";
import { readSettings } from "../settings/index.ts";
import { t, useLocale } from "../i18n/i18n.ts";

type Result = NonNullable<PlayerView["result"]>;

export type ResultOutcome = "win" | "loss" | "draw";
export type ResultForm = "panel" | "chip";

/** R1843004: the other player's `fled` is a draw for the viewer (`outcome.ts`). */
export function outcomeFor(result: Result, viewer: PlayerId): ResultOutcome {
  return seatOutcome(result.winner, result.reason, viewer);
}

/** BUILD M5-T4's overlay words, which e2e reads (`cy.expectResult`). */
export const RESULT_WORD: Readonly<Record<ResultOutcome, string>> = { win: "Win", loss: "Loss", draw: "Draw" };

/**
 * Why the game ended, as the viewer would say it. `winner` is the result's, which tells a draw by two
 * fleers from the other player's flight (R1843002).
 */
export function resultReason(outcome: ResultOutcome, reason: GameOverReason, winner: PlayerId | "draw" | null = null): string {
  switch (reason) {
    case "hero-death":
      return outcome === "win" ? "The opposing hero has fallen." : "Your hero has fallen.";
    case "both-heroes-dead":
      return "Both heroes fell together.";
    case "concede":
      return outcome === "win" ? "Your opponent conceded." : "You conceded.";
    case "draw-accepted":
      return "Game drawn by agreement.";
    case "turn-cap":
      return "The turn limit was reached.";
    case "disconnect":
      return outcome === "win" ? "Your opponent left the match." : "You left the match.";
    case "match-ceiling":
      return "The match reached its time limit.";
    case "voided":
      // R679: a Glitch voided the match; it counts for nobody.
      return "This match never happened.";
    case "alt-win":
      // R850: a held alternative win won.
      if (outcome === "draw") return "Both players met a winning condition.";
      return outcome === "win"
        ? "You met your winning condition."
        : "Your opponent met their winning condition.";
    case "won-by-effect":
      // R850: an effect won the game outright.
      return outcome === "win" ? "Your card won the game." : "Your opponent's card won the game.";
    case "voted-out":
      // R1843000: a player's tally filled, and the cards that voted removed them.
      return outcome === "win" ? "Your opponent was voted out of the game." : "You were voted out of the game.";
    case "both-voted-out":
      // R1843001: both tallies filled at once.
      return "Both players were voted out together.";
    case "fled":
      // R1843002, R1843004: a win for the fleer and a draw for the other player; two fleers draw.
      if (outcome === "win") return "You escaped the universe. Your opponent draws.";
      return winner === "draw" ? "Both players escaped the universe." : "Your opponent escaped the universe. You draw.";
    case "fled-decisive":
      // R1843002: the fleer chose a loss for the other player.
      return outcome === "win"
        ? "You escaped the universe and left your opponent behind."
        : "Your opponent escaped the universe and left you behind.";
  }
}

/**
 * How long the result waits: the effects layer replays the killing blow in at most
 * FX_LETHAL_LEAD_MAX_MS and then plays Victory / Defeat in FX_RESULT_MS; with no effects to watch, 0.
 */
export function resultRevealMs(): number {
  let reduced = prefersReducedMotion();
  try {
    reduced ||= readSettings().reduceMotion;
  } catch {
    // Unreadable settings are the defaults: motion on.
  }
  if (reduced || getFxSettings().intensity === "off") return 0;
  return FX_LETHAL_LEAD_MAX_MS + FX_RESULT_MS;
}

/** R434: what the result shows of the opponent's hand, once the view reveals it; else null. */
export function theirHandOf(view: PlayerView): TheirHandProps | null {
  const cards = revealedOpponentHand(view);
  return cards === null ? null : { view, cards };
}

/** Whether the result panel around it has come in; true outside one. */
const ResultRevealedContext = createContext(true);

/**
 * Its children once the result panel it sits in has come in (`data-revealed`), so what plays in them
 * is seen playing and not over before the panel shows: the XP reveal of #1664.
 */
export function OnceRevealed({ children }: { children: ReactNode }): ReactElement | null {
  return useContext(ResultRevealedContext) ? <>{children}</> : null;
}

type GameResultProps = {
  result: Result;
  viewer: PlayerId;
  form: ResultForm;
  /** The route's ways on, shown in the panel form. */
  actions?: ReactNode;
  /** The animation-runner marker (BUILD M5-T4). */
  animating?: string;
  /** R434: the opponent's hand, once the view reveals it (`revealedOpponentHand`), and its view. */
  theirHand?: TheirHandProps | null;
};

export function GameResult({ result, viewer, form, actions, animating, theirHand = null }: GameResultProps): ReactElement {
  useLocale();
  const outcome = outcomeFor(result, viewer);
  // Only the first appearance waits for the effects layer; a fold or an unfold is immediate.
  const [open, setOpen] = useState(form === "panel");
  const [toggled, setToggled] = useState(false);
  const [revealMs] = useState(resultRevealMs);
  // Until it has come in, it is transparent and takes no pointer, so nothing under it is blocked.
  const [revealed, setRevealed] = useState(revealMs === 0);
  useEffect(() => {
    if (revealed) return undefined;
    const timer = window.setTimeout(() => {
      setRevealed(true);
    }, revealMs);
    return () => {
      window.clearTimeout(timer);
    };
  }, [revealed, revealMs]);
  const shownForm: ResultForm = form === "panel" && open ? "panel" : "chip";
  const delay: CSSProperties | undefined =
    toggled || revealMs === 0 ? undefined : { animationDelay: `${String(revealMs)}ms` };
  const revealedAttr = revealed ? "true" : "false";

  // The panel takes focus as it comes in (and again when "Result" reopens it), on its first way on,
  // as PracticeResult does with Play again: a keyboard is where the next move is.
  const firstAction = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (shownForm !== "panel" || !revealed) return;
    firstAction.current?.querySelector<HTMLElement>("button, a")?.focus({ preventScroll: true });
  }, [shownForm, revealed]);

  return (
    <>
      {shownForm === "panel" ? (
        <div className="result-scrim" style={delay} aria-hidden="true" />
      ) : null}
      <div
        className="result-overlay"
        data-testid={testid.result}
        data-animating={animating}
        data-form={shownForm}
        data-outcome={outcome}
        data-reason={result.reason}
        data-revealed={revealedAttr}
        role="status"
        style={delay}
      >
        <strong className="result-overlay__word">{t(RESULT_WORD[outcome])}</strong>
        <span className="result-overlay__reason">{t(resultReason(outcome, result.reason, result.winner))}</span>
        {shownForm === "panel" && theirHand !== null ? <TheirHand view={theirHand.view} cards={theirHand.cards} /> : null}
        {shownForm === "panel" ? (
          <div className="result-overlay__actions" ref={firstAction}>
            <ResultRevealedContext.Provider value={revealed}>{actions}</ResultRevealedContext.Provider>
            <button
              type="button"
              className="result-overlay__view"
              data-testid="result-view-board"
              onClick={() => {
                setToggled(true);
                setOpen(false);
              }}
            >
              {t("View the board")}
            </button>
          </div>
        ) : form === "panel" ? (
          <button
            type="button"
            className="result-overlay__reopen"
            data-testid="result-reopen"
            onClick={() => {
              setToggled(true);
              setOpen(true);
            }}
          >
            {t("Result")}
          </button>
        ) : null}
        {shownForm === "chip" && theirHand !== null ? <TheirHandButton view={theirHand.view} cards={theirHand.cards} /> : null}
      </div>
    </>
  );
}
