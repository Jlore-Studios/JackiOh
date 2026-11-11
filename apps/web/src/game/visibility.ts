// R3497000: the page-visibility source an online board drains its animation queue on, injectable
// in tests. The page's own source reads `document` exactly as `FxLayer.tsx` does.
import type { FxVisibility } from "../fx/types.ts";

/** The page's own visibility: hidden while the tab is hidden, notified on every change. */
export function defaultVisibility(): FxVisibility {
  return {
    hidden: () => document.hidden === true,
    subscribe: (listener) => {
      document.addEventListener("visibilitychange", listener);
      return () => document.removeEventListener("visibilitychange", listener);
    },
  };
}
