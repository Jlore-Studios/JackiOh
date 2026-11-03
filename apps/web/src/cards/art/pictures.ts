// R2949000 (#2949): a look with a picture of its own (the Signature look) draws its card's own painter seeded
// apart from the card's normal picture, by a seed of the card's id and the picture's name (`artSeedOf`): a
// Stinkyscape or Training Grounds card keeps its subject, and a procedural one what its name, tags and type pick
// (theme, composition, motif), the rest grown from the seed (ART.md, Pictures). The normal picture keeps the
// card's id alone, so it and every golden hash are unchanged.
// R2940001 (#2940): the Alt art and Alt full art versions draw the picture "alt", seeded apart as Signature's is;
// the Sketch version draws the card's normal picture in pencil and ink on paper (`sketchSvg`), a filter over it.
// Presentation only: nothing here reaches the engine, an action, `PlayerView` or the log (CLAUDE.md rule 7).

import { fmt, hashId } from "./hash.ts";
import { ART_BOX } from "./procedural.ts";

/**
 * R2949000: which of a card's pictures a face draws: its normal one, or the Signature look's own. R2940001: or Alt
 * art's own, or the normal one sketched.
 */
export type ArtPicture = "normal" | "signature" | "alt" | "sketch";

/** What joins a card's id to a picture's name in its seed: no catalog id holds it. */
const PICTURE_SEED_SEPARATOR = "#";

/**
 * R2949000: the id a card's painter seeds `picture` by: the card's own id for its normal picture, and R2940001 for
 * its Sketch, which pencils the normal picture.
 */
export function artSeedOf(defId: string, picture: ArtPicture): string {
  return picture === "normal" || picture === "sketch" ? defId : `${defId}${PICTURE_SEED_SEPARATOR}${picture}`;
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
