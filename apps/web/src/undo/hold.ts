// R2119000: a deletion is undone, not asked. Deleting a deck, a trio or a custom card, or removing a
// friend, takes the item off the screen at once and holds its request for `UNDO_WINDOW_MS` behind the
// undo toast (`UndoToast.tsx`), one at a time. Nothing here is a rule (CLAUDE.md rule 7): the server
// still decides every deletion.

import { useCallback, useEffect, useRef, useState } from "react";

/** How long a deletion waits, its Undo offered, before its request goes. */
export const UNDO_WINDOW_MS = 8000;

/** The timers the window runs on; a test hands a manual one (`game/deckbuilder/testkit.ts`). */
export type UndoTimer = {
  setTimeout(run: () => void, ms: number): unknown;
  clearTimeout(handle: unknown): void;
};

const BROWSER_TIMER: UndoTimer = {
  setTimeout: (run, ms) => globalThis.setTimeout(run, ms),
  clearTimeout: (handle) => {
    globalThis.clearTimeout(handle as ReturnType<typeof globalThis.setTimeout>);
  },
};

/** One held deletion, as its screen describes it. */
export type HeldDeletion = {
  /** The toast's words, worded when drawn, so a language switch changes them too. */
  words: () => string;
  /** Sends the request; `keepalive` when the page is going away. A failure is the screen's to show. */
  send: (keepalive: boolean) => void;
  /** Undo: the item comes back where it was, and nothing is sent. */
  restore: () => void;
};

export type UndoHold = {
  /** The deletion the toast offers to undo, if any. */
  held: HeldDeletion | null;
  /** Holds a deletion, sending any held before it at once. */
  hold: (deletion: HeldDeletion) => void;
  undo: () => void;
  /** Sends the held deletion at once, before something that needs it gone (a new deck at the cap). */
  sendNow: () => void;
};

export function useUndoHold(timer: UndoTimer = BROWSER_TIMER): UndoHold {
  const [held, setHeld] = useState<HeldDeletion | null>(null);
  const current = useRef<HeldDeletion | null>(null);
  const handle = useRef<unknown>(null);
  const timerRef = useRef(timer);
  timerRef.current = timer;

  /** Ends the hold, if there is one, and gives back what it held. */
  const release = useCallback((): HeldDeletion | null => {
    const deletion = current.current;
    if (deletion === null) return null;
    if (handle.current !== null) {
      timerRef.current.clearTimeout(handle.current);
      handle.current = null;
    }
    current.current = null;
    setHeld(null);
    return deletion;
  }, []);

  const send = useCallback(
    (keepalive: boolean): void => {
      release()?.send(keepalive);
    },
    [release],
  );

  const hold = useCallback(
    (deletion: HeldDeletion): void => {
      send(false);
      current.current = deletion;
      setHeld(deletion);
      handle.current = timerRef.current.setTimeout(() => {
        handle.current = null;
        send(false);
      }, UNDO_WINDOW_MS);
    },
    [send],
  );

  const undo = useCallback((): void => {
    release()?.restore();
  }, [release]);

  const sendNow = useCallback((): void => {
    send(false);
  }, [send]);

  // The page going away sends at once, as a request that outlives it; leaving the screen sends too.
  useEffect(() => {
    const onPageHide = (): void => {
      send(true);
    };
    const onVisibility = (): void => {
      if (document.visibilityState === "hidden") send(true);
    };
    window.addEventListener("pagehide", onPageHide);
    document.addEventListener("visibilitychange", onVisibility);
    return () => {
      window.removeEventListener("pagehide", onPageHide);
      document.removeEventListener("visibilitychange", onVisibility);
      send(false);
    };
  }, [send]);

  return { held, hold, undo, sendNow };
}
