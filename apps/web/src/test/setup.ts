// Vitest setup. jsdom lacks `matchMedia`, which the animation table needs (BUILD M5-T4); each test
// starts on an empty read cache (R2125000: never on another's kept answers). The Rust engine
// (docs/v0.3.0/SURFACE.md §10.3) loads synchronously from the bytes `scripts/build-wasm.sh` (run by
// `pretest`) writes, as jsdom cannot fetch a file URL, before any test file is imported.

import "@testing-library/jest-dom/vitest";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { beforeEach } from "vitest";

import { clearReadCache } from "../net/readCache.ts";
import { loadWasmSync } from "../wasm/index.ts";

// A path, not `new URL(…, import.meta.url)`: Vite rewrites that into an asset URL against the page's
// origin, which `readFileSync` refuses.
loadWasmSync(readFileSync(join(dirname(fileURLToPath(import.meta.url)), "../wasm/pkg/jackioh_wasm_bg.wasm")));

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

// jsdom's `window.scrollTo` logs "Not implemented", and `navigate` scrolls each new screen to the top
// (#1756). `net/navigate.test.ts` spies on this no-op.
if (typeof window !== "undefined") {
  Object.defineProperty(window, "scrollTo", { configurable: true, writable: true, value: () => {} });
}

// A `PointerEvent` that drops `pointerType` makes every touch read as a mouse click (#2215).
function ensurePointerEvent(): void {
  const win = document.defaultView;
  if (win === null) return;
  const probe = typeof win.PointerEvent === "function" ? new win.PointerEvent("pointerdown", { pointerType: "touch" }) : null;
  if (probe !== null && probe.pointerType === "touch") return;

  class TestPointerEvent extends win.MouseEvent {
    readonly pointerType: string;
    readonly pointerId: number;
    readonly isPrimary: boolean;
    constructor(type: string, init: PointerEventInit = {}) {
      super(type, init);
      this.pointerType = init.pointerType ?? "";
      this.pointerId = init.pointerId ?? 1;
      this.isPrimary = init.isPrimary ?? true;
    }
  }
  for (const target of [win, globalThis]) {
    Object.defineProperty(target, "PointerEvent", { value: TestPointerEvent, configurable: true, writable: true });
  }
}

if (typeof window !== "undefined") ensurePointerEvent();

beforeEach(() => {
  clearReadCache();
  try {
    window.sessionStorage.clear();
  } catch {
    // Some tests stub storage getters to throw.
  }
});
