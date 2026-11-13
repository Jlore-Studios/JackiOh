// The queue search in the shell's status row (R3728002, R3729001): this tab's mode, the time it has waited as a
// ticking m:ss clock, the expected wait the server counted for that mode, and Cancel. It shows on every menu
// screen, `/play` included, so a player who goes looking for something else while they wait never loses it.
//
// CLAUDE.md rule 7: nothing here is a rule. The mode and the moment are what this tab remembers
// (`net/liveGame.ts`), the expected wait what the server answered (`net/queueReadout.ts`), and Cancel is
// intent: the server still decides, and a refusal keeps the search on screen.

import { useEffect, useState, type ReactElement } from "react";

import { dequeue, type QueueMode } from "../net/api.ts";
import { attempt } from "../net/errors.ts";
import { MODE_LABEL, forgetQueued, readQueuedSince } from "../net/liveGame.ts";
import { useQueueReadout, waitWords } from "../net/queueReadout.ts";
import { t } from "../i18n/i18n.ts";
import { GameBanner } from "../routes/GameBanner.tsx";
import { menuShellTestid } from "./testids.ts";

/** How often the clock is drawn again, in milliseconds: it counts whole seconds. */
export const QUEUE_CLOCK_TICK_MS = 1000;

/** Unit conversion, not configuration. */
const MS_PER_SECOND = 1000;
const SECONDS_PER_MINUTE = 60;
/** Digits a seconds field is padded to: "1:05". */
const SECONDS_DIGITS = 2;

/** R3729001: `elapsedMs` as minutes and whole seconds, floored and never below zero: "0:00", "1:05", "62:05". */
export function queueClock(elapsedMs: number): string {
  const total = Math.max(0, Math.floor(elapsedMs / MS_PER_SECOND));
  const minutes = Math.floor(total / SECONDS_PER_MINUTE);
  const seconds = total % SECONDS_PER_MINUTE;
  return `${String(minutes)}:${String(seconds).padStart(SECONDS_DIGITS, "0")}`;
}

/** The time now, drawn again every `QUEUE_CLOCK_TICK_MS`. */
function useClock(): number {
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    const timer = window.setInterval(() => {
      setNow(Date.now());
    }, QUEUE_CLOCK_TICK_MS);
    return () => {
      window.clearInterval(timer);
    };
  }, []);
  return now;
}

/** R3728002, R3729001: this tab's queue search, and the way out of it. */
export function QueueBanner({ mode, token }: { mode: QueueMode; token: string }): ReactElement {
  const [leaving, setLeaving] = useState(false);
  // Storage that kept no moment (a tab that queued before this was written, or blocked storage) counts from here.
  const [mounted] = useState(() => Date.now());
  const now = useClock();
  const since = readQueuedSince() ?? mounted;
  const readout = useQueueReadout(token, true);
  const expected = readout?.expectedWaitSeconds?.[mode] ?? null;

  const onPress = (): void => {
    if (leaving) return;
    setLeaving(true);
    // A refusal keeps the search on screen, its button pressable again; `/play` says why.
    attempt(() => dequeue(token))
      .then(forgetQueued, () => undefined)
      .finally(() => {
        setLeaving(false);
      });
  };
  return (
    <GameBanner
      testId={menuShellTestid.queue}
      title="You're in the queue"
      action={{ label: "Cancel", testId: menuShellTestid.queueLeave, onPress }}
      data={{ mode }}
    >
      {t("Looking for a {mode} opponent…", { mode: t(MODE_LABEL[mode]) })}{" "}
      <span data-testid={menuShellTestid.queueClock}>{queueClock(now - since)}</span>
      {expected === null ? null : (
        <>
          {" · "}
          <span data-testid={menuShellTestid.queueWait}>{waitWords(expected)}</span>
        </>
      )}
    </GameBanner>
  );
}
