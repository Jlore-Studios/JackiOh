// Node stand-in for a queued client's poll (R897000; e2e/README.md). The queue cancels a ticket
// nobody has heard from for `QUEUE_TICKET_STALE_MS`, and a real client sends `GET /api/auth/me`
// every `SERIES_POLL_SECONDS`, so a seat a spec leaves waiting over HTTP must send it too. The poll
// ends once the server stops naming the ticket; `before:spec`/`after:spec` clear the rest.

export type StayQueuedCommand = {
  /** The API's origin (`server.http()`). */
  url: string;
  token: string;
  /** The client's own cadence: `SERIES_POLL_SECONDS` in ms. */
  everyMs: number;
};

/** A bound on the loop, not a rule: a ticket never paired or canceled (a spec failed half-way) keeps no poll running all run. */
const GIVE_UP_AFTER_MS = 180_000;

const stops = new Map<string, () => void>();

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

/** Whether the server still holds this account's ticket; a read that fails says nothing either way. */
async function ticketOpen(url: string, token: string): Promise<boolean> {
  try {
    const response = await fetch(`${url}/api/auth/me`, { headers: { authorization: `Bearer ${token}` } });
    const body: unknown = await response.json();
    return !isRecord(body) || body.queued !== false;
  } catch {
    return true;
  }
}

/** Poll as `command.token` until its ticket is gone; the answer is `null`, as a Cypress task needs. */
export function stayQueued(command: StayQueuedCommand): null {
  const { url, token, everyMs } = command;
  stops.get(token)?.();
  const giveUpAt = Date.now() + GIVE_UP_AFTER_MS;
  let live = true;
  let timer: NodeJS.Timeout | undefined;
  const poll = async (): Promise<void> => {
    const open = await ticketOpen(url, token);
    if (!live) return;
    if (!open || Date.now() >= giveUpAt) {
      stops.delete(token);
      return;
    }
    timer = setTimeout(() => void poll(), everyMs);
  };
  stops.set(token, () => {
    live = false;
    clearTimeout(timer);
  });
  timer = setTimeout(() => void poll(), everyMs);
  return null;
}

/** Every poll still running, ended: each spec file starts and ends with none. */
export function stopQueued(): null {
  for (const stop of stops.values()) stop();
  stops.clear();
  return null;
}
