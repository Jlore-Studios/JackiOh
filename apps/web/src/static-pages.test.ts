// The build's static pages (`apps/web/static-pages.ts`): each public page's own title, canonical link,
// description and text, written from the data the app reads, so a card or a patch added to it needs no
// edit here. `writeStaticPages` is called on a temp directory with the source `index.html`, so no test
// needs a full build.

import { copyFileSync, mkdirSync, mkdtempSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { createElement } from "react";

import { render } from "@testing-library/react";
import { CATALOG } from "@jackioh/cards";
import { DECK_SIZE, GLITCH_DEF_ID, HERO_HEALTH, MAX_MANA } from "@jackioh/engine/config";
import type { CardDefs } from "@jackioh/shared";
import { fillParams } from "@jackioh/shared";
import { afterEach, describe, expect, it, vi } from "vitest";

import { ADDRESSED_PUBLIC_FILES, PAGE_TEXT, PUBLIC_PAGES, sitemapXml, writeStaticPages } from "../static-pages.ts";
import { DEFAULT_FILTER, DEFAULT_SORT, almanacPool } from "./game/deckbuilder/filters.ts";
import { almanacShelf } from "./game/deckbuilder/shelf.ts";
import { SITE_ORIGIN, originOf, paths, siteOrigin } from "./net/navigate.ts";
import { canonicalUrlFor, documentTitleFor } from "./net/head.ts";
import { newestFirst } from "./patches/history.ts";
import { NOTE_SECTIONS, RELEASES, playerTitle, releaseGroups } from "./patches/releases.ts";
import { FIXTURE_PATCHES, FIXTURE_SNAPSHOTS, V3, fixtureDef } from "./patches/fixtures.ts";
import type { Patch } from "./patches/source.ts";
import AccessibilityRoute from "./routes/accessibility.tsx";
import CopyrightRoute from "./routes/copyright.tsx";
import PrivacyRoute from "./routes/privacy.tsx";
import TermsRoute from "./routes/terms.tsx";
import { SHIPPED_SETS } from "./wire/catalog.ts";

const HERE = dirname(fileURLToPath(import.meta.url));
const TEMPLATE = readFileSync(join(HERE, "../index.html"), "utf8");
const REAL_PATCHES = JSON.parse(
  readFileSync(join(HERE, "../../../crates/cards/patches/patches.json"), "utf8"),
) as Patch[];

const ADDED_PATCH: Patch = {
  version: "v6-fourth",
  date: "2026-04-05",
  title: "Added later",
  source: "Fixture commit 4",
  notes: "A patch added to the list alone.",
  changes: [],
};

const dirs: string[] = [];
afterEach(() => {
  for (const dir of dirs.splice(0)) rmSync(dir, { recursive: true, force: true });
});

/** Writes every page for this data and origin, with the addressed public files, and returns the directory. */
function writeAll(cards: CardDefs, patches: readonly Patch[], origin: string = SITE_ORIGIN): string {
  const outDir = mkdtempSync(join(tmpdir(), "jackioh-static-"));
  dirs.push(outDir);
  for (const file of ADDRESSED_PUBLIC_FILES) {
    const target = join(outDir, file);
    mkdirSync(dirname(target), { recursive: true });
    copyFileSync(join(HERE, "../public", file), target);
  }
  writeStaticPages({ template: TEMPLATE, pages: PUBLIC_PAGES, cards, patches, outDir, origin });
  return outDir;
}

/** One written page's HTML, as text and as a document. */
function read(outDir: string, path: string): { html: string; doc: Document } {
  const html = readFileSync(join(outDir, path, "index.html"), "utf8");
  return { html, doc: new DOMParser().parseFromString(html, "text/html") };
}

/** Writes every page for this data and returns a reader of one page's HTML, as text and as a document. */
function build(
  cards: CardDefs,
  patches: readonly Patch[],
  origin: string = SITE_ORIGIN,
): (path: string) => { html: string; doc: Document } {
  const outDir = writeAll(cards, patches, origin);
  return (path) => read(outDir, path);
}

/** Rendered text with its whitespace collapsed, for comparing static copy with its route. */
const norm = (t: string | null | undefined): string => (t ?? "").replace(/\s+/gu, " ").trim();

/** The fixture catalog, with a card added to a shipped set, Glitch, and a card of a set not shipped. */
function fixtureCatalog(): CardDefs {
  const card = fixtureDef(V3, "classic-001");
  return {
    ...(FIXTURE_SNAPSHOTS[V3] ?? {}),
    "classic-777": { ...card, id: "classic-777", index: "777", name: "Card Added Later" },
    [GLITCH_DEF_ID]: { ...card, id: GLITCH_DEF_ID, index: "T", name: "Glitch" },
    // A set the catalog could hold before it ships (R1420): Boss is reserved and not in SHIPPED_SETS.
    "boss-001": { ...card, id: "boss-001", index: "1", name: "Unreleased Boss", set: "Boss" },
  };
}

const attr = (doc: Document, selector: string, name: string): string | null | undefined =>
  doc.head.querySelector(selector)?.getAttribute(name);

describe("the static pages", () => {
  it("give every public page its own title, canonical link, description and one heading", () => {
    const page = build(CATALOG, REAL_PATCHES);
    for (const { path, description } of PUBLIC_PAGES) {
      const { doc } = page(path);
      const canonical = canonicalUrlFor(path);
      expect(doc.querySelectorAll("title"), path).toHaveLength(1);
      expect(doc.title, path).toBe(documentTitleFor(path));
      expect(doc.querySelectorAll('link[rel="canonical"]'), path).toHaveLength(1);
      expect(attr(doc, 'link[rel="canonical"]', "href"), path).toBe(canonical);
      expect(attr(doc, 'meta[property="og:url"]', "content"), path).toBe(canonical);
      expect(attr(doc, 'meta[property="og:title"]', "content"), path).toBe(documentTitleFor(path));
      expect(attr(doc, 'meta[name="description"]', "content"), path).toBe(description);
      expect(attr(doc, 'meta[property="og:description"]', "content"), path).toBe(description);
      expect(doc.querySelectorAll("h1"), path).toHaveLength(1);
      const links = [...doc.querySelectorAll("#root .static-body nav a")].map((a) => a.getAttribute("href"));
      expect(links, path).toEqual(PUBLIC_PAGES.map((other) => other.path).filter((other) => other !== path));
    }
    const source = new DOMParser().parseFromString(TEMPLATE, "text/html");
    expect(PUBLIC_PAGES[0]?.description).toBe(attr(source, 'meta[name="description"]', "content"));
  });

  it("hide their static text from a visitor whose script runs, show it without script, and keep the landing hero", () => {
    const page = build(CATALOG, REAL_PATCHES);
    for (const { path } of PUBLIC_PAGES) {
      const { html, doc } = page(path);
      const head = html.slice(0, html.indexOf("</head>"));
      expect(head.split(".static-body{display:none}"), path).toHaveLength(2);
      expect(head.split("<noscript><style>.static-body{display:block}</style></noscript>"), path).toHaveLength(2);
      const classes = [...(doc.getElementById("root")?.children ?? [])].map((child) => child.className);
      expect(classes, path).toEqual(path === paths.landing ? ["landing tavern", "static-body"] : ["static-body"]);
    }
  });

  it.each([
    ["the real catalog", CATALOG],
    ["the fixture catalog", fixtureCatalog()],
  ])("R630 list on /almanac exactly the almanac's shelf, in catalog order, both faces filled in (%s)", (_name, cards) => {
    const { doc } = build(cards, [])(paths.almanac);
    const ids = [...doc.querySelectorAll("article")].map((article) => article.id);
    expect(ids).toEqual(almanacShelf(cards));
    expect([...ids].sort()).toEqual([...almanacPool({ version: "test", cards }, DEFAULT_FILTER, DEFAULT_SORT)].sort());
    for (const article of doc.querySelectorAll("article")) expect(article.textContent, article.id).not.toContain("{");
  });

  it("R674 R1420 put a card added to a shipped set on /almanac with no other edit, and never Glitch or an unshipped set's card", () => {
    const cards = fixtureCatalog();
    const { doc } = build(cards, [])(paths.almanac);
    const ids = [...doc.querySelectorAll("article")].map((article) => article.id);
    expect(ids).toContain("classic-777");
    expect(ids).not.toContain(GLITCH_DEF_ID);
    expect(ids).not.toContain("boss-001");
    const bolt = fixtureDef(V3, "core-002");
    const text = doc.getElementById("core-002")?.textContent ?? "";
    expect(text).toContain(fillParams(bolt, "base"));
    expect(text).toContain(fillParams(bolt, "radiant"));
    expect(text).toContain("(2) Cost, Embiggened (4)");
  });

  it.each([
    ["the fixture history and a patch added to it", [...FIXTURE_PATCHES, ADDED_PATCH]],
    ["the real history", REAL_PATCHES],
  ])("R388 R724000 list every patch on /patch-notes by release line, newest first, with its player note (%s)", (_name, patches) => {
    const { doc } = build({}, patches)(paths.patchNotes);
    expect([...doc.querySelectorAll(".static-body section > h2")].map((h) => h.textContent)).toEqual(
      releaseGroups(patches, RELEASES).map(({ release }) => release.name),
    );
    expect([...doc.querySelectorAll(".static-body article h3")].map((h) => h.textContent)).toEqual(
      newestFirst(patches).map((patch) => `${patch.version} ${playerTitle(patch)}`),
    );
    const text = doc.body.textContent ?? "";
    for (const patch of patches) {
      expect(text, patch.version).toContain(patch.date);
      for (const { key } of NOTE_SECTIONS) {
        for (const line of patch.player?.[key] ?? []) expect(text, patch.version).toContain(line);
      }
      // The record's notes are never what players read (R724001).
      expect(text, patch.version).not.toContain(patch.notes);
    }
    for (const { release } of releaseGroups(patches, RELEASES)) {
      expect(text, release.id).toContain(release.summary);
      for (const line of release.technical) expect(text, release.id).toContain(line);
    }
  });

  it("write the sitemap from the page table and SITE_ORIGIN: addresses only", () => {
    const outDir = writeAll({}, []);
    const sitemap = readFileSync(join(outDir, "sitemap.xml"), "utf8");
    expect(sitemap).toBe(sitemapXml(PUBLIC_PAGES));
    const locs = [...sitemap.matchAll(/<loc>([^<]*)<\/loc>/gu)].map((match) => match[1]);
    expect(locs).toEqual(PUBLIC_PAGES.map((page) => `${SITE_ORIGIN}${page.path}`));
    expect(sitemap).not.toMatch(/lastmod|changefreq|priority/u);
  });

  it("name the host each build is served from in every address: jackioh.com in production, the Vercel host on staging", () => {
    const production = originOf(/^VITE_SITE_ORIGIN=(.*)$/mu.exec(readFileSync(join(HERE, "../.env.production"), "utf8"))?.[1]);
    const vercel = JSON.parse(readFileSync(join(HERE, "../../../vercel.json"), "utf8")) as { buildCommand: string };
    const staging = originOf(/VITE_SITE_ORIGIN=(\S+)/u.exec(vercel.buildCommand)?.[1]);
    expect(production).toBe("https://jackioh.com");
    expect(staging).toBe(SITE_ORIGIN);
    for (const origin of [production, staging]) {
      const other = origin === production ? staging : production;
      const outDir = writeAll(CATALOG, REAL_PATCHES, origin);
      for (const { path } of PUBLIC_PAGES) {
        const { html, doc } = read(outDir, path);
        expect(attr(doc, 'link[rel="canonical"]', "href"), `${origin} ${path}`).toBe(`${origin}${path}`);
        expect(attr(doc, 'meta[property="og:url"]', "content"), `${origin} ${path}`).toBe(`${origin}${path}`);
        expect(attr(doc, 'meta[property="og:image"]', "content"), `${origin} ${path}`).toBe(`${origin}/og.png`);
        const jsonLd = JSON.parse(
          doc.querySelector('script[type="application/ld+json"]')?.textContent ?? "null",
        ) as { url: string };
        expect(jsonLd.url, `${origin} ${path}`).toBe(`${origin}/`);
        expect(html, `${origin} ${path} names no other host`).not.toContain(other);
      }
      const locs = [...readFileSync(join(outDir, "sitemap.xml"), "utf8").matchAll(/<loc>([^<]*)<\/loc>/gu)].map(
        (match) => match[1],
      );
      expect(locs, `${origin} sitemap`).toEqual(PUBLIC_PAGES.map((page) => `${origin}${page.path}`));
      const robots = readFileSync(join(outDir, "robots.txt"), "utf8");
      expect(robots, `${origin} robots.txt`).toContain(`Sitemap: ${origin}/sitemap.xml`);
      expect(robots, `${origin} robots.txt names no other host`).not.toContain(other);
      const security = readFileSync(join(outDir, ".well-known/security.txt"), "utf8");
      expect(security, `${origin} security.txt`).toContain(`Canonical: ${origin}/.well-known/security.txt`);
      expect(security, `${origin} security.txt names no other host`).not.toContain(other);
    }
  });

  it("give the running app the bundle's own address for its canonical link, and the source files' when the build sets none", () => {
    expect(originOf(undefined)).toBe(SITE_ORIGIN);
    expect(originOf("")).toBe(SITE_ORIGIN);
    expect(originOf("https://jackioh.com/")).toBe("https://jackioh.com");
    expect(siteOrigin()).toBe(SITE_ORIGIN);
    try {
      vi.stubEnv("VITE_SITE_ORIGIN", "https://jackioh.com");
      expect(canonicalUrlFor(paths.privacy)).toBe("https://jackioh.com/privacy");
      expect(canonicalUrlFor("/nope")).toBeNull();
    } finally {
      vi.unstubAllEnvs();
    }
  });

  it("describe the game on the landing page without script: its lanes, mana, traps, numbers and sets", () => {
    const { doc } = build(CATALOG, REAL_PATCHES)(paths.landing);
    const text = doc.querySelector("#root .static-body")?.textContent ?? "";
    for (const want of [
      `${String(DECK_SIZE)}-card deck`,
      `${String(HERO_HEALTH)} health`,
      `up to ${String(MAX_MANA)}`,
      "shared lanes",
      "face down in the backrow",
      "Radiant face",
      "tutorial",
      "invite code",
      "almanac",
      "patch notes",
      ...SHIPPED_SETS,
    ]) {
      expect(text, want).toContain(want);
    }
  });

  it("give every public page text of its own beyond its title and description", () => {
    const page = build(CATALOG, REAL_PATCHES);
    for (const { path, description } of PUBLIC_PAGES) {
      const { doc } = page(path);
      const body = doc.querySelector("#root .static-body");
      expect(body?.querySelector("p")?.textContent, path).toBe(description);
      if (path === paths.almanac || path === paths.patchNotes) continue;
      expect(PAGE_TEXT[path], path).toBeDefined();
      const sections = [...(body?.querySelectorAll("section") ?? [])];
      expect(sections.length, `${path} has a section`).toBeGreaterThan(0);
      for (const element of sections) {
        expect(element.querySelector("h2")?.textContent?.trim() ?? "", `${path} section heading`).not.toBe("");
        const texts = [...element.querySelectorAll("p, li")].map((el) => el.textContent?.trim() ?? "");
        expect(texts.length, `${path} section text`).toBeGreaterThan(0);
        for (const text of texts) expect(text, path).not.toBe("");
      }
    }
  });

  it("quote /privacy, /terms, /accessibility and /copyright word for word from their own pages", () => {
    const page = build(CATALOG, REAL_PATCHES);
    const cases = [
      [PrivacyRoute, paths.privacy],
      [TermsRoute, paths.terms],
      [AccessibilityRoute, paths.accessibility],
      [CopyrightRoute, paths.copyright],
    ] as const;
    for (const [Route, path] of cases) {
      const { unmount } = render(createElement(Route));
      const said = new Set(
        [...document.querySelectorAll("section h3, section p, section li")].map((el) => norm(el.textContent)),
      );
      unmount();
      const { doc } = page(path);
      const statics = [
        ...doc.querySelectorAll("#root .static-body section h2, #root .static-body section p, #root .static-body section li"),
      ];
      expect(statics.length, `${path} static-pages.ts PAGE_TEXT has text`).toBeGreaterThan(0);
      for (const el of statics) {
        expect(said, `${path} static-pages.ts PAGE_TEXT quotes its route: ${norm(el.textContent)}`).toContain(
          norm(el.textContent),
        );
      }
    }
  });
});
