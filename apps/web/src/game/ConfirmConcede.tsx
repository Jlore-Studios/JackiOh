// "Concede this game?": what the board's Concede control opens, in every mode. A concede ends the
// game at once and cannot be taken back, and the control sits one stray tap from End turn, so it
// asks first. Only "Concede" sends the action; "Keep playing" is the default: it takes the focus,
// and Escape or a click outside the panel means the same.
//
// It decides nothing (CLAUDE.md rule 7): the caller dispatches `{ type: "concede" }`.
//
// An `alertdialog`, modal, labelled by its question and described by its line of consequence. Focus
// moves in on open, Tab and Shift+Tab stay between the two buttons, and the caller puts the focus
// back on the Concede control once it closes.

import { useEffect, useId, useRef, type ReactElement } from "react";

import { testid } from "./contract.ts";
import "./notices.css";

export type ConfirmConcedeProps = {
  /** "Concede": send the concede. */
  onConfirm(): void;
  /** "Keep playing", Escape or a click outside the panel. */
  onCancel(): void;
};

export const CONCEDE_TITLE = "Concede this game?";
export const CONCEDE_BODY = "Your opponent wins and the game ends here.";

export default function ConfirmConcede({ onConfirm, onCancel }: ConfirmConcedeProps): ReactElement {
  const titleId = useId();
  const bodyId = useId();
  const stay = useRef<HTMLButtonElement>(null);
  const concede = useRef<HTMLButtonElement>(null);

  // The safe choice has the focus from the start, so a stray Enter or Space keeps the game going.
  useEffect(() => {
    stay.current?.focus({ preventScroll: true });
  }, []);

  // Both keys are read at the window, in the capture phase, so they hold wherever the focus is (a
  // click on the scrim can take it off the buttons). Tab never reaches the board behind the scrim.
  useEffect(() => {
    function onKey(event: globalThis.KeyboardEvent): void {
      if (event.key === "Escape") {
        event.preventDefault();
        event.stopPropagation();
        onCancel();
        return;
      }
      if (event.key !== "Tab") return;
      const order = [stay.current, concede.current].filter((el): el is HTMLButtonElement => el !== null);
      if (order.length === 0) return;
      const at = order.indexOf(document.activeElement as HTMLButtonElement);
      const next = event.shiftKey ? (at <= 0 ? order.length - 1 : at - 1) : at === order.length - 1 ? 0 : at + 1;
      event.preventDefault();
      event.stopPropagation();
      order[next]?.focus();
    }
    window.addEventListener("keydown", onKey, true);
    return () => {
      window.removeEventListener("keydown", onKey, true);
    };
  }, [onCancel]);

  return (
    <div
      className="concede-scrim"
      onClick={(event) => {
        if (event.target === event.currentTarget) onCancel();
      }}
    >
      <section
        className="concede-dialog"
        data-testid={testid.concedeDialog}
        role="alertdialog"
        aria-modal="true"
        aria-labelledby={titleId}
        aria-describedby={bodyId}
        // A click on the panel's text keeps the focus in the dialog.
        tabIndex={-1}
      >
        <h2 className="concede-dialog__title" id={titleId}>
          {CONCEDE_TITLE}
        </h2>
        <p className="concede-dialog__body" id={bodyId}>
          {CONCEDE_BODY}
        </p>
        <div className="concede-dialog__actions">
          <button
            ref={stay}
            type="button"
            className="concede-dialog__stay"
            data-testid={testid.concedeCancel}
            onClick={onCancel}
          >
            Keep playing
          </button>
          <button
            ref={concede}
            type="button"
            className="concede-dialog__concede"
            data-testid={testid.concedeConfirm}
            onClick={onConfirm}
          >
            Concede
          </button>
        </div>
      </section>
    </div>
  );
}
