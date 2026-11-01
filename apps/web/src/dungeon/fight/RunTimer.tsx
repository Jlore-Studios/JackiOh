// B24 (DR9, DR14 part E, #2706): the run's time on screen during a fight, read from the moment the
// run started (`Dungeon.startedAt`), so it goes on from fight to fight and over a reload.

import { useEffect, useState, type ReactElement } from "react";

import { MS_PER_SECOND } from "../../game/clockConstants.ts";
import { t } from "../../i18n/i18n.ts";
import { CLOCK_DIGITS, FIGHT_TIMER_TICK_MS, SECONDS_PER_MINUTE } from "./config.ts";
import { fightTestid } from "./testids.ts";
import "./fight.css";

/** `elapsedMs` as minutes and whole seconds, floored and never below zero: "0:00", "1:05", "62:05". */
export function formatRunTime(elapsedMs: number): string {
  const total = Math.max(0, Math.floor(elapsedMs / MS_PER_SECOND));
  const minutes = Math.floor(total / SECONDS_PER_MINUTE);
  const seconds = total % SECONDS_PER_MINUTE;
  return `${String(minutes)}:${String(seconds).padStart(CLOCK_DIGITS, "0")}`;
}

/** The run's time since `startedAt` (`Date.now()` terms); a run with no start reads 0:00. */
export function RunTimer({ startedAt }: { startedAt: number | null }): ReactElement {
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    const timer = window.setInterval(() => {
      setNow(Date.now());
    }, FIGHT_TIMER_TICK_MS);
    return () => {
      window.clearInterval(timer);
    };
  }, []);
  return (
    <span className="run-timer" data-testid={fightTestid.timer}>
      <span className="run-timer__label">{t("Run time")}</span>{" "}
      <span className="run-timer__value">{formatRunTime(startedAt === null ? 0 : now - startedAt)}</span>
    </span>
  );
}
