// TEMPORARY (#3391 build tooling, deleted before delivery): writes preview sheets of kit parts and card pictures.
import { writeFileSync } from "node:fs";

import type { KitShape } from "./scene.ts";

const PAL: Record<string, string> = {
  black: "#000000", white: "#ffffff", "grey-500": "#6b7280", "stone-500": "#8a8577", "wood-950": "#0e0905", "wood-900": "#120b07",
  "wood-800": "#2b170c", "wood-700": "#5a341b", "wood-600": "#713f12", "brass-950": "#2b1800", "brass-800": "#4a3c1a", "brass-600": "#a86a1f",
  "brass-500": "#c9953f", "gold-400": "#f0c04a", "gold-300": "#ffd24a", "gold-200": "#fde68a", "yellow-400": "#facc15", parchment: "#efe0bf",
  "red-300": "#fca5a5", "red-400": "#f87171", "red-500": "#ef4444", "red-600": "#c0453f", "red-900": "#7f1d1d", "ember-500": "#ff5a36",
  "orange-400": "#fb923c", "lime-200": "#d9f99d", "lime-400": "#a3e635", "green-200": "#bbf7d0", "green-400": "#4ade80", "jade-400": "#4ec98a",
  "green-500": "#22c55e", "green-700": "#15803d", "green-900": "#14532d", "cyan-400": "#22d3ee", "sky-300": "#7dd3fc", "sky-400": "#38bdf8",
  "blue-200": "#bfdbfe", "blue-600": "#3d7bd4", "blue-700": "#1d4ed8", "blue-800": "#2f5d8c", "blue-900": "#1e3a8a", "slate-100": "#e8eaf2",
  "slate-400": "#9aa1b8", "slate-700": "#3f4964", "slate-800": "#2b3349", "slate-900": "#141a2e", "violet-300": "#c4b5fd", "violet-500": "#8b5cf6",
  "violet-700": "#6d28d9", "purple-400": "#c084fc",
};

/** The shapes as SVG paths, inked ones outlined in fine black. */
export function shapesSvg(shapes: readonly KitShape[]): string {
  return shapes
    .map((s) => `<path d="${s.d}" fill="${PAL[s.paint] ?? "#f0f"}" opacity="${s.opacity}" fill-rule="evenodd"${s.inked === true ? ' stroke="#000" stroke-width="0.4" stroke-linejoin="round"' : ""}/>`)
    .join("");
}

/** A grid of 100-unit cells, each its label and its shapes drawn in the 0..100 box, on `bg`. */
export function partSheet(cells: readonly { label: string; shapes: readonly KitShape[] }[], file: string, cols = 5, bg = "#2b3349"): void {
  const rows = Math.ceil(cells.length / cols);
  const body = cells
    .map((cell, i) => {
      const x = (i % cols) * 104;
      const y = Math.floor(i / cols) * 112;
      return `<g transform="translate(${x} ${y})"><rect width="100" height="100" fill="${bg}"/><svg x="0" y="0" width="100" height="100" viewBox="0 0 100 100" overflow="hidden">${shapesSvg(cell.shapes)}</svg><text x="2" y="108" font-size="7" fill="#fff" font-family="sans-serif">${cell.label}</text></g>`;
    })
    .join("");
  writeFileSync(file, `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${cols * 104} ${rows * 112}" width="${cols * 104 * 3}" height="${rows * 112 * 3}"><rect width="100%" height="100%" fill="#111"/>${body}</svg>`);
}

/** A grid of whole card pictures (each an `<svg>` string) with labels. */
export function pictureSheet(cells: readonly { label: string; svg: string }[], file: string, cols = 4): void {
  const rows = Math.ceil(cells.length / cols);
  const body = cells
    .map((cell, i) => {
      const x = (i % cols) * 104;
      const y = Math.floor(i / cols) * 112;
      let inner = cell.svg.replace(/^<svg /, `<svg x="0" y="0" `);
      for (const [, id] of cell.svg.matchAll(/id="([^"]+)"/g)) inner = inner.split(`id="${id}"`).join(`id="c${i}${id}"`).split(`url(#${id})`).join(`url(#c${i}${id})`);
      return `<g transform="translate(${x} ${y})">${inner}<text x="2" y="108" font-size="7" fill="#fff" font-family="sans-serif">${cell.label}</text></g>`;
    })
    .join("");
  writeFileSync(file, `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${cols * 104} ${rows * 112}" width="${cols * 104 * 4}" height="${rows * 112 * 4}"><rect width="100%" height="100%" fill="#111"/>${body}</svg>`);
}
