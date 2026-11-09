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

/** R3399002: a takeover's drawing, painted in order over a frame that fills the screen (`xMidYMid slice`). */
export type TakeoverDrawing = { viewBox: string; paths: readonly { d: string; fill: string; part: TakeoverDrawingPart }[] };

/** Every drawing's frame: 16 by 9, so a landscape screen crops little and a phone keeps the middle. */
const VIEW_BOX = "0 0 160 90";

/** R3399002: each look drawn as a picture of its own; a look not listed draws none (it may have parts). */
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
};
