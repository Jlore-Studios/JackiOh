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
  withSecurityHeaders(headers);
  // `manual` is a Workers-runtime-only body hint with no observable accessor, so no unit test here can
  // tell it from any other string. A disable covers only the line after it, so it stands last.
  // Stryker disable next-line StringLiteral: a Workers-only body hint no test can read (above)
  return new Response(stored.body, { status: 200, headers, encodeBody: "manual" });
}

/**
 * #4713, R4691002: the server's one read the Worker answers from Cloudflare's cache: a shipped patch's
 * catalog, `GET /api/catalog/<version>`, the same bytes for everybody whatever the request carries
 * (#4011's rule, held by the server's test), which the server marks `public`. Only with no query string,
 * so the cache's key is the path alone. Never `/api/catalog` itself, whose `x-deployed-commit` the
 * deploys' live check polls (CLAUDE.md rule 13), and no other `/api/*` read (#4011).
 */
export const EDGE_CACHED_READ = /^\/api\/catalog\/[^/]+$/u;

export function isEdgeCachedRead(request: Request): boolean {
  const url = new URL(request.url);
  return request.method === "GET" && url.search === "" && EDGE_CACHED_READ.test(url.pathname);
}

/** The slice of the Workers Cache API (the data centre's default cache) `edgeCachedRead` uses. */
export interface EdgeCache {
  match(request: Request): Promise<Response | undefined>;
  put(request: Request, response: Response): Promise<void>;
}

/**
 * #4713, R4691002: a read `isEdgeCachedRead` names, from `cache` when this data centre holds it, so it
 * never reaches the server; else from `load` (the server), kept through `waitUntil` when the server
 * marked it `public` (a 404, or the live catalog's `no-store`, is never kept). A cache that fails is
 * passed by: the server answers as it would with no cache.
 */
export async function edgeCachedRead(
  request: Request,
  cache: EdgeCache,
  load: () => Promise<Response>,
  waitUntil: (promise: Promise<unknown>) => void,
): Promise<Response> {
  try {
    const kept = await cache.match(request);
    if (kept !== undefined) return kept;
  } catch (error) {
    console.warn("edge.cache_unreadable", error);
  }
  const answer = await load();
  if (answer.status === 200 && /\bpublic\b/u.test(answer.headers.get("Cache-Control") ?? "")) {
    waitUntil(
      cache.put(request, answer.clone()).catch((error: unknown) => {
        console.warn("edge.cache_unwritten", error);
      }),
    );
  }
  return answer;
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

/**
 * #4297: the static security headers the Worker's own answers carry, copied character-for-character
 * from the `/*` block of `apps/web/public/_headers`. The Worker sets them because those answers are
 * never files Cloudflare serves: `precompressedModule` builds the module's answer and `index.ts`
 * builds its own 404. `Content-Security-Policy` is excluded: its `connect-src` is a per-build
 * `%CONNECT_SRC%` placeholder `static-pages.ts` fills, which the edge cannot reproduce, and a
 * hardcoded CSP would drift or break hosts.
 */
export const SECURITY_HEADERS: Readonly<Record<string, string>> = Object.freeze({
  "X-Content-Type-Options": "nosniff",
  "X-Frame-Options": "DENY",
  "Referrer-Policy": "strict-origin-when-cross-origin",
  "Strict-Transport-Security": "max-age=31536000; includeSubDomains",
});

/** #4297: sets `SECURITY_HEADERS` on a Worker-synthesized answer's headers. */
export function withSecurityHeaders(headers: Headers): void {
  for (const [name, value] of Object.entries(SECURITY_HEADERS)) headers.set(name, value);
}

/** The server's probes (crates/server/src/api/health.rs): the image check reads liveness's `x-deployed-commit`, the daily cron asks for readiness. */
export const HEALTH_PATH = "/healthz";
export const READY_PATH = "/readyz";

/**
 * #3727: the daily games' sweep (crates/server/src/daily/notify.rs): ends the games whose move ran out
 * and sends the "your move" emails and pushes while nobody else wakes the server. Hourly, beside the
 * daily wake-up's cron, and posted to a path outside `/api` and `/ws`, which `isServerPath` never
 * forwards from the public.
 */
export const DAILY_SWEEP_CRON = "7 * * * *";
export const DAILY_SWEEP_PATH = "/daily-sweep";

/**
 * R4691000: what the Worker asks the server as it goes to sleep: when a daily sweep next has something
 * due (crates/server's `daily::notify::next_due`), as `{ "nextDueAt": <epoch ms> | null }`. Outside `/api`
 * and `/ws`, as the sweep's path is.
 */
export const DAILY_NEXT_PATH = "/daily-next";

/**
 * R4691000: `/daily-next`'s time out of its answer: a number, null (nothing due until a request changes
 * what is stored), or undefined for an answer that says neither, which is kept as no answer.
 */
export function nextDueOf(body: unknown): number | null | undefined {
  if (typeof body !== "object" || body === null || !("nextDueAt" in body)) return undefined;
  const at = body.nextDueAt;
  if (at === null) return null;
  return typeof at === "number" && Number.isFinite(at) ? at : undefined;
}

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
  // #3727: the daily games' "your move" email (a Resend key of its own) and Web Push (the VAPID pair,
  // whose public half `GET /api/daily/settings` gives the browser). Each channel is off while unset.
  "PLAYER_RESEND_API_KEY",
  "VAPID_PRIVATE_KEY",
  "VAPID_PUBLIC_KEY",
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

/**
 * #4118: the lowercase hex SHA-256 of `containerEnv(settings)` as JSON with its names in order, so the same
 * settings in any key order give the same digest, and a change to any one of them, the commit among them,
 * changes it. The Worker keeps it in the object's storage to tell a running server that started on other
 * settings from one that started on these (index.ts). It is a digest of secrets: never log or send it.
 */
export async function settingsFingerprint(settings: ServerSettings): Promise<string> {
  const env = containerEnv(settings);
  const entries = Object.keys(env)
    .sort()
    .map((name) => [name, env[name]]);
  const digest = await crypto.subtle.digest("SHA-256", new TextEncoder().encode(JSON.stringify(entries)));
  return Array.from(new Uint8Array(digest), (byte) => byte.toString(16).padStart(2, "0")).join("");
}
