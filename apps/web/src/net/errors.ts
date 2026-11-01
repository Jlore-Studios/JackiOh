// What a failure says, whether it was a cancel, and a call whose throw becomes a rejection: one copy
// for every screen and worker (#2160). It imports nothing, so the practice worker, `net/` and a
// screen that loads its heavy half lazily take it without React or that half.

/** A failure's sentence: an `Error`'s message (the server's own for `ApiRequestError`), anything else as text. */
export function messageOf(cause: unknown): string {
  return cause instanceof Error ? cause.message : String(cause);
}

/** Whether a failure is a cancel: the `AbortError` a cancelled fetch, share or export throws. */
export function isAbort(cause: unknown): boolean {
  return cause instanceof DOMException && cause.name === "AbortError";
}

/** Runs `call` and turns a synchronous throw into a rejected promise. */
export function attempt<T>(call: () => Promise<T>): Promise<T> {
  return Promise.resolve().then(call);
}
