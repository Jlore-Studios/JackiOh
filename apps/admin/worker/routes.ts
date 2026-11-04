// What the dashboard's Worker hands on to the site's (index.ts), kept apart from the Workers runtime so
// the page's tests can read it too (wrangler.test.ts holds `run_worker_first` to it).

/** One route the Worker hands on: an exact path or a path prefix, for one method or any (`null`). */
export type Forwarded = { method: string | null; path: string } | { method: null; prefix: string };

/**
 * What the dashboard calls (src/api.ts): who the account is, every `/api/admin/` and `/api/dev/` route,
 * and the two holdings the Dev tools section reads.
 */
export const FORWARDED: readonly Forwarded[] = [
  { method: "GET", path: "/api/auth/me" },
  { method: null, prefix: "/api/admin/" },
  { method: null, prefix: "/api/dev/" },
  { method: "GET", path: "/api/quests" },
  { method: "GET", path: "/api/wallet" },
];

/** Whether the Worker hands `method pathname` to the site's Worker. `pathname` is a parsed URL's, so dot segments are gone. */
export function forwards(method: string, pathname: string): boolean {
  return FORWARDED.some((route) =>
    "prefix" in route
      ? pathname.startsWith(route.prefix)
      : pathname === route.path && (route.method === null || route.method === method),
  );
}
