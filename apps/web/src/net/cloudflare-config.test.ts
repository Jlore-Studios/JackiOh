// Cloudflare production must match Vercel staging: routes, 404s and security headers.
// `vercel.json` is source of truth; this holds redirects, headers and Wrangler configuration to it,
// but for the one header only staging sends (`X-Robots-Tag: noindex`, #770).
// Cloudflare rewrites target `/`, list both slash spellings and use `404-page`.
// Generated static pages need a folder rule; Vercel serves files before rewrites.

import { readFileSync, readdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { describe, expect, it } from "vitest";

import { PUBLIC_PAGES } from "../../static-pages.ts";
import { paths } from "./navigate.ts";

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = join(HERE, "../../../..");
const PUBLIC = join(HERE, "../../public");

type VercelConfig = {
  rewrites: { source: string; destination: string }[];
  headers: { source: string; headers: { key: string; value: string }[] }[];
};

type Header = { key: string; value: string };
type HeaderBlock = { path: string; headers: Header[] };

/** The one header staging sends and production must not: search engines leave the test copy out (#770). */
const STAGING_ONLY_HEADER: Header = { key: "X-Robots-Tag", value: "noindex" };

const isStagingOnly = (header: Header): boolean =>
  header.key === STAGING_ONLY_HEADER.key && header.value === STAGING_ONLY_HEADER.value;

/**
 * The header blocks `_headers` must hold for these vercel.json blocks: each path in Cloudflare's
 * spelling (`(.*)` as `*`), and the every-path block without STAGING_ONLY_HEADER. Every other
 * header, value and path is copied as it is, so any other difference fails the comparison.
 */
function productionHeaders(staging: VercelConfig["headers"]): HeaderBlock[] {
  return staging.map((block) => ({
    path: block.source.replace("(.*)", "*"),
    headers: block.source === "/(.*)" ? block.headers.filter((header) => !isStagingOnly(header)) : block.headers,
  }));
}

const vercel = JSON.parse(readFileSync(join(ROOT, "vercel.json"), "utf8")) as VercelConfig;

const EMITTED = new Set(PUBLIC_PAGES.map((page) => page.path));

function expectedRules(): string[] {
  const rules: string[] = [];
  for (const { source } of vercel.rewrites) {
    const listed = /^\/\(([a-z|-]+)\)\{\/\}\?$/u.exec(source);
    const byId = /^\/\(([a-z|-]+)\)\/\(\[\^\/\]\+\)\{\/\}\?$/u.exec(source);
    const names = (listed ?? byId)?.[1]?.split("|");
    if (names === undefined) throw new Error(`cloudflare-config.test.ts cannot read this source: ${source}`);
    for (const name of names) {
      const path = byId === null ? `/${name}` : `/${name}/:id`;
      if (EMITTED.has(path)) rules.push(`${path} ${path}/ 200`);
      else rules.push(`${path} / 200`, `${path}/ / 200`);
    }
  }
  return rules;
}

const lines = (text: string): string[] =>
  text
    .split("\n")
    .map((line) => line.replace(/\s+$/u, ""))
    .filter((line) => line.trim() !== "" && !line.trim().startsWith("#"));

describe("public/_redirects", () => {
  const rules = lines(readFileSync(join(PUBLIC, "_redirects"), "utf8")).map((line) => line.split(/\s+/u));

  it("rewrites exactly the paths vercel.json rewrites, each page the build writes to its own folder", () => {
    expect(rules.map((rule) => rule.join(" ")).sort()).toEqual(expectedRules().sort());
  });

  it("rewrites (200), never to an `index.html`, which Cloudflare answers with a redirect", () => {
    for (const [from, to, status] of rules) {
      expect(status, from).toBe("200");
      expect(to, from).not.toMatch(/index\.html$/u);
    }
  });

  function served(path: string): boolean {
    if (path === "/" || rules.some(([from]) => from === path)) return true;
    return path.endsWith("/") && EMITTED.has(path.slice(0, -1));
  }

  it("serves every screen in the route table, with or without a trailing slash: its emitted page or a rewrite", () => {
    const fixed = Object.entries(paths)
      // Production serves a genuine 404 at /dev/hotseat (and /dev/replays, the replay list's old home).
      .filter(([name]) => name !== "hotseat")
      .flatMap(([, path]) => (typeof path === "string" ? [path] : []));
    for (const path of fixed) {
      expect(served(path), path).toBe(true);
      if (path !== paths.landing) expect(served(`${path}/`), `${path}/`).toBe(true);
    }
    for (const path of ["/loginx", "/almanac/extra", "/dev/hotseat", "/dev/replays"]) expect(served(path), path).toBe(false);
  });
});

describe("public/_headers", () => {
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

  it("sends vercel.json's headers on every path, value for value, but for staging's noindex", () => {
    const [first] = blocks();
    expect(first?.path).toBe("/*");
    const all = vercel.headers.find((block) => block.source === "/(.*)");
    expect(first?.headers).toEqual(all?.headers.filter((header) => !isStagingOnly(header)));
  });

  it("has exactly vercel.json's header blocks, each path in Cloudflare's spelling (`(.*)` as `*`)", () => {
    expect(blocks()).toEqual(productionHeaders(vercel.headers));
  });

  it("keeps staging out of search results (#770): vercel.json sends X-Robots-Tag: noindex on every path, _headers never", () => {
    const all = vercel.headers.find((block) => block.source === "/(.*)");
    expect(all?.headers.filter(isStagingOnly)).toHaveLength(1);
    const keys = blocks().flatMap((block) => block.headers.map((header) => header.key.toLowerCase()));
    expect(keys).not.toContain("x-robots-tag");
  });

  it("allows staging exactly one extra header, X-Robots-Tag: noindex on every path (#770)", () => {
    const shared = { key: "X-Frame-Options", value: "DENY" };
    const cache = { key: "Cache-Control", value: "public, max-age=31536000, immutable" };
    expect(
      productionHeaders([
        { source: "/(.*)", headers: [shared, STAGING_ONLY_HEADER] },
        { source: "/fonts/(.*)", headers: [cache] },
      ]),
    ).toEqual([
      { path: "/*", headers: [shared] },
      { path: "/fonts/*", headers: [cache] },
    ]);
  });

  it("fails on any other difference between the two header sets (#770)", () => {
    const shared = { key: "X-Frame-Options", value: "DENY" };
    const cache = { key: "Cache-Control", value: "public, max-age=31536000, immutable" };
    const fonts = { source: "/fonts/(.*)", headers: [cache] };
    const production: HeaderBlock[] = [
      { path: "/*", headers: [shared] },
      { path: "/fonts/*", headers: [cache] },
    ];
    const differing: VercelConfig["headers"][] = [
      // Another extra header on every path.
      [{ source: "/(.*)", headers: [shared, STAGING_ONLY_HEADER, { key: "X-Other", value: "1" }] }, fonts],
      // The header with another value, or spelled another way.
      [{ source: "/(.*)", headers: [shared, { key: "X-Robots-Tag", value: "none" }] }, fonts],
      [{ source: "/(.*)", headers: [shared, { key: "x-robots-tag", value: "noindex" }] }, fonts],
      // The header on another path.
      [{ source: "/(.*)", headers: [shared] }, { source: "/fonts/(.*)", headers: [cache, STAGING_ONLY_HEADER] }],
      // A shared header missing or changed, or a block missing.
      [{ source: "/(.*)", headers: [STAGING_ONLY_HEADER] }, fonts],
      [{ source: "/(.*)", headers: [{ key: "X-Frame-Options", value: "SAMEORIGIN" }, STAGING_ONLY_HEADER] }, fonts],
      [{ source: "/(.*)", headers: [shared, STAGING_ONLY_HEADER] }],
    ];
    for (const staging of differing) {
      expect(productionHeaders(staging), JSON.stringify(staging)).not.toEqual(production);
    }
    // Production sending the header is a difference too.
    expect(productionHeaders([{ source: "/(.*)", headers: [shared, STAGING_ONLY_HEADER] }, fonts])).not.toEqual([
      { path: "/*", headers: [shared, STAGING_ONLY_HEADER] },
      { path: "/fonts/*", headers: [cache] },
    ]);
  });

  it("lets a browser keep the web fonts a year (#262): their names carry their versions", () => {
    const fonts = blocks().find((block) => block.path === "/fonts/*");
    expect(fonts?.headers).toEqual([{ key: "Cache-Control", value: "public, max-age=31536000, immutable" }]);
    const files = readdirSync(join(PUBLIC, "fonts")).filter((file) => file.endsWith(".woff2"));
    expect(files.length).toBeGreaterThan(0);
    for (const file of files) expect(file, "a versioned name").toMatch(/-\d+\.\d+\.\d+-/u);
  });

  it("lets a browser keep the bundle and the engine's module a year (#778): Vite names every file in /assets/ by its hash", () => {
    const cache = [{ key: "Cache-Control", value: "public, max-age=31536000, immutable" }];
    expect(blocks().find((block) => block.path === "/assets/*")?.headers).toEqual(cache);
    expect(vercel.headers.find((block) => block.source === "/assets/(.*)")?.headers).toEqual(cache);
    // Only Vite writes /assets/: nothing in public/ is served there under a name without a hash.
    expect(readdirSync(PUBLIC)).not.toContain("assets");
  });
});

describe("wrangler.jsonc", () => {
  const text = readFileSync(join(ROOT, "wrangler.jsonc"), "utf8")
    .split("\n")
    .filter((line) => !line.trim().startsWith("//"))
    .join("\n");
  const config = JSON.parse(text) as {
    main?: string;
    assets?: { directory?: string; not_found_handling?: string; run_worker_first?: unknown };
  };

  it("serves the web build as static assets, with no Worker script and so no secrets", () => {
    expect(config.main).toBeUndefined();
    expect(config.assets?.directory).toBe("./apps/web/dist");
    expect(config.assets?.run_worker_first).toBeUndefined();
  });

  it("answers an unrouted path with public/404.html and a 404, as Vercel does", () => {
    expect(config.assets?.not_found_handling).toBe("404-page");
  });
});
