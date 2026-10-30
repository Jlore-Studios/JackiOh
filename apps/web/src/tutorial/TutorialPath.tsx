// Tutorial path for `/practice` (SPEC §9.10, R294); locked lessons remain focusable.
// R322 (with account copy, R321): the player may persistently hide the path, with focus returned to Show or Hide.
// R2205002: a lesson's chips are the Field Guide entries it sets up, each marked once found.

import { useEffect, useId, useRef, useState, type ReactElement } from "react";

import { t, useLocale } from "../i18n/i18n.ts";
import { discoveryById } from "./discoveries.ts";
import { FieldGuideButton } from "./FieldGuide.tsx";
import { TUTORIAL_LESSONS, type TutorialLesson } from "./lessons.ts";
import {
  fieldGuideOf,
  isTutorialHidden,
  lessonLost,
  lessonStatus,
  nextLessonToPlay,
  setTutorialHidden,
  useTutorialProgress,
  type LessonStatus,
} from "./progress.ts";
import { tutorialTestid } from "./testids.ts";
import "./tutorial.css";
import "./path-visibility.css";

type TutorialPathProps = {
  onStart(lesson: TutorialLesson): void;
};

function CheckIcon(): ReactElement {
  return (
    <svg className="tutorial-icon" viewBox="0 0 16 16" aria-hidden="true">
      <path d="M3.5 8.5 L6.5 11.5 L12.5 4.5" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round" strokeLinejoin="round" />
    </svg>
  );
}

function LockIcon(): ReactElement {
  return (
    <svg className="tutorial-icon" viewBox="0 0 16 16" aria-hidden="true">
      <rect x="3.5" y="7" width="9" height="7" rx="1.5" fill="currentColor" />
      <path d="M5.5 7 V5.2 a2.5 2.5 0 0 1 5 0 V7" fill="none" stroke="currentColor" strokeWidth="1.6" />
    </svg>
  );
}

function statusText(status: LessonStatus, lesson: TutorialLesson, isNext: boolean, lost: boolean): string {
  // R2434002: a conceded lesson stays open, and says it was lost.
  if (lost && status === "unlocked") return t("Lost: you conceded");
  switch (status) {
    case "completed":
      return t("Completed");
    case "locked":
      return t("Finish lesson {n} first", { n: lesson.number - 1 });
    case "unlocked":
      return isNext ? t("Up next") : t("Open");
  }
}

function buttonLabel(status: LessonStatus): string {
  switch (status) {
    case "completed":
      return t("Replay");
    case "locked":
      return t("Locked");
    case "unlocked":
      return t("Start");
  }
}

function Chips({ lesson, met }: { lesson: TutorialLesson; met: readonly string[] }): ReactElement {
  return (
    <ul className="tutorial-lesson__chips" aria-label={t("What you can find")}>
      {lesson.discoveries.map((id) => {
        const found = met.includes(id);
        const name = t(discoveryById(id)?.name ?? id);
        return (
          <li key={id} className="tutorial-lesson__chip" data-found={found ? "true" : "false"}>
            {found ? <span className="tutorial-sr-only">{t("Found: {name}", { name })}</span> : null}
            <span aria-hidden={found ? "true" : undefined}>{name}</span>
          </li>
        );
      })}
    </ul>
  );
}

function LessonNode({
  lesson,
  status,
  isNext,
  lost,
  met,
  onStart,
}: {
  lesson: TutorialLesson;
  status: LessonStatus;
  isNext: boolean;
  lost: boolean;
  met: readonly string[];
  onStart(lesson: TutorialLesson): void;
}): ReactElement {
  const statusId = useId();
  const locked = status === "locked";
  const label = buttonLabel(status);
  return (
    <li
      className="tutorial-lesson"
      data-testid={tutorialTestid.lesson(lesson.id)}
      data-status={status}
      data-next={isNext ? "true" : undefined}
      data-lost={lost ? "true" : undefined}
    >
      <span className="tutorial-lesson__node" aria-hidden="true">
        {status === "completed" ? <CheckIcon /> : status === "locked" ? <LockIcon /> : lesson.number}
      </span>
      <div className="tutorial-lesson__card">
        <p className="tutorial-lesson__kicker">
          {t("Lesson {number}", { number: lesson.number })}
          {isNext ? <span className="tutorial-lesson__next"> · {t("Up next")}</span> : null}
        </p>
        <h3 className="tutorial-lesson__title">{t(lesson.title)}</h3>
        <p className="tutorial-lesson__summary">{t(lesson.summary)}</p>
        <Chips lesson={lesson} met={met} />
        <div className="tutorial-lesson__foot">
          {/* An open lesson's kicker already says "Up next"; its status is there for a screen reader. */}
          <span
            className={status === "unlocked" && !lost ? "tutorial-lesson__status tutorial-sr-only" : "tutorial-lesson__status"}
            id={statusId}
          >
            {status === "completed" ? <CheckIcon /> : null}
            {statusText(status, lesson, isNext, lost)}
          </span>
          <button
            type="button"
            className={`tutorial-lesson__button tutorial-lesson__button--${status}${isNext ? " tutorial-lesson__button--next" : ""}`}
            data-testid={tutorialTestid.lessonStart(lesson.id)}
            aria-disabled={locked ? "true" : undefined}
            aria-label={t("{label}: lesson {number}, {title}", { label, number: lesson.number, title: t(lesson.title) })}
            aria-describedby={statusId}
            onClick={() => {
              if (!locked) onStart(lesson);
            }}
          >
            {locked ? <LockIcon /> : null}
            {label}
          </button>
        </div>
      </div>
    </li>
  );
}

export function TutorialPath({ onStart }: TutorialPathProps): ReactElement {
  useLocale();
  const progress = useTutorialProgress();
  const headingId = useId();
  const listId = useId();
  const total = TUTORIAL_LESSONS.length;
  const done = TUTORIAL_LESSONS.filter((lesson) => lessonStatus(progress, lesson) === "completed").length;
  const allDone = done === total;
  const next = nextLessonToPlay(progress);
  const met = fieldGuideOf(progress);
  const [openChoice, setOpenChoice] = useState<boolean | null>(null);
  const open = openChoice ?? !allDone;

  const hidden = isTutorialHidden(progress);
  const focusAfter = useRef<"show" | "reveal" | null>(null);
  const showButton = useRef<HTMLButtonElement>(null);
  const hideButton = useRef<HTMLButtonElement>(null);
  const toggleButton = useRef<HTMLButtonElement>(null);
  useEffect(() => {
    const wanted = focusAfter.current;
    if (wanted === null) return;
    focusAfter.current = null;
    const target = wanted === "show" ? showButton.current : (hideButton.current ?? toggleButton.current);
    target?.focus();
  }, [hidden]);

  if (hidden) {
    return (
      <section
        className="tutorial-path-hidden"
        data-testid={tutorialTestid.hidden}
        data-complete={allDone ? "true" : "false"}
        aria-labelledby={headingId}
      >
        <p className="tutorial-path-hidden__text">
          <span className="tutorial-path-hidden__title" id={headingId}>
            {t("Tutorial hidden")}
          </span>
          <span className="tutorial-path-hidden__progress">
            {t("{done} of {total} lessons complete", { done, total })}
          </span>
        </p>
        <button
          ref={showButton}
          type="button"
          className="tutorial-path-hidden__show"
          data-testid={tutorialTestid.show}
          onClick={() => {
            focusAfter.current = "reveal";
            setTutorialHidden(false);
          }}
        >
          {t("Show tutorial")}
        </button>
      </section>
    );
  }

  return (
    <section
      className="tutorial-path"
      data-testid={tutorialTestid.path}
      data-complete={allDone ? "true" : "false"}
      aria-labelledby={headingId}
    >
      <div className="tutorial-path__head">
        <div className="tutorial-path__heading">
          <h2 className="tutorial-path__title" id={headingId}>
            {t("Learn to play")}
          </h2>
        </div>
        <div className="tutorial-path__summary">
          <p className="tutorial-path__progress">
            <span className="tutorial-path__meter" aria-hidden="true">
              {TUTORIAL_LESSONS.map((lesson) => (
                <i key={lesson.id} data-done={lessonStatus(progress, lesson) === "completed" ? "true" : "false"} />
              ))}
            </span>
            {t("{done} of {total} lessons complete", { done, total })}
          </p>
          {next !== undefined && done > 0 ? (
            <button
              type="button"
              className="tutorial-path__continue"
              data-testid={tutorialTestid.continue}
              onClick={() => {
                onStart(next);
              }}
            >
              {t("Continue: Lesson {number}", { number: next.number })}
            </button>
          ) : null}
          <FieldGuideButton testid={tutorialTestid.fieldGuidePath} />
          {!allDone ? (
            <button
              ref={hideButton}
              type="button"
              className="tutorial-path__toggle tutorial-path__hide"
              data-testid={tutorialTestid.hide}
              onClick={() => {
                focusAfter.current = "show";
                setTutorialHidden(true);
              }}
            >
              {t("Hide tutorial")}
            </button>
          ) : null}
          {allDone ? (
            <button
              ref={toggleButton}
              type="button"
              className="tutorial-path__toggle"
              data-testid={tutorialTestid.pathToggle}
              aria-expanded={open}
              aria-controls={listId}
              onClick={() => {
                setOpenChoice(!open);
              }}
            >
              {open ? t("Hide lessons") : t("Show lessons")}
            </button>
          ) : null}
        </div>
      </div>
      <ol className="tutorial-path__lessons" id={listId} hidden={!open}>
        {TUTORIAL_LESSONS.map((lesson) => (
          <LessonNode
            key={lesson.id}
            lesson={lesson}
            status={lessonStatus(progress, lesson)}
            isNext={next?.id === lesson.id}
            lost={lessonLost(progress, lesson)}
            met={met}
            onStart={onStart}
          />
        ))}
      </ol>
    </section>
  );
}
