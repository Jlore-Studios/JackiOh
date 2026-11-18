// The Worker in front of the site and its game server (wrangler.jsonc, apps/edge/README.md).
//
// Static files never reach this code but one: wrangler.jsonc's `run_worker_first` sends only `/api/*`,
// `/ws/*` and the engine's brotli-compressed module here. The first two go to one container running
// crates/server's image, through the `Server` Durable Object, which starts it on the first request and
// stops it once nothing has reached it for `SLEEP_AFTER`. An open WebSocket counts as a request in
// flight, so a live match keeps it awake. #3183: the module is the stored file, served with its
// `Content-Encoding` (routing.ts `precompressedModule`). #3618: a server that cannot start, or that the
// daily `/readyz` cannot reach, cannot tell people itself, so the Worker raises the alert (alerts.ts), at
// most once an hour, from `ctx.waitUntil`.

import { Container } from "@cloudflare/containers";

import { ALERT_WINDOW_MS, canTell, tell } from "./alerts.ts";
import {
  DAILY_SWEEP_CRON,
  DAILY_SWEEP_PATH,
  HEALTH_PATH,
  READY_PATH,
  SERVER_PORT,
  containerEnv,
  forServer,
  isModulePath,
  isServerPath,
  precompressedModule,
  settingsFingerprint,
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

/**
 * #3618: the key in the object's storage that holds when the Worker last told people the server was
 * unreachable (epoch ms). Prefixed, so it cannot meet the container library's own keys.
 */
export const TOLD_AT_KEY = "jackioh:server.unreachable:toldAt";

/**
 * #4118: the key in the object's storage that holds the fingerprint (routing.ts `settingsFingerprint`) of the
 * settings the container was last started with. Prefixed, like `TOLD_AT_KEY`; the value is never logged or sent.
 */
export const SETTINGS_FINGERPRINT_KEY = "jackioh:server.settings:fingerprint";

/** The one instance every request reaches: the server holds every live match in memory. */
const INSTANCE = "main";

/** How long a server on an older image gets to drain (its `SHUTDOWN_DRAIN_SECONDS`, 20) before it is killed. */
const RETIRE_WAIT_MS = 30_000;
const RETIRE_POLL_MS = 500;
/** The least time between two image checks that failed: a request inside it skips the check. */
export const IMAGE_CHECK_RETRY_MS = 10_000;
/** How long the `/healthz` fetch of the image check may take before it counts as failed. */
export const IMAGE_CHECK_TIMEOUT_MS = 5_000;

export interface Env extends ServerSettings {
  SERVER: DurableObjectNamespace<Server>;
  /** The web build's files (wrangler.jsonc `assets.binding`): the module this Worker serves is one. */
  ASSETS: Fetcher;
}

export class Server extends Container<Env> {
  override defaultPort = SERVER_PORT;
  override sleepAfter = SLEEP_AFTER;
  override envVars = containerEnv(this.env);

  /** True once a check has settled: a deploy restarts this object, so the first request after one checks the running image. */
  private imageVerified = false;
  /** The check in flight, shared by the requests that arrive while it runs. */
  private imageCheck: Promise<void> | undefined;
  /**
   * When the last check began (`Date.now()`): a failed one is not retried before `IMAGE_CHECK_RETRY_MS` has
   * passed. Before the first, -Infinity, which no retry interval reaches.
   */
  private imageCheckedAt = Number.NEGATIVE_INFINITY;
  /** #3618: when the Worker last told people (`TOLD_AT_KEY`'s value once loaded); -Infinity before the first notice. */
  private toldAt = Number.NEGATIVE_INFINITY;
  /** The read of `TOLD_AT_KEY` that sets `toldAt`, made once per object. */
  private toldAtLoad: Promise<void> | undefined;

  override async fetch(request: Request): Promise<Response> {
    await this.ensureImageChecked();
    await this.awaitStart(request.signal);
    return super.fetch(request);
  }

  /** Checks the running image until a check settles; a request inside the retry interval of a failed one goes on unchecked. */
  private async ensureImageChecked(): Promise<void> {
    if (this.imageVerified) return;
    if (this.imageCheck === undefined) {
      if (Date.now() - this.imageCheckedAt < IMAGE_CHECK_RETRY_MS) return;
      this.imageCheckedAt = Date.now();
      // `retireOlderImage` never rejects, so this always runs and clears the check.
      this.imageCheck = this.retireOlderImage().then((settled) => {
        this.imageVerified = settled;
        this.imageCheck = undefined;
      });
    }
    await this.imageCheck;
  }

  /**
   * Starts a sleeping server and waits up to `START_WAIT_MS` for it to listen. A server that never
   * does is left to `super.fetch`, which tries once more and turns the failure into its own answer.
   * #3618: the failure is also told to people, from `ctx.waitUntil` so the request never waits for it,
   * unless the client hung up (its abort ends this wait, and no fault of the server's).
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
      if (!signal.aborted) {
        const message = error instanceof Error ? error.message : String(error);
        this.ctx.waitUntil(this.tellUnreachable({ cause: "start", message }));
      }
    }
  }

  /**
   * #3618: tells people the server cannot be started or reached, unless it did within `ALERT_WINDOW_MS`
   * (alerts.ts). Public, since the daily cron calls it over Durable Object RPC; never rejects.
   */
  async tellUnreachable(fields: Record<string, string>): Promise<void> {
    try {
      if (!canTell(this.env)) return;
      const now = Date.now();
      if (!(await this.claimNotice(now))) return;
      await tell(this.env, { fields, at: now });
    } catch (error) {
      console.error("server.alert_failed", error);
    }
  }

  /**
   * Whether a notice may go at `now`, which it then counts as told, in memory and in storage (so a new
   * object after a restart knows). The in-memory claim comes before the storage write's await: requests
   * that wait on one failing boot fail together, and two of them must not both read the old time.
   */
  private async claimNotice(now: number): Promise<boolean> {
    // A storage that fails is no reason to tell nobody: an unread time is taken as never told, an unwritten one costs a repeat.
    this.toldAtLoad ??= this.ctx.storage.get<number>(TOLD_AT_KEY).then(
      (at) => {
        this.toldAt = at ?? Number.NEGATIVE_INFINITY;
      },
      (error: unknown) => {
        console.warn("server.alert_state_unreadable", error);
      },
    );
    await this.toldAtLoad;
    if (now - this.toldAt < ALERT_WINDOW_MS) return false;
    this.toldAt = now;
    try {
      await this.ctx.storage.put(TOLD_AT_KEY, now);
    } catch (error) {
      console.warn("server.alert_state_unwritten", error);
    }
    return true;
  }

  /**
   * A container keeps the image it was started with when a new version deploys, so a server still
   * running from before the deploy is stopped (SIGTERM: it drains, and every live match rebuilds
   * from its seed and log when its sockets return, SPEC §9.5) and the next request starts the new
   * one. The running server names its commit in `/healthz`'s `x-deployed-commit`.
   * #4118: it keeps the settings it started with too, so a server on the deployed commit is stopped the
   * same way when the Worker's settings are not those (a redeploy after a secret change names the same
   * commit): the fingerprint of the settings the container was last started with is in the object's
   * storage (`SETTINGS_FINGERPRINT_KEY`), and an absent one counts as changed. The fingerprint is stored
   * when the check settles with no server running, or once a retire has (the next start takes the
   * Worker's settings). Resolves true when the check is settled (no server running, its commit and
   * settings current, or it retired) and false when a later request should ask again (the server did
   * not answer within `IMAGE_CHECK_TIMEOUT_MS`, the stored fingerprint could not be read, or stopping
   * the server failed); it never rejects.
   */
  private async retireOlderImage(): Promise<boolean> {
    if (!this.ctx.container?.running) {
      // Nothing to stop: the server the request starts next takes these settings, so remember them.
      await this.rememberSettings();
      return true;
    }
    let running: string | null;
    try {
      const health = await this.containerFetch(
        `http://container${HEALTH_PATH}`,
        { signal: AbortSignal.timeout(IMAGE_CHECK_TIMEOUT_MS) },
        SERVER_PORT,
      );
      // `containerFetch` turns a failed or aborted fetch into a 500 (or 503, 429) answer rather than rejecting, and
      // such an answer names no commit: it is a failed check, not a server on an older image.
      if (!health.ok) {
        console.warn("server.image_check_failed", health.status);
        return false;
      }
      running = health.headers.get("x-deployed-commit");
    } catch (error) {
      // A server that cannot answer is left to the request that follows, which reports it; a later one asks again.
      console.warn("server.image_check_failed", error);
      return false;
    }
    if (running === (this.env.DEPLOYED_COMMIT ?? null)) {
      try {
        const started = await this.ctx.storage.get<string>(SETTINGS_FINGERPRINT_KEY);
        if (started === (await settingsFingerprint(this.env))) return true;
      } catch (error) {
        // A storage fault is no reason to restart a live site; a later request asks again.
        console.warn("server.settings_unreadable", error);
        return false;
      }
      // Names no setting and carries no digest: the logs are not for secrets.
      console.warn("server.settings_changed");
    }
    try {
      await this.stop("SIGTERM");
      const deadline = Date.now() + RETIRE_WAIT_MS;
      while (this.ctx.container.running && Date.now() < deadline) {
        await new Promise((resolve) => setTimeout(resolve, RETIRE_POLL_MS));
      }
      if (this.ctx.container.running) await this.destroy();
    } catch (error) {
      // The request goes on to start (or reach) a server; a later one tries the retire again.
      console.warn("server.retire_failed", error);
      return false;
    }
    await this.rememberSettings();
    return true;
  }

  /**
   * #4118: keeps the fingerprint of the settings the next start takes. A write that fails is logged and
   * costs at most one more restart at a later deploy; it never rejects.
   */
  private async rememberSettings(): Promise<void> {
    try {
      await this.ctx.storage.put(SETTINGS_FINGERPRINT_KEY, await settingsFingerprint(this.env));
    } catch (error) {
      console.warn("server.settings_unwritten", error);
    }
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

  /**
   * wrangler.jsonc's daily cron: wakes the server so its hourly loops run and Supabase sees the project in use.
   * #3618: a `/readyz` that fails is told to people, from `ctx.waitUntil`, before the run fails.
   */
  async scheduled(controller, env, ctx): Promise<void> {
    // #3727: the hourly cron runs the daily games' sweep; the daily one wakes the server as before.
    if (controller.cron === DAILY_SWEEP_CRON) {
      const swept = await server(env).fetch(new Request(`http://container${DAILY_SWEEP_PATH}`, { method: "POST" }));
      if (!swept.ok) throw new Error(`the server's ${DAILY_SWEEP_PATH} answered ${swept.status}`);
      return;
    }
    const stub = server(env);
    let failure: string | undefined;
    try {
      const ready = await stub.fetch(new Request(`http://container${READY_PATH}`));
      if (!ready.ok) failure = `the server's ${READY_PATH} answered ${ready.status}`;
    } catch (error) {
      failure = `the server's ${READY_PATH} failed: ${error instanceof Error ? error.message : String(error)}`;
    }
    if (failure !== undefined) {
      // A start that failed inside that fetch told already (`awaitStart`); the object's window holds this repeat.
      ctx.waitUntil(stub.tellUnreachable({ cause: "readyz", message: failure }));
      throw new Error(failure);
    }
  },
} satisfies ExportedHandler<Env>;
