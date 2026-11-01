// What a failure says, and a call whose throw becomes a rejection: one copy for every screen and
// worker (#2160). It imports nothing, so the practice worker and `net/` take it without React.

/** A failure's sentence: an `Error`'s message (the server's own for `ApiRequestError`), anything else as text. */
export function messageOf(cause: unknown): string {
  return cause instanceof Error ? cause.message : String(cause);
}

/** Runs `call` and turns a synchronous throw into a rejected promise. */
export function attempt<T>(call: () => Promise<T>): Promise<T> {
  return Promise.resolve().then(call);
}
