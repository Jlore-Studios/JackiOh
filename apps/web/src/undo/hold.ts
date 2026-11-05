// R2119000: a deletion is undone, not asked. Deleting a deck or a trio, or removing a
// friend, takes the item off the screen at once and holds its request for `UNDO_WINDOW_MS` behind the
// undo toast (`UndoToast.tsx`), one at a time. A second deletion within `UNDO_REPEAT_MS` of the first is a
// double click and is refused (R3059000). Nothing here is a rule (CLAUDE.md rule 7): the server still
// decides every deletion.

import { useCallback, useEffect, useRef, useState } from "react";

/** How long a deletion waits, its Undo offered, before its request goes. */
export const UNDO_WINDOW_MS = 8000;

/** A deletion this soon after the last one taken is a double click, and is refused (R3059000). */
export const UNDO_REPEAT_MS = 500;

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
  /**
   * Holds a deletion, sending any held before it at once, and returns `true`. Returns `false`, sending and
   * holding nothing, when it comes within `UNDO_REPEAT_MS` of the last one taken: the caller then leaves
   * its item where it is.
   */
  hold: (deletion: HeldDeletion) => boolean;
  undo: () => void;
  /** Sends the held deletion at once, before something that needs it gone (a new deck at the cap). */
  sendNow: () => void;
};

export function useUndoHold(timer: UndoTimer = BROWSER_TIMER): UndoHold {
  const [held, setHeld] = useState<HeldDeletion | null>(null);
  const current = useRef<HeldDeletion | null>(null);
  const handle = useRef<unknown>(null);
  /** The repeat guard's timer: non-null while a deletion was just taken. */
  const repeat = useRef<unknown>(null);
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
    if (repeat.current !== null) {
      timerRef.current.clearTimeout(repeat.current);
      repeat.current = null;
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
    (deletion: HeldDeletion): boolean => {
      if (repeat.current !== null) return false;
      send(false);
      current.current = deletion;
      setHeld(deletion);
      handle.current = timerRef.current.setTimeout(() => {
        handle.current = null;
        send(false);
      }, UNDO_WINDOW_MS);
      // A refused call leaves this timer as it is, so a third press after the guard ends is taken.
      repeat.current = timerRef.current.setTimeout(() => {
        repeat.current = null;
      }, UNDO_REPEAT_MS);
      return true;
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
