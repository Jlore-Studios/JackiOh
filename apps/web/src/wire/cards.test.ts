// The catalog and version readers (`./cards.ts`, SURFACE §10.4): the catalog is the
// compiled `catalog.json`, the version its newest patch, and the version names map the
// old lines through `aliases.json` (R1371000, R2532000).

import { describe, expect, it } from "vitest";

import { CATALOG, CATALOG_VERSION, currentVersionName, versionNameFrom } from "./cards.ts";

describe("CATALOG", () => {
  it("holds the compiled catalog by id", () => {
    expect(typeof CATALOG).toBe("object");
    expect(CATALOG["core-001"]).toMatchObject({ id: "core-001" });
  });
});

describe("CATALOG_VERSION", () => {
  it("is the newest patch's non-empty version", () => {
    expect(typeof CATALOG_VERSION).toBe("string");
    expect(CATALOG_VERSION.length).toBeGreaterThan(0);
  });
});

describe("currentVersionName", () => {
  it("maps an old line's name to today's and leaves today's alone", () => {
    expect(currentVersionName("v0.2.1")).toBe("v0.2.0b");
    expect(currentVersionName("v0.4.0")).toBe("v0.4.0");
    expect(currentVersionName("not-a-version")).toBe("not-a-version");
  });
});

describe("versionNameFrom", () => {
  it("reads scheme 1 through today's map", () => {
    expect(versionNameFrom(1, "v0.2.1")).toBe("v0.2.0b");
    expect(versionNameFrom(0, "v0.4.0")).toBe("v0.4.0");
  });

  it("reads an older marked scheme through its own map and today's as it is", () => {
    expect(versionNameFrom(2, "v0.5.1")).toBe("v0.5.0b");
    expect(versionNameFrom(2, "no-such-version")).toBe("no-such-version");
    expect(versionNameFrom(3, "v0.5.1")).toBe("v0.5.1");
  });
});
