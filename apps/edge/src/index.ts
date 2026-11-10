// The Worker in front of the site and its game server (wrangler.jsonc, apps/edge/README.md).
//
// Static files never reach this code but one: wrangler.jsonc's `run_worker_first` sends only `/api/*`,
// `/ws/*` and the engine's brotli-compressed module here. The first two go to one container running
// crates/server's image, through the `Server` Durable Object, which starts it on the first request and
// stops it once nothing has reached it for `SLEEP_AFTER`. An open WebSocket counts as a request in
// flight, so a live match keeps it awake. #3183: the module is the stored file, served with its
// `Content-Encoding` (routing.ts `precompressedModule`).

import { Container } from "@cloudflare/containers";

import {
  HEALTH_PATH,
  READY_PATH,
  SERVER_PORT,
  containerEnv,
  forServer,
  isModulePath,
  isServerPath,
  precompressedModule,
  type ServerSettings,
} from "./routing.ts";

/** How long the server runs with no request before it is stopped; the next request starts it again. */
const SLEEP_AFTER = "5m";

/**
 * How long a request waits for a sleeping server to listen. The library's own wait is 20 s, which a
 * wake with work to do outran, and its first caller got a 500 while the boot went on. It stays under
 * the client's `API_REQUEST_TIMEOUT_SECONDS`, so the client hears this answer, not its own timeout.
 */
export const START_WAIT_MS = 60_000;

/** The one instance every request reaches: the server holds every live match in memory. */
const INSTANCE = "main";

/** How long a server on an older image gets to drain (its `SHUTDOWN_DRAIN_SECONDS`, 20) before it is killed. */
const RETIRE_WAIT_MS = 30_000;
const RETIRE_POLL_MS = 500;

export interface Env extends ServerSettings {
  SERVER: DurableObjectNamespace<Server>;
  /** The web build's files (wrangler.jsonc `assets.binding`): the module this Worker serves is one. */
  ASSETS: Fetcher;
}

export class Server extends Container<Env> {
  override defaultPort = SERVER_PORT;
  override sleepAfter = SLEEP_AFTER;
  override envVars = containerEnv(this.env);

  /** Settles once per object: a deploy restarts this object, so the first request after one checks the running image. */
  private imageChecked: Promise<void> | undefined;

  override async fetch(request: Request): Promise<Response> {
    this.imageChecked ??= this.retireOlderImage();
    await this.imageChecked;
    await this.awaitStart(request.signal);
    return super.fetch(request);
  }

  /**
   * Starts a sleeping server and waits up to `START_WAIT_MS` for it to listen. A server that never
   * does is left to `super.fetch`, which tries once more and turns the failure into its own answer.
   */
  private async awaitStart(signal: AbortSignal): Promise<void> {
    if (this.ctx.container?.running && (await this.getState()).status === "healthy") return;
    try {
      await this.startAndWaitForPorts({
        ports: SERVER_PORT,
        cancellationOptions: { abort: signal, portReadyTimeoutMS: START_WAIT_MS },
      });
    } catch (error) {
      console.warn("server.start_failed", error);
    }
  }

  /**
   * A container keeps the image it was started with when a new version deploys, so a server still
   * running from before the deploy is stopped (SIGTERM: it drains, and every live match rebuilds
   * from its seed and log when its sockets return, SPEC §9.5) and the next request starts the new
   * one. The running server names its commit in `/healthz`'s `x-deployed-commit`.
   */
  private async retireOlderImage(): Promise<void> {
    if (!this.ctx.container?.running) return;
    let running: string | null;
    try {
      const health = await this.containerFetch(`http://container${HEALTH_PATH}`, {}, SERVER_PORT);
      running = health.headers.get("x-deployed-commit");
    } catch (error) {
      // A server that cannot answer is left to the request that follows, which reports it.
      console.warn("server.image_check_failed", error);
      return;
    }
    if (running === (this.env.DEPLOYED_COMMIT ?? null)) return;
    await this.stop("SIGTERM");
    const deadline = Date.now() + RETIRE_WAIT_MS;
    while (this.ctx.container.running && Date.now() < deadline) {
      await new Promise((resolve) => setTimeout(resolve, RETIRE_POLL_MS));
    }
    if (this.ctx.container.running) await this.destroy();
  }
}

function server(env: Env): DurableObjectStub<Server> {
  return env.SERVER.get(env.SERVER.idFromName(INSTANCE), { locationHint: "enam" });
}

export default {
  async fetch(request, env): Promise<Response> {
    const { pathname } = new URL(request.url);
    if (isModulePath(pathname)) return precompressedModule(await env.ASSETS.fetch(request));
    if (!isServerPath(pathname)) return new Response("Not found", { status: 404 });
    return server(env).fetch(forServer(request));
  },

  /** wrangler.jsonc's daily cron: wakes the server so its hourly loops run and Supabase sees the project in use. */
  async scheduled(_controller, env): Promise<void> {
    const ready = await server(env).fetch(new Request(`http://container${READY_PATH}`));
    if (!ready.ok) throw new Error(`the server's ${READY_PATH} answered ${ready.status}`);
  },
} satisfies ExportedHandler<Env>;
