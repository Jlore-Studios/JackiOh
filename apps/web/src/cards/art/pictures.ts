// R2940001 (#2940): a look with a picture of its own (Alt art's, "alt") draws its card's own painter seeded apart from
// the card's normal picture, by a seed of the card's id and the picture's name (`artSeedOf`): a Stinkyscape or
// Training Grounds card keeps its subject, and a procedural one what its name, tags and type pick (theme, composition,
// motif), the rest grown from the seed (ART.md, Pictures). The normal picture keeps the card's id alone, so it and
// every golden hash are unchanged. R3015002 (#3015): every level of a variant draws its first level's picture, and no
// look has a picture of its own past Alt art's (R2949000's Signature picture is gone).
// R2940001: the Alt art and Alt full art versions draw the picture "alt";
// the Sketch version draws the card's normal picture in pencil and ink on paper (`sketchSvg`), a filter over it.
// R2942001 (#2942): the Pixel and Comic versions restyle the normal picture as the Sketch does, by a filter over it:
// Pixel's 8-bit cells (`pixelSvg`) and Comic's pop art (`comicSvg`); `restyledSvg` draws whichever a face wears.
// R2943001 (#2943): the Set showcase version restyles it in its set's own treatment (`showcaseSvg`): Core's glass orb,
// Classic's temple stone, Meditative's ink wash, Stinkyscape's deep space and Training Grounds' carved wood, Classic+
// and Meditative+ drawn as their parents are (`showcaseFamilyOf`). The Out of frame version draws the normal picture in
// two parts (`ArtPart`): its backdrop in the window and its subject cut out over the frame (CardFace).
// R2944001 (#2944): the Immersive version draws its backdrop hazed as far off (`depthSvg`) under a near layer of its
// subject (CardFace), and the Prismatic version its picture in mirror chrome under an iridescent sheen (`prismaticSvg`).
// R3017002 (#3017): four of part 6's variants are repaints (`ArtRepaint`): a new picture of the card's subject, seeded
// apart as Alt art's is (`artSeedOf`), under a treatment of its own: Night's night sky (`nightSvg`), Watercolor's washes
// on paper (`watercolorSvg`), Stained-glass window's leaded panes (`stainedGlassSvg`) and Tarot card's arched card
// (`tarotSvg`). The other two restyle the card's own picture: Woodcut's three-tone print (`woodcutSvg`) and Neon sign's
// glowing tubes (`neonSvg`). None draws a colour of its own in Radiant's gold window (R2725004).
// Presentation only: nothing here reaches the engine, an action, `PlayerView` or the log (CLAUDE.md rule 7).

import { fmt, hashId } from "./hash.ts";
import type { ArtRole } from "./motion.ts";
import { ART_BOX } from "./procedural.ts";

/**
 * R2943001: the look a Set showcase face draws its set in: Classic+ and Meditative+ share their parents' (`SHOWCASE_FAMILIES`).
 */
export type ShowcaseFamily = "core" | "classic" | "meditative" | "stinkyscape" | "training-grounds";

/** R2943001: each shipped set's Set showcase look, by its catalog name. */
export const SHOWCASE_FAMILIES: Readonly<Record<string, ShowcaseFamily>> = {
  Core: "core",
  Classic: "classic",
  "Classic+": "classic",
  Meditative: "meditative",
  "Meditative+": "meditative",
  Stinkyscape: "stinkyscape",
  "Training Grounds": "training-grounds",
};

/** R2943001: the Set showcase look of a card of `set`, or null for a set with none (no set, a boss's). */
export function showcaseFamilyOf(set: string | null): ShowcaseFamily | null {
  return set !== null && Object.hasOwn(SHOWCASE_FAMILIES, set) ? (SHOWCASE_FAMILIES[set] ?? null) : null;
}

/**
 * R3017002: a repaint, a treatment over a new picture of the card's subject, seeded apart from its normal one: Night's,
 * Watercolor's, Stained-glass window's or Tarot card's.
 */
export type ArtRepaint = "night" | "watercolor" | "stained-glass" | "tarot";

/** R3017002: every repaint. */
const REPAINTS: ReadonlySet<string> = new Set<ArtRepaint>(["night", "watercolor", "stained-glass", "tarot"]);

/** R3017002: whether `picture` is a repaint, a treatment over a picture of its own rather than the normal one. */
export function isRepaint(picture: string): picture is ArtRepaint {
  return REPAINTS.has(picture);
}

/**
 * R2942001: a restyle of the card's normal picture, a filter over it: the Sketch's pencil (R2940001), Pixel's cells
 * or Comic's pop art; R2943001: or a set's own Set showcase treatment; R2944001: or Immersive's hazed depth or
 * Prismatic's chrome; R3017002: or Woodcut's print or Neon sign's tubes, or a repaint's treatment, over its own picture.
 */
export type ArtRestyle = "sketch" | "pixel" | "comic" | `showcase-${ShowcaseFamily}` | "depth" | "prismatic" | "woodcut" | "neon" | ArtRepaint;

/** R2943001: what a Set showcase restyle's name starts with, its family after it. */
const SHOWCASE_PREFIX = "showcase-";

/** R2943001: every Set showcase restyle, one per family. */
const SHOWCASE_RESTYLES: ReadonlySet<string> = new Set(Object.values(SHOWCASE_FAMILIES).map((family) => `${SHOWCASE_PREFIX}${family}`));

/**
 * R3344000: the depth planes a face with depth stacks a painter's picture in, far to nearest: the background, the
 * props, the subject, and what lies over it (its over layers, its light and its motes, and its particles).
 */
export type ArtPlane = "far" | "middle" | "near" | "nearest";

/** R3344000: every plane, far to nearest, the order a face stacks them in. */
export const ART_PLANES: readonly ArtPlane[] = ["far", "middle", "near", "nearest"];

/**
 * R2943001: which part of a painter's picture a face draws: all of it, its backdrop alone (the subject left out) or
 * its subject alone (on nothing), which Out of frame lays over its frame. R3344000: or one depth plane of it.
 */
export type ArtPart = "whole" | "backdrop" | "subject" | ArtPlane;

/** R3344000: the plane each of a painter's layer roles (motion.ts, R2715001) is drawn in. */
export const PLANE_OF_ROLE: Readonly<Record<ArtRole, ArtPlane>> = {
  back: "far",
  props: "middle",
  subject: "near",
  over: "nearest",
  light: "nearest",
  motes: "nearest",
};

/**
 * R2943001, R3344000: whether `part` draws a layer of `role`: the whole picture every layer, its backdrop every layer
 * but the subject's, its subject the subject's alone, and a plane the layers of its roles (`PLANE_OF_ROLE`).
 */
export function partDraws(part: ArtPart, role: ArtRole): boolean {
  switch (part) {
    case "whole":
      return true;
    case "backdrop":
      return role !== "subject";
    case "subject":
      return role === "subject";
    default:
      return PLANE_OF_ROLE[role] === part;
  }
}

/**
 * R2943001, R3344000: whether `part` draws what a painter holds in no role, which belongs to `plane`: its sky and rays
 * and vignette to the far plane, its particles to the nearest. The whole picture draws all of it, its backdrop all but
 * what the subject's plane holds, its subject only that, and a plane its own.
 */
export function partHolds(part: ArtPart, plane: ArtPlane): boolean {
  switch (part) {
    case "whole":
      return true;
    case "backdrop":
      return plane !== "near";
    case "subject":
      return plane === "near";
    default:
      return part === plane;
  }
}

/**
 * R2940001: which of a card's pictures a face draws: its normal one, Alt art's own, or the normal one sketched;
 * R2942001: or pixelled, or drawn as a comic; R2943001: or in its set's Set showcase treatment; R2944001: or hazed as
 * far off, or in chrome; R3017002: or cut as a woodcut, or in neon, or a repaint. R3015002: no other (R2949000's
 * Signature picture is gone).
 */
export type ArtPicture = "normal" | "alt" | ArtRestyle;

/** R2942001, R3017002: the restyles with no family, each its own name. */
const PLAIN_RESTYLES: ReadonlySet<string> = new Set(["sketch", "pixel", "comic", "depth", "prismatic", "woodcut", "neon", ...REPAINTS]);

/** R2942001: the restyle `picture` is, or null for a picture of its own with no treatment (normal, Alt art's). */
export function restyleOf(picture: ArtPicture): ArtRestyle | null {
  return PLAIN_RESTYLES.has(picture) || SHOWCASE_RESTYLES.has(picture) ? (picture as ArtRestyle) : null;
}

/** What joins a card's id to a picture's name in its seed: no catalog id holds it. */
const PICTURE_SEED_SEPARATOR = "#";

/**
 * R2940001: the id a card's painter seeds `picture` by: the card's own id for its normal picture and for its Sketch,
 * which pencils the normal picture (R2942001: for any restyle of it), and one of its id and the picture's name for
 * Alt art's (R3017002: and for a repaint's, a picture of its own under its treatment).
 */
export function artSeedOf(defId: string, picture: ArtPicture): string {
  return picture === "normal" || (restyleOf(picture) !== null && !isRepaint(picture)) ? defId : `${defId}${PICTURE_SEED_SEPARATOR}${picture}`;
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

/**
 * A painter's whole picture as a restyle draws it: inside a filtered group of its own `<svg>`, in the same box, and
 * R2943001 `over` (markup of the restyle's own, unfiltered) drawn over it.
 */
function restyledRoot(id: string, filter: string, picture: string, over = ""): string {
  const drawn = picture.replace(SVG_OPEN, "").replace(SVG_CLOSE, "");
  return (
    `<svg viewBox="0 0 ${ART_BOX} ${ART_BOX}" width="${ART_BOX}" height="${ART_BOX}" ` +
    `xmlns="http://www.w3.org/2000/svg" preserveAspectRatio="xMidYMid slice">` +
    `<defs><filter id="${id}" filterUnits="userSpaceOnUse" x="0" y="0" width="${ART_BOX}" height="${ART_BOX}" ` +
    `color-interpolation-filters="sRGB">${filter}</filter></defs>` +
    `<g filter="url(#${id})">${drawn}</g>${over}</svg>`
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

/** R2943001: how far Core's glass orb pushes the picture's colour: past 1, so it reads as lit glass. */
export const SHOWCASE_CORE_SATURATE = 1.4;
/** R2943001: how far Core's glass orb lifts the picture's contrast about its middle grey, each channel multiplied. */
export const SHOWCASE_CORE_CONTRAST = 1.15;
/** R2943001: the glint on Core's glass orb, palette.css's `--pal-slate-100`. */
export const SHOWCASE_CORE_GLINT = "#e8eaf2";
/** R2943001: how bright the glint is at its middle, from 0 to 1. */
export const SHOWCASE_CORE_GLINT_OPACITY = 0.45;
/** R2943001: where the glint sits, in the picture's 100 units, toward the upper left as a lit orb's does. */
export const SHOWCASE_CORE_GLINT_X = 32;
export const SHOWCASE_CORE_GLINT_Y = 28;
/** R2943001: the glint's radius, in the picture's 100 units. */
export const SHOWCASE_CORE_GLINT_R = 24;
/** R2943001: the rim that rounds Core's picture into an orb, palette.css's `--pal-blue-900`. */
export const SHOWCASE_CORE_RIM = "#1e3a8a";
/** R2943001: how dark the rim is at the picture's corners, from 0 to 1. */
export const SHOWCASE_CORE_RIM_OPACITY = 0.7;
/** R2943001: the share of the rim's radius that is clear before it darkens toward the corners. */
export const SHOWCASE_CORE_RIM_START = 0.55;
/** R2943001: the rim's radius, in the picture's 100 units: past the middle of each side, so the corners darken most. */
export const SHOWCASE_CORE_RIM_R = 72;
/** R2943001: the stone Classic's relief is cut in, palette.css's `--pal-stone-500`. */
export const SHOWCASE_STONE = "#8a8577";
/** R2943001: the light Classic's relief is lit by, palette.css's `--pal-slate-100`. */
export const SHOWCASE_STONE_LIGHT = "#e8eaf2";
/** R2943001: where the relief's light comes from: the upper left (degrees round from the right), and how high. */
export const SHOWCASE_STONE_AZIMUTH_DEG = 225;
export const SHOWCASE_STONE_ELEVATION_DEG = 50;
/** R2943001: how deep the relief is cut, the picture's lightness standing as its height. */
export const SHOWCASE_STONE_DEPTH = 3;
/** R2943001: how much light the relief's stone gives back. */
export const SHOWCASE_STONE_DIFFUSE = 1.3;
/** R2943001: the share of the lit stone and of the greyed picture in Classic's relief, so its shading shows through. */
export const SHOWCASE_STONE_MIX = 0.6;
/** R2943001: Meditative's ink and paper, palette.css's `--pal-slate-900` and `--pal-slate-100`. */
export const SHOWCASE_INK = "#141a2e";
export const SHOWCASE_INK_PAPER = "#e8eaf2";
/** R2943001: how far Meditative's wash is softened before it is cut into tones. */
export const SHOWCASE_INK_SOFT = 0.6;
/** R2943001: the tones Meditative's ink wash is cut to, each to its nearest, from the ink to the paper. */
export const SHOWCASE_INK_LEVELS: readonly number[] = [0, 0.35, 0.7, 1];
/** R2943001: Meditative's seal at the picture's lower right, palette.css's `--pal-red-600`. */
export const SHOWCASE_INK_SEAL = "#c0453f";
/** R2943001: the seal's corner and side, in the picture's 100 units, and how strongly it is stamped, from 0 to 1. */
export const SHOWCASE_INK_SEAL_AT = 80;
export const SHOWCASE_INK_SEAL_SIDE = 10;
export const SHOWCASE_INK_SEAL_OPACITY = 0.85;
/** R2943001: Stinkyscape's deep space, from its dark to its light, palette.css's `--pal-slate-900` and `--pal-violet-300`. */
export const SHOWCASE_SPACE = "#141a2e";
export const SHOWCASE_SPACE_LIGHT = "#c4b5fd";
/** R2943001: the stars over Stinkyscape's dark, palette.css's `--pal-white`. */
export const SHOWCASE_STAR = "#ffffff";
/** R2943001: the star noise's frequency: high, so each star is a point. */
export const SHOWCASE_STAR_FREQUENCY = 0.9;
/** R2943001: the star noise's seed: every Set showcase sky holds the same stars. */
export const SHOWCASE_STAR_SEED = 7;
/** R2943001: the noise level a point must pass to be a star, from 0 to 1: high, so stars are sparse. */
export const SHOWCASE_STAR_CUT = 0.7;
/** R2943001: Training Grounds' carved wood, from its dark to its light: palette.css's `--pal-wood-800` and setMark.ts's wood. */
export const SHOWCASE_WOOD = "#2b170c";
export const SHOWCASE_WOOD_LIGHT = "#c58a48";
/** R2943001: the tones Training Grounds' wood is cut to, each to its nearest. */
export const SHOWCASE_WOOD_LEVELS: readonly number[] = [0, 0.5, 1];
/** R2943001: the wood grain's frequency along it and across it: long, close lines. */
export const SHOWCASE_GRAIN_FREQUENCY_X = 0.02;
export const SHOWCASE_GRAIN_FREQUENCY_Y = 0.35;
/** R2943001: the grain's seed, and how dark it lies over the wood, from 0 to 1. */
export const SHOWCASE_GRAIN_SEED = 3;
export const SHOWCASE_GRAIN_OPACITY = 0.25;
/**
 * R2943001: the share of a Radiant face's painted picture washed back over its Set showcase, so its Radiant paint shows
 * (Core's glass orb keeps the picture's own colours, so it needs none).
 */
export const SHOWCASE_RADIANT_WASH = 0.45;

/** A transfer mapping its input's channels, from 0 to 1, onto the line from `dark` to `light`, named `result`. */
function mapOnto(dark: string, light: string, result: string, input: string): string {
  const from = channels(dark);
  const to = channels(light);
  const func = (name: string, at: number): string =>
    `<feFunc${name} type="linear" slope="${fmt((to[at] ?? 0) - (from[at] ?? 0))}" intercept="${fmt(from[at] ?? 0)}"/>`;
  return `<feComponentTransfer in="${input}" result="${result}">${func("R", 0)}${func("G", 1)}${func("B", 2)}</feComponentTransfer>`;
}

/** R2943001: a Radiant face's painted picture washed back over `under`, at `SHOWCASE_RADIANT_WASH`. */
function showcaseWash(under: string): string {
  return (
    `<feComposite in="SourceGraphic" in2="${under}" operator="arithmetic" k1="0" k2="${fmt(SHOWCASE_RADIANT_WASH)}" ` +
    `k3="${fmt(1 - SHOWCASE_RADIANT_WASH)}" k4="0" result="sc-wash"/>`
  );
}

/** R2943001: each Set showcase family's filter id, filter and the markup it draws over the filtered picture. */
function showcaseParts(family: ShowcaseFamily, radiant: boolean): { id: string; filter: string; over: string } {
  const grey = `<feColorMatrix in="SourceGraphic" type="saturate" values="0" result="sc-grey"/>`;
  const wash = (under: string): string => (radiant ? showcaseWash(under) : "");
  switch (family) {
    case "core": {
      const slope = fmt(SHOWCASE_CORE_CONTRAST);
      const intercept = fmt((1 - SHOWCASE_CORE_CONTRAST) / 2);
      const half = ART_BOX / 2;
      return {
        id: "sc-core",
        filter:
          `<feColorMatrix in="SourceGraphic" type="saturate" values="${fmt(SHOWCASE_CORE_SATURATE)}" result="sc-vivid"/>` +
          `<feComponentTransfer in="sc-vivid" result="sc-glass"><feFuncR type="linear" slope="${slope}" intercept="${intercept}"/>` +
          `<feFuncG type="linear" slope="${slope}" intercept="${intercept}"/><feFuncB type="linear" slope="${slope}" intercept="${intercept}"/>` +
          `</feComponentTransfer>`,
        over:
          `<radialGradient id="sc-rim" gradientUnits="userSpaceOnUse" cx="${half}" cy="${half}" r="${SHOWCASE_CORE_RIM_R}">` +
          `<stop offset="${fmt(SHOWCASE_CORE_RIM_START)}" stop-color="${SHOWCASE_CORE_RIM}" stop-opacity="0"/>` +
          `<stop offset="1" stop-color="${SHOWCASE_CORE_RIM}" stop-opacity="${fmt(SHOWCASE_CORE_RIM_OPACITY)}"/></radialGradient>` +
          `<radialGradient id="sc-glint"><stop offset="0" stop-color="${SHOWCASE_CORE_GLINT}" stop-opacity="${fmt(SHOWCASE_CORE_GLINT_OPACITY)}"/>` +
          `<stop offset="1" stop-color="${SHOWCASE_CORE_GLINT}" stop-opacity="0"/></radialGradient>` +
          `<rect width="${ART_BOX}" height="${ART_BOX}" fill="url(#sc-rim)"/>` +
          `<circle cx="${SHOWCASE_CORE_GLINT_X}" cy="${SHOWCASE_CORE_GLINT_Y}" r="${SHOWCASE_CORE_GLINT_R}" fill="url(#sc-glint)"/>`,
      };
    }
    case "classic":
      return {
        id: "sc-stone",
        filter:
          grey +
          `<feColorMatrix in="sc-grey" type="luminanceToAlpha" result="sc-height"/>` +
          `<feDiffuseLighting in="sc-height" surfaceScale="${fmt(SHOWCASE_STONE_DEPTH)}" diffuseConstant="${fmt(SHOWCASE_STONE_DIFFUSE)}" ` +
          `lighting-color="${SHOWCASE_STONE_LIGHT}" result="sc-lit">` +
          `<feDistantLight azimuth="${SHOWCASE_STONE_AZIMUTH_DEG}" elevation="${SHOWCASE_STONE_ELEVATION_DEG}"/></feDiffuseLighting>` +
          `<feFlood flood-color="${SHOWCASE_STONE}" result="sc-rock"/>` +
          `<feBlend in="sc-lit" in2="sc-rock" mode="multiply" result="sc-relief"/>` +
          `<feComposite in="sc-relief" in2="sc-grey" operator="arithmetic" k1="0" k2="${fmt(SHOWCASE_STONE_MIX)}" ` +
          `k3="${fmt(SHOWCASE_STONE_MIX)}" k4="0" result="sc-carved"/>` +
          wash("sc-carved"),
        over: "",
      };
    case "meditative": {
      const seal = SHOWCASE_INK_SEAL_SIDE;
      return {
        id: "sc-ink",
        filter:
          grey +
          `<feGaussianBlur in="sc-grey" stdDeviation="${fmt(SHOWCASE_INK_SOFT)}" result="sc-soft"/>` +
          cutTo(SHOWCASE_INK_LEVELS, "sc-tones", "sc-soft") +
          mapOnto(SHOWCASE_INK, SHOWCASE_INK_PAPER, "sc-washed", "sc-tones") +
          wash("sc-washed"),
        over:
          `<rect x="${SHOWCASE_INK_SEAL_AT}" y="${SHOWCASE_INK_SEAL_AT}" width="${seal}" height="${seal}" rx="1" ` +
          `fill="${SHOWCASE_INK_SEAL}" opacity="${fmt(SHOWCASE_INK_SEAL_OPACITY)}"/>`,
      };
    }
    case "stinkyscape": {
      const [r, g, b] = channels(SHOWCASE_STAR);
      return {
        id: "sc-space",
        filter:
          grey +
          mapOnto(SHOWCASE_SPACE, SHOWCASE_SPACE_LIGHT, "sc-deep", "sc-grey") +
          `<feTurbulence type="fractalNoise" baseFrequency="${fmt(SHOWCASE_STAR_FREQUENCY)}" numOctaves="1" seed="${SHOWCASE_STAR_SEED}" result="sc-noise"/>` +
          `<feColorMatrix in="sc-noise" type="matrix" values="0 0 0 0 ${fmt(r)} 0 0 0 0 ${fmt(g)} 0 0 0 0 ${fmt(b)} 1 0 0 0 0" result="sc-points"/>` +
          `<feComponentTransfer in="sc-points" result="sc-stars"><feFuncA type="linear" slope="${fmt(1 / (1 - SHOWCASE_STAR_CUT))}" ` +
          `intercept="${fmt(-SHOWCASE_STAR_CUT / (1 - SHOWCASE_STAR_CUT))}"/></feComponentTransfer>` +
          `<feColorMatrix in="sc-grey" type="luminanceToAlpha" result="sc-light"/>` +
          `<feComposite in="sc-stars" in2="sc-light" operator="out" result="sc-night"/>` +
          `<feMerge result="sc-sky"><feMergeNode in="sc-deep"/><feMergeNode in="sc-night"/></feMerge>` +
          wash("sc-sky"),
        over: "",
      };
    }
    case "training-grounds":
      return {
        id: "sc-wood",
        filter:
          grey +
          cutTo(SHOWCASE_WOOD_LEVELS, "sc-tones", "sc-grey") +
          mapOnto(SHOWCASE_WOOD, SHOWCASE_WOOD_LIGHT, "sc-plank", "sc-tones") +
          `<feTurbulence type="fractalNoise" baseFrequency="${fmt(SHOWCASE_GRAIN_FREQUENCY_X)} ${fmt(SHOWCASE_GRAIN_FREQUENCY_Y)}" ` +
          `numOctaves="2" seed="${SHOWCASE_GRAIN_SEED}" result="sc-noise"/>` +
          `<feColorMatrix in="sc-noise" type="matrix" values="0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 ${fmt(SHOWCASE_GRAIN_OPACITY)} 0 0 0 0" result="sc-grain"/>` +
          `<feBlend in="sc-grain" in2="sc-plank" mode="multiply" result="sc-carved"/>` +
          wash("sc-carved"),
        over: "",
      };
  }
}

/**
 * R2943001: `picture` (a painter's whole SVG document, 100 units square) in its set's Set showcase treatment, by
 * `family`: Core's glass orb (its colour pushed, a rim and a glint over it), Classic's temple stone (its lightness cut
 * as relief in lit stone), Meditative's ink wash (greyed, softened and cut to `SHOWCASE_INK_LEVELS` of ink on paper,
 * with a seal), Stinkyscape's deep space (its lightness mapped from the dark of space to violet, stars in its dark) or
 * Training Grounds' carved wood (cut to `SHOWCASE_WOOD_LEVELS` of wood, grained). On a Radiant face (`radiant`) every
 * family but Core's washes the painted picture back over it at `SHOWCASE_RADIANT_WASH`, so its Radiant paint shows. One
 * `<svg>`, as `pixelSvg`.
 */
export function showcaseSvg(picture: string, family: ShowcaseFamily, radiant: boolean): string {
  const { id, filter, over } = showcaseParts(family, radiant);
  return restyledRoot(id, filter, picture, over);
}

/** R2944001: how far Immersive's far backdrop is softened, in the picture's 100 units: a little, as distance blurs. */
export const DEPTH_BLUR = 0.5;
/** R2944001: how much of its colour Immersive's far backdrop keeps, below 1, as distant things grey. */
export const DEPTH_SATURATE = 0.7;
/** R2944001: the air Immersive's far backdrop is seen through, palette.css's `--pal-sky-300`. */
export const DEPTH_HAZE = "#7dd3fc";
/** R2944001: the share of the air in Immersive's far backdrop, the rest the softened, dulled picture. */
export const DEPTH_HAZE_MIX = 0.25;
/** R2944001: the share of a Radiant face's painted picture washed back over its Immersive backdrop, so its Radiant paint shows. */
export const DEPTH_RADIANT_WASH = 0.45;
/** R2944001: Prismatic's chrome, from its dark to its light, palette.css's `--pal-slate-900` and `--pal-slate-100`. */
export const PRISMATIC_CHROME_DARK = "#141a2e";
export const PRISMATIC_CHROME_LIGHT = "#e8eaf2";
/**
 * R2944001: the tone curve Prismatic's chrome maps the picture's grey through, from its darkest to its lightest: not
 * rising evenly but banded, light and dark in turn, as a mirror's polish catches what is round it.
 */
export const PRISMATIC_CHROME_TABLE: readonly number[] = [0.1, 0.85, 0.35, 1, 0.6];
/**
 * R2944001: Prismatic's spectrum, palette.css's `--pal-red-400`, `--pal-purple-400`, `--pal-violet-500`, `--pal-sky-400`,
 * `--pal-cyan-400` and `--pal-green-400`: a rainbow with no yellow or orange, so none of it is Radiant's gold.
 */
export const PRISMATIC_SPECTRUM: readonly string[] = ["#f87171", "#c084fc", "#8b5cf6", "#38bdf8", "#22d3ee", "#4ade80"];
/**
 * R2944001: the iridescent sheen across Prismatic's picture: its spectrum there and back, so each colour's neighbours
 * are its spectrum's own and no two of them mix to gold. Mirrored in cards.css's Prismatic edge.
 */
export const PRISMATIC_SHEEN: readonly string[] = [...PRISMATIC_SPECTRUM, ...PRISMATIC_SPECTRUM.slice(0, -1).reverse()];
/** R2944001: how strongly the sheen lies over Prismatic's chrome, from 0 to 1. */
export const PRISMATIC_SHEEN_OPACITY = 0.4;
/** R2944001: the share of a Radiant face's painted picture washed back over its Prismatic chrome, so its Radiant paint shows. */
export const PRISMATIC_RADIANT_WASH = 0.45;

/**
 * R2944001: `picture` (a painter's whole SVG document, 100 units square; on Immersive's window its backdrop alone) as
 * far off: softened (`DEPTH_BLUR`), its colour dulled (`DEPTH_SATURATE`) and seen through `DEPTH_HAZE` at
 * `DEPTH_HAZE_MIX`, so the near layer CardFace lays over it stands out. On a Radiant face (`radiant`) the painted
 * picture is washed back over it at `DEPTH_RADIANT_WASH`, so its Radiant paint shows. One `<svg>`, as `pixelSvg`.
 */
export function depthSvg(picture: string, radiant: boolean): string {
  const filter =
    `<feGaussianBlur in="SourceGraphic" stdDeviation="${fmt(DEPTH_BLUR)}" result="dp-soft"/>` +
    `<feColorMatrix in="dp-soft" type="saturate" values="${fmt(DEPTH_SATURATE)}" result="dp-dull"/>` +
    `<feFlood flood-color="${DEPTH_HAZE}" result="dp-air"/>` +
    `<feComposite in="dp-air" in2="dp-dull" operator="arithmetic" k1="0" k2="${fmt(DEPTH_HAZE_MIX)}" k3="${fmt(1 - DEPTH_HAZE_MIX)}" k4="0" result="dp-far"/>` +
    (radiant
      ? `<feComposite in="SourceGraphic" in2="dp-far" operator="arithmetic" k1="0" k2="${fmt(DEPTH_RADIANT_WASH)}" ` +
        `k3="${fmt(1 - DEPTH_RADIANT_WASH)}" k4="0" result="dp-wash"/>`
      : "");
  return restyledRoot("dp-haze", filter, picture);
}

/**
 * R2944001: `picture` (a painter's whole SVG document, 100 units square) in mirror chrome: greyed, its grey banded
 * through `PRISMATIC_CHROME_TABLE` and mapped from `PRISMATIC_CHROME_DARK` to `PRISMATIC_CHROME_LIGHT`, under an
 * iridescent sheen of `PRISMATIC_SHEEN` at `PRISMATIC_SHEEN_OPACITY`, corner to corner. On a Radiant face (`radiant`)
 * the painted picture is washed back over the chrome at `PRISMATIC_RADIANT_WASH`, so its Radiant paint shows. One
 * `<svg>`, as `pixelSvg`.
 */
export function prismaticSvg(picture: string, radiant: boolean): string {
  const table = PRISMATIC_CHROME_TABLE.map(fmt).join(" ");
  const func = (name: string): string => `<feFunc${name} type="table" tableValues="${table}"/>`;
  const filter =
    `<feColorMatrix in="SourceGraphic" type="saturate" values="0" result="pr-grey"/>` +
    `<feComponentTransfer in="pr-grey" result="pr-banded">${func("R")}${func("G")}${func("B")}</feComponentTransfer>` +
    mapOnto(PRISMATIC_CHROME_DARK, PRISMATIC_CHROME_LIGHT, "pr-mirror", "pr-banded") +
    (radiant
      ? `<feComposite in="SourceGraphic" in2="pr-mirror" operator="arithmetic" k1="0" k2="${fmt(PRISMATIC_RADIANT_WASH)}" ` +
        `k3="${fmt(1 - PRISMATIC_RADIANT_WASH)}" k4="0" result="pr-wash"/>`
      : "");
  const last = PRISMATIC_SHEEN.length - 1;
  const stops = PRISMATIC_SHEEN.map((color, at) => `<stop offset="${fmt(at / last)}" stop-color="${color}"/>`).join("");
  const over =
    `<linearGradient id="pr-sheen" x1="0" y1="0" x2="1" y2="1">${stops}</linearGradient>` +
    `<rect width="${ART_BOX}" height="${ART_BOX}" fill="url(#pr-sheen)" opacity="${fmt(PRISMATIC_SHEEN_OPACITY)}"/>`;
  return restyledRoot("pr-chrome", filter, picture, over);
}

/** R3017002: a Radiant face's painted picture washed back over `under` at `share`, named `result`, so its Radiant paint shows. */
function radiantWash(share: number, under: string, result: string): string {
  return `<feComposite in="SourceGraphic" in2="${under}" operator="arithmetic" k1="0" k2="${fmt(share)}" k3="${fmt(1 - share)}" k4="0" result="${result}"/>`;
}

/** R3017002: `top` laid over `under` at `share` of `top`, the rest `under`, named `result`. */
function mixOf(top: string, under: string, share: number, result: string): string {
  return `<feComposite in="${top}" in2="${under}" operator="arithmetic" k1="0" k2="${fmt(share)}" k3="${fmt(1 - share)}" k4="0" result="${result}"/>`;
}

/** R3017002: every channel of `input` multiplied by `slope`, named `result`. */
function scaled(slope: number, result: string, input: string): string {
  const func = (name: string): string => `<feFunc${name} type="linear" slope="${fmt(slope)}"/>`;
  return `<feComponentTransfer in="${input}" result="${result}">${func("R")}${func("G")}${func("B")}</feComponentTransfer>`;
}

/**
 * R3017002: where `input`'s grey steps, an outline of alpha 1 widened `width` each way, named `result`: Comic's kernel
 * (R2942001) over the grey, gained by `gain` and cut.
 */
function edgesOf(prefix: string, input: string, gain: number, width: number, result: string): string {
  return (
    `<feColorMatrix in="${input}" type="saturate" values="0" result="${prefix}-edge-grey"/>` +
    `<feConvolveMatrix in="${prefix}-edge-grey" order="3" kernelMatrix="0 -1 0 -1 4 -1 0 -1 0" preserveAlpha="true" result="${prefix}-edges"/>` +
    `<feColorMatrix in="${prefix}-edges" type="matrix" values="0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 ${fmt(gain)} 0 0 0 0" result="${prefix}-edge"/>` +
    `<feComponentTransfer in="${prefix}-edge" result="${prefix}-line"><feFuncA type="discrete" tableValues="0 1"/></feComponentTransfer>` +
    `<feMorphology in="${prefix}-line" operator="dilate" radius="${fmt(width)}" result="${result}"/>`
  );
}

/** R3017002: `color` wherever `shape` is drawn, named `result`. */
function paintedIn(prefix: string, color: string, shape: string, result: string): string {
  return `<feFlood flood-color="${color}" result="${prefix}-fill"/><feComposite in="${prefix}-fill" in2="${shape}" operator="in" result="${result}"/>`;
}

/** R3017002: how much of its colour Night's picture keeps, below 1, as colour fades in the dark. */
export const NIGHT_SATURATE = 0.5;
/** R3017002: how far Night dims the picture, each channel multiplied. */
export const NIGHT_DIM = 0.5;
/** R3017002: Night's sky, palette.css's `--pal-blue-900`, mixed into the dimmed picture so its dark reads blue. */
export const NIGHT_SKY = "#1e3a8a";
/** R3017002: the share of the sky in Night's picture, the rest the dimmed one. */
export const NIGHT_SKY_MIX = 0.35;
/** R3017002: Night's moon and its glow, palette.css's `--pal-slate-100`. */
export const NIGHT_MOON = "#e8eaf2";
/** R3017002: where the moon sits, in the picture's 100 units: high on the right, inside every window's crop. */
export const NIGHT_MOON_X = 76;
export const NIGHT_MOON_Y = 28;
/** R3017002: the moon's radius, and its glow's, in the picture's 100 units. */
export const NIGHT_MOON_R = 6;
export const NIGHT_GLOW_R = 18;
/** R3017002: how bright the moon's glow is at its middle, from 0 to 1. */
export const NIGHT_GLOW_OPACITY = 0.5;
/** R3017002: Night's stars, palette.css's `--pal-white`. */
export const NIGHT_STAR = "#ffffff";
/** R3017002: how bright a star is, from 0 to 1. */
export const NIGHT_STAR_OPACITY = 0.85;
/** R3017002: each star's place and radius, in the picture's 100 units, fixed and high in the sky, so every card's sky holds the same. */
export const NIGHT_STARS: readonly (readonly [number, number, number])[] = [
  [14, 24, 0.9],
  [27, 31, 0.6],
  [38, 22, 0.8],
  [53, 27, 0.6],
  [62, 20, 0.9],
  [90, 36, 0.6],
  [8, 38, 0.6],
];
/** R3017002: the share of a Radiant face's painted picture washed back over its Night, so its Radiant paint shows. */
export const NIGHT_RADIANT_WASH = 0.45;

/**
 * R3017002: `picture` (a painter's whole SVG document, 100 units square) at night: its colour dulled (`NIGHT_SATURATE`)
 * and dimmed (`NIGHT_DIM`), mixed toward `NIGHT_SKY` at `NIGHT_SKY_MIX`, under a `NIGHT_MOON` moon in its glow and
 * `NIGHT_STARS` up high. On a Radiant face (`radiant`) the painted picture is washed back over it at `NIGHT_RADIANT_WASH`,
 * so its Radiant paint shows. One `<svg>`, as `pixelSvg`.
 */
export function nightSvg(picture: string, radiant: boolean): string {
  const filter =
    `<feColorMatrix in="SourceGraphic" type="saturate" values="${fmt(NIGHT_SATURATE)}" result="nt-dull"/>` +
    scaled(NIGHT_DIM, "nt-dim", "nt-dull") +
    `<feFlood flood-color="${NIGHT_SKY}" result="nt-sky"/>` +
    mixOf("nt-sky", "nt-dim", NIGHT_SKY_MIX, "nt-dark") +
    (radiant ? radiantWash(NIGHT_RADIANT_WASH, "nt-dark", "nt-wash") : "");
  const stars = NIGHT_STARS.map(([x, y, r]) => `<circle cx="${fmt(x)}" cy="${fmt(y)}" r="${fmt(r)}"/>`).join("");
  const over =
    `<radialGradient id="nt-glow"><stop offset="0" stop-color="${NIGHT_MOON}" stop-opacity="${fmt(NIGHT_GLOW_OPACITY)}"/>` +
    `<stop offset="1" stop-color="${NIGHT_MOON}" stop-opacity="0"/></radialGradient>` +
    `<g fill="${NIGHT_STAR}" opacity="${fmt(NIGHT_STAR_OPACITY)}">${stars}</g>` +
    `<circle cx="${fmt(NIGHT_MOON_X)}" cy="${fmt(NIGHT_MOON_Y)}" r="${fmt(NIGHT_GLOW_R)}" fill="url(#nt-glow)"/>` +
    `<circle cx="${fmt(NIGHT_MOON_X)}" cy="${fmt(NIGHT_MOON_Y)}" r="${fmt(NIGHT_MOON_R)}" fill="${NIGHT_MOON}"/>`;
  return restyledRoot("nt-night", filter, picture, over);
}

/** R3017002: the paper Watercolor is painted on, palette.css's `--pal-slate-100`. */
export const WATERCOLOR_PAPER = "#e8eaf2";
/** R3017002: the frequency of the noise that runs Watercolor's edges, as wet paint runs. */
export const WATERCOLOR_FLOW_FREQUENCY = 0.06;
/** R3017002: how far Watercolor's edges run, in the picture's 100 units. */
export const WATERCOLOR_FLOW_SCALE = 4;
/** R3017002: how far Watercolor softens its run paint. */
export const WATERCOLOR_SOFT = 0.9;
/** R3017002: the share of the paint in Watercolor's picture, the rest the paper showing through its thin washes. */
export const WATERCOLOR_PIGMENT = 0.7;
/** R3017002: the paper grain's frequency, and how dark it lies over the paint, from 0 to 1. */
export const WATERCOLOR_GRAIN_FREQUENCY = 0.8;
export const WATERCOLOR_GRAIN_OPACITY = 0.12;
/** R3017002: the noise seed of the run and the grain: every Watercolor runs alike. */
export const WATERCOLOR_SEED = 5;
/** R3017002: the share of a Radiant face's painted picture washed back over its Watercolor, so its Radiant paint shows. */
export const WATERCOLOR_RADIANT_WASH = 0.45;

/**
 * R3017002: `picture` (a painter's whole SVG document, 100 units square) in watercolor: its edges run by noise
 * (`WATERCOLOR_FLOW_SCALE`) and softened (`WATERCOLOR_SOFT`), laid on `WATERCOLOR_PAPER` and thinned toward it
 * (`WATERCOLOR_PIGMENT`), grained. On a Radiant face (`radiant`) the painted picture is washed back over it at
 * `WATERCOLOR_RADIANT_WASH`, so its Radiant paint shows. One `<svg>`, as `pixelSvg`.
 */
export function watercolorSvg(picture: string, radiant: boolean): string {
  const filter =
    `<feTurbulence type="fractalNoise" baseFrequency="${fmt(WATERCOLOR_FLOW_FREQUENCY)}" numOctaves="2" seed="${WATERCOLOR_SEED}" result="wc-flow"/>` +
    `<feDisplacementMap in="SourceGraphic" in2="wc-flow" scale="${fmt(WATERCOLOR_FLOW_SCALE)}" xChannelSelector="R" yChannelSelector="G" result="wc-run"/>` +
    `<feGaussianBlur in="wc-run" stdDeviation="${fmt(WATERCOLOR_SOFT)}" result="wc-soft"/>` +
    `<feFlood flood-color="${WATERCOLOR_PAPER}" result="wc-paper"/>` +
    `<feMerge result="wc-laid"><feMergeNode in="wc-paper"/><feMergeNode in="wc-soft"/></feMerge>` +
    mixOf("wc-laid", "wc-paper", WATERCOLOR_PIGMENT, "wc-thin") +
    `<feTurbulence type="fractalNoise" baseFrequency="${fmt(WATERCOLOR_GRAIN_FREQUENCY)}" numOctaves="2" seed="${WATERCOLOR_SEED}" result="wc-noise"/>` +
    `<feColorMatrix in="wc-noise" type="matrix" values="0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 ${fmt(WATERCOLOR_GRAIN_OPACITY)} 0 0 0 0" result="wc-grain"/>` +
    `<feBlend in="wc-grain" in2="wc-thin" mode="multiply" result="wc-painted"/>` +
    (radiant ? radiantWash(WATERCOLOR_RADIANT_WASH, "wc-painted", "wc-wash") : "");
  return restyledRoot("wc-paint", filter, picture);
}

/** R3017002: how far Stained-glass window pushes the picture's colour before it cuts it into panes. */
export const STAINED_GLASS_SATURATE = 1.6;
/** R3017002: how far Stained-glass window smooths the picture before it cuts it, so its panes are broad. */
export const STAINED_GLASS_SMOOTH = 1;
/** R3017002: how far Stained-glass window brightens the picture before it cuts it, each channel multiplied: glass lit from behind. */
export const STAINED_GLASS_LIFT = 1.6;
/** R3017002: the levels each colour channel of a pane is cut to, each to its nearest: Pixel's 27 colours (R2942001), none gold. */
export const STAINED_GLASS_LEVELS: readonly number[] = [0, 0.5, 1];
/** R3017002: the light behind the glass, palette.css's `--pal-slate-100`, and how strongly it shines through, from 0 to 1. */
export const STAINED_GLASS_LIGHT = "#e8eaf2";
export const STAINED_GLASS_GLOW_OPACITY = 0.2;
/** R3017002: the lead between the panes, palette.css's `--pal-slate-900`. */
export const STAINED_GLASS_LEAD = "#141a2e";
/** R3017002: how strongly an edge between two panes turns to lead, and how far the lead is widened each way. */
export const STAINED_GLASS_LEAD_GAIN = 6;
export const STAINED_GLASS_LEAD_WIDTH = 0.6;
/** R3017002: the noise whose contour cuts the panes apart where the picture has no edge: its frequency and seed. */
export const STAINED_GLASS_PANE_FREQUENCY = 0.04;
export const STAINED_GLASS_PANE_SEED = 11;
/** R3017002: the levels the pane noise's one channel is cut to, so it steps once along a contour, where lead runs. */
export const STAINED_GLASS_PANE_LEVELS: readonly number[] = [0, 1];
/** R3017002: the share of a Radiant face's painted picture washed back over its window, so its Radiant paint shows. */
export const STAINED_GLASS_RADIANT_WASH = 0.45;

/**
 * R3017002: `picture` (a painter's whole SVG document, 100 units square) as a stained-glass window: its colour pushed
 * (`STAINED_GLASS_SATURATE`), smoothed, brightened (`STAINED_GLASS_LIFT`) and cut to `STAINED_GLASS_LEVELS`, lit from
 * behind by `STAINED_GLASS_LIGHT`, and leaded in `STAINED_GLASS_LEAD` along every edge between its panes and along the
 * contour of one channel of noise, cut to `STAINED_GLASS_PANE_LEVELS`, that parts the broad ones. On a Radiant face (`radiant`) the painted picture is washed back over it at `STAINED_GLASS_RADIANT_WASH`, so
 * its Radiant paint shows. One `<svg>`, as `pixelSvg`.
 */
export function stainedGlassSvg(picture: string, radiant: boolean): string {
  const filter =
    `<feColorMatrix in="SourceGraphic" type="saturate" values="${fmt(STAINED_GLASS_SATURATE)}" result="sg-vivid"/>` +
    `<feGaussianBlur in="sg-vivid" stdDeviation="${fmt(STAINED_GLASS_SMOOTH)}" result="sg-smooth"/>` +
    scaled(STAINED_GLASS_LIFT, "sg-bright", "sg-smooth") +
    cutTo(STAINED_GLASS_LEVELS, "sg-flat", "sg-bright") +
    `<feFlood flood-color="${STAINED_GLASS_LIGHT}" flood-opacity="${fmt(STAINED_GLASS_GLOW_OPACITY)}" result="sg-light"/>` +
    `<feBlend in="sg-light" in2="sg-flat" mode="screen" result="sg-lit"/>` +
    edgesOf("sg", "sg-flat", STAINED_GLASS_LEAD_GAIN, STAINED_GLASS_LEAD_WIDTH, "sg-seams") +
    `<feTurbulence type="fractalNoise" baseFrequency="${fmt(STAINED_GLASS_PANE_FREQUENCY)}" numOctaves="1" seed="${STAINED_GLASS_PANE_SEED}" result="sg-noise"/>` +
    `<feColorMatrix in="sg-noise" type="matrix" values="1 0 0 0 0 1 0 0 0 0 1 0 0 0 0 0 0 0 0 1" result="sg-solid"/>` +
    cutTo(STAINED_GLASS_PANE_LEVELS, "sg-cells", "sg-solid") +
    edgesOf("sg-pane", "sg-cells", STAINED_GLASS_LEAD_GAIN, STAINED_GLASS_LEAD_WIDTH, "sg-panes") +
    `<feMerge result="sg-cames"><feMergeNode in="sg-seams"/><feMergeNode in="sg-panes"/></feMerge>` +
    paintedIn("sg-lead", STAINED_GLASS_LEAD, "sg-cames", "sg-leaded") +
    `<feMerge result="sg-window"><feMergeNode in="sg-lit"/><feMergeNode in="sg-leaded"/></feMerge>` +
    (radiant ? radiantWash(STAINED_GLASS_RADIANT_WASH, "sg-window", "sg-wash") : "");
  return restyledRoot("sg-glass", filter, picture);
}

/** R3017002: Tarot card's duotone, from its dark to its light, palette.css's `--pal-blue-900` and `--pal-slate-100`. */
export const TAROT_DARK = "#1e3a8a";
export const TAROT_LIGHT = "#e8eaf2";
/** R3017002: how much of its colour Tarot card's picture keeps under its duotone, below 1, as an old print's fades. */
export const TAROT_SATURATE = 0.6;
/** R3017002: the share of the duotone in Tarot card's picture, the rest its muted colour. */
export const TAROT_DUOTONE_MIX = 0.55;
/** R3017002: the card's arched border, palette.css's `--pal-violet-700`, and the line inside it, `--pal-slate-100`. */
export const TAROT_BORDER = "#6d28d9";
export const TAROT_LINE = "#e8eaf2";
/** R3017002: the line's width, in the picture's 100 units. */
export const TAROT_LINE_WIDTH = 0.8;
/** R3017002: the arch's pillars, from each side, and its base bar, from the bottom, in the picture's 100 units. */
export const TAROT_PILLAR = 7;
export const TAROT_BASE = 78;
/** R3017002: where the arch springs from its pillars, in the picture's 100 units; its crown is half the window's width above. */
export const TAROT_SPRING = 62;
/** R3017002: the sun in the arch's left spandrel and the crescent in its right, their places and radii, in the picture's 100 units. */
export const TAROT_SUN_X = 13;
export const TAROT_MOON_X = 87;
export const TAROT_EMBLEM_Y = 30;
export const TAROT_EMBLEM_R = 3;
/** R3017002: how far the sun's rays reach past it, in its radii, and how many it has. */
export const TAROT_RAY_REACH = 1.8;
export const TAROT_RAYS = 8;
/** R3017002: the stars in the spandrels and on the pillars, their places and sizes, in the picture's 100 units. */
export const TAROT_STARS: readonly (readonly [number, number, number])[] = [
  [24, 21, 1.4],
  [76, 21, 1.4],
  [3.5, 70, 1.1],
  [96.5, 70, 1.1],
];
/** R3017002: the share of a Radiant face's painted picture washed back over its Tarot card, so its Radiant paint shows. */
export const TAROT_RADIANT_WASH = 0.45;

/** R3017002: a four-pointed star at (`x`, `y`), `size` from its middle to each point. */
function starPath(x: number, y: number, size: number): string {
  const waist = size / 4;
  return (
    `M${fmt(x)} ${fmt(y - size)}L${fmt(x + waist)} ${fmt(y - waist)}L${fmt(x + size)} ${fmt(y)}L${fmt(x + waist)} ${fmt(y + waist)}` +
    `L${fmt(x)} ${fmt(y + size)}L${fmt(x - waist)} ${fmt(y + waist)}L${fmt(x - size)} ${fmt(y)}L${fmt(x - waist)} ${fmt(y - waist)}Z`
  );
}

/**
 * R3017002: `picture` (a painter's whole SVG document, 100 units square) as a fortune card's: its colour muted
 * (`TAROT_SATURATE`) and mixed with a `TAROT_DARK` to `TAROT_LIGHT` duotone at `TAROT_DUOTONE_MIX`, framed by a
 * `TAROT_BORDER` arch on two pillars and a base, a `TAROT_LINE` inside it, a sun and a crescent in its spandrels and
 * stars about it: no words. On a Radiant face (`radiant`) the painted picture is washed back over it at
 * `TAROT_RADIANT_WASH`, so its Radiant paint shows. One `<svg>`, as `pixelSvg`.
 */
export function tarotSvg(picture: string, radiant: boolean): string {
  const filter =
    `<feColorMatrix in="SourceGraphic" type="saturate" values="0" result="tr-grey"/>` +
    mapOnto(TAROT_DARK, TAROT_LIGHT, "tr-duotone", "tr-grey") +
    `<feColorMatrix in="SourceGraphic" type="saturate" values="${fmt(TAROT_SATURATE)}" result="tr-muted"/>` +
    mixOf("tr-duotone", "tr-muted", TAROT_DUOTONE_MIX, "tr-card") +
    (radiant ? radiantWash(TAROT_RADIANT_WASH, "tr-card", "tr-wash") : "");
  const [left, right] = [TAROT_PILLAR, ART_BOX - TAROT_PILLAR];
  const radius = (right - left) / 2;
  const opening = `M${fmt(left)} ${fmt(TAROT_BASE)}V${fmt(TAROT_SPRING)}A${fmt(radius)} ${fmt(radius)} 0 0 1 ${fmt(right)} ${fmt(TAROT_SPRING)}V${fmt(TAROT_BASE)}Z`;
  const rays = Array.from({ length: TAROT_RAYS }, (_, at) => {
    const turn = (2 * Math.PI * at) / TAROT_RAYS;
    const reach = TAROT_EMBLEM_R * TAROT_RAY_REACH;
    return `M${fmt(TAROT_SUN_X)} ${fmt(TAROT_EMBLEM_Y)}L${fmt(TAROT_SUN_X + reach * Math.cos(turn))} ${fmt(TAROT_EMBLEM_Y + reach * Math.sin(turn))}`;
  }).join("");
  const stars = TAROT_STARS.map(([x, y, size]) => starPath(x, y, size)).join("");
  const over =
    `<path d="M0 0H${ART_BOX}V${ART_BOX}H0Z${opening}" fill="${TAROT_BORDER}" fill-rule="evenodd"/>` +
    `<path d="${opening}" fill="none" stroke="${TAROT_LINE}" stroke-width="${fmt(TAROT_LINE_WIDTH)}"/>` +
    `<path d="${rays}" stroke="${TAROT_LINE}" stroke-width="${fmt(TAROT_LINE_WIDTH)}"/>` +
    `<circle cx="${fmt(TAROT_SUN_X)}" cy="${fmt(TAROT_EMBLEM_Y)}" r="${fmt(TAROT_EMBLEM_R)}" fill="${TAROT_LINE}"/>` +
    `<circle cx="${fmt(TAROT_MOON_X)}" cy="${fmt(TAROT_EMBLEM_Y)}" r="${fmt(TAROT_EMBLEM_R)}" fill="${TAROT_LINE}"/>` +
    `<circle cx="${fmt(TAROT_MOON_X + TAROT_EMBLEM_R / 2)}" cy="${fmt(TAROT_EMBLEM_Y - TAROT_EMBLEM_R / 3)}" r="${fmt(TAROT_EMBLEM_R)}" fill="${TAROT_BORDER}"/>` +
    `<path d="${stars}" fill="${TAROT_LINE}"/>`;
  return restyledRoot("tr-tarot", filter, picture, over);
}

/** R3017002: Woodcut's ink and paper, palette.css's `--pal-slate-900` and `--pal-slate-100`. */
export const WOODCUT_INK = "#141a2e";
export const WOODCUT_PAPER = "#e8eaf2";
/** R3017002: the tones a woodcut's grey is cut to, each to its nearest, from the ink to the paper. */
export const WOODCUT_LEVELS: readonly number[] = [0, 0.5, 1];
/** R3017002: the alpha of the gouges over each of the cut's tones: over its middle tone alone, as a cutter's hatching. */
export const WOODCUT_GOUGE_TONES: readonly number[] = [0, 1, 0];
/** R3017002: the gouge noise's frequency along its strokes and across them: long strokes, close together. */
export const WOODCUT_GOUGE_FREQUENCY_X = 0.015;
export const WOODCUT_GOUGE_FREQUENCY_Y = 0.45;
/** R3017002: how fast a gouge's opacity falls as its noise rises, and its opacity where the noise is nil: thin, firm strokes. */
export const WOODCUT_GOUGE_SLOPE = -6;
export const WOODCUT_GOUGE_OFFSET = 1.4;
/** R3017002: the gouges' noise seed: every Woodcut is cut alike. */
export const WOODCUT_GOUGE_SEED = 13;
/** R3017002: the share of a Radiant face's painted picture washed back over its Woodcut, so its Radiant paint shows. */
export const WOODCUT_RADIANT_WASH = 0.45;

/**
 * R3017002: `picture` (a painter's whole SVG document, 100 units square) as a woodcut print: greyed, cut to
 * `WOODCUT_LEVELS` and printed from `WOODCUT_INK` to `WOODCUT_PAPER`, its middle tone gouged by long strokes of ink. On
 * a Radiant face (`radiant`) the painted picture is washed back over it at `WOODCUT_RADIANT_WASH`, so its Radiant paint
 * shows. One `<svg>`, as `pixelSvg`.
 */
export function woodcutSvg(picture: string, radiant: boolean): string {
  const [r, g, b] = channels(WOODCUT_INK);
  const tones = WOODCUT_GOUGE_TONES.map(fmt).join(" ");
  const filter =
    `<feColorMatrix in="SourceGraphic" type="saturate" values="0" result="wd-grey"/>` +
    cutTo(WOODCUT_LEVELS, "wd-tones", "wd-grey") +
    mapOnto(WOODCUT_INK, WOODCUT_PAPER, "wd-printed", "wd-tones") +
    `<feColorMatrix in="wd-tones" type="luminanceToAlpha" result="wd-light"/>` +
    `<feComponentTransfer in="wd-light" result="wd-middle"><feFuncA type="discrete" tableValues="${tones}"/></feComponentTransfer>` +
    `<feTurbulence type="turbulence" baseFrequency="${fmt(WOODCUT_GOUGE_FREQUENCY_X)} ${fmt(WOODCUT_GOUGE_FREQUENCY_Y)}" numOctaves="1" seed="${WOODCUT_GOUGE_SEED}" result="wd-noise"/>` +
    `<feColorMatrix in="wd-noise" type="matrix" values="0 0 0 0 ${fmt(r)} 0 0 0 0 ${fmt(g)} 0 0 0 0 ${fmt(b)} ` +
    `${fmt(WOODCUT_GOUGE_SLOPE)} 0 0 0 ${fmt(WOODCUT_GOUGE_OFFSET)}" result="wd-strokes"/>` +
    `<feComposite in="wd-strokes" in2="wd-middle" operator="in" result="wd-gouges"/>` +
    `<feMerge result="wd-cut"><feMergeNode in="wd-printed"/><feMergeNode in="wd-gouges"/></feMerge>` +
    (radiant ? radiantWash(WOODCUT_RADIANT_WASH, "wd-cut", "wd-wash") : "");
  return restyledRoot("wd-print", filter, picture);
}

/** R3017002: the wall Neon sign hangs on, palette.css's `--pal-slate-900`, and how much of the picture shows on it, from 0 to 1. */
export const NEON_WALL = "#141a2e";
export const NEON_PICTURE_SHOW = 0.2;
/** R3017002: how far Neon sign smooths the picture's grey before it cuts it, and the tones it cuts it to: its tubes run between them. */
export const NEON_SMOOTH = 0.8;
export const NEON_LEVELS: readonly number[] = [0, 0.33, 0.67, 1];
/** R3017002: how strongly a step between two tones becomes a tube, and how far a tube is widened each way. */
export const NEON_TUBE_GAIN = 6;
export const NEON_TUBE_WIDTH = 0.5;
/** R3017002: a tube's colour where the picture is dark and where it is light, palette.css's `--pal-purple-400` and `--pal-cyan-400`. */
export const NEON_TUBE_DARK = "#c084fc";
export const NEON_TUBE_LIGHT = "#22d3ee";
/** R3017002: how far a tube's glow spreads, in the picture's 100 units. */
export const NEON_GLOW_BLUR = 1.6;
/** R3017002: the share of a Radiant face's painted picture washed back over its Neon sign, so its Radiant paint shows. */
export const NEON_RADIANT_WASH = 0.45;

/**
 * R3017002: `picture` (a painter's whole SVG document, 100 units square) as a neon sign: its grey smoothed and cut to
 * `NEON_LEVELS`, a tube along every step between them, coloured from `NEON_TUBE_DARK` where the picture is dark to
 * `NEON_TUBE_LIGHT` where it is light, glowing (`NEON_GLOW_BLUR`) on a `NEON_WALL` wall that shows the picture at
 * `NEON_PICTURE_SHOW`. On a Radiant face (`radiant`) the painted picture is washed back over it at `NEON_RADIANT_WASH`,
 * so its Radiant paint shows. One `<svg>`, as `pixelSvg`.
 */
export function neonSvg(picture: string, radiant: boolean): string {
  const filter =
    `<feColorMatrix in="SourceGraphic" type="saturate" values="0" result="ne-grey"/>` +
    `<feGaussianBlur in="ne-grey" stdDeviation="${fmt(NEON_SMOOTH)}" result="ne-soft"/>` +
    cutTo(NEON_LEVELS, "ne-steps", "ne-soft") +
    edgesOf("ne", "ne-steps", NEON_TUBE_GAIN, NEON_TUBE_WIDTH, "ne-bends") +
    mapOnto(NEON_TUBE_DARK, NEON_TUBE_LIGHT, "ne-hue", "ne-soft") +
    `<feComposite in="ne-hue" in2="ne-bends" operator="in" result="ne-tubes"/>` +
    `<feGaussianBlur in="ne-tubes" stdDeviation="${fmt(NEON_GLOW_BLUR)}" result="ne-glow"/>` +
    `<feFlood flood-color="${NEON_WALL}" result="ne-wall"/>` +
    mixOf("SourceGraphic", "ne-wall", NEON_PICTURE_SHOW, "ne-dim") +
    `<feMerge result="ne-sign"><feMergeNode in="ne-dim"/><feMergeNode in="ne-glow"/><feMergeNode in="ne-glow"/><feMergeNode in="ne-tubes"/></feMerge>` +
    (radiant ? radiantWash(NEON_RADIANT_WASH, "ne-sign", "ne-wash") : "");
  return restyledRoot("ne-neon", filter, picture);
}

/**
 * R2942001: `picture` as the restyle `restyle` draws it: the Sketch's pencil (seeded by `seedId`, R2940001), Pixel's
 * cells or Comic's pop art, base or Radiant (`radiant`); R2943001: or its set's Set showcase treatment; R2944001: or
 * Immersive's depth or Prismatic's chrome; R3017002: or Night's, Watercolor's, Stained-glass window's, Tarot card's,
 * Woodcut's or Neon sign's.
 */
export function restyledSvg(picture: string, restyle: ArtRestyle, radiant: boolean, seedId: string): string {
  if (restyle === "sketch") return sketchSvg(picture, radiant, seedId);
  if (restyle === "pixel") return pixelSvg(picture, radiant);
  if (restyle === "comic") return comicSvg(picture, radiant);
  if (restyle === "depth") return depthSvg(picture, radiant);
  if (restyle === "prismatic") return prismaticSvg(picture, radiant);
  if (restyle === "night") return nightSvg(picture, radiant);
  if (restyle === "watercolor") return watercolorSvg(picture, radiant);
  if (restyle === "stained-glass") return stainedGlassSvg(picture, radiant);
  if (restyle === "tarot") return tarotSvg(picture, radiant);
  if (restyle === "woodcut") return woodcutSvg(picture, radiant);
  if (restyle === "neon") return neonSvg(picture, radiant);
  return showcaseSvg(picture, restyle.slice(SHOWCASE_PREFIX.length) as ShowcaseFamily, radiant);
}
