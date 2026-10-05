// The practice worker's entry (SPEC §9.9, R187). It owns the one practice `GameState` and answers
// each request with `core.handle`, which never throws. `host.ts` spawns it as a module worker. A free
// game in progress is kept in IndexedDB (R659, `saveStore.ts`), and every request waits for that
// store's one read at boot; requests still run one at a time, in the order they came.
//
// `self` is typed with a local minimal interface rather than the `WebWorker` lib, which would clash
// with the `DOM` lib the rest of `apps/web` compiles against.

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
  now: () => performance.now(),
  dev: import.meta.env.MODE !== "production",
  saves,
});

scope.onmessage = (event) => {
  // `ready` never rejects, and callbacks on one promise run in the order they were added.
  void saves.ready.then(() => {
    scope.postMessage(core.handle(event.data));
  });
};
