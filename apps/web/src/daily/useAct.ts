// One request at a time from a daily screen's controls: the games' list (`DailyRoute.tsx`) and the
// notification settings (`NotificationSettings.tsx`). While one runs the others wait, and its failure is
// the screen's one error.

import { useState } from "react";

import { attempt, messageOf } from "../net/errors.ts";

/**
 * `act(work)` runs `work` unless one is already running, clearing the last error first and keeping the
 * new one; `onSettled` runs once it has settled either way.
 */
export function useAct(onSettled?: () => void): { busy: boolean; error: string | null; act: (work: () => Promise<void>) => void } {
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const act = (work: () => Promise<void>): void => {
    if (busy) return;
    setBusy(true);
    setError(null);
    attempt(work)
      .then(
        () => undefined,
        (cause: unknown) => {
          setError(messageOf(cause));
        },
      )
      .finally(() => {
        setBusy(false);
        onSettled?.();
      });
  };
  return { busy, error, act };
}
