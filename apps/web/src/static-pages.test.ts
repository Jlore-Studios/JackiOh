// The build's static pages (`apps/web/static-pages.ts`): each public page's own title, canonical link,
// description and text, written from the data the app reads, so a card or a patch added to it needs no
// edit here. `writeStaticPages` is called on a temp directory with the source `index.html`, so no test
// needs a full build.

import { mkdtempSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { CATALOG } from "@jackioh/cards";
import { GLITCH_DEF_ID } from "@jackioh/engine/config";
import type { CardDefs } from "@jackioh/shared";
import { fillParams } from "@jackioh/shared";
import { afterEach, describe, expect, it } from "vitest";

import { PUBLIC_PAGES, sitemapXml, writeStaticPages } from "../static-pages.ts";
import { DEFAULT_FILTER, DEFAULT_SORT, almanacPool } from "./game/deckbuilder/filters.ts";
import { almanacShelf } from "./game/deckbuilder/shelf.ts";
import { SITE_ORIGIN, paths } from "./net/navigate.ts";
import { canonicalUrlFor, documentTitleFor } from "./net/head.ts";
import { newestFirst } from "./patches/history.ts";
import { FIXTURE_PATCHES, FIXTURE_SNAPSHOTS, V3, fixtureDef } from "./patches/fixtures.ts";
import type { Patch } from "./patches/source.ts";

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

/** Writes every page for this data and returns a reader of one page's HTML, as text and as a document. */
function build(cards: CardDefs, patches: readonly Patch[]): (path: string) => { html: string; doc: Document } {
  const outDir = mkdtempSync(join(tmpdir(), "jackioh-static-"));
  dirs.push(outDir);
  writeStaticPages({ template: TEMPLATE, pages: PUBLIC_PAGES, cards, patches, outDir });
  return (path) => {
    const html = readFileSync(join(outDir, path, "index.html"), "utf8");
    return { html, doc: new DOMParser().parseFromString(html, "text/html") };
  };
}

/** The fixture catalog, with a card added to a shipped set, Glitch, and a card of a set not shipped. */
function fixtureCatalog(): CardDefs {
  const card = fixtureDef(V3, "classic-001");
  return {
    ...(FIXTURE_SNAPSHOTS[V3] ?? {}),
    "classic-777": { ...card, id: "classic-777", index: "777", name: "Card Added Later" },
    [GLITCH_DEF_ID]: { ...card, id: GLITCH_DEF_ID, index: "T", name: "Glitch" },
    "meditative-001": { ...card, id: "meditative-001", index: "1", name: "Unreleased Monk", set: "Meditative" },
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
    expect(ids).not.toContain("meditative-001");
    const bolt = fixtureDef(V3, "core-002");
    const text = doc.getElementById("core-002")?.textContent ?? "";
    expect(text).toContain(fillParams(bolt, "base"));
    expect(text).toContain(fillParams(bolt, "radiant"));
    expect(text).toContain("2 embiggen 4");
  });

  it.each([
    ["the fixture history and a patch added to it", [...FIXTURE_PATCHES, ADDED_PATCH]],
    ["the real history", REAL_PATCHES],
  ])("R388 list every patch on /patch-notes, newest first (%s)", (_name, patches) => {
    const { doc } = build({}, patches)(paths.patchNotes);
    expect([...doc.querySelectorAll(".static-body h2")].map((h) => h.textContent)).toEqual(
      newestFirst(patches).map((patch) => `${patch.version} ${patch.title}`),
    );
    const text = doc.body.textContent ?? "";
    for (const patch of patches) {
      expect(text, patch.version).toContain(patch.date);
      expect(text, patch.version).toContain(patch.notes);
    }
  });

  it("write the sitemap from the page table and SITE_ORIGIN: addresses only", () => {
    const outDir = mkdtempSync(join(tmpdir(), "jackioh-static-"));
    dirs.push(outDir);
    writeStaticPages({ template: TEMPLATE, pages: PUBLIC_PAGES, cards: {}, patches: [], outDir });
    const sitemap = readFileSync(join(outDir, "sitemap.xml"), "utf8");
    expect(sitemap).toBe(sitemapXml(PUBLIC_PAGES));
    const locs = [...sitemap.matchAll(/<loc>([^<]*)<\/loc>/gu)].map((match) => match[1]);
    expect(locs).toEqual(PUBLIC_PAGES.map((page) => `${SITE_ORIGIN}${page.path}`));
    expect(sitemap).not.toMatch(/lastmod|changefreq|priority/u);
  });
});
