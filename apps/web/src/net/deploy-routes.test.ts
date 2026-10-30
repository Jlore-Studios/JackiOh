// `public/_redirects` must cover the route table, or a cold load of a screen gets the 404 page.
// Each rule is matched the way Cloudflare's asset worker matches it: a rule with no `:name` or `*` is
// the exact path, `:name` is one path segment and `*` anything, and case counts. A path is the app's
// when it is `/`, a rule matches it, or it is the slashed path of a page the build writes
// (static-pages.ts), which is that page's own file; wrangler.jsonc sends `/api/*` and `/ws/*` to the
// Worker before any rule is read.

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import JSON5 from "json5";
import { describe, expect, it } from "vitest";

import { PUBLIC_PAGES } from "../../static-pages.ts";
import { matchIdOf, paths, seriesIdOf } from "./navigate.ts";
import { coopLinkPath, roomLinkPath } from "./roomLink.ts";

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = join(HERE, "../../../..");

/** `_redirects`' rules, each `[from, to, status]`. */
const rules = readFileSync(join(HERE, "../../public/_redirects"), "utf8")
  .split("\n")
  .map((line) => line.trim())
  .filter((line) => line !== "" && !line.startsWith("#"))
  .map((line) => line.split(/\s+/u));

/** A rule's source as Cloudflare's asset worker matches it: `:name` one path segment, `*` anything, case and all. */
function sourceRegex(from: string): RegExp {
  const parts = from.split("*").map((part) => part.replace(/[\\^$.+?()[\]{}|]/gu, "\\$&").replace(/:[A-Za-z]\w*/gu, "[^/]+"));
  return new RegExp(`^${parts.join(".*")}$`, "u");
}

const matchers = rules.map(([from = ""]) => ({ from, regex: sourceRegex(from) }));

/** The pages the build writes as `<path>/index.html`. */
const EMITTED = new Set(PUBLIC_PAGES.map((page) => page.path));

function servedByApp(path: string): boolean {
  if (path === "/") return true;
  if (matchers.some(({ regex }) => regex.test(path))) return true;
  return path.endsWith("/") && EMITTED.has(path.slice(0, -1));
}

describe("public/_redirects", () => {
  it("matches the way Cloudflare does: a whole path, `:id` as one segment, and case", () => {
    expect(sourceRegex("/match/:id").test("/match/abc")).toBe(true);
    expect(sourceRegex("/match/:id").test("/match/a/b")).toBe(false);
    expect(sourceRegex("/match/:id").test("/match/")).toBe(false);
    expect(sourceRegex("/login").test("/Login")).toBe(false);
    expect(sourceRegex("/login").test("/loginx")).toBe(false);
    expect(sourceRegex("/a/*").test("/a/b/c")).toBe(true);
  });

  it("routes every screen in the route table, with or without a trailing slash", () => {
    const fixed = Object.entries(paths)
      // Production serves a genuine 404 at /dev/hotseat and /dev/duos (and /dev/replays, the replay list's old home).
      .filter(([name]) => name !== "hotseat" && name !== "duosDev")
      .flatMap(([, path]) => (typeof path === "string" ? [path] : []));
    expect(fixed).toContain(paths.privacy);
    // R630: the Card Almanac is public.
    expect(fixed).toContain(paths.almanac);
    // R654: the statistics page is public.
    expect(fixed).toContain(paths.stats);
    for (const path of fixed) {
      expect(servedByApp(path), path).toBe(true);
      // `currentPath` drops trailing slashes.
      if (path !== paths.landing) expect(servedByApp(`${path}/`), `${path}/`).toBe(true);
    }
  });

  it("rewrites nothing but the route table's screens, in either spelling", () => {
    const screens = Object.entries(paths)
      .filter(([name]) => name !== "hotseat" && name !== "duosDev")
      .flatMap(([, path]) => {
        if (typeof path === "string") return [path];
        // A screen by id, in `_redirects`' spelling; the others are a screen with a query.
        const byId = (path as (id: string) => string)(":id");
        return byId.includes("?") ? [] : [byId];
      });
    const spellings = new Set(screens.flatMap((path) => [path, `${path}/`]));
    expect(spellings).toContain("/match/:id");
    for (const { from } of matchers) expect(spellings.has(from), from).toBe(true);
  });

  it("routes a match, a series, a watched game and a co-op game by id, the way matchIdOf and seriesIdOf read them", () => {
    const id = "0b9f3c1e-2d4a-4b8e-9f00-123456789abc";
    for (const path of [paths.match(id), paths.series(id), paths.watch(id), paths.duos(id)]) {
      expect(servedByApp(path), path).toBe(true);
      expect(servedByApp(`${path}/`), `${path}/`).toBe(true);
    }
    expect(matchIdOf(paths.match(id))).toBe(id);
    expect(seriesIdOf(paths.series(id))).toBe(id);
  });

  it("leaves every other path to the 404 page", () => {
    for (const path of [
      "/polish-website-404-check-7f3a9c",
      "/assets/does-not-exist.js",
      "/favicon.ico",
      "/dev/hotseat",
      "/dev/duos",
      "/dev/replays",
      "/Login",
      "/loginx",
      "/almanac/extra",
      "/login/extra",
      "/match",
      "/match/",
      "/match/a/b",
      "/series",
    ]) {
      expect(servedByApp(path), path).toBe(false);
    }
  });

  it("keeps the query string: every rule is a rewrite, which leaves the address as the browser asked for it, and none spells a query", () => {
    for (const [from = "", to = "", status] of rules) {
      expect(status, from).toBe("200");
      // Cloudflare matches the path alone, and a rewrite serves its target's file for the address asked.
      expect(`${from} ${to}`, from).not.toContain("?");
    }
    const id = "core-001";
    for (const link of [
      paths.almanacCard(id),
      paths.collectionSection("cards"),
      roomLinkPath("QRSTUV", "bo1"),
      coopLinkPath("QRSTUV"),
    ]) {
      const url = new URL(link, "https://jackioh.com");
      expect(url.search, link).not.toBe("");
      expect(servedByApp(url.pathname), link).toBe(true);
    }
  });

  it("rewrites no path the Worker answers: wrangler.jsonc sends /api/* and /ws/* to it first", () => {
    const config = JSON5.parse<{ assets?: { run_worker_first?: string[] } }>(readFileSync(join(ROOT, "wrangler.jsonc"), "utf8"));
    const prefixes = (config.assets?.run_worker_first ?? []).map((route) => route.replace(/\*$/u, ""));
    expect(prefixes).toEqual(["/api/", "/ws/"]);
    const worker = prefixes.flatMap((prefix) => [prefix, `${prefix}x`, `${prefix}x/`, `${prefix}a/b`]);
    worker.push("/api/auth/me", "/api/catalog", "/ws/match", "/ws/duos", "/ws/watch");
    for (const { from, regex } of matchers) {
      expect(from, "a rule under the Worker's paths").not.toMatch(/^\/(api|ws)(\/|$)/u);
      for (const path of worker) expect(regex.test(path), `${from} matches ${path}`).toBe(false);
    }
  });
});
