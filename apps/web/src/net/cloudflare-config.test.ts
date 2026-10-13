// Cloudflare production must match Vercel staging: routes, 404s and security headers.
// `vercel.json` is source of truth; this holds redirects, headers and Wrangler configuration to it.
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
      // Production serves a genuine 404 at /dev/hotseat.
      .filter(([name]) => name !== "hotseat")
      .flatMap(([, path]) => (typeof path === "string" ? [path] : []));
    for (const path of fixed) {
      expect(served(path), path).toBe(true);
      if (path !== paths.landing) expect(served(`${path}/`), `${path}/`).toBe(true);
    }
    for (const path of ["/loginx", "/almanac/extra", "/dev/hotseat"]) expect(served(path), path).toBe(false);
  });
});

describe("public/_headers", () => {
  function blocks(): { path: string; headers: { key: string; value: string }[] }[] {
    const out: { path: string; headers: { key: string; value: string }[] }[] = [];
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

  it("sends vercel.json's headers on every path, value for value", () => {
    const [first] = blocks();
    expect(first?.path).toBe("/*");
    const all = vercel.headers.find((block) => block.source === "/(.*)");
    expect(first?.headers).toEqual(all?.headers);
  });

  it("has exactly vercel.json's header blocks, each path in Cloudflare's spelling (`(.*)` as `*`)", () => {
    const expected = vercel.headers.map((block) => ({ path: block.source.replace("(.*)", "*"), headers: block.headers }));
    expect(blocks()).toEqual(expected);
  });

  it("lets a browser keep the web fonts a year (#262): their names carry their versions", () => {
    const fonts = blocks().find((block) => block.path === "/fonts/*");
    expect(fonts?.headers).toEqual([{ key: "Cache-Control", value: "public, max-age=31536000, immutable" }]);
    const files = readdirSync(join(PUBLIC, "fonts")).filter((file) => file.endsWith(".woff2"));
    expect(files.length).toBeGreaterThan(0);
    for (const file of files) expect(file, "a versioned name").toMatch(/-\d+\.\d+\.\d+-/u);
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
