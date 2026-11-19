// Whether the WebAssembly module is in (`loadWasm`, index.ts) and the scene tables are installed
// (`loadSceneData`, sceneLoader.ts), for the engine-free screens that draw a card: the landing
// page's fan and the not-found page's card. A card face is worked out by the module (jackioh-shared's
// face model) and drawn from the scene tables (scene.ts's `sceneRecipeOf`), so they draw it once both
// have arrived, and the rest of their screen does not wait for either (#778). A screen behind
// `EngineGate` never needs this. #4180, R1301: the Chinese card tables load beside the module,
// best-effort, so a flagged view's card falls back to English until the tables arrive. #4222: the
// scene tables load beside the module too, but readiness waits for both, since a face drawn before
// they arrive throws instead of drawing the painter's picture; a scene-chunk failure leaves the face
// undrawn rather than wrong.

import { useEffect, useState } from "react";

import { loadSceneData, sceneDataReady } from "../cards/art/sceneLoader.ts";
import { loadCardTables } from "../cards/chinese.ts";
import { loadWasm, wasmLoaded } from "./index.ts";

export function useEngineReady(): boolean {
  const [ready, setReady] = useState(wasmLoaded() && sceneDataReady());
  useEffect(() => {
    if (ready) return undefined;
    let live = true;
    void Promise.all([loadWasm(), loadSceneData()]).then(
      () => {
        // Stryker disable next-line ConditionalExpression: React 19 drops a state update on an unmounted component without a word, so setting ready after the cleanup shows nothing.
        if (live) setReady(true);
      },
      () => undefined,
    );
    loadCardTables().then(
      () => undefined,
      () => undefined,
    );
    // Stryker disable next-line BlockStatement: React 19 drops a state update on an unmounted component without a word, so a cleanup that leaves live true shows nothing.
    return () => {
      // Stryker disable next-line BooleanLiteral: React 19 drops a state update on an unmounted component without a word, so leaving live true shows nothing.
      live = false;
    };
  }, [ready]);
  return ready;
}
