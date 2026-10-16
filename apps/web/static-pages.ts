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
import { NOTE_SECTIONS, RELEASES, playerTitle, releaseGroups } from "./src/patches/releases.ts";
import type { Patch } from "./src/patches/source.ts";
import { SHIPPED_SETS, fillParams } from "./src/wire/catalog.ts";
import { DECK_SIZE, HERO_HEALTH, MAX_MANA } from "./src/wire/engineConfig.ts";
import { AUTH_PENDING_ADDRESS_TTL_SECONDS, MIN_ACCOUNT_AGE_YEARS } from "./src/wire/serverConfig.ts";
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
    path: paths.copyright,
    description: "How to report content on JackiOh that infringes your copyright, and how to answer a report.",
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

/** Unit conversion, not configuration. */
const SECONDS_PER_HOUR = 3600;

/** A section of a page's static text: its heading and its paragraphs or list, as HTML. */
const section = (heading: string, body: string): string => `<section><h2>${heading}</h2>${body}</section>`;

/**
 * Each page's own text, after its description, for a reader that runs no script. /privacy, /terms,
 * /accessibility and /copyright quote their routes word for word (static-pages.test.ts renders each to check);
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
        "in the tutorial. Create an account and redeem an invite code to build decks and play friends online. " +
        "Look up any card in the almanac, and read what each patch changed in the patch notes.</p>",
    ),
  [paths.practice]:
    section(
      "A game against the AI",
      "<p>A practice game runs in your browser, and its moves are never sent to a server, so no account " +
        "is needed. Pick a difficulty and a deck, a random one or a preset deck, or one of your saved " +
        "decks once your account is active, and play.</p>",
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
        "<li>While a new account waits for its email to be confirmed: the month and year of birth you gave " +
        "and the version of the terms you agreed to, until your first sign-in on this browser sends them " +
        `to us, for ${String(AUTH_PENDING_ADDRESS_TTL_SECONDS / SECONDS_PER_HOUR)} hours at most.</li>` +
        "<li>If this browser was refused an account: a note saying so, with no age or date in it.</li>" +
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
        "custom cards, your tutorial progress, the records of how you played, and the record of your age " +
        "check and of the terms you agreed to. Finished matches and their results stay in " +
        "the other player&rsquo;s history, with your account removed from them. Co-op games are different: " +
        "deleting either teammate&rsquo;s account deletes the whole co-op game, its moves and its result, for " +
        "both of you. You can&rsquo;t delete your account during a match, a co-op game or a Conquest series " +
        "that hasn&rsquo;t finished.</p>" +
        "<p>Deleting your account also deletes everything of yours on Play together: your friends, friend " +
        "requests, blocks, challenges, co-op rooms and replay shares, and your chats, the messages your " +
        "friends sent you included.</p>" +
        "<p>To ask for a copy of your data, email us. Please don&rsquo;t put your email address or " +
        "other personal details in a public GitHub issue.</p>",
    ),
  [paths.terms]:
    section(
      "Agreeing to these terms",
      "<p>You agree to these terms and to the Privacy Policy when you create an account, by ticking the box " +
        "that says so. When either of them changes, we ask you to agree again before you go on, and keep a " +
        "record of the version you agreed to and when.</p>",
    ) +
    section(
      "Who can have an account",
      `<p>You must be at least ${String(MIN_ACCOUNT_AGE_YEARS)} years old to have an account. Anyone can play ` +
        "against the AI in Practice without one.</p>",
    ) +
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
    ) +
    section(
      "What you submit",
      "<p>You keep whatever rights you have in what you submit. By submitting it, you let us store it, and " +
        "show it to the people named below, for as long as we keep it (the Privacy Policy says how long):</p>" +
        "<ul><li>Your username, which other players see.</li>" +
        "<li>Your chat messages, which the friend you send them to sees, and the reason you give when you " +
        "report one, which only we see.</li>" +
        "<li>Your custom cards (their names, text and scripts) and the names of your decks, which only you see.</li>" +
        "<li>The replays you share, which the friends you share them with see.</li></ul>" +
        "<p>Don&rsquo;t submit anything that infringes someone else&rsquo;s rights. We remove content that " +
        "infringes, and close the accounts of players who repeatedly infringe; see Copyright for how to report it.</p>",
    ) +
    section(
      "Chat",
      "<p>Be respectful in chat. Don&rsquo;t harass, threaten or bully anyone, use hate speech, send spam, " +
        "or share anyone&rsquo;s personal details, such as an address, a phone number or an email address, " +
        "your own included.</p>" +
        "<p>Before a message is stored, we replace each word on our filter list with asterisks. You can report " +
        "any message a friend sends you. We review reports, and abuse in chat can get an account suspended or " +
        "banned.</p>",
    ) +
    section("Ranked play", "<p>When no suitable opponent is available, ranked may pair you with an AI opponent.</p>"),
  [paths.copyright]:
    section(
      "Reporting content that infringes your copyright",
      "<p>If you believe something on JackiOh infringes a copyright you own or act for, such as a username or " +
        "a custom card&rsquo;s name or text, send a written notice to our designated agent below. Under the Digital Millennium Copyright Act " +
        "(17 U.S.C. &sect; 512(c)(3)), the notice must include:</p>" +
        "<ul><li>Your physical or electronic signature, as the copyright owner or someone authorised to act for them.</li>" +
        "<li>The copyrighted work you say is infringed.</li>" +
        "<li>The material you say infringes it, with enough detail for us to find it on JackiOh.</li>" +
        "<li>Your name, postal address, telephone number and email address.</li>" +
        "<li>A statement that you believe in good faith that the use is not authorised by the copyright owner, " +
        "its agent or the law.</li>" +
        "<li>A statement that the information in the notice is accurate and, under penalty of perjury, that you " +
        "are the copyright owner or authorised to act for them.</li></ul>",
    ) +
    section(
      "Repeat infringers",
      "<p>We close the accounts of players who repeatedly infringe others&rsquo; copyrights.</p>",
    ),
  [paths.accessibility]:
    section(
      "Our goal",
      "<p>We want JackiOh to be usable by as many players as we can make it. It has had an automated " +
        "check against the Web Content Accessibility Guidelines (WCAG) 2.2 at levels A and AA, and no " +
        "audit by a person. We do not claim that it conforms to WCAG.</p>",
    ) +
    section(
      "What we checked",
      "<p>On the date above we checked every screen with axe-core 4.14 in Chrome, against its WCAG 2.0, " +
        "2.1 and 2.2 rules at levels A and AA, and fixed every serious or critical problem it found, except " +
        "those listed below. The same check runs every day, and fails when a new one appears.</p>" +
        "<ul><li>The home page, signing in, creating an account, resetting a password, the invite code " +
        "screen and the screen that asks for your date of birth and agreement to the terms.</li>" +
        "<li>Play, Decks, Account and the leaderboard.</li>" +
        "<li>The Almanac, Patch notes and Stats.</li>" +
        "<li>Practice, its setup and a game, and a game on one device.</li>" +
        "<li>The Privacy Policy, the Terms, this page and the Copyright page.</li></ul>",
    ) +
    section(
      "Current support",
      "<ul><li>Menus, the deck builder, and the board&rsquo;s cards, zones and prompts can be used " +
        "from the keyboard.</li>" +
        "<li>Focused controls are marked, and each screen has headings.</li>" +
        "<li>Card art is decorative and hidden from screen readers; each card&rsquo;s name and text are " +
        "on the card as text.</li>" +
        "<li>Reduce motion, effects and sound can be changed in Settings.</li></ul>",
    ) +
    section(
      "Known gaps",
      "<ul><li>A card shown as a button, on the home page, in the Almanac&rsquo;s card pool and in a game&rsquo;s " +
        "prompts, is named by a short summary that names the card, not by all the text on its face.</li>" +
        "<li>Dragging a card to play it needs a mouse or a touch screen. Choosing the card and then where " +
        "it goes does the same, by click or by Enter on each.</li>" +
        "<li>With a mouse, the card names in the game log are closer together than 24 pixels; on a touch screen each line is 44 pixels tall.</li>" +
        "<li>Colour contrast is checked only where the tool can measure it, so text over card art and " +
        "gradients has not been checked.</li>" +
        "<li>No one has tested JackiOh with a screen reader or other assistive technology yet.</li></ul>",
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

const items = (lines: readonly string[]): string => `<ul>${lines.map((line) => `<li>${escapeHtml(line)}</li>`).join("")}</ul>`;

/** R724000: every patch by release line, newest first, each with its player note (R724001), never its record's notes. */
function patchList(patches: readonly Patch[]): string {
  return releaseGroups(patches, RELEASES)
    .map(({ release, patches: group }) => {
      const entries = group.map((patch) => {
        const note = patch.player;
        const sections =
          note === undefined
            ? ""
            : NOTE_SECTIONS.filter(({ key }) => note[key].length > 0)
                .map(({ key, heading }) => `<h4>${escapeHtml(heading)}</h4>${items(note[key])}`)
                .join("");
        return (
          `<article><h3>${escapeHtml(`${patch.version} ${playerTitle(patch)}`)}</h3>` +
          `<p><time datetime="${escapeHtml(patch.date)}">${escapeHtml(patch.date)}</time></p>` +
          sections +
          "</article>"
        );
      });
      return (
        `<section><h2>${escapeHtml(release.name)}</h2>` +
        (release.summary === "" ? "" : `<p>${escapeHtml(release.summary)}</p>`) +
        (release.technical.length === 0 ? "" : `<h3>Technical changes</h3>${items(release.technical)}`) +
        entries.join("\n") +
        "</section>"
      );
    })
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
