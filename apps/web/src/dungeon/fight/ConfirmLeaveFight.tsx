// "Leave this fight?": what leaving a dungeon run's fight in progress asks, since the fight is
// dealt again from its seed when the run starts it again (R2706000). Staying is the default: it
// has the focus, and Escape, Back (R2127000) or a click outside the panel means stay. Leaving
// makes the move the leave guard held (R1754000). It decides nothing (CLAUDE.md rule 7).
// Modeled on practice's question (PracticeLeave.tsx), minus the Save and leave a dungeon fight
// never has (R2972000).

import { useEffect, useId, useRef, type ReactElement } from "react";

import { t } from "../../i18n/i18n.ts";
import { useBackLayer } from "../../net/backLayers.ts";
import { fightTestid } from "./testids.ts";
import "../../practice/practice.css";

export type ConfirmLeaveFightProps = {
  /** "Keep playing", Escape, Back or a click outside the panel. */
  onStay(): void;
  /** "Leave fight": make the held move. */
  onLeave(): void;
};

export default function ConfirmLeaveFight({ onStay, onLeave }: ConfirmLeaveFightProps): ReactElement {
  const titleId = useId();
  const bodyId = useId();
  const stay = useRef<HTMLButtonElement>(null);
  useBackLayer(onStay);

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
        data-testid={fightTestid.leaveDialog}
        role="alertdialog"
        aria-modal="true"
        aria-labelledby={titleId}
        aria-describedby={bodyId}
      >
        <h2 className="practice-leave__title" id={titleId}>
          {t("Leave this fight?")}
        </h2>
        <p className="practice-leave__body" id={bodyId}>
          {t("It ends here and the fight starts over from its first turn.")}
        </p>
        <div className="practice-leave__actions">
          <button ref={stay} type="button" className="practice-play" data-testid={fightTestid.leaveStay} onClick={onStay}>
            {t("Keep playing")}
          </button>
          <button
            type="button"
            className="practice-result__secondary"
            data-testid={fightTestid.leaveConfirm}
            onClick={onLeave}
          >
            {t("Leave fight")}
          </button>
        </div>
      </section>
    </div>
  );
}
