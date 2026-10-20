// #1378: the current catalog is the one the client ships (SPEC §9.4, R163): the engine module's
// registered definitions and compiled-in version, read once, and no screen asks the server for it.

import { readFileSync, readdirSync, statSync } from "node:fs";
import { dirname, join, relative } from "node:path";
import { fileURLToPath } from "node:url";

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { CATALOG, CATALOG_VERSION } from "@jackioh/cards";

import { catalogVersion, registeredCatalog } from "../wasm/index.ts";
import { currentCatalog, currentCatalogLookup } from "./currentCatalog.ts";

const requests = vi.fn((input: unknown) => Promise.reject(new TypeError(`Failed to fetch ${String(input)}`)));

beforeEach(() => {
  vi.stubGlobal("fetch", requests);
});

afterEach(() => {
  requests.mockClear();
  vi.unstubAllGlobals();
});

describe("the current catalog provider", () => {
  it("#1378 its definitions and version are the registered catalog's, the one the bundle ships", () => {
    const snapshot = currentCatalog();
    expect(snapshot.cards).toBe(registeredCatalog());
    expect(snapshot.version).toBe(catalogVersion());
    expect(snapshot.version).toBe(CATALOG_VERSION);
    expect(Object.keys(snapshot.cards).sort()).toEqual(Object.keys(CATALOG).sort());
    expect(requests).not.toHaveBeenCalled();
  });

  it("#1378 it is read once: the same snapshot and the same lookup on every call", () => {
    expect(currentCatalog()).toBe(currentCatalog());
    expect(currentCatalogLookup()).toBe(currentCatalogLookup());
    expect(currentCatalogLookup()("core-001", false)?.name).toBe(registeredCatalog()["core-001"]?.name);
    expect(requests).not.toHaveBeenCalled();
  });
});

describe("#1378 no screen asks the server for the current catalog", () => {
  const SRC = join(dirname(fileURLToPath(import.meta.url)), "..");

  function sources(dir: string): string[] {
    return readdirSync(dir).flatMap((name) => {
      const path = join(dir, name);
      if (statSync(path).isDirectory()) return name === "test" || name === "pkg" ? [] : sources(path);
      return /\.(ts|tsx)$/u.test(name) && !/\.test\.tsx?$/u.test(name) ? [path] : [];
    });
  }

  it("#1378 no client source names the unversioned /api/catalog route", () => {
    const files = sources(SRC);
    expect(files.length).toBeGreaterThan(100);
    const naming = files.filter((path) => /["'`]\/api\/catalog["'`]/u.test(readFileSync(path, "utf8")));
    expect(naming.map((path) => relative(SRC, path))).toEqual([]);
  });
});
