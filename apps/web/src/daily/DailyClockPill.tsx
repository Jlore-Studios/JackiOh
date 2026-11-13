// A daily game's clock (#3727), in the board's End turn housing: whoever holds the move and how long it has
// left, in days and hours, in the urgent color under `DAILY_URGENT_HOURS`. The holds and their deadlines
// are the server's (CLAUDE.md rule 7); during the mulligan both seats hold one, and the viewer's own shows.

import { useEffect, useState, type ReactElement } from "react";

import type { PlayerId } from "@jackioh/shared";

import { t } from "../i18n/i18n.ts";
import type { DailyHold } from "../wire/generated/DailyHold.ts";
import { DAILY_TICK_MS } from "./config.ts";
import { isUrgent, timeLeftText } from "./order.ts";
import { dailyTestid } from "./testids.ts";

import "./daily.css";

/** The time now, drawn again every `everyMs`: the time left counts hours, so the pace can be slow. */
export function useNow(everyMs: number = DAILY_TICK_MS): number {
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    const timer = window.setInterval(() => {
      setNow(Date.now());
    }, everyMs);
    return () => {
      window.clearInterval(timer);
    };
  }, [everyMs]);
  return now;
}

/** "Your move · 1d 4h left", "Their move · 5h left", or the move's time run out. */
export function moveLine(mine: boolean, left: number): string {
  return t(mine ? "Your move · {left}" : "Their move · {left}", { left: timeLeftText(left) });
}

export function DailyClockPill({ holds, seat, now }: { holds: readonly DailyHold[]; seat: PlayerId; now: number }): ReactElement | null {
  const mine = holds.find((hold) => hold.player === seat);
  const shown = mine ?? holds[0];
  if (shown === undefined) return null;
  const left = shown.deadline - now;
  return (
    <span
      className="daily-clock"
      data-testid={dailyTestid.clock}
      data-holder={mine === undefined ? "opponent" : "you"}
      data-urgent={isUrgent(left) ? "true" : "false"}
      role="timer"
    >
      {moveLine(mine !== undefined, left)}
    </span>
  );
}
