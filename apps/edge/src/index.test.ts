// Holds how long a request waits for a sleeping server, with the container library replaced: its
// real class needs Cloudflare's runtime.

import { beforeEach, describe, expect, it, vi } from "vitest";

import { API_REQUEST_TIMEOUT_SECONDS } from "../../web/src/wire/serverConfig.ts";
import { Server, START_WAIT_MS } from "./index.ts";
import { SERVER_PORT } from "./routing.ts";

const library = vi.hoisted(() => ({
  fetch: vi.fn<(request: Request) => Promise<Response>>(),
  getState: vi.fn<() => Promise<{ status: string }>>(),
  startAndWaitForPorts: vi.fn<(options: unknown) => Promise<void>>(),
  containerFetch: vi.fn<() => Promise<Response>>(),
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
    containerFetch(): Promise<Response> {
      return library.containerFetch();
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
