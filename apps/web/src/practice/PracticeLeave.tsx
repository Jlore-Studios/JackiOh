// "Leave this game?": what "New game" and "Menu" ask while a practice game is still being played.
//
// A game against the AI costs nothing to abandon, but it is still minutes of play one stray tap
// from gone, and "New game" sits in the HUD beside the board. Staying is the default: it has the
// focus, and Escape or a click outside the panel means stay.
//
// R765: a free game is the player's to keep. Its question offers Save and leave, which keeps the
// game for the practice menu's banner, beside Leave without saving, which gives it up. A tutorial
// lesson is never kept (R668), so its question keeps the one way out it always had.

import { useEffect, useId, useRef, type ReactElement } from "react";

import { practiceTestid } from "./testids.ts";
import "./practice.css";

/**
 * Where leaving goes: back to the setup ("New game"), out to the main menu ("Menu"), or from a
 * tutorial lesson back to the lesson path ("Exit tutorial").
 */
export type PracticeLeaveTo = "setup" | "menu" | "lessons";

type PracticeLeaveProps = {
  to: PracticeLeaveTo;
  onStay(): void;
  /** Leave and give the game up. */
  onLeave(): void;
  /** R765: leave and keep the game; offered only when given (a free game, never a lesson). */
  onSaveAndLeave?: () => void;
};

/** Where the player lands, in the question's own words. */
const LEAVE_TO: Record<PracticeLeaveTo, string> = {
  setup: "choosing a difficulty and a deck",
  menu: "the main menu",
  lessons: "the lessons",
};

const LEAVE_BODY: Record<PracticeLeaveTo, string> = {
  setup: `It ends here and you go back to ${LEAVE_TO.setup}.`,
  menu: `It ends here and you go back to ${LEAVE_TO.menu}.`,
  lessons: `It ends here and you go back to ${LEAVE_TO.lessons}. You can start it again any time.`,
};

/** R765: a free game's question, which can keep it. */
function saveBody(to: PracticeLeaveTo): string {
  return `You go back to ${LEAVE_TO[to]}. Save it to pick it up later from the practice menu, or leave without saving and it ends here.`;
}

export function PracticeLeave({ to, onStay, onLeave, onSaveAndLeave }: PracticeLeaveProps): ReactElement {
  const titleId = useId();
  const stay = useRef<HTMLButtonElement>(null);
  const canSave = onSaveAndLeave !== undefined;

  useEffect(() => {
    stay.current?.focus();
  }, []);

  useEffect(() => {
    function onKey(event: KeyboardEvent): void {
      if (event.key === "Escape") onStay();
    }
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("keydown", onKey);
    };
  }, [onStay]);

  return (
    <div
      className="practice-leave-scrim"
      onClick={(event) => {
        if (event.target === event.currentTarget) onStay();
      }}
    >
      <section
        className="practice-leave"
        data-testid={practiceTestid.leave}
        data-can-save={canSave ? "true" : "false"}
        role="alertdialog"
        aria-modal="true"
        aria-labelledby={titleId}
      >
        <h2 className="practice-leave__title" id={titleId}>
          Leave this game?
        </h2>
        <p className="practice-leave__body">{canSave ? saveBody(to) : LEAVE_BODY[to]}</p>
        <div className={canSave ? "practice-leave__actions practice-leave__actions--save" : "practice-leave__actions"}>
          <button
            ref={stay}
            type="button"
            className="practice-play"
            data-testid={practiceTestid.leaveStay}
            onClick={onStay}
          >
            Keep playing
          </button>
          {canSave ? (
            <button
              type="button"
              className="practice-result__secondary"
              data-testid={practiceTestid.leaveSave}
              onClick={onSaveAndLeave}
            >
              Save and leave
            </button>
          ) : null}
          <button
            type="button"
            className="practice-result__secondary"
            data-testid={practiceTestid.leaveConfirm}
            onClick={onLeave}
          >
            {canSave ? "Leave without saving" : "Leave game"}
          </button>
        </div>
      </section>
    </div>
  );
}
