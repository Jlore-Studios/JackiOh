// Reading untrusted JSON (a server body, a stored value, a bundled manifest) safely.

/** A JSON object: not null, not an array. */
export function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

/** Issue #4015: outside keys that must never become object keys (prototype pollution). */
export const UNSAFE_KEYS: ReadonlySet<string> = new Set(["__proto__", "constructor", "prototype"]);

/** Issue #4015: false for a key parsed from outside that must never become an object key. */
export function isSafeKey(key: string): boolean {
  return !UNSAFE_KEYS.has(key);
}
