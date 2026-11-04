// R2949000 (#2949): a look with a picture of its own (the Signature look) draws its card's own painter seeded
// apart from the card's normal picture, by a seed of the card's id and the picture's name (`artSeedOf`): a
// Stinkyscape or Training Grounds card keeps its subject, and a procedural one what its name, tags and type pick
// (theme, composition, motif), the rest grown from the seed (ART.md, Pictures). The normal picture keeps the
// card's id alone, so it and every golden hash are unchanged.
// R2940001 (#2940): the Alt art and Alt full art versions draw the picture "alt", seeded apart as Signature's is;
// the Sketch version draws the card's normal picture in pencil and ink on paper (`sketchSvg`), a filter over it.
// R2942001 (#2942): the Pixel and Comic versions restyle the normal picture as the Sketch does, by a filter over it:
// Pixel's 8-bit cells (`pixelSvg`) and Comic's pop art (`comicSvg`); `restyledSvg` draws whichever a face wears.
// Presentation only: nothing here reaches the engine, an action, `PlayerView` or the log (CLAUDE.md rule 7).

import { fmt, hashId } from "./hash.ts";
import { ART_BOX } from "./procedural.ts";

/**
 * R2942001: a restyle of the card's normal picture, a filter over it: the Sketch's pencil (R2940001), Pixel's cells
 * or Comic's pop art.
 */
export type ArtRestyle = "sketch" | "pixel" | "comic";

/**
 * R2949000: which of a card's pictures a face draws: its normal one, or the Signature look's own. R2940001: or Alt
 * art's own, or the normal one sketched; R2942001: or pixelled, or drawn as a comic.
 */
export type ArtPicture = "normal" | "signature" | "alt" | ArtRestyle;

/** R2942001: the restyle `picture` is, or null for a picture of its own (normal, Signature's, Alt art's). */
export function restyleOf(picture: ArtPicture): ArtRestyle | null {
  return picture === "sketch" || picture === "pixel" || picture === "comic" ? picture : null;
}

/** What joins a card's id to a picture's name in its seed: no catalog id holds it. */
const PICTURE_SEED_SEPARATOR = "#";

/**
 * R2949000: the id a card's painter seeds `picture` by: the card's own id for its normal picture, and R2940001 for
 * its Sketch, which pencils the normal picture; R2942001 for any restyle of it.
 */
export function artSeedOf(defId: string, picture: ArtPicture): string {
  return picture === "normal" || restyleOf(picture) !== null ? defId : `${defId}${PICTURE_SEED_SEPARATOR}${picture}`;
}

/** R2940001: the paper a Sketch is drawn on, palette.css's `--pal-slate-100`. */
export const SKETCH_PAPER = "#e8eaf2";
/** R2940001: the hatching's ink, palette.css's `--pal-slate-700`. */
export const SKETCH_INK = "#3f4964";
/** R2940001: how far the line work's blur reaches, in the picture's 100 units: wider draws heavier lines. */
export const SKETCH_LINE_BLUR = 1.6;
/** R2940001: the gamma that darkens the line work past the dodge's faint greys. */
export const SKETCH_LINE_GAMMA = 2.2;
/** R2940001: how far the hatching turns from the horizontal, in degrees. */
export const SKETCH_HATCH_TURN_DEG = 35;
/** R2940001: the hatching noise's frequency along its strokes: low, so each stroke runs long. */
export const SKETCH_HATCH_FREQUENCY_X = 0.02;
/** R2940001: the hatching noise's frequency across its strokes: high, so strokes sit close. */
export const SKETCH_HATCH_FREQUENCY_Y = 0.7;
/** R2940001: how fast a stroke's opacity falls as its hatching noise rises: steep, so strokes stay thin. */
export const SKETCH_HATCH_SLOPE = -7;
/** R2940001: a stroke's opacity where its hatching noise is nil, before the slope takes it down. */
export const SKETCH_HATCH_OFFSET = 1.2;
/** R2940001: the paper grain's frequency. */
export const SKETCH_GRAIN_FREQUENCY = 0.9;
/** R2940001: how dark the paper grain lies over the drawing, from 0 to 1. */
export const SKETCH_GRAIN_OPACITY = 0.16;
/** R2940001: the noise seeds a card's Sketch takes are its id's hash modulo this, so each card's strokes differ. */
export const SKETCH_SEED_MODULUS = 997;
/** R2940001: the share of a Radiant face's painted picture washed back over its Sketch, so its Radiant paint shows. */
export const SKETCH_RADIANT_WASH = 0.45;
/** The filter region's margin past the picture's 100 units, so the turned hatching covers every corner. */
const SKETCH_FILTER_MARGIN = 25;
const HEX_CHANNEL = 255;
/** A painter's root element, opening and closing: the Sketch draws what it holds inside its own, in the same box. */
const SVG_OPEN = /^<svg[^>]*>/;
const SVG_CLOSE = /<\/svg>$/;

/** A `#rrggbb` colour's channels from 0 to 1. */
function channels(hex: string): [number, number, number] {
  return [1, 3, 5].map((at) => parseInt(hex.slice(at, at + 2), 16) / HEX_CHANNEL) as [number, number, number];
}

/**
 * R2940001: `picture` (a painter's whole SVG document, 100 units square) drawn as a pencil sketch on paper: its
 * greys dodged into line work, hatched in the dark by long strokes of ink and grained, its turbulence seeded by
 * `seedId`. On a Radiant face (`radiant`) the painted picture is washed back over it, so its Radiant paint shows.
 * One `<svg>`, its picture's root unwrapped, and no `<text>`, `<title>` or `href`, as every picture (svg.ts).
 */
export function sketchSvg(picture: string, radiant: boolean, seedId: string): string {
  const drawn = picture.replace(SVG_OPEN, "").replace(SVG_CLOSE, "");
  const seed = hashId(seedId) % SKETCH_SEED_MODULUS;
  const [r, g, b] = channels(SKETCH_INK);
  const side = ART_BOX + 2 * SKETCH_FILTER_MARGIN;
  const center = ART_BOX / 2;
  const wash = radiant
    ? `<feComposite in="SourceGraphic" in2="sk-pencil" operator="arithmetic" k1="0" k2="${fmt(SKETCH_RADIANT_WASH)}" ` +
      `k3="${fmt(1 - SKETCH_RADIANT_WASH)}" k4="0" result="sk-wash"/>`
    : "";
  return (
    `<svg viewBox="0 0 ${ART_BOX} ${ART_BOX}" width="${ART_BOX}" height="${ART_BOX}" ` +
    `xmlns="http://www.w3.org/2000/svg" preserveAspectRatio="xMidYMid slice">` +
    `<defs><filter id="sk-pencil" filterUnits="userSpaceOnUse" x="-${SKETCH_FILTER_MARGIN}" y="-${SKETCH_FILTER_MARGIN}" ` +
    `width="${side}" height="${side}" color-interpolation-filters="sRGB">` +
    `<feColorMatrix type="saturate" values="0" result="sk-grey"/>` +
    `<feComponentTransfer in="sk-grey" result="sk-negative">` +
    `<feFuncR type="linear" slope="-1" intercept="1"/><feFuncG type="linear" slope="-1" intercept="1"/>` +
    `<feFuncB type="linear" slope="-1" intercept="1"/></feComponentTransfer>` +
    `<feGaussianBlur in="sk-negative" stdDeviation="${fmt(SKETCH_LINE_BLUR)}" result="sk-soft"/>` +
    `<feBlend in="sk-soft" in2="sk-grey" mode="color-dodge" result="sk-dodged"/>` +
    `<feComponentTransfer in="sk-dodged" result="sk-lines">` +
    `<feFuncR type="gamma" exponent="${fmt(SKETCH_LINE_GAMMA)}"/><feFuncG type="gamma" exponent="${fmt(SKETCH_LINE_GAMMA)}"/>` +
    `<feFuncB type="gamma" exponent="${fmt(SKETCH_LINE_GAMMA)}"/></feComponentTransfer>` +
    `<feFlood flood-color="${SKETCH_PAPER}" result="sk-paper"/>` +
    `<feBlend in="sk-lines" in2="sk-paper" mode="multiply" result="sk-drawn"/>` +
    `<feTurbulence type="turbulence" baseFrequency="${fmt(SKETCH_HATCH_FREQUENCY_X)} ${fmt(SKETCH_HATCH_FREQUENCY_Y)}" ` +
    `numOctaves="1" seed="${seed}" result="sk-streaks"/>` +
    `<feColorMatrix in="sk-streaks" type="matrix" values="0 0 0 0 ${fmt(r)} 0 0 0 0 ${fmt(g)} 0 0 0 0 ${fmt(b)} ` +
    `${fmt(SKETCH_HATCH_SLOPE)} 0 0 0 ${fmt(SKETCH_HATCH_OFFSET)}" result="sk-strokes"/>` +
    `<feColorMatrix in="sk-grey" type="luminanceToAlpha" result="sk-light"/>` +
    `<feComposite in="sk-strokes" in2="sk-light" operator="out" result="sk-hatch"/>` +
    `<feBlend in="sk-hatch" in2="sk-drawn" mode="multiply" result="sk-shaded"/>` +
    `<feTurbulence type="fractalNoise" baseFrequency="${fmt(SKETCH_GRAIN_FREQUENCY)}" numOctaves="2" seed="${seed}" result="sk-noise"/>` +
    `<feColorMatrix in="sk-noise" type="matrix" values="0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 ${fmt(SKETCH_GRAIN_OPACITY)} 0 0 0 0" result="sk-grain"/>` +
    `<feBlend in="sk-grain" in2="sk-shaded" mode="multiply" result="sk-pencil"/>` +
    wash +
    `</filter></defs>` +
    `<rect width="${ART_BOX}" height="${ART_BOX}" fill="${SKETCH_PAPER}"/>` +
    // The outer turn lays the hatching's long strokes on the diagonal; the inner one sets the picture upright again.
    `<g transform="rotate(${fmt(SKETCH_HATCH_TURN_DEG)} ${center} ${center})" filter="url(#sk-pencil)">` +
    `<g transform="rotate(${fmt(-SKETCH_HATCH_TURN_DEG)} ${center} ${center})">${drawn}</g></g></svg>`
  );
}

/** R2942001: the side of one of Pixel's cells, in the picture's 100 units: 20 cells across. */
export const PIXEL_CELL = 5;
/**
 * R2942001: the side of the sample Pixel takes in the middle of each cell, which grows by half a cell each way to
 * fill it, a little past its edges, so cells meet whatever whole number of pixels a browser grows it by.
 */
export const PIXEL_SAMPLE = 1;
/**
 * R2942001: the levels each colour channel of a base face's Pixel picture is cut to, each to its nearest: 27 colours,
 * each hue a multiple of 30°, so none in the Radiant gold window (R2725004).
 */
export const PIXEL_LEVELS: readonly number[] = [0, 0.5, 1];
/** R2942001: how far Comic blurs the picture before it flattens it, so fine texture flattens into areas of one colour. */
export const COMIC_SMOOTH = 0.8;
/** R2942001: how far Comic brightens the picture before it flattens it, each channel multiplied: pop art reads bright. */
export const COMIC_LIFT = 1.4;
/** R2942001: how far Comic pushes the picture's colour before it flattens it: past 1, so its colours read bright. */
export const COMIC_SATURATE = 1.8;
/** R2942001: the levels each colour channel of Comic's flat colour is cut to, as Pixel's (`PIXEL_LEVELS`). */
export const COMIC_LEVELS: readonly number[] = [0, 0.5, 1];
/** R2942001: the halftone's grid, a dot every this many of the picture's 100 units. */
export const COMIC_DOT_PITCH = 3.5;
/** R2942001: a halftone dot's side before it is softened round: wide enough to outlast the cut on a small face. */
export const COMIC_DOT_SIZE = 1.5;
/** R2942001: how far a halftone dot is blurred before it is cut, which rounds its corners. */
export const COMIC_DOT_SOFT = 0.5;
/** R2942001: how dark a halftone dot lies over the lightest of the picture, from 0 to 1; it fades out toward the dark. */
export const COMIC_DOT_OPACITY = 0.35;
/** R2942001: how strongly an edge between two flat colours turns to ink: higher inks fainter edges. */
export const COMIC_EDGE_GAIN = 6;
/** R2942001: how far an ink outline is widened each way, in the picture's 100 units. */
export const COMIC_INK_WIDTH = 0.6;
/** R2942001: the ink of Comic's outlines and halftone dots, palette.css's `--pal-slate-900`. */
export const COMIC_INK = "#141a2e";
/** R2942001: the share of a Radiant face's painted picture washed back over its Comic, so its Radiant paint shows. */
export const COMIC_RADIANT_WASH = 0.45;

/** A painter's whole picture as a restyle draws it: inside a filtered group of its own `<svg>`, in the same box. */
function restyledRoot(id: string, filter: string, picture: string): string {
  const drawn = picture.replace(SVG_OPEN, "").replace(SVG_CLOSE, "");
  return (
    `<svg viewBox="0 0 ${ART_BOX} ${ART_BOX}" width="${ART_BOX}" height="${ART_BOX}" ` +
    `xmlns="http://www.w3.org/2000/svg" preserveAspectRatio="xMidYMid slice">` +
    `<defs><filter id="${id}" filterUnits="userSpaceOnUse" x="0" y="0" width="${ART_BOX}" height="${ART_BOX}" ` +
    `color-interpolation-filters="sRGB">${filter}</filter></defs>` +
    `<g filter="url(#${id})">${drawn}</g></svg>`
  );
}

/**
 * A transfer cutting the colour channels of its input to `levels`, each to its nearest: a discrete table holds each
 * level between the first and the last twice, so the steps fall halfway between levels. Its result is named `result`.
 */
function cutTo(levels: readonly number[], result: string, input = ""): string {
  const table = levels.flatMap((level, at) => (at === 0 || at === levels.length - 1 ? [level] : [level, level])).map(fmt).join(" ");
  const into = input === "" ? "" : ` in="${input}"`;
  return (
    `<feComponentTransfer${into} result="${result}">` +
    `<feFuncR type="discrete" tableValues="${table}"/><feFuncG type="discrete" tableValues="${table}"/>` +
    `<feFuncB type="discrete" tableValues="${table}"/></feComponentTransfer>`
  );
}

/** A grid over the whole picture, one `side`-square at the middle of every `pitch`-square cell, named `result`. */
function grid(prefix: string, pitch: number, side: number, color: string, result: string): string {
  const at = (pitch - side) / 2;
  const flood = color === "" ? "" : ` flood-color="${color}"`;
  return (
    `<feFlood x="${fmt(at)}" y="${fmt(at)}" width="${fmt(side)}" height="${fmt(side)}"${flood} result="${prefix}-dot"/>` +
    `<feComposite in="${prefix}-dot" in2="${prefix}-dot" x="0" y="0" width="${fmt(pitch)}" height="${fmt(pitch)}" result="${prefix}-cell"/>` +
    `<feTile in="${prefix}-cell" result="${result}"/>`
  );
}

/**
 * R2942001: `picture` (a painter's whole SVG document, 100 units square) as 8-bit pixel art: sampled once in the
 * middle of every `PIXEL_CELL` square (`PIXEL_SAMPLE`), each sample grown to fill its cell, over the picture blurred
 * by half a cell, which fills any seam a small face's whole pixels leave between cells; on a base face its colours
 * cut to `PIXEL_LEVELS`. A Radiant face (`radiant`) keeps its painted colours, so its Radiant paint shows. One
 * `<svg>`, its picture's root unwrapped, and no `<text>`, `<title>` or `href`, as every picture (svg.ts).
 */
export function pixelSvg(picture: string, radiant: boolean): string {
  const half = fmt(PIXEL_CELL / 2);
  const filter =
    grid("px", PIXEL_CELL, PIXEL_SAMPLE, "", "px-grid") +
    `<feComposite in="SourceGraphic" in2="px-grid" operator="in" result="px-samples"/>` +
    `<feMorphology in="px-samples" operator="dilate" radius="${half}" result="px-cells"/>` +
    `<feGaussianBlur in="SourceGraphic" stdDeviation="${half}" result="px-under"/>` +
    `<feMerge result="px-drawn"><feMergeNode in="px-under"/><feMergeNode in="px-cells"/></feMerge>` +
    (radiant ? "" : cutTo(PIXEL_LEVELS, "px-palette", "px-drawn"));
  return restyledRoot("px-pixel", filter, picture);
}

/**
 * R2942001: `picture` (a painter's whole SVG document, 100 units square) as pop art: smoothed (`COMIC_SMOOTH`),
 * brightened (`COMIC_LIFT`), its colour pushed by `COMIC_SATURATE` and flattened to `COMIC_LEVELS`, halftone dots of
 * `COMIC_INK` laid over its lighter parts and a heavy ink outline round every edge between its flat colours. On a
 * Radiant face (`radiant`) the painted picture is washed back over it at `COMIC_RADIANT_WASH`, so its Radiant paint
 * shows. One `<svg>`, as `pixelSvg`.
 */
export function comicSvg(picture: string, radiant: boolean): string {
  const gain = fmt(COMIC_EDGE_GAIN);
  const lift = fmt(COMIC_LIFT);
  const wash = radiant
    ? `<feComposite in="SourceGraphic" in2="pop-drawn" operator="arithmetic" k1="0" k2="${fmt(COMIC_RADIANT_WASH)}" ` +
      `k3="${fmt(1 - COMIC_RADIANT_WASH)}" k4="0" result="pop-wash"/>`
    : "";
  const filter =
    `<feGaussianBlur in="SourceGraphic" stdDeviation="${fmt(COMIC_SMOOTH)}" result="pop-smooth"/>` +
    `<feComponentTransfer in="pop-smooth" result="pop-lit"><feFuncR type="linear" slope="${lift}"/><feFuncG type="linear" slope="${lift}"/>` +
    `<feFuncB type="linear" slope="${lift}"/></feComponentTransfer>` +
    `<feColorMatrix in="pop-lit" type="saturate" values="${fmt(COMIC_SATURATE)}" result="pop-vivid"/>` +
    cutTo(COMIC_LEVELS, "pop-flat", "pop-vivid") +
    // The halftone: a grid of soft dots cut round, kept where the flat picture is light, as a print's tint.
    grid("pop", COMIC_DOT_PITCH, COMIC_DOT_SIZE, COMIC_INK, "pop-grid") +
    `<feGaussianBlur in="pop-grid" stdDeviation="${fmt(COMIC_DOT_SOFT)}" result="pop-soft"/>` +
    `<feComponentTransfer in="pop-soft" result="pop-dots"><feFuncA type="discrete" tableValues="0 ${fmt(COMIC_DOT_OPACITY)}"/></feComponentTransfer>` +
    `<feColorMatrix in="pop-flat" type="luminanceToAlpha" result="pop-light"/>` +
    `<feComposite in="pop-dots" in2="pop-light" operator="in" result="pop-shade"/>` +
    // The outline: where the flat picture's grey steps, inked and widened.
    `<feColorMatrix in="pop-flat" type="saturate" values="0" result="pop-grey"/>` +
    `<feConvolveMatrix in="pop-grey" order="3" kernelMatrix="0 -1 0 -1 4 -1 0 -1 0" preserveAlpha="true" result="pop-edges"/>` +
    `<feColorMatrix in="pop-edges" type="matrix" values="0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 ${gain} 0 0 0 0" result="pop-edge"/>` +
    `<feComponentTransfer in="pop-edge" result="pop-line"><feFuncA type="discrete" tableValues="0 1"/></feComponentTransfer>` +
    `<feMorphology in="pop-line" operator="dilate" radius="${fmt(COMIC_INK_WIDTH)}" result="pop-outline"/>` +
    `<feFlood flood-color="${COMIC_INK}" result="pop-ink-fill"/>` +
    `<feComposite in="pop-ink-fill" in2="pop-outline" operator="in" result="pop-ink"/>` +
    `<feMerge result="pop-drawn"><feMergeNode in="pop-flat"/><feMergeNode in="pop-shade"/><feMergeNode in="pop-ink"/></feMerge>` +
    wash;
  return restyledRoot("pop-comic", filter, picture);
}

/**
 * R2942001: `picture` as the restyle `restyle` draws it: the Sketch's pencil (seeded by `seedId`, R2940001), Pixel's
 * cells or Comic's pop art, base or Radiant (`radiant`).
 */
export function restyledSvg(picture: string, restyle: ArtRestyle, radiant: boolean, seedId: string): string {
  if (restyle === "sketch") return sketchSvg(picture, radiant, seedId);
  return restyle === "pixel" ? pixelSvg(picture, radiant) : comicSvg(picture, radiant);
}
