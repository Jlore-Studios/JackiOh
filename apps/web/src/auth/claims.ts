// A token's claims, decoded without verifying them (the server alone verifies a token).
// It imports nothing, so `net/readCache.ts` can read `sub` without an import cycle (`net/auth.ts` imports `net/api.ts`).

function claimsOf(token: string): Record<string, unknown> | null {
  const parts = token.split(".");
  const payload = parts[1];
  if (payload === undefined || payload.length === 0) return null;
  try {
    const base64 = payload.replace(/-/gu, "+").replace(/_/gu, "/");
    const padded = base64 + "=".repeat((4 - (base64.length % 4)) % 4);
    const binary = atob(padded);
    const bytes = Uint8Array.from(binary, (character) => character.charCodeAt(0));
    const claims: unknown = JSON.parse(new TextDecoder().decode(bytes));
    return typeof claims === "object" && claims !== null ? (claims as Record<string, unknown>) : null;
  } catch {
    return null;
  }
}

function text(value: unknown): string | null {
  return typeof value === "string" && value.length > 0 ? value : null;
}

export function tokenClaim(token: string, name: "email" | "sub" | "session_id"): string | null {
  return text(claimsOf(token)?.[name]);
}

/**
 * A string in one of the token's metadata objects. For a screen's wording or a field's first value only:
 * `user_metadata` is the user's to edit, so nothing here decides anything (the server judges).
 */
export function tokenMetadata(token: string, bag: "app_metadata" | "user_metadata", name: string): string | null {
  const metadata = claimsOf(token)?.[bag];
  if (typeof metadata !== "object" || metadata === null) return null;
  return text((metadata as Record<string, unknown>)[name]);
}
