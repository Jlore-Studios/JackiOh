// What the Worker decides without the Workers runtime, so vitest can hold it: which paths are the
// server's, what the server is told about the caller, and which settings reach the container.

/** The paths the Worker hands to the server; wrangler.jsonc's `assets.run_worker_first` names the same, and every other path is a static asset. */
export const SERVER_ROUTES = ["/api/*", "/ws/*"] as const;

/** `SERVER_ROUTES` as path prefixes. */
const SERVER_PREFIXES = SERVER_ROUTES.map((route) => route.slice(0, -1));

export function isServerPath(pathname: string): boolean {
  return SERVER_PREFIXES.some((prefix) => pathname.startsWith(prefix));
}

/**
 * #3183: the engine's WebAssembly module, which the build ships brotli-compressed (scripts/build-wasm.sh)
 * and Vite names `jackioh_wasm_bg.wasm-<hash>.br`. Cloudflare serves a stored file with the type of its
 * extension and no `Content-Encoding`, and a `_headers` rule cannot add one (the asset Worker sets it on
 * a response whose body the runtime would then compress again), so this Worker serves it.
 */
export const MODULE_ROUTES = ["/assets/*.wasm-*.br"] as const;

/** Every path wrangler.jsonc's `assets.run_worker_first` sends here: the server's, and the module's. */
export const WORKER_ROUTES = [...SERVER_ROUTES, ...MODULE_ROUTES] as const;

/** `MODULE_ROUTES`' one glob, as Cloudflare's router reads a `run_worker_first` glob: `*` is any run of characters. */
const MODULE_PATH = /^\/assets\/.*\.wasm-.*\.br$/u;

export function isModulePath(pathname: string): boolean {
  return MODULE_PATH.test(pathname);
}

/**
 * How a browser may keep the module: a year, as `_headers`' rule for `/assets/*` says of every built file,
 * whose name carries its content's hash (apps/web/public/_headers; config.test.ts holds the two alike).
 */
export const MODULE_CACHE_CONTROL = "public, max-age=31536000, immutable";

/**
 * #3183: the stored `.br` as the browser takes it: the same bytes, with `Content-Encoding: br` and the
 * module's own type, so the browser decodes it as it arrives and `instantiateStreaming` compiles it.
 * `encodeBody: "manual"` tells the runtime the body is encoded already; without it the runtime would
 * compress it to match the header, and the page would get brotli inside brotli. The stored answer's
 * other headers (its ETag, its year-long `Cache-Control`, given if it came without) are kept, and an answer with no file to send
 * (a 304 for a copy the browser holds, a 404) is passed on as it is.
 */
export function precompressedModule(stored: Response): Response {
  if (stored.status !== 200) return stored;
  const headers = new Headers(stored.headers);
  headers.set("Content-Type", "application/wasm");
  headers.set("Content-Encoding", "br");
  if (!headers.has("Cache-Control")) headers.set("Cache-Control", MODULE_CACHE_CONTROL);
  return new Response(stored.body, { status: 200, headers, encodeBody: "manual" });
}

/**
 * The request as the server sees it. R190: the per-IP limits read the `X-Forwarded-For` entry
 * `TRUSTED_PROXY_HOPS` places from the right, so the Worker replaces whatever the caller wrote with
 * the one address Cloudflare saw (`CF-Connecting-IP`), and `TRUSTED_PROXY_HOPS` is 1.
 */
export function forServer(request: Request): Request {
  const headers = new Headers(request.headers);
  const caller = request.headers.get("CF-Connecting-IP");
  if (caller === null) headers.delete("X-Forwarded-For");
  else headers.set("X-Forwarded-For", caller);
  return new Request(request, { headers });
}

/** The port the server listens on in the container (its `PORT`). */
export const SERVER_PORT = 8080;

/** The server's probes (crates/server/src/api/health.rs): the image check reads liveness's `x-deployed-commit`, the daily cron asks for readiness. */
export const HEALTH_PATH = "/healthz";
export const READY_PATH = "/readyz";

/** The server's settings that wrangler.jsonc's `vars` hold, one set per environment. */
export const SERVER_VARS = ["PUBLIC_ORIGINS", "SUPABASE_URL", "TRUSTED_PROXY_HOPS", "LADDER_BOTS", "DEV_STRICT"] as const;

/** The server's secrets, set with `wrangler secret put` (apps/edge/README.md); the first three are required. */
export const SERVER_SECRETS = [
  "SUPABASE_SECRET_KEY",
  "DATABASE_URL",
  "CODE_PEPPER",
  "CREATE_ISSUES_TOKEN",
  "DEV_PROFILE_IDS",
  // #3595: an alert's email (crates/server/src/alerts.rs): a sending-only Resend key and the addresses.
  "ALERT_RESEND_API_KEY",
  "ALERT_EMAILS",
] as const;

export type ServerSetting = (typeof SERVER_VARS)[number] | (typeof SERVER_SECRETS)[number];

/** What the Worker's `env` holds for the container: the settings, and the commit the deploy named (`--var DEPLOYED_COMMIT:<sha>`). */
export type ServerSettings = Partial<Record<ServerSetting | "DEPLOYED_COMMIT", string>>;

/**
 * The container's environment: every setting the Worker holds, the commit the server reports in
 * `x-deployed-commit`, its port, and `NODE_ENV`, which keeps `E2E=1` refused (src/env.rs).
 */
export function containerEnv(settings: ServerSettings): Record<string, string> {
  const vars: Record<string, string> = { NODE_ENV: "production", PORT: String(SERVER_PORT) };
  for (const name of [...SERVER_VARS, ...SERVER_SECRETS, "DEPLOYED_COMMIT"] as const) {
    const value = settings[name];
    if (value !== undefined && value !== "") vars[name] = value;
  }
  return vars;
}
