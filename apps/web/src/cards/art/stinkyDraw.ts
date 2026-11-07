// R3333001: the Stinkyscape painters' drawing primitives, moved out of stinkyscape.ts unchanged so the
// Starship's modules and hulls (starshipModules.ts, starshipHulls.ts) draw with them without an import
// cycle. Every body is the one stinkyscape.ts shipped, so no picture moves (R503).

import { EMBLEM_BOX, EMBLEMS, type EmblemGlyph } from "./emblems.ts";
import { fmt } from "./hash.ts";

export type Paint = {
  space: readonly [string, string];
  nebula: readonly [string, string];
  /** The main surface's light, middle and shadow tones. */
  light: string;
  hull: string;
  dark: string;
  accent: string;
  glow: string;
  line: string;
};

export const OUTLINE = "#0a0d14";

export type Rng = () => number;

export const DEGREES = Math.PI / 180;
export const FULL_TURN = 360;
export const STROKE = 0.7;

export function between(rng: Rng, low: number, high: number): number {
  return low + (high - low) * rng();
}

export function whole(rng: Rng, low: number, high: number): number {
  return low + Math.floor(rng() * (high - low + 1));
}

export function attrs(values: Readonly<Record<string, string | number>>): string {
  return Object.entries(values)
    .map(([key, value]) => `${key}="${typeof value === "number" ? fmt(value) : value}"`)
    .join(" ");
}

export function shape(tag: string, values: Readonly<Record<string, string | number>>): string {
  return `<${tag} ${attrs(values)}/>`;
}

export function points(list: readonly (readonly [number, number])[]): string {
  return list.map(([x, y]) => `${fmt(x)},${fmt(y)}`).join(" ");
}

export function poly(list: readonly (readonly [number, number])[], fill: string, line: string, extra: Record<string, string | number> = {}): string {
  return shape("polygon", { points: points(list), fill, stroke: line, "stroke-width": STROKE, "stroke-linejoin": "round", ...extra });
}

export function mirror(list: readonly (readonly [number, number])[]): (readonly [number, number])[] {
  return [...list, ...[...list].reverse().map(([x, y]) => [x, -y] as const)];
}

export function circle(cx: number, cy: number, r: number, fill: string, extra: Record<string, string | number> = {}): string {
  return shape("circle", { cx, cy, r, fill, ...extra });
}

export function ellipse(cx: number, cy: number, rx: number, ry: number, fill: string, extra: Record<string, string | number> = {}): string {
  return shape("ellipse", { cx, cy, rx, ry, fill, ...extra });
}

export function rect(x: number, y: number, w: number, h: number, fill: string, extra: Record<string, string | number> = {}): string {
  return shape("rect", { x, y, width: w, height: h, fill, ...extra });
}

export function path(d: string, fill: string, extra: Record<string, string | number> = {}): string {
  return shape("path", { d, fill, ...extra });
}

export function line(x1: number, y1: number, x2: number, y2: number, stroke: string, width: number, extra: Record<string, string | number> = {}): string {
  return shape("line", { x1, y1, x2, y2, stroke, "stroke-width": width, "stroke-linecap": "round", ...extra });
}

export function inked(p: Paint): Record<string, string | number> {
  return { stroke: p.line, "stroke-width": STROKE };
}

export function group(transform: string, body: string, extra: Record<string, string | number> = {}): string {
  const more = Object.keys(extra).length === 0 ? "" : ` ${attrs(extra)}`;
  return `<g transform="${transform}"${more}>${body}</g>`;
}

export function at(x: number, y: number, rotate = 0, scale = 1): string {
  return `translate(${fmt(x)} ${fmt(y)}) rotate(${fmt(rotate)}) scale(${fmt(scale)})`;
}

export function twinkle(x: number, y: number, r: number, fill: string, opacity: number): string {
  const w = r * 0.22;
  return path(
    `M${fmt(x)} ${fmt(y - r)}L${fmt(x + w)} ${fmt(y - w)}L${fmt(x + r)} ${fmt(y)}L${fmt(x + w)} ${fmt(y + w)}` +
      `L${fmt(x)} ${fmt(y + r)}L${fmt(x - w)} ${fmt(y + w)}L${fmt(x - r)} ${fmt(y)}L${fmt(x - w)} ${fmt(y - w)}Z`,
    fill,
    { opacity },
  );
}

/** One of EMBLEMS' glyphs, centered at (x, y), `size` across. */
export function glyph(name: EmblemGlyph, x: number, y: number, size: number, fill: string, stroke: string): string {
  const drawn = EMBLEMS[name];
  const scale = size / EMBLEM_BOX;
  const center = EMBLEM_BOX / 2;
  return group(
    `translate(${fmt(x)} ${fmt(y)}) scale(${fmt(scale)}) translate(-${center} -${center})`,
    path(drawn.d, fill, { stroke, "stroke-width": 1.1, "stroke-linejoin": "round", "fill-rule": drawn.rule }),
  );
}
