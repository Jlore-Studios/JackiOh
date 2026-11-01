// The Field Guide (SPEC §9.10, R2205002): every mechanic the lessons let a player find, in lesson
// order, each met one with its rule and each not yet met as "?" under the lesson that sets it up. Its
// modal dialog gives focus back to the button that opened it.

import { useEffect, useId, useRef, useState, type ReactElement } from "react";
import { createPortal } from "react-dom";

import { t, useLocale } from "../i18n/i18n.ts";
import { useModalKeys } from "../together/modalKeys.ts";
import { ruleSentence } from "./coach.ts";
import { FIELD_GUIDE } from "./discoveries.ts";
import { lessonById } from "./lessons.ts";
import { fieldGuideOf, useTutorialProgress } from "./progress.ts";
import { tutorialTestid } from "./testids.ts";
import "./tutorial.css";

function FieldGuideDialog({ met, onClose }: { met: readonly string[]; onClose(): void }): ReactElement {
  const headingId = useId();
  const dialog = useRef<HTMLElement>(null);
  const close = useRef<HTMLButtonElement>(null);
  useEffect(() => {
    close.current?.focus();
  }, []);
  // The client's modal keys (R2126000 adds no key handler on a board route): Escape closes, Tab stays inside.
  useModalKeys(dialog, onClose);

  return (
    <div className="field-guide-scrim" onClick={onClose}>
      <section
        ref={dialog}
        className="field-guide"
        data-testid={tutorialTestid.fieldGuide}
        role="dialog"
        aria-modal="true"
        aria-labelledby={headingId}
        onClick={(event) => {
          event.stopPropagation();
        }}
      >
        <header className="field-guide__head">
          <h2 className="field-guide__title" id={headingId}>
            {t("Field Guide")}
          </h2>
          <p className="field-guide__count">{t("{found} of {total} found", { found: met.length, total: FIELD_GUIDE.length })}</p>
        </header>
        <ul className="field-guide__entries">
          {FIELD_GUIDE.map((entry) => {
            const found = met.includes(entry.id);
            const lesson = lessonById(entry.lesson);
            return (
              <li
                key={entry.id}
                className="field-guide__entry"
                data-testid={tutorialTestid.fieldGuideEntry(entry.id)}
                data-found={found ? "true" : "false"}
              >
                {found ? (
                  <>
                    <strong className="field-guide__name">{t(entry.name)}</strong>
                    <span className="field-guide__rule">{ruleSentence(entry)}</span>
                  </>
                ) : (
                  <>
                    <strong className="field-guide__name" aria-label={t("Not found yet")}>
                      ?
                    </strong>
                    <span className="field-guide__rule">
                      {lesson === undefined
                        ? t("Not found yet")
                        : t("Waiting in lesson {number}: {title}", { number: lesson.number, title: t(lesson.title) })}
                    </span>
                  </>
                )}
              </li>
            );
          })}
        </ul>
        <div className="field-guide__actions">
          <button ref={close} type="button" className="field-guide__close" data-testid={tutorialTestid.fieldGuideClose} onClick={onClose}>
            {t("Close")}
          </button>
        </div>
      </section>
    </div>
  );
}

/** The button that opens the Field Guide, and the dialog while it is open. */
export function FieldGuideButton({ testid, compact = false }: { testid: string; compact?: boolean }): ReactElement {
  useLocale();
  const progress = useTutorialProgress();
  const met = fieldGuideOf(progress);
  const [open, setOpen] = useState(false);
  const button = useRef<HTMLButtonElement>(null);
  const label = t("Field Guide {found}/{total}", { found: met.length, total: FIELD_GUIDE.length });
  return (
    <>
      <button
        ref={button}
        type="button"
        className="field-guide-button"
        data-testid={testid}
        aria-haspopup="dialog"
        aria-label={label}
        onClick={() => {
          setOpen(true);
        }}
      >
        <span aria-hidden="true">{compact ? t("Guide") : label}</span>
      </button>
      {/* On the page itself: the HUD it opens from is a containing block that would clip it. */}
      {open
        ? createPortal(
            <FieldGuideDialog
              met={met}
              onClose={() => {
                setOpen(false);
                button.current?.focus();
              }}
            />,
            document.body,
          )
        : null}
    </>
  );
}
