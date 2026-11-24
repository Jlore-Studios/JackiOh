// R4691001: whether anybody is using the tab. A tab with no input for `IDLE_QUIET_MS` is idle, and the
// reads and sockets that only keep a screen fresh (`pollWhileVisible`, `quietWhileAway` in `net.ts`) treat
// it as they treat a hidden one, so a tab left open on a screen nobody uses lets its server sleep.
// Presentation only (CLAUDE.md rule 7): the client asks for less and decides nothing.
//
// One watch for the whole page, started by its first listener and stopped with its last. Input is a
// pointer pressed or moved, a key, a wheel or a touch; a tab turning visible counts too, since somebody
// just looked at it. The listeners hear why a tab came back, so a reader that `pageWake` already wakes
// for a tab turning visible does not read twice.

import { IDLE_QUIET_MS } from "../together/config.ts";

/** What ended an idle spell: input on the page, or the tab turning visible. */
export type ActiveCause = "input" | "visible";

/** `idle` true once the tab has gone `IDLE_QUIET_MS` with no input; false, with its cause, when it ends. */
export type IdleListener = (idle: boolean, cause: ActiveCause | null) => void;

const INPUT_EVENTS = ["pointerdown", "pointermove", "keydown", "wheel", "touchstart"] as const;

const listeners = new Set<IdleListener>();
let lastInputAt = 0;
let idle = false;
let timer: ReturnType<typeof setTimeout> | null = null;

function tell(next: boolean, cause: ActiveCause | null): void {
  for (const listener of [...listeners]) {
    try {
      listener(next, cause);
    } catch {
      // A throwing listener must not stop the others.
    }
  }
}

/** Arms the check for the moment the last input turns `IDLE_QUIET_MS` old. */
function arm(): void {
  if (timer !== null) clearTimeout(timer);
  timer = setTimeout(check, Math.max(0, lastInputAt + IDLE_QUIET_MS - Date.now()));
}

function check(): void {
  timer = null;
  if (Date.now() - lastInputAt < IDLE_QUIET_MS) {
    arm();
    return;
  }
  idle = true;
  tell(true, null);
}

/** Notes activity; an idle tab comes back. The timer already running re-arms itself when it fires. */
function active(cause: ActiveCause): void {
  lastInputAt = Date.now();
  if (!idle) return;
  idle = false;
  arm();
  tell(false, cause);
}

const onInput = (): void => {
  active("input");
};

const onVisibility = (): void => {
  if (document.visibilityState === "visible") active("visible");
};

function start(): void {
  lastInputAt = Date.now();
  idle = false;
  for (const name of INPUT_EVENTS) window.addEventListener(name, onInput, { capture: true, passive: true });
  document.addEventListener("visibilitychange", onVisibility);
  arm();
}

function stop(): void {
  for (const name of INPUT_EVENTS) window.removeEventListener(name, onInput, { capture: true });
  document.removeEventListener("visibilitychange", onVisibility);
  if (timer !== null) clearTimeout(timer);
  timer = null;
  idle = false;
}

/**
 * R4691001: calls `listener(true, null)` once the tab has gone `IDLE_QUIET_MS` with no input, and
 * `listener(false, cause)` when input or the tab turning visible ends that. A watch that starts counts
 * from its start. Returns what stops listening.
 */
export function watchIdle(listener: IdleListener): () => void {
  if (typeof window === "undefined" || typeof document === "undefined") return () => undefined;
  if (listeners.size === 0) start();
  listeners.add(listener);
  return () => {
    if (!listeners.delete(listener)) return;
    if (listeners.size === 0) stop();
  };
}

/** R4691001: whether the tab is idle now (false while nothing watches). */
export function tabIdle(): boolean {
  return idle;
}
