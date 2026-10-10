// Browser session and pending-auth storage.
// CLAUDE.md rule 7: bearer tokens are not game state, so browser storage stays in this module.
// Per-tab `sessionStorage` limits token persistence (R632); migrate shared tokens and prefer it to the
// M8 fixture in `localStorage` (e2e/README.md A6). Pending addresses are non-secret; recovery stays per tab (R193).

import { AUTH_PENDING_ADDRESS_TTL_SECONDS } from "@jackioh/server-config";

export const SESSION_STORAGE_KEY = "jackioh.session";

export const E2E_SESSION_STORAGE_KEY = "jackioh.e2e.session";

function tabStorage(): Storage | null {
  if (typeof window === "undefined") return null;
  try {
    return window.sessionStorage;
  } catch {
    return null;
  }
}

function sharedStorage(): Storage | null {
  if (typeof window === "undefined") return null;
  try {
    return window.localStorage;
  } catch {
    return null;
  }
}

function read(store: Storage | null, key: string): string | null {
  try {
    return store?.getItem(key) ?? null;
  } catch {
    // Storage is unavailable.
    return null;
  }
}

function remove(store: Storage | null, key: string): void {
  try {
    store?.removeItem(key);
  } catch {
    // Storage is unavailable.
  }
}

export type Session = {
  accessToken: string;
  refreshToken?: string | null;
  expiresAt?: number | null;
};

/** Untrusted storage must not crash boot or become a session without a non-empty token. */
function parse(raw: string | null): Session | null {
  if (raw === null) return null;
  let value: unknown;
  try {
    value = JSON.parse(raw);
  } catch {
    return null;
  }
  if (typeof value !== "object" || value === null) return null;
  const token = (value as { accessToken?: unknown }).accessToken;
  if (typeof token !== "string" || token.length === 0) return null;
  const session: Session = { accessToken: token };
  const refresh = (value as { refreshToken?: unknown }).refreshToken;
  if (typeof refresh === "string" || refresh === null) session.refreshToken = refresh;
  const expires = (value as { expiresAt?: unknown }).expiresAt;
  if (typeof expires === "number" || expires === null) session.expiresAt = expires;
  return session;
}

export function readSession(): Session | null {
  if (typeof window === "undefined") return null;
  const tab = tabStorage();
  const shared = sharedStorage();

  const current = parse(read(tab, SESSION_STORAGE_KEY));
  if (current !== null) return current;

  // Migrate a shared session to this tab.
  const older = parse(read(shared, SESSION_STORAGE_KEY));
  if (older !== null) {
    try {
      tab?.setItem(SESSION_STORAGE_KEY, JSON.stringify(older));
      remove(shared, SESSION_STORAGE_KEY);
    } catch {
      // Keep this read's session if storage is unavailable.
    }
    return older;
  }

  return parse(read(shared, E2E_SESSION_STORAGE_KEY));
}

export function writeSession(session: Session): void {
  if (typeof window === "undefined") return;
  try {
    tabStorage()?.setItem(SESSION_STORAGE_KEY, JSON.stringify(session));
  } catch {
    // The caller keeps the session until the page is left.
    return;
  }
  // R632: shared tokens must not remain there.
  remove(sharedStorage(), SESSION_STORAGE_KEY);
}

export function clearSession(): void {
  if (typeof window === "undefined") return;
  remove(tabStorage(), SESSION_STORAGE_KEY);
  remove(sharedStorage(), SESSION_STORAGE_KEY);
  remove(sharedStorage(), E2E_SESSION_STORAGE_KEY);
}

// Pending sign-up and reset addresses (R193).
// Sign-up hints never authenticate; the reset address prevents recovery-link account swaps. Both expire with
// `AUTH_PENDING_ADDRESS_TTL_SECONDS` (R192) and clear on sign-in or sign-out.

export const PENDING_EMAIL_STORAGE_KEY = "jackioh.auth.pendingEmail";

/** Guards recovery links against account swaps. */
export const PENDING_RESET_STORAGE_KEY = "jackioh.auth.pendingReset";

type RememberedAddress = { address: string; at: number };

function rememberAddress(key: string, email: string): void {
  if (typeof window === "undefined") return;
  const trimmed = email.trim();
  if (trimmed.length === 0) return;
  const value: RememberedAddress = { address: trimmed, at: Date.now() };
  try {
    window.localStorage.setItem(key, JSON.stringify(value));
  } catch {
    // Blocked storage leaves the sign-in path intact.
  }
}

export type PendingAddress = { readonly address: string; readonly at: number };

/** Reject missing, expired, or malformed remembered addresses. */
function rememberedEntry(key: string): PendingAddress | null {
  if (typeof window === "undefined") return null;
  let raw: string | null;
  try {
    raw = window.localStorage.getItem(key);
  } catch {
    return null;
  }
  if (raw === null) return null;
  let value: unknown;
  try {
    value = JSON.parse(raw);
  } catch {
    return null;
  }
  if (typeof value !== "object" || value === null) return null;
  const { address, at } = value as { address?: unknown; at?: unknown };
  if (typeof address !== "string" || typeof at !== "number" || !Number.isFinite(at)) return null;
  const age = Date.now() - at;
  if (age < 0 || age > AUTH_PENDING_ADDRESS_TTL_SECONDS * 1000) {
    forgetAddress(key);
    return null;
  }
  const trimmed = address.trim();
  return trimmed.length === 0 ? null : { address: trimmed, at };
}

function rememberedAddress(key: string): string | null {
  return rememberedEntry(key)?.address ?? null;
}

function forgetAddress(key: string): void {
  if (typeof window === "undefined") return;
  try {
    window.localStorage.removeItem(key);
  } catch {
    // Storage is unavailable.
  }
}

export function rememberPendingEmail(email: string): void {
  rememberAddress(PENDING_EMAIL_STORAGE_KEY, email);
}

export function pendingEmail(): string | null {
  return rememberedAddress(PENDING_EMAIL_STORAGE_KEY);
}

/** Keeps the provider's per-address interval across reloads (R192). */
export function pendingEmailEntry(): PendingAddress | null {
  return rememberedEntry(PENDING_EMAIL_STORAGE_KEY);
}

export function forgetPendingEmail(): void {
  forgetAddress(PENDING_EMAIL_STORAGE_KEY);
}

export function rememberPendingReset(email: string): void {
  rememberAddress(PENDING_RESET_STORAGE_KEY, email);
}

export function pendingReset(): string | null {
  return rememberedAddress(PENDING_RESET_STORAGE_KEY);
}

/** Uses the same provider interval (R192). */
export function pendingResetEntry(): PendingAddress | null {
  return rememberedEntry(PENDING_RESET_STORAGE_KEY);
}

export function forgetPendingReset(): void {
  forgetAddress(PENDING_RESET_STORAGE_KEY);
}

/** R193: an address left armed could accept someone else's link. */
export function forgetPendingAddresses(): void {
  forgetPendingEmail();
  forgetPendingReset();
}
