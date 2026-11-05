/// <reference types="vite/client" />
// Static pages give crawlers page-specific HTML and are replaced at client startup without hydration.
// Reuse `net/head.ts`, `almanacShelf` (R674, R1420; no unshipped cards), `fillParams`, and `patches.json`.
// This Vite entry uses relative imports because Vitest and Cypress do not resolve `@jackioh/*` here.
// `<style>` hides static content until startup; `<noscript>` restores it without script.
// #3127, R3127000: the build also writes `404.html` from the same template, the page Cloudflare serves with a 404
// status for a path nothing routes. It loads the client, which draws its own `NotFound`; its body is a script-free
// fallback in the app's look (the tavern board, the stacked logo, three placeholder card faces), visible from the
// first byte (no `.static-body`) and replaced when the client starts.
// Every address names the build's own host (`StaticPagesInput.origin`, #696), and so does the CSP the
// build writes into `_headers` (`connectSources`).

import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";

import type { Plugin } from "vite";

import { shellTestid } from "./src/auth/testids.ts";
import { motifFor } from "./src/cards/art/motifs.ts";
import { proceduralArtUri } from "./src/cards/art/svg.ts";
import { compositionFor, themeFor } from "./src/cards/art/themes.ts";
import { almanacShelf } from "./src/game/deckbuilder/shelf.ts";
import { SITE_NAME, canonicalUrlFor, documentTitleFor, screenNameFor } from "./src/net/head.ts";
import { SITE_ORIGIN, originOf, paths } from "./src/net/navigate.ts";
import { RELEASE_ART_DIR, releaseArtAsset, releaseArtAssets } from "./src/patches/releaseArt.ts";
import { NOTE_SECTIONS, RELEASES, playerTitle, releaseGroups, technicalLines } from "./src/patches/releases.ts";
import type { Patch, Snapshot, TechnicalEntry } from "./src/patches/source.ts";
import { SHIPPED_SETS, fillParams } from "./src/wire/catalog.ts";
import { DECK_SIZE, HERO_HEALTH, MAX_MANA } from "./src/wire/engineConfig.ts";
import { MIN_ACCOUNT_AGE_YEARS } from "./src/wire/serverConfig.ts";
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
      "JackiOh's privacy policy: what the site collects, why, who sees it, and how to delete your account.",
  },
  { path: paths.terms, description: "The terms of playing JackiOh." },
  {
    path: paths.accessibility,
    description: "JackiOh's accessibility statement, and how to tell us about a barrier.",
  },
  {
    path: paths.ai,
    description: "How JackiOh uses AI: its computer opponents, what was made with AI, and what AI does with your data.",
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

const section = (heading: string, body: string): string => `<section><h2>${heading}</h2>${body}</section>`;

/**
 * Each page's own text, after its description, for a reader that runs no script. /privacy, /terms,
 * /accessibility and /copyright quote their routes word for word (static-pages.test.ts renders each to check).
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
      "What we collect",
      `<p>Your email and username, confirmation that you&rsquo;re ${String(MIN_ACCOUNT_AGE_YEARS)} or older, your ` +
        "decks, cards, matches and how you play, your friends, chats and shared replays, and technical info " +
        "like your IP address and browser.</p>",
    ) +
    section(
      "Why",
      "<p>To run your account and online games, rank players, balance cards, improve our AI opponents and " +
        "prevent abuse.</p>",
    ) +
    section(
      "Who sees it",
      "<p>Our hosting and sign-in providers process it for us. Other players see your username, rank and " +
        "moves. Your friends, and your opponent&rsquo;s friends, can watch your online games, remove or block " +
        "someone to stop that. Your player stats are public.</p>" +
        "<p>We don&rsquo;t sell your data, show ads or use third-party trackers. We don&rsquo;t track you " +
        "across sites, so a browser&rsquo;s &ldquo;Do Not Track&rdquo; setting changes nothing here.</p>",
    ) +
    section("Your browser", "<p>We store your settings and sign-in in your browser.</p>") +
    section(
      "How long",
      "<p>We keep your data while you have an account. Card statistics that don&rsquo;t identify you may be " +
        "kept indefinitely.</p>",
    ) +
    section(
      "Your choices",
      "<p>Delete your account at any time from the Account page. Email us for a copy of your data.</p>",
    ) +
    section(
      "Children",
      `<p>You must be ${String(MIN_ACCOUNT_AGE_YEARS)} or older to have an account. We delete any account we ` +
        "learn belongs to someone younger.</p>",
    ) +
    section("Changes", "<p>When this policy changes, we&rsquo;ll update the date and ask you to agree again.</p>"),
  [paths.terms]:
    "<p>By creating an account, you agree to these Terms and the Privacy Policy. You must be " +
    `${String(MIN_ACCOUNT_AGE_YEARS)} or older.</p>` +
    section(
      "Play fair",
      "<p>Don&rsquo;t cheat, exploit bugs, use bots, or try to break the service or get into anything that " +
        "isn&rsquo;t yours. Don&rsquo;t harass, threaten, spam or share anyone&rsquo;s personal info. We may " +
        "moderate content and suspend or ban accounts that break these rules.</p>",
    ) +
    section(
      "Your content",
      "<p>You keep the rights to what you create or post. You let us store it and show it to the players " +
        "it&rsquo;s meant for. Don&rsquo;t post what you don&rsquo;t have the rights to. We remove infringing " +
        "content and ban repeat infringers (see Copyright).</p>",
    ) +
    section(
      "AI opponents",
      "<p>When no one near your rank is available, ranked may match you against an AI opponent. Nothing in " +
        "the game marks which opponents are AI.</p>",
    ) +
    section(
      "No guarantees",
      "<p>JackiOh is free and provided &ldquo;as is.&rdquo; We may change or end it at any time. As far as " +
        "the law allows, we aren&rsquo;t liable for damages from using it.</p>",
    ) +
    section("Changes", "<p>When these Terms change, we&rsquo;ll ask you to agree again.</p>"),
  [paths.copyright]:
    section(
      "Reporting content that infringes your copyright",
      "<p>If you believe something on JackiOh infringes a copyright you own or act for, such as a username, " +
        "send a written notice to our designated agent below. Under the Digital Millennium Copyright Act " +
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
  [paths.ai]:
    section(
      "The computer opponents",
      "<p>JackiOh has three kinds of computer opponent. Each plays by the same rules you do and sees only what a " +
        "player in its seat could see.</p>" +
        "<ul><li>In Practice, at the difficulty you pick, and in the tutorial, an opponent that runs in your browser.</li>" +
        "<li>In ranked, when no one near your rating is free after a short wait, an AI opponent that our server " +
        "plays. Nothing in the game marks it, as the Terms say.</li>" +
        "<li>In co-op, the team you and a friend play against.</li></ul>" +
        "<p>They choose their moves by searching the moves the rules allow, with a game-playing program we wrote " +
        "and tune on games it plays against itself. They are not chatbots, and they say nothing to you but the " +
        "emotes a player can send.</p>",
    ) +
    section(
      "Made with AI",
      "<ul><li>The music, every track and the few bars each Legendary and Mythic card plays, was composed by an AI " +
        "model, Anthropic&rsquo;s Claude, written as code and played with free instrument sounds. No melody or " +
        "recording was taken from another work. Where each track came from.</li>" +
        "<li>The cards&rsquo; spoken lines are read by computer text-to-speech voices, not recorded by actors.</li>" +
        "<li>Card art is drawn by code from each card&rsquo;s name, type and tags, not by an image-generation model.</li>" +
        "<li>JackiOh&rsquo;s code is written with the help of AI coding assistants.</li></ul>",
    ) +
    section(
      "What AI does with your data",
      "<p>We don&rsquo;t send anything you write, or your account&rsquo;s details, to an AI service. Records of " +
        "online games are used as the Privacy Policy says: for the card statistics, to time the ranked AI " +
        "opponents&rsquo; moves and habits like a person&rsquo;s, and to choose the decks they bring from decks " +
        "players have taken into ranked.</p>",
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
      "<p>On the date above we checked every screen with axe-core 4.14 in Chrome, at a computer's " +
        "size and at a phone's, against its WCAG 2.0, 2.1 and 2.2 rules at levels A and AA, and fixed " +
        "every serious or critical problem it found, except " +
        "those listed below. The same check runs every day, and fails when a new one appears.</p>" +
        "<ul><li>The home page, signing in, creating an account, resetting a password, the invite code " +
        "screen and the screen that asks for your date of birth and agreement to the terms.</li>" +
        "<li>Play, Decks, Account and the leaderboard.</li>" +
        "<li>The Almanac, Patch notes and Stats.</li>" +
        "<li>Practice, its setup and a game, and a game on one device.</li>" +
        "<li>Replays, Play together and the Collection.</li>" +
        "<li>The Privacy Policy, the Terms, this page and the Copyright page.</li></ul>",
    ) +
    section(
      "Current support",
      "<ul><li>Menus, the deck builder, and the board's cards, zones and prompts can be used " +
        "from the keyboard.</li>" +
        "<li>Focused controls are marked, and each screen has headings.</li>" +
        "<li>Card art is decorative and hidden from screen readers; each card's name and text are " +
        "on the card as text.</li>" +
        "<li>Reduce motion, effects and sound can be changed in Settings.</li></ul>",
    ) +
    section(
      "Known gaps",
      "<ul><li>A card shown as a button, on the home page, in the Almanac's card pool and in a game's " +
        "prompts, is named by a short summary that names the card, not by all the text on its face.</li>" +
        "<li>Dragging a card to play it needs a mouse or a touch screen. Choosing the card and then where " +
        "it goes does the same, by click or by Enter on each.</li>" +
        "<li>With a mouse, the card names in the game log are closer together than 24 pixels; on a touch screen each line is 44 pixels tall.</li>" +
        "<li>Color contrast is checked only where the tool can measure it, so text over card art and " +
        "gradients has not been checked.</li>" +
        "<li>No one has tested JackiOh with a screen reader or other assistive technology yet.</li></ul>",
    ),
};

/** Files Vite copies from `public/` before `writeBundle` that name `SITE_ORIGIN`. */
export const ADDRESSED_PUBLIC_FILES = ["robots.txt", ".well-known/security.txt"] as const;

/** The headers Cloudflare sends with the build's files (wrangler.jsonc `assets`), copied from `public/`. */
export const HEADERS_FILE = "_headers";

/** What `public/_headers` spells in place of its CSP's `connect-src` sources, which differ by build. */
export const CONNECT_SRC_PLACEHOLDER = "%CONNECT_SRC%";

/** R2983000: Google Identity Services' requests (Google's own sign-in button), as the CSP names them. */
export const GOOGLE_IDENTITY_SOURCE = "https://accounts.google.com/gsi/";

/** Any `%NAME%` left in a written file: a placeholder the build did not fill. */
const PLACEHOLDER = /%[A-Z][A-Z0-9_]*%/u;

/**
 * The CSP's `connect-src` for a build served from `origin`: 'self', where the server answers
 * `/api/*` (wrangler.jsonc `run_worker_first`), Supabase's origin and Google Identity Services'
 * (R2983000), which a build without `VITE_SUPABASE_URL` never signs in through, and `wss://` on the
 * build's own host for the sockets.
 */
export function connectSources(origin: string, supabaseUrl: string | undefined): string {
  const sources = ["'self'"];
  if (supabaseUrl !== undefined && supabaseUrl.length > 0) {
    sources.push(new URL(supabaseUrl).origin);
    sources.push(GOOGLE_IDENTITY_SOURCE);
  }
  sources.push(`wss://${new URL(origin).host}`);
  return sources.join(" ");
}

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

/** `html` without what `pattern` matches; like `swap`, a template that lacks it is an error. */
function strip(html: string, pattern: RegExp): string {
  if (!pattern.test(html)) throw new Error(`static-pages.ts: index.html has no match for ${String(pattern)}`);
  return html.replace(pattern, "");
}

/** The page Cloudflare serves, with a 404 status, for a path no rule routes (`wrangler.jsonc` `not_found_handling`). */
export const NOT_FOUND_FILE = "404.html";

/**
 * #3127, R3127000: the fallback's hand, left to right. The gems read 4, 0, 4. The ids are in no catalog: these are
 * placeholder Spells with no set, rarity, attack, health or rules text, nothing a player could take for a real card.
 */
export const NOT_FOUND_HAND = [
  { id: "not-found-error", name: "Error", cost: 4 },
  { id: "not-found-not", name: "Not", cost: 0 },
  { id: "not-found-found", name: "Found", cost: 4 },
] as const;

export type HandCard = (typeof NOT_FOUND_HAND)[number];

/**
 * One hand card as `cards/CardFace.tsx` draws a Spell that has no card behind it (`faceModel({ defId, name, type:
 * "Spell", radiant: false, liveCost })`, layout "full"): its frame, cost gem, art window, name banner and type line,
 * and an empty rules box. The Vite config loader cannot import CardFace (its `.css` imports, its hooks), so the markup
 * is written here from the same pure art builders `CardArt` calls, and static-pages.test.ts holds it to CardFace's own
 * render, element by element: a changed CardFace fails that test.
 */
export function handCardHtml({ id, name, cost }: HandCard): string {
  const theme = themeFor([], "Spell");
  const motif = motifFor(name, theme);
  const motifAttr = motif === null ? "" : ` data-art-motif="${motif}"`;
  const uri = proceduralArtUri(id, theme, compositionFor("Spell"), false, motif);
  return (
    '<span class="cf cf--full" data-layout="full" data-card-type="Spell" data-name-tier="s" data-text-tier="s" ' +
    'data-foil="none" style="--cf-name-scale:1;--cf-text-scale:1"><span class="cf-scale">' +
    `<span class="cost-gem" data-cost="${String(cost)}" data-tone="base">${String(cost)}</span>` +
    '<span class="cf-art-frame">' +
    `<span class="cf-art cf-art--window" data-art="procedural" data-art-theme="${theme}" data-art-variant="base"${motifAttr} ` +
    `aria-hidden="true" style="background-image:url(&quot;${escapeHtml(uri)}&quot;);background-size:cover"></span></span>` +
    `<span class="card-name">${escapeHtml(name)}</span><span class="card-type">Spell</span>` +
    '<span class="card-text"><span class="cf-text-base"></span></span></span></span>'
  );
}

/** The fallback's two sentences, as the client's 404 panel says them (main.tsx `NotFound`). */
const NOT_FOUND_NOTICE = "That page doesn't exist.";
const NOT_FOUND_HINT = "The link may be wrong or out of date.";

/** The stacked logo and its size, as main.tsx's `SHELL_LOGO` has them (static-pages.test.ts holds the two equal). */
const STATIC_LOGO = { src: "/brand/jackioh-logo-stacked.svg", width: 1536, height: 1306 } as const;

/**
 * #3127, R3127000: what shows with no script, and while the entry script has not started: the shell's tavern board
 * (main.tsx `ShellPanel`, auth/tavern.css) around the 404 panel's words, the hand of three card faces turning over
 * from the brand's card back (notfound.css `.nf-hand`), and the app's own button home. Its styles are the entry
 * stylesheet's, which the template links. It carries no `static-body` and no `<noscript>`: nothing hides it.
 */
function notFoundBody(): string {
  const slots = NOT_FOUND_HAND.map(
    (card) =>
      '<div class="nf-hand__slot"><div class="nf-hand__card"><div class="nf-hand__side nf-hand__side--back"></div>' +
      `<div class="nf-hand__side nf-hand__side--face">${handCardHtml(card)}</div></div></div>`,
  ).join("");
  return (
    `<div class="app-shell tavern shell-panel" data-testid="${shellTestid.notFoundStatic}" data-shown="true">` +
    `<h1 class="shell-panel__brand"><img class="shell-panel__logo" src="${STATIC_LOGO.src}" width="${String(STATIC_LOGO.width)}" ` +
    `height="${String(STATIC_LOGO.height)}" alt="${SITE_NAME}" /></h1>` +
    '<section class="panel panel--auth">' +
    `<p class="notice">${NOT_FOUND_NOTICE}</p>` +
    `<p class="auth-hint">${NOT_FOUND_HINT}</p>` +
    `<div class="nf-hand" aria-hidden="true">${slots}</div>` +
    '<div class="row"><a class="button-secondary" href="/">Back to the start</a></div>' +
    "</section></div>"
  );
}

const NOT_FOUND_DESCRIPTION = `${NOT_FOUND_NOTICE} ${NOT_FOUND_HINT}`;

/**
 * #3127, R3127000: `404.html`, from the built `index.html`. The client starts from it and, at the path it was
 * served for, draws its own 404 (`NotFound`); Cloudflare still answers with a 404 status. Its head names the 404 as
 * the client does (`documentTitleFor`), asks not to be indexed and names no address of its own: no canonical link,
 * no `og:url`, no structured data (they name the landing page).
 */
export function notFoundHtml(template: string, origin: string): string {
  const title = escapeHtml(documentTitleFor(`/${NOT_FOUND_FILE}`));
  const description = escapeHtml(NOT_FOUND_DESCRIPTION);
  let html = swap(template.replaceAll(SITE_ORIGIN, origin), /(<title>)[^<]*/u, title);
  html = swap(html, /(<meta name="description" content=")[^"]*/u, description);
  html = swap(html, /(<meta property="og:title" content=")[^"]*/u, title);
  html = swap(html, /(<meta property="og:description" content=")[^"]*/u, description);
  html = strip(html, /<meta property="og:url"[^>]*>\s*/u);
  html = strip(html, /<script type="application\/ld\+json">[\s\S]*?<\/script>\s*/u);
  html = swap(html, /()<\/head>/u, '<meta name="robots" content="noindex" /></head>');

  const open = html.indexOf(ROOT_OPEN) + ROOT_OPEN.length;
  const close = html.lastIndexOf("</div>");
  if (open < ROOT_OPEN.length || close < open) throw new Error("static-pages.ts: index.html has no #root");
  return html.slice(0, open) + notFoundBody() + html.slice(close);
}

/** A price in R432's words, as the patch diff writes it (patches/diff.ts): "(2) Cost, Embiggened (4)" for an embiggen card (R1175000). */
const costText = (cost: CardCost): string =>
  typeof cost === "object" ? `(${String(cost.base)}) Cost, Embiggened (${String(cost.embiggen)})` : `(${String(cost)}) Cost`;

function almanacList(cards: CardDefs): string {
  const entries: string[] = [];
  for (const id of almanacShelf(cards)) {
    const def = cards[id];
    if (def === undefined) continue;
    const facts = `${def.set} · ${def.type} · ${def.rarity} · ${costText(def.cost)}`;
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

/**
 * R724000: every patch by release line, newest first, each with its player note (R724001) and its
 * technical log's player lines (R1369000), never its record's notes.
 */
function patchList(patches: readonly Patch[], log: readonly TechnicalEntry[]): string {
  const technical = technicalLines(log);
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
        const lines = technical.get(patch.version) ?? [];
        return (
          `<article><h3>${escapeHtml(`${patch.version} ${playerTitle(patch)}`)}</h3>` +
          `<p><time datetime="${escapeHtml(patch.date)}">${escapeHtml(patch.date)}</time></p>` +
          sections +
          (lines.length === 0 ? "" : `<h4>Technical changes</h4>${items(lines)}`) +
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
  technical?: readonly TechnicalEntry[];
  /** #2681, R3012001: one version's catalog snapshot; with it, the release artwork of every major and patch (and of the micros that shipped with art) is written beside the pages. */
  snapshot?: (version: string) => Snapshot | null;
  outDir: string;
  origin: string;
  /** The build's `VITE_SUPABASE_URL`, for its CSP (`connectSources`); unset, the CSP names no Supabase host. */
  supabaseUrl?: string | undefined;
};

/** Where the release artwork is written under the output directory (releaseArt.ts's `RELEASE_ART_DIR`, without its slash). */
const RELEASE_ART_FOLDER = "release-art";

export function writeStaticPages({
  template,
  pages,
  cards,
  patches,
  technical = [],
  snapshot,
  outDir,
  origin,
  supabaseUrl,
}: StaticPagesInput): void {
  for (const page of pages) {
    const extra =
      page.path === paths.almanac
        ? almanacList(cards)
        : page.path === paths.patchNotes
          ? patchList(patches, technical)
          : (PAGE_TEXT[page.path] ?? "");
    const file = join(outDir, page.path, "index.html");
    mkdirSync(dirname(file), { recursive: true });
    writeFileSync(file, pageHtml(template, page, pages, extra, origin));
  }
  writeFileSync(join(outDir, NOT_FOUND_FILE), notFoundHtml(template, origin));
  writeFileSync(join(outDir, "sitemap.xml"), sitemapXml(pages, origin));
  if (snapshot !== undefined) {
    mkdirSync(join(outDir, RELEASE_ART_FOLDER), { recursive: true });
    for (const { path, svg } of releaseArtAssets(patches, snapshot)) writeFileSync(join(outDir, decodeURIComponent(path)), svg);
  }
  for (const file of ADDRESSED_PUBLIC_FILES) {
    const text = readFileSync(join(outDir, file), "utf8");
    if (!text.includes(SITE_ORIGIN)) throw new Error(`static-pages.ts: ${file} does not name ${SITE_ORIGIN}`);
    writeFileSync(join(outDir, file), text.replaceAll(SITE_ORIGIN, origin));
  }
  const headers = readFileSync(join(outDir, HEADERS_FILE), "utf8");
  if (!headers.includes(CONNECT_SRC_PLACEHOLDER)) {
    throw new Error(`static-pages.ts: ${HEADERS_FILE} does not spell ${CONNECT_SRC_PLACEHOLDER}`);
  }
  const filled = headers.replaceAll(CONNECT_SRC_PLACEHOLDER, connectSources(origin, supabaseUrl));
  const left = PLACEHOLDER.exec(filled)?.[0];
  if (left !== undefined) throw new Error(`static-pages.ts: ${HEADERS_FILE} still holds ${left}, which nothing fills`);
  writeFileSync(join(outDir, HEADERS_FILE), filled);
}

export function staticPages(): Plugin {
  let root = "";
  let outDir = "";
  let origin = SITE_ORIGIN;
  let supabaseUrl: string | undefined;
  // Reading here avoids bundling the catalog into each config load.
  const data = (file: string): unknown =>
    JSON.parse(readFileSync(resolve(root, "../../crates/cards", file), "utf8"));
  const snapshot = (version: string): Snapshot | null => {
    const file = resolve(root, "../../crates/cards/patches", `${version}.json`);
    return existsSync(file) ? (JSON.parse(readFileSync(file, "utf8")) as Snapshot) : null;
  };
  return {
    name: "jackioh:static-pages",
    configResolved(config) {
      root = config.root;
      outDir = resolve(config.root, config.build.outDir);
      // `config.env` is Vite's loadEnv: a variable the build's process sets (staging's, from
      // .env.staging by `build:staging`) wins over `.env.production`'s.
      const configured: unknown = config.env.VITE_SITE_ORIGIN;
      origin = originOf(configured);
      const supabase: unknown = config.env.VITE_SUPABASE_URL;
      supabaseUrl = typeof supabase === "string" ? supabase : undefined;
    },
    // #2681: the dev server draws a patch's artwork when asked for it, as the build writes it, and
    // answers no other path: nothing is written into the source tree.
    configureServer(server) {
      server.middlewares.use((request, response, next) => {
        const path = (request.url ?? "").split("?")[0] ?? "";
        try {
          const asset = path.startsWith(RELEASE_ART_DIR)
            ? releaseArtAsset(path, data("patches/patches.json") as Patch[], snapshot)
            : null;
          if (asset === null) {
            next();
            return;
          }
          response.setHeader("Content-Type", "image/svg+xml");
          response.end(asset.svg);
        } catch (error) {
          next(error);
        }
      });
    },
    // Only a build writes pages: `serve` never runs this hook.
    writeBundle() {
      writeStaticPages({
        template: readFileSync(join(outDir, "index.html"), "utf8"),
        pages: PUBLIC_PAGES,
        cards: data("catalog.json") as CardDefs,
        patches: data("patches/patches.json") as Patch[],
        technical: data("patches/technical.json") as TechnicalEntry[],
        snapshot,
        outDir,
        origin,
        supabaseUrl,
      });
    },
  };
}
