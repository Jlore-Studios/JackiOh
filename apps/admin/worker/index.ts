// The dev dashboard's Worker (#3074, apps/admin/README.md). Static files never reach this code:
// wrangler.jsonc's `run_worker_first` sends only `/api/*` here. It hands the routes the dashboard calls
// (routes.ts) to the site's own Worker through the `SITE` service binding (`jackioh`, or
// `jackioh-staging`), which gives them to the game server exactly as it does the site's own requests,
// and answers every other `/api/*` path 404 itself, so admin.jackioh.com reaches nothing of the server
// but those.
//
// It decides nothing else: Cloudflare Access guards the hostname before this runs, and the server
// checks the bearer token, `DEV_PROFILE_IDS` and (when strict) the session's aal2 on every route.

import { forwards } from "./routes.ts";

export { forwards } from "./routes.ts";

export interface Env {
  /** The site's Worker in this environment (wrangler.jsonc `services`). */
  SITE: Fetcher;
}

/**
 * #4297: the static security headers the Worker's own 404 carries, kept equal to the site Worker's
 * `SECURITY_HEADERS` (apps/edge/src/routing.ts). The test pins both to `apps/web/public/_headers`,
 * so a drift fails loudly. The dashboard cannot import the edge package: separate deploy units.
 */
export const SECURITY_HEADERS: Readonly<Record<string, string>> = Object.freeze({
  "X-Content-Type-Options": "nosniff",
  "X-Frame-Options": "DENY",
  "Referrer-Policy": "strict-origin-when-cross-origin",
  "Strict-Transport-Security": "max-age=31536000; includeSubDomains",
});

/** The server's one error shape (crates/server/src/api/http.rs), for a path this Worker does not hand on. */
export function notFound(): Response {
  return Response.json(
    { error: { code: "not_found", message: "The dev dashboard has no such route." } },
    { status: 404, headers: { ...SECURITY_HEADERS } },
  );
}

export default {
  async fetch(request, env): Promise<Response> {
    const { pathname } = new URL(request.url);
    if (!forwards(request.method, pathname)) return notFound();
    return env.SITE.fetch(request);
  },
} satisfies ExportedHandler<Env>;
