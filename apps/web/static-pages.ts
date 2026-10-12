/// <reference types="vite/client" />
// Static pages give crawlers page-specific HTML and are replaced at client startup without hydration.
// Reuse `net/head.ts`, `almanacShelf` (R674, R1420; no unshipped cards), `fillParams`, and `patches.json`.
// This Vite entry uses relative imports because Vitest and Cypress do not resolve `@jackioh/*` here.
// `<style>` hides static content until startup; `<noscript>` restores it without script.
// Every address names the build's own host (`StaticPagesInput.origin`, #696).

import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";

import type { Plugin } from "vite";

import { almanacShelf } from "./src/game/deckbuilder/shelf.ts";
import { SITE_NAME, canonicalUrlFor, documentTitleFor, screenNameFor } from "./src/net/head.ts";
import { SITE_ORIGIN, originOf, paths } from "./src/net/navigate.ts";
import type { Patch } from "./src/patches/source.ts";
import { SHIPPED_SETS, fillParams } from "./src/wire/catalog.ts";
import { DECK_SIZE, HERO_HEALTH, MAX_MANA } from "./src/wire/engineConfig.ts";
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

/** A section of a page's static text: its heading and its paragraphs or list, as HTML. */
const section = (heading: string, body: string): string => `<section><h2>${heading}</h2>${body}</section>`;

/**
 * Each page's own text, after its description, for a reader that runs no script. /privacy, /terms
 * and /accessibility quote their routes word for word (static-pages.test.ts renders each to check);
 * /almanac and /patch-notes are written from their data (`almanacList`, `patchList`).
 */
export const PAGE_TEXT: Readonly<Partial<Record<string, string>>> = {
  [paths.landing]:
    section(
      "A card game of lanes, mana and hidden traps",
      `<p>Each player brings a ${String(DECK_SIZE)}-card deck and a hero with ${String(HERO_HEALTH)} health. ` +
        `Mana grows each turn, up to ${String(MAX_MANA)}, and pays for Units that fight in shared lanes, ` +
        `Spells, and Traps set face down in the backrow, hidden from the opponent until they spring. ` +
        `Every card also has a Radiant face.</p>` +
        `<p>The cards come in ${String(SHIPPED_SETS.length)} sets: ${SHIPPED_SETS.join(", ")}.</p>`,
    ) +
    section(
      "Ways to play",
      "<p>Play the AI in your browser with no account, at the difficulty you choose, or learn the game " +
        "in the tutorial. Create an account to build decks and, with an invite code, play friends online. " +
        "Look up any card in the almanac, and read what each patch changed in the patch notes.</p>",
    ),
  [paths.practice]:
    section(
      "A game against the AI",
      "<p>A practice game runs in your browser, and its moves are never sent to a server, so no account " +
        "is needed. Pick a difficulty and a deck, a random one or a preset deck, or one of your saved " +
        "decks once you have signed in, and play.</p>",
    ) +
    section(
      "The tutorial",
      "<p>New to JackiOh? The tutorial at the top of the page teaches the game in short lessons, each a " +
        "game against the AI with a coach beside the board, and this browser remembers the lessons you finish.</p>",
    ),
  [paths.login]: section(
    "Your account",
    "<p>Sign in with the email address and password of your JackiOh account, or create an account here. " +
      "A new account confirms its email address first, and online play is invite-only for now: you need " +
      "an invite code after you sign up.</p>" +
      "<p>Forgot your password? This page can email you a link to set a new one. " +
      "Playing the AI needs no account at all.</p>",
  ),
  [paths.stats]: section(
    "Cards and players",
    "<p>The Cards view lists every card's win rate and play rate in a table you can sort, and filter by " +
      "set, rarity and cost. Any card opens its own figures: its win rate by patch and by the turn it " +
      "was played, and the cards most often played beside it. The Players view shows the players' " +
      "public records.</p><p>The figures load when the page opens.</p>",
  ),
  [paths.privacy]:
    section(
      "What we don&rsquo;t do",
      "<ul><li>We don&rsquo;t sell your data or share it for advertising.</li>" +
        "<li>We don&rsquo;t show ads, and we use no third-party analytics, tracking pixels or session recording.</li>" +
        "<li>We don&rsquo;t track you across other sites, and no third party tracks you across sites " +
        "through JackiOh. So a browser&rsquo;s &ldquo;Do Not Track&rdquo; signal doesn&rsquo;t change " +
        "anything here.</li></ul>",
    ) +
    section(
      "What stays on your device",
      "<p>JackiOh sets no cookies. It keeps these in your browser&rsquo;s own storage, and only to work:</p>" +
        "<ul><li>The email address of a sign-up or password reset you are waiting on, and a one-time key " +
        "that lets the emailed link sign in on this browser.</li>" +
        "<li>Your settings, such as sound and reduced motion.</li>" +
        "<li>Your tutorial progress.</li>" +
        "<li>For the open tab only, and gone when you close it: your sign-in session and a password-reset " +
        "link you opened.</li></ul>" +
        "<p>Your sign-in session stays in the tab you signed in on. You sign in again in a new tab, or " +
        "after you close the browser. Signing out removes your session from this browser.</p>",
    ) +
    section(
      "Deleting your account or getting a copy",
      "<p>You can delete your account yourself: sign in, open your Account page, and choose " +
        "&ldquo;Delete my account&rdquo;. That deletes your sign-in, your decks, trios and cards, your " +
        "tutorial progress, and the records of how you played. Finished matches and their results stay in " +
        "the other player&rsquo;s history, with your account removed from them. You can&rsquo;t delete " +
        "your account during a match or a Conquest series that hasn&rsquo;t finished.</p>" +
        "<p>To ask for a copy of your data, contact us. Please don&rsquo;t put your email address or " +
        "other personal details in a public GitHub issue.</p>",
    ),
  [paths.terms]:
    section(
      "Using JackiOh",
      "<p>You may use JackiOh for personal play. Don&rsquo;t try to break the service, automate abuse, " +
        "or bypass game rules or access controls.</p>",
    ) +
    section(
      "Accounts",
      "<p>You are responsible for activity on your account. Keep your sign-in details private. The " +
        "owner may suspend or ban accounts for abuse, cheating, or violations of these terms.</p>",
    ) +
    section(
      "Content and availability",
      "<p>JackiOh is provided as-is and may change, pause, or stop at any time. Patch notes and game " +
        "updates can change cards, mechanics, and progression.</p>",
    ),
  [paths.accessibility]:
    section(
      "Our goal",
      "<p>We want JackiOh to be usable by as many players as we can make it. It has not had a formal " +
        "accessibility audit, and some parts are not yet fully accessible.</p>",
    ) +
    section(
      "Current support",
      "<ul><li>Menus, the deck builder, and the board&rsquo;s cards, zones and prompts can be used " +
        "from the keyboard.</li>" +
        "<li>Focused controls are marked, and each screen has headings.</li>" +
        "<li>Card art is decorative and hidden from screen readers; each card&rsquo;s name and text are " +
        "on the card as text.</li>" +
        "<li>Reduce motion, effects and sound can be changed in Settings.</li></ul>",
    ),
};

/** Files Vite copies from `public/` before `writeBundle` that name `SITE_ORIGIN`. */
export const ADDRESSED_PUBLIC_FILES = ["robots.txt", ".well-known/security.txt"] as const;

const STATIC_HEAD =
  "<style>.static-body{display:none}</style><noscript><style>.static-body{display:block}</style></noscript>";
const ROOT_OPEN = '<div id="root">';

const escapeHtml = (text: string): string => text.replace(/[&<>"]/gu, (c) => `&#${String(c.charCodeAt(0))};`);

function nameOf(path: string): string {
  const name = screenNameFor(path);
  if (name === null) throw new Error(`static-pages.ts: ${path} is not a screen the client serves`);
  return name === "" ? SITE_NAME : name;
}

function canonicalOf(path: string, origin: string): string {
  const url = canonicalUrlFor(path, origin);
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

function pageHtml(
  template: string,
  page: PublicPage,
  pages: readonly PublicPage[],
  extra: string,
  origin: string,
): string {
  const title = escapeHtml(documentTitleFor(page.path));
  const description = escapeHtml(page.description);
  const url = canonicalOf(page.path, origin);
  let html = swap(template.replaceAll(SITE_ORIGIN, origin), /(<title>)[^<]*/u, title);
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
      ? `${html.slice(open, close)}<div class="static-body"><p>${description}</p>${nav}\n${extra}</div>`
      : `<div class="static-body"><h1>${escapeHtml(nameOf(page.path))}</h1><p>${description}</p>${nav}\n${extra}</div>`;
  return html.slice(0, open) + body + html.slice(close);
}

export function sitemapXml(pages: readonly PublicPage[], origin: string = SITE_ORIGIN): string {
  const urls = pages.map((page) => `  <url><loc>${canonicalOf(page.path, origin)}</loc></url>\n`).join("");
  return `<?xml version="1.0" encoding="UTF-8"?>\n<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">\n${urls}</urlset>\n`;
}

export type StaticPagesInput = {
  template: string;
  pages: readonly PublicPage[];
  cards: CardDefs;
  patches: readonly Patch[];
  outDir: string;
  origin: string;
};

export function writeStaticPages({ template, pages, cards, patches, outDir, origin }: StaticPagesInput): void {
  for (const page of pages) {
    const extra =
      page.path === paths.almanac
        ? almanacList(cards)
        : page.path === paths.patchNotes
          ? patchList(patches)
          : (PAGE_TEXT[page.path] ?? "");
    const file = join(outDir, page.path, "index.html");
    mkdirSync(dirname(file), { recursive: true });
    writeFileSync(file, pageHtml(template, page, pages, extra, origin));
  }
  writeFileSync(join(outDir, "sitemap.xml"), sitemapXml(pages, origin));
  for (const file of ADDRESSED_PUBLIC_FILES) {
    const text = readFileSync(join(outDir, file), "utf8");
    if (!text.includes(SITE_ORIGIN)) throw new Error(`static-pages.ts: ${file} does not name ${SITE_ORIGIN}`);
    writeFileSync(join(outDir, file), text.replaceAll(SITE_ORIGIN, origin));
  }
}

export function staticPages(): Plugin {
  let root = "";
  let outDir = "";
  let origin = SITE_ORIGIN;
  return {
    name: "jackioh:static-pages",
    apply: "build",
    configResolved(config) {
      root = config.root;
      outDir = resolve(config.root, config.build.outDir);
      // `config.env` is Vite's loadEnv: a variable the build's process sets (staging's, vercel.json)
      // wins over `.env.production`'s.
      const configured: unknown = config.env.VITE_SITE_ORIGIN;
      origin = originOf(configured);
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
        origin,
      });
    },
  };
}
