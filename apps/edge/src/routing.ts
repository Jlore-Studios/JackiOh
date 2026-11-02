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
export const SERVER_VARS = ["PUBLIC_ORIGINS", "SUPABASE_URL", "TRUSTED_PROXY_HOPS", "LADDER_BOTS"] as const;

/** The server's secrets, set with `wrangler secret put` (apps/edge/README.md); the first three are required. */
export const SERVER_SECRETS = [
  "SUPABASE_SECRET_KEY",
  "DATABASE_URL",
  "CODE_PEPPER",
  "CREATE_ISSUES_TOKEN",
  "DEV_PROFILE_IDS",
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
