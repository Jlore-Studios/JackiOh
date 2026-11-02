// PKCE for the emailed links (R323, R324; an email change's, R663), the email sign-in link (R664) and
// OAuth (R666): only the browser holding the verifier can exchange the returned code, so no token is in a
// URL. Verifiers live in `localStorage`, since a link usually opens in a new tab, which `sessionStorage`
// would not reach; a code with no verifier here was asked for elsewhere (R324).

import { getStored, removeStored, setStored } from "../net/storage.ts";

export const PKCE_STORAGE_KEY = "jackioh.auth.pkce";
/** The stored value's shape version: anything else reads as no verifier. */
const PKCE_STORAGE_VERSION = 1;
/** RFC 7636 §4.1: a verifier is 43 to 128 characters; 64 random bytes are 86 in base64url. */
export const PKCE_VERIFIER_BYTES = 64;
const PKCE_VERIFIER_MIN_LENGTH = 43;
const PKCE_VERIFIER_MAX_LENGTH = 128;
export const PKCE_METHOD = "s256";

/** Link flows (R663, R664, R666); exchange only with this browser's verifier to prevent R193 login CSRF. */
export type PkceFlow = "signup" | "recovery" | "email_change" | "magiclink" | "oauth";

const FLOWS: readonly PkceFlow[] = ["signup", "recovery", "email_change", "magiclink", "oauth"];

type Stored = { verifier: string; at: number };
type StoredValue = { v: number } & Partial<Record<PkceFlow, Stored>>;

export type PkceChallenge = { code_challenge: string; code_challenge_method: typeof PKCE_METHOD };

function base64url(bytes: Uint8Array): string {
  let binary = "";
  for (const byte of bytes) binary += String.fromCharCode(byte);
  return btoa(binary).replace(/\+/gu, "-").replace(/\//gu, "_").replace(/=+$/u, "");
}

const VERIFIER_PATTERN = /^[A-Za-z0-9._~-]+$/u;

function isVerifier(value: unknown): value is string {
  return (
    typeof value === "string" &&
    value.length >= PKCE_VERIFIER_MIN_LENGTH &&
    value.length <= PKCE_VERIFIER_MAX_LENGTH &&
    VERIFIER_PATTERN.test(value)
  );
}

export function newVerifier(): string {
  const bytes = new Uint8Array(PKCE_VERIFIER_BYTES);
  crypto.getRandomValues(bytes);
  return base64url(bytes);
}

/** RFC 7636 §4.2 S256: base64url(SHA-256(verifier)). */
export async function challengeFor(verifier: string): Promise<string> {
  const digest = await crypto.subtle.digest("SHA-256", new TextEncoder().encode(verifier));
  return base64url(new Uint8Array(digest));
}

/** The stored verifiers, read as untrusted input: anything malformed is no verifier. */
function readStored(): StoredValue {
  const empty: StoredValue = { v: PKCE_STORAGE_VERSION };
  const raw = getStored(PKCE_STORAGE_KEY);
  if (raw === null) return empty;
  try {
    const parsed: unknown = JSON.parse(raw);
    if (typeof parsed !== "object" || parsed === null) return empty;
    const record = parsed as Record<string, unknown>;
    if (record.v !== PKCE_STORAGE_VERSION) return empty;
    const value: StoredValue = { v: PKCE_STORAGE_VERSION };
    for (const flow of FLOWS) {
      const entry = record[flow] as { verifier?: unknown; at?: unknown } | undefined;
      if (entry === undefined || entry === null || typeof entry !== "object") continue;
      if (!isVerifier(entry.verifier)) continue;
      value[flow] = { verifier: entry.verifier, at: typeof entry.at === "number" && Number.isFinite(entry.at) ? entry.at : 0 };
    }
    return value;
  } catch {
    return empty;
  }
}

function writeStored(value: StoredValue): void {
  // Blocked storage: the link still confirms the address; the code just cannot be exchanged
  // here, which reads as a link opened elsewhere (R324).
  if (FLOWS.every((flow) => value[flow] === undefined)) removeStored(PKCE_STORAGE_KEY);
  else setStored(PKCE_STORAGE_KEY, JSON.stringify(value));
}

/** A new verifier each time, but a resend of the confirmation reuses the sign-up's, so the first link still works. */
export async function challengeForRequest(flow: PkceFlow, options: { reuse?: boolean } = {}): Promise<PkceChallenge> {
  const kept = options.reuse === true ? readStored()[flow]?.verifier : undefined;
  const verifier = kept ?? newVerifier();
  // Hashed before it is kept: a browser that cannot hash sends no challenge and keeps nothing.
  const challenge = await challengeFor(verifier);
  const stored = readStored();
  stored[flow] = { verifier, at: Date.now() };
  writeStored(stored);
  return { code_challenge: challenge, code_challenge_method: PKCE_METHOD };
}

export function storedVerifiers(): { flow: PkceFlow; verifier: string }[] {
  const stored = readStored();
  return FLOWS.flatMap((flow) => {
    const entry = stored[flow];
    return entry === undefined ? [] : [{ flow, verifier: entry.verifier, at: entry.at }];
  })
    .sort((a, b) => b.at - a.at)
    .map(({ flow, verifier }) => ({ flow, verifier }));
}

export function newestFlow(): PkceFlow | null {
  return storedVerifiers()[0]?.flow ?? null;
}

export function forgetVerifier(flow: PkceFlow): void {
  const stored = readStored();
  delete stored[flow];
  writeStored(stored);
}
