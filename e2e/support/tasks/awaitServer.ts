// Node-side wait on the E2E server's own answer (spec 49): `GET <url><path>` as `token`, every
// `everyMs`, until the JSON at `key` (a dot path) is set, a non-empty string or a number above 0,
// then that body; past `timeoutMs`, the last body with `timedOut`. No fixed wait in a spec
// (e2e/README.md): the wait is the server's answer, as `stayQueued` polls it.

export type AwaitServerCommand = {
  /** The API's origin (`server.http()`). */
  url: string;
  path: string;
  token: string;
  /** A dot path into the answer, e.g. `currentMatchId` or `record.games`. */
  key: string;
  timeoutMs: number;
  everyMs: number;
};

export type AwaitServerResult = { body: unknown; timedOut: boolean };

/** The value at a dot path, or undefined. */
export function at(body: unknown, key: string): unknown {
  let value: unknown = body;
  for (const part of key.split(".")) {
    if (typeof value !== "object" || value === null) return undefined;
    value = (value as Record<string, unknown>)[part];
  }
  return value;
}

/** Whether a polled value says the wait is over. */
export function arrived(value: unknown): boolean {
  if (typeof value === "number") return value > 0;
  if (typeof value === "string") return value.length > 0;
  return value !== undefined && value !== null && value !== false;
}

export async function awaitServer(command: AwaitServerCommand): Promise<AwaitServerResult> {
  const { url, path, token, key, timeoutMs, everyMs } = command;
  const giveUpAt = Date.now() + timeoutMs;
  let last: unknown = null;
  for (;;) {
    try {
      const response = await fetch(`${url}${path}`, { headers: { authorization: `Bearer ${token}` } });
      last = (await response.json()) as unknown;
      if (arrived(at(last, key))) return { body: last, timedOut: false };
    } catch {
      // A read that fails says nothing either way; the next one asks again.
    }
    if (Date.now() >= giveUpAt) return { body: last, timedOut: true };
    await new Promise((resolve) => setTimeout(resolve, everyMs));
  }
}
