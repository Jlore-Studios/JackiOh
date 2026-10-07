// Vitest setup for the web package: jsdom matchers, a `matchMedia` stub, which jsdom lacks and the
// animation table needs for `prefers-reduced-motion` (BUILD M5-T4), and the engine.
//
// The engine, the AI and the validator are the Rust ones, compiled to WebAssembly
// (docs/v0.3.0/SURFACE.md §10.3). jsdom cannot fetch the module from a file URL the way the page
// does, so its bytes are read from disk here and instantiated before any test file is imported
// (`scripts/build-wasm.sh`, which `pretest` runs, writes them). Every test then reaches the engine
// synchronously, as the page does after `main.tsx` has awaited `loadWasm()`.

import "@testing-library/jest-dom/vitest";
import { readFileSync } from "node:fs";
import { beforeEach } from "vitest";

import { loadWasmSync } from "../wasm/index.ts";

loadWasmSync(readFileSync(new URL("../wasm/pkg/jackioh_wasm_bg.wasm", import.meta.url)));

let reducedMotion = false;

export const REDUCED_MOTION_QUERY = "(prefers-reduced-motion: reduce)";

/** Flip `prefers-reduced-motion` for a test. Always reset it in an `afterEach`. */
export function setReducedMotion(on: boolean): void {
  reducedMotion = on;
}

function matches(query: string): boolean {
  return reducedMotion && query.replace(/\s+/g, "") === REDUCED_MOTION_QUERY.replace(/\s+/g, "");
}

if (typeof window !== "undefined" && typeof window.matchMedia !== "function") {
  Object.defineProperty(window, "matchMedia", {
    writable: true,
    value: (query: string): MediaQueryList =>
      ({
        media: query,
        get matches() {
          return matches(query);
        },
        onchange: null,
        addListener: () => {},
        removeListener: () => {},
        addEventListener: () => {},
        removeEventListener: () => {},
        dispatchEvent: () => false,
      }) as unknown as MediaQueryList,
  });
}

beforeEach(() => {
  try {
    window.sessionStorage.clear();
  } catch {
    // Some tests stub storage getters to throw.
  }
});
