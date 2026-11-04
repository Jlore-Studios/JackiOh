// `googleIdentity.ts` (R2983000): Google Identity Services' one script, added only when asked for and
// tried again after a failure, and the nonce each attempt is armed with.

import { afterEach, describe, expect, it, vi } from "vitest";

import { GIS_SCRIPT_URL, gisApi, loadGoogleIdentity, newGoogleNonce } from "./googleIdentity.ts";

function scripts(): HTMLScriptElement[] {
  return [...document.querySelectorAll<HTMLScriptElement>(`script[src="${GIS_SCRIPT_URL}"]`)];
}

/** A stand-in for `google.accounts.id`, as GIS defines it once loaded. */
function stubGis(): { initialize: () => void; renderButton: () => void } {
  const id = { initialize: vi.fn(), renderButton: vi.fn() };
  vi.stubGlobal("google", { accounts: { id } });
  return id;
}

async function hex(raw: string): Promise<string> {
  const digest = await crypto.subtle.digest("SHA-256", new TextEncoder().encode(raw));
  return [...new Uint8Array(digest)].map((byte) => byte.toString(16).padStart(2, "0")).join("");
}

afterEach(() => {
  vi.unstubAllGlobals();
  // A load left in flight settles, so the next test starts with none.
  for (const script of scripts()) {
    script.dispatchEvent(new Event("error"));
    script.remove();
  }
});

describe("R2983000 Google Identity Services' script", () => {
  it("R2983000 adds GIS's one script only when asked, once while it loads", async () => {
    expect(scripts()).toHaveLength(0);
    expect(gisApi()).toBeNull();
    const first = loadGoogleIdentity();
    const second = loadGoogleIdentity();
    expect(scripts()).toHaveLength(1);
    const [script] = scripts();
    expect(script?.async).toBe(true);
    expect(script?.parentElement).toBe(document.head);

    const id = stubGis();
    script?.dispatchEvent(new Event("load"));
    await expect(first).resolves.toBe(id);
    await expect(second).resolves.toBe(id);
    // Loaded: the next caller gets it at once, and no second script is added.
    await expect(loadGoogleIdentity()).resolves.toBe(id);
    expect(scripts()).toHaveLength(1);
  });

  it("R2983000 a script that fails, or loads without google.accounts.id, rejects and the next call tries again", async () => {
    const failed = loadGoogleIdentity();
    scripts()[0]?.dispatchEvent(new Event("error"));
    await expect(failed).rejects.toThrow("GIS did not load");
    // The failed script is gone, so a later attempt adds its own.
    expect(scripts()).toHaveLength(0);

    const empty = loadGoogleIdentity();
    expect(scripts()).toHaveLength(1);
    vi.stubGlobal("google", { accounts: {} });
    scripts()[0]?.dispatchEvent(new Event("load"));
    await expect(empty).rejects.toThrow("GIS loaded without google.accounts.id");
    expect(gisApi()).toBeNull();

    const again = loadGoogleIdentity();
    expect(scripts()).toHaveLength(2);
    const id = stubGis();
    scripts()[1]?.dispatchEvent(new Event("load"));
    await expect(again).resolves.toBe(id);
  });

  it("R2983000 reads google.accounts.id as untrusted: both calls must be functions", () => {
    vi.stubGlobal("google", { accounts: { id: { initialize: vi.fn(), renderButton: "no" } } });
    expect(gisApi()).toBeNull();
    vi.stubGlobal("google", "not an object");
    expect(gisApi()).toBeNull();
    const id = stubGis();
    expect(gisApi()).toBe(id);
  });
});

describe("R2983000 the nonce of each attempt", () => {
  it("R2983000 a nonce is fresh each time and GIS gets its SHA-256 in hex", async () => {
    const first = await newGoogleNonce();
    const second = await newGoogleNonce();
    for (const nonce of [first, second]) {
      expect(nonce.raw).toMatch(/^[A-Za-z0-9_-]{86}$/u);
      expect(nonce.hashed).toMatch(/^[0-9a-f]{64}$/u);
      expect(nonce.hashed).toBe(await hex(nonce.raw));
    }
    expect(second.raw).not.toBe(first.raw);
    expect(second.hashed).not.toBe(first.hashed);
  });

  it("R2983000 no nonce without the platform's hash", async () => {
    vi.stubGlobal("crypto", { getRandomValues: (bytes: Uint8Array) => bytes, subtle: undefined });
    await expect(newGoogleNonce()).rejects.toThrow();
  });
});
