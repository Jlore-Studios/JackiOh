// The raster's worker entry (#513): draws a GIF's frame plan on an `OffscreenCanvas`, off the main
// thread, and answers with its progress and then the bytes. `export.ts` starts it with
// `new Worker(new URL("./raster.worker.ts", import.meta.url), { type: "module" })`, which Vite splits
// into a chunk of its own that loads only when a clip is exported; a browser that cannot draw in a
// worker is answered `unsupported` and `export.ts` rasterizes on the main thread instead.
//
// `self` is typed with a local minimal interface rather than the `WebWorker` lib, which would clash
// with the `DOM` lib the rest of `apps/web` compiles against (as `practice.worker.ts` does).

import { canvasContext, rasterize } from "./raster.ts";
import type { RasterAnswer, RasterRequest } from "./raster.ts";

type RasterWorkerScope = {
  onmessage: ((event: MessageEvent<RasterRequest>) => void) | null;
  postMessage(message: RasterAnswer, transfer?: Transferable[]): void;
};

const scope = self as unknown as RasterWorkerScope;

scope.onmessage = (event) => {
  const { plan, assets, maxBytes } = event.data;
  const ctx = typeof OffscreenCanvas === "function" ? canvasContext(plan.width, plan.height) : null;
  if (ctx === null) {
    scope.postMessage({ type: "unsupported" });
    return;
  }
  rasterize(ctx, plan, assets, {
    maxBytes,
    aborted: () => false,
    onFrame: (frame) => {
      scope.postMessage({ type: "frame", frame });
    },
  }).then(
    (bytes) => {
      if (bytes === null) scope.postMessage({ type: "over" });
      else scope.postMessage({ type: "done", bytes }, [bytes.buffer]);
    },
    (cause: unknown) => {
      scope.postMessage({ type: "failed", message: cause instanceof Error ? cause.message : String(cause) });
    },
  );
};
