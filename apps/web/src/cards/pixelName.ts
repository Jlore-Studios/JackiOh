// R2942001 (#2942): the Pixel version's name, set in a pixel face: a 5×7 bitmap face built in (`PIXEL_GLYPHS`), each
// name drawn in capitals as one small SVG of unit squares, white over a black drop shadow, that the name banner wears
// as its background (CardFace, cards.css). The banner's own text stays in the DOM, unseen, so a screen reader and a
// finder by name read it as ever. A name with a character the face lacks (a Chinese name), or longer than the banner
// holds at a readable size (`PIXEL_NAME_MAX_GLYPHS`), prints in the normal face, as a small face's name does (cards.css).
// No `<text>`, `<title>` or `href`: the picture is squares alone. Presentation only (CLAUDE.md rule 7).

/** R2942001: a glyph's width in the face's cells. */
export const PIXEL_GLYPH_COLS = 5;
/** R2942001: a glyph's height in the face's cells. */
export const PIXEL_GLYPH_ROWS = 7;
/** R2942001: the empty cells between two glyphs. */
export const PIXEL_GLYPH_GAP = 1;
/** R2942001: how far the drop shadow lies right of and below the name, in cells. */
export const PIXEL_NAME_SHADOW_OFFSET = 1;
/**
 * R2942001: the least a pixel name's cell may measure, in CSS pixels, on the smallest full face (FACE_TEXT_MIN_HEIGHT_PX
 * tall): below it the bitmap's cells blur together and the name cannot be read.
 */
export const PIXEL_NAME_CELL_MIN_PX = 1;
/**
 * R2942001: the most glyphs a pixel name may have: the most that, side by side across the name banner's content box
 * (cards.css), keep every cell at `PIXEL_NAME_CELL_MIN_PX` or more on the smallest full face. A longer name prints in
 * the normal face, which shrinks it to fit (fit.ts).
 */
export const PIXEL_NAME_MAX_GLYPHS = 15;
/** R2942001: the name's ink, palette.css's `--pal-white`, as the normal name banner's. */
export const PIXEL_NAME_INK = "#ffffff";
/** R2942001: the name's drop shadow, palette.css's `--pal-black`, as the normal name's outline. */
export const PIXEL_NAME_SHADOW = "#000000";

const DATA_PREFIX = "data:image/svg+xml,";
const LIT = "1";

/**
 * R2942001: the pixel face, a capital's or a sign's seven rows of five cells, "1" lit: every letter, every digit, the
 * space and each sign a card's name holds.
 */
export const PIXEL_GLYPHS: Readonly<Record<string, readonly string[]>> = {
  A: ["01110", "10001", "10001", "11111", "10001", "10001", "10001"],
  B: ["11110", "10001", "10001", "11110", "10001", "10001", "11110"],
  C: ["01110", "10001", "10000", "10000", "10000", "10001", "01110"],
  D: ["11110", "10001", "10001", "10001", "10001", "10001", "11110"],
  E: ["11111", "10000", "10000", "11110", "10000", "10000", "11111"],
  F: ["11111", "10000", "10000", "11110", "10000", "10000", "10000"],
  G: ["01110", "10001", "10000", "10111", "10001", "10001", "01111"],
  H: ["10001", "10001", "10001", "11111", "10001", "10001", "10001"],
  I: ["01110", "00100", "00100", "00100", "00100", "00100", "01110"],
  J: ["00111", "00010", "00010", "00010", "00010", "10010", "01100"],
  K: ["10001", "10010", "10100", "11000", "10100", "10010", "10001"],
  L: ["10000", "10000", "10000", "10000", "10000", "10000", "11111"],
  M: ["10001", "11011", "10101", "10101", "10001", "10001", "10001"],
  N: ["10001", "10001", "11001", "10101", "10011", "10001", "10001"],
  O: ["01110", "10001", "10001", "10001", "10001", "10001", "01110"],
  P: ["11110", "10001", "10001", "11110", "10000", "10000", "10000"],
  Q: ["01110", "10001", "10001", "10001", "10101", "10010", "01101"],
  R: ["11110", "10001", "10001", "11110", "10100", "10010", "10001"],
  S: ["01111", "10000", "10000", "01110", "00001", "00001", "11110"],
  T: ["11111", "00100", "00100", "00100", "00100", "00100", "00100"],
  U: ["10001", "10001", "10001", "10001", "10001", "10001", "01110"],
  V: ["10001", "10001", "10001", "10001", "10001", "01010", "00100"],
  W: ["10001", "10001", "10001", "10101", "10101", "10101", "01010"],
  X: ["10001", "10001", "01010", "00100", "01010", "10001", "10001"],
  Y: ["10001", "10001", "01010", "00100", "00100", "00100", "00100"],
  Z: ["11111", "00001", "00010", "00100", "01000", "10000", "11111"],
  Ñ: ["01101", "10010", "10001", "11001", "10101", "10011", "10001"],
  "0": ["01110", "10001", "10011", "10101", "11001", "10001", "01110"],
  "1": ["00100", "01100", "00100", "00100", "00100", "00100", "01110"],
  "2": ["01110", "10001", "00001", "00010", "00100", "01000", "11111"],
  "3": ["11111", "00010", "00100", "00010", "00001", "10001", "01110"],
  "4": ["00010", "00110", "01010", "10010", "11111", "00010", "00010"],
  "5": ["11111", "10000", "11110", "00001", "00001", "10001", "01110"],
  "6": ["00110", "01000", "10000", "11110", "10001", "10001", "01110"],
  "7": ["11111", "00001", "00010", "00100", "01000", "01000", "01000"],
  "8": ["01110", "10001", "10001", "01110", "10001", "10001", "01110"],
  "9": ["01110", "10001", "10001", "01111", "00001", "00010", "01100"],
  " ": ["00000", "00000", "00000", "00000", "00000", "00000", "00000"],
  "!": ["00100", "00100", "00100", "00100", "00100", "00000", "00100"],
  '"': ["01010", "01010", "01010", "00000", "00000", "00000", "00000"],
  "&": ["01100", "10010", "10100", "01000", "10101", "10010", "01101"],
  "'": ["00100", "00100", "01000", "00000", "00000", "00000", "00000"],
  "(": ["00010", "00100", "01000", "01000", "01000", "00100", "00010"],
  ")": ["01000", "00100", "00010", "00010", "00010", "00100", "01000"],
  "+": ["00000", "00100", "00100", "11111", "00100", "00100", "00000"],
  ",": ["00000", "00000", "00000", "00000", "01100", "00100", "01000"],
  "-": ["00000", "00000", "00000", "11111", "00000", "00000", "00000"],
  ".": ["00000", "00000", "00000", "00000", "00000", "01100", "01100"],
  "/": ["00000", "00001", "00010", "00100", "01000", "10000", "00000"],
  ":": ["00000", "01100", "01100", "00000", "01100", "01100", "00000"],
  "?": ["01110", "10001", "00001", "00010", "00100", "00000", "00100"],
  "…": ["00000", "00000", "00000", "00000", "00000", "00000", "10101"],
};

/** The path of `glyphs`' lit cells, each row's runs one rectangle, moved `shift` cells right and down. */
function cellsPath(glyphs: readonly (readonly string[])[], shift: number): string {
  const runs: string[] = [];
  glyphs.forEach((rows, at) => {
    const left = at * (PIXEL_GLYPH_COLS + PIXEL_GLYPH_GAP) + shift;
    rows.forEach((row, y) => {
      for (let x = 0; x < row.length; x += 1) {
        if (row[x] !== LIT) continue;
        let end = x;
        while (row[end + 1] === LIT) end += 1;
        const run = end - x + 1;
        runs.push(`M${String(left + x)} ${String(y + shift)}h${String(run)}v1h-${String(run)}z`);
        x = end;
      }
    });
  });
  return runs.join("");
}

/**
 * R2942001: `name` in the pixel face, in capitals: one `<svg>` of unit squares, its drop shadow under its ink, or null
 * for an empty name, one with a character the face has no glyph for, or one longer than `PIXEL_NAME_MAX_GLYPHS`.
 */
export function pixelNameSvg(name: string): string | null {
  const glyphs: (readonly string[])[] = [];
  for (const char of name.toUpperCase()) {
    const glyph = PIXEL_GLYPHS[char];
    if (glyph === undefined) return null;
    glyphs.push(glyph);
  }
  if (glyphs.length === 0 || glyphs.length > PIXEL_NAME_MAX_GLYPHS) return null;
  const width = glyphs.length * (PIXEL_GLYPH_COLS + PIXEL_GLYPH_GAP) - PIXEL_GLYPH_GAP + PIXEL_NAME_SHADOW_OFFSET;
  const height = PIXEL_GLYPH_ROWS + PIXEL_NAME_SHADOW_OFFSET;
  return (
    `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${String(width)} ${String(height)}" width="${String(width)}" ` +
    `height="${String(height)}" shape-rendering="crispEdges">` +
    `<path fill="${PIXEL_NAME_SHADOW}" d="${cellsPath(glyphs, PIXEL_NAME_SHADOW_OFFSET)}"/>` +
    `<path fill="${PIXEL_NAME_INK}" d="${cellsPath(glyphs, 0)}"/></svg>`
  );
}

/** R2942001: `name` in the pixel face as a `data:` URI for a CSS `url("…")`, or null where `pixelNameSvg` gives none. */
export function pixelNameUri(name: string): string | null {
  const svg = pixelNameSvg(name);
  return svg === null ? null : `${DATA_PREFIX}${encodeURIComponent(svg)}`;
}
