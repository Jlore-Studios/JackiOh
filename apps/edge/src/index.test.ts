// Holds how long a request waits for a sleeping server, with the container library replaced: its
// real class needs Cloudflare's runtime.

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { API_REQUEST_TIMEOUT_SECONDS } from "../../web/src/wire/serverConfig.ts";
import { ALERT_WINDOW_MS } from "./alerts.ts";
import worker, {
  IMAGE_CHECK_RETRY_MS,
  IMAGE_CHECK_TIMEOUT_MS,
  SETTINGS_FINGERPRINT_KEY,
  Server,
  START_WAIT_MS,
  TOLD_AT_KEY,
  type Env,
} from "./index.ts";
import { FakeNetwork } from "./network.fake.ts";
import { DAILY_SWEEP_CRON, DAILY_SWEEP_PATH, SERVER_PORT, settingsFingerprint } from "./routing.ts";

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
/** The context a cron run gets, for a test that is not about what the run tells people. */
const CRON_CTX = { waitUntil: vi.fn() } as unknown as ExecutionContext;
/** #4118: the settings of most tests: the commit and nothing else. */
const SETTINGS = { DEPLOYED_COMMIT: COMMIT };
/** #4118: the settings after a secret changed. */
const CHANGED = { DEPLOYED_COMMIT: COMMIT, DEV_PROFILE_IDS: "profile-1" };
/** #4118: a secret the logs must never carry. */
const SECRET = "postgresql://postgres:hunter2@db.example.com/postgres";
const FINGERPRINT = await settingsFingerprint(SETTINGS);

/**
 * #4118: an object's storage over a Map, which two objects can share (a restart), holding `SETTINGS`'
 * fingerprint unless given another Map.
 */
function memoryStorage(entries = new Map<string, unknown>([[SETTINGS_FINGERPRINT_KEY, FINGERPRINT]])) {
  return {
    entries,
    get: vi.fn((key: string) => Promise.resolve(entries.get(key))),
    put: vi.fn((key: string, value: unknown) => {
      entries.set(key, value);
      return Promise.resolve();
    }),
  };
}

/** #4118: every argument a `console.warn` spy got, as text. */
function logged(warn: { mock: { calls: unknown[][] } }): string {
  return JSON.stringify(warn.mock.calls.flat().map(String));
}

function serverWith(running: boolean, storage = memoryStorage()): Server {
  const ctx = { container: { running }, waitUntil: vi.fn(), storage };
  return new Server(
    ctx as unknown as ConstructorParameters<typeof Server>[0],
    SETTINGS as unknown as ConstructorParameters<typeof Server>[1],
  );
}

/** A server whose `ctx` and `env` the test sets by hand; with no `storage` of its own it gets one holding `SETTINGS`' fingerprint. */
function serverWithCtx(ctx: unknown, env: unknown): Server {
  return new Server(
    { storage: memoryStorage(), ...(ctx as Record<string, unknown>) } as unknown as ConstructorParameters<typeof Server>[0],
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
    const storage = memoryStorage();

    await serverWith(true, storage).fetch(request());

    expect(library.stop).not.toHaveBeenCalled();
    expect(library.destroy).not.toHaveBeenCalled();
    // Its settings are the ones it started with, so there is nothing to remember.
    expect(storage.put).not.toHaveBeenCalled();
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

/** #4118: a redeploy that names the same commit but other settings restarts a running server. */
describe("#4118 the settings check on the first request", () => {
  /** A running container whose `stop` takes it down and whose start brings it back, as the library's would. */
  function runningContainer(running = true) {
    const container = { running };
    library.stop.mockImplementation(() => {
      container.running = false;
      return Promise.resolve();
    });
    library.startAndWaitForPorts.mockImplementation(() => {
      container.running = true;
      return Promise.resolve();
    });
    return container;
  }

  beforeEach(() => {
    resetLibrary();
  });

  it("pins its storage key", () => {
    expect(SETTINGS_FINGERPRINT_KEY).toBe("jackioh:server.settings:fingerprint");
    expect(SETTINGS_FINGERPRINT_KEY).not.toBe(TOLD_AT_KEY);
  });

  it("stops a server on the deployed commit whose settings changed, and the request starts the new one", async () => {
    const warn = vi.spyOn(console, "warn").mockImplementation(() => undefined);
    const container = runningContainer();
    const storage = memoryStorage();

    expect(await serverWithCtx({ container, storage }, CHANGED).fetch(request())).toBe(LISTENING);

    expect(library.stop).toHaveBeenCalledWith("SIGTERM");
    expect(library.destroy).not.toHaveBeenCalled();
    expect(library.startAndWaitForPorts).toHaveBeenCalledOnce();
    expect(library.startAndWaitForPorts.mock.invocationCallOrder[0]).toBeGreaterThan(library.stop.mock.invocationCallOrder[0]!);
    expect(warn.mock.calls.filter(([message]) => message === "server.settings_changed")).toEqual([["server.settings_changed"]]);
    expect(storage.entries.get(SETTINGS_FINGERPRINT_KEY)).toBe(await settingsFingerprint(CHANGED));
  });

  it("waits RETIRE_WAIT_MS for it, then destroys it", async () => {
    // `setImmediate` stays real: the fingerprint's digest settles on the event loop, which a fake clock does not drive.
    vi.useFakeTimers({ toFake: ["Date", "setTimeout", "clearTimeout"] });
    try {
      resetLibrary();
      vi.spyOn(console, "warn").mockImplementation(() => undefined);
      const storage = memoryStorage();
      // Waits for the stop itself, not a count of event-loop turns: on a busy runner the digest can take more.
      const stopped = new Promise<void>((resolve) => {
        library.stop.mockImplementation(() => {
          resolve();
          return Promise.resolve();
        });
      });

      const fetching = serverWithCtx({ container: { running: true }, storage }, CHANGED).fetch(request());
      await stopped;
      // Still draining: the destroy has to wait for the full wait, not a skipped or shortened one.
      await vi.advanceTimersByTimeAsync(29_000);
      expect(library.stop).toHaveBeenCalledWith("SIGTERM");
      expect(library.destroy).not.toHaveBeenCalled();
      // Past the wait: the server is still up, so it is destroyed.
      await vi.advanceTimersByTimeAsync(1_000);
      expect(library.destroy).toHaveBeenCalledOnce();
      await fetching;

      expect(storage.entries.get(SETTINGS_FINGERPRINT_KEY)).toBe(await settingsFingerprint(CHANGED));
    } finally {
      vi.useRealTimers();
    }
  });

  it("counts a missing fingerprint as changed", async () => {
    vi.spyOn(console, "warn").mockImplementation(() => undefined);
    const container = runningContainer();
    const storage = memoryStorage(new Map());

    await serverWithCtx({ container, storage }, SETTINGS).fetch(request());

    expect(library.stop).toHaveBeenCalledWith("SIGTERM");
    expect(storage.entries.get(SETTINGS_FINGERPRINT_KEY)).toBe(FINGERPRINT);
  });

  it("stores the fingerprint when no server runs, and a new object on that storage leaves the server it started", async () => {
    const container = runningContainer(false);
    const storage = memoryStorage(new Map());

    await serverWithCtx({ container, storage }, SETTINGS).fetch(request());

    expect(storage.entries.get(SETTINGS_FINGERPRINT_KEY)).toBe(FINGERPRINT);
    expect(library.containerFetch).not.toHaveBeenCalled();

    // A restart of the object: the same storage and settings, and the server the first one started.
    expect(await serverWithCtx({ container, storage }, SETTINGS).fetch(request())).toBe(LISTENING);

    expect(library.containerFetch).toHaveBeenCalledOnce();
    expect(library.stop).not.toHaveBeenCalled();
    expect(library.startAndWaitForPorts).toHaveBeenCalledOnce();
  });

  it("does not loop: after retiring for changed settings, a new object with the same settings leaves the server", async () => {
    vi.spyOn(console, "warn").mockImplementation(() => undefined);
    const container = runningContainer();
    const storage = memoryStorage();

    await serverWithCtx({ container, storage }, CHANGED).fetch(request());
    await serverWithCtx({ container, storage }, CHANGED).fetch(request());

    expect(library.stop).toHaveBeenCalledOnce();
    expect(library.startAndWaitForPorts).toHaveBeenCalledOnce();
  });

  it("remembers the settings after retiring an older commit too", async () => {
    const container = runningContainer();
    const storage = memoryStorage(new Map());
    library.containerFetch.mockResolvedValue(new Response(null, { headers: { "x-deployed-commit": "older" } }));

    await serverWithCtx({ container, storage }, SETTINGS).fetch(request());

    expect(library.stop).toHaveBeenCalledOnce();
    expect(storage.entries.get(SETTINGS_FINGERPRINT_KEY)).toBe(FINGERPRINT);

    // The server the request started is on the deployed commit: a new object on this storage leaves it.
    library.containerFetch.mockResolvedValue(new Response(null, { headers: { "x-deployed-commit": COMMIT } }));
    await serverWithCtx({ container, storage }, SETTINGS).fetch(request());

    expect(library.stop).toHaveBeenCalledOnce();
  });

  it("leaves the server running when the stored fingerprint cannot be read, and asks again after the retry interval", async () => {
    vi.useFakeTimers();
    try {
      resetLibrary();
      const warn = vi.spyOn(console, "warn").mockImplementation(() => undefined);
      const container = runningContainer();
      const storage = memoryStorage();
      storage.get.mockRejectedValueOnce(new Error("storage down"));
      const env = { ...CHANGED, DATABASE_URL: SECRET };
      const server = serverWithCtx({ container, storage }, env);

      expect(await server.fetch(request())).toBe(LISTENING);
      expect(library.stop).not.toHaveBeenCalled();
      expect(warn).toHaveBeenCalledWith("server.settings_unreadable", expect.any(Error));
      expect(logged(warn)).not.toContain("hunter2");
      expect(logged(warn)).not.toContain(await settingsFingerprint(env));

      // The storage reads again, and the settings are not the ones the server started with.
      vi.advanceTimersByTime(IMAGE_CHECK_RETRY_MS);
      expect(await server.fetch(request())).toBe(LISTENING);
      expect(library.stop).toHaveBeenCalledWith("SIGTERM");
    } finally {
      vi.useRealTimers();
    }
  });

  it("still answers when the fingerprint cannot be written, with no server running", async () => {
    const warn = vi.spyOn(console, "warn").mockImplementation(() => undefined);
    const container = runningContainer(false);
    const storage = memoryStorage(new Map());
    storage.put.mockRejectedValue(new Error("storage down"));
    const env = { DEPLOYED_COMMIT: COMMIT, DATABASE_URL: SECRET };

    expect(await serverWithCtx({ container, storage }, env).fetch(request())).toBe(LISTENING);

    expect(library.startAndWaitForPorts).toHaveBeenCalledOnce();
    expect(warn).toHaveBeenCalledWith("server.settings_unwritten", expect.any(Error));
    expect(logged(warn)).not.toContain("hunter2");
    expect(logged(warn)).not.toContain(await settingsFingerprint(env));
  });

  it("still answers when the fingerprint cannot be written after a retire", async () => {
    const warn = vi.spyOn(console, "warn").mockImplementation(() => undefined);
    const container = runningContainer();
    const storage = memoryStorage();
    storage.put.mockRejectedValue(new Error("storage down"));

    expect(await serverWithCtx({ container, storage }, CHANGED).fetch(request())).toBe(LISTENING);

    expect(library.stop).toHaveBeenCalledWith("SIGTERM");
    expect(warn).toHaveBeenCalledWith("server.settings_unwritten", expect.any(Error));
    expect(logged(warn)).not.toContain(await settingsFingerprint(CHANGED));
  });

  it("stores nothing when the retire fails", async () => {
    const warn = vi.spyOn(console, "warn").mockImplementation(() => undefined);
    library.stop.mockRejectedValue(new Error("refused"));
    const storage = memoryStorage();

    expect(await serverWithCtx({ container: { running: true }, storage }, CHANGED).fetch(request())).toBe(LISTENING);

    expect(warn).toHaveBeenCalledWith("server.retire_failed", expect.any(Error));
    expect(storage.put).not.toHaveBeenCalled();
    expect(storage.entries.get(SETTINGS_FINGERPRINT_KEY)).toBe(FINGERPRINT);
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
    const tellUnreachable = vi.fn(() => Promise.resolve());
    const env = {
      SERVER: { idFromName: vi.fn((name: string) => name), get: vi.fn((_id: string) => ({ fetch, tellUnreachable })) },
    } as unknown as Env;
    return { env, fetch };
  }

  it("asks the server for readiness", async () => {
    const { env, fetch } = scheduledEnv(200);

    await worker.scheduled({} as Parameters<typeof worker.scheduled>[0], env, CRON_CTX);

    expect(env.SERVER.idFromName).toHaveBeenCalledWith("main");
    expect(fetch).toHaveBeenCalledOnce();
    expect((fetch.mock.calls[0]?.[0] as Request).url).toBe("http://container/readyz");
  });

  it("fails naming the path and the status when the server is not ready", async () => {
    const { env } = scheduledEnv(500);

    await expect(worker.scheduled({} as Parameters<typeof worker.scheduled>[0], env, CRON_CTX)).rejects.toThrow(
      "the server's /readyz answered 500",
    );
  });

  it("CLAUDE.md rule 13: the daily cron still asks for readiness, not the sweep", async () => {
    const { env, fetch } = scheduledEnv(200);

    await worker.scheduled({ cron: "23 14 * * *" } as Parameters<typeof worker.scheduled>[0], env, CRON_CTX);

    expect((fetch.mock.calls[0]?.[0] as Request).url).toBe("http://container/readyz");
  });
});

describe("#3727 the daily games' hourly sweep", () => {
  function sweepEnv(status: number): { env: Env; fetch: ReturnType<typeof vi.fn> } {
    const fetch = vi.fn(() => Promise.resolve(new Response("{}", { status })));
    const tellUnreachable = vi.fn(() => Promise.resolve());
    const env = {
      SERVER: { idFromName: vi.fn((name: string) => name), get: vi.fn((_id: string) => ({ fetch, tellUnreachable })) },
    } as unknown as Env;
    return { env, fetch };
  }

  it("posts the sweep to the server on the hourly cron", async () => {
    const { env, fetch } = sweepEnv(200);

    await worker.scheduled({ cron: DAILY_SWEEP_CRON } as Parameters<typeof worker.scheduled>[0], env, CRON_CTX);

    expect(fetch).toHaveBeenCalledOnce();
    const request = fetch.mock.calls[0]?.[0] as Request;
    expect(request.url).toBe(`http://container${DAILY_SWEEP_PATH}`);
    expect(request.method).toBe("POST");
  });

  it("fails naming the path and the status when the sweep fails", async () => {
    const { env } = sweepEnv(500);

    await expect(
      worker.scheduled({ cron: DAILY_SWEEP_CRON } as Parameters<typeof worker.scheduled>[0], env, CRON_CTX),
    ).rejects.toThrow("the server's /daily-sweep answered 500");
  });
});

/** What a request or a cron run tells people (#3618), with the alert's two services faked (network.fake.ts). */
describe("#3618 server.unreachable", () => {
  const T0 = Date.parse("2026-11-15T03:00:00Z");
  const TITLE = "Patch: alert server.unreachable on jackioh.com";
  const ENV = {
    DEPLOYED_COMMIT: COMMIT,
    PUBLIC_ORIGINS: "https://jackioh.com",
    CREATE_ISSUES_TOKEN: "ghp_token",
    ALERT_RESEND_API_KEY: "re_key",
    ALERT_EMAILS: "owner@example.com",
  };

  let network: FakeNetwork;

  /** A Server over a Map-backed storage, which several can share (a restart), and the promises it gave `waitUntil`. */
  function alerting(env: Record<string, string> = ENV, storage = new Map<string, unknown>()) {
    const waiting: Promise<unknown>[] = [];
    const ctx = {
      container: { running: false },
      waitUntil: vi.fn((promise: Promise<unknown>) => {
        waiting.push(promise);
      }),
      storage: {
        get: vi.fn((key: string) => Promise.resolve(storage.get(key))),
        put: vi.fn((key: string, value: unknown) => {
          storage.set(key, value);
          return Promise.resolve();
        }),
      },
    };
    const server = serverWithCtx(ctx, env);
    return { server, ctx, storage, settled: () => Promise.all(waiting.splice(0)) };
  }

  /** One request to a server whose start fails, then what it told. */
  async function failingRequest(handle: ReturnType<typeof alerting>): Promise<Response> {
    const answer = await handle.server.fetch(request());
    await handle.settled();
    return answer;
  }

  beforeEach(() => {
    resetLibrary();
    library.startAndWaitForPorts.mockRejectedValue(new Error("the container failed to start"));
    vi.useFakeTimers({ toFake: ["Date"] });
    vi.setSystemTime(T0);
    network = new FakeNetwork();
    vi.stubGlobal("fetch", network.fetch);
    for (const level of ["info", "warn", "error"] as const) vi.spyOn(console, level).mockImplementation(() => undefined);
  });

  afterEach(() => {
    vi.useRealTimers();
    vi.unstubAllGlobals();
  });

  describe("from a request", () => {
    it("opens one issue and sends one email for a boot loop, and every request still gets the library's answer", async () => {
      const handle = alerting();
      const refused = new Response("Failed to start container", { status: 500 });
      library.fetch.mockResolvedValue(refused);

      expect(await handle.server.fetch(request())).toBe(refused);
      expect(await handle.server.fetch(request())).toBe(refused);
      expect(await handle.server.fetch(request())).toBe(refused);
      await handle.settled();

      expect(network.to("search")).toHaveLength(1);
      expect(network.to("create")).toHaveLength(1);
      expect(network.to("create")[0]?.body?.title).toBe(TITLE);
      expect(network.to("comment")).toHaveLength(0);
      expect(network.to("email")).toHaveLength(1);
      expect(network.to("email")[0]?.body?.text).toContain("cause: start");
      expect(network.to("email")[0]?.body?.text).toContain("message: the container failed to start");
      expect(network.to("email")[0]?.headers["idempotency-key"]).toBe(
        `jackioh-alert/jackioh.com/server.unreachable/${FakeNetwork.issueUrl(1)}`,
      );
    });

    it("tells nothing again within the window", async () => {
      const handle = alerting();
      await failingRequest(handle);
      network.calls.length = 0;

      vi.setSystemTime(T0 + ALERT_WINDOW_MS - 1_000);
      await failingRequest(handle);

      expect(network.calls).toHaveLength(0);
    });

    it("comments, with no email, once an issue is open and the window has passed", async () => {
      const handle = alerting();
      await failingRequest(handle);
      network.calls.length = 0;
      network.openIssues = [{ number: 1, title: TITLE, html_url: FakeNetwork.issueUrl(1) }];

      vi.setSystemTime(T0 + ALERT_WINDOW_MS);
      await failingRequest(handle);

      expect(network.to("comment")).toHaveLength(1);
      expect(network.to("comment")[0]?.url.pathname).toBe("/repos/Jlore-Studios/JackiOh/issues/1/comments");
      expect(network.to("create")).toHaveLength(0);
      expect(network.to("email")).toHaveLength(0);
    });

    it("opens a new issue and emails again once the issue is closed", async () => {
      const handle = alerting();
      await failingRequest(handle);
      network.calls.length = 0;

      vi.setSystemTime(T0 + ALERT_WINDOW_MS);
      await failingRequest(handle);

      expect(network.to("create")).toHaveLength(1);
      expect(network.to("email")).toHaveLength(1);
      expect(network.to("email")[0]?.headers["idempotency-key"]).toBe(
        `jackioh-alert/jackioh.com/server.unreachable/${FakeNetwork.issueUrl(2)}`,
      );
    });

    it("keeps when it told in the object's storage, under a key of its own", async () => {
      const handle = alerting();
      await failingRequest(handle);

      expect(handle.storage.get(TOLD_AT_KEY)).toBe(T0);
      expect(TOLD_AT_KEY).toBe("jackioh:server.unreachable:toldAt");
      // #4118: the start's settings are kept in the same storage, under a key of their own.
      expect(handle.ctx.storage.put.mock.calls.filter(([key]) => key === TOLD_AT_KEY)).toHaveLength(1);
    });

    it("remembers across a restart: a new object on the same storage tells nothing inside the window", async () => {
      const first = alerting();
      await failingRequest(first);
      network.calls.length = 0;

      vi.setSystemTime(T0 + 60_000);
      await failingRequest(alerting(ENV, first.storage));

      expect(network.calls).toHaveLength(0);
    });

    it("tells again after a restart once the window has passed", async () => {
      const first = alerting();
      await failingRequest(first);
      network.calls.length = 0;

      vi.setSystemTime(T0 + ALERT_WINDOW_MS);
      await failingRequest(alerting(ENV, first.storage));

      expect(network.to("create")).toHaveLength(1);
    });

    it("makes one notice for requests that fail together", async () => {
      const handle = alerting();

      await Promise.all([handle.server.fetch(request()), handle.server.fetch(request())]);
      await handle.settled();

      expect(network.to("create")).toHaveLength(1);
      expect(network.to("email")).toHaveLength(1);
      expect(handle.ctx.storage.get).toHaveBeenCalledOnce();
    });

    it("sends from waitUntil, never while the request waits", async () => {
      vi.stubGlobal("fetch", () => new Promise<Response>(() => undefined));
      const handle = alerting();

      const answer = await handle.server.fetch(request());

      expect(answer).toBe(LISTENING);
      expect(handle.ctx.waitUntil).toHaveBeenCalledOnce();
    });

    it("is not raised for a client that hung up", async () => {
      const handle = alerting();

      await handle.server.fetch(new Request("https://jackioh.com/api/catalog", { signal: AbortSignal.abort() }));

      expect(handle.ctx.waitUntil).not.toHaveBeenCalled();
      expect(network.calls).toHaveLength(0);
    });

    it("says nothing for a start that succeeds", async () => {
      library.startAndWaitForPorts.mockResolvedValue();
      const handle = alerting();

      await failingRequest(handle);

      expect(handle.ctx.waitUntil).not.toHaveBeenCalled();
      expect(network.calls).toHaveLength(0);
    });

    it("says nothing with no token and no email key, and keeps no time", async () => {
      const handle = alerting({ DEPLOYED_COMMIT: COMMIT });

      await failingRequest(handle);

      expect(network.calls).toHaveLength(0);
      expect(handle.ctx.storage.get).not.toHaveBeenCalled();
      expect(handle.storage.has(TOLD_AT_KEY)).toBe(false);
    });

    it("falls back to the day's one email when GitHub fails", async () => {
      network.status.create = 500;
      const handle = alerting();

      await failingRequest(handle);

      expect(network.to("email")).toHaveLength(1);
      expect(network.to("email")[0]?.headers["idempotency-key"]).toBe(
        "jackioh-alert/jackioh.com/server.unreachable/2026-11-15",
      );
    });

    it("tells what it can when the object's storage fails", async () => {
      const warn = vi.spyOn(console, "warn").mockImplementation(() => undefined);
      const handle = alerting();
      handle.ctx.storage.get.mockRejectedValue(new Error("storage down"));
      handle.ctx.storage.put.mockRejectedValue(new Error("storage down"));

      await failingRequest(handle);

      expect(network.to("create")).toHaveLength(1);
      expect(warn).toHaveBeenCalledWith("server.alert_state_unreadable", expect.any(Error));
      expect(warn).toHaveBeenCalledWith("server.alert_state_unwritten", expect.any(Error));
    });

    it("never rejects, even when telling fails outright", async () => {
      const error = vi.spyOn(console, "error").mockImplementation(() => undefined);
      vi.stubGlobal("fetch", () => {
        throw new Error("fetch is gone");
      });
      const handle = alerting();

      await expect(failingRequest(handle)).resolves.toBe(LISTENING);

      expect(error).toHaveBeenCalledWith("alert.issue.failed", "fetch is gone");
    });

    it("tells a failure that is no Error by its text", async () => {
      library.startAndWaitForPorts.mockRejectedValue("no container");
      const handle = alerting();

      await failingRequest(handle);

      expect(network.to("email")[0]?.body?.text).toContain("message: no container");
    });
  });

  describe("from the daily cron", () => {
    function cronEnv(fetch: () => Promise<Response>) {
      const tellUnreachable = vi.fn(() => Promise.resolve());
      const env = {
        SERVER: { idFromName: vi.fn((name: string) => name), get: vi.fn((_id: string) => ({ fetch, tellUnreachable })) },
      } as unknown as Env;
      const waitUntil = vi.fn();
      return { env, tellUnreachable, waitUntil, ctx: { waitUntil } as unknown as ExecutionContext };
    }
    const daily = {} as Parameters<typeof worker.scheduled>[0];

    it("alerts when /readyz answers an error, from waitUntil, and still fails the run", async () => {
      const { env, tellUnreachable, waitUntil, ctx } = cronEnv(() => Promise.resolve(new Response("no", { status: 500 })));

      await expect(worker.scheduled(daily, env, ctx)).rejects.toThrow("the server's /readyz answered 500");

      expect(tellUnreachable).toHaveBeenCalledExactlyOnceWith({
        cause: "readyz",
        message: "the server's /readyz answered 500",
      });
      expect(waitUntil).toHaveBeenCalledOnce();
      expect(waitUntil).toHaveBeenCalledWith(tellUnreachable.mock.results[0]?.value);
    });

    it("alerts when the server cannot be reached, and fails the run", async () => {
      const { env, tellUnreachable, waitUntil, ctx } = cronEnv(() => Promise.reject(new Error("container never started")));

      await expect(worker.scheduled(daily, env, ctx)).rejects.toThrow(
        "the server's /readyz failed: container never started",
      );

      expect(tellUnreachable).toHaveBeenCalledExactlyOnceWith({
        cause: "readyz",
        message: "the server's /readyz failed: container never started",
      });
      expect(waitUntil).toHaveBeenCalledOnce();
    });

    it("names a failure that is no Error by its text", async () => {
      const { env, tellUnreachable, ctx } = cronEnv(() => Promise.reject("gone"));

      await expect(worker.scheduled(daily, env, ctx)).rejects.toThrow("the server's /readyz failed: gone");

      expect(tellUnreachable).toHaveBeenCalledOnce();
    });

    it("says nothing when /readyz answers", async () => {
      const { env, tellUnreachable, waitUntil, ctx } = cronEnv(() => Promise.resolve(new Response("ready")));

      await worker.scheduled(daily, env, ctx);

      expect(tellUnreachable).not.toHaveBeenCalled();
      expect(waitUntil).not.toHaveBeenCalled();
    });

    it("does not alert for the hourly sweep that fails", async () => {
      const { env, tellUnreachable, waitUntil, ctx } = cronEnv(() => Promise.resolve(new Response("no", { status: 500 })));

      await expect(
        worker.scheduled({ cron: DAILY_SWEEP_CRON } as Parameters<typeof worker.scheduled>[0], env, ctx),
      ).rejects.toThrow("the server's /daily-sweep answered 500");

      expect(tellUnreachable).not.toHaveBeenCalled();
      expect(waitUntil).not.toHaveBeenCalled();
    });

    it("tells people through the object the request path uses, so the two share one window", async () => {
      // The stub the cron holds is the Durable Object: its `tellUnreachable` is the method a request calls.
      const handle = alerting();
      const { env, waitUntil, ctx } = cronEnv(() => Promise.resolve(new Response("no", { status: 500 })));
      (env.SERVER.get as unknown as ReturnType<typeof vi.fn>).mockReturnValue({
        fetch: () => Promise.resolve(new Response("no", { status: 500 })),
        tellUnreachable: (fields: Record<string, string>) => handle.server.tellUnreachable(fields),
      });

      await expect(worker.scheduled(daily, env, ctx)).rejects.toThrow();
      await Promise.all(waitUntil.mock.calls.map(([promise]) => promise as Promise<unknown>));
      await failingRequest(handle);

      expect(network.to("create")).toHaveLength(1);
      expect(network.to("email")).toHaveLength(1);
      expect(network.to("email")[0]?.body?.text).toContain("cause: readyz");
    });
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
