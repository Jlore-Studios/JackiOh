// wrangler.jsonc and public/_headers: what Cloudflare serves the site with, in both environments.
// Each environment serves apps/web/dist as static assets and sends only the server's paths, `/api/*`
// and `/ws/*`, and the engine's brotli-compressed module (#3183) to apps/edge's Worker
// (`assets.run_worker_first`); every file it serves carries
// `_headers`' security headers, whose CSP names the build's own hosts (static-pages.ts
// `connectSources`). cloudflare-serve.test.ts holds how a path that is no file is answered, and
// deploy-routes.test.ts holds `_redirects` to the route table.

import { readFileSync, readdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import JSON5 from "json5";
import { describe, expect, it } from "vitest";

import { GIS_SCRIPT_URL } from "../auth/googleIdentity.ts";
import { CONNECT_SRC_PLACEHOLDER, GOOGLE_IDENTITY_SOURCE, connectSources } from "../../static-pages.ts";

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

/**
 * The paths the Worker answers (apps/edge/src/routing.ts `WORKER_ROUTES`): crates/server's routes, and
 * #3183's module, which the Worker serves with its `Content-Encoding` (Vite names it `jackioh_wasm_bg.wasm-<hash>.br`).
 */
const WORKER_ROUTES = ["/api/*", "/ws/*", "/assets/*.wasm-*.br"];

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

  it("answers an unrouted path with the build's 404.html and a 404 in both environments", () => {
    for (const [name, environment] of ENVIRONMENTS) expect(environment.assets?.not_found_handling, name).toBe("404-page");
  });

  it("runs the Worker first for exactly /api/*, /ws/* and the compressed module in both environments: every other path is a file, a rewrite or the 404 page", () => {
    expect(wrangler.main).toBe("./apps/edge/src/index.ts");
    for (const [name, environment] of ENVIRONMENTS) expect(environment.assets?.run_worker_first, name).toEqual(WORKER_ROUTES);
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

/**
 * The CSP, exactly. WebAssembly compilation needs `wasm-unsafe-eval` (docs/v0.3.0/SURFACE.md §10.3);
 * Google's own sign-in button (R2983000) needs its script and, under its path, its styles, frames and
 * requests (connect-src's, filled in by the build); `frame-src` keeps the 'self' it had from default-src.
 */
const CSP =
  "default-src 'self'; script-src 'self' 'wasm-unsafe-eval' https://accounts.google.com/gsi/client; worker-src 'self'; " +
  "style-src 'self' 'unsafe-inline' https://accounts.google.com/gsi/; img-src 'self' data: blob:; media-src 'self' blob:; " +
  "font-src 'self'; frame-src 'self' https://accounts.google.com/gsi/; connect-src %CONNECT_SRC%; object-src 'none'; " +
  "base-uri 'self'; form-action 'self'; frame-ancestors 'none'";

function csp(): string {
  return blocks()[0]?.headers.find((header) => header.key === "Content-Security-Policy")?.value ?? "";
}

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
      "strict-transport-security",
    ]);
    expect(headers.get("x-frame-options")).toBe("DENY");
    expect(headers.get("x-content-type-options")).toBe("nosniff");
    expect(headers.get("referrer-policy")).toBe("strict-origin-when-cross-origin");
    expect(headers.get("permissions-policy")).toBe("camera=(), microphone=(), geolocation=()");
    // #3485: HSTS pairs with Always Use HTTPS (infra/cloudflare.tf); no `preload`, never submitted.
    expect(headers.get("strict-transport-security")).toBe("max-age=31536000; includeSubDomains");
    expect(headers.get("content-security-policy")).toBe(CSP);
  });

  it("leaves the CSP's connect-src to the build, which names its own hosts in it", () => {
    expect(/connect-src ([^;]+)/u.exec(csp())?.[1]).toBe(CONNECT_SRC_PLACEHOLDER);
    // The bundle connects only to its own origin (the API), Supabase and Google Identity Services
    // (sign-in, R2983000) and its own host's sockets.
    expect(connectSources("https://jackioh.com", "https://exmjdaswedxhnzmpzqrq.supabase.co")).toBe(
      "'self' https://exmjdaswedxhnzmpzqrq.supabase.co https://accounts.google.com/gsi/ wss://jackioh.com",
    );
    expect(connectSources("https://staging.jackioh.com", "https://exmjdaswedxhnzmpzqrq.supabase.co/")).toBe(
      "'self' https://exmjdaswedxhnzmpzqrq.supabase.co https://accounts.google.com/gsi/ wss://staging.jackioh.com",
    );
    // A build without Supabase (`build:e2e`) never signs in, so its CSP names neither sign-in host.
    expect(connectSources("https://staging.jackioh.com", undefined)).toBe("'self' wss://staging.jackioh.com");
    expect(connectSources("https://staging.jackioh.com", "")).toBe("'self' wss://staging.jackioh.com");
  });

  it("R2983000 widens the CSP by Google Identity Services' sources alone", () => {
    const sources = csp()
      .split(";")
      .flatMap((directive) => directive.trim().split(/\s+/u).slice(1))
      .filter((source) => source.startsWith("https:"));
    expect(sources).toEqual([GIS_SCRIPT_URL, GOOGLE_IDENTITY_SOURCE, GOOGLE_IDENTITY_SOURCE]);
    const directive = (name: string): string[] =>
      csp()
        .split("; ")
        .find((part) => part.startsWith(`${name} `))
        ?.split(" ")
        .slice(1) ?? [];
    expect(directive("script-src")).toEqual(["'self'", "'wasm-unsafe-eval'", GIS_SCRIPT_URL]);
    expect(directive("style-src")).toEqual(["'self'", "'unsafe-inline'", GOOGLE_IDENTITY_SOURCE]);
    expect(directive("frame-src")).toEqual(["'self'", GOOGLE_IDENTITY_SOURCE]);
    // Google's requests are let in by the build, beside the Supabase host it signs in through.
    const connect = connectSources("https://jackioh.com", "https://fuzpvmjdejvbjcarypyi.supabase.co").split(" ");
    expect(connect.filter((source) => source.startsWith("https:"))).toEqual([
      "https://fuzpvmjdejvbjcarypyi.supabase.co",
      GOOGLE_IDENTITY_SOURCE,
    ]);
    expect(GIS_SCRIPT_URL).toBe("https://accounts.google.com/gsi/client");
    expect(GOOGLE_IDENTITY_SOURCE).toBe("https://accounts.google.com/gsi/");
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
