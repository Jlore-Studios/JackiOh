// wrangler.jsonc and public/_headers: what Cloudflare serves the site with, in both environments.
// Each environment serves apps/web/dist as static assets and sends only the server's paths, `/api/*`
// and `/ws/*`, to apps/edge's Worker (`assets.run_worker_first`); every file it serves carries
// `_headers`' security headers, whose CSP names the build's own hosts (static-pages.ts
// `connectSources`). cloudflare-serve.test.ts holds how a path that is no file is answered, and
// deploy-routes.test.ts holds `_redirects` to the route table.

import { readFileSync, readdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import JSON5 from "json5";
import { describe, expect, it } from "vitest";

import { CONNECT_SRC_PLACEHOLDER, connectSources } from "../../static-pages.ts";

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = join(HERE, "../../../..");
const PUBLIC = join(HERE, "../../public");

type Assets = { directory?: string; not_found_handling?: string; run_worker_first?: unknown };
type Environment = { name?: string; workers_dev?: boolean; preview_urls?: boolean; assets?: Assets };
type Wrangler = Environment & { main?: string; env?: Record<string, Environment> };

const wrangler = JSON5.parse<Wrangler>(readFileSync(join(ROOT, "wrangler.jsonc"), "utf8"));

/** Production (the top level) and staging (`env.staging`), each as wrangler deploys it. */
const ENVIRONMENTS: [string, Environment][] = [
  ["production", wrangler],
  ["staging", wrangler.env?.["staging"] ?? {}],
];

/** The paths the Worker answers: crates/server's routes (apps/edge/src/routing.ts `SERVER_ROUTES`). */
const SERVER_ROUTES = ["/api/*", "/ws/*"];

describe("wrangler.jsonc", () => {
  it("has exactly the two environments, production at the top level and staging", () => {
    expect(Object.keys(wrangler.env ?? {})).toEqual(["staging"]);
    expect(wrangler.name).toBe("jackioh");
    expect(wrangler.env?.["staging"]?.name).toBe("jackioh-staging");
  });

  it("serves the web build as static assets in both environments", () => {
    for (const [name, environment] of ENVIRONMENTS) {
      expect(environment.assets?.directory, name).toBe("./apps/web/dist");
    }
  });

  it("answers an unrouted path with public/404.html and a 404 in both environments", () => {
    for (const [name, environment] of ENVIRONMENTS) expect(environment.assets?.not_found_handling, name).toBe("404-page");
  });

  it("runs the Worker first for exactly /api/* and /ws/* in both environments: every other path is a file, a rewrite or the 404 page", () => {
    expect(wrangler.main).toBe("./apps/edge/src/index.ts");
    for (const [name, environment] of ENVIRONMENTS) expect(environment.assets?.run_worker_first, name).toEqual(SERVER_ROUTES);
  });

  it("serves staging only at staging.jackioh.com, behind Access: no workers.dev or preview address around it", () => {
    const staging = wrangler.env?.["staging"];
    expect(staging?.workers_dev).toBe(false);
    expect(staging?.preview_urls).toBe(false);
  });
});

type Header = { key: string; value: string };
type HeaderBlock = { path: string; headers: Header[] };

const lines = (text: string): string[] =>
  text
    .split("\n")
    .map((line) => line.replace(/\s+$/u, ""))
    .filter((line) => line.trim() !== "" && !line.trim().startsWith("#"));

function blocks(): HeaderBlock[] {
  const out: HeaderBlock[] = [];
  for (const line of lines(readFileSync(join(PUBLIC, "_headers"), "utf8"))) {
    if (!/^\s/u.test(line)) {
      out.push({ path: line.trim(), headers: [] });
      continue;
    }
    const at = line.indexOf(":");
    out.at(-1)?.headers.push({ key: line.slice(0, at).trim(), value: line.slice(at + 1).trim() });
  }
  return out;
}

const IMMUTABLE: Header[] = [{ key: "Cache-Control", value: "public, max-age=31536000, immutable" }];

describe("public/_headers", () => {
  it("has three blocks: every path, the fonts and the bundle", () => {
    expect(blocks().map((block) => block.path)).toEqual(["/*", "/fonts/*", "/assets/*"]);
  });

  it("sends the security headers on every path, with an enforced CSP", () => {
    const all = blocks().find((block) => block.path === "/*");
    const headers = new Map((all?.headers ?? []).map(({ key, value }) => [key.toLowerCase(), value]));
    expect([...headers.keys()]).toEqual([
      "content-security-policy",
      "x-frame-options",
      "x-content-type-options",
      "referrer-policy",
      "permissions-policy",
    ]);
    expect(headers.get("x-frame-options")).toBe("DENY");
    expect(headers.get("x-content-type-options")).toBe("nosniff");
    expect(headers.get("referrer-policy")).toBe("strict-origin-when-cross-origin");
    expect(headers.get("permissions-policy")).toBe("camera=(), microphone=(), geolocation=()");
    const csp = headers.get("content-security-policy") ?? "";
    // WebAssembly compilation needs `wasm-unsafe-eval` (docs/v0.3.0/SURFACE.md §10.3).
    expect(csp).toMatch(/script-src 'self' 'wasm-unsafe-eval'(;|$)/u);
    expect(csp).toMatch(/frame-ancestors 'none'/u);
    expect(csp).toMatch(/^default-src 'self';/u);
  });

  it("leaves the CSP's connect-src to the build, which names its own hosts in it", () => {
    const csp = blocks()[0]?.headers.find((header) => header.key === "Content-Security-Policy")?.value ?? "";
    expect(/connect-src ([^;]+)/u.exec(csp)?.[1]).toBe(CONNECT_SRC_PLACEHOLDER);
    // The bundle connects only to its own origin (the API), Supabase (sign-in) and its own host's sockets.
    expect(connectSources("https://jackioh.com", "https://exmjdaswedxhnzmpzqrq.supabase.co")).toBe(
      "'self' https://exmjdaswedxhnzmpzqrq.supabase.co wss://jackioh.com",
    );
    expect(connectSources("https://staging.jackioh.com", "https://exmjdaswedxhnzmpzqrq.supabase.co/")).toBe(
      "'self' https://exmjdaswedxhnzmpzqrq.supabase.co wss://staging.jackioh.com",
    );
    // A build without Supabase (`build:e2e`) never signs in, so its CSP names no Supabase host.
    expect(connectSources("https://staging.jackioh.com", undefined)).toBe("'self' wss://staging.jackioh.com");
    expect(connectSources("https://staging.jackioh.com", "")).toBe("'self' wss://staging.jackioh.com");
  });

  it("sends no X-Robots-Tag: staging is behind Access, so no crawler reaches it, and production is meant to be found", () => {
    const keys = blocks().flatMap((block) => block.headers.map((header) => header.key.toLowerCase()));
    expect(keys).not.toContain("x-robots-tag");
  });

  it("lets a browser keep the web fonts a year (#262): their names carry their versions", () => {
    expect(blocks().find((block) => block.path === "/fonts/*")?.headers).toEqual(IMMUTABLE);
    const files = readdirSync(join(PUBLIC, "fonts")).filter((file) => file.endsWith(".woff2"));
    expect(files.length).toBeGreaterThan(0);
    for (const file of files) expect(file, "a versioned name").toMatch(/-\d+\.\d+\.\d+-/u);
  });

  it("lets a browser keep the bundle and the engine's module a year (#778): Vite names every file in /assets/ by its hash", () => {
    expect(blocks().find((block) => block.path === "/assets/*")?.headers).toEqual(IMMUTABLE);
    // Only Vite writes /assets/: nothing in public/ is served there under a name without a hash.
    expect(readdirSync(PUBLIC)).not.toContain("assets");
  });
});
