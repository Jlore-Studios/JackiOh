// R3400000: the art a Mythic's own takeover look draws over the dark, part by part (fx.css's
// `.takeover-art[data-look]`): Bob's Plan's ink-wash summit, Scratch Pad's blank canvas and Goetzmann
// Game Design's napkin. R3405000: and each Meditative Mythic's own: Reach the Summit's ascent, True Craft
// a Card's workshop, Aestheticize the Game's brush, Red Jade's mine, Jade Beauty's quarry, YileGPT's ai,
// YileGPT Prime's pavilion, Yile's Virus's virus, AI Girlfriend's dojo, Pareto Optimality's studyHall,
// The Cane's caneRack, Jlockeed's Evil Blueprints' warRoom and Jlockeed's Headquarters' fortress, each
// part named for its look. R3395001: and Stinkyscape's five, Tylannt's vault door, Rod of Ages' colossal
// rod, Great Paperclip Theory's wire, Paragon-X's white dwarf and Prototype-X2's forge. The takeover cue
// mounts them live (dom.ts), timed to its landing; Reduce Motion's still mounts the same parts at rest
// (game/TakeoverStill.tsx). Shapes only, never a card face (R202): True Craft's card is a blank card
// shape. A part is a class, `.takeover-art-<part>`, so each name is used by one look alone.
// R3399002: Classic+'s four Mythics' looks draw their art as a drawing of their own instead of parts
// (`TAKEOVER_ART_DRAWINGS`), mounted in the same art layer over the dark, live and still. The catalog's card
// art is procedural and ART_MANIFEST is empty, so each is drawn here by hand as paths on a 160 by 90 frame,
// after its card's concept:
// - "zealot" (Zephrys Zealotism): a robed wind-zealot looming from a lectern of cloud, a finger raised mid-lecture,
//   an indigo cyclone spinning across the frame;
// - "portal" (Portal to the Past): a carved stone ring grinding open on a sepia vortex, the frozen ruins of a past
//   battle drifting out of it toward the viewer;
// - "stair" (Twice Forward One Step Backwards): a vast stone stair turning on itself in a violet void cracked cyan;
// - "vineyard" (Mythic Grape): a luminous violet grape swelling over an ancient vineyard, its terraced hills of vines
//   spiralling into it.
// Each is a picture keyed by its look, never a card face, and no string here names a card (R202): the takeover plays
// only for a Mythic its viewer can read, and the DOM carries the look alone. A path's `part` is how fx.css moves it
// under the live takeover, by transform and opacity only (B55): "back" and "subject" hold still, "spin" turns and
// "drift" swells, each about the centre of its own box (`transform-box: fill-box`), so each moving path is drawn
// centred on the point it turns or swells from. The colors are the drawing's own, as TAKEOVER_TINTS (takeover.ts)
// are the looks'. Data only.
// R3402000: Core's four Mythics' looks draw a drawing of their own too (issue #3402), as a scene of shapes rather
// than coloured paths (a drawing's `shapes`), on a 100 by 100 frame sliced to cover the screen, each subject kept to
// the middle that both a phone's crop and a wide screen's show: My Pawn's "pawn", a hooded hand over a glass
// chessboard, its strings snapping taut down to the pieces; Zephrys's "genie", a wind spirit uncoiling in a cyclone
// of silk, chin on fist, as card backs orbit him; Heroic Power's "rotunda", a white marble rotunda of thirteen
// glowing doors, a hero in one; Craft a Card's "crystalForge", a floating crystal forge, two glowing rails converging
// on an anvil under a giant hammer, gears overhead. A shape carries only its geometry and its part, its class
// `takeover-drawing-<part>`: fx.css paints each part from the look's tint (`--fx-tint-*`, set by the takeover and by
// Reduce Motion's still) and moves the look's own parts under the live takeover by transform and opacity alone,
// staggering a part's shapes by their place among them (dom.ts); at most TAKEOVER_ART_MAX_SHAPES of them. No shape
// is text, a card's face or a card's name, so the scene names no card (R202).

import type { TakeoverLook } from "./types.ts";

/** Each look with art, its parts bottom to top; a look not listed draws none (R3399002: but may draw a drawing). */
export const TAKEOVER_ART_PARTS: Readonly<Partial<Record<TakeoverLook, readonly string[]>>> = {
  summit: ["wash", "range", "arrow", "flag"],
  canvas: ["paper", "peaks", "cursor", "rows"],
  napkin: ["napkin", "lamp", "pencil", "sketch", "spiral"],
  ascent: ["ascent-stars", "ascent-ledges", "ascent-climber", "ascent-clouds"],
  workshop: ["workshop-bench", "workshop-maker", "workshop-threads", "workshop-card"],
  brush: ["brush-scroll", "brush-wash", "brush-bloom", "brush-pen"],
  mine: ["mine-rock", "mine-seam", "mine-lamp", "mine-shards"],
  quarry: ["quarry-cliffs", "quarry-boulder", "quarry-veins", "quarry-dust"],
  ai: ["ai-scroll", "ai-scan", "ai-strokes"],
  pavilion: ["pavilion-hall", "pavilion-ring", "pavilion-scholar"],
  virus: ["virus-snow", "virus-face", "virus-eyes"],
  dojo: ["dojo-floor", "dojo-mat", "dojo-beams"],
  studyHall: ["hall-scroll", "hall-curve", "hall-students", "hall-cane"],
  caneRack: ["rack-lacquer", "rack-cane", "rack-hand"],
  warRoom: ["war-table", "war-villain", "war-blueprint", "war-lamps"],
  fortress: ["fortress-keep", "fortress-peak", "fortress-racks", "fortress-beacons"],
  vault: ["murk", "gap", "door", "dial"],
  rod: ["gulf", "colossus", "bandlight"],
  wire: ["blueprint", "strand", "stairs"],
  whiteDwarf: ["dwarf", "flare", "halo", "cut"],
  forge: ["sphere", "magma", "convoy", "hull"],
};

/** The parts `look`'s art draws, none for a look without art. */
export function takeoverArtParts(look: TakeoverLook): readonly string[] {
  return TAKEOVER_ART_PARTS[look] ?? [];
}

/** R3399002: how fx.css moves a path of a takeover's drawing: not at all ("back", "subject"), turning or swelling. */
export type TakeoverDrawingPart = "back" | "subject" | "spin" | "drift";

/** R3402000: the attributes a scene's shape may carry, spelled alike by React and the DOM: geometry only, its paint is fx.css's. */
export const ART_ATTRS = ["d", "cx", "cy", "r", "rx", "ry", "x", "y", "width", "height", "x1", "y1", "x2", "y2", "points", "transform"] as const;
export type ArtAttr = (typeof ART_ATTRS)[number];

/** R3402000: one shape of a scene; `part` names its class (`takeover-drawing-<part>`), which fx.css paints from the look's tint and moves. */
export type ArtShape = {
  tag: "path" | "circle" | "ellipse" | "rect" | "line" | "polygon";
  part: string;
  attrs: Readonly<Partial<Record<ArtAttr, string>>>;
};

/**
 * R3399002: a takeover's drawing, painted in order over a frame that fills the screen (`xMidYMid slice`): paths in
 * colours of their own, each moved as its `part` says; R3402000: or a scene's shapes, painted and moved by fx.css part by part.
 */
export type TakeoverDrawing =
  | { viewBox: string; paths: readonly { d: string; fill: string; part: TakeoverDrawingPart }[] }
  | { viewBox: string; shapes: readonly ArtShape[] };

/** Every drawing's frame: 16 by 9, so a landscape screen crops little and a phone keeps the middle. */
const VIEW_BOX = "0 0 160 90";

/** R3402000: a scene's frame: 100 by 100, so it covers the screen at any shape, its subject in the middle. */
const SCENE_VIEW_BOX = "0 0 100 100";

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
const CRYSTAL_FORGE: readonly ArtShape[] = [
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

/** R3399002: each look drawn as a picture of its own (R3402000: Core's four as scenes); a look not listed draws none (it may have parts). */
export const TAKEOVER_ART_DRAWINGS: Readonly<Partial<Record<TakeoverLook, TakeoverDrawing>>> = {
  zealot: {
    viewBox: VIEW_BOX,
    paths: [
      // The night sky.
      { part: "back", fill: "#1e1b4b", d: "M0 0H160V90H0Z" },
      // The glow behind the zealot.
      { part: "back", fill: "#312e81", d: "M40 38a40 34 0 1 0 80 0a40 34 0 1 0 -80 0Z" },
      // The cyclone's broad arms.
      { part: "spin", fill: "#4338ca", d: "M83.3 46C84.8 48.2 93.4 53.9 92.4 59.4C91.5 64.8 85.8 76 77.5 78.7C69.3 81.4 52.8 82.4 43.1 75.4C33.3 68.5 20.1 52.2 19.1 36.8C18.2 21.5 23.8 -3.6 37.3 -16.7C50.8 -29.8 77.9 -43.6 100 -41.8C122.2 -40.1 158.6 -12.1 170.3 -6.2L156 2.1C146.1 -2.8 115.3 -26.1 96.8 -27.5C78.2 -28.8 55.7 -17 44.5 -6.1C33.3 4.9 28.9 25.8 29.8 38.4C30.7 51.1 41.9 64.4 49.9 70C57.9 75.6 71.4 74.5 78 72.3C84.7 70 88.9 60.9 89.7 56.5C90.5 52.1 83.9 47.7 82.7 46ZM76.7 46C75.2 43.8 66.6 38.1 67.6 32.6C68.5 27.2 74.2 16 82.5 13.3C90.7 10.6 107.2 9.6 116.9 16.6C126.7 23.5 139.9 39.8 140.9 55.2C141.8 70.5 136.2 95.6 122.7 108.7C109.2 121.8 82.1 135.6 60 133.8C37.8 132.1 1.4 104.1 -10.3 98.1L4 89.8C13.9 94.8 44.7 118.1 63.2 119.5C81.8 120.8 104.3 109 115.5 98.1C126.7 87.1 131.1 66.2 130.2 53.6C129.3 40.9 118.1 27.6 110.1 22C102.1 16.4 88.6 17.5 82 19.7C75.3 22 71.1 31.1 70.3 35.5C69.5 39.9 76.1 44.3 77.3 46Z" },
      // The cyclone's bright streaks.
      { part: "spin", fill: "#818cf8", d: "M80 52.3C77.9 53.5 72.6 60 67.6 59.3C62.7 58.6 53.6 54.8 50.2 48.2C46.9 41.7 43.9 29.2 47.5 20.1C51.2 11 60.6 -1.9 72.1 -6.5C83.6 -11.1 102.7 -13.4 116.4 -7.4C130.1 -1.5 147.7 12.8 154.3 29C160.9 45.3 155.8 79.8 156 89.9L149.5 86.1C149.2 76.8 153.8 45.3 147.7 30.5C141.7 15.8 125.6 2.8 113.1 -2.5C100.6 -7.9 83.2 -5.7 72.8 -1.5C62.5 2.7 54 14.5 50.8 22.7C47.6 31 50.3 42.1 53.4 48C56.4 53.8 64.6 57.2 69 57.8C73.5 58.4 78.2 52.7 80 51.7ZM80 39.7C82.1 38.5 87.4 32 92.4 32.7C97.3 33.4 106.4 37.2 109.8 43.8C113.1 50.3 116.1 62.8 112.5 71.9C108.8 81 99.4 93.9 87.9 98.5C76.4 103.1 57.3 105.4 43.6 99.4C29.9 93.5 12.3 79.2 5.7 63C-0.9 46.7 4.2 12.2 4 2.1L10.5 5.9C10.8 15.2 6.2 46.7 12.3 61.5C18.3 76.2 34.4 89.2 46.9 94.5C59.4 99.9 76.8 97.7 87.2 93.5C97.5 89.3 106 77.5 109.2 69.3C112.4 61 109.7 49.9 106.6 44C103.6 38.2 95.4 34.8 91 34.2C86.5 33.6 81.8 39.3 80 40.3Z" },
      // Gusts of wind.
      { part: "drift", fill: "#c7d2fe", d: "M8 24Q30 17 52 22Q30 20 8 25.4ZM108 16Q130 11 152 18Q130 14 108 17.4ZM118 58Q136 52 156 56Q136 55 118 59.4ZM4 60Q22 55 40 58Q22 57.5 4 61.2Z" },
      // The lectern's cloud, behind.
      { part: "subject", fill: "#a5b4fc", d: "M28 90C22 84 26 74 36 75C36 66 48 61 56 66C60 57 74 55 80 62C86 55 100 57 104 66C112 61 124 66 124 75C134 74 138 84 132 90Z" },
      // The robed zealot.
      { part: "subject", fill: "#0b0a24", d: "M80 9C86 9 90 14 90 20C90 24 88.5 27 86.5 29C95 31 100 37 102 46L108 72H52L58 46C60 37 65 31 73.5 29C71.5 27 70 24 70 20C70 14 74 9 80 9Z" },
      // The arm raised mid-lecture, one finger up.
      { part: "subject", fill: "#0b0a24", d: "M92 33L103.5 19.5C103 16.5 105 14.5 106 15.5L106 8.6C106 7.4 107.6 7.4 107.6 8.6L107.8 16C109.6 16.4 110.2 19.4 108.6 21.6L99 40Z" },
      // The hood's rim of light.
      { part: "subject", fill: "#a5b4fc", d: "M73.5 13C70.5 16 70 23 73.5 28.5C72.2 23 72.5 17.5 75.5 12Z" },
      // The lectern's cloud, in front.
      { part: "subject", fill: "#e0e7ff", d: "M42 90C38 84 42 78 49 79C49 73 54 70 59 71C63 67 70 67 74 69C78 65 86 65 90 67.5C94 64 102 64 106 67C111 66 114 71 112 76C119 76 122 85 117 90Z" },
      // The spark at the fingertip.
      { part: "drift", fill: "#eef2ff", d: "M106.8 -0.5L107.9 3.4L111.8 4.5L107.9 5.6L106.8 9.5L105.7 5.6L101.8 4.5L105.7 3.4Z" },
    ],
  },
  portal: {
    viewBox: VIEW_BOX,
    paths: [
      // The dark.
      { part: "back", fill: "#1c1209", d: "M0 0H160V90H0Z" },
      // The ring's shadow.
      { part: "back", fill: "#3b2610", d: "M30 45a50 50 0 1 0 100 0a50 50 0 1 0 -100 0Z" },
      // The vortex's floor.
      { part: "back", fill: "#5c3a12", d: "M46 45a34 34 0 1 0 68 0a34 34 0 1 0 -68 0Z" },
      // The vortex's sepia arms.
      { part: "spin", fill: "#a16207", d: "M81.2 45.4C81.4 46.4 83.5 49.8 82.2 51.5C80.9 53.1 76.7 56 73.4 55.2C70.2 54.4 64.3 51.1 62.8 46.7C61.3 42.3 60.9 33.8 64.3 28.9C67.8 24.1 76.2 17.8 83.4 17.6C90.7 17.5 102.6 21.2 107.9 27.9C113.3 34.7 114.3 52.9 115.5 57.9L106.5 54.6C105.5 50.9 104.6 37.3 100.6 32.4C96.6 27.5 87.8 24.9 82.5 25C77.2 25.2 71.2 29.9 68.8 33.5C66.3 37 66.8 43.1 67.9 46.2C69.1 49.3 73.3 51.4 75.6 51.9C77.8 52.4 80.5 50.2 81.4 49.1C82.2 48 80.8 45.9 80.7 45.2ZM78.8 44.6C78.6 43.6 76.5 40.2 77.8 38.5C79.1 36.9 83.3 34 86.6 34.8C89.8 35.6 95.7 38.9 97.2 43.3C98.7 47.7 99.1 56.2 95.7 61.1C92.2 65.9 83.8 72.2 76.6 72.4C69.3 72.5 57.4 68.8 52.1 62.1C46.7 55.3 45.7 37.1 44.5 32.1L53.5 35.4C54.5 39.1 55.4 52.7 59.4 57.6C63.4 62.5 72.2 65.1 77.5 65C82.8 64.8 88.8 60.1 91.2 56.5C93.7 53 93.2 46.9 92.1 43.8C90.9 40.7 86.7 38.6 84.4 38.1C82.2 37.6 79.5 39.8 78.6 40.9C77.8 42 79.2 44.1 79.3 44.8Z" },
      // The vortex's pale arms.
      { part: "spin", fill: "#d6a35c", d: "M79.2 47.2C78.3 47.2 75.4 48.7 73.8 47.6C72.3 46.5 69.7 43.3 69.9 40.5C70.2 37.6 71.9 32.6 75.1 30.5C78.4 28.4 84.7 26.4 89.4 27.8C94 29.2 100.7 33.7 103.1 39.1C105.5 44.5 106.5 53.9 103.6 60.2C100.7 66.5 88.6 74 85.6 76.8L84.8 72.3C87.4 69.9 97.6 63.4 100.1 58C102.6 52.6 101.6 44.6 99.5 40C97.5 35.5 91.8 31.8 87.9 30.6C83.9 29.4 78.6 31.2 76 33C73.3 34.8 72 39 71.8 41.3C71.7 43.7 73.9 46.2 75.1 47.1C76.4 47.9 78.7 46.7 79.4 46.6ZM80.8 42.8C81.7 42.8 84.6 41.3 86.2 42.4C87.7 43.5 90.3 46.7 90.1 49.5C89.8 52.4 88.1 57.4 84.9 59.5C81.6 61.6 75.3 63.6 70.6 62.2C66 60.8 59.3 56.3 56.9 50.9C54.5 45.5 53.5 36.1 56.4 29.8C59.3 23.5 71.4 16 74.4 13.2L75.2 17.7C72.6 20.1 62.4 26.6 59.9 32C57.4 37.4 58.4 45.4 60.5 50C62.5 54.5 68.2 58.2 72.1 59.4C76.1 60.6 81.4 58.8 84 57C86.7 55.2 88 51 88.2 48.7C88.3 46.3 86.1 43.8 84.9 42.9C83.6 42.1 81.3 43.3 80.6 43.4Z" },
      // The light at its heart.
      { part: "drift", fill: "#fef3c7", d: "M74 45a6 6 0 1 0 12 0a6 6 0 1 0 -12 0Z" },
      // The carved stone ring.
      { part: "subject", fill: "#78716c", d: "M39 45a41 41 0 1 1 82 0a41 41 0 1 1 -82 0ZM47 45a33 33 0 1 0 66 0a33 33 0 1 0 -66 0Z" },
      // The ring's worn edge, lit.
      { part: "subject", fill: "#a8a29e", d: "M40.4 34.4A41 41 0 0 1 94 6.5L93.3 8.4A39 39 0 0 0 42.3 34.9Z" },
      // The ring's carved notches.
      { part: "subject", fill: "#44403c", d: "M113.4 42.7L120.4 42.9L120.4 47.1L113.4 47.3ZM110.1 59.7L116.1 63.4L114 67.1L107.8 63.7ZM98.7 72.8L102.1 79L98.4 81.1L94.7 75.1ZM82.3 78.4L82.1 85.4L77.9 85.4L77.7 78.4ZM65.3 75.1L61.6 81.1L57.9 79L61.3 72.8ZM52.2 63.7L46 67.1L43.9 63.4L49.9 59.7ZM46.6 47.3L39.6 47.1L39.6 42.9L46.6 42.7ZM49.9 30.3L43.9 26.6L46 22.9L52.2 26.3ZM61.3 17.2L57.9 11L61.6 8.9L65.3 14.9ZM77.7 11.6L77.9 4.6L82.1 4.6L82.3 11.6ZM94.7 14.9L98.4 8.9L102.1 11L98.7 17.2ZM107.8 26.3L114 22.9L116.1 26.6L110.1 30.3Z" },
      // Ruins: a broken column, a fallen arch, a toppled block.
      { part: "drift", fill: "#c9a26b", d: "M63.5 46V37.5L64.6 35.6L65.6 37L66.8 34.6L67.7 36.4V46ZM62.8 46H68.4V47.4H62.8ZM88 56Q93 47 99 51.5L97.6 53.4Q93.2 50.4 90.2 56.6ZM70 59L74 56L77.2 59.8L73.2 62.8Z" },
      // Ruins: a spear and a shield.
      { part: "drift", fill: "#e7d3a8", d: "M90 41L98.6 29.6L99.6 30.4L91 41.8ZM98.4 29.8L101.6 26.4L100.2 30.8ZM57.5 51a3 3 0 1 0 6 0a3 3 0 1 0 -6 0Z" },
    ],
  },
  stair: {
    viewBox: VIEW_BOX,
    paths: [
      // The violet void.
      { part: "back", fill: "#1a0b2e", d: "M0 0H160V90H0Z" },
      // The cracks' glow.
      { part: "back", fill: "#0e7490", d: "M-0.5 17.5L12.2 21.4L18.9 15.3L29.8 23.1L40.8 18.9L49.7 26.3L50.3 25.7L41.2 17.1L30.2 20.9L19.1 12.7L11.8 18.6L0.5 14.5ZM122.5 0.6L126.6 8.8L120.8 15.2L130.1 24.3L126.7 29.8L127.3 30.2L131.9 23.7L123.2 14.8L129.4 9.2L125.5 -0.6ZM160.5 72.5L147.8 68.6L141.1 74.7L130.2 66.9L119.2 71.1L110.3 63.7L109.7 64.3L118.8 72.9L129.8 69.1L140.9 77.3L148.2 71.4L159.5 75.5ZM37.5 89.4L33.4 81.2L39.2 74.8L29.9 65.7L33.3 60.2L32.7 59.8L28.1 66.3L36.8 75.2L30.6 80.8L34.5 90.6Z" },
      // The cyan cracks.
      { part: "back", fill: "#22d3ee", d: "M-0.2 16.6L12.1 20.5L18.9 14.5L29.9 22.4L40.9 18.4L49.9 26.1L50.1 25.9L41.1 17.6L30.1 21.6L19.1 13.5L11.9 19.5L0.2 15.4ZM123.5 0.2L127.5 8.9L121.6 15.1L130.6 24.1L126.9 29.9L127.1 30.1L131.4 23.9L122.4 14.9L128.5 9.1L124.5 -0.2ZM160.2 73.4L147.9 69.5L141.1 75.5L130.1 67.6L119.1 71.6L110.1 63.9L109.9 64.1L118.9 72.4L129.9 68.4L140.9 76.5L148.1 70.5L159.8 74.6ZM36.5 89.8L32.5 81.1L38.4 74.9L29.4 65.9L33.1 60.1L32.9 59.9L28.6 66.1L37.6 75.1L31.5 80.9L35.5 90.2Z" },
      // The steps' risers.
      { part: "spin", fill: "#4c1d95", d: "M87 46.8L96 46.8L95.2 51.7L86.7 49ZM89.9 50.9L99.4 54.6L96.2 60.2L88.2 53.8ZM90.6 56.9L99 64.5L92.8 69.6L87.1 59.7ZM88.1 63.6L93.8 74.8L84.8 77.7L82.8 65.3ZM82.2 69.3L83.7 83L72.7 82.4L75.6 68.9ZM73.5 72.4L69.6 86.9L58.2 81.8L66.4 69.3ZM63.2 71.7L53.7 84.8L43.8 74.9L56.9 65.4ZM53 66.4L38.4 75.9L32.1 61.7L48.9 57.2ZM45 56.6L26.8 60.5L25.9 43.5L44.4 45.5ZM73 46.8L64 46.8L64.8 41.9L73.3 44.6ZM70.1 43.3L60.6 39.6L63.8 34L71.8 40.4ZM69.4 37.9L61 30.3L67.2 25.2L72.9 35.1ZM71.9 31.8L66.2 20.6L75.2 17.7L77.2 30.1ZM77.8 26.7L76.3 13L87.3 13.6L84.4 27.1ZM86.5 24.2L90.4 9.7L101.8 14.8L93.6 27.3ZM96.8 25.5L106.3 12.4L116.2 22.3L103.1 31.8ZM107 31.4L121.6 21.9L127.9 36.1L111.1 40.6ZM115 41.8L133.2 37.9L134.1 54.9L115.6 52.9Z" },
      // The steps' treads, a stair turning on itself.
      { part: "spin", fill: "#8b5cf6", d: "M87 45L96 45L95.2 49.9L86.7 47.2ZM89.9 48.8L99.4 52.5L96.2 58.1L88.2 51.7ZM90.6 54.5L99 62.1L92.8 67.2L87.1 57.3ZM88.1 60.9L93.8 72.1L84.8 75L82.8 62.6ZM82.2 66.3L83.7 80L72.7 79.4L75.6 65.9ZM73.5 69.1L69.6 83.6L58.2 78.5L66.4 66ZM63.2 68.1L53.7 81.2L43.8 71.3L56.9 61.8ZM53 62.5L38.4 72L32.1 57.8L48.9 53.3ZM45 52.4L26.8 56.3L25.9 39.3L44.4 41.3ZM73 45L64 45L64.8 40.1L73.3 42.8ZM70.1 41.2L60.6 37.5L63.8 31.9L71.8 38.3ZM69.4 35.5L61 27.9L67.2 22.8L72.9 32.7ZM71.9 29.1L66.2 17.9L75.2 15L77.2 27.4ZM77.8 23.7L76.3 10L87.3 10.6L84.4 24.1ZM86.5 20.9L90.4 6.4L101.8 11.5L93.6 24ZM96.8 21.9L106.3 8.8L116.2 18.7L103.1 28.2ZM107 27.5L121.6 18L127.9 32.2L111.1 36.7ZM115 37.6L133.2 33.7L134.1 50.7L115.6 48.7Z" },
      // The stair's newel.
      { part: "subject", fill: "#2e1065", d: "M74 45a6 6 0 1 0 12 0a6 6 0 1 0 -12 0Z" },
      // Its cyan core.
      { part: "subject", fill: "#67e8f9", d: "M77.6 45a2.4 2.4 0 1 0 4.8 0a2.4 2.4 0 1 0 -4.8 0Z" },
      // Loose stones.
      { part: "drift", fill: "#a78bfa", d: "M18 60L25 58L27 63L20 65ZM26 26L31 24L33 28L28 30ZM58 8L63 7L64 11L59 12ZM142 30L135 32L133 27L140 25ZM134 64L129 66L127 62L132 60ZM102 82L97 83L96 79L101 78Z" },
      // Cyan shards.
      { part: "drift", fill: "#a5f3fc", d: "M46 40L49 38L48 42ZM70 16L73 15L71 19ZM14 44L17 43L15 47ZM114 50L111 52L112 48ZM90 74L87 75L89 71ZM146 46L143 47L145 43Z" },
    ],
  },
  vineyard: {
    viewBox: VIEW_BOX,
    paths: [
      // The night sky.
      { part: "back", fill: "#1e0b36", d: "M0 0H160V90H0Z" },
      // The far hills.
      { part: "back", fill: "#2d1b4e", d: "M0 58C18 50 34 52 50 56C66 60 78 50 96 50C114 50 128 58 160 52V90H0Z" },
      // The near hills.
      { part: "back", fill: "#1a2e05", d: "M0 70C20 62 42 66 62 70C82 74 104 62 124 64C140 66 150 68 160 66V90H0Z" },
      // The terraces of vines.
      { part: "spin", fill: "#3f6212", d: "M92.4 45.1C92.3 48.4 95.6 58.9 92.1 64.9C88.6 70.9 80.4 79 71.3 81.2C62.1 83.3 47.7 83.4 37.1 77.8C26.6 72.1 13.4 60.6 7.8 47C2.2 33.5 -1.1 13.1 3.6 -3.6C8.3 -20.3 20 -41.2 36.1 -53.1C52.2 -65.1 89.4 -71.8 100 -75.5L96.1 -53.3C87.5 -50.3 57.7 -44.8 44.8 -35.1C31.9 -25.4 22.6 -8.7 18.9 4.7C15.2 18.1 17.9 34.4 22.4 45.2C26.8 56.1 37.3 65.2 45.8 69.8C54.2 74.3 65.6 74.3 73 72.7C80.4 71.1 86.8 64.7 90 60.1C93.1 55.5 91.5 47.4 91.9 44.9ZM67.6 30.8C67.7 27.6 64.4 17.1 67.9 11.1C71.4 5.1 79.6 -3 88.7 -5.2C97.9 -7.3 112.3 -7.4 122.9 -1.8C133.4 3.9 146.6 15.4 152.2 29C157.8 42.5 161.1 62.9 156.4 79.6C151.7 96.3 140 117.2 123.9 129.1C107.8 141.1 70.6 147.8 60 151.5L63.9 129.3C72.5 126.3 102.3 120.8 115.2 111.1C128.1 101.4 137.4 84.7 141.1 71.3C144.8 57.9 142.1 41.6 137.6 30.8C133.2 19.9 122.7 10.8 114.2 6.2C105.8 1.7 94.4 1.7 87 3.3C79.6 4.9 73.2 11.3 70 15.9C66.9 20.5 68.5 28.6 68.1 31.1Z" },
      // The vine rows.
      { part: "spin", fill: "#65a30d", d: "M84.2 53.7C81.9 55.6 76.7 63.6 70.5 65C64.4 66.5 54.4 66.4 47.1 62.2C39.9 58.1 30.8 49.8 27.1 40C23.4 30.2 21.2 15.5 24.9 3.4C28.6 -8.7 37.2 -23.9 49.1 -32.7C61.1 -41.6 79.5 -49.5 96.6 -49.7C113.7 -49.8 142.5 -36.3 151.6 -33.6L147 -29C138.4 -31.4 111.5 -44 95.5 -43.9C79.5 -43.7 62.4 -36.2 51.2 -28C40.1 -19.7 32.1 -5.6 28.7 5.7C25.2 17 27.3 30.7 30.7 39.8C34.2 49 42.7 56.7 49.4 60.6C56.1 64.5 65.4 64.5 71.2 63.3C76.9 62 81.9 54.9 84.1 53.2ZM75.8 22.3C78.1 20.4 83.3 12.4 89.5 11C95.6 9.5 105.6 9.6 112.9 13.8C120.1 17.9 129.2 26.2 132.9 36C136.6 45.8 138.8 60.5 135.1 72.6C131.4 84.7 122.8 99.9 110.9 108.7C98.9 117.6 80.5 125.5 63.4 125.7C46.3 125.8 17.5 112.3 8.4 109.6L13 105C21.6 107.4 48.5 120 64.5 119.9C80.5 119.7 97.6 112.2 108.8 104C119.9 95.7 127.9 81.6 131.3 70.3C134.8 59 132.7 45.3 129.3 36.2C125.8 27 117.3 19.3 110.6 15.4C103.9 11.5 94.6 11.5 88.8 12.7C83.1 14 78.1 21.1 75.9 22.8Z" },
      // The grape's far glow.
      { part: "drift", fill: "#3b1466", d: "M53 38a27 27 0 1 0 54 0a27 27 0 1 0 -54 0Z" },
      // The grape's near glow.
      { part: "drift", fill: "#7e22ce", d: "M60 38a20 20 0 1 0 40 0a20 20 0 1 0 -40 0Z" },
      // The grape.
      { part: "drift", fill: "#a855f7", d: "M65 38a15 15 0 1 0 30 0a15 15 0 1 0 -30 0Z" },
      // Its shade.
      { part: "drift", fill: "#9333ea", d: "M94.1 43.1A15 15 0 0 1 70.4 49.5A22 22 0 0 0 94.1 43.1Z" },
      // Its shine.
      { part: "drift", fill: "#f3e8ff", d: "M69.1 33.3a4.5 3 -30 1 1 7.8 -4.5a4.5 3 -30 1 1 -7.8 4.5Z" },
      // Its stem.
      { part: "subject", fill: "#3f6212", d: "M80 24C80 20 81 17 84 14L85.4 15C83 17.6 82 20.4 82 24Z" },
      // Its leaf.
      { part: "subject", fill: "#65a30d", d: "M84 15C88 10 94 10 97 12C94 16 88 17 84 15Z" },
    ],
  },
  // R3402000: Core's four Mythics' scenes.
  pawn: { viewBox: SCENE_VIEW_BOX, shapes: PAWN },
  genie: { viewBox: SCENE_VIEW_BOX, shapes: GENIE },
  rotunda: { viewBox: SCENE_VIEW_BOX, shapes: ROTUNDA },
  crystalForge: { viewBox: SCENE_VIEW_BOX, shapes: CRYSTAL_FORGE },
};
