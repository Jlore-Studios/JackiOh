// The practice worker's entry (SPEC §9.9, R187): it owns the one practice `GameState` and answers each
// request with `core.handle`, which never throws. Every request waits for the WebAssembly module
// (docs/v0.3.0/SURFACE.md §10.3) and the save store's one read (R668, `saveStore.ts`), both at boot.
//
// `self` is typed with a local minimal interface rather than the `WebWorker` lib, which would clash
// with the `DOM` lib the rest of `apps/web` compiles against.

import { loadWasm } from "../wasm/index.ts";
import { createPracticeDriver } from "./driver.ts";
import { defaultSaveStore } from "./saveStore.ts";
import type { PracticeRequest, PracticeResponse } from "./protocol.ts";
import { messageOf } from "../net/errors.ts";

type PracticeWorkerScope = {
  onmessage: ((event: MessageEvent<PracticeRequest>) => void) | null;
  postMessage(message: PracticeResponse): void;
};

const scope = self as unknown as PracticeWorkerScope;

const saves = defaultSaveStore();
const core = createPracticeDriver({
  // `Date.now`, the clock the WebAssembly side measures the AI's deadline on (`driver.ts`).
  now: () => Date.now(),
  dev: import.meta.env.MODE !== "production",
  saves,
});

/** A load that fails is answered on every request as a failure: there is no game without the engine. */
const engine: Promise<string | null> = loadWasm().then(
  () => null,
  (cause: unknown) => `the practice engine could not be loaded: ${messageOf(cause)}`,
);

/** Both boot reads at once. Neither ever rejects. */
const booted = Promise.all([engine, saves.ready]);

/** Requests, one at a time in the order they came. */
let chain: Promise<unknown> = booted;

scope.onmessage = (event) => {
  const request = event.data;
  chain = chain.then(async () => {
    const [failure] = await booted;
    if (failure !== null) {
      scope.postMessage({ id: request.id, type: "failed", message: failure });
      return;
    }
    scope.postMessage(core.handle(request));
  });
};
