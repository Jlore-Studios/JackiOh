// The core runs in a module worker, or in-thread under jsdom; `request` preserves response order.
// The in-thread host answers on a macrotask to match worker asynchrony.
// Type-only imports keep the core out of the page bundle; the worker dynamically imports it.
// The engine and AI are WebAssembly (docs/v0.3.0/SURFACE.md §10.3), loaded before the first request.

import { loadWasm } from "../wasm/index.ts";
import type { PracticeCore, PracticeCoreEnv } from "./core.ts";
import { defaultSaveStore } from "./saveStore.ts";
import type { PracticeRequest, PracticeRequestBody, PracticeResponse } from "./protocol.ts";

export type PracticeHost = {
  request(body: PracticeRequestBody): Promise<PracticeResponse>;
  dispose(): void;
};

type PracticeHostOptions = { forceInThread?: boolean; env?: Partial<PracticeCoreEnv> };

const CLOSED_MESSAGE = "the practice game was closed";

function failed(id: number, message: string): PracticeResponse {
  return { id, type: "failed", message };
}

function messageOf(cause: unknown): string {
  return cause instanceof Error ? cause.message : String(cause);
}

function withId(body: PracticeRequestBody, id: number): PracticeRequest {
  return { ...body, id } as PracticeRequest;
}

export function createPracticeHost(options: PracticeHostOptions = {}): PracticeHost {
  if (typeof Worker === "function" && options.forceInThread !== true) return createWorkerHost();
  return createInThreadHost(options.env ?? {});
}

// The worker host

function createWorkerHost(): PracticeHost {
  // Vite recognises this exact shape and bundles the worker as a separate chunk.
  const worker = new Worker(new URL("./practice.worker.ts", import.meta.url), { type: "module" });

  let nextId = 1;
  let broken: string | null = null;
  const waiting = new Map<number, (response: PracticeResponse) => void>();

  function failAll(message: string): void {
    const entries = [...waiting.entries()];
    waiting.clear();
    for (const [id, resolve] of entries) resolve(failed(id, message));
  }

  worker.onmessage = (event: MessageEvent<PracticeResponse>) => {
    const response = event.data;
    const resolve = waiting.get(response.id);
    if (resolve === undefined) return;
    waiting.delete(response.id);
    resolve(response);
  };

  // A missing deployed chunk fires a plain Event, unlike a script ErrorEvent.
  worker.onerror = (event: Event) => {
    event.preventDefault();
    const message = (event as Partial<ErrorEvent>).message;
    broken =
      typeof message === "string" && message !== ""
        ? `the practice worker failed: ${message}`
        : "the practice worker could not start: reload the page (the site may have just been updated)";
    failAll(broken);
  };

  worker.onmessageerror = () => {
    broken = "the practice worker sent a message the page could not read";
    failAll(broken);
  };

  return {
    request(body: PracticeRequestBody): Promise<PracticeResponse> {
      const id = nextId;
      nextId += 1;
      if (broken !== null) return Promise.resolve(failed(id, broken));
      return new Promise<PracticeResponse>((resolve) => {
        waiting.set(id, resolve);
        try {
          worker.postMessage(withId(body, id));
        } catch (cause) {
          waiting.delete(id);
          resolve(failed(id, `the practice worker could not be reached: ${messageOf(cause)}`));
        }
      });
    },

    dispose(): void {
      if (broken === CLOSED_MESSAGE) return;
      broken = CLOSED_MESSAGE;
      worker.terminate();
      failAll(CLOSED_MESSAGE);
    },
  };
}

// The in-thread host for jsdom and browsers without module workers

function macrotask(): Promise<void> {
  return new Promise<void>((resolve) => {
    setTimeout(resolve, 0);
  });
}

function createInThreadHost(env: Partial<PracticeCoreEnv>): PracticeHost {
  let nextId = 1;
  let disposed = false;
  let core: Promise<PracticeCore> | null = null;
  let chain: Promise<unknown> = Promise.resolve();

  function load(): Promise<PracticeCore> {
    if (core === null) {
      // R668: an injected save store outlives the host; otherwise use this scope's store.
      const saves = env.saves ?? defaultSaveStore();
      core = Promise.all([import("./core.ts"), saves.ready, loadWasm()]).then(([mod]) =>
        mod.createPracticeCore({
          now: () => Date.now(),
          dev: import.meta.env.MODE !== "production",
          ...env,
          saves,
        }),
      );
      // A failed import is retried by the next request rather than cached as a failure.
      core.catch(() => {
        core = null;
      });
    }
    return core;
  }

  async function answer(request: PracticeRequest): Promise<PracticeResponse> {
    await macrotask();
    if (disposed) return failed(request.id, CLOSED_MESSAGE);
    try {
      const loaded = await load();
      if (disposed) return failed(request.id, CLOSED_MESSAGE);
      return loaded.handle(request);
    } catch (cause) {
      return failed(request.id, `the practice engine could not be loaded: ${messageOf(cause)}`);
    }
  }

  return {
    request(body: PracticeRequestBody): Promise<PracticeResponse> {
      const request = withId(body, nextId);
      nextId += 1;
      const result = chain.then(() => answer(request));
      chain = result.catch(() => undefined);
      return result;
    },

    dispose(): void {
      disposed = true;
    },
  };
}
