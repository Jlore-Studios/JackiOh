// Draws a card's face only once the WebAssembly module is in and the scene tables are installed: the face is worked
// out by the module (jackioh-shared's face model) and drawn from the scene tables (scene.ts's `sceneRecipeOf`, #4222),
// and a screen that never waits for them (the landing, the shop; main.tsx's route table, #778) must not work one out
// before they have arrived, or the throw takes the whole screen down to the route's error panel. The children are a
// function, so the face is worked out when both are in and not while the parent renders.

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
