// #4428: the sound-effect recipes load through `sfxLoader.ts`, without waiting on the WebAssembly
// module (they need no catalog). The suite mocks the module's loader to never resolve: `loadSfx()`
// resolving anyway proves the recipes wait on nothing but their own chunk.

import { afterEach, describe, expect, it, vi } from "vitest";

import { deferSfxForTests, loadSfx, onSfxLoaded, setSfxForTests, sfxNow } from "./sfxLoader.ts";

vi.mock("../wasm/index.ts", () => ({ loadWasm: () => new Promise<never>(() => {}) }));

afterEach(() => {
  setSfxForTests(null);
});

describe("#4428 the recipes load without the module", () => {
  it("#4428 the recipes load without waiting on the WebAssembly module", async () => {
    // The setup installs the recipes for every test file: empty them first, so this test proves
    // the load itself waits on nothing but its own chunk (the module's loader never resolves).
    setSfxForTests(null);
    expect(sfxNow()).toBeNull();
    const recipes = await loadSfx();
    expect(sfxNow()).toBe(recipes);
    expect(Object.keys(recipes.SFX).length).toBeGreaterThan(0);
  });

  it("#4428 a listener hears the recipes once, and a late subscriber hears them at once", async () => {
    const early: unknown[] = [];
    const unsub = onSfxLoaded((recipes) => {
      early.push(recipes);
    });
    try {
      const recipes = await loadSfx();
      expect(early).toEqual([recipes]);
      const late: unknown[] = [];
      const unsubLate = onSfxLoaded((r) => {
        late.push(r);
      });
      try {
        expect(late).toEqual([recipes]);
      } finally {
        unsubLate();
      }
    } finally {
      unsub();
      setSfxForTests(null);
    }
  });

  it("#4428 deferring empties the recipes until they resolve", async () => {
    const deferred = deferSfxForTests();
    try {
      expect(sfxNow()).toBeNull();
      const pending = loadSfx();
      const recipes = await import("./sfx.ts");
      deferred.resolve(recipes);
      expect(await pending).toBe(recipes);
      expect(sfxNow()).toBe(recipes);
    } finally {
      setSfxForTests(null);
    }
  });
});
