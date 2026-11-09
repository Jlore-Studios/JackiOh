// R3349000: the new-level marks are keyed by the token's non-secret `sub`, never by the token itself.

import { afterEach, beforeEach, describe, expect, it } from "vitest";

import { clearSeenForCard, hasUnseen, markNewLevels, resetNewLevelsForTests } from "./newLevels.ts";

function tokenFor(sub: string, nonce: string): string {
  const encode = (value: object): string =>
    btoa(JSON.stringify(value)).replace(/\+/gu, "-").replace(/\//gu, "_").replace(/=+$/gu, "");
  return `${encode({ alg: "none" })}.${encode({ sub, nonce })}.${encode({ signature: nonce })}`;
}

describe("R3349000 new-level marks are kept per account", () => {
  beforeEach(() => {
    window.localStorage.clear();
    resetNewLevelsForTests();
  });
  afterEach(() => {
    window.localStorage.clear();
    resetNewLevelsForTests();
  });

  it("R3349000 a renewed token keeps the marks and no storage key holds a token", () => {
    const first = tokenFor("account-1", "a");
    const renewed = tokenFor("account-1", "b");
    markNewLevels(first, [{ id: "level-1", cardId: "core-001" }]);

    expect(hasUnseen(renewed, "core-001")).toBe(true);
    const keys = Array.from({ length: window.localStorage.length }, (_, index) => window.localStorage.key(index) ?? "");
    expect(keys.length).toBeGreaterThan(0);
    for (const key of keys) {
      expect(key).not.toContain(first);
      expect(key).not.toContain(renewed);
    }

    clearSeenForCard(renewed, "core-001");
    expect(hasUnseen(first, "core-001")).toBe(false);
  });

  it("R3349000 another account does not see the marks, and a token with no sub is never stored", () => {
    markNewLevels(tokenFor("account-1", "a"), [{ id: "level-1", cardId: "core-001" }]);
    expect(hasUnseen(tokenFor("account-2", "a"), "core-001")).toBe(false);

    markNewLevels("opaque-token", [{ id: "level-2", cardId: "core-002" }]);
    expect(hasUnseen("opaque-token", "core-002")).toBe(true);
    for (let index = 0; index < window.localStorage.length; index += 1) {
      expect(window.localStorage.key(index)).not.toContain("opaque-token");
    }
  });
});
