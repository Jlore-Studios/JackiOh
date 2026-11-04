// A watched game's turn clock (#2974): the friend's seat's `clockMs` from the newest delayed frame, counted
// down from the moment that frame arrived, so the number moves between frames instead of freezing until the
// friend acts. It is read-only and silent: no `useTurnClockAlarm`, no haptic, no `../audio` (the player's
// `Clock.tsx` sounds the alarm for the viewer's own seat, and a watcher's "you" is the friend). It decides
// nothing; the server runs every clock (CLAUDE.md rule 7).

import { useEffect, useState, type ReactElement } from "react";

import { secondsLeft } from "../game/clockUrgency.ts";
import { defaultMonotonic } from "../game/net.ts";
import { formatSeconds, useLocale } from "../i18n/i18n.ts";
import { WATCH_CLOCK_TICK_MS } from "./config.ts";
import { watchTestid } from "./testids.ts";

/** A frame's `clockMs` and the monotonic reading when the frame arrived; a new frame is a new reading. */
export type WatchClockReading = { ms: number; at: number };

/** The reading a frame's `clockMs` makes now; none when the frame carries no turn clock. */
export function watchClockOf(clockMs: number | null): WatchClockReading | null {
  return clockMs === null ? null : { ms: clockMs, at: defaultMonotonic() };
}

/** The friend's turn clock, for the board's End turn housing (`Game`'s `turnClock`, the co-op chrome's `clock`). */
export function WatchClock({ reading }: { reading: WatchClockReading | null }): ReactElement | null {
  useLocale();
  const [, setTick] = useState(0);
  useEffect(() => {
    if (reading === null) return undefined;
    const id = window.setInterval(() => {
      setTick((tick) => tick + 1);
    }, WATCH_CLOCK_TICK_MS);
    return () => {
      window.clearInterval(id);
    };
  }, [reading]);
  if (reading === null) return null;
  const left = Math.max(0, reading.ms - Math.max(0, defaultMonotonic() - reading.at));
  return (
    <span className="clock turn-mechanism-clock" data-testid={watchTestid.clock} data-remaining-ms={String(Math.round(left))}>
      {formatSeconds(secondsLeft(left))}
    </span>
  );
}
