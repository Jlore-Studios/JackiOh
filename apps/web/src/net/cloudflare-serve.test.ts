// How Cloudflare answers a path that is no file of the build: public/_redirects rewrites the app's
// screens, wrangler.jsonc's `run_worker_first` hands the server's paths to the Worker, and everything
// else gets the 404.html the build writes, with a 404 status (`not_found_handling`). A rewrite targets `/`, not
// `/index.html`, which Cloudflare answers with a 307 to `/` that would drop the path, and a page the
// build writes (static-pages.ts) is rewritten to its own folder: a rule wins over a file, and its
// slashed path is the file itself. deploy-routes.test.ts holds the rules to the route table and to
// the Worker's paths. #3127, R3127000: the 404 page is the client itself (static-pages.ts `notFoundHtml`), so it
// loads the entry script; static-pages.test.ts holds its fallback and its hand.

import { existsSync, readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { describe, expect, it } from "vitest";

import { PUBLIC_PAGES, notFoundHtml } from "../../static-pages.ts";
import { SITE_ORIGIN } from "./navigate.ts";

const HERE = dirname(fileURLToPath(import.meta.url));
const PUBLIC = join(HERE, "../../public");

/** `_redirects`' rules, each `[from, to, status]`. */
const rules = readFileSync(join(PUBLIC, "_redirects"), "utf8")
  .split("\n")
  .map((line) => line.trim())
  .filter((line) => line !== "" && !line.startsWith("#"))
  .map((line) => line.split(/\s+/u));

/** The pages the build writes as `<path>/index.html`. */
const EMITTED = new Set(PUBLIC_PAGES.map((page) => page.path));

describe("public/_redirects", () => {
  it("rewrites (200), never to an `index.html`: a page the build writes to its own folder, any other screen to `/`", () => {
    expect(rules.length).toBeGreaterThan(0);
    for (const [from = "", to, status, ...rest] of rules) {
      expect(rest, from).toEqual([]);
      expect(status, from).toBe("200");
      expect(to, from).not.toMatch(/index\.html$/u);
      if (EMITTED.has(from)) expect(to, from).toBe(`${from}/`);
      else expect(to, from).toBe("/");
    }
  });

  it("lists both spellings of every other screen, with and without the trailing slash; a page the build writes serves its slashed path itself", () => {
    const from = new Set(rules.map(([source]) => source));
    for (const source of from) {
      if (source === undefined) continue;
      const bare = source.endsWith("/") ? source.slice(0, -1) : source;
      if (EMITTED.has(bare)) {
        expect(from.has(`${bare}/`), `${bare}/ is its own file`).toBe(false);
      } else {
        expect(from.has(bare), bare).toBe(true);
        expect(from.has(`${bare}/`), `${bare}/`).toBe(true);
      }
    }
    for (const { path } of PUBLIC_PAGES) {
      if (path !== "/") expect(from.has(path), path).toBe(true);
    }
  });
});

describe("the build's 404.html", () => {
  it("R3127000 ships no public/404.html: the build writes the page not_found_handling 404-page serves", () => {
    expect(existsSync(join(PUBLIC, "404.html"))).toBe(false);
  });

  it("R3127000 is the client's page: its entry script, noindex and a link home", () => {
    const page = notFoundHtml(readFileSync(join(HERE, "../../index.html"), "utf8"), SITE_ORIGIN);
    expect(page).toContain('src="/src/main.tsx"');
    expect(page).toContain('href="/"');
    expect(page).toContain('<meta name="robots" content="noindex" />');
    expect(page).not.toContain('rel="canonical"');
  });
});
