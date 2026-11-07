// R3402000: each Core Mythic's takeover draws its own full-screen art (issue #3402), over the board's
// dark and under the sweep (dom.ts): My Pawn's hooded hand over a glass chessboard, its strings snapping
// taut down to the pieces; Zephrys's wind spirit uncoiling in a cyclone of silk, chin on fist, as card
// backs orbit him; Heroic Power's white marble rotunda of thirteen glowing doors, a hero in one; Craft a
// Card's floating crystal forge, two glowing rails converging on an anvil under a giant hammer, gears
// overhead. Each scene is data, shapes on a 100 × 100 view sliced to cover the screen, its subject kept
// to the middle that both a phone's crop and a wide screen's show. A shape carries only its geometry and its part:
// fx.css paints and moves each part from the look's tint (transform and opacity alone), and board.css
// paints the still TakeoverStill.tsx draws under Reduce Motion. No shape is text, a card's face or a
// card's name, so the art names no card (R202); a look with no scene keeps its takeover as it was.

import type { TakeoverLook } from "./types.ts";

/** R3402000: the attributes a shape may carry, spelled alike by React and the DOM: geometry only, its paint is CSS's. */
export const ART_ATTRS = ["d", "cx", "cy", "r", "rx", "ry", "x", "y", "width", "height", "x1", "y1", "x2", "y2", "points", "transform"] as const;
export type ArtAttr = (typeof ART_ATTRS)[number];

/** R3402000: one shape of a scene; `part` names its class (`fx-takeover-art-<part>`, `takeover-still-art-<part>`). */
export type ArtShape = {
  tag: "path" | "circle" | "ellipse" | "rect" | "line" | "polygon";
  part: string;
  attrs: Readonly<Partial<Record<ArtAttr, string>>>;
};

/** R3402000: every scene's view, sliced (`TAKEOVER_ART_ASPECT`) so it covers the screen at any shape. */
export const TAKEOVER_ART_VIEWBOX = "0 0 100 100";
export const TAKEOVER_ART_ASPECT = "xMidYMid slice";

const SVG_NS = "http://www.w3.org/2000/svg";

/* My Pawn: the indigo veil, the hooded figure behind the board, the board, the strings, the pieces, the hand. */
const PAWN: readonly ArtShape[] = [
  { tag: "rect", part: "veil", attrs: { x: "0", y: "0", width: "100", height: "100" } },
  {
    tag: "path",
    part: "hood",
    attrs: {
      d:
        "M0 50C12 44 22 40 28 36C26 20 36 6 50 6C64 6 74 20 72 36C78 40 88 44 100 50L100 60C84 56 72 52 67 52L33 52" +
        "C28 52 16 56 0 60Z",
    },
  },
  { tag: "ellipse", part: "shadow", attrs: { cx: "50", cy: "24", rx: "9", ry: "10.5" } },
  { tag: "ellipse", part: "eye", attrs: { cx: "46.4", cy: "24.5", rx: "1.3", ry: "0.7" } },
  { tag: "ellipse", part: "eye", attrs: { cx: "53.6", cy: "24.5", rx: "1.3", ry: "0.7" } },
  { tag: "polygon", part: "board", attrs: { points: "33,52 67,52 84,78 16,78" } },
  { tag: "polygon", part: "slab", attrs: { points: "16,78 84,78 84,80.4 16,80.4" } },
  {
    tag: "path",
    part: "squares",
    attrs: {
      d:
        "M19.8 72.2L27.3 72.2L29.6 67.6L22.8 67.6ZM25.3 63.8L31.5 63.8L33 60.7L27.3 60.7ZM29.1 58L34.3 58L35.4 55.7" +
        "L30.6 55.7ZM31.9 53.7L36.4 53.7L37.3 52L33 52ZM24.5 78L33 78L34.9 72.2L27.3 72.2ZM29.6 67.6L36.4 67.6L37.6 63.8" +
        "L31.5 63.8ZM33 60.7L38.7 60.7L39.5 58L34.3 58ZM35.4 55.7L40.3 55.7L40.9 53.7L36.4 53.7ZM34.9 72.2L42.4 72.2" +
        "L43.2 67.6L36.4 67.6ZM37.6 63.8L43.8 63.8L44.3 60.7L38.7 60.7ZM39.5 58L44.8 58L45.1 55.7L40.3 55.7ZM40.9 53.7" +
        "L45.5 53.7L45.8 52L41.5 52ZM41.5 78L50 78L50 72.2L42.4 72.2ZM43.2 67.6L50 67.6L50 63.8L43.8 63.8ZM44.3 60.7" +
        "L50 60.7L50 58L44.8 58ZM45.1 55.7L50 55.7L50 53.7L45.5 53.7ZM50 72.2L57.6 72.2L56.8 67.6L50 67.6ZM50 63.8" +
        "L56.2 63.8L55.7 60.7L50 60.7ZM50 58L55.2 58L54.9 55.7L50 55.7ZM50 53.7L54.5 53.7L54.3 52L50 52ZM58.5 78L67 78" +
        "L65.1 72.2L57.6 72.2ZM56.8 67.6L63.6 67.6L62.4 63.8L56.2 63.8ZM55.7 60.7L61.3 60.7L60.5 58L55.2 58ZM54.9 55.7" +
        "L59.7 55.7L59.1 53.7L54.5 53.7ZM65.1 72.2L72.7 72.2L70.4 67.6L63.6 67.6ZM62.4 63.8L68.5 63.8L67 60.7L61.3 60.7Z" +
        "M60.5 58L65.7 58L64.6 55.7L59.7 55.7ZM59.1 53.7L63.6 53.7L62.8 52L58.5 52ZM75.5 78L84 78L80.2 72.2L72.7 72.2Z" +
        "M70.4 67.6L77.2 67.6L74.7 63.8L68.5 63.8ZM67 60.7L72.7 60.7L70.9 58L65.7 58ZM64.6 55.7L69.4 55.7L68.1 53.7" +
        "L63.6 53.7Z",
    },
  },
  { tag: "line", part: "string", attrs: { x1: "35.4", y1: "45.9", x2: "35.2", y2: "58.1" } },
  { tag: "line", part: "string", attrs: { x1: "41.1", y1: "49.6", x2: "41.8", y2: "55.6" } },
  { tag: "line", part: "string", attrs: { x1: "46.6", y1: "52", x2: "46.4", y2: "64.7" } },
  { tag: "line", part: "string", attrs: { x1: "52.1", y1: "51.4", x2: "58.9", y2: "58.1" } },
  { tag: "line", part: "string", attrs: { x1: "57.6", y1: "49", x2: "66.2", y2: "61.1" } },
  { tag: "path", part: "piece", attrs: { d: "M33.6 62.8L36.9 62.8L36.4 61.7L35.7 59.7A0.8 0.8 0 1 0 34.7 59.7L34 61.7Z" } },
  { tag: "path", part: "piece", attrs: { d: "M40.3 59.9L43.4 59.9L42.9 58.9L42.3 57.1A0.7 0.7 0 1 0 41.4 57.1L40.7 58.9Z" } },
  { tag: "path", part: "piece", attrs: { d: "M44.4 70.4L48.4 70.4L47.9 69.1L47.1 66.7A1 1 0 1 0 45.8 66.7L45 69.1Z" } },
  { tag: "path", part: "piece", attrs: { d: "M57.2 62.8L60.5 62.8L60.1 61.7L59.4 59.7A0.8 0.8 0 1 0 58.3 59.7L57.7 61.7Z" } },
  { tag: "path", part: "piece", attrs: { d: "M64.4 66.2L68 66.2L67.5 65L66.8 62.9A0.9 0.9 0 1 0 65.6 62.9L64.9 65Z" } },
  {
    tag: "path",
    part: "hand",
    attrs: {
      d:
        "M42 33.5C40.4 35.4 39.6 36.9 39.3 38.2L34.6 44.1A1.6 1.6 0 0 0 36.9 46.3L38.6 44L38.6 48.2" +
        "A2.5 2.5 0 0 0 43.6 48.2L43.8 43.6L44.1 50.6A2.5 2.5 0 0 0 49.1 50.6L49.3 43.8L49.6 50A2.5 2.5 0 0 0 54.6 50" +
        "L54.8 43.6L55.1 47.6A2.5 2.5 0 0 0 60.1 47.6L60.2 41C60.5 38 59.6 35.6 58 33.5Z",
    },
  },
  { tag: "path", part: "cuff", attrs: { d: "M40.5 30.5C45 32.5 55 32.5 59.5 30.5L59 35.5C54.5 37 45.5 37 41 35.5Z" } },
];

/* Zephrys: the veil, three spirals of silk, the spirit (its tail, topknot and head), its arms, the card backs. */
const GENIE: readonly ArtShape[] = [
  { tag: "rect", part: "veil", attrs: { x: "0", y: "0", width: "100", height: "100" } },
  {
    tag: "path",
    part: "silk",
    attrs: {
      d:
        "M57 56C57.4 57.3 60.1 60.9 59.5 63.7C59 66.6 56.9 71 53.6 73.1C50.3 75.2 44.5 77.2 39.7 76.3" +
        "C34.8 75.3 28.1 72.2 24.4 67.4C20.8 62.6 17.4 54.5 17.9 47.4C18.4 40.3 21.6 30.7 27.4 24.9" +
        "C33.1 19 43.1 13.2 52.3 12.3C61.5 11.4 74.2 13.7 82.8 19.6C91.3 25.5 100.5 36.5 103.6 47.5" +
        "C106.7 58.5 106.6 74.3 101.5 85.8C96.5 97.2 85.4 110.3 73.2 116.4C61 122.6 35.8 121.6 28.4 122.6",
    },
  },
  {
    tag: "path",
    part: "silk",
    attrs: {
      d:
        "M45.5 63.8C44 63.4 38.9 63.5 36.5 61.2C34.1 58.8 31.2 54.3 31.1 49.9C31.1 45.5 32.6 38.9 36.2 34.8" +
        "C39.9 30.8 46.7 26.2 53.2 25.5C59.8 24.8 69.2 26.1 75.5 30.5C81.9 34.8 89 43.2 91.3 51.7" +
        "C93.6 60.2 93.5 72.5 89.3 81.6C85.2 90.6 76.2 101 66.2 105.8C56.2 110.5 41.1 112.9 29.3 109.9" +
        "C17.5 106.9 3.1 98.4-4.7 87.6C-12.5 76.8-18.6 59.7-17.7 45.3C-16.9 30.8-2.6 8.4 0.5 1",
    },
  },
  {
    tag: "path",
    part: "silk",
    attrs: {
      d:
        "M47 50.8C47.8 49.9 49.2 46.1 51.7 45.1C54.2 44.2 58.7 43.7 61.9 45.3C65.1 46.9 69.4 50.6 71 54.9" +
        "C72.5 59.2 73.2 66.1 71 71.3C68.9 76.5 64 83 58 85.9C52.1 88.9 42.8 90.8 35.4 88.9C27.9 87 18.5 81.7 13.5 74.6" +
        "C8.4 67.5 4.3 56.1 5 46.4C5.7 36.7 10.4 24.1 17.9 16.4C25.4 8.6 38.3 1.3 50 0C61.7-1.3 77.6 1.5 88.4 8.6" +
        "C99.1 15.6 110.2 36.7 114.6 42.3",
    },
  },
  {
    tag: "path",
    part: "spirit",
    attrs: {
      d:
        "M36 46C36 42.5 40 41 45 40.6L55 40.6C60 41 64 42.5 64 46C64 51 61 55 57.5 59C55 63 61 68 61.5 75" +
        "C62 82 54 86 49 83C45 80.5 46.5 76 50.5 77C47.5 72 44 66 43 60C39 55 36 50.5 36 46Z",
    },
  },
  { tag: "path", part: "spirit", attrs: { d: "M50 29.6C47.4 28 48.2 24.6 50.8 23.2C50.2 25 51.8 26.4 52.6 27.8C53.2 29 51.8 29.8 50 29.6Z" } },
  { tag: "circle", part: "spirit", attrs: { cx: "50", cy: "34.5", r: "5.4" } },
  {
    tag: "path",
    part: "arm",
    attrs: {
      d:
        "M37 46C35.6 50 36.4 54 39.6 56C41 57 43 57.2 45 57L58 56.8C60 56.6 60.4 53.6 58.4 53.4L45 53.2" +
        "C42.6 53 41.4 51 41.6 48Z",
    },
  },
  {
    tag: "path",
    part: "arm",
    attrs: {
      d:
        "M60 43C63.5 44.5 64.6 50 63 54.5C62 57 58.2 57.2 57.2 55L52.6 45.6C50.6 45.8 49.2 44.2 49.4 42.2" +
        "C49.6 40 52 39 53.8 39.9C55.5 40.7 56.2 42.4 55.6 43.7L59.6 51.6C60.2 49.4 59.8 46 58.2 44.4Z",
    },
  },
  { tag: "ellipse", part: "eye", attrs: { cx: "48", cy: "34", rx: "1", ry: "0.55" } },
  { tag: "ellipse", part: "eye", attrs: { cx: "52.2", cy: "34", rx: "1", ry: "0.55" } },
  { tag: "rect", part: "card", attrs: { x: "27.9", y: "30", width: "7.2", height: "10", rx: "0.9" } },
  { tag: "rect", part: "card", attrs: { x: "64.9", y: "25", width: "7.2", height: "10", rx: "0.9" } },
  { tag: "rect", part: "card", attrs: { x: "65.9", y: "64", width: "7.2", height: "10", rx: "0.9" } },
  { tag: "rect", part: "card", attrs: { x: "26.9", y: "67", width: "7.2", height: "10", rx: "0.9" } },
  { tag: "rect", part: "card-mark", attrs: { x: "29.1", y: "31.2", width: "4.8", height: "7.6", rx: "0.5" } },
  { tag: "rect", part: "card-mark", attrs: { x: "66.1", y: "26.2", width: "4.8", height: "7.6", rx: "0.5" } },
  { tag: "rect", part: "card-mark", attrs: { x: "67.1", y: "65.2", width: "4.8", height: "7.6", rx: "0.5" } },
  { tag: "rect", part: "card-mark", attrs: { x: "28.1", y: "68.2", width: "4.8", height: "7.6", rx: "0.5" } },
];

/* Heroic Power: the veil, the dome, its ribs and oculus, the wall, the floor, the pillars, the doors, the hero. */
const ROTUNDA: readonly ArtShape[] = [
  { tag: "rect", part: "veil", attrs: { x: "0", y: "0", width: "100", height: "100" } },
  { tag: "path", part: "dome", attrs: { d: "M6 30C8 6 30 -6 50 -6C70 -6 92 6 94 30A44 10 0 0 1 6 30Z" } },
  {
    tag: "path",
    part: "rib",
    attrs: {
      d:
        "M8.7 33.4Q14.9 6 44.4 4M18.9 37.1Q23.6 6 45.8 4M35 39.4Q37.2 6 47.9 4M65 39.4Q62.8 6 52.1 4M81.1 37.1" +
        "Q76.4 6 54.2 4M91.3 33.4Q85.1 6 55.6 4",
    },
  },
  { tag: "ellipse", part: "oculus", attrs: { cx: "50", cy: "4", rx: "7", ry: "2.4" } },
  { tag: "path", part: "wall", attrs: { d: "M6 70L6 30A44 10 0 0 0 94 30L94 70A44 11 0 0 0 6 70Z" } },
  { tag: "ellipse", part: "floor", attrs: { cx: "50", cy: "70", rx: "44", ry: "11" } },
  { tag: "ellipse", part: "inlay", attrs: { cx: "50", cy: "70", rx: "24", ry: "6" } },
  { tag: "line", part: "pillar", attrs: { x1: "7.8", y1: "66.8", x2: "7.8", y2: "32.9" } },
  { tag: "line", part: "pillar", attrs: { x1: "11.9", y1: "64.5", x2: "11.9", y2: "35" } },
  { tag: "line", part: "pillar", attrs: { x1: "18", y1: "62.5", x2: "18", y2: "36.9" } },
  { tag: "line", part: "pillar", attrs: { x1: "25.8", y1: "60.8", x2: "25.8", y2: "38.4" } },
  { tag: "line", part: "pillar", attrs: { x1: "35", y1: "59.7", x2: "35", y2: "39.4" } },
  { tag: "line", part: "pillar", attrs: { x1: "44.9", y1: "59.1", x2: "44.9", y2: "39.9" } },
  { tag: "line", part: "pillar", attrs: { x1: "55.1", y1: "59.1", x2: "55.1", y2: "39.9" } },
  { tag: "line", part: "pillar", attrs: { x1: "65", y1: "59.7", x2: "65", y2: "39.4" } },
  { tag: "line", part: "pillar", attrs: { x1: "74.2", y1: "60.8", x2: "74.2", y2: "38.4" } },
  { tag: "line", part: "pillar", attrs: { x1: "82", y1: "62.5", x2: "82", y2: "36.9" } },
  { tag: "line", part: "pillar", attrs: { x1: "88.1", y1: "64.5", x2: "88.1", y2: "35" } },
  { tag: "line", part: "pillar", attrs: { x1: "92.2", y1: "66.8", x2: "92.2", y2: "32.9" } },
  { tag: "path", part: "door", attrs: { d: "M5.5 68.1L5.5 55.2A1.2 4.3 0 0 1 7.8 55.2L7.8 68.1Z" } },
  { tag: "path", part: "door", attrs: { d: "M7.7 65.6L7.7 53.6A1.9 4 0 0 1 11.5 53.6L11.5 65.6Z" } },
  { tag: "path", part: "door", attrs: { d: "M12.3 63.4L12.3 52.2A2.4 3.8 0 0 1 17.1 52.2L17.1 63.4Z" } },
  { tag: "path", part: "door", attrs: { d: "M18.9 61.6L18.9 50.9A2.8 3.5 0 0 1 24.5 50.9L24.5 61.6Z" } },
  { tag: "path", part: "door", attrs: { d: "M27.2 60.2L27.2 50A3.1 3.4 0 0 1 33.3 50L33.3 60.2Z" } },
  { tag: "path", part: "door", attrs: { d: "M36.6 59.3L36.6 49.4A3.2 3.3 0 0 1 43.1 49.4L43.1 59.3Z" } },
  { tag: "path", part: "door", attrs: { d: "M46.7 59L46.7 49.3A3.3 3.3 0 0 1 53.2 49.3L53.2 59Z" } },
  { tag: "path", part: "door", attrs: { d: "M56.9 59.3L56.9 49.4A3.2 3.3 0 0 1 63.4 49.4L63.4 59.3Z" } },
  { tag: "path", part: "door", attrs: { d: "M66.7 60.2L66.7 50A3.1 3.4 0 0 1 72.8 50L72.8 60.2Z" } },
  { tag: "path", part: "door", attrs: { d: "M75.5 61.6L75.5 50.9A2.8 3.5 0 0 1 81.1 50.9L81.1 61.6Z" } },
  { tag: "path", part: "door", attrs: { d: "M82.9 63.4L82.9 52.2A2.4 3.8 0 0 1 87.7 52.2L87.7 63.4Z" } },
  { tag: "path", part: "door", attrs: { d: "M88.5 65.6L88.5 53.6A1.9 4 0 0 1 92.3 53.6L92.3 65.6Z" } },
  { tag: "path", part: "door", attrs: { d: "M92.2 68.1L92.2 55.2A1.2 4.3 0 0 1 94.5 55.2L94.5 68.1Z" } },
  {
    tag: "path",
    part: "hero",
    attrs: {
      d:
        "M57.9 59.3L58.7 54C58.8 53.1 59.4 52.9 60.1 52.9C60.9 52.9 61.5 53.1 61.6 54L62.4 59.3ZM61.1 51.7" +
        "A1 1 0 1 0 59.2 51.7A1 1 0 1 0 61.1 51.7Z",
    },
  },
];

/* Craft a Card: the veil, the gears, the rails, the crystal, the anvil, the hammer, the sparks of its strike. */
const FORGE: readonly ArtShape[] = [
  { tag: "rect", part: "veil", attrs: { x: "0", y: "0", width: "100", height: "100" } },
  {
    tag: "path",
    part: "gear",
    attrs: {
      d:
        "M37.9 29.7L39.9 30L39.9 32L37.9 32.3L37.3 34L38.8 35.4L37.6 37.1L35.8 36.1L34.4 37.1L34.7 39.2L32.8 39.8" +
        "L31.9 37.9L30.1 37.9L29.2 39.8L27.3 39.2L27.6 37.1L26.2 36.1L24.4 37.1L23.2 35.4L24.7 34L24.1 32.3L22.1 32" +
        "L22.1 30L24.1 29.7L24.7 28L23.2 26.6L24.4 24.9L26.2 25.9L27.6 24.9L27.3 22.8L29.2 22.2L30.1 24.1L31.9 24.1" +
        "L32.8 22.2L34.7 22.8L34.4 24.9L35.8 25.9L37.6 24.9L38.8 26.6L37.3 28Z",
    },
  },
  {
    tag: "path",
    part: "gear",
    attrs: {
      d:
        "M74.3 25.7L75.9 26L75.9 28L74.3 28.3L73.6 29.8L74.6 31.2L73.2 32.6L71.8 31.6L70.3 32.3L70 33.9L68 33.9L67.7 32.3" +
        "L66.2 31.6L64.8 32.6L63.4 31.2L64.4 29.8L63.7 28.3L62.1 28L62.1 26L63.7 25.7L64.4 24.2L63.4 22.8L64.8 21.4" +
        "L66.2 22.4L67.7 21.7L68 20.1L70 20.1L70.3 21.7L71.8 22.4L73.2 21.4L74.6 22.8L73.6 24.2Z",
    },
  },
  { tag: "circle", part: "hub", attrs: { cx: "31", cy: "31", r: "2.6" } },
  { tag: "circle", part: "hub", attrs: { cx: "69", cy: "27", r: "2" } },
  { tag: "polygon", part: "rail", attrs: { points: "-4,76 40,61.4 40,63.2 -4,83" } },
  { tag: "polygon", part: "rail", attrs: { points: "104,76 60,61.4 60,63.2 104,83" } },
  { tag: "line", part: "rail-glow", attrs: { x1: "-4", y1: "79.5", x2: "40", y2: "62.3" } },
  { tag: "line", part: "rail-glow", attrs: { x1: "104", y1: "79.5", x2: "60", y2: "62.3" } },
  { tag: "polygon", part: "crystal", attrs: { points: "37,66 63,66 58,74 50,92 42,74" } },
  { tag: "path", part: "facet", attrs: { d: "M37 66L44 70L56 70L63 66M44 70L50 92M56 70L50 92M42 74L44 70M58 74L56 70" } },
  {
    tag: "path",
    part: "anvil",
    attrs: {
      d:
        "M31 57.2C35 56.4 37 56 40 56L61 56L61 58.6C58 58.8 56.5 59.6 55.5 61.2L56.5 63.8L59.5 65.6L40.5 65.6L43.5 63.8" +
        "L44.5 61.2C43.5 59.6 41.5 58.8 39 58.6C36 58.6 33.5 58.2 31 57.2Z",
    },
  },
  { tag: "rect", part: "hammer", attrs: { x: "48.6", y: "14", width: "2.8", height: "32" } },
  { tag: "rect", part: "hammer", attrs: { x: "42", y: "45.4", width: "16", height: "10.6", rx: "0.8" } },
  { tag: "polygon", part: "spark", attrs: { points: "38.5,50.5 39.3,54.2 43,55 39.3,55.8 38.5,59.5 37.7,55.8 34,55 37.7,54.2" } },
  { tag: "polygon", part: "spark", attrs: { points: "61.5,50.5 62.3,54.2 66,55 62.3,55.8 61.5,59.5 60.7,55.8 57,55 60.7,54.2" } },
];

/** R3402000: the scene of each look that has one, the four Core Mythics' own; "void", "ai" and "prismatic" have none. */
export const TAKEOVER_ART: Readonly<Partial<Record<TakeoverLook, readonly ArtShape[]>>> = {
  pawn: PAWN,
  genie: GENIE,
  rotunda: ROTUNDA,
  forge: FORGE,
};

/**
 * R3402000: `look`'s scene as an svg for the takeover's art layer, or null when the look has none. Each
 * shape is `fx-takeover-art-<part>`, with its place among its part's shapes (`--fx-art-i` of `--fx-art-n`)
 * for fx.css to stagger them by.
 */
export function buildTakeoverArt(doc: Document, look: TakeoverLook): SVGSVGElement | null {
  const shapes = TAKEOVER_ART[look];
  if (shapes === undefined) return null;
  const svg = doc.createElementNS(SVG_NS, "svg");
  svg.setAttribute("viewBox", TAKEOVER_ART_VIEWBOX);
  svg.setAttribute("preserveAspectRatio", TAKEOVER_ART_ASPECT);
  svg.setAttribute("aria-hidden", "true");
  const counts = new Map<string, number>();
  for (const shape of shapes) counts.set(shape.part, (counts.get(shape.part) ?? 0) + 1);
  const seen = new Map<string, number>();
  for (const shape of shapes) {
    const el = doc.createElementNS(SVG_NS, shape.tag);
    el.setAttribute("class", `fx-takeover-art-${shape.part}`);
    for (const name of ART_ATTRS) {
      const value = shape.attrs[name];
      if (value !== undefined) el.setAttribute(name, value);
    }
    const index = seen.get(shape.part) ?? 0;
    seen.set(shape.part, index + 1);
    el.setAttribute("style", `--fx-art-i: ${String(index)}; --fx-art-n: ${String(counts.get(shape.part) ?? 1)}`);
    svg.appendChild(el);
  }
  return svg;
}
