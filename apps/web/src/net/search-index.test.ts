// #3485: what Google should index, held to the files that serve it. Every sitemap.xml URL is
// https:// and its own canonical (head.ts `canonicalUrlFor`), every sitemap page but `/` has a 200
// rewrite rule (`public/_redirects`, whose emitted-page shape cloudflare-serve.test.ts pins), and the
// account-gated screens (main.tsx's `Gated` screens with no public content) stay out of the sitemap
// and are disallowed in `public/robots.txt` on purpose: a signed-out crawler gets sent to /login in
// JS, which Search Console records as "Page with redirect". `/creator` is no route and falls through
// to the 404 page (no rule, no canonical, R3127000's noindex page).

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { describe, expect, it } from "vitest";

import { PUBLIC_PAGES, sitemapXml } from "../../static-pages.ts";
import { canonicalUrlFor } from "./head.ts";
import { paths } from "./navigate.ts";

const HERE = dirname(fileURLToPath(import.meta.url));
const PUBLIC = join(HERE, "../../public");

/** `_redirects`' rules, each `[from, to, status]`. */
const rules = readFileSync(join(PUBLIC, "_redirects"), "utf8")
  .split("\n")
  .map((line) => line.trim())
  .filter((line) => line !== "" && !line.startsWith("#"))
  .map((line) => line.split(/\s+/u));

/** `robots.txt`'s `Disallow:` values, in order. */
const disallows = readFileSync(join(PUBLIC, "robots.txt"), "utf8")
  .split("\n")
  .map((line) => line.trim())
  .filter((line) => line.startsWith("Disallow:"))
  .map((line) => line.slice("Disallow:".length).trim());

/** Every `Gated` screen of main.tsx with no public content: fixed paths, then id-addressed prefixes. */
const GATED_FIXED = [
  paths.invite,
  paths.decks,
  paths.play,
  paths.account,
  paths.leaderboard,
  paths.collection,
  paths.shop,
  paths.together,
];
const GATED_PREFIXES = ["/match/", "/series/", "/watch/", "/duos/"];

const locsOf = (sitemap: string): string[] =>
  [...sitemap.matchAll(/<loc>([^<]*)<\/loc>/gu)].map((match) => match[1] ?? "");

describe("the indexed pages (#3485)", () => {
  it("every sitemap URL is https and its own canonical (sitemap-https-and-own-canonical)", () => {
    const origin = "https://jackioh.com";
    const sitemap = sitemapXml(PUBLIC_PAGES, origin);
    const locs = locsOf(sitemap);
    expect(locs).toEqual(PUBLIC_PAGES.map((page) => `${origin}${page.path}`));
    for (const page of PUBLIC_PAGES) {
      expect(canonicalUrlFor(page.path, origin), page.path).toBe(`${origin}${page.path}`);
    }
    for (const loc of locs) expect(loc.startsWith("https://"), loc).toBe(true);
  });

  it("every sitemap page has a 200 rule (sitemap-pages-have-200-rules)", () => {
    const rows = new Set(rules.map(([from = "", to = "", status = ""]) => `${from} ${to} ${status}`));
    for (const { path } of PUBLIC_PAGES) {
      // `/` is served directly and needs no rule.
      if (path === paths.landing) continue;
      expect(rows.has(`${path} ${path}/ 200`), path).toBe(true);
    }
  });

  it("no gated route is in the sitemap (gated-routes-not-in-sitemap)", () => {
    const locs = locsOf(sitemapXml(PUBLIC_PAGES, "https://jackioh.com"));
    for (const fixed of GATED_FIXED) {
      expect(PUBLIC_PAGES.map((page) => page.path), fixed).not.toContain(fixed);
    }
    for (const loc of locs) {
      const pathname = new URL(loc).pathname;
      for (const fixed of GATED_FIXED) expect(pathname, `${loc} vs ${fixed}`).not.toBe(fixed);
      for (const prefix of GATED_PREFIXES) {
        expect(pathname.startsWith(prefix), `${loc} vs ${prefix}`).toBe(false);
      }
    }
  });

  it("every gated route is disallowed (gated-routes-disallowed)", () => {
    for (const fixed of GATED_FIXED) {
      // Robots prefix rules: a `Disallow` value blocks every path that starts with it.
      expect(
        disallows.some((line) => fixed === line || fixed.startsWith(line)),
        fixed,
      ).toBe(true);
    }
    for (const prefix of GATED_PREFIXES) {
      expect(disallows, prefix).toContain(prefix);
    }
  });

  it("/creator falls to the 404 page (creator-falls-to-404)", () => {
    const routed: string[] = Object.values(paths).flatMap((path) => (typeof path === "string" ? [path] : []));
    expect(routed).not.toContain("/creator");
    const from = new Set(rules.map(([source = ""]) => source));
    expect(from.has("/creator")).toBe(false);
    expect(from.has("/creator/")).toBe(false);
    expect(canonicalUrlFor("/creator")).toBeNull();
  });
});
