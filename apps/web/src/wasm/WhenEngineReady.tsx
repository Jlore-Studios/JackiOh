// Draws a card's face only once the WebAssembly module is in: the face is worked out by the module (jackioh-shared's
// face model), and a screen that never waits for it (the landing, the shop; main.tsx's route table, #778) must not
// work one out before it has arrived, or the throw takes the whole screen down to the route's error panel. The
// children are a function, so the face is worked out when the module is in and not while the parent renders.

import type { ReactElement, ReactNode } from "react";

import { useEngineReady } from "./useEngineReady.ts";

export type WhenEngineReadyProps = {
  /** What to draw once the module is in. */
  children: () => ReactNode;
  /** What stands in the meantime. */
  fallback?: ReactNode;
};

export function WhenEngineReady({ children, fallback = null }: WhenEngineReadyProps): ReactElement {
  return <>{useEngineReady() ? children() : fallback}</>;
}
