/// <reference types="vite/client" />
// The build writes a real HTML file for each public page, and `sitemap.xml`, so a crawler or a link
// preview that reads a page without running its script sees that page's title, description and text,
// and not the landing hero every route used to share. Nothing the browser runs changes: each file is
// the built `index.html` with its head swapped for the page's and `#root` filled with plain markup,
// which `createRoot` replaces the way it replaces the landing hero. There is no hydration.
//
// NOTHING IS TYPED TWICE. A title and a canonical link are `net/head.ts`'s, the almanac's cards are
// `almanacShelf`'s (the page's own shelf rule: R674, R1420, so a card of a set that has not shipped is
// never published) with `fillParams`'s text, and the patches are `patches.json`'s, newest first. The
// nine descriptions below are the only text written here. A page's prose (the privacy policy) is not
// copied: a second copy is a second thing to keep accurate.
//
// ALIAS-FREE. Vite loads this file through `vite.config.ts`, which `vitest.config.ts` and
// `e2e/cypress.config.ts` import too, and none of them resolve the client's `@jackioh/*` aliases
// there. It may reach only modules whose imports are relative, which is why the shelf rule lives in
// `shelf.ts` and not `filters.ts`.
//
// THE HEAD RULE. A visitor whose script runs sees no static text flash: a `<style>` hides it, and a
// `<noscript>` style shows it again to a visitor without script. The CSP allows inline style and no
// inline script. The landing hero is kept on `/`, as it always painted there.

import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";

import type { Plugin } from "vite";

import { almanacShelf } from "./src/game/deckbuilder/shelf.ts";
import { SITE_NAME, canonicalUrlFor, documentTitleFor, screenNameFor } from "./src/net/head.ts";
import { paths } from "./src/net/navigate.ts";
import type { Patch } from "./src/patches/source.ts";
import { SHIPPED_SETS, fillParams } from "./src/wire/catalog.ts";
import type { CardCost, CardDefs } from "./src/wire/index.ts";

export type PublicPage = { readonly path: string; readonly description: string };

/** The public pages: one row each, in the sitemap's order. A new public route is a row here. */
export const PUBLIC_PAGES: readonly PublicPage[] = [
  {
    path: paths.landing,
    description:
      "JackiOh is a 1v1 card game of lanes, mana and hidden traps. Play the AI in your browser with no account, or play friends online with an invite code.",
  },
  {
    path: paths.practice,
    description:
      "Play JackiOh against the AI in your browser, with no account: pick a difficulty, or learn the game in the tutorial.",
  },
  {
    path: paths.login,
    description:
      "Sign in to JackiOh, or create an account to build decks and play friends online with an invite code.",
  },
  {
    path: paths.privacy,
    description:
      "JackiOh's privacy policy: what the site collects, what your browser keeps, and how to delete your account.",
  },
  { path: paths.terms, description: "The terms of playing JackiOh." },
  {
    path: paths.accessibility,
    description: "JackiOh's accessibility statement, and how to tell us about a barrier.",
  },
  {
    path: paths.patchNotes,
    description: "Every JackiOh patch, newest first: its date, its title and what it changed.",
  },
  {
    path: paths.almanac,
    description: `Every JackiOh card of ${SHIPPED_SETS.join(", ")}, tokens included, with its base and Radiant text.`,
  },
  {
    path: paths.stats,
    description: "JackiOh's public statistics: every card's win rate and play rate, and the players' records.",
  },
];

const STATIC_HEAD =
  "<style>.static-body{display:none}</style><noscript><style>.static-body{display:block}</style></noscript>";
const ROOT_OPEN = '<div id="root">';

const escapeHtml = (text: string): string => text.replace(/[&<>"]/gu, (c) => `&#${String(c.charCodeAt(0))};`);

function nameOf(path: string): string {
  const name = screenNameFor(path);
  if (name === null) throw new Error(`static-pages.ts: ${path} is not a screen the client serves`);
  return name === "" ? SITE_NAME : name;
}

function canonicalOf(path: string): string {
  const url = canonicalUrlFor(path);
  if (url === null) throw new Error(`static-pages.ts: ${path} has no canonical address`);
  return url;
}

/** `html` with the one match of `pattern` (its group 1 kept) followed by `value` in place of the rest. */
function swap(html: string, pattern: RegExp, value: string): string {
  if (!pattern.test(html)) throw new Error(`static-pages.ts: index.html has no match for ${String(pattern)}`);
  return html.replace(pattern, (_whole, lead: string) => lead + value);
}

const costText = (cost: CardCost): string =>
  typeof cost === "object" ? `${String(cost.base)} embiggen ${String(cost.embiggen)}` : String(cost);

/** One entry per card the almanac shows, in catalog order, both faces with their numbers filled in. */
function almanacList(cards: CardDefs): string {
  const entries: string[] = [];
  for (const id of almanacShelf(cards)) {
    const def = cards[id];
    if (def === undefined) continue;
    const facts = `${def.set} · ${def.type} · ${def.rarity} · Cost ${costText(def.cost)}`;
    const base = fillParams(def, "base");
    const radiant = fillParams(def, "radiant");
    entries.push(
      `<article id="${escapeHtml(id)}"><h2>${escapeHtml(def.name)}</h2><p>${escapeHtml(facts)}</p>` +
        (base === "" ? "" : `<p>${escapeHtml(base)}</p>`) +
        (radiant === "" ? "" : `<p>Radiant: ${escapeHtml(radiant)}</p>`) +
        "</article>",
    );
  }
  return entries.join("\n");
}

/** One entry per patch, newest first (the page's `newestFirst`). */
function patchList(patches: readonly Patch[]): string {
  return [...patches]
    .reverse()
    .map(
      (patch) =>
        `<article><h2>${escapeHtml(`${patch.version} ${patch.title}`)}</h2>` +
        `<p><time datetime="${escapeHtml(patch.date)}">${escapeHtml(patch.date)}</time></p>` +
        (patch.notes === "" ? "" : `<p>${escapeHtml(patch.notes)}</p>`) +
        "</article>",
    )
    .join("\n");
}

function pageHtml(template: string, page: PublicPage, pages: readonly PublicPage[], extra: string): string {
  const title = escapeHtml(documentTitleFor(page.path));
  const description = escapeHtml(page.description);
  const url = canonicalOf(page.path);
  let html = swap(template, /(<title>)[^<]*/u, title);
  html = swap(html, /(<meta name="description" content=")[^"]*/u, description);
  html = swap(html, /(<meta property="og:title" content=")[^"]*/u, title);
  html = swap(html, /(<meta property="og:description" content=")[^"]*/u, description);
  html = swap(html, /(<meta property="og:url" content=")[^"]*/u, url);
  html = swap(html, /()<\/head>/u, `<link rel="canonical" href="${url}" />${STATIC_HEAD}</head>`);

  const links = pages
    .filter((other) => other.path !== page.path)
    .map((other) => `<a href="${other.path}">${escapeHtml(nameOf(other.path))}</a>`)
    .join(" ");
  const nav = `<nav aria-label="${SITE_NAME}">${links}</nav>`;

  const open = html.indexOf(ROOT_OPEN) + ROOT_OPEN.length;
  const close = html.lastIndexOf("</div>");
  if (open < ROOT_OPEN.length || close < open) throw new Error("static-pages.ts: index.html has no #root");
  const body =
    page.path === paths.landing
      ? `${html.slice(open, close)}<div class="static-body">${nav}</div>`
      : `<div class="static-body"><h1>${escapeHtml(nameOf(page.path))}</h1><p>${description}</p>${nav}\n${extra}</div>`;
  return html.slice(0, open) + body + html.slice(close);
}

/** `sitemap.xml`: every page's canonical address, and nothing else. */
export function sitemapXml(pages: readonly PublicPage[]): string {
  const urls = pages.map((page) => `  <url><loc>${canonicalOf(page.path)}</loc></url>\n`).join("");
  return `<?xml version="1.0" encoding="UTF-8"?>\n<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">\n${urls}</urlset>\n`;
}

export type StaticPagesInput = {
  /** The built `index.html`. */
  template: string;
  pages: readonly PublicPage[];
  cards: CardDefs;
  /** `patches.json`, oldest first. */
  patches: readonly Patch[];
  outDir: string;
};

/** Writes `<outDir>/<path>/index.html` for each page (`/` is `<outDir>/index.html`) and `sitemap.xml`. */
export function writeStaticPages({ template, pages, cards, patches, outDir }: StaticPagesInput): void {
  for (const page of pages) {
    const extra =
      page.path === paths.almanac ? almanacList(cards) : page.path === paths.patchNotes ? patchList(patches) : "";
    const file = join(outDir, page.path, "index.html");
    mkdirSync(dirname(file), { recursive: true });
    writeFileSync(file, pageHtml(template, page, pages, extra));
  }
  writeFileSync(join(outDir, "sitemap.xml"), sitemapXml(pages));
}

/** The build's hook around `writeStaticPages`: runs once the bundle, and so `index.html`, is written. */
export function staticPages(): Plugin {
  let root = "";
  let outDir = "";
  return {
    name: "jackioh:static-pages",
    apply: "build",
    configResolved(config) {
      root = config.root;
      outDir = resolve(config.root, config.build.outDir);
    },
    writeBundle() {
      // Read in the hook, not imported: an import would bundle the catalog into every config load.
      const data = (file: string): unknown =>
        JSON.parse(readFileSync(resolve(root, "../../crates/cards", file), "utf8"));
      writeStaticPages({
        template: readFileSync(join(outDir, "index.html"), "utf8"),
        pages: PUBLIC_PAGES,
        cards: data("catalog.json") as CardDefs,
        patches: data("patches/patches.json") as Patch[],
        outDir,
      });
    },
  };
}
