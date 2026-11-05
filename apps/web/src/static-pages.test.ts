// The build's static pages (`apps/web/static-pages.ts`), written from the data the app reads, so a card or
// a patch added to it needs no edit here, and the public files it rewrites with the build's hosts
// (robots.txt, security.txt, `_headers`' CSP). `writeStaticPages` runs on a temp directory with the source
// `index.html`, so no test needs a full build.

import { copyFileSync, existsSync, mkdirSync, mkdtempSync, readFileSync, readdirSync, rmSync, writeFileSync } from "node:fs";
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

import {
  ADDRESSED_PUBLIC_FILES,
  CONNECT_SRC_PLACEHOLDER,
  HEADERS_FILE,
  NOT_FOUND_FILE,
  NOT_FOUND_HAND,
  PAGE_TEXT,
  PUBLIC_PAGES,
  connectSources,
  notFoundHtml,
  sitemapXml,
  staticPages,
  writeStaticPages,
} from "../static-pages.ts";
import { CardFace } from "./cards/CardFace.tsx";
import { faceModel } from "./cards/model.ts";
import { DEFAULT_FILTER, DEFAULT_SORT, almanacPool } from "./game/deckbuilder/filters.ts";
import { almanacShelf } from "./game/deckbuilder/shelf.ts";
import { SITE_ORIGIN, originOf, paths, siteOrigin } from "./net/navigate.ts";
import { canonicalUrlFor, documentTitleFor } from "./net/head.ts";
import { newestFirst } from "./patches/history.ts";
import { releaseArtAssets, releaseArtPath } from "./patches/releaseArt.ts";
import { NOTE_SECTIONS, RELEASES, playerTitle, releaseGroups } from "./patches/releases.ts";
import { FIXTURE_PATCHES, FIXTURE_SNAPSHOTS, M1, M2, TIERED_PATCHES, TIERED_SNAPSHOTS, V1, V2, V3, fixtureDef } from "./patches/fixtures.ts";
import type { Patch, Snapshot } from "./patches/source.ts";
import AccessibilityRoute from "./routes/accessibility.tsx";
import AiRoute from "./routes/ai.tsx";
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

/** The public files the build rewrites, copied into a fresh output directory as Vite copies them. */
function publicFiles(): string {
  const outDir = mkdtempSync(join(tmpdir(), "jackioh-static-"));
  dirs.push(outDir);
  for (const file of [...ADDRESSED_PUBLIC_FILES, HEADERS_FILE]) {
    const target = join(outDir, file);
    mkdirSync(dirname(target), { recursive: true });
    copyFileSync(join(HERE, "../public", file), target);
  }
  return outDir;
}

/**
 * Writes every page for this data and origin, with the public files it rewrites, and returns the
 * directory: with a Supabase host, the CSP names it; with a snapshot reader, each patch's release
 * artwork is written too.
 */
function writeAll(
  cards: CardDefs,
  patches: readonly Patch[],
  origin: string = SITE_ORIGIN,
  {
    supabaseUrl,
    snapshot,
  }: { supabaseUrl?: string | undefined; snapshot?: (version: string) => Snapshot | null } = {},
): string {
  const outDir = publicFiles();
  writeStaticPages({
    template: TEMPLATE,
    pages: PUBLIC_PAGES,
    cards,
    patches,
    ...(snapshot === undefined ? {} : { snapshot }),
    outDir,
    origin,
    supabaseUrl,
  });
  return outDir;
}

/** A `.env` file's value for `key`. */
const envValue = (file: string, key: string): string | undefined =>
  new RegExp(`^${key}=(.*)$`, "mu").exec(readFileSync(join(HERE, "..", file), "utf8"))?.[1];

/** One written page's HTML, as text and as a document. */
function read(outDir: string, path: string): { html: string; doc: Document } {
  const html = readFileSync(join(outDir, path, "index.html"), "utf8");
  return { html, doc: new DOMParser().parseFromString(html, "text/html") };
}

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

  it("name the host each build is served from in every address and in its CSP: jackioh.com in production, staging.jackioh.com on staging", () => {
    const production = originOf(envValue(".env.production", "VITE_SITE_ORIGIN"));
    const staging = originOf(envValue(".env.staging", "VITE_SITE_ORIGIN"));
    expect(production).toBe("https://jackioh.com");
    expect(staging).toBe(SITE_ORIGIN);
    for (const [origin, file] of [
      [production, ".env.production"],
      [staging, ".env.staging"],
    ] as const) {
      const other = origin === production ? staging : production;
      const supabase = envValue(file, "VITE_SUPABASE_URL");
      const outDir = writeAll(CATALOG, REAL_PATCHES, origin, { supabaseUrl: supabase });
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
      const headers = readFileSync(join(outDir, HEADERS_FILE), "utf8");
      const connect = /connect-src ([^;]+);/u.exec(headers)?.[1];
      // R2983000: Google Identity Services' requests beside the Supabase host the build signs in through.
      expect(connect, `${origin} _headers`).toBe(
        `'self' ${supabase ?? ""} https://accounts.google.com/gsi/ wss://${new URL(origin).host}`,
      );
      expect(connect, `${origin} _headers`).toBe(connectSources(origin, supabase));
      expect(headers, `${origin} _headers names no other host`).not.toContain(`wss://${new URL(other).host}`);
      expect(headers, `${origin} _headers leaves no placeholder`).not.toMatch(/%[A-Z_]+%/u);
    }
  });

  it("refuse a _headers file the build cannot fill: one without the CSP's placeholder, or with one nothing fills", () => {
    const write = (headers: string): void => {
      const outDir = publicFiles();
      writeFileSync(join(outDir, HEADERS_FILE), headers);
      writeStaticPages({ template: TEMPLATE, pages: PUBLIC_PAGES, cards: {}, patches: [], outDir, origin: SITE_ORIGIN });
    };
    const source = readFileSync(join(HERE, "../public", HEADERS_FILE), "utf8");
    expect(source).toContain(CONNECT_SRC_PLACEHOLDER);
    expect(() => {
      write(source);
    }).not.toThrow();
    expect(() => {
      write(source.replace(CONNECT_SRC_PLACEHOLDER, "'self'"));
    }).toThrow(/does not spell %CONNECT_SRC%/u);
    expect(() => {
      write(`${source}/x\n  X-Other: %SOMETHING_ELSE%\n`);
    }).toThrow(/still holds %SOMETHING_ELSE%/u);
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

  it("quote /privacy, /terms, /accessibility, /ai and /copyright word for word from their own pages", () => {
    const page = build(CATALOG, REAL_PATCHES);
    const cases = [
      [PrivacyRoute, paths.privacy],
      [TermsRoute, paths.terms],
      [AccessibilityRoute, paths.accessibility],
      [AiRoute, paths.ai],
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

describe("#2681 the release artwork beside the static pages", () => {
  const snapshot = (version: string): Snapshot | null => TIERED_SNAPSHOTS[version] ?? null;
  // V1 is a major, V2 and V3 patches and M1 and M2 micros (R3012001).
  const patches = TIERED_PATCHES;

  it("R3012001 writes release art for every major and patch, a still companion for each major, and no file for a new micro", () => {
    const outDir = writeAll(fixtureCatalog(), patches, SITE_ORIGIN, { snapshot });
    expect(readdirSync(join(outDir, "release-art")).sort()).toEqual([`${V1}-still.svg`, `${V1}.svg`, `${V2}.svg`, `${V3}.svg`].sort());
    expect(existsSync(join(outDir, releaseArtPath(M1)))).toBe(false);
    expect(existsSync(join(outDir, releaseArtPath(M2)))).toBe(false);
    // Each file is what releaseArt.ts draws for its path, and nothing in any of them moves; a major's `-still` file is its poster.
    const assets = releaseArtAssets(patches, snapshot);
    expect(assets).toHaveLength(4);
    for (const asset of assets) {
      const written = readFileSync(join(outDir, asset.path), "utf8");
      expect(written, asset.path).toBe(asset.svg);
      expect(written, asset.path).not.toMatch(/<style|<animate|@keyframes|animation/u);
    }
    expect(readFileSync(join(outDir, releaseArtPath(V1, "still")), "utf8")).toBe(readFileSync(join(outDir, releaseArtPath(V1)), "utf8"));
    // The pages are written as before, and a build handed no snapshots writes no artwork.
    expect(existsSync(join(outDir, "patch-notes", "index.html"))).toBe(true);
    expect(existsSync(join(writeAll(fixtureCatalog(), patches), "release-art"))).toBe(false);
  });

  it("#2681 the dev server draws a release's artwork when asked, and passes every other path on", () => {
    type Handler = (request: { url?: string }, response: { setHeader: (name: string, value: string) => void; end: (body: string) => void }, next: (error?: unknown) => void) => void;
    const plugin = staticPages() as unknown as {
      configResolved: (config: unknown) => void;
      configureServer: (server: { middlewares: { use: (handler: Handler) => void } }) => void;
    };
    plugin.configResolved({ root: join(HERE, ".."), build: { outDir: "dist" }, env: {} });
    let handler: Handler | null = null;
    plugin.configureServer({ middlewares: { use: (use) => (handler = use) } });
    const serve = (url: string): { body: string | null; type: string | null; passed: boolean } => {
      const seen = { body: null as string | null, type: null as string | null, passed: false };
      handler?.(
        { url },
        {
          setHeader: (name, value) => {
            if (name === "Content-Type") seen.type = value;
          },
          end: (body) => {
            seen.body = body;
          },
        },
        () => {
          seen.passed = true;
        },
      );
      return seen;
    };
    // The real history: a major's file and its still companion, a patch's file, and a micro patch's file from before micros lost theirs, with a query string.
    const poster = serve(`${releaseArtPath("v0.4.0")}?t=1`);
    expect(poster.type).toBe("image/svg+xml");
    expect(poster.body).toContain("v0.4.0");
    expect(poster.body).not.toContain("<style");
    expect(serve(releaseArtPath("v0.4.0", "still")).body).toBe(poster.body);
    expect(serve(releaseArtPath("v0.4.1")).body).toContain("v0.4.1");
    expect(serve(releaseArtPath("v0.1.0b")).body).toContain("v0.1.0b");
    for (const url of [releaseArtPath("v0.4.1", "still"), "/release-art/nobody.svg", "/patch-notes", "/", "/release-art/"]) {
      const seen = serve(url);
      expect(seen.passed, url).toBe(true);
      expect(seen.body, url).toBeNull();
    }
  });
});

describe("R3127000 the 404 page", () => {
  /** The built 404.html, as text and as a document. */
  function page(): { html: string; doc: Document } {
    const html = readFileSync(join(writeAll({}, []), NOT_FOUND_FILE), "utf8");
    return { html, doc: new DOMParser().parseFromString(html, "text/html") };
  }

  /** An element as the test compares it: its tag, its attributes but `style` (sorted), its style, its own text and its children. */
  type Shape = { tag: string; attrs: string[]; style: string; text: string; children: Shape[] };
  const shapeOf = (el: Element): Shape => ({
    tag: el.tagName.toLowerCase(),
    attrs: [...el.attributes]
      .filter((a) => a.name !== "style")
      .map((a) => `${a.name}=${a.value}`)
      .sort(),
    style: (el as HTMLElement).style.cssText,
    text: [...el.childNodes]
      .filter((node) => node.nodeType === Node.TEXT_NODE)
      .map((node) => node.textContent ?? "")
      .join(""),
    children: [...el.children].map(shapeOf),
  });

  it("R3127000 writes 404.html from the built index.html: its entry script, noindex, the 404 title, no canonical", () => {
    const outDir = writeAll({}, []);
    const { html, doc } = page();
    const entry = (source: Document): string | null | undefined =>
      source.querySelector('script[type="module"]')?.getAttribute("src");
    const template = new DOMParser().parseFromString(TEMPLATE, "text/html");
    expect(entry(doc)).toBe("/src/main.tsx");
    expect(entry(doc)).toBe(entry(template));
    expect(attr(doc, 'meta[name="robots"]', "content")).toBe("noindex");
    expect(doc.title).toBe(documentTitleFor(`/${NOT_FOUND_FILE}`));
    expect(doc.title).toBe("Page not found · JackiOh");
    expect(attr(doc, 'meta[property="og:title"]', "content")).toBe(doc.title);
    expect(doc.querySelector('link[rel="canonical"]')).toBeNull();
    expect(doc.querySelector('meta[property="og:url"]')).toBeNull();
    expect(doc.querySelector('script[type="application/ld+json"]')).toBeNull();
    // The shell it shares with every page is kept: the viewport, the fonts' preloads, the root the client mounts in.
    expect(doc.querySelectorAll("#root")).toHaveLength(1);
    expect(doc.querySelectorAll('link[rel="preload"][as="font"]')).toHaveLength(2);
    // Every public page is still written beside it, and the page names the build's own host, as the others do.
    for (const { path } of PUBLIC_PAGES) expect(existsSync(join(outDir, path, "index.html")), path).toBe(true);
    expect(html).toContain(`${SITE_ORIGIN}/og.png`);
    const other = "https://example.test";
    expect(notFoundHtml(TEMPLATE, other)).not.toContain(SITE_ORIGIN);
    expect(notFoundHtml(TEMPLATE, other)).toContain(`${other}/og.png`);
  });

  it("R3127000 the hand is three placeholder Spell faces named Error, Not, Found with cost gems 4, 0, 4", () => {
    const faces = [...page().doc.querySelectorAll("#root .cf")];
    expect(faces).toHaveLength(3);
    expect(faces.map((face) => face.querySelector(".card-name")?.textContent)).toEqual(["Error", "Not", "Found"]);
    expect(faces.map((face) => face.querySelector(".cost-gem")?.textContent)).toEqual(["4", "0", "4"]);
    expect(faces.map((face) => face.querySelector(".cost-gem")?.getAttribute("data-cost"))).toEqual(["4", "0", "4"]);
    expect(NOT_FOUND_HAND.map(({ name, cost }) => [name, cost])).toEqual([["Error", 4], ["Not", 0], ["Found", 4]]);
    for (const face of faces) {
      // A Spell with no attack or health, no rarity, set or crest: the gem's number is the only number on it.
      expect(face.getAttribute("data-card-type")).toBe("Spell");
      expect(face.getAttribute("data-foil")).toBe("none");
      expect(face.hasAttribute("data-rarity")).toBe(false);
      for (const part of [".cf-stats", ".cf-atk", ".cf-hp", ".cf-set", ".cf-gem", ".cf-crest", ".cf-watermark"]) {
        expect(face.querySelector(part), part).toBeNull();
      }
      expect(face.querySelector(".card-text")?.textContent).toBe("");
      expect(face.querySelector('.cf-art[data-art="procedural"]')).not.toBeNull();
      expect(face.textContent).toMatch(/^\d[A-Za-z]+Spell$/u);
    }
    // Nothing a player could mistake for a real card: no card of the catalog has these ids.
    for (const { id } of NOT_FOUND_HAND) expect(CATALOG[id], id).toBeUndefined();
  });

  it("R3127000 each hand card is what CardFace renders", () => {
    const faces = [...page().doc.querySelectorAll("#root .cf")];
    expect(faces).toHaveLength(NOT_FOUND_HAND.length);
    NOT_FOUND_HAND.forEach((card, i) => {
      const { container, unmount } = render(
        createElement(CardFace, {
          face: faceModel({ defId: card.id, name: card.name, type: "Spell", radiant: false, liveCost: card.cost }),
        }),
      );
      const rendered = container.firstElementChild;
      expect(rendered, card.name).not.toBeNull();
      expect(shapeOf(faces[i] as Element), card.name).toEqual(shapeOf(rendered as Element));
      unmount();
    });
  });

  it("R3127000 the fallback is the app's own look: the tavern board, the stacked logo, the fonts and the button home", () => {
    const { doc } = page();
    const root = doc.querySelector("#root");
    expect(root?.children).toHaveLength(1);
    const shell = root?.firstElementChild;
    expect(shell?.matches('div.app-shell.tavern.shell-panel[data-shown="true"]')).toBe(true);
    expect(shell?.getAttribute("data-testid")).toBe("not-found-static");
    const logo = shell?.querySelector("h1.shell-panel__brand > img.shell-panel__logo");
    expect(logo?.getAttribute("src")).toBe("/brand/jackioh-logo-stacked.svg");
    expect(logo?.getAttribute("alt")).toBe("JackiOh");
    expect(shell?.querySelector("section.panel.panel--auth")).not.toBeNull();
    const home = shell?.querySelector('a.button-secondary[href="/"]');
    expect(home?.textContent).toBe("Back to the start");
    // The logo's file and size are the client's own (main.tsx SHELL_LOGO).
    const main = readFileSync(join(HERE, "main.tsx"), "utf8");
    const shellLogo = /const SHELL_LOGO = \{ src: "([^"]+)", width: (\d+), height: (\d+) \}/u.exec(main);
    expect([logo?.getAttribute("src"), logo?.getAttribute("width"), logo?.getAttribute("height")]).toEqual(shellLogo?.slice(1));
    // The two web faces are preloaded from /fonts/, and src/fonts.css names both files.
    const fonts = readFileSync(join(HERE, "fonts.css"), "utf8");
    const preloads = [...doc.querySelectorAll('link[rel="preload"][as="font"]')].map((link) => link.getAttribute("href") ?? "");
    expect(preloads).toHaveLength(2);
    for (const href of preloads) {
      expect(href.startsWith("/fonts/"), href).toBe(true);
      expect(fonts, href).toContain(href);
    }
    // The plank floor and the brass-studded board are the tavern sheet's own rules.
    const tavern = readFileSync(join(HERE, "auth/tavern.css"), "utf8");
    expect(tavern).toContain('.tavern.shell-panel[data-shown="true"]');
    expect(tavern).toContain(".panel--auth::before");
    // Everything the fallback loads is the site's own (the CSP allows 'self' and data: images).
    for (const el of root?.querySelectorAll("[src], [href]") ?? []) {
      const url = el.getAttribute("src") ?? el.getAttribute("href") ?? "";
      expect(/^(\/(?!\/)|data:)/u.test(url), url).toBe(true);
    }
    const csp = readFileSync(join(HERE, "../public/_headers"), "utf8");
    expect(csp).toContain("img-src 'self' data:");
    expect(csp).toContain("font-src 'self'");
    expect(csp).toContain("style-src 'self' 'unsafe-inline'");
  });

  it("R3127000 the fallback shows with script off and while the entry script has not loaded: nothing hides it", () => {
    const { html, doc } = page();
    expect(html).not.toContain("static-body");
    expect(html).not.toContain("display:none");
    expect(html).not.toMatch(/<noscript/iu);
    expect(doc.querySelectorAll("style")).toHaveLength(0);
    // The page is #root's fallback alone, none of the landing page's hero the template holds.
    expect(doc.querySelector("#root")?.children).toHaveLength(1);
    expect(doc.querySelector(".landing")).toBeNull();
    // The stylesheet it relies on is the entry's, linked in the head, and the client's script replaces #root when it starts.
    expect(new DOMParser().parseFromString(TEMPLATE, "text/html").querySelector("#root .landing")).not.toBeNull();
  });

  it("R3127000 the hand deals, fans, turns over and floats, and lies still and face up under Reduce motion", () => {
    const css = readFileSync(join(HERE, "notfound/notfound.css"), "utf8");
    const block = (selector: string): string => {
      const start = css.indexOf(`${selector} {`);
      expect(start, selector).toBeGreaterThanOrEqual(0);
      return css.slice(start, css.indexOf("}", start));
    };
    // The card rests face up, and the slots rest fanned: every animation only runs toward them.
    expect(block(".nf-hand__card")).toContain("transform: rotateY(180deg);");
    expect(block(".nf-hand__slot")).toContain("transform: rotate(var(--turn));");
    for (const name of ["nf-hand-deal", "nf-hand-fan", "nf-hand-flip"]) {
      const frames = block(`@keyframes ${name}`);
      expect(frames, name).toContain("from {");
      expect(frames, name).not.toMatch(/\bto \{|^\s*\d+% \{/mu);
    }
    expect(css).toContain("@keyframes nf-hand-float");
    expect(css).toMatch(/\.nf-hand__slot:nth-child\(1\) \{ --turn: -15deg;/u);
    expect(css).toMatch(/\.nf-hand__slot:nth-child\(2\) \{ --turn: 0deg;/u);
    expect(css).toMatch(/\.nf-hand__slot:nth-child\(3\) \{ --turn: 15deg;/u);
    expect(block(".nf-hand__card")).toContain("nf-hand-deal");
    expect(block(".nf-hand__card")).toContain("nf-hand-flip");
    expect(block(".nf-hand__slot")).toContain("nf-hand-fan");
    expect(block(".nf-hand")).toContain("nf-hand-float");
    expect(block(".nf-hand__side--back")).toContain("/brand/jackioh-card-back.svg");
    // Reduce motion: the three moving layers stop.
    const reduced = css.slice(css.lastIndexOf("@media (prefers-reduced-motion: reduce)"));
    for (const selector of [".nf-hand", ".nf-hand__slot", ".nf-hand__card"]) expect(reduced, selector).toContain(selector);
    expect(reduced).toContain("animation: none;");
    // Every nf-hand class the markup uses is styled.
    const used = new Set([...page().html.matchAll(/class="(nf-hand[^"]*)"/gu)].flatMap((match) => (match[1] ?? "").split(" ")));
    expect([...used].sort()).toEqual(["nf-hand", "nf-hand__card", "nf-hand__side", "nf-hand__side--back", "nf-hand__side--face", "nf-hand__slot"]);
    for (const name of used) expect(css, name).toContain(`.${name}`);
  });

  it("R3127000 no public/404.html ships beside the generated one", () => {
    expect(existsSync(join(HERE, "../public/404.html"))).toBe(false);
  });
});
