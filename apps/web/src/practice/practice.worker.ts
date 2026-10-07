// The practice worker's entry (SPEC §9.9, R187). It owns the one practice `GameState` and answers
// each request with `core.handle`, which never throws. `host.ts` spawns it as a module worker. The
// engine and the AI are the Rust ones, compiled to WebAssembly (docs/v0.3.0/SURFACE.md §10.3): the
// worker loads the module once, at boot, and every request waits for it. A free game in progress
// is kept in IndexedDB (R668, `saveStore.ts`), and every request waits for that store's one read at
// boot too; requests still run one at a time, in the order they came.
//
// `self` is typed with a local minimal interface rather than the `WebWorker` lib, which would clash
// with the `DOM` lib the rest of `apps/web` compiles against.

import { loadWasm } from "../wasm/index.ts";
import { createPracticeCore } from "./core.ts";
import { defaultSaveStore } from "./saveStore.ts";
import type { PracticeRequest, PracticeResponse } from "./protocol.ts";

type PracticeWorkerScope = {
  onmessage: ((event: MessageEvent<PracticeRequest>) => void) | null;
  postMessage(message: PracticeResponse): void;
};

const scope = self as unknown as PracticeWorkerScope;

const saves = defaultSaveStore();
const core = createPracticeCore({
  // `Date.now`, the clock the WebAssembly side measures the AI's deadline on (`core.ts`).
  now: () => Date.now(),
  dev: import.meta.env.MODE !== "production",
  saves,
});

/**
 * The module, loaded once. A load that fails is answered on every request as a failure: the page
 * shows it as it shows any other, and there is no game to run without the engine.
 */
const engine: Promise<string | null> = loadWasm().then(
  () => null,
  (cause: unknown) => `the practice engine could not be loaded: ${cause instanceof Error ? cause.message : String(cause)}`,
);

/** Both boot reads at once. Neither ever rejects. */
const booted = Promise.all([engine, saves.ready]);

scope.onmessage = (event) => {
  // Callbacks on one promise run in the order they were added, so requests keep their order.
  const request = event.data;
  void booted.then(([failure]) => {
    scope.postMessage(failure === null ? core.handle(request) : { id: request.id, type: "failed", message: failure });
  });
};
