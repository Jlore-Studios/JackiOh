// #3619: a crash in this browser, told to the owners. An error `RouteErrorBoundary` caught, an uncaught
// `error` and an `unhandledrejection` each send one small report to `POST /api/client-errors`
// (`crates/server/src/api/client_errors.rs`), which raises the `client.error` alert.
//
// CLAUDE.md rule 7: the report is built from a whitelist of five fields: the trigger, the message, a
// trimmed stack, the route as a pattern (`routeOf`: no query, no hash, no id) and the catalog version.
// It carries no hand, board or state, and no address or email; the bearer token rides in the header
// only, for the server's per-account limit.
//
// Left out as noise: a lazy chunk that failed to load (the boundary's reload panel covers it), anything
// that is not an `Error`, and an error whose stack holds no file of ours (a browser extension's, or an
// opaque cross-origin one). A page sends at most `CLIENT_ERROR_REPORTS_PER_PAGE`, a repeat once.
// A report that fails is dropped without a word: it never reports itself.

import { CLIENT_ERROR_MESSAGE_MAX_CHARS, CLIENT_ERROR_STACK_MAX_CHARS } from "@jackioh/server-config";

import { catalogVersion, wasmLoaded } from "../wasm/index.ts";
import { apiRequest } from "./api.ts";
import { messageOf } from "./errors.ts";
import { currentPath, paths } from "./navigate.ts";
import { readSession } from "./session.ts";

/** How many reports one page load sends, so a render loop cannot flood the server. */
export const CLIENT_ERROR_REPORTS_PER_PAGE = 3;
/** How many lines of the stack go in a report, before the server's character cap. */
export const CLIENT_ERROR_STACK_LINES = 10;
export const CLIENT_ERROR_PATH = "/api/client-errors";

/** The routes whose second segment is an id or a room code, reported as `/<first>/:id`. */
const ID_ROUTES = ["match", "series", "watch", "duos", "lobby", "daily"];

/** What a browser says when a lazy chunk or its stylesheet will not load (Chrome, Firefox, Safari, Vite). */
const CHUNK_LOAD_FAILURE =
  /dynamically imported module|Importing a module script failed|ChunkLoadError|Loading (CSS )?chunk .* failed|Unable to preload CSS/i;

export type ClientErrorTrigger = "boundary" | "error" | "unhandledrejection";

let sent = 0;
const seen = new Set<string>();
let installed = false;

export function isChunkLoadFailure(cause: unknown): boolean {
  return CHUNK_LOAD_FAILURE.test(messageOf(cause));
}

/** Whether the stack names a file served from this origin: Chrome's `at f (https://…)` and Safari's `f@https://…`. */
export function hasOurFrame(stack: string): boolean {
  return stack.includes(`${window.location.origin}/`);
}

const staticRoutes: ReadonlySet<string> = new Set(
  Object.values(paths).flatMap((value) => (typeof value === "string" ? [value] : [])),
);

/** The screen's route as a pattern, so an id or a room code never leaves the browser. */
export function routeOf(path: string): string {
  if (staticRoutes.has(path)) return path;
  const parts = path.split("/").filter((part) => part.length > 0);
  const first = parts[0];
  if (parts.length === 2 && first !== undefined && ID_ROUTES.includes(first)) return `/${first}/:id`;
  return "unknown";
}

function cut(text: string, max: number): string {
  return Array.from(text).slice(0, max).join("");
}

function loadedCatalogVersion(): string | null {
  try {
    return wasmLoaded() ? catalogVersion() : null;
  } catch {
    return null;
  }
}

/** Sends one report for `cause`, or nothing: see the header. It never throws. */
export function reportClientError(trigger: ClientErrorTrigger, cause: unknown): void {
  try {
    if (!(cause instanceof Error) || isChunkLoadFailure(cause)) return;
    const stack = cause.stack ?? "";
    if (stack.length === 0 || !hasOurFrame(stack)) return;
    if (sent >= CLIENT_ERROR_REPORTS_PER_PAGE) return;
    const frame = stack.split("\n").find((line) => line.includes(`${window.location.origin}/`)) ?? "";
    const signature = `${cause.message}\n${frame}`;
    if (seen.has(signature)) return;
    seen.add(signature);
    sent += 1;

    const version = loadedCatalogVersion();
    const body = {
      trigger,
      message: cut(cause.message, CLIENT_ERROR_MESSAGE_MAX_CHARS),
      stack: cut(stack.split("\n").slice(0, CLIENT_ERROR_STACK_LINES).join("\n"), CLIENT_ERROR_STACK_MAX_CHARS),
      route: routeOf(currentPath()),
      ...(version === null ? {} : { catalogVersion: version }),
    };
    apiRequest(CLIENT_ERROR_PATH, {
      method: "POST",
      token: readSession()?.accessToken ?? null,
      body,
      keepalive: true,
    }).catch(() => undefined);
  } catch {
    // A report that cannot be made is not reported.
  }
}

/** Listens for uncaught errors and unhandled rejections, once. */
export function installClientErrorReports(): void {
  if (installed) return;
  installed = true;
  window.addEventListener("error", (event) => {
    reportClientError("error", event.error);
  });
  window.addEventListener("unhandledrejection", (event) => {
    reportClientError("unhandledrejection", event.reason);
  });
}

export function resetClientErrorReportsForTests(): void {
  sent = 0;
  seen.clear();
  installed = false;
}
