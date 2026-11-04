// Google Identity Services (GIS), Google's own "Sign in with Google" button, which signs in on this
// site's origin so Google's screen names it (R2983000). Only `/login` uses this module, through
// `GoogleButton.tsx`, and only when the build names Google and its client ID (`googleClientId`): the
// script is added to the page on first use, never by `index.html`. Each attempt has a fresh nonce:
// GIS is given its SHA-256 and the raw nonce stays in memory for the exchange (`signInWithGoogleIdToken`).

import { newVerifier } from "./pkce.ts";

export const GIS_SCRIPT_URL = "https://accounts.google.com/gsi/client";

/** What GIS hands the `callback`: `credential` is the ID token, read as untrusted. */
export type GisCredentialResponse = { credential?: unknown };

export type GisIdConfiguration = {
  client_id: string;
  callback: (response: GisCredentialResponse) => void;
  /** The SHA-256 of the attempt's raw nonce, in hex: the ID token carries it back. */
  nonce: string;
  context: "signin";
  ux_mode: "popup";
  use_fedcm_for_button: true;
};

export type GisButtonConfiguration = {
  type: "standard";
  theme: "outline";
  size: "large";
  text: "continue_with";
  shape: "rectangular";
  logo_alignment: "left";
};

/** Google's button in its light theme, as R666's own Google button was drawn (#1890). */
export const GIS_BUTTON: GisButtonConfiguration = {
  type: "standard",
  theme: "outline",
  size: "large",
  text: "continue_with",
  shape: "rectangular",
  logo_alignment: "left",
};

/** The part of `google.accounts.id` this client calls. */
export type GisId = {
  initialize(config: GisIdConfiguration): void;
  renderButton(parent: HTMLElement, options: GisButtonConfiguration): void;
};

function field(value: unknown, key: string): unknown {
  return typeof value === "object" && value !== null ? (value as Record<string, unknown>)[key] : undefined;
}

/** `google.accounts.id` once GIS has loaded, read as untrusted: null unless it has both calls. */
export function gisApi(): GisId | null {
  const id = field(field((globalThis as { google?: unknown }).google, "accounts"), "id");
  if (typeof field(id, "initialize") !== "function" || typeof field(id, "renderButton") !== "function") return null;
  return id as GisId;
}

/** The load in flight, shared by every caller; null once it has settled, so a failed load is tried again. */
let loading: Promise<GisId> | null = null;

/** GIS's one script, added on first use; rejects when it does not load or loads without `google.accounts.id`. */
export function loadGoogleIdentity(): Promise<GisId> {
  const ready = gisApi();
  if (ready !== null) return Promise.resolve(ready);
  loading ??= new Promise<GisId>((resolve, reject) => {
    const script = document.createElement("script");
    script.src = GIS_SCRIPT_URL;
    script.async = true;
    script.addEventListener("load", () => {
      loading = null;
      const api = gisApi();
      if (api === null) reject(new Error("GIS loaded without google.accounts.id"));
      else resolve(api);
    });
    script.addEventListener("error", () => {
      loading = null;
      script.remove();
      reject(new Error("GIS did not load"));
    });
    document.head.append(script);
  });
  return loading;
}

/** Unit conversion, not configuration. */
const HEX_RADIX = 16;
/** Unit conversion, not configuration. */
const HEX_DIGITS_PER_BYTE = 2;

/** One attempt's nonce: `raw` goes to the exchange, `hashed` (its SHA-256 in hex) to GIS. */
export type GoogleNonce = { raw: string; hashed: string };

/** A fresh nonce from the platform's randomness; rejects in a browser that cannot hash, which GIS then counts as unable. */
export async function newGoogleNonce(): Promise<GoogleNonce> {
  const raw = newVerifier();
  const digest = await crypto.subtle.digest("SHA-256", new TextEncoder().encode(raw));
  const hashed = Array.from(new Uint8Array(digest), (byte) =>
    byte.toString(HEX_RADIX).padStart(HEX_DIGITS_PER_BYTE, "0"),
  ).join("");
  return { raw, hashed };
}
