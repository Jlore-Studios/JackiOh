// `vercel.json` must cover the route table or cold production loads return a 404.
// Root and web copies stay byte-identical; Vercel sources are strict and case-sensitive.

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { describe, expect, it } from "vitest";

import { matchIdOf, paths, seriesIdOf } from "./navigate.ts";

const HERE = dirname(fileURLToPath(import.meta.url));
const WEB_CONFIG = join(HERE, "../../vercel.json");
const ROOT_CONFIG = join(HERE, "../../../../vercel.json");

type VercelConfig = {
  git: { deploymentEnabled: Record<string, boolean> };
  ignoreCommand: string;
  rewrites: { source: string; destination: string }[];
  headers: { source: string; headers: { key: string; value: string }[] }[];
};

const config = JSON.parse(readFileSync(WEB_CONFIG, "utf8")) as VercelConfig;

function sourceRegex(source: string): RegExp {
  const listed = /^\/\(([a-z|-]+)\)\{\/\}\?$/u.exec(source);
  if (listed !== null) return new RegExp(`^/(${listed[1] ?? ""})/?$`, "u");
  const byId = /^\/\(([a-z|-]+)\)\/\(\[\^\/\]\+\)\{\/\}\?$/u.exec(source);
  if (byId !== null) return new RegExp(`^/(${byId[1] ?? ""})/([^/]+)/?$`, "u");
  throw new Error(`deploy-routes.test.ts cannot read this source: ${source}`);
}

const rewrites = config.rewrites.map((rewrite) => ({ ...rewrite, regex: sourceRegex(rewrite.source) }));

function servedByApp(path: string): boolean {
  if (path === "/") return true;
  return rewrites.some(({ regex }) => regex.test(path));
}

describe("vercel.json", () => {
  it("is the same file at the repo root and in apps/web", () => {
    expect(readFileSync(ROOT_CONFIG, "utf8")).toBe(readFileSync(WEB_CONFIG, "utf8"));
  });

  it("sends every rewrite to index.html", () => {
    expect(rewrites.length).toBeGreaterThan(0);
    for (const rewrite of rewrites) expect(rewrite.destination, rewrite.source).toBe("/index.html");
  });

  it("routes every screen in the route table, with or without a trailing slash", () => {
    const fixed = Object.entries(paths)
      // Production serves a genuine 404 at /dev/hotseat.
      .filter(([name]) => name !== "hotseat")
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

  it("routes a match and a series by id, the way matchIdOf and seriesIdOf read them", () => {
    const id = "0b9f3c1e-2d4a-4b8e-9f00-123456789abc";
    for (const path of [paths.match(id), paths.series(id), `${paths.match(id)}/`, `${paths.series(id)}/`]) {
      expect(servedByApp(path), path).toBe(true);
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
      "/Login",
      "/loginx",
      "/almanac/extra",
      "/login/extra",
      "/match",
      "/match/",
      "/match/a/b",
      "/series",
      "/api/auth/me",
    ]) {
      expect(servedByApp(path), path).toBe(false);
    }
  });

  it("sends the security headers on every response, with an enforced CSP", () => {
    const all = config.headers.find((block) => block.source === "/(.*)");
    expect(all).toBeDefined();
    const headers = new Map((all?.headers ?? []).map(({ key, value }) => [key.toLowerCase(), value]));
    expect(headers.get("x-frame-options")).toBe("DENY");
    expect(headers.get("x-content-type-options")).toBe("nosniff");
    expect(headers.get("referrer-policy")).toBe("strict-origin-when-cross-origin");
    expect(headers.get("permissions-policy")).toBe("camera=(), microphone=(), geolocation=()");
    const csp = headers.get("content-security-policy") ?? "";
    // WebAssembly compilation needs `wasm-unsafe-eval` (docs/v0.3.0/SURFACE.md §10.3).
    expect(csp).toMatch(/script-src 'self' 'wasm-unsafe-eval'(;|$)/u);
    expect(csp).toMatch(/frame-ancestors 'none'/u);
    // The bundle connects only to Auth, the API and the match socket.
    const connect = /connect-src ([^;]+)/u.exec(csp)?.[1] ?? "";
    expect(connect.split(" ")).toEqual(
      expect.arrayContaining([
        "'self'",
        "https://exmjdaswedxhnzmpzqrq.supabase.co",
        "https://jackioh-server.onrender.com",
        "wss://jackioh-server.onrender.com",
      ]),
    );
  });
});

// Machine branches disable Vercel deployment; other branches defer to `ignoreCommand`.

function branchRegex(pattern: string): RegExp {
  const family = /^([a-z-]+)\/\*\*$/u.exec(pattern);
  if (family !== null) return new RegExp(`^${family[1] ?? ""}/.+$`, "u");
  if (/^[a-z-]+$/u.test(pattern)) return new RegExp(`^${pattern}$`, "u");
  throw new Error(`deploy-routes.test.ts cannot read this branch pattern: ${pattern}`);
}

function deploys(branch: string): boolean {
  const off = Object.entries(config.git.deploymentEnabled).some(
    ([pattern, enabled]) => !enabled && branchRegex(pattern).test(branch),
  );
  return !off;
}

describe("vercel.json deployment flag", () => {
  it("creates no deployment for the branches machines push to, and never for main", () => {
    for (const branch of [
      "bot-state",
      "bot/issue-63",
      // Squishy's state and issue branches.
      "squishy-state",
      "squishy/issue-63",
      "claude/stoic-tesla-837kse",
      "copilot/fix-1",
      "dependabot/npm_and_yarn/vite-7",
      "patch/v0.1.1",
      "patches/ship-74c8823",
      "polish/3-ai",
      // Cloudflare's branch: Vercel staging builds main.
      "production",
      // Temporary head for a production merge.
      "promote/20261005-abc1234",
      "wt/engine",
    ]) {
      expect(deploys(branch), branch).toBe(false);
    }
    for (const branch of ["main", "feat/live-cards", "fix/anim-double", "machine-ids", "bot", "botany", "productions", "promotes"]) {
      expect(deploys(branch), branch).toBe(true);
    }
  });

  it("hands the build decision to scripts/vercel-ignore.sh, and builds unless that exits 0", () => {
    expect(config.ignoreCommand).toBe(
      'sh "$(git rev-parse --show-toplevel)/scripts/vercel-ignore.sh" && exit 0; exit 1',
    );
  });
});
