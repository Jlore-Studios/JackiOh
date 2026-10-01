// The set mark on a card's frame (R503): a small glyph that says which set a card is from, the way a
// printed card carries its expansion symbol. Core is a ringed orb, Classic a columned temple, and
// Classic+ the same temple with a plus beside it; a set with no glyph of its own (the reserved Boss
// sets, or one this client does not know yet) gets a plain diamond, so every card still shows one.
//
// Presentation only: the set is public catalog data (§5.1) and no rule reads the mark (CLAUDE.md
// rule 7). Each glyph is an original SVG drawn as an `<img>` of a `data:` URI with no <text> and no
// <title>, as icons.tsx draws the face's other glyphs, so a face stays span, strong and img (B12).

export type SetMarkKind = "core" | "classic" | "classic-plus" | "unknown";

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

const SVG_NS = "http://www.w3.org/2000/svg";
const BRONZE = "#e6b35c";
const BRONZE_INK = "#2a1906";

/** A temple front: a pediment, three columns and a plinth. */
const TEMPLE =
  `<path d='M12 1.8L22 7.2H2Z M4.2 8.4H7.4V16.6H4.2Z M10.4 8.4H13.6V16.6H10.4Z M16.6 8.4H19.8V16.6H16.6Z M1.8 17.8H22.2V21.6H1.8Z' ` +
  `fill='${BRONZE}' stroke='${BRONZE_INK}' stroke-width='1.3' stroke-linejoin='round'/>`;

const GLYPHS: Readonly<Record<SetMarkKind, string>> = {
  core:
    `<circle cx='12' cy='12' r='9.6' fill='#d3dae6' stroke='#1b1f27' stroke-width='1.6'/>` +
    `<circle cx='12' cy='12' r='5.8' fill='#1b1f27'/><circle cx='12' cy='12' r='3' fill='#d3dae6'/>`,
  classic: TEMPLE,
  "classic-plus":
    `<g transform='translate(0 4.6) scale(0.8)'>${TEMPLE}</g>` +
    `<path d='M17.6 0.8H21.2V4.4H23.6V8H21.2V11.6H17.6V8H14.4V4.4H17.6Z' fill='#9dff8a' stroke='#0f2e0a' stroke-width='1.2' stroke-linejoin='round'/>`,
  unknown: `<path d='M12 1.6L22.4 12L12 22.4L1.6 12Z' fill='#bba8dc' stroke='#1d1530' stroke-width='1.6' stroke-linejoin='round'/><path d='M12 7.4L16.6 12L12 16.6L7.4 12Z' fill='#1d1530'/>`,
};

function dataUri(body: string): string {
  return `data:image/svg+xml,${encodeURIComponent(`<svg xmlns='${SVG_NS}' viewBox='0 0 24 24'>${body}</svg>`)}`;
}

const SRC: Readonly<Record<SetMarkKind, string>> = {
  core: dataUri(GLYPHS.core),
  classic: dataUri(GLYPHS.classic),
  "classic-plus": dataUri(GLYPHS["classic-plus"]),
  unknown: dataUri(GLYPHS.unknown),
};

const KINDS: Readonly<Record<string, SetMarkKind>> = {
  Core: "core",
  Classic: "classic",
  "Classic+": "classic-plus",
};

/** The mark a card of `set` wears. A set name this client has no glyph for gets the diamond. */
export function setMarkOf(set: string): SetMark {
  const kind = Object.hasOwn(KINDS, set) ? (KINDS[set] ?? "unknown") : "unknown";
  return { kind, label: `${set} set`, src: SRC[kind] };
}
