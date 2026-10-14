// Reading untrusted JSON (a server body, a stored value, a bundled manifest) safely.

/** A JSON object: not null, not an array. */
export function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}
