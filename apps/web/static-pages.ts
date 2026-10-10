/// <reference types="vite/client" />
// Static pages give crawlers page-specific HTML and are replaced at client startup without hydration.
// Reuse `net/head.ts`, `almanacShelf` (R674, R1420; no unshipped cards), `fillParams`, and `patches.json`.
// This Vite entry uses relative imports because Vitest and Cypress do not resolve `@jackioh/*` here.
// `<style>` hides static content until startup; `<noscript>` restores it without script.

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

function swap(html: string, pattern: RegExp, value: string): string {
  if (!pattern.test(html)) throw new Error(`static-pages.ts: index.html has no match for ${String(pattern)}`);
  return html.replace(pattern, (_whole, lead: string) => lead + value);
}

const costText = (cost: CardCost): string =>
  typeof cost === "object" ? `${String(cost.base)} embiggen ${String(cost.embiggen)}` : String(cost);

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

export function sitemapXml(pages: readonly PublicPage[]): string {
  const urls = pages.map((page) => `  <url><loc>${canonicalOf(page.path)}</loc></url>\n`).join("");
  return `<?xml version="1.0" encoding="UTF-8"?>\n<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">\n${urls}</urlset>\n`;
}

export type StaticPagesInput = {
  template: string;
  pages: readonly PublicPage[];
  cards: CardDefs;
  patches: readonly Patch[];
  outDir: string;
};

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
      // Reading here avoids bundling the catalog into each config load.
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
