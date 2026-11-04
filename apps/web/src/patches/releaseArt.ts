// #2681, R3012001: the artwork of a release, one 1920×1080 picture per major and per patch, drawn by code
// from the public definitions the patch added, changed or removed, as every card's own art is (CardArt).
// A patch that changed no card is drawn with card backs instead. A major (R1369000) is a poster: its
// line's name large, more headline cards (`RELEASE_ART_MAJOR_CARD_LIMIT`, the rarest first) and a frame;
// a patch has the same style at a patch's scale. Nothing moves: no animation, no style sheet, so the
// page needs no motion setting for it, and a major's `.svg` and its `-still.svg` are the same bytes
// (the second is kept for the links published while the art drifted). A micro patch has no art of its
// own: its notes sit under the patch it follows, so only a micro shipped before
// `RELEASE_ART_LEGACY_MICRO_LAST` keeps the file its published links name (CLAUDE.md rule 13).
//
// A pure function of the record and its snapshots, so every build and every run of
// `scripts/github-releases.ts` draws the same bytes: no clock, no random source, no engine or match
// state (CLAUDE.md rule 7). The module is loaded by Node (static-pages.ts, the releases script), so it
// takes relative imports only and nothing browser-bound. Nothing per version is kept in the tree: the
// next patch has its art when `patches ship` adds it to patches.json (docs/issues-and-patches.md).

import type { CardDef } from "@jackioh/shared";

import { proceduralCardArtUri } from "./cardArt.ts";
import { fmt } from "../cards/art/hash.ts";
import { SITE_NAME } from "../net/head.ts";
import { GLITCH_DEF_ID } from "../wire/engineConfig.ts";
import {
  RELEASE_ART_CARD_LIMIT,
  RELEASE_ART_HEIGHT,
  RELEASE_ART_LEGACY_MICRO_LAST,
  RELEASE_ART_MAJOR_CARD_LIMIT,
  RELEASE_ART_MAJOR_CARD_SCALE,
  RELEASE_ART_WIDTH,
} from "./constants.ts";
import { RELEASES, playerTitle, releaseGroups, tiers, type Release } from "./releases.ts";
import type { Patch, Snapshot } from "./source.ts";

/** R3012001: a major's poster, or a patch's art: the same style at two scales. */
export type ReleaseArtKind = "major" | "patch";

/** `main` is the version's own file; `still` is a major's `-still` companion, the same bytes (links published while the art moved name it). */
export type ReleaseArtFile = "main" | "still";

/** One file of artwork: where it is served, and its SVG. */
export type ReleaseArtAsset = { readonly path: string; readonly svg: string };

/** What one patch's artwork is drawn from: `before` is the snapshot of the patch ahead of it, null for the first. */
export type ReleaseArtInput = {
  readonly patch: Patch;
  readonly release: Release;
  /** R3012001: a major's poster, or a patch's art. */
  readonly kind: ReleaseArtKind;
  readonly before: Snapshot | null;
  readonly after: Snapshot;
};

/** Where the artwork is served from, on both hosts: `/release-art/<version>.svg`, and `…-still.svg` for a major's companion. */
export const RELEASE_ART_DIR = "/release-art/";
const NORMAL_SUFFIX = ".svg";
const STILL_SUFFIX = "-still.svg";

// ---- the picture: a tavern at night, its palette the Patch notes page's (auth/tavern.css) ----------

const INK = "#0e0905";
const WOOD = "#341e10";
const BRASS = "#c9953f";
const GOLD = "#f3c969";
const GOLD_PALE = "#fbe7b0";
const PARCHMENT = "#efe0bf";
const TEAL = "#3fb6a8";
const SERIF = "Georgia, 'Times New Roman', serif";

const MARGIN_X = 120;
const CENTER_X = RELEASE_ART_WIDTH / 2;

/** Where the head of a picture sits and how big it is: the line's name, the version, the player title, the site's mark, the date and a major's stat line. */
type ArtLayout = {
  readonly lineY: number;
  readonly lineFontSize: number;
  readonly lineLetterSpacing: number;
  readonly lineFill: string;
  readonly versionY: number;
  readonly versionFontSize: number;
  readonly titleY: number;
  readonly titleFontSize: number;
  readonly titleLineHeight: number;
  readonly titleLineChars: number;
  readonly markY: number;
  readonly dateY: number;
  /** Where a major's "n cards" line sits, under the date; null for a patch, which has none. */
  readonly statY: number | null;
  /** The fan: where its middle card sits, how fast it drops to its sides, and how big a card is drawn against `CARD_WIDTH`. */
  readonly fanCenterY: number;
  readonly fanDrop: number;
  readonly cardScale: number;
  /** A major's brass frame around the picture. */
  readonly framed: boolean;
};

/** A patch's art: the head runs along the top, the fan under it. */
const PATCH_LAYOUT: ArtLayout = {
  lineY: 170,
  lineFontSize: 34,
  lineLetterSpacing: 8,
  lineFill: TEAL,
  versionY: 320,
  versionFontSize: 160,
  titleY: 410,
  titleFontSize: 60,
  titleLineHeight: 70,
  titleLineChars: 46,
  markY: 170,
  dateY: 220,
  statY: null,
  fanCenterY: 760,
  fanDrop: 5,
  cardScale: 1,
  framed: false,
};

/** A major's poster: the line's name large over a smaller version, the site's mark, date and card count in a column at the right, and a wider fan of smaller cards. */
const MAJOR_LAYOUT: ArtLayout = {
  lineY: 140,
  lineFontSize: 96,
  lineLetterSpacing: 4,
  lineFill: GOLD,
  versionY: 280,
  versionFontSize: 120,
  titleY: 360,
  titleFontSize: 52,
  titleLineHeight: 60,
  titleLineChars: 56,
  markY: 230,
  dateY: 280,
  statY: 325,
  fanCenterY: 790,
  fanDrop: 3,
  cardScale: RELEASE_ART_MAJOR_CARD_SCALE,
  framed: true,
};

const LAYOUTS: Readonly<Record<ReleaseArtKind, ArtLayout>> = { major: MAJOR_LAYOUT, patch: PATCH_LAYOUT };

const TITLE_MAX_LINES = 2;
const MARK_FONT_SIZE = 44;
const DATE_FONT_SIZE = 30;

/** A line's name is drawn in capitals at this many ems a letter, so a long one is made smaller to fit between the margins. */
const LINE_GLYPH_EMS = 0.8;

/** A major's frame: its inset from the picture's edge, its corner and its stroke. */
const FRAME_INSET = 24;
const FRAME_RADIUS = 28;
const FRAME_STROKE = 4;

/** A card: its frame, its art window above and its name plate below, 5:7 as every card is. */
const CARD_WIDTH = 300;
const CARD_HEIGHT = 420;
const CARD_RADIUS = 22;
const CARD_BORDER = 6;
const ART_INSET = 16;
const ART_HEIGHT = 292;
const ART_RADIUS = 12;
const NAME_FONT_SIZE = 20;
const NAME_LINE_HEIGHT = 24;
const NAME_LINE_CHARS = 19;
const NAME_MAX_LINES = 3;

/** The fan: the cards in an arc along the bottom, the middle one on top. It runs nearer the edges than the head does. */
const FAN_MARGIN_X = 60;
const FAN_MAX_SPACING = 330;
const FAN_STEP_DEGREES = 3.2;

/** A patch that changed no card shows this many card backs. */
const BACK_COUNT = 5;
const BACK_INNER_INSET = 18;
const BACK_EMBLEM_RADII: readonly number[] = [84, 56, 28];
const BACK_EMBLEM_DOT = 14;

/** The backdrop: the first card's art, enlarged, over the dark; and the lattice that textures it. */
const BACKDROP_SIZE = RELEASE_ART_WIDTH;
const BACKDROP_OPACITY = 0.3;
const LATTICE_CELL = 96;
const LATTICE_OPACITY = 0.08;
const GLOW_RADIUS = 760;

const escapeXml = (text: string): string =>
  text.replace(/[&<>"']/gu, (char) => `&#${String(char.codePointAt(0))};`);

/** Words to lines of at most `chars` characters, at most `maxLines`; what does not fit ends the last line with "…". */
function wrap(text: string, chars: number, maxLines: number): string[] {
  const fit = (line: string): string => ([...line].length > chars ? `${[...line].slice(0, chars - 1).join("")}…` : line);
  const lines: string[] = [];
  let line = "";
  for (const word of text.split(/\s+/u).filter((each) => each !== "")) {
    const next = line === "" ? word : `${line} ${word}`;
    if ([...next].length <= chars) {
      line = next;
    } else {
      if (line !== "") lines.push(line);
      line = word;
    }
  }
  if (line !== "") lines.push(line);
  const kept = lines.slice(0, maxLines).map(fit);
  const last = kept.length - 1;
  if (lines.length > maxLines && last >= 0) kept[last] = fit(`${kept[last] ?? ""}…`);
  return kept;
}

function textLines(lines: readonly string[], x: number, y: number, lineHeight: number, attributes: string): string {
  const spans = lines.map((line, at) => `<tspan x="${fmt(x)}" y="${fmt(y + at * lineHeight)}">${escapeXml(line)}</tspan>`).join("");
  return `<text ${attributes}>${spans}</text>`;
}

/** One card of the fan: which it is, where it sits and how far it turns. */
type Place = { readonly at: number; readonly x: number; readonly y: number; readonly angle: number; readonly rank: number };

function fan(count: number, layout: ArtLayout): Place[] {
  const span = RELEASE_ART_WIDTH - 2 * FAN_MARGIN_X - CARD_WIDTH * layout.cardScale;
  const spacing = count < 2 ? 0 : Math.min(FAN_MAX_SPACING * layout.cardScale, span / (count - 1));
  const middle = (count - 1) / 2;
  return Array.from({ length: count }, (_none, at) => {
    const off = at - middle;
    return {
      at,
      x: CENTER_X + off * spacing,
      y: layout.fanCenterY + off * off * layout.fanDrop,
      angle: off * FAN_STEP_DEGREES,
      rank: Math.abs(off),
    };
  });
}

/** The far cards first and the middle one last, so it lies on top; ties keep the left card first. */
function drawOrder(places: readonly Place[]): Place[] {
  return [...places].sort((a, b) => b.rank - a.rank || a.at - b.at);
}

const placed = (place: Place, inner: string, scale: number): string =>
  `<g transform="translate(${fmt(place.x)} ${fmt(place.y)}) rotate(${fmt(place.angle)}) scale(${fmt(scale)}) translate(${-CARD_WIDTH / 2} ${-CARD_HEIGHT / 2})">${inner}</g>`;

function cardMarkup(def: CardDef): string {
  const plateTop = ART_INSET * 2 + ART_HEIGHT;
  const plateHeight = CARD_HEIGHT - plateTop - ART_INSET;
  const lines = wrap(def.name, NAME_LINE_CHARS, NAME_MAX_LINES);
  const firstBaseline = plateTop + (plateHeight - (lines.length - 1) * NAME_LINE_HEIGHT) / 2 + NAME_FONT_SIZE / 3;
  return (
    `<rect width="${CARD_WIDTH}" height="${CARD_HEIGHT}" rx="${CARD_RADIUS}" fill="url(#frame)" stroke="${GOLD}" stroke-width="${CARD_BORDER}"/>` +
    `<image href="${proceduralCardArtUri(def, false)}" x="${ART_INSET}" y="${ART_INSET}" width="${CARD_WIDTH - 2 * ART_INSET}" height="${ART_HEIGHT}" ` +
    `preserveAspectRatio="xMidYMid slice" clip-path="url(#window)"/>` +
    `<rect x="${ART_INSET}" y="${ART_INSET}" width="${CARD_WIDTH - 2 * ART_INSET}" height="${ART_HEIGHT}" rx="${ART_RADIUS}" fill="none" stroke="${BRASS}" stroke-width="3"/>` +
    textLines(
      lines,
      CARD_WIDTH / 2,
      firstBaseline,
      NAME_LINE_HEIGHT,
      `text-anchor="middle" font-family="${SERIF}" font-size="${NAME_FONT_SIZE}" font-weight="700" fill="${PARCHMENT}"`,
    )
  );
}

/** The back of a card, drawn once in `<defs>` and used for each back. */
function backMarkup(): string {
  const mid = CARD_WIDTH / 2;
  const emblem = BACK_EMBLEM_RADII.map((radius, at) =>
    at % 2 === 0
      ? `<circle r="${radius}" fill="none" stroke="${GOLD}" stroke-width="3" opacity="0.8"/>`
      : `<path d="M0 ${-radius}L${radius} 0L0 ${radius}L${-radius} 0Z" fill="none" stroke="${GOLD}" stroke-width="3" opacity="0.8"/>`,
  ).join("");
  return (
    `<g id="back"><rect width="${CARD_WIDTH}" height="${CARD_HEIGHT}" rx="${CARD_RADIUS}" fill="url(#back-fill)" stroke="${GOLD}" stroke-width="${CARD_BORDER}"/>` +
    `<rect x="${BACK_INNER_INSET}" y="${BACK_INNER_INSET}" width="${CARD_WIDTH - 2 * BACK_INNER_INSET}" height="${CARD_HEIGHT - 2 * BACK_INNER_INSET}" rx="${ART_RADIUS}" fill="none" stroke="${BRASS}" stroke-width="3"/>` +
    `<g transform="translate(${mid} ${CARD_HEIGHT / 2})">${emblem}<circle r="${BACK_EMBLEM_DOT}" fill="${GOLD}"/></g></g>`
  );
}

function defsMarkup(): string {
  return (
    "<defs>" +
    `<linearGradient id="sky" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="${WOOD}"/><stop offset="1" stop-color="${INK}"/></linearGradient>` +
    `<radialGradient id="glow" cx="0.5" cy="0.5" r="0.5"><stop offset="0" stop-color="${GOLD}" stop-opacity="0.28"/><stop offset="1" stop-color="${GOLD}" stop-opacity="0"/></radialGradient>` +
    `<linearGradient id="shade" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="${INK}" stop-opacity="0.35"/><stop offset="1" stop-color="${INK}" stop-opacity="0.9"/></linearGradient>` +
    `<linearGradient id="frame" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="${WOOD}"/><stop offset="1" stop-color="${INK}"/></linearGradient>` +
    `<linearGradient id="back-fill" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="${TEAL}" stop-opacity="0.55"/><stop offset="1" stop-color="${WOOD}"/></linearGradient>` +
    `<pattern id="lattice" width="${LATTICE_CELL}" height="${LATTICE_CELL}" patternUnits="userSpaceOnUse">` +
    `<path d="M${LATTICE_CELL / 2} 0L${LATTICE_CELL} ${LATTICE_CELL / 2}L${LATTICE_CELL / 2} ${LATTICE_CELL}L0 ${LATTICE_CELL / 2}Z" fill="none" stroke="${BRASS}" stroke-opacity="${LATTICE_OPACITY}" stroke-width="2"/></pattern>` +
    `<clipPath id="window"><rect x="${ART_INSET}" y="${ART_INSET}" width="${CARD_WIDTH - 2 * ART_INSET}" height="${ART_HEIGHT}" rx="${ART_RADIUS}"/></clipPath>` +
    backMarkup() +
    "</defs>"
  );
}

/**
 * R388, R1369000: every definition a patch's record claims, in its order and each once: each card it
 * added or changed as `after` holds it, each it removed as `before` held it. Glitch is never shown
 * (R674). A card the record claims and its snapshot lacks fails the build rather than drawing another
 * card in its place.
 */
function releaseArtChanged(patch: Patch, after: Snapshot, before: Snapshot | null): CardDef[] {
  const seen = new Set<string>();
  const cards: CardDef[] = [];
  for (const change of patch.changes) {
    if (change.id === GLITCH_DEF_ID || seen.has(change.id)) continue;
    seen.add(change.id);
    const def = change.kind === "removed" ? before?.[change.id] : after[change.id];
    if (def === undefined) {
      const where = change.kind === "removed" ? "the snapshot before it" : "its own snapshot";
      throw new Error(`release art: ${patch.version} records ${change.id} as ${change.kind}, but ${where} holds no such card`);
    }
    cards.push(def);
  }
  return cards;
}

/**
 * R388, R1369000: the definitions a patch's artwork shows, from its record alone (`releaseArtChanged`).
 * More than `RELEASE_ART_CARD_LIMIT` are sampled evenly, the first and the last among them; a record
 * with no card yields none, and the artwork draws card backs.
 */
export function releaseArtCards(patch: Patch, after: Snapshot, before: Snapshot | null): CardDef[] {
  const cards = releaseArtChanged(patch, after, before);
  if (cards.length <= RELEASE_ART_CARD_LIMIT) return cards;
  const last = cards.length - 1;
  const picked: CardDef[] = [];
  for (let at = 0; at < RELEASE_ART_CARD_LIMIT; at += 1) {
    const def = cards[Math.round((at * last) / (RELEASE_ART_CARD_LIMIT - 1))];
    if (def !== undefined) picked.push(def);
  }
  return picked;
}

/** R3012001: how a major's poster ranks a card for its fan: the rarest first, a token last. */
const RARITY_RANK: Readonly<Record<CardDef["rarity"], number>> = { Mythic: 4, Legendary: 3, Epic: 2, Rare: 1, Common: 0, Token: -1 };

/**
 * R3012001: the headline cards of a major's poster: the `RELEASE_ART_MAJOR_CARD_LIMIT` rarest cards of
 * its record (ties keep the record's order), laid out so the best one is the fan's middle card, on top,
 * and each next one alternates to its right and its left.
 */
export function releaseArtHeadlines(patch: Patch, after: Snapshot, before: Snapshot | null): CardDef[] {
  const ranked = releaseArtChanged(patch, after, before)
    .map((def, at) => ({ def, at }))
    .sort((a, b) => RARITY_RANK[b.def.rarity] - RARITY_RANK[a.def.rarity] || a.at - b.at)
    .slice(0, RELEASE_ART_MAJOR_CARD_LIMIT);
  const centered: CardDef[] = [];
  ranked.forEach(({ def }, at) => {
    if (at % 2 === 0) centered.push(def);
    else centered.unshift(def);
  });
  return centered;
}

/** The size a line's capitals are drawn at: the layout's, or smaller when the name would run past the margins. */
function lineFontSize(name: string, layout: ArtLayout): number {
  const room = (RELEASE_ART_WIDTH - 2 * MARGIN_X) / Math.max(1, [...name].length) - layout.lineLetterSpacing;
  return Math.min(layout.lineFontSize, Math.floor(room / LINE_GLYPH_EMS));
}

/** R3012001: how many distinct cards a record names, the figure on a major's poster; Glitch is never counted (R674). */
function cardsNamed(patch: Patch): string | null {
  const count = new Set(patch.changes.filter((change) => change.id !== GLITCH_DEF_ID).map((change) => change.id)).size;
  return count === 0 ? null : count === 1 ? "1 card" : `${String(count)} cards`;
}

/** The artwork's SVG: self-contained, with no script, no style sheet, no remote file and no clock. Nothing in it moves. */
export function releaseArtSvg(input: ReleaseArtInput): string {
  const { patch, release, kind, before, after } = input;
  const layout = LAYOUTS[kind];
  const cards = kind === "major" ? releaseArtHeadlines(patch, after, before) : releaseArtCards(patch, after, before);
  const title = playerTitle(patch);
  const description =
    cards.length === 0
      ? `${release.name}: ${patch.date}. This patch changed no card.`
      : `${release.name}: ${patch.date}. ${cards.map((def) => def.name).join(", ")}.`;
  const places = fan(cards.length === 0 ? BACK_COUNT : cards.length, layout);
  const faces =
    cards.length === 0
      ? drawOrder(places).map((place) => placed(place, '<use href="#back"/>', layout.cardScale))
      : drawOrder(places).map((place) => {
          const def = cards[place.at];
          return def === undefined ? "" : placed(place, cardMarkup(def), layout.cardScale);
        });
  const first = cards[0];
  const backdrop =
    first === undefined
      ? ""
      : `<image href="${proceduralCardArtUri(first, false)}" x="${(RELEASE_ART_WIDTH - BACKDROP_SIZE) / 2}" y="${(RELEASE_ART_HEIGHT - BACKDROP_SIZE) / 2}" ` +
        `width="${BACKDROP_SIZE}" height="${BACKDROP_SIZE}" preserveAspectRatio="xMidYMid slice" opacity="${BACKDROP_OPACITY}"/>`;
  const right = RELEASE_ART_WIDTH - MARGIN_X;
  const stat = layout.statY === null ? null : cardsNamed(patch);
  const head =
    `<text x="${MARGIN_X}" y="${layout.lineY}" font-family="${SERIF}" font-size="${lineFontSize(release.name, layout)}" font-weight="700" letter-spacing="${layout.lineLetterSpacing}" fill="${layout.lineFill}">${escapeXml(release.name.toUpperCase())}</text>` +
    `<text x="${MARGIN_X}" y="${layout.versionY}" font-family="${SERIF}" font-size="${layout.versionFontSize}" font-weight="800" fill="${GOLD}">${escapeXml(patch.version)}</text>` +
    textLines(
      wrap(title, layout.titleLineChars, TITLE_MAX_LINES),
      MARGIN_X,
      layout.titleY,
      layout.titleLineHeight,
      `font-family="${SERIF}" font-size="${layout.titleFontSize}" font-weight="700" fill="${GOLD_PALE}"`,
    ) +
    `<text x="${right}" y="${layout.markY}" text-anchor="end" font-family="${SERIF}" font-size="${MARK_FONT_SIZE}" font-weight="800" fill="${GOLD}">${escapeXml(SITE_NAME)}</text>` +
    `<text x="${right}" y="${layout.dateY}" text-anchor="end" font-family="${SERIF}" font-size="${DATE_FONT_SIZE}" fill="${PARCHMENT}">${escapeXml(patch.date)}</text>` +
    (stat === null || layout.statY === null
      ? ""
      : `<text x="${right}" y="${layout.statY}" text-anchor="end" font-family="${SERIF}" font-size="${DATE_FONT_SIZE}" font-weight="700" fill="${TEAL}">${escapeXml(stat)}</text>`);
  const frame = layout.framed
    ? `<rect x="${FRAME_INSET}" y="${FRAME_INSET}" width="${RELEASE_ART_WIDTH - 2 * FRAME_INSET}" height="${RELEASE_ART_HEIGHT - 2 * FRAME_INSET}" rx="${FRAME_RADIUS}" fill="none" stroke="${BRASS}" stroke-width="${FRAME_STROKE}"/>`
    : "";
  return (
    `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${RELEASE_ART_WIDTH} ${RELEASE_ART_HEIGHT}" width="${RELEASE_ART_WIDTH}" height="${RELEASE_ART_HEIGHT}" ` +
    `role="img" aria-labelledby="title desc">` +
    `<title id="title">${escapeXml(`${patch.version}: ${title}`)}</title>` +
    `<desc id="desc">${escapeXml(description)}</desc>` +
    defsMarkup() +
    `<rect width="${RELEASE_ART_WIDTH}" height="${RELEASE_ART_HEIGHT}" fill="url(#sky)"/>` +
    `<g>${backdrop}<rect width="${RELEASE_ART_WIDTH}" height="${RELEASE_ART_HEIGHT}" fill="url(#shade)"/></g>` +
    `<rect width="${RELEASE_ART_WIDTH}" height="${RELEASE_ART_HEIGHT}" fill="url(#lattice)"/>` +
    `<circle cx="${CENTER_X}" cy="${layout.fanCenterY}" r="${GLOW_RADIUS}" fill="url(#glow)"/>` +
    frame +
    head +
    `<g>${faces.join("")}</g>` +
    "</svg>\n"
  );
}

/** Where a version's artwork is served: its own file, or a major's `-still` companion. */
export function releaseArtPath(version: string, file: ReleaseArtFile = "main"): string {
  return `${RELEASE_ART_DIR}${encodeURIComponent(version)}${file === "still" ? STILL_SUFFIX : NORMAL_SUFFIX}`;
}

/** Each version's release line: the line `releaseGroups` seats it in (R724000). */
function linesOf(patches: readonly Patch[], releases: readonly Release[]): Map<string, Release> {
  const lines = new Map<string, Release>();
  for (const group of releaseGroups(patches, releases)) {
    for (const patch of group.patches) lines.set(patch.version, group.release);
  }
  return lines;
}

/**
 * R3012001: what each version's artwork is, or null for a version with none: a major draws a poster, a
 * patch its art, and a micro nothing, except those up to `legacyMicroLast` in patches.json's order,
 * whose published links keep a file (CLAUDE.md rule 13). Versions are matched by equality (R105).
 */
function artKinds(patches: readonly Patch[], legacyMicroLast: string): (ReleaseArtKind | null)[] {
  const legacy = patches.findIndex((patch) => patch.version === legacyMicroLast);
  return tiers(patches).map((tier, at): ReleaseArtKind | null => (tier === "major" ? "major" : tier === "patch" || at <= legacy ? "patch" : null));
}

function inputAt(
  patches: readonly Patch[],
  at: number,
  kind: ReleaseArtKind,
  snapshot: (version: string) => Snapshot | null,
  releases: readonly Release[],
): ReleaseArtInput {
  const patch = patches[at];
  if (patch === undefined) throw new Error(`release art: no patch at ${String(at)}`);
  const after = snapshot(patch.version);
  if (after === null) throw new Error(`release art: no snapshot of ${patch.version}`);
  const previous = patches[at - 1];
  const release = linesOf(patches, releases).get(patch.version);
  if (release === undefined) throw new Error(`release art: ${patch.version} sits in no release line`);
  return { patch, release, kind, before: previous === undefined ? null : snapshot(previous.version), after };
}

/**
 * Every file of artwork of the history, in patches.json's order (R105): each major's and patch's own
 * file, and for each major its `-still` companion after it, the same bytes. A micro has none, but one
 * up to `legacyMicroLast`. `snapshot` reads one version's catalog snapshot; the previous one is
 * carried forward, so each is read once, in order (micros' too), and a removed card is drawn as its
 * last snapshot held it.
 */
export function releaseArtAssets(
  patches: readonly Patch[],
  snapshot: (version: string) => Snapshot | null,
  releases: readonly Release[] = RELEASES,
  legacyMicroLast: string = RELEASE_ART_LEGACY_MICRO_LAST,
): ReleaseArtAsset[] {
  const lines = linesOf(patches, releases);
  const kinds = artKinds(patches, legacyMicroLast);
  const assets: ReleaseArtAsset[] = [];
  let before: Snapshot | null = null;
  patches.forEach((patch, at) => {
    const after = snapshot(patch.version);
    if (after === null) throw new Error(`release art: no snapshot of ${patch.version}`);
    const release = lines.get(patch.version);
    if (release === undefined) throw new Error(`release art: ${patch.version} sits in no release line`);
    const kind = kinds[at] ?? null;
    if (kind !== null) {
      const svg = releaseArtSvg({ patch, release, kind, before, after });
      assets.push({ path: releaseArtPath(patch.version), svg });
      if (kind === "major") assets.push({ path: releaseArtPath(patch.version, "still"), svg });
    }
    before = after;
  });
  return assets;
}

/**
 * One file of the history by its path, or null for any other path: the dev server's answer, drawn
 * from that patch and the one before it alone. A path is matched whole against the names
 * `releaseArtPath` gives, never parsed: the `.bak` or the upper-cased name of a file is no file.
 */
export function releaseArtAsset(
  path: string,
  patches: readonly Patch[],
  snapshot: (version: string) => Snapshot | null,
  releases: readonly Release[] = RELEASES,
  legacyMicroLast: string = RELEASE_ART_LEGACY_MICRO_LAST,
): ReleaseArtAsset | null {
  const kinds = artKinds(patches, legacyMicroLast);
  for (const [at, patch] of patches.entries()) {
    const kind = kinds[at] ?? null;
    if (kind === null) continue;
    if (path === releaseArtPath(patch.version) || (kind === "major" && path === releaseArtPath(patch.version, "still"))) {
      return { path, svg: releaseArtSvg(inputAt(patches, at, kind, snapshot, releases)) };
    }
  }
  return null;
}
