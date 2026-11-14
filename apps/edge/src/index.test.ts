// Holds how long a request waits for a sleeping server, with the container library replaced: its
// real class needs Cloudflare's runtime.

import { beforeEach, describe, expect, it, vi } from "vitest";

import { API_REQUEST_TIMEOUT_SECONDS } from "../../web/src/wire/serverConfig.ts";
import worker, { IMAGE_CHECK_RETRY_MS, IMAGE_CHECK_TIMEOUT_MS, Server, START_WAIT_MS, type Env } from "./index.ts";
import { DAILY_SWEEP_CRON, DAILY_SWEEP_PATH, SERVER_PORT } from "./routing.ts";

const library = vi.hoisted(() => ({
  fetch: vi.fn<(request: Request) => Promise<Response>>(),
  getState: vi.fn<() => Promise<{ status: string }>>(),
  startAndWaitForPorts: vi.fn<(options: unknown) => Promise<void>>(),
  containerFetch: vi.fn<(...args: unknown[]) => Promise<Response>>(),
  stop: vi.fn<(...args: unknown[]) => Promise<void>>(),
  destroy: vi.fn<() => Promise<void>>(),
}));

vi.mock("@cloudflare/containers", () => ({
  Container: class {
    constructor(
      protected ctx: unknown,
      protected env: unknown,
    ) {}

    // Methods, not fields: a field would shadow the `Server` method under test.
    fetch(request: Request): Promise<Response> {
      return library.fetch(request);
    }
    getState(): Promise<{ status: string }> {
      return library.getState();
    }
    startAndWaitForPorts(options: unknown): Promise<void> {
      return library.startAndWaitForPorts(options);
    }
    containerFetch(...args: unknown[]): Promise<Response> {
      return library.containerFetch(...args);
    }
    stop(...args: unknown[]): Promise<void> {
      return library.stop(...args);
    }
    destroy(): Promise<void> {
      return library.destroy();
    }
  },
}));

const COMMIT = "abc123";
const LISTENING = new Response("from the server");

function serverWith(running: boolean): Server {
  const ctx = { container: { running } };
  const env = { DEPLOYED_COMMIT: COMMIT };
  return new Server(ctx as unknown as ConstructorParameters<typeof Server>[0], env as unknown as ConstructorParameters<typeof Server>[1]);
}

/** A server whose `ctx` and `env` the test sets by hand. */
function serverWithCtx(ctx: unknown, env: unknown): Server {
  return new Server(
    ctx as unknown as ConstructorParameters<typeof Server>[0],
    env as unknown as ConstructorParameters<typeof Server>[1],
  );
}

function resetLibrary(): void {
  vi.clearAllMocks();
  library.fetch.mockResolvedValue(LISTENING);
  library.getState.mockResolvedValue({ status: "healthy" });
  library.startAndWaitForPorts.mockResolvedValue();
  library.stop.mockResolvedValue();
  library.destroy.mockResolvedValue();
  library.containerFetch.mockResolvedValue(new Response(null, { headers: { "x-deployed-commit": COMMIT } }));
}

function request(): Request {
  return new Request("https://jackioh.com/api/catalog");
}

describe("a request and a sleeping server", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    library.fetch.mockResolvedValue(LISTENING);
    library.getState.mockResolvedValue({ status: "healthy" });
    library.startAndWaitForPorts.mockResolvedValue();
    library.containerFetch.mockResolvedValue(new Response(null, { headers: { "x-deployed-commit": COMMIT } }));
  });

  it("waits START_WAIT_MS for it to listen, then reaches it", async () => {
    const answer = await serverWith(false).fetch(request());

    expect(library.startAndWaitForPorts).toHaveBeenCalledOnce();
    expect(library.startAndWaitForPorts).toHaveBeenCalledWith({
      ports: SERVER_PORT,
      cancellationOptions: expect.objectContaining({ portReadyTimeoutMS: START_WAIT_MS }) as unknown,
    });
    expect(library.fetch.mock.invocationCallOrder[0]).toBeGreaterThan(
      library.startAndWaitForPorts.mock.invocationCallOrder[0]!,
    );
    expect(answer).toBe(LISTENING);
  });

  it("waits the same for a server already booting, the second caller of a wake", async () => {
    library.getState.mockResolvedValue({ status: "running" });

    await serverWith(true).fetch(request());

    expect(library.startAndWaitForPorts).toHaveBeenCalledOnce();
  });

  it("does not wait for a healthy server", async () => {
    await serverWith(true).fetch(request());

    expect(library.startAndWaitForPorts).not.toHaveBeenCalled();
    expect(library.fetch).toHaveBeenCalledOnce();
  });

  it("leaves a server that never listens to the library's own answer", async () => {
    const refused = new Response("Failed to start container", { status: 500 });
    library.startAndWaitForPorts.mockRejectedValue(new Error("not listening"));
    library.fetch.mockResolvedValue(refused);
    const warn = vi.spyOn(console, "warn").mockImplementation(() => undefined);

    expect(await serverWith(false).fetch(request())).toBe(refused);
    expect(warn).toHaveBeenCalledWith("server.start_failed", expect.any(Error));
  });

  it("answers before the client gives up on the request", () => {
    expect(START_WAIT_MS).toBeLessThan(API_REQUEST_TIMEOUT_SECONDS * 1000);
  });
});

describe("the image check on the first request", () => {
  beforeEach(() => {
    resetLibrary();
  });

  it("asks the running server for its commit at /healthz on the server's port", async () => {
    await serverWith(true).fetch(request());

    expect(library.containerFetch).toHaveBeenCalledOnce();
    expect(library.containerFetch).toHaveBeenCalledWith("http://container/healthz", { signal: expect.any(AbortSignal) as unknown }, SERVER_PORT);
  });

  it("leaves a server that is not running unchecked", async () => {
    await serverWith(false).fetch(request());

    expect(library.containerFetch).not.toHaveBeenCalled();
    expect(library.fetch).toHaveBeenCalledOnce();
  });

  it("leaves a server on the deployed commit running", async () => {
    await serverWith(true).fetch(request());

    expect(library.stop).not.toHaveBeenCalled();
    expect(library.destroy).not.toHaveBeenCalled();
  });

  it("starts a server whose context names no container", async () => {
    const answer = await serverWithCtx({}, { DEPLOYED_COMMIT: COMMIT }).fetch(request());

    expect(library.startAndWaitForPorts).toHaveBeenCalledOnce();
    expect(answer).toBe(LISTENING);
  });

  it("reports a server it cannot ask, and still reaches it", async () => {
    library.containerFetch.mockRejectedValue(new Error("no answer"));
    const warn = vi.spyOn(console, "warn").mockImplementation(() => undefined);

    expect(await serverWith(true).fetch(request())).toBe(LISTENING);
    expect(warn).toHaveBeenCalledWith("server.image_check_failed", expect.any(Error));
    expect(library.stop).not.toHaveBeenCalled();
  });

  it("stops a server on an older image with SIGTERM, and leaves it once it stops", async () => {
    const container = { running: true };
    library.containerFetch.mockResolvedValue(new Response(null, { headers: { "x-deployed-commit": "older" } }));
    library.stop.mockImplementation(() => {
      container.running = false;
      return Promise.resolve();
    });

    await serverWithCtx({ container }, { DEPLOYED_COMMIT: COMMIT }).fetch(request());

    expect(library.stop).toHaveBeenCalledWith("SIGTERM");
    expect(library.destroy).not.toHaveBeenCalled();
  });

  it("waits out a server on an older image that stays up, then destroys it", async () => {
    vi.useFakeTimers();
    try {
      resetLibrary();
      library.containerFetch.mockResolvedValue(new Response(null, { headers: { "x-deployed-commit": "older" } }));

      const fetching = serverWithCtx({ container: { running: true } }, { DEPLOYED_COMMIT: COMMIT }).fetch(request());
      // Still draining: the destroy has to wait for the full wait, not a skipped or shortened one.
      await vi.advanceTimersByTimeAsync(29_000);
      expect(library.stop).toHaveBeenCalledWith("SIGTERM");
      expect(library.destroy).not.toHaveBeenCalled();
      // Past the wait: the server is still up, so it is destroyed.
      await vi.advanceTimersByTimeAsync(1_000);
      expect(library.destroy).toHaveBeenCalledOnce();
      await fetching;
    } finally {
      vi.useRealTimers();
    }
  });

  describe("a check that did not settle, or one that did", () => {
    const OLDER = () => new Response(null, { headers: { "x-deployed-commit": "older" } });

    it("asks again after the retry interval when the check failed, and retires an older commit then", async () => {
      vi.useFakeTimers();
      try {
        resetLibrary();
        vi.spyOn(console, "warn").mockImplementation(() => undefined);
        const container = { running: true };
        library.containerFetch.mockRejectedValueOnce(new Error("no answer")).mockResolvedValue(OLDER());
        library.stop.mockImplementation(() => {
          container.running = false;
          return Promise.resolve();
        });
        const server = serverWithCtx({ container }, { DEPLOYED_COMMIT: COMMIT });

        expect(await server.fetch(request())).toBe(LISTENING);
        expect(library.containerFetch).toHaveBeenCalledOnce();
        expect(library.stop).not.toHaveBeenCalled();

        // Inside the interval the request goes through without a new check.
        expect(await server.fetch(request())).toBe(LISTENING);
        expect(library.containerFetch).toHaveBeenCalledOnce();

        vi.advanceTimersByTime(IMAGE_CHECK_RETRY_MS);
        expect(await server.fetch(request())).toBe(LISTENING);
        expect(library.containerFetch).toHaveBeenCalledTimes(2);
        expect(library.stop).toHaveBeenCalledWith("SIGTERM");
      } finally {
        vi.useRealTimers();
      }
    });

    it("leaves that request and the next one answered when stop rejects once", async () => {
      vi.useFakeTimers();
      try {
        resetLibrary();
        const warn = vi.spyOn(console, "warn").mockImplementation(() => undefined);
        const container = { running: true };
        library.containerFetch.mockImplementation(() => Promise.resolve(OLDER()));
        library.stop.mockRejectedValueOnce(new Error("refused")).mockImplementation(() => {
          container.running = false;
          return Promise.resolve();
        });
        const server = serverWithCtx({ container }, { DEPLOYED_COMMIT: COMMIT });

        expect(await server.fetch(request())).toBe(LISTENING);
        expect(warn).toHaveBeenCalledWith("server.retire_failed", expect.any(Error));
        expect(library.destroy).not.toHaveBeenCalled();

        vi.advanceTimersByTime(IMAGE_CHECK_RETRY_MS);
        expect(await server.fetch(request())).toBe(LISTENING);
        expect(library.stop).toHaveBeenCalledTimes(2);
      } finally {
        vi.useRealTimers();
      }
    });

    it("logs a destroy that rejects and still answers", async () => {
      vi.useFakeTimers();
      try {
        resetLibrary();
        const warn = vi.spyOn(console, "warn").mockImplementation(() => undefined);
        library.containerFetch.mockResolvedValue(OLDER());
        library.destroy.mockRejectedValueOnce(new Error("refused"));

        const fetching = serverWithCtx({ container: { running: true } }, { DEPLOYED_COMMIT: COMMIT }).fetch(request());
        await vi.advanceTimersByTimeAsync(30_000);

        expect(await fetching).toBe(LISTENING);
        expect(warn).toHaveBeenCalledWith("server.retire_failed", expect.any(Error));
      } finally {
        vi.useRealTimers();
      }
    });

    it("lets the request through when /healthz never answers, once the timeout passes", async () => {
      // A native `AbortSignal.timeout` is not driven by fake timers, so the test holds the signal.
      const controller = new AbortController();
      const timeout = vi.spyOn(AbortSignal, "timeout").mockReturnValue(controller.signal);
      try {
        resetLibrary();
        const warn = vi.spyOn(console, "warn").mockImplementation(() => undefined);
        library.containerFetch.mockImplementation(
          (_url, init) =>
            new Promise<Response>((resolve) => {
              // The library's `containerFetch` answers an aborted fetch with a 500 instead of rejecting.
              (init as RequestInit).signal?.addEventListener("abort", () =>
                resolve(new Response("Error proxying request to container: timed out", { status: 500 })),
              );
            }),
        );

        let settled = false;
        const fetching = serverWith(true)
          .fetch(request())
          .finally(() => {
            settled = true;
          });
        await Promise.resolve();
        expect(timeout).toHaveBeenCalledWith(IMAGE_CHECK_TIMEOUT_MS);
        expect(settled).toBe(false);

        controller.abort();
        expect(await fetching).toBe(LISTENING);
        expect(warn).toHaveBeenCalledWith("server.image_check_failed", 500);
        expect(library.stop).not.toHaveBeenCalled();
      } finally {
        timeout.mockRestore();
      }
    });

    it("takes the library's 500 for a failed check, not an older image, and asks again after the interval", async () => {
      vi.useFakeTimers();
      try {
        resetLibrary();
        const warn = vi.spyOn(console, "warn").mockImplementation(() => undefined);
        const container = { running: true };
        library.containerFetch
          .mockResolvedValueOnce(new Response("Error proxying request to container: timed out", { status: 500 }))
          .mockResolvedValue(OLDER());
        library.stop.mockImplementation(() => {
          container.running = false;
          return Promise.resolve();
        });
        const server = serverWithCtx({ container }, { DEPLOYED_COMMIT: COMMIT });

        expect(await server.fetch(request())).toBe(LISTENING);
        expect(warn).toHaveBeenCalledWith("server.image_check_failed", 500);
        expect(library.stop).not.toHaveBeenCalled();

        vi.advanceTimersByTime(IMAGE_CHECK_RETRY_MS);
        expect(await server.fetch(request())).toBe(LISTENING);
        expect(library.containerFetch).toHaveBeenCalledTimes(2);
        expect(library.stop).toHaveBeenCalledWith("SIGTERM");
      } finally {
        vi.useRealTimers();
      }
    });

    it("does not repeat a check that succeeded", async () => {
      vi.useFakeTimers();
      try {
        resetLibrary();
        const server = serverWith(true);

        await server.fetch(request());
        vi.advanceTimersByTime(IMAGE_CHECK_RETRY_MS * 3);
        await server.fetch(request());

        expect(library.containerFetch).toHaveBeenCalledOnce();
      } finally {
        vi.useRealTimers();
      }
    });

    it("does not check again a server it found asleep and then started", async () => {
      vi.useFakeTimers();
      try {
        resetLibrary();
        const container = { running: false };
        library.startAndWaitForPorts.mockImplementation(() => {
          container.running = true;
          return Promise.resolve();
        });
        const server = serverWithCtx({ container }, { DEPLOYED_COMMIT: COMMIT });

        await server.fetch(request());
        vi.advanceTimersByTime(IMAGE_CHECK_RETRY_MS * 3);
        await server.fetch(request());

        // Nothing was running to be on an older image, and what the first request started is the deployed one.
        expect(library.startAndWaitForPorts).toHaveBeenCalledOnce();
        expect(library.containerFetch).not.toHaveBeenCalled();
      } finally {
        vi.useRealTimers();
      }
    });

    it("holds a request that comes during the check until that one check settles", async () => {
      resetLibrary();
      let answer: (response: Response) => void = () => undefined;
      library.containerFetch.mockImplementation(
        () =>
          new Promise<Response>((resolve) => {
            answer = resolve;
          }),
      );
      const server = serverWith(true);

      const first = server.fetch(request());
      const second = server.fetch(request());
      await new Promise((resolve) => setTimeout(resolve, 0));

      // Neither reaches the server while the check is out, and the second starts no check of its own.
      expect(library.containerFetch).toHaveBeenCalledOnce();
      expect(library.fetch).not.toHaveBeenCalled();
      answer(new Response(null, { headers: { "x-deployed-commit": COMMIT } }));
      expect(await first).toBe(LISTENING);
      expect(await second).toBe(LISTENING);
      expect(library.fetch).toHaveBeenCalledTimes(2);
      expect(library.containerFetch).toHaveBeenCalledOnce();
    });

    it("does not repeat a check that retired the old server", async () => {
      vi.useFakeTimers();
      try {
        resetLibrary();
        const container = { running: true };
        library.containerFetch.mockImplementation(() => Promise.resolve(OLDER()));
        library.stop.mockImplementation(() => {
          container.running = false;
          return Promise.resolve();
        });
        // The request then starts the deployed image's server, so a second check would find one to ask.
        library.startAndWaitForPorts.mockImplementation(() => {
          container.running = true;
          return Promise.resolve();
        });
        const server = serverWithCtx({ container }, { DEPLOYED_COMMIT: COMMIT });

        await server.fetch(request());
        vi.advanceTimersByTime(IMAGE_CHECK_RETRY_MS * 3);
        await server.fetch(request());

        expect(library.startAndWaitForPorts).toHaveBeenCalledOnce();
        expect(library.containerFetch).toHaveBeenCalledOnce();
        expect(library.stop).toHaveBeenCalledOnce();
      } finally {
        vi.useRealTimers();
      }
    });
  });
});

describe("the Worker's routes", () => {
  beforeEach(() => {
    resetLibrary();
  });

  function routedEnv(fetch: (request: Request) => Promise<Response>): Env {
    return {
      ASSETS: { fetch: vi.fn() },
      SERVER: { idFromName: vi.fn((name: string) => name), get: vi.fn((_id: string) => ({ fetch })) },
    } as unknown as Env;
  }

  it("answers anything that is neither the server's nor the module's with 404", async () => {
    const env = routedEnv(() => Promise.resolve(new Response("unreached")));
    const answer = await worker.fetch(new Request("https://jackioh.com/login"), env);

    expect(answer.status).toBe(404);
    expect(await answer.text()).toBe("Not found");
    expect(env.SERVER.get).not.toHaveBeenCalled();
  });

  it("forwards the server's paths through the one instance, without the caller's address", async () => {
    const fetch = vi.fn((_request: Request) => Promise.resolve(new Response("from the server")));
    const env = routedEnv(fetch);
    const answer = await worker.fetch(
      new Request("https://jackioh.com/api/catalog", { headers: { "X-Forwarded-For": "spoofed" } }),
      env,
    );

    expect(env.SERVER.idFromName).toHaveBeenCalledWith("main");
    expect(env.SERVER.get).toHaveBeenCalledWith("main", { locationHint: "enam" });
    expect(fetch).toHaveBeenCalledOnce();
    const forwarded = fetch.mock.calls[0]?.[0] as Request;
    expect(forwarded.headers.has("X-Forwarded-For")).toBe(false);
    expect(await answer.text()).toBe("from the server");
  });
});

describe("the daily wake-up", () => {
  function scheduledEnv(status: number): { env: Env; fetch: ReturnType<typeof vi.fn> } {
    const fetch = vi.fn(() => Promise.resolve(new Response(status === 200 ? "ready" : "not ready", { status })));
    const env = {
      SERVER: { idFromName: vi.fn((name: string) => name), get: vi.fn((_id: string) => ({ fetch })) },
    } as unknown as Env;
    return { env, fetch };
  }

  it("asks the server for readiness", async () => {
    const { env, fetch } = scheduledEnv(200);

    await worker.scheduled({} as Parameters<typeof worker.scheduled>[0], env);

    expect(env.SERVER.idFromName).toHaveBeenCalledWith("main");
    expect(fetch).toHaveBeenCalledOnce();
    expect((fetch.mock.calls[0]?.[0] as Request).url).toBe("http://container/readyz");
  });

  it("fails naming the path and the status when the server is not ready", async () => {
    const { env } = scheduledEnv(500);

    await expect(worker.scheduled({} as Parameters<typeof worker.scheduled>[0], env)).rejects.toThrow(
      "the server's /readyz answered 500",
    );
  });

  it("CLAUDE.md rule 13: the daily cron still asks for readiness, not the sweep", async () => {
    const { env, fetch } = scheduledEnv(200);

    await worker.scheduled({ cron: "23 14 * * *" } as Parameters<typeof worker.scheduled>[0], env);

    expect((fetch.mock.calls[0]?.[0] as Request).url).toBe("http://container/readyz");
  });
});

describe("#3727 the daily games' hourly sweep", () => {
  function sweepEnv(status: number): { env: Env; fetch: ReturnType<typeof vi.fn> } {
    const fetch = vi.fn(() => Promise.resolve(new Response("{}", { status })));
    const env = {
      SERVER: { idFromName: vi.fn((name: string) => name), get: vi.fn((_id: string) => ({ fetch })) },
    } as unknown as Env;
    return { env, fetch };
  }

  it("posts the sweep to the server on the hourly cron", async () => {
    const { env, fetch } = sweepEnv(200);

    await worker.scheduled({ cron: DAILY_SWEEP_CRON } as Parameters<typeof worker.scheduled>[0], env);

    expect(fetch).toHaveBeenCalledOnce();
    const request = fetch.mock.calls[0]?.[0] as Request;
    expect(request.url).toBe(`http://container${DAILY_SWEEP_PATH}`);
    expect(request.method).toBe("POST");
  });

  it("fails naming the path and the status when the sweep fails", async () => {
    const { env } = sweepEnv(500);

    await expect(
      worker.scheduled({ cron: DAILY_SWEEP_CRON } as Parameters<typeof worker.scheduled>[0], env),
    ).rejects.toThrow("the server's /daily-sweep answered 500");
  });
});

describe("#3183 the engine's compressed module", () => {
  it("is served from the web build's files with its Content-Encoding, and never wakes the server", async () => {
    const assets = { fetch: vi.fn(() => Promise.resolve(new Response("stored brotli bytes"))) };
    const env = { ASSETS: assets, SERVER: { idFromName: vi.fn(), get: vi.fn() } };
    // An incoming request carries Cloudflare's `cf` facts, which a test's own Request has none of.
    const path = new Request("https://jackioh.com/assets/jackioh_wasm_bg.wasm-BIXTgzSO.br") as Parameters<typeof worker.fetch>[0];
    const served = await worker.fetch(path, env as unknown as Env);
    expect(assets.fetch).toHaveBeenCalledWith(path);
    expect(env.SERVER.get).not.toHaveBeenCalled();
    expect(served.headers.get("Content-Encoding")).toBe("br");
    expect(served.headers.get("Content-Type")).toBe("application/wasm");
    expect(await served.text()).toBe("stored brotli bytes");
  });
});
