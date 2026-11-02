// The set mark on a card's frame (R503), as a printed card carries its expansion symbol: Core a ringed
// orb, Classic a columned temple (Classic+ with a plus), Meditative an ensō with a red seal (Meditative+
// with a plus, R2718000), Stinkyscape a green planet with stink lines (R1883000), Training Grounds a
// straw target over two crossed practice swords (R2202000); any other set (the reserved Boss sets, one
// this client does not know yet) a plain diamond, so every card shows one.
//
// Presentation only: the set is public catalog data (§5.1) and no rule reads the mark (CLAUDE.md
// rule 7). Each glyph is an original SVG drawn as an `<img>` of a `data:` URI with no <text> and no
// <title>, as icons.tsx draws the face's other glyphs, so a face stays span, strong and img (B12).
//
// R2718000: the same glyph, as one faint flat silhouette cut by an SVG filter, is the set's watermark
// behind a face's rules text (`watermarkOf`), so a later set needs only its mark; a set with no glyph of
// its own (Boss, Boss-X, Custom, one newer than this build) prints none, not the diamond.

import { t } from "../i18n/i18n.ts";

export type SetMarkKind =
  | "core"
  | "classic"
  | "classic-plus"
  | "meditative"
  | "meditative-plus"
  | "stinkyscape"
  | "training-grounds"
  | "unknown";

/** A set mark that is a set's own, not the fallback diamond: the sets that have a watermark (R2718000). */
export type WatermarkKind = Exclude<SetMarkKind, "unknown">;

export type Watermark = {
  kind: WatermarkKind;
  /** The set's silhouette in `WATERMARK_INK` at `WATERMARK_OPACITY`, as a `data:image/svg+xml` URI, for a parchment rules box. */
  src: string;
  /** The same silhouette in `WATERMARK_INK_ON_DARK` at `WATERMARK_OPACITY_ON_DARK`, for the inspect panels' dark ground. */
  srcOnDark: string;
};

export type SetMark = {
  kind: SetMarkKind;
  /** What `title` and `aria-label` say: "Classic+ set". */
  label: string;
  /** The glyph, as a `data:image/svg+xml` URI. */
  src: string;
};

/**
 * Below this face height the mark is noise (a face-up backrow card, a phone's smallest hand): the
 * `@container cardface (max-height: 99px)` rule in setmark.css hides it, and must say one less.
 */
export const SET_MARK_MIN_FACE_PX = 100;

/** R2718000: the watermark's ink on a parchment rules box, a darker shade of the box's own colour. */
export const WATERMARK_INK = "#6b4f2a";
/** R2718000: how much of `WATERMARK_INK` shows on parchment; low enough that the text keeps the box's contrast floor. */
export const WATERMARK_OPACITY = 0.12;
/** R2718000: the watermark's ink on the inspect panels' dark ground, a lighter shade of the panel's own colour. */
export const WATERMARK_INK_ON_DARK = "#e3d2ac";
/** R2718000: how much of `WATERMARK_INK_ON_DARK` shows on the dark ground. */
export const WATERMARK_OPACITY_ON_DARK = 0.06;
/** R2718000: the watermark's height as a percentage of its box's; setmark.css and inspect.css write it as `top` and `height`. */
export const WATERMARK_SIZE_PCT = 88;

const SVG_NS = "http://www.w3.org/2000/svg";
const BRONZE = "#e6b35c";
const BRONZE_INK = "#2a1906";
const WOOD = "#c58a48";
const WOOD_INK = "#3a2210";
const STRAW = "#f1dfa8";
const TARGET_RED = "#d2452f";

/** A wooden practice sword from the hilt at (x1, y1) to the tip at (x2, y2), outlined. */
function woodenSword(x1: number, y1: number, x2: number, y2: number): string {
  return (
    `<path d='M${x1} ${y1}L${x2} ${y2}' stroke='${WOOD_INK}' stroke-width='3.6' stroke-linecap='round'/>` +
    `<path d='M${x1} ${y1}L${x2} ${y2}' stroke='${WOOD}' stroke-width='2' stroke-linecap='round'/>`
  );
}

/** A temple front: a pediment, three columns and a plinth. */
const TEMPLE =
  `<path d='M12 1.8L22 7.2H2Z M4.2 8.4H7.4V16.6H4.2Z M10.4 8.4H13.6V16.6H10.4Z M16.6 8.4H19.8V16.6H16.6Z M1.8 17.8H22.2V21.6H1.8Z' ` +
  `fill='${BRONZE}' stroke='${BRONZE_INK}' stroke-width='1.3' stroke-linejoin='round'/>`;

/** An ensō: a brushed open circle in ink, with a small red seal. */
const ENSO =
  `<circle cx='12' cy='12' r='10.6' fill='#efe6d2' stroke='#1b1f27' stroke-width='1.2'/>` +
  `<path d='M15.9 5.25A7.8 7.8 0 1 0 19.33 9.33L18.35 9.12A5.8 5.8 0 1 1 15.8 6.08Z' fill='#17130f' stroke='#17130f' stroke-width='0.7' stroke-linejoin='round'/>` +
  `<rect x='15.6' y='15.6' width='4.2' height='4.2' rx='0.5' fill='#c8322b' stroke='#5a0f0b' stroke-width='0.6'/>`;

/** The plus a "+" set's mark carries, in the top right corner. */
const PLUS = `<path d='M17.6 0.8H21.2V4.4H23.6V8H21.2V11.6H17.6V8H14.4V4.4H17.6Z' fill='#9dff8a' stroke='#0f2e0a' stroke-width='1.2' stroke-linejoin='round'/>`;

/** A "+" set's mark: its parent's scaled down, and the plus. */
function plusMark(parent: string): string {
  return `<g transform='translate(0 4.6) scale(0.8)'>${parent}</g>${PLUS}`;
}

const GLYPHS: Readonly<Record<SetMarkKind, string>> = {
  core:
    `<circle cx='12' cy='12' r='9.6' fill='#d3dae6' stroke='#1b1f27' stroke-width='1.6'/>` +
    `<circle cx='12' cy='12' r='5.8' fill='#1b1f27'/><circle cx='12' cy='12' r='3' fill='#d3dae6'/>`,
  classic: TEMPLE,
  "classic-plus": plusMark(TEMPLE),
  meditative: ENSO,
  "meditative-plus": plusMark(ENSO),
  stinkyscape:
    `<circle cx='12' cy='12' r='10.6' fill='#1c1446' stroke='#08061a' stroke-width='1.2'/>` +
    `<path d='M7.6 8.6Q6.6 7.4 7.6 6.2T7.6 3.8M10.6 8Q9.6 6.8 10.6 5.6T10.6 3.2M13.6 8.6Q12.6 7.4 13.6 6.2T13.6 3.8' fill='none' stroke='#b6f06a' stroke-width='1.1' stroke-linecap='round'/>` +
    `<path d='M2.6 14.2A8 2.2 0 0 1 18.6 14.2' transform='rotate(-18 10.6 14.2)' fill='none' stroke='#f1dc6b' stroke-width='1.2'/>` +
    `<circle cx='10.6' cy='14.2' r='4.6' fill='#7fd23e' stroke='#16300a' stroke-width='1'/>` +
    `<path d='M18.6 14.2A8 2.2 0 0 1 2.6 14.2' transform='rotate(-18 10.6 14.2)' fill='none' stroke='#f1dc6b' stroke-width='1.2'/>` +
    `<path d='M18.4 4.6L19.1 6.7L21.2 7.4L19.1 8.1L18.4 10.2L17.7 8.1L15.6 7.4L17.7 6.7Z' fill='#fff6c8'/>`,
  "training-grounds":
    woodenSword(3, 21, 21.2, 2.8) +
    woodenSword(21, 21, 2.8, 2.8) +
    `<path d='M3.4 17.2L6.8 20.6M20.6 17.2L17.2 20.6' stroke='${WOOD_INK}' stroke-width='1.8' stroke-linecap='round'/>` +
    `<circle cx='12' cy='12' r='7.6' fill='${STRAW}' stroke='${WOOD_INK}' stroke-width='1.2'/>` +
    `<circle cx='12' cy='12' r='5.2' fill='${TARGET_RED}' stroke='${WOOD_INK}' stroke-width='0.6'/>` +
    `<circle cx='12' cy='12' r='3' fill='${STRAW}'/><circle cx='12' cy='12' r='1.3' fill='${TARGET_RED}'/>`,
  unknown: `<path d='M12 1.6L22.4 12L12 22.4L1.6 12Z' fill='#bba8dc' stroke='#1d1530' stroke-width='1.6' stroke-linejoin='round'/><path d='M12 7.4L16.6 12L12 16.6L7.4 12Z' fill='#1d1530'/>`,
};

/** A glyph's body, drawn on a 24-unit square, as an SVG data URI; the pack's own emblems are drawn the same way. */
export function glyphUri(body: string): string {
  return `data:image/svg+xml,${encodeURIComponent(`<svg xmlns='${SVG_NS}' viewBox='0 0 24 24'>${body}</svg>`)}`;
}

const SRC: Readonly<Record<SetMarkKind, string>> = {
  core: glyphUri(GLYPHS.core),
  classic: glyphUri(GLYPHS.classic),
  "classic-plus": glyphUri(GLYPHS["classic-plus"]),
  meditative: glyphUri(GLYPHS.meditative),
  "meditative-plus": glyphUri(GLYPHS["meditative-plus"]),
  stinkyscape: glyphUri(GLYPHS.stinkyscape),
  "training-grounds": glyphUri(GLYPHS["training-grounds"]),
  unknown: glyphUri(GLYPHS.unknown),
};

const KINDS: Readonly<Record<string, WatermarkKind>> = {
  Core: "core",
  Classic: "classic",
  "Classic+": "classic-plus",
  Meditative: "meditative",
  "Meditative+": "meditative-plus",
  Stinkyscape: "stinkyscape",
  "Training Grounds": "training-grounds",
};

/** The mark a card of `set` wears, the diamond for a set with no glyph. */
export function setMarkOf(set: string): SetMark {
  const kind = Object.hasOwn(KINDS, set) ? (KINDS[set] ?? "unknown") : "unknown";
  return { kind, label: t("{set} set", { set: t(set) }), src: SRC[kind] };
}

/** R2718000: each pixel keeps its coverage less its luminance, cut to all or nothing, in one ink. */
function silhouetteFilter(ink: string): string {
  return (
    `<filter id='w' color-interpolation-filters='sRGB'>` +
    `<feColorMatrix type='matrix' values='0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 -0.2126 -0.7152 -0.0722 1 0'/>` +
    `<feComponentTransfer result='shape'><feFuncA type='discrete' tableValues='0 1'/></feComponentTransfer>` +
    `<feFlood flood-color='${ink}'/><feComposite in2='shape' operator='in'/></filter>`
  );
}

function watermarkSrc(kind: WatermarkKind, ink: string, opacity: number): string {
  return glyphUri(`${silhouetteFilter(ink)}<g filter='url(#w)' opacity='${String(opacity)}'>${GLYPHS[kind]}</g>`);
}

function watermark(kind: WatermarkKind): Watermark {
  return {
    kind,
    src: watermarkSrc(kind, WATERMARK_INK, WATERMARK_OPACITY),
    srcOnDark: watermarkSrc(kind, WATERMARK_INK_ON_DARK, WATERMARK_OPACITY_ON_DARK),
  };
}

const WATERMARKS: Readonly<Record<WatermarkKind, Watermark>> = {
  core: watermark("core"),
  classic: watermark("classic"),
  "classic-plus": watermark("classic-plus"),
  meditative: watermark("meditative"),
  "meditative-plus": watermark("meditative-plus"),
  stinkyscape: watermark("stinkyscape"),
  "training-grounds": watermark("training-grounds"),
};

/** R2718000: the watermark behind a card of `set`'s rules text; none for no set or a set with no symbol. */
export function watermarkOf(set: string | null): Watermark | null {
  if (set === null || !Object.hasOwn(KINDS, set)) return null;
  const kind = KINDS[set];
  return kind === undefined ? null : WATERMARKS[kind];
}
