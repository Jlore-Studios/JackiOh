// A token's claims, decoded without verifying them (the server alone verifies a token).
// It imports nothing, so `net/readCache.ts` can read `sub` without an import cycle (`net/auth.ts` imports `net/api.ts`).

export function tokenClaim(token: string, name: "email" | "sub" | "session_id"): string | null {
  const parts = token.split(".");
  const payload = parts[1];
  if (payload === undefined || payload.length === 0) return null;
  try {
    const base64 = payload.replace(/-/gu, "+").replace(/_/gu, "/");
    const padded = base64 + "=".repeat((4 - (base64.length % 4)) % 4);
    const binary = atob(padded);
    const bytes = Uint8Array.from(binary, (character) => character.charCodeAt(0));
    const claims: unknown = JSON.parse(new TextDecoder().decode(bytes));
    if (typeof claims !== "object" || claims === null) return null;
    const value = (claims as Record<string, unknown>)[name];
    return typeof value === "string" && value.length > 0 ? value : null;
  } catch {
    return null;
  }
}
