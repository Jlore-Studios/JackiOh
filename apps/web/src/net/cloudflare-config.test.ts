// Cloudflare (production) against vercel.json (staging). The two hosts must serve the same site:
// the same paths to the app, the same 404s, the same security headers. vercel.json stays the source
// of truth (deploy-routes.test.ts holds it against the route table); this file holds
// public/_redirects, public/_headers and wrangler.jsonc equal to it, so a route or header added to
// one host and not the other fails here rather than in production.
//
// Behaviour measured under `wrangler dev` (wrangler 4.147) before this file was written:
//   - a rewrite to `/index.html` answers 307 -> `/` (assets' html_handling strips it), which drops
//     the path, so every rewrite targets `/` instead;
//   - `/login/` is not matched by a `/login` rule, so each path is listed with and without the slash;
//   - unmatched paths get public/404.html with a 404 status (`not_found_handling: "404-page"`).

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { describe, expect, it } from "vitest";

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = join(HERE, "../../../..");
const PUBLIC = join(HERE, "../../public");

type VercelConfig = {
  rewrites: { source: string; destination: string }[];
  headers: { source: string; headers: { key: string; value: string }[] }[];
};

const vercel = JSON.parse(readFileSync(join(ROOT, "vercel.json"), "utf8")) as VercelConfig;

/** The paths Vercel rewrites, spelled the way _redirects must spell them. */
function expectedRules(): string[] {
  const rules: string[] = [];
  for (const { source } of vercel.rewrites) {
    const listed = /^\/\(([a-z|-]+)\)\{\/\}\?$/u.exec(source);
    const byId = /^\/\(([a-z|-]+)\)\/\(\[\^\/\]\+\)\{\/\}\?$/u.exec(source);
    const names = (listed ?? byId)?.[1]?.split("|");
    if (names === undefined) throw new Error(`cloudflare-config.test.ts cannot read this source: ${source}`);
    for (const name of names) {
      const path = byId === null ? `/${name}` : `/${name}/:id`;
      rules.push(path, `${path}/`);
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

  it("rewrites exactly the paths vercel.json rewrites, with and without a trailing slash", () => {
    expect(rules.map(([from]) => from).sort()).toEqual(expectedRules().sort());
  });

  it("rewrites (200) to `/`, never to `/index.html`, which Cloudflare answers with a redirect", () => {
    for (const rule of rules) expect(rule.slice(1), rule[0]).toEqual(["/", "200"]);
  });
});

describe("public/_headers", () => {
  it("sends vercel.json's headers on every path, value for value", () => {
    const [scope, ...rest] = lines(readFileSync(join(PUBLIC, "_headers"), "utf8"));
    expect(scope).toBe("/*");
    const headers = rest.map((line) => {
      const at = line.indexOf(":");
      return { key: line.slice(0, at).trim(), value: line.slice(at + 1).trim() };
    });
    const all = vercel.headers.find((block) => block.source === "/(.*)");
    expect(headers).toEqual(all?.headers);
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
