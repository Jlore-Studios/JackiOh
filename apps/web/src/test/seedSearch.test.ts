// #2569: the web tests' seed search.

import { describe, expect, it } from "vitest";

import { SEED_SEARCH_BOUND, seedWhere } from "./seedSearch.ts";

describe("#2569 seedWhere: the web tests' seed search", () => {
  it("returns the first seed accepted, trying them in order", () => {
    const tried: string[] = [];
    const seed = seedWhere("seed-search", (candidate) => {
      tried.push(candidate);
      return candidate.endsWith("-7");
    });
    expect(seed).toBe("seed-search-7");
    expect(tried).toEqual(Array.from({ length: 8 }, (_, index) => `seed-search-${String(index)}`));
  });

  it("names its prefix and its bound when no seed fits", () => {
    const search = (): string => seedWhere("seed-search-none", () => false);
    expect(search).toThrow('"seed-search-none-0"');
    expect(search).toThrow(`"seed-search-none-${String(SEED_SEARCH_BOUND - 1)}"`);
    expect(search).toThrow(`first ${String(SEED_SEARCH_BOUND)}`);
  });
});
