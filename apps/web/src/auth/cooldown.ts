// A wait counted on the clock, not in timer ticks (R192's per-address mail interval, the gate's stated
// waits): a background tab's timers are frozen (iOS Safari) or throttled, so a wait is a DEADLINE (epoch
// ms) and the seconds left are read from the clock at every render; the interval only asks for a re-render.

import { useCallback, useEffect, useState } from "react";

/** Unit conversion, not configuration. */
const MS_PER_SECOND = 1000;

export function secondsUntil(deadline: number | null, now: number): number {
  if (deadline === null || !Number.isFinite(deadline)) return 0;
  return Math.max(0, Math.ceil((deadline - now) / MS_PER_SECOND));
}

export function deadlineAfter(seconds: number, from: number): number {
  return from + Math.max(0, seconds) * MS_PER_SECOND;
}

/** Re-renders once a second while `deadline` lies ahead, and at once when the page is shown again. */
export function useSecondsUntil(deadline: number | null): number {
  const [, rerender] = useState(0);
  const left = secondsUntil(deadline, Date.now());
  const running = left > 0;

  useEffect(() => {
    if (!running || deadline === null) return;
    const refresh = (): void => {
      rerender((count) => count + 1);
    };
    const timer = window.setInterval(() => {
      refresh();
      if (Date.now() >= deadline) window.clearInterval(timer);
    }, MS_PER_SECOND);
    document.addEventListener("visibilitychange", refresh);
    window.addEventListener("pageshow", refresh);
    window.addEventListener("focus", refresh);
    return () => {
      window.clearInterval(timer);
      document.removeEventListener("visibilitychange", refresh);
      window.removeEventListener("pageshow", refresh);
      window.removeEventListener("focus", refresh);
    };
  }, [deadline, running]);

  return left;
}

/** An address as the auth provider compares it: trimmed, case-insensitive. */
export function addressKey(address: string): string {
  return address.trim().toLowerCase();
}

export type AddressCooldown = {
  /** Whole seconds left before the mailer may be used again for `address`; 0 for any other. */
  secondsFor: (address: string) => number;
  startUntil: (address: string, deadline: number) => void;
};

/**
 * R192: the provider's per-address mail interval, for the one address last mailed (a corrected
 * address can be sent to at once). `initial` is one already running when the screen opens.
 */
export function useAddressCooldown(initial: { address: string; deadline: number } | null = null): AddressCooldown {
  const [state, setState] = useState<{ key: string; deadline: number } | null>(() =>
    initial === null ? null : { key: addressKey(initial.address), deadline: initial.deadline },
  );
  const left = useSecondsUntil(state?.deadline ?? null);

  const secondsFor = useCallback(
    (address: string): number => (state !== null && state.key === addressKey(address) ? left : 0),
    [state, left],
  );
  const startUntil = useCallback((address: string, deadline: number): void => {
    setState({ key: addressKey(address), deadline });
  }, []);

  return { secondsFor, startUntil };
}

/** Seconds left of a stated wait, from when it was stated; 0 when there is none. */
export function useCountdown(totalMs: number | undefined): number {
  const [deadline, setDeadline] = useState(() => (totalMs === undefined ? null : Date.now() + totalMs));
  const [stated, setStated] = useState(totalMs);
  if (stated !== totalMs) {
    // A new wait was stated: counted from now.
    setStated(totalMs);
    setDeadline(totalMs === undefined ? null : Date.now() + totalMs);
  }
  return useSecondsUntil(deadline);
}
