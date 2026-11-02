// "Leave the match?": what every way off a live online match opens (R2479000). Leaving closes the
// match socket, and the server's disconnect grace runs: back within DISCONNECT_GRACE_SECONDS, or the
// match counts as a loss (§9.5). Stay is the default: it takes the focus, and Escape, Back (R2127000)
// or a click outside the panel means the same. It decides nothing (CLAUDE.md rule 7): Leave makes
// the move the route's leave guard held (R1754000).
//
// An `alertdialog`, modal, labelled by its question and described by its line of consequence. Its
// keys are `useModalKeys`'s (R2126000: a board route takes no key handler of its own).

import { useEffect, useId, useRef, type ReactElement } from "react";

import { DISCONNECT_GRACE_SECONDS } from "@jackioh/server-config";

import { t, useLocale } from "../i18n/i18n.ts";
import { useModalKeys } from "../together/modalKeys.ts";
import "./notices.css";

export type ConfirmLeaveMatchProps = {
  /** "Stay", Escape, Back or a click outside the panel. */
  onStay(): void;
  /** "Leave": make the held move. */
  onLeave(): void;
};

export const leaveMatchTestid = {
  dialog: "leave-match-dialog",
  stay: "leave-match-stay",
  leave: "leave-match-leave",
} as const;

export default function ConfirmLeaveMatch({ onStay, onLeave }: ConfirmLeaveMatchProps): ReactElement {
  useLocale();
  const titleId = useId();
  const bodyId = useId();
  const root = useRef<HTMLElement>(null);
  const stay = useRef<HTMLButtonElement>(null);
  useModalKeys(root, onStay);

  // The safe choice has the focus from the start, so a stray Enter or Space keeps the player in the match.
  useEffect(() => {
    stay.current?.focus({ preventScroll: true });
  }, []);

  return (
    <div
      className="concede-scrim"
      onClick={(event) => {
        if (event.target === event.currentTarget) onStay();
      }}
    >
      <section
        ref={root}
        className="concede-dialog"
        data-testid={leaveMatchTestid.dialog}
        role="alertdialog"
        aria-modal="true"
        aria-labelledby={titleId}
        aria-describedby={bodyId}
        // A click on the panel's text keeps the focus in the dialog.
        tabIndex={-1}
      >
        <h2 className="concede-dialog__title" id={titleId}>
          {t("Leave the match?")}
        </h2>
        <p className="concede-dialog__body" id={bodyId}>
          {t("It goes on without you. Come back within {seconds} seconds or it counts as a loss.", {
            seconds: DISCONNECT_GRACE_SECONDS,
          })}
        </p>
        <div className="concede-dialog__actions">
          <button
            ref={stay}
            type="button"
            className="concede-dialog__stay"
            data-testid={leaveMatchTestid.stay}
            onClick={onStay}
          >
            {t("Stay")}
          </button>
          <button
            type="button"
            className="concede-dialog__concede"
            data-testid={leaveMatchTestid.leave}
            onClick={onLeave}
          >
            {t("Leave")}
          </button>
        </div>
      </section>
    </div>
  );
}
