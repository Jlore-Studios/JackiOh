// The bar above a lesson's board (SPEC §9.10), with Exit tutorial (nothing the coach draws is ever placed
// over it) and the Field Guide (R2205002); there is no Skip step (R314). It wears the practice HUD's look
// (`practice-hud`), so the board below is laid out exactly as in any practice game.

import { useSyncExternalStore, type CSSProperties, type ReactElement } from "react";

import type { PlayerId, PlayerView } from "@jackioh/shared";

import { t, useLocale } from "../i18n/i18n.ts";
import { ModifierList } from "../game/ModifierList.tsx";
import { OUTCOME_TITLE, type PracticeOutcome } from "../practice/PracticeResult.tsx";
import { ThinkIndicator } from "../practice/ThinkIndicator.tsx";
import { FieldGuideButton } from "./FieldGuide.tsx";
import type { TutorialLesson } from "./lessons.ts";
import { tutorialTestid } from "./testids.ts";
import type { CoachTracker } from "./tracker.ts";
import "./tutorial.css";

type TutorialHudProps = {
  lesson: TutorialLesson;
  tracker: CoachTracker;
  view: PlayerView;
  humanSeat: PlayerId;
  aiSeat: PlayerId | null;
  thinking: boolean;
  outcome: PracticeOutcome | null;
  onShowResult(): void;
  /** Mid-lesson this opens "Leave this game?"; once it is over it leaves at once. */
  onExit(): void;
};

export function TutorialHud({
  lesson,
  tracker,
  view,
  humanSeat,
  aiSeat,
  thinking,
  outcome,
  onShowResult,
  onExit,
}: TutorialHudProps): ReactElement {
  useLocale();
  const coach = useSyncExternalStore(tracker.subscribe, tracker.getState, tracker.getState);
  const display = coach.display;
  const over = outcome !== null;
  const counting = display.mode !== "finished";
  const stepNumber = counting ? display.stepNumber : 0;
  const stepCount = counting ? display.stepCount : 0;

  return (
    <header
      className="practice-hud tutorial-hud"
      data-testid={tutorialTestid.hud}
      data-lesson={lesson.id}
      data-human-seat={humanSeat}
      data-ai-seat={aiSeat ?? ""}
      data-thinking={thinking ? "true" : "false"}
    >
      <span className="tutorial-hud__lesson">
        <span className="tutorial-hud__badge" aria-hidden="true">
          {lesson.number}
        </span>
        <span className="tutorial-hud__label">
          <span className="practice-hud__mode tutorial-hud__mode">{t("Tutorial ·")}</span>
          <strong className="tutorial-hud__title">
            {t("Lesson {number}: {title}", { number: lesson.number, title: t(lesson.title) })}
          </strong>
        </span>
      </span>
      {counting && !over ? (
        <span className="tutorial-hud__step" data-testid={tutorialTestid.step}>
          <span className="tutorial-hud__step-text">
            <span className="tutorial-hud__step-long">
              {t("Step {n} of {total}", { n: stepNumber, total: stepCount })}
            </span>
            <span className="tutorial-hud__step-short" aria-hidden="true">
              {stepNumber}/{stepCount}
            </span>
          </span>
          <span
            className="tutorial-hud__meter"
            aria-hidden="true"
            style={{ "--done": stepCount === 0 ? 0 : (stepNumber - 1) / stepCount } as CSSProperties}
          />
        </span>
      ) : null}
      <span className="practice-hud__status">
        <ModifierList view={view} opponentLabel={t("AI")} />
        <ThinkIndicator thinking={thinking} />
        {outcome === null ? null : (
          <button
            type="button"
            className="practice-hud__outcome"
            data-testid={tutorialTestid.outcome}
            data-outcome={outcome}
            onClick={onShowResult}
          >
            {t(OUTCOME_TITLE[outcome])}
          </button>
        )}
      </span>
      <FieldGuideButton testid={tutorialTestid.fieldGuideHud} compact />
      <button
        type="button"
        className="tutorial-hud__exit"
        data-testid={tutorialTestid.exit}
        aria-haspopup={over ? undefined : "dialog"}
        aria-label={t("Exit tutorial")}
        onClick={onExit}
      >
        <span className="tutorial-hud__long">{t("Exit tutorial")}</span>
        <span className="tutorial-hud__short" aria-hidden="true">
          {t("Exit")}
        </span>
      </button>
    </header>
  );
}
