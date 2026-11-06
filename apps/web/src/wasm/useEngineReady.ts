// Whether the WebAssembly module is in (`loadWasm`, index.ts), for the engine-free screens that draw a card:
// the landing page's fan and the not-found page's card. A card face is worked out by the module (jackioh-shared's
// face model), so they draw it once the module has arrived, and the rest of their screen does not wait for it
// (#778). A screen behind `EngineGate` never needs this.

import { useEffect, useState } from "react";

import { loadWasm, wasmLoaded } from "./index.ts";

export function useEngineReady(): boolean {
  const [ready, setReady] = useState(wasmLoaded());
  useEffect(() => {
    if (ready) return undefined;
    let live = true;
    loadWasm().then(
      () => {
        if (live) setReady(true);
      },
      () => undefined,
    );
    return () => {
      live = false;
    };
  }, [ready]);
  return ready;
}
