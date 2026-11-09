// R3403000: Stinkyscape's later scene recipes, part 2 of three (#120 to #159 with #135's, #137's, #147's, #148's and #150's tokens), one per entry of
// #3403's table, each drawn from its brief in Stinkyscape's deep-space look (ART.md, "Stinkyscape's later cards drawn
// as scenes"): its subject within x 30 to 70 and y 25 to 85, its foreground near, its middle in the mid and subject
// layers and its background far, richer the rarer. Presentation only (CLAUDE.md rule 7): a recipe is read by the
// card's id alone.

import { figureLookFor, type FigureLook } from "./figureKit.ts";
import type { SceneRecipe } from "./scene.ts";

/** The look every card of this file starts from but the Humans: the hooded traveller. */
const BASE: FigureLook = figureLookFor([]);
/** The look a Human card of this file starts from. */
const HUMAN: FigureLook = figureLookFor(["Human"]);
/** The look a Jlockeed card of this file starts from: Jlockeed's visored people in their suits. */
const JLOCKEED: FigureLook = figureLookFor(["Jlockeed"]);

/** Wild System (Stinkyscape #120, Epic): a sparking, unplugged control console in a decommissioned robot depot, a free-for-all brawl of robots, drones and fighters behind it, one dented survivor mech standing atop the scrap pile, in flickering red and green light. */
const WILD_SYSTEM: SceneRecipe = {
  family: "stinkyscape",
  sky: ["black", "slate-900"],
  light: { x: 50, y: 34, r: 38, paint: "red-500", opacity: 0.3 },
  emblem: { x: 22, y: 80, size: 5, rotate: 0 },
  layers: {
    far: [
      { part: "room-wall", x: 50, y: 62, paint: "slate-800" },
      { part: "scrap-heap", x: 50, y: 50, paint: "slate-700" },
      { part: "mech", x: 52, y: 46, scale: 0.6, paint: "slate-400" },
      { part: "crate", x: 86, y: 62, scale: 1.2, count: 2, spread: [5, 0] },
    ],
    mid: [
      { part: "tile-floor", x: 50, y: 64, paint: "slate-800" },
      { part: "crate", x: 12, y: 78, scale: 1.3, paint: "slate-700" },
    ],
    subject: [
      { part: "robot", x: 40, y: 80, scale: 1.3 },
      { part: "robot", x: 58, y: 80, scale: 1.2, flip: -1, paint: "red-600" },
      { part: "drone", x: 50, y: 56, scale: 1.1 },
      { part: "fighter", x: 32, y: 46, scale: 0.9 },
      { part: "fighter", x: 70, y: 44, scale: 0.8, flip: -1, paint: "jade-400" },
      { part: "zap", x: 44, y: 66, scale: 0.4, paint: "green-400" },
    ],
    near: [
      { part: "console", x: 22, y: 98, scale: 1.4 },
      { part: "cable", x: 34, y: 94, scale: 0.8 },
    ],
    light: [
      { part: "halo", x: 52, y: 36, scale: 1.8, paint: "red-500" },
      { part: "halo", x: 48, y: 70, scale: 1.8, paint: "green-400" },
      { part: "rim-light", x: 52, y: 32, scale: 1, paint: "green-400" },
    ],
    particles: [
      { part: "sparks", x: 30, y: 84, scale: 1.2, count: 3, spread: [6, 6], paint: "green-400" },
      { part: "sparks", x: 50, y: 64, scale: 1, count: 2, spread: [14, 10], paint: "red-400" },
    ],
  },
};

/** System Schematic (Stinkyscape #121, Epic): an engineer's gloved hand hovers over four glowing blueprints on a drafting table, holograms of a reset lever, a blast shutter, a broken firewall and a brawl rising from them, a drafting office window on a cobalt nebula behind; the brawl is two faint robots. */
const SYSTEM_SCHEMATIC: SceneRecipe = {
  family: "stinkyscape",
  sky: ["black", "blue-900"],
  light: { x: 50, y: 62, r: 36, paint: "cyan-400", opacity: 0.3 },
  emblem: { x: 64, y: 80, size: 5, rotate: 0 },
  layers: {
    sky: [
      { part: "stars", x: 50, y: 24, scale: 2 },
      { part: "nebula", x: 44, y: 28, scale: 1.4, paint: "blue-600" },
    ],
    far: [
      { part: "room-wall", x: 50, y: 72, paint: "slate-800" },
      { part: "viewport", x: 50, y: 30, scale: 1.9, paint: "slate-400" },
    ],
    mid: [
      { part: "reset-lever", x: 24, y: 64, scale: 0.9, paint: "cyan-400", fade: 0.6 },
      { part: "blast-door", x: 41, y: 62, scale: 0.6, paint: "sky-300", fade: 0.6 },
      { part: "energy-wall", x: 60, y: 64, scale: 0.55, paint: "cyan-400", fade: 0.6 },
      { part: "robot", x: 74, y: 62, scale: 0.8, fade: 0.55, paint: "sky-300" },
      { part: "robot", x: 82, y: 62, scale: 0.7, flip: -1, fade: 0.55, paint: "sky-300" },
    ],
    subject: [
      { part: "wide-table", x: 50, y: 100, scale: 2.8, paint: "slate-700" },
      { part: "holo-blueprint", x: 30, y: 74, scale: 0.6 },
      { part: "holo-blueprint", x: 44, y: 73, scale: 0.6 },
      { part: "holo-blueprint", x: 58, y: 74, scale: 0.6 },
      { part: "holo-blueprint", x: 72, y: 73, scale: 0.6 },
    ],
    near: [
      { part: "gloved-hand", x: 16, y: 62, scale: 1.1 },
    ],
    light: [
      { part: "light-shaft", x: 30, y: 74, scale: 0.5, paint: "cyan-400" },
      { part: "light-shaft", x: 72, y: 73, scale: 0.5, paint: "cyan-400" },
      { part: "halo", x: 50, y: 70, scale: 2.4, paint: "cyan-400" },
      { part: "rim-light", x: 30, y: 60, scale: 1, paint: "sky-300" },
    ],
    particles: [
      { part: "motes", x: 50, y: 60, scale: 1.4, paint: "cyan-400" },
      { part: "shards", x: 62, y: 60, scale: 0.6 },
    ],
  },
};

/** Gokugains (Stinkyscape #123, Rare): a muscle-bound grinning smuggler in a sleeveless grey vest flexes in a shadowy cargo hold, siphoning a faint glow from a sealed crate beside him. */
const GOKUGAINS: SceneRecipe = {
  family: "stinkyscape",
  sky: ["black", "wood-900"],
  light: { x: 66, y: 66, r: 26, paint: "cyan-400", opacity: 0.32 },
  emblem: { x: 44, y: 54, size: 5, rotate: 0 },
  figure: {
    x: 44,
    y: 86,
    scale: 1,
    pose: "pose-flex",
    look: { ...BASE, hair: "hair-short", outfit: "outfit-sleeveless", item: null, skin: "brass-600", cloth: "grey-500" },
  },
  layers: {
    far: [
      { part: "room-wall", x: 50, y: 72, paint: "slate-800" },
      { part: "cargo-net", x: 16, y: 72, scale: 1, fade: 0.8 },
      { part: "crate", x: 84, y: 72, scale: 1.4, count: 3, spread: [8, 0] },
    ],
    mid: [
      { part: "tile-floor", x: 50, y: 72, paint: "slate-900" },
      { part: "crate", x: 20, y: 82, scale: 1.6 },
    ],
    subject: [{ part: "crate", x: 66, y: 86, scale: 1.8, paint: "slate-700" }],
    near: [
      { part: "crate", x: 90, y: 98, scale: 2 },
      { part: "cable", x: 4, y: 94, scale: 0.8 },
    ],
    light: [
      { part: "halo", x: 66, y: 74, scale: 1.6, paint: "cyan-400" },
      { part: "rim-light", x: 46, y: 62, scale: 1.2, paint: "cyan-400" },
      { part: "light-shaft", x: 44, y: 6, scale: 1, paint: "orange-400" },
    ],
    particles: [
      { part: "motes", x: 58, y: 66, scale: 0.6, count: 3, spread: [6, 6], paint: "cyan-400" },
      { part: "sparks", x: 56, y: 40, scale: 0.8, paint: "cyan-400" },
      { part: "dust", x: 44, y: 88, scale: 1.2, paint: "slate-400" },
    ],
  },
};

/** Jlockeed's Shipbuilding Grant (Stinkyscape #124, Common): a sealed grant capsule drops into a bustling Jlockeed shipyard as a crane swings a fresh hull section toward an unfinished rocket, under bright work lamps and sparks; the hull section is a ship module. */
const JLOCKEEDS_SHIPBUILDING_GRANT: SceneRecipe = {
  family: "stinkyscape",
  sky: ["slate-900", "blue-800"],
  light: { x: 50, y: 22, r: 34, paint: "orange-400", opacity: 0.3 },
  emblem: { x: 52, y: 70, size: 5, rotate: 0 },
  layers: {
    sky: [{ part: "stars", x: 50, y: 18, scale: 2, fade: 0.6 }],
    far: [
      { part: "shipyard-crane", x: 82, y: 82, scale: 1.4, flip: -1 },
      { part: "gantry", x: 32, y: 84, scale: 1.4 },
    ],
    mid: [{ part: "tile-floor", x: 50, y: 80, paint: "slate-800" }],
    subject: [
      { part: "rocket", x: 50, y: 84, scale: 1.3 },
      { part: "ship-module", x: 62, y: 44, scale: 0.9 },
      { part: "capsule", x: 38, y: 30, scale: 0.9, flip: -1 },
    ],
    light: [
      { part: "spotlight", x: 20, y: 2, scale: 1, paint: "orange-400" },
      { part: "halo", x: 38, y: 30, scale: 1, paint: "white" },
    ],
    particles: [
      { part: "sparks", x: 52, y: 60, scale: 1.2, count: 3, spread: [6, 10], paint: "orange-400" },
      { part: "speed-streaks", x: 38, y: 26, scale: 0.4, flip: -1, paint: "white" },
    ],
  },
};

/** Jlockeed's Spacerace Propaganda (Stinkyscape #125, Rare): a space-age parade plaza lined with wordless posters of astronauts saluting rockets, a float rocket rolling past on its cart, ticker tape drifting, under a cobalt sky; the posters are recruiting posters, the onlookers a crowd. */
const JLOCKEEDS_SPACERACE_PROPAGANDA: SceneRecipe = {
  family: "stinkyscape",
  sky: ["blue-900", "blue-700"],
  light: { x: 50, y: 30, r: 40, paint: "sky-300", opacity: 0.28 },
  emblem: { x: 50, y: 74, size: 5, rotate: 0 },
  layers: {
    sky: [
      { part: "stars", x: 50, y: 14, scale: 2, fade: 0.5 },
      { part: "planet", x: 80, y: 18, scale: 0.8, paint: "blue-200" },
    ],
    far: [
      { part: "skyline", x: 50, y: 62, scale: 1, paint: "slate-700" },
      { part: "banner", x: 8, y: 66, scale: 1.4, paint: "blue-600" },
      { part: "banner", x: 92, y: 66, scale: 1.4, flip: -1, paint: "blue-600" },
    ],
    mid: [
      { part: "tile-floor", x: 50, y: 64, paint: "slate-400" },
      { part: "poster", x: 18, y: 72, scale: 1.6 },
      { part: "poster", x: 32, y: 68, scale: 1.2 },
      { part: "poster", x: 68, y: 68, scale: 1.2 },
      { part: "poster", x: 82, y: 72, scale: 1.6 },
      { part: "bunting", x: 50, y: 50, scale: 1.4 },
    ],
    subject: [
      { part: "rocket-cart", x: 50, y: 86, scale: 1.4 },
      { part: "rocket", x: 50, y: 78, scale: 1 },
    ],
    near: [{ part: "crew-crowd", x: 50, y: 100, scale: 2.2 }],
    light: [
      { part: "god-rays", x: 50, y: 0, scale: 1, paint: "sky-300" },
      { part: "rim-light", x: 50, y: 56, scale: 1.2, paint: "white" },
    ],
    particles: [
      { part: "ticker-tape", x: 50, y: 40, scale: 1.4 },
      { part: "motes", x: 50, y: 40, scale: 1.4, paint: "white" },
    ],
  },
};

/** Jlockeed's Permanent Procurement Lobbying (Stinkyscape #126, Epic): a smiling lobbyist in a sharp grey suit slides a velvet case across a senator's desk, through the window a drone delivering a fresh starship part at dawn and delivery ships queueing over a capital planet's skyline; the velvet case is a small chest. */
const JLOCKEEDS_PERMANENT_PROCUREMENT_LOBBYING: SceneRecipe = {
  family: "stinkyscape",
  sky: ["violet-700", "orange-400"],
  light: { x: 50, y: 54, r: 38, paint: "orange-400", opacity: 0.34 },
  emblem: { x: 42, y: 78, size: 5, rotate: 0 },
  figure: {
    x: 36,
    y: 94,
    scale: 0.92,
    pose: "pose-hold",
    look: { ...JLOCKEED, hair: "hair-short", item: null, skin: "red-300", cloth: "grey-500" },
  },
  extras: [
    {
      layer: "subject",
      x: 66,
      y: 94,
      scale: 0.9,
      flip: -1,
      pose: "pose-stand",
      look: { ...BASE, hair: "pompadour", outfit: "outfit-blazer", item: null, skin: "wood-700", cloth: "blue-900" },
    },
  ],
  layers: {
    sky: [{ part: "sun-disc", x: 50, y: 56, scale: 1.4, paint: "orange-400" }],
    far: [
      { part: "city-skyline", x: 50, y: 70, scale: 0.7, paint: "slate-800" },
      { part: "shuttle", x: 30, y: 24, scale: 0.55 },
      { part: "freighter", x: 72, y: 26, scale: 0.3, flip: -1 },
    ],
    mid: [
      { part: "drone", x: 62, y: 42, scale: 0.9 },
      { part: "ship-module", x: 62, y: 52, scale: 0.6 },
      { part: "viewport", x: 50, y: 44, scale: 2.2, paint: "slate-700" },
    ],
    near: [
      { part: "wide-table", x: 50, y: 102, scale: 2.4, paint: "wood-700" },
      { part: "chest", x: 52, y: 74, scale: 0.5, paint: "violet-700" },
    ],
    light: [
      { part: "god-rays", x: 50, y: 56, scale: 0.8, paint: "orange-400" },
      { part: "halo", x: 50, y: 56, scale: 2, paint: "orange-400" },
      { part: "rim-light", x: 36, y: 70, scale: 1, flip: -1, paint: "orange-400" },
    ],
    particles: [{ part: "sparks", x: 52, y: 72, scale: 0.6, paint: "white" }],
  },
};

/** The welders of Jlockeed's Starship Retrofitting Program. */
const WELDER: FigureLook = { ...JLOCKEED, hair: "goggles", outfit: "outfit-overalls", item: "item-torch", skin: "brass-600", cloth: "orange-400" };

/** Jlockeed's Starship Retrofitting Program (Stinkyscape #130, Epic): welders bolt fresh fins onto an old Jlockeed tank in an orbital body shop, paint drones spraying it cobalt and a crane lowering a rocket booster onto its back, in red-orange welding light; the fins are steel shards. */
const JLOCKEEDS_STARSHIP_RETROFITTING_PROGRAM: SceneRecipe = {
  family: "stinkyscape",
  sky: ["black", "red-900"],
  light: { x: 50, y: 66, r: 36, paint: "orange-400", opacity: 0.4 },
  emblem: { x: 44, y: 78, size: 5, rotate: 0 },
  extras: [
    { layer: "near", x: 24, y: 94, scale: 0.7, pose: "pose-kneel", look: WELDER },
    { layer: "near", x: 76, y: 94, scale: 0.72, flip: -1, pose: "pose-hold", look: { ...WELDER, hair: "face-mask", cloth: "slate-700" } },
  ],
  layers: {
    far: [
      { part: "shipyard-crane", x: 76, y: 66, scale: 1.2, flip: -1 },
      { part: "rocket", x: 50, y: 56, scale: 0.7 },
    ],
    mid: [
      { part: "tile-floor", x: 50, y: 72, paint: "slate-800" },
      { part: "drone", x: 32, y: 54, scale: 1, paint: "blue-700" },
      { part: "drone", x: 68, y: 52, scale: 1, flip: -1, paint: "blue-700" },
      { part: "spray", x: 36, y: 62, scale: 1.2, paint: "blue-600" },
    ],
    subject: [
      { part: "tank", x: 50, y: 86, scale: 2, paint: "blue-700" },
      { part: "crystal", x: 36, y: 72, scale: 1.4, paint: "slate-400" },
      { part: "crystal", x: 64, y: 72, scale: 1.4, paint: "slate-400" },
    ],
    near: [{ part: "cable", x: 90, y: 96, scale: 0.8, flip: -1 }],
    light: [
      { part: "halo", x: 36, y: 74, scale: 1.2, paint: "orange-400" },
      { part: "halo", x: 64, y: 74, scale: 1.2, paint: "orange-400" },
      { part: "rim-light", x: 50, y: 74, scale: 2, paint: "ember-500" },
    ],
    particles: [
      { part: "sparks", x: 36, y: 76, scale: 1, paint: "orange-400" },
      { part: "sparks", x: 64, y: 76, scale: 1, paint: "orange-400" },
      { part: "embers", x: 50, y: 70, scale: 1.4 },
    ],
  },
};

/** Sacked (Stinkyscape #135, Common): a burlap sack drops from a cargo hatch over a freshly arrived alien trooper, its drawstring cinching shut above his kicking boots, under a loading-bay lamp; the cargo hatch is an open airlock overhead. */
const SACKED: SceneRecipe = {
  family: "stinkyscape",
  sky: ["black", "slate-900"],
  light: { x: 50, y: 18, r: 30, paint: "brass-500", opacity: 0.36 },
  emblem: { x: 78, y: 70, size: 5, rotate: 0 },
  layers: {
    far: [
      { part: "room-wall", x: 50, y: 72, paint: "slate-800" },
      { part: "airlock", x: 50, y: 14, scale: 1.1 },
      { part: "crate", x: 82, y: 74, scale: 1.4, count: 2, spread: [6, 0] },
    ],
    mid: [{ part: "tile-floor", x: 50, y: 74, paint: "slate-700" }],
    subject: [
      { part: "alien", x: 50, y: 86, scale: 1.4 },
      { part: "kicking-sack", x: 50, y: 76, scale: 1.8 },
    ],
    light: [
      { part: "spotlight", x: 50, y: 20, scale: 1.1, paint: "brass-500" },
      { part: "halo", x: 50, y: 60, scale: 1.4, paint: "brass-500" },
    ],
    particles: [
      { part: "dust", x: 50, y: 88, scale: 1.1, paint: "slate-400" },
      { part: "motes", x: 50, y: 50, scale: 1.2, paint: "brass-500" },
    ],
  },
};

/** Sack (Stinkyscape #135.1, Token): a lumpy burlap sack thrashes on a station's lost-and-found counter, a clawed foot poking out of the drawstring and dust puffing, in dim greenish work light; the shelf is a counter with a lost hat and helmet beside it. */
const SACK: SceneRecipe = {
  family: "stinkyscape",
  sky: ["black", "green-900"],
  light: { x: 50, y: 26, r: 30, paint: "green-400", opacity: 0.22 },
  emblem: { x: 36, y: 78, size: 5, rotate: 0 },
  layers: {
    far: [
      { part: "room-wall", x: 50, y: 70, paint: "slate-800" },
      { part: "shelf", x: 16, y: 56, scale: 0.9, paint: "slate-700" },
      { part: "shelf", x: 84, y: 56, scale: 0.9, paint: "slate-700" },
    ],
    mid: [
      { part: "tile-floor", x: 50, y: 86, paint: "slate-900" },
      { part: "counter", x: 50, y: 88, scale: 1.4, paint: "slate-700" },
    ],
    subject: [
      { part: "kicking-sack", x: 50, y: 74, scale: 2.2 },
      { part: "hat", x: 26, y: 74, scale: 1 },
      { part: "helmet", x: 74, y: 74, scale: 1.2 },
    ],
    light: [
      { part: "lantern", x: 50, y: 8, scale: 1.2 },
      { part: "rim-light", x: 50, y: 60, scale: 1.2, paint: "green-400" },
    ],
    particles: [
      { part: "dust-cloud", x: 58, y: 72, scale: 1 },
      { part: "motes", x: 50, y: 50, scale: 1.2, paint: "green-200" },
    ],
  },
};

/** Starfleet Commander (Stinkyscape #137, Legendary): a grey-haired officer in a navy greatcoat stands on a launch gantry, one arm raised to signal, as a starship's engines ignite behind her and tear it free of its dock while four doctrine banners for land, sea, air and space snap in the exhaust, in blue-white engine glare. */
const STARFLEET_COMMANDER: SceneRecipe = {
  family: "stinkyscape",
  sky: ["black", "blue-900"],
  light: { x: 30, y: 36, r: 40, paint: "blue-200", opacity: 0.45 },
  emblem: { x: 48, y: 56, size: 5, rotate: 0 },
  figure: {
    x: 48,
    y: 86,
    scale: 1,
    pose: "pose-raise",
    look: { ...HUMAN, hair: "hair-silver", outfit: "outfit-greatcoat", item: null, skin: "wood-700", cloth: "blue-900" },
  },
  layers: {
    sky: [
      { part: "stars", x: 50, y: 18, scale: 2.2 },
      { part: "nebula", x: 74, y: 22, scale: 1.1, paint: "blue-600" },
    ],
    far: [
      { part: "starship", x: 58, y: 34, scale: 1.1 },
      { part: "thruster-flame", x: 31, y: 36, scale: 1.4, paint: "sky-300" },
      { part: "gantry", x: 88, y: 64, scale: 1.2, flip: -1 },
      { part: "wreckage", x: 80, y: 46, scale: 0.4 },
    ],
    mid: [
      { part: "tile-floor", x: 50, y: 84, paint: "slate-700" },
      { part: "gantry", x: 22, y: 86, scale: 1.1 },
      { part: "gantry", x: 76, y: 86, scale: 1.1, flip: -1 },
    ],
    near: [
      { part: "banner", x: 8, y: 100, scale: 1.3, paint: "brass-600" },
      { part: "banner", x: 24, y: 98, scale: 1.1, paint: "cyan-400" },
      { part: "banner", x: 76, y: 98, scale: 1.1, flip: -1, paint: "sky-300" },
      { part: "banner", x: 92, y: 100, scale: 1.3, flip: -1, paint: "violet-500" },
    ],
    light: [
      { part: "lens-flare", x: 30, y: 36, scale: 0.9 },
      { part: "halo", x: 30, y: 36, scale: 2.2, paint: "blue-200" },
      { part: "rim-light", x: 48, y: 60, scale: 1.2, flip: -1, paint: "blue-200" },
    ],
    particles: [
      { part: "speed-streaks", x: 26, y: 36, scale: 0.8, flip: -1, paint: "white" },
      { part: "sparks", x: 30, y: 40, scale: 1.2, count: 2, spread: [8, 6], paint: "sky-300" },
    ],
  },
};

/** Land Doctrine (Stinkyscape #137.1, Token): treaded armour columns roll over a walled fortress on a dusty ochre planet as a starship's shadow fires down on the defenders, in brown dust light. */
const LAND_DOCTRINE: SceneRecipe = {
  family: "stinkyscape",
  sky: ["wood-800", "brass-600"],
  light: { x: 50, y: 60, r: 34, paint: "brass-500", opacity: 0.32 },
  emblem: { x: 42, y: 78, size: 5, rotate: 0 },
  layers: {
    sky: [
      { part: "starship", x: 52, y: 14, scale: 0.9, paint: "slate-800" },
      { part: "energy-beam", x: 52, y: 22, scale: 0.6, paint: "red-500" },
    ],
    far: [
      { part: "regolith", x: 50, y: 66, paint: "brass-600" },
      { part: "fortress", x: 52, y: 66, scale: 1, paint: "wood-700" },
      { part: "mesa", x: 12, y: 64, scale: 0.5, paint: "wood-700" },
    ],
    subject: [
      { part: "tank", x: 40, y: 82, scale: 1.4, paint: "brass-600" },
      { part: "tank", x: 66, y: 88, scale: 1.2, paint: "wood-700" },
      { part: "tank", x: 18, y: 88, scale: 1, paint: "wood-700" },
    ],
    light: [{ part: "halo", x: 52, y: 62, scale: 1.4, paint: "red-400" }],
    particles: [
      { part: "dust-cloud", x: 40, y: 86, scale: 1.2, paint: "brass-500" },
      { part: "sparks", x: 52, y: 60, scale: 0.9, paint: "orange-400" },
      { part: "smoke", x: 60, y: 58, scale: 0.8, paint: "wood-700" },
    ],
  },
};

/** The starship's crew healing on its deck in Sea Doctrine. */
const DECKHAND: FigureLook = { ...BASE, hair: "kerchief", outfit: "outfit-flightsuit", item: null, skin: "wood-600", cloth: "jade-400" };

/** Sea Doctrine (Stinkyscape #137.2, Token): a vast starship hull rides the swell of an alien ocean world, waves breaking over it as two wounded crewmen heal in sea-green spray, under an overcast teal sky. */
const SEA_DOCTRINE: SceneRecipe = {
  family: "stinkyscape",
  sky: ["slate-700", "jade-400"],
  light: { x: 50, y: 40, r: 34, paint: "green-200", opacity: 0.25 },
  emblem: { x: 60, y: 64, size: 5, rotate: 0 },
  extras: [
    { layer: "subject", x: 44, y: 56, scale: 0.26, pose: "pose-kneel", look: DECKHAND },
    { layer: "subject", x: 54, y: 56, scale: 0.26, flip: -1, pose: "pose-sit", look: { ...DECKHAND, cloth: "slate-400" } },
  ],
  layers: {
    sky: [
      { part: "cloud", x: 22, y: 20, scale: 1.4, paint: "slate-400" },
      { part: "cloud", x: 72, y: 16, scale: 1.6, paint: "slate-400" },
    ],
    far: [{ part: "sea", x: 50, y: 58, paint: "blue-800" }],
    subject: [{ part: "starship", x: 50, y: 64, scale: 1.3, paint: "slate-400" }],
    near: [
      { part: "surf-wave", x: 22, y: 82, scale: 1.1, paint: "jade-400" },
      { part: "surf-wave", x: 80, y: 84, scale: 1, flip: -1, paint: "jade-400" },
    ],
    light: [{ part: "heal-glow", x: 49, y: 52, scale: 0.9, paint: "green-400" }],
    particles: [
      { part: "spray", x: 30, y: 62, scale: 1.2, paint: "green-200" },
      { part: "spray", x: 70, y: 64, scale: 1.2, paint: "green-200" },
    ],
  },
};

/** Air Doctrine (Stinkyscape #137.3, Token): fighter squadrons pour from a starship's bays over a cloud-sea planet, missiles fanning down onto scattered enemy positions, in bright high-altitude sun; the enemy positions are turrets on the cloud tops. */
const AIR_DOCTRINE: SceneRecipe = {
  family: "stinkyscape",
  sky: ["blue-700", "sky-300"],
  light: { x: 82, y: 14, r: 30, paint: "white", opacity: 0.4 },
  emblem: { x: 36, y: 22, size: 5, rotate: 0 },
  layers: {
    sky: [
      { part: "sun", x: 84, y: 14, scale: 0.9 },
      { part: "starship", x: 32, y: 22, scale: 0.8 },
    ],
    far: [{ part: "cloud", x: 50, y: 84, scale: 1.8, count: 4, spread: [40, 4], paint: "white" }],
    subject: [
      { part: "fighter", x: 50, y: 40, scale: 0.9, count: 4, spread: [16, 10] },
      { part: "missile", x: 46, y: 62, scale: 0.8 },
      { part: "missile", x: 64, y: 66, scale: 0.7 },
    ],
    near: [
      { part: "turret", x: 26, y: 88, scale: 1, flip: -1 },
      { part: "turret", x: 76, y: 90, scale: 1 },
    ],
    particles: [
      { part: "burst", x: 76, y: 84, scale: 0.6, paint: "orange-400" },
      { part: "smoke", x: 28, y: 82, scale: 0.6, paint: "slate-400" },
    ],
  },
};

/** Space Doctrine (Stinkyscape #137.4, Token): a starship climbs out of a blue planet's atmosphere into black space, ghostly copies of its modules fanning out behind it like wings, in cold starlight. */
const SPACE_DOCTRINE: SceneRecipe = {
  family: "stinkyscape",
  sky: ["black", "blue-900"],
  light: { x: 50, y: 76, r: 38, paint: "sky-300", opacity: 0.3 },
  emblem: { x: 58, y: 48, size: 5, rotate: 0 },
  layers: {
    sky: [
      { part: "stars", x: 50, y: 20, scale: 2.2 },
      { part: "stars", x: 30, y: 50, scale: 1.4, fade: 0.7 },
    ],
    far: [{ part: "planet-limb", x: 50, y: 74, paint: "blue-600" }],
    mid: [
      { part: "ship-module", x: 30, y: 34, scale: 0.7, fade: 0.4, paint: "sky-300" },
      { part: "ship-module", x: 30, y: 60, scale: 0.7, fade: 0.4, paint: "sky-300" },
      { part: "ship-module", x: 16, y: 26, scale: 0.6, fade: 0.25, paint: "sky-300" },
      { part: "ship-module", x: 16, y: 68, scale: 0.6, fade: 0.25, paint: "sky-300" },
    ],
    subject: [{ part: "starship", x: 56, y: 47, scale: 0.95 }],
    light: [{ part: "rim-light", x: 56, y: 46, scale: 1.6, paint: "blue-200" }],
    particles: [{ part: "motes", x: 40, y: 46, scale: 1.4, paint: "sky-300" }],
  },
};

/** Pocket Peptides (Stinkyscape #138, Common): on a boarding ramp mid-firefight, a falling soldier in jade-trimmed armour snaps open a pocket vial, jade liquid fizzing as his muscles surge, under red alarms. */
const POCKET_PEPTIDES: SceneRecipe = {
  family: "stinkyscape",
  sky: ["black", "red-900"],
  light: { x: 50, y: 40, r: 32, paint: "red-500", opacity: 0.34 },
  emblem: { x: 46, y: 60, size: 5, rotate: 0 },
  figure: {
    x: 46,
    y: 86,
    scale: 1,
    pose: "pose-kneel",
    look: { ...BASE, hair: "steel-helmet", outfit: "outfit-armor", item: "item-vial", skin: "orange-400", cloth: "jade-400" },
  },
  layers: {
    far: [
      { part: "room-wall", x: 50, y: 66, paint: "slate-800" },
      { part: "airlock", x: 76, y: 46, scale: 1.2 },
    ],
    mid: [{ part: "ramp", x: 50, y: 88, scale: 2.4, paint: "slate-700" }],
    near: [{ part: "laser", x: 4, y: 40, scale: 0.8, paint: "red-500" }],
    light: [
      { part: "alarm-light", x: 14, y: 20, scale: 1.2 },
      { part: "alarm-light", x: 88, y: 20, scale: 1.2, flip: -1 },
      { part: "halo", x: 63, y: 44, scale: 1, paint: "jade-400" },
    ],
    particles: [
      { part: "bubbles", x: 63, y: 38, scale: 0.8, paint: "jade-400" },
      { part: "sparks", x: 30, y: 40, scale: 0.9, paint: "red-400" },
    ],
  },
};

/** Wormhole (Stinkyscape #139, Rare): a swirling wormhole snaps open before an enemy's healing beam, swallowing it and spitting it out at the other fleet's flagship, in an indigo and cyan swirl. */
const WORMHOLE: SceneRecipe = {
  family: "stinkyscape",
  sky: ["black", "violet-700"],
  light: { x: 48, y: 52, r: 36, paint: "cyan-400", opacity: 0.34 },
  emblem: { x: 84, y: 54, size: 5, rotate: 0 },
  layers: {
    sky: [
      { part: "stars", x: 50, y: 22, scale: 2.2 },
      { part: "stars", x: 50, y: 80, scale: 2 },
      { part: "nebula", x: 24, y: 30, scale: 1.2, paint: "violet-500" },
    ],
    far: [
      { part: "starship", x: 14, y: 58, scale: 0.5, paint: "red-600" },
      { part: "starship", x: 86, y: 54, scale: 0.6, flip: -1, paint: "slate-100" },
      { part: "fighter", x: 22, y: 74, scale: 0.6, paint: "red-600" },
      { part: "fighter", x: 78, y: 70, scale: 0.6, flip: -1 },
    ],
    mid: [
      { part: "laser", x: 24, y: 56, scale: 0.55, paint: "green-400" },
      { part: "laser", x: 58, y: 54, scale: 0.6, paint: "green-400" },
    ],
    subject: [{ part: "wormhole", x: 48, y: 54, scale: 1.1 }],
    light: [
      { part: "halo", x: 48, y: 54, scale: 2.2, paint: "cyan-400" },
      { part: "heal-glow", x: 34, y: 56, scale: 0.6, paint: "green-400" },
      { part: "echo-rings", x: 48, y: 54, scale: 1.4, paint: "violet-300" },
    ],
    particles: [
      { part: "sparks", x: 48, y: 54, scale: 1.2, count: 2, spread: [12, 10], paint: "cyan-400" },
      { part: "motes", x: 50, y: 50, scale: 1.4, paint: "violet-300" },
    ],
  },
};

/** Prepare For Liftoff (Stinkyscape #144, Common): a pilot kneels to kiss her dog goodbye at a launch pad as a broken hull part crashes nearby and the rocket's engines prime, under an orange dawn sky. */
const PREPARE_FOR_LIFTOFF: SceneRecipe = {
  family: "stinkyscape",
  sky: ["violet-700", "orange-400"],
  light: { x: 66, y: 78, r: 34, paint: "orange-400", opacity: 0.4 },
  emblem: { x: 38, y: 65, size: 5, rotate: 0 },
  figure: {
    x: 38,
    y: 88,
    scale: 0.9,
    pose: "pose-genuflect",
    look: { ...BASE, hair: "hair-bun", outfit: "outfit-flightsuit", item: null, skin: "red-300", cloth: "blue-800" },
  },
  layers: {
    sky: [{ part: "sun-disc", x: 18, y: 58, scale: 1.2, paint: "orange-400" }],
    far: [
      { part: "launch-pad", x: 68, y: 74, scale: 0.8 },
      { part: "rocket", x: 68, y: 72, scale: 1.3 },
    ],
    mid: [{ part: "regolith", x: 50, y: 76, paint: "wood-700" }],
    subject: [
      { part: "dog", x: 49, y: 88, scale: 1.1, flip: -1 },
      { part: "ship-module", x: 22, y: 34, scale: 0.8 },
    ],
    light: [{ part: "halo", x: 68, y: 74, scale: 1.4, paint: "orange-400" }],
    particles: [
      { part: "smoke", x: 66, y: 74, scale: 0.9, paint: "slate-400" },
      { part: "speed-streaks", x: 26, y: 30, scale: 0.5, flip: -1, paint: "orange-400" },
      { part: "embers", x: 22, y: 38, scale: 1 },
    ],
  },
};

/** Fortify Flesh (Stinkyscape #145, Rare): a marine's bare forearm hardens in waves from bark to stone to riveted steel plating inside a med-bay scanner, a medic at its console, in cool white scan light; the three waves are a bark block, a stone and a steel plate along his arm. */
const FORTIFY_FLESH: SceneRecipe = {
  family: "stinkyscape",
  sky: ["slate-800", "slate-400"],
  light: { x: 56, y: 34, r: 34, paint: "white", opacity: 0.36 },
  emblem: { x: 40, y: 53, size: 5, rotate: 0 },
  figure: {
    x: 40,
    y: 86,
    scale: 1.05,
    pose: "pose-block",
    look: { ...BASE, hair: "hair-spiky", outfit: "outfit-vest", item: null, skin: "wood-700", cloth: "green-700" },
  },
  extras: [
    { layer: "mid", x: 82, y: 80, scale: 0.6, flip: -1, pose: "pose-hold", look: { ...BASE, hair: "glasses", outfit: "outfit-labcoat", item: "item-clipboard", skin: "red-300", cloth: "white" } },
  ],
  layers: {
    far: [
      { part: "room-wall", x: 50, y: 70, paint: "slate-400" },
      { part: "pod", x: 14, y: 72, scale: 0.8 },
    ],
    mid: [
      { part: "tile-floor", x: 50, y: 72, paint: "slate-100" },
      { part: "console", x: 70, y: 80, scale: 0.8 },
    ],
    subject: [{ part: "light-ring", x: 54, y: 46, scale: 1.4, paint: "sky-300" }],
    near: [
      { part: "log", x: 49.5, y: 44, scale: 0.3, paint: "wood-700" },
      { part: "lifting-stone", x: 53.5, y: 47.3, scale: 0.4, paint: "stone-500" },
      { part: "shield", x: 57, y: 46.5, scale: 0.4, paint: "slate-400" },
    ],
    light: [
      { part: "tractor-beam", x: 54, y: 14, scale: 1.2, paint: "white" },
      { part: "rim-light", x: 44, y: 60, scale: 1.2, paint: "white" },
      { part: "halo", x: 56, y: 47, scale: 1.2, paint: "sky-300" },
    ],
    particles: [
      { part: "motes", x: 50, y: 40, scale: 1.4, paint: "white" },
      { part: "sparks", x: 58, y: 46, scale: 0.7, paint: "sky-300" },
    ],
  },
};

/** The travellers Flash Freeze locks in ice. */
const TRAVELLER: FigureLook = { ...BASE, hair: "beanie", outfit: "outfit-parka", item: null, skin: "red-300", cloth: "red-600" };

/** Flash Freeze (Stinkyscape #146, Rare): a whole station concourse flash-frozen in an instant, two travellers, a porter robot and a dropped cup locked in pale blue ice mid-motion, in a frost glare. */
const FLASH_FREEZE: SceneRecipe = {
  family: "stinkyscape",
  sky: ["slate-900", "blue-800"],
  light: { x: 50, y: 40, r: 40, paint: "blue-200", opacity: 0.4 },
  emblem: { x: 50, y: 78, size: 5, rotate: 0 },
  figure: { x: 31, y: 86, scale: 0.72, pose: "pose-run", look: TRAVELLER },
  extras: [
    { layer: "subject", x: 70, y: 86, scale: 0.7, flip: -1, pose: "pose-stride", look: { ...TRAVELLER, hair: "hat-brim", outfit: "outfit-coat", cloth: "blue-800" } },
  ],
  layers: {
    far: [
      { part: "room-wall", x: 50, y: 66, paint: "slate-700" },
      { part: "column", x: 10, y: 70, scale: 1.4, paint: "slate-400" },
      { part: "column", x: 90, y: 70, scale: 1.4, paint: "slate-400" },
    ],
    mid: [{ part: "tile-floor", x: 50, y: 68, paint: "blue-200" }],
    subject: [
      { part: "robot", x: 50, y: 86, scale: 1.3 },
      { part: "crate", x: 54, y: 66, scale: 0.6 },
      { part: "cup", x: 40, y: 56, scale: 1.2 },
    ],
    near: [
      { part: "ice-block", x: 31, y: 88, scale: 2.1 },
      { part: "ice-block", x: 70, y: 88, scale: 2 },
      { part: "ice-block", x: 50, y: 88, scale: 1.3 },
      { part: "ice-block", x: 40, y: 58, scale: 0.35 },
      { part: "frost", x: 50, y: 92, scale: 2.2 },
    ],
    light: [
      { part: "lens-flare", x: 64, y: 32, scale: 0.8 },
      { part: "halo", x: 50, y: 60, scale: 2, paint: "blue-200" },
    ],
    particles: [
      { part: "shards", x: 50, y: 50, scale: 1.2, count: 2, spread: [20, 10] },
      { part: "motes", x: 50, y: 50, scale: 1.4, paint: "blue-200" },
    ],
  },
};

/** IN33DH3 (Stinkyscape #147, Legendary): a lanky young analyst in an oversized hoodie and headset hunches at a battle-analysis console in a carrier's combat room as a translucent teal AI companion leans over his shoulder, pointing, the lines on the tactical hologram lighting up the best move as enemy ships close in outside, in teal glow and a red alarm rim light. */
const IN33DH3: SceneRecipe = {
  family: "stinkyscape",
  sky: ["black", "slate-900"],
  light: { x: 62, y: 62, r: 34, paint: "cyan-400", opacity: 0.38 },
  emblem: { x: 38, y: 62, size: 5, rotate: 0 },
  figure: {
    x: 38,
    y: 88,
    scale: 1.05,
    pose: "pose-sit",
    look: { ...HUMAN, hair: "headphones", outfit: "outfit-hoodie", item: null, skin: "orange-400", cloth: "slate-400" },
  },
  layers: {
    sky: [
      { part: "stars", x: 50, y: 22, scale: 2 },
      { part: "starship", x: 70, y: 24, scale: 0.4, flip: -1, paint: "red-600" },
      { part: "fighter", x: 40, y: 18, scale: 0.6, flip: -1, paint: "red-600" },
      { part: "fighter", x: 54, y: 30, scale: 0.5, flip: -1, paint: "red-600" },
    ],
    far: [
      { part: "viewport", x: 50, y: 26, scale: 2.1, paint: "slate-700" },
      { part: "room-wall", x: 50, y: 70, paint: "slate-800" },
    ],
    mid: [
      { part: "tile-floor", x: 50, y: 70, paint: "slate-900" },
      { part: "server-rack", x: 88, y: 76, scale: 0.9 },
    ],
    subject: [
      { part: "spirit", x: 26, y: 86, scale: 0.95, paint: "cyan-400", fade: 0.7 },
      { part: "chair", x: 38, y: 90, scale: 1.1, paint: "slate-700" },
      { part: "holo-table", x: 66, y: 84, scale: 1.1 },
    ],
    near: [{ part: "console", x: 62, y: 98, scale: 1.3 }],
    light: [
      { part: "alarm-light", x: 8, y: 50, scale: 1.1 },
      { part: "rim-light", x: 36, y: 64, scale: 1.2, flip: -1, paint: "red-500" },
      { part: "halo", x: 66, y: 66, scale: 1.6, paint: "cyan-400" },
      { part: "halo", x: 26, y: 56, scale: 1.4, paint: "cyan-400" },
    ],
    particles: [
      { part: "motes", x: 56, y: 56, scale: 1.2, paint: "cyan-400" },
      { part: "sparks", x: 66, y: 66, scale: 0.7, paint: "cyan-400" },
    ],
  },
};

/** IN33DH3 Prime (Stinkyscape #147.1, Token): IN33DH3 grown older and calm in a sleek black command coat and a glowing neural crown stands on a command deck as a hologram AI bows to him. */
const IN33DH3_PRIME: SceneRecipe = {
  family: "stinkyscape",
  sky: ["black", "blue-900"],
  light: { x: 44, y: 38, r: 32, paint: "cyan-400", opacity: 0.3 },
  emblem: { x: 42, y: 52, size: 5, rotate: 0 },
  figure: {
    x: 42,
    y: 86,
    scale: 1.05,
    pose: "pose-stand",
    look: { ...HUMAN, hair: "neural-crown", outfit: "outfit-greatcoat", item: null, skin: "orange-400", cloth: "slate-900" },
  },
  layers: {
    sky: [{ part: "stars", x: 50, y: 22, scale: 2 }],
    far: [{ part: "viewport", x: 50, y: 30, scale: 2, paint: "slate-700" }],
    mid: [{ part: "tile-floor", x: 50, y: 66, paint: "slate-800" }],
    subject: [{ part: "spirit", x: 66, y: 86, scale: 0.8, flip: -1, paint: "cyan-400", fade: 0.75 }],
    light: [
      { part: "halo", x: 42, y: 32, scale: 1, paint: "cyan-400" },
      { part: "rim-light", x: 42, y: 58, scale: 1.2, paint: "sky-300" },
    ],
    particles: [{ part: "motes", x: 56, y: 56, scale: 1.2, paint: "cyan-400" }],
  },
};

/** Wardrum Blob (Stinkyscape #147.2, Token): a cosy ship's rec room where a wobbling teal jelly-blob AI hums on a drum-shaped pedestal, nudging pieces on a game board, in soft lamp light. */
const WARDRUM_BLOB: SceneRecipe = {
  family: "stinkyscape",
  sky: ["slate-900", "wood-800"],
  light: { x: 30, y: 30, r: 34, paint: "brass-500", opacity: 0.3 },
  emblem: { x: 40, y: 78, size: 5, rotate: 0 },
  layers: {
    far: [
      { part: "room-wall", x: 50, y: 70, paint: "wood-800" },
      { part: "porthole", x: 78, y: 36, scale: 1.4 },
    ],
    mid: [{ part: "boards", x: 50, y: 72, paint: "wood-700" }],
    subject: [
      { part: "drum", x: 40, y: 88, scale: 1.6, paint: "cyan-400" },
      { part: "jelly-blob", x: 40, y: 70, scale: 1.1, paint: "cyan-400" },
      { part: "wide-table", x: 68, y: 92, scale: 0.8 },
      { part: "game-board", x: 68, y: 81, scale: 0.8 },
    ],
    light: [
      { part: "lantern", x: 22, y: 6, scale: 1.2 },
      { part: "echo-rings", x: 40, y: 62, scale: 1, paint: "cyan-400" },
    ],
    particles: [{ part: "motes", x: 50, y: 50, scale: 1.2, paint: "brass-500" }],
  },
};

/** Edge of the Universe (Stinkyscape #148.1, Token): a lone traveller steps off the frayed edge of the starfield into blank white light, the galaxy curling up behind her. */
const EDGE_OF_THE_UNIVERSE: SceneRecipe = {
  family: "stinkyscape",
  sky: ["black", "violet-700"],
  light: { x: 68, y: 50, r: 40, paint: "white", opacity: 0.45 },
  emblem: { x: 50, y: 57, size: 5, rotate: 0 },
  figure: {
    x: 50,
    y: 84,
    scale: 0.8,
    pose: "pose-stride",
    look: { ...BASE, hair: "hair-long", outfit: "outfit-cloak", item: null, skin: "brass-600", cloth: "slate-700" },
  },
  layers: {
    sky: [
      { part: "stars", x: 30, y: 30, scale: 1.6 },
      { part: "stars", x: 24, y: 70, scale: 1.4 },
      { part: "galaxy", x: 24, y: 32, scale: 1 },
    ],
    far: [{ part: "space-rift", x: 70, y: 52, scale: 1.8 }],
    subject: [{ part: "asteroid", x: 48, y: 88, scale: 2, paint: "slate-700" }],
    light: [
      { part: "god-rays", x: 70, y: 30, scale: 1, paint: "white" },
      { part: "rim-light", x: 52, y: 62, scale: 1.1, paint: "white" },
    ],
    particles: [
      { part: "motes", x: 64, y: 50, scale: 1.4, paint: "white" },
      { part: "shards", x: 60, y: 70, scale: 0.8 },
    ],
  },
};

/** Wildar (Stinkyscape #149, Legendary): Wildar, a grinning trickster in a patchwork coat of nebula colours, stands on the open deck of a tumbling derelict and snaps a churning orb of chaos in two, flicking the dull, sputtering half to a startled rival and keeping the dazzling prismatic half in his own palm, in crackling multicolour arcs and swirling smoke. */
const WILDAR: SceneRecipe = {
  family: "stinkyscape",
  sky: ["black", "violet-700"],
  light: { x: 54, y: 50, r: 36, paint: "purple-400", opacity: 0.42 },
  emblem: { x: 38, y: 52, size: 5, rotate: 0 },
  figure: {
    x: 38,
    y: 86,
    scale: 1,
    pose: "pose-cast",
    look: { ...BASE, hair: "top-hat", outfit: "outfit-patched", item: "item-orb", skin: "red-300", cloth: "violet-500" },
  },
  extras: [
    { layer: "subject", x: 76, y: 86, scale: 0.82, flip: -1, pose: "pose-block", look: { ...BASE, hair: "goggles", outfit: "outfit-jacket", item: null, skin: "wood-700", cloth: "slate-700" } },
  ],
  layers: {
    sky: [
      { part: "stars", x: 50, y: 20, scale: 2.2 },
      { part: "nebula", x: 70, y: 24, scale: 1.3, paint: "violet-500" },
      { part: "planet", x: 16, y: 18, scale: 0.7, paint: "jade-400" },
    ],
    far: [
      { part: "wreckage", x: 84, y: 40, scale: 0.6 },
      { part: "ruin-wall", x: 16, y: 72, scale: 1.2, paint: "slate-700" },
      { part: "pipe", x: 62, y: 66, scale: 1, flip: -1 },
    ],
    mid: [
      { part: "tile-floor", x: 50, y: 72, paint: "slate-800" },
      { part: "crate", x: 92, y: 80, scale: 1.2 },
    ],
    subject: [
      { part: "orb", x: 62, y: 56, scale: 0.4, paint: "slate-700" },
      { part: "zap", x: 52, y: 52, scale: 0.4, paint: "violet-300" },
    ],
    near: [
      { part: "crate", x: 10, y: 98, scale: 1.4, paint: "slate-700" },
      { part: "cable", x: 72, y: 96, scale: 0.8 },
    ],
    light: [
      { part: "halo", x: 52, y: 52, scale: 1.4, paint: "cyan-400" },
      { part: "halo", x: 52, y: 52, scale: 0.8, paint: "lime-400" },
      { part: "lightning", x: 50, y: 30, scale: 0.6, paint: "purple-400" },
      { part: "rim-light", x: 40, y: 60, scale: 1.2, paint: "violet-300" },
    ],
    particles: [
      { part: "sparks", x: 52, y: 52, scale: 1.1, count: 2, spread: [8, 8], paint: "cyan-400" },
      { part: "sparks", x: 52, y: 52, scale: 1, paint: "lime-400" },
      { part: "smoke", x: 64, y: 54, scale: 0.7, paint: "slate-400" },
    ],
  },
};

/** Alteran Reach (Stinkyscape #148, Mythic): a starscape route laid out as a glowing path of twenty island-stations stepping away through deep space, each a tiny world of its own (a coral moon, a crystal spire, a jungle rock, a ringed outpost) with a souvenir glinting at each, a lone ship mid-jump between two, the path ending at a sheer cliff where the universe's fabric peels back into white light, in violet and cyan depth and star dust; the islands are asteroids, the path a ribbon of light. */
const ALTERAN_REACH: SceneRecipe = {
  family: "stinkyscape",
  sky: ["black", "violet-700"],
  light: { x: 50, y: 18, r: 34, paint: "white", opacity: 0.4 },
  emblem: { x: 36, y: 68, size: 6, rotate: 0 },
  layers: {
    sky: [
      { part: "stars", x: 50, y: 40, scale: 2.4 },
      { part: "nebula", x: 24, y: 40, scale: 1.4, paint: "cyan-400" },
      { part: "nebula", x: 76, y: 34, scale: 1.2, paint: "violet-500" },
    ],
    far: [
      { part: "cliff", x: 50, y: 24, scale: 0.5, paint: "slate-700" },
      { part: "space-rift", x: 50, y: 14, scale: 0.6 },
      { part: "asteroid", x: 50, y: 30, scale: 0.45, count: 8, spread: [16, 4], paint: "violet-500" },
    ],
    mid: [
      { part: "asteroid", x: 50, y: 44, scale: 0.9, count: 6, spread: [26, 6], paint: "slate-400" },
      { part: "crystal", x: 30, y: 42, scale: 0.9, paint: "cyan-400" },
      { part: "moon", x: 70, y: 40, scale: 0.4, paint: "red-300" },
      { part: "planet", x: 22, y: 54, scale: 0.4, paint: "violet-300" },
    ],
    subject: [
      { part: "asteroid", x: 36, y: 66, scale: 2.2, paint: "green-700" },
      { part: "asteroid", x: 66, y: 58, scale: 1.8, paint: "slate-700" },
      { part: "tower", x: 66, y: 54, scale: 0.4, paint: "slate-400" },
      { part: "shuttle", x: 52, y: 52, scale: 0.6 },
      { part: "speed-streaks", x: 47, y: 52, scale: 0.4, paint: "cyan-400" },
    ],
    near: [
      { part: "asteroid", x: 20, y: 92, scale: 3, paint: "slate-700" },
      { part: "asteroid", x: 80, y: 88, scale: 2.6, paint: "violet-700" },
      { part: "crystal", x: 22, y: 82, scale: 1.2, paint: "violet-300" },
    ],
    light: [
      { part: "light-ribbon", x: 50, y: 56, scale: 1, paint: "cyan-400" },
      { part: "god-rays", x: 50, y: 14, scale: 0.7, paint: "white" },
      { part: "halo", x: 50, y: 14, scale: 1.4, paint: "white" },
      { part: "halo", x: 52, y: 52, scale: 0.8, paint: "cyan-400" },
    ],
    particles: [
      { part: "motes", x: 50, y: 40, scale: 1.4, paint: "violet-300" },
      { part: "sparks", x: 50, y: 60, scale: 0.8, count: 2, spread: [30, 24], paint: "cyan-400" },
    ],
  },
};

/** The heroes Call to Chaos (Stinky Edition) flings out. */
const FLUNG_HERO: FigureLook = { ...BASE, hair: "helm", outfit: "outfit-plate", item: "item-sword", skin: "red-300", cloth: "slate-400" };

/** Call to Chaos (Stinky Edition) (Stinkyscape #150, Legendary): over a lime-green gas giant a pungent green nebula erupts from a cracked chaos egg, spewing heroes, a golem, a quest fortress and a frozen enemy ship every way while a grinning captain ducks the blast on a tiny ship, in sickly green and violet light and swirling stink clouds; the golem is its flung stone fist, the tiny ship a shuttle he crouches on. */
const CALL_TO_CHAOS_STINKY_EDITION: SceneRecipe = {
  family: "stinkyscape",
  sky: ["black", "violet-700"],
  light: { x: 50, y: 46, r: 40, paint: "lime-400", opacity: 0.42 },
  emblem: { x: 30, y: 80, size: 5, rotate: 0 },
  figure: {
    x: 68,
    y: 80,
    scale: 0.6,
    flip: -1,
    pose: "pose-kneel",
    look: { ...BASE, hair: "cap-feather", outfit: "outfit-jacket", item: null, skin: "brass-600", cloth: "green-900" },
  },
  extras: [
    { layer: "mid", x: 24, y: 46, scale: 0.34, pose: "pose-spread", look: FLUNG_HERO },
    { layer: "mid", x: 78, y: 40, scale: 0.3, flip: -1, pose: "pose-strike", look: { ...FLUNG_HERO, hair: "hood", outfit: "outfit-robe", item: "item-staff", cloth: "violet-500" } },
  ],
  layers: {
    sky: [
      { part: "planet", x: 50, y: 92, scale: 3.6, paint: "lime-400" },
      { part: "nebula", x: 50, y: 40, scale: 1.6, paint: "green-400" },
    ],
    far: [
      { part: "fortress", x: 84, y: 24, scale: 0.36, paint: "slate-400" },
      { part: "fist", x: 82, y: 62, scale: 0.8, paint: "stone-500" },
    ],
    mid: [
      { part: "fighter", x: 22, y: 64, scale: 0.8, flip: -1, paint: "blue-200" },
      { part: "ice-block", x: 22, y: 69, scale: 0.6 },
    ],
    subject: [
      { part: "stink-cloud", x: 50, y: 42, scale: 1.3 },
      { part: "chaos-egg", x: 50, y: 50, scale: 1.4 },
      { part: "shuttle", x: 68, y: 84, scale: 1.4, flip: -1 },
    ],
    near: [{ part: "shards", x: 34, y: 38, scale: 0.8 }],
    light: [
      { part: "halo", x: 50, y: 48, scale: 2.4, paint: "lime-400" },
      { part: "halo", x: 50, y: 48, scale: 1.2, paint: "violet-300" },
      { part: "rim-light", x: 68, y: 70, scale: 0.9, flip: -1, paint: "lime-200" },
    ],
    particles: [{ part: "sparks", x: 50, y: 48, scale: 1.1, count: 2, spread: [22, 18], paint: "violet-300" }],
  },
};

/** Stinkyscape Golem (Stinkyscape #150.1, Token): a hulking golem of scrap stone and hull plates with thruster boots and wing fins rockets over a junk-strewn moon, green stink vapour trailing behind. */
const STINKYSCAPE_GOLEM: SceneRecipe = {
  family: "stinkyscape",
  sky: ["black", "slate-900"],
  light: { x: 50, y: 46, r: 34, paint: "lime-400", opacity: 0.25 },
  emblem: { x: 50, y: 48, size: 5, rotate: 0 },
  layers: {
    sky: [
      { part: "stars", x: 50, y: 20, scale: 2.2 },
      { part: "planet", x: 82, y: 18, scale: 0.8, paint: "green-400" },
    ],
    far: [{ part: "scrap-heap", x: 50, y: 82, paint: "slate-700" }],
    subject: [{ part: "scrap-golem", x: 50, y: 50, scale: 1.15 }],
    particles: [
      { part: "stink-cloud", x: 32, y: 80, scale: 0.8 },
      { part: "sparks", x: 50, y: 74, scale: 1, paint: "lime-200" },
    ],
  },
};

/** Call to Chaos (Scape Edition) (Stinkyscape #151, Legendary): from a tiny figure's viewpoint on a vast glassy plain of space, the whole starscape tilts like a tabletop as drone swarms, a battleship, tumbling starship parts and open rifts cascade down toward her in a churning spiral, in deep indigo and electric cyan with storm-light flashes; the drones are interceptors. */
const CALL_TO_CHAOS_SCAPE_EDITION: SceneRecipe = {
  family: "stinkyscape",
  sky: ["black", "violet-700"],
  light: { x: 50, y: 36, r: 40, paint: "cyan-400", opacity: 0.36 },
  emblem: { x: 64, y: 82, size: 5, rotate: 0 },
  figure: {
    x: 50,
    y: 86,
    scale: 0.42,
    pose: "pose-spread",
    look: { ...BASE, hair: "space-helmet", outfit: "outfit-flightsuit", item: null, skin: "red-300", cloth: "white" },
  },
  layers: {
    sky: [
      { part: "stars", x: 50, y: 20, scale: 2.4 },
      { part: "void-swirl", x: 50, y: 36, scale: 2.6 },
      { part: "nebula", x: 24, y: 24, scale: 1.2, paint: "blue-600" },
    ],
    far: [
      { part: "starship", x: 52, y: 20, scale: 0.7, flip: -1, paint: "slate-400" },
      { part: "space-rift", x: 16, y: 34, scale: 0.5 },
      { part: "space-rift", x: 84, y: 44, scale: 0.4 },
    ],
    mid: [{ part: "tile-floor", x: 50, y: 70, paint: "blue-900" }],
    subject: [
      { part: "interceptor", x: 34, y: 46, scale: 0.8, count: 2, spread: [10, 6] },
      { part: "interceptor", x: 68, y: 50, scale: 0.8, flip: -1 },
      { part: "ship-module", x: 42, y: 62, scale: 0.6 },
    ],
    near: [{ part: "shards", x: 50, y: 92, scale: 1.4, count: 2, spread: [22, 2] }],
    light: [
      { part: "lightning", x: 30, y: 10, scale: 0.8, paint: "cyan-400" },
      { part: "lightning", x: 72, y: 14, scale: 0.7, paint: "cyan-400" },
      { part: "halo", x: 50, y: 36, scale: 2.4, paint: "cyan-400" },
      { part: "rim-light", x: 50, y: 76, scale: 0.6, paint: "cyan-400" },
    ],
    particles: [
      { part: "sparks", x: 50, y: 40, scale: 1, count: 3, spread: [26, 16], paint: "cyan-400" },
      { part: "motes", x: 50, y: 60, scale: 1.4, paint: "sky-300" },
    ],
  },
};

/** The relic-keeper of Sanctum. */
const ARCHIVIST: FigureLook = { ...BASE, hair: "hair-bun", outfit: "outfit-robe", item: null, skin: "wood-700", cloth: "violet-700" };

/** Sanctum (Stinkyscape #152, Legendary): a hushed cathedral-library inside a hollowed asteroid, curving shelves of glowing relics, a central pool reflecting the stars and ghostly echoes of past spells replaying above the altar as a lone archivist draws one back up into the light, in pale silver light through a rose-window porthole and cool violet shadows; the echoes are faint ghost figures and spell bolts. */
const SANCTUM: SceneRecipe = {
  family: "stinkyscape",
  sky: ["black", "violet-700"],
  light: { x: 50, y: 24, r: 36, paint: "slate-100", opacity: 0.38 },
  emblem: { x: 54, y: 79, size: 5, rotate: 0 },
  figure: { x: 34, y: 86, scale: 0.8, pose: "pose-lift", look: ARCHIVIST },
  layers: {
    sky: [
      { part: "cavern", x: 50, y: 10, paint: "slate-800" },
      { part: "stars", x: 50, y: 22, scale: 0.8 },
    ],
    far: [
      { part: "porthole", x: 50, y: 22, scale: 1.8 },
      { part: "bookshelf", x: 12, y: 80, scale: 1.6, paint: "slate-700" },
      { part: "bookshelf", x: 88, y: 80, scale: 1.6, paint: "slate-700" },
      { part: "crystal", x: 12, y: 50, scale: 0.6, paint: "violet-300" },
      { part: "crystal", x: 88, y: 50, scale: 0.6, paint: "cyan-400" },
    ],
    mid: [
      { part: "tile-floor", x: 50, y: 74, paint: "slate-700" },
      { part: "still-water", x: 50, y: 88, paint: "blue-200" },
    ],
    subject: [
      { part: "altar", x: 54, y: 84, scale: 1.3, paint: "slate-400" },
      { part: "ghost-figure", x: 50, y: 64, scale: 0.8, fade: 0.6 },
      { part: "ghost-figure", x: 64, y: 60, scale: 0.7, flip: -1, fade: 0.45 },
      { part: "magic-bolt", x: 54, y: 46, scale: 0.6, paint: "violet-300" },
    ],
    near: [
      { part: "stars", x: 50, y: 94, scale: 1, fade: 0.6 },
      { part: "column", x: 4, y: 100, scale: 2, paint: "slate-800" },
      { part: "column", x: 96, y: 100, scale: 2, paint: "slate-800" },
    ],
    light: [
      { part: "light-shaft", x: 50, y: 24, scale: 1.3, paint: "slate-100" },
      { part: "halo", x: 42, y: 34, scale: 1.4, paint: "white" },
      { part: "rim-light", x: 34, y: 60, scale: 1, paint: "slate-100" },
    ],
    particles: [
      { part: "motes", x: 50, y: 46, scale: 1.4, paint: "slate-100" },
      { part: "sparks", x: 42, y: 34, scale: 0.8, paint: "violet-300" },
    ],
  },
};

/** The cheering shoppers of Stimmycheck. */
const SHOPPER: FigureLook = { ...BASE, hair: "beanie", outfit: "outfit-jacket", item: null, skin: "orange-400", cloth: "jade-400" };

/** Stimmycheck (Stinkyscape #153, Legendary): in a crowded orbital market a delivery drone showers green credit chips over cheering shoppers while every price hologram over the stalls flickers upward as a rising bar and a vendor frantically re-tags his wares, a second drone already queued in the sky, in mint-green and coral light with chips swirling like confetti. */
const STIMMYCHECK: SceneRecipe = {
  family: "stinkyscape",
  sky: ["black", "slate-800"],
  light: { x: 50, y: 34, r: 40, paint: "green-200", opacity: 0.36 },
  emblem: { x: 70, y: 59, size: 5, rotate: 0 },
  figure: {
    x: 70,
    y: 86,
    scale: 0.8,
    flip: -1,
    pose: "pose-hold",
    look: { ...BASE, hair: "bucket-hat", outfit: "outfit-work-apron", item: "item-card", skin: "wood-700", cloth: "red-400" },
  },
  extras: [
    { layer: "subject", x: 28, y: 88, scale: 0.72, pose: "pose-raise", look: SHOPPER },
    { layer: "subject", x: 44, y: 90, scale: 0.66, pose: "pose-spread", look: { ...SHOPPER, hair: "hair-long", cloth: "red-300" } },
  ],
  layers: {
    sky: [{ part: "drone", x: 84, y: 14, scale: 0.7, flip: -1 }],
    far: [
      { part: "awning", x: 22, y: 72, scale: 1.2, paint: "jade-400" },
      { part: "awning", x: 76, y: 72, scale: 1.2, flip: -1, paint: "red-400" },
      { part: "price-bars", x: 22, y: 46, scale: 1, paint: "green-400" },
      { part: "price-bars", x: 76, y: 46, scale: 1, paint: "red-300" },
    ],
    mid: [{ part: "tile-floor", x: 50, y: 72, paint: "slate-700" }],
    subject: [
      { part: "drone", x: 50, y: 28, scale: 1.3 },
      { part: "credit-chip", x: 46, y: 50, scale: 1, count: 3, spread: [18, 10] },
    ],
    near: [{ part: "confetti", x: 50, y: 70, scale: 1 }],
    light: [
      { part: "halo", x: 50, y: 30, scale: 1.6, paint: "green-200" },
      { part: "halo", x: 76, y: 50, scale: 1.2, paint: "red-300" },
      { part: "light-shaft", x: 50, y: 30, scale: 0.9, paint: "green-200" },
    ],
    particles: [
      { part: "confetti", x: 40, y: 56, scale: 1.2 },
      { part: "motes", x: 50, y: 44, scale: 1.2, paint: "green-200" },
    ],
  },
};

/** Comet Crash (Stinkyscape #155, Rare): a smoking fresh comet crater on a grey moon, its molten rim glowing orange, ice and ore crystals studding its floor, a prospector rover nearby. */
const COMET_CRASH: SceneRecipe = {
  family: "stinkyscape",
  sky: ["black", "slate-900"],
  light: { x: 46, y: 72, r: 32, paint: "orange-400", opacity: 0.4 },
  emblem: { x: 80, y: 80, size: 5, rotate: 0 },
  layers: {
    sky: [
      { part: "stars", x: 50, y: 24, scale: 2.2 },
      { part: "planet", x: 80, y: 20, scale: 1, paint: "blue-600" },
    ],
    far: [{ part: "regolith", x: 50, y: 60, paint: "grey-500" }],
    subject: [
      { part: "bowl-crater", x: 46, y: 74, scale: 1.9 },
      { part: "ore-chunk", x: 40, y: 76, scale: 0.7, paint: "sky-400" },
      { part: "crystal", x: 52, y: 76, scale: 0.9, paint: "blue-200" },
      { part: "ice-shard", x: 34, y: 74, scale: 0.6 },
    ],
    near: [{ part: "rover", x: 80, y: 88, scale: 1.3, flip: -1 }],
    light: [
      { part: "halo", x: 46, y: 72, scale: 1.8, paint: "orange-400" },
      { part: "rim-light", x: 46, y: 72, scale: 1.6, paint: "ember-500" },
    ],
    particles: [
      { part: "smoke", x: 46, y: 66, scale: 1.1, paint: "slate-400" },
      { part: "embers", x: 46, y: 68, scale: 1.2, count: 2, spread: [12, 4] },
    ],
  },
};

/** Catalyst Comet (Stinkyscape #156, Epic): an astronomer on the rooftop of a desert colony dome catches a sparkling capsule as it drops, a teal-tailed comet swinging past on its looping orbit and faint ellipses of its earlier passes traced across a violet nebula behind. */
const CATALYST_COMET: SceneRecipe = {
  family: "stinkyscape",
  sky: ["black", "violet-700"],
  light: { x: 52, y: 38, r: 34, paint: "cyan-400", opacity: 0.34 },
  emblem: { x: 30, y: 84, size: 5, rotate: 0 },
  figure: {
    x: 46,
    y: 74,
    scale: 0.8,
    pose: "pose-raise",
    look: { ...BASE, hair: "spectacles", outfit: "outfit-kaftan", item: null, skin: "wood-700", cloth: "orange-400" },
  },
  layers: {
    sky: [
      { part: "stars", x: 50, y: 20, scale: 2.2 },
      { part: "nebula", x: 60, y: 26, scale: 1.4, paint: "violet-500" },
    ],
    far: [{ part: "orbit-trace", x: 54, y: 28, scale: 1.1, paint: "violet-300" }],
    mid: [
      { part: "comet", x: 72, y: 34, scale: 1.1, flip: -1, paint: "cyan-400" },
      { part: "sand", x: 50, y: 84, paint: "brass-600" },
      { part: "colony-dome", x: 50, y: 92, scale: 1.7 },
    ],
    subject: [{ part: "capsule", x: 52, y: 26, scale: 0.8 }],
    near: [{ part: "telescope", x: 30, y: 76, scale: 1, flip: -1 }],
    light: [
      { part: "halo", x: 52, y: 26, scale: 1.2, paint: "cyan-400" },
      { part: "rim-light", x: 46, y: 56, scale: 1, paint: "cyan-400" },
    ],
    particles: [
      { part: "sparks", x: 52, y: 26, scale: 1, count: 2, spread: [5, 5], paint: "white" },
      { part: "motes", x: 60, y: 40, scale: 1.4, paint: "cyan-400" },
    ],
  },
};

/** The Gauntlet (Stinkyscape #157, Epic): an arena floor on a battle station split into three parallel corridors, two mechs locked in combat inside the centre lane, crackling purple force-cages sealing each corridor at its far end and spectators behind glass above, in storm-violet light. */
const THE_GAUNTLET: SceneRecipe = {
  family: "stinkyscape",
  sky: ["black", "violet-700"],
  light: { x: 50, y: 50, r: 38, paint: "violet-500", opacity: 0.4 },
  emblem: { x: 50, y: 64, size: 5, rotate: 0 },
  layers: {
    far: [
      { part: "room-wall", x: 50, y: 58, paint: "slate-900" },
      { part: "glass-wall", x: 30, y: 36, scale: 1.1 },
      { part: "glass-wall", x: 70, y: 36, scale: 1.1 },
      { part: "crew-crowd", x: 30, y: 34, scale: 0.9 },
      { part: "crew-crowd", x: 70, y: 34, scale: 0.9 },
      { part: "energy-wall", x: 26, y: 58, scale: 0.6, paint: "purple-400" },
      { part: "energy-wall", x: 50, y: 58, scale: 0.6, paint: "purple-400" },
      { part: "energy-wall", x: 74, y: 58, scale: 0.6, paint: "purple-400" },
    ],
    mid: [{ part: "lanes", x: 50, y: 58, paint: "slate-700" }],
    subject: [
      { part: "mech", x: 42, y: 82, scale: 0.85 },
      { part: "mech", x: 58, y: 82, scale: 0.85, flip: -1, paint: "red-600" },
    ],
    near: [
      { part: "column", x: 4, y: 100, scale: 2.2, paint: "slate-800" },
      { part: "column", x: 96, y: 100, scale: 2.2, paint: "slate-800" },
    ],
    light: [
      { part: "lightning", x: 26, y: 40, scale: 0.5, paint: "purple-400" },
      { part: "lightning", x: 74, y: 40, scale: 0.5, paint: "purple-400" },
      { part: "halo", x: 50, y: 60, scale: 2, paint: "violet-500" },
    ],
    particles: [
      { part: "sparks", x: 50, y: 62, scale: 1.1, count: 2, spread: [6, 6], paint: "violet-300" },
      { part: "motes", x: 50, y: 50, scale: 1.4, paint: "purple-400" },
    ],
  },
};

/** Stratos Swarm (Stinkyscape #158, Rare): dozens of identical small Stratos fighters pour from a cargo freighter's open hold in tidy rows, exhaust trails streaking over a blue planet. */
const STRATOS_SWARM: SceneRecipe = {
  family: "stinkyscape",
  sky: ["black", "blue-900"],
  light: { x: 30, y: 44, r: 30, paint: "sky-300", opacity: 0.3 },
  emblem: { x: 30, y: 46, size: 5, rotate: 0 },
  layers: {
    sky: [
      { part: "stars", x: 50, y: 22, scale: 2.2 },
      { part: "planet", x: 50, y: 98, scale: 4, paint: "blue-600" },
    ],
    subject: [
      { part: "freighter", x: 32, y: 46, scale: 0.8 },
      { part: "fighter", x: 54, y: 34, scale: 0.6, paint: "sky-400" },
      { part: "fighter", x: 66, y: 34, scale: 0.6, paint: "sky-400" },
      { part: "fighter", x: 78, y: 34, scale: 0.6, paint: "sky-400" },
      { part: "fighter", x: 90, y: 34, scale: 0.6, paint: "sky-400" },
      { part: "fighter", x: 58, y: 46, scale: 0.6, paint: "sky-400" },
      { part: "fighter", x: 70, y: 46, scale: 0.6, paint: "sky-400" },
      { part: "fighter", x: 82, y: 46, scale: 0.6, paint: "sky-400" },
      { part: "fighter", x: 54, y: 58, scale: 0.6, paint: "sky-400" },
      { part: "fighter", x: 66, y: 58, scale: 0.6, paint: "sky-400" },
      { part: "fighter", x: 78, y: 58, scale: 0.6, paint: "sky-400" },
    ],
    light: [
      { part: "halo", x: 36, y: 48, scale: 1.2, paint: "sky-300" },
      { part: "rim-light", x: 32, y: 44, scale: 1.4, paint: "blue-200" },
    ],
    particles: [
      { part: "streaks", x: 60, y: 34, scale: 0.5, paint: "sky-300" },
      { part: "streaks", x: 64, y: 46, scale: 0.5, paint: "sky-300" },
      { part: "streaks", x: 60, y: 58, scale: 0.5, paint: "sky-300" },
      { part: "motes", x: 60, y: 46, scale: 1.4, paint: "sky-300" },
    ],
  },
};

/** The councillors of Faction: ETHICS. */
const COUNCILLOR: FigureLook = { ...BASE, hair: "hair-short", outfit: "outfit-blazer", item: null, skin: "red-300", cloth: "white" };

/** Faction: ETHICS (Stinkyscape #159, Rare): six councillors round a council table in a starship's ring raise their hands in a unanimous vote as approval stamps thump onto blank papers, in daylight; the stamps are the pages' own seal and a thump of rings. */
const FACTION_ETHICS: SceneRecipe = {
  family: "stinkyscape",
  sky: ["blue-600", "sky-300"],
  light: { x: 50, y: 24, r: 40, paint: "white", opacity: 0.34 },
  emblem: { x: 36, y: 78, size: 5, rotate: 0 },
  figure: { x: 42, y: 86, scale: 0.7, pose: "pose-raise", look: COUNCILLOR },
  extras: [
    { layer: "subject", x: 58, y: 86, scale: 0.7, flip: -1, pose: "pose-raise", look: { ...COUNCILLOR, hair: "hair-silver", skin: "wood-700", cloth: "violet-500" } },
    { layer: "subject", x: 26, y: 88, scale: 0.72, pose: "pose-raise", look: { ...COUNCILLOR, hair: "hair-long", skin: "wood-700", cloth: "blue-800" } },
    { layer: "subject", x: 74, y: 88, scale: 0.72, flip: -1, pose: "pose-raise", look: { ...COUNCILLOR, hair: "glasses", skin: "brass-600", cloth: "slate-400" } },
    { layer: "subject", x: 11, y: 92, scale: 0.76, pose: "pose-raise", look: { ...COUNCILLOR, hair: "hair-bun", skin: "orange-400", cloth: "jade-400" } },
    { layer: "subject", x: 89, y: 92, scale: 0.76, flip: -1, pose: "pose-raise", look: { ...COUNCILLOR, hair: "beard", skin: "wood-600", cloth: "red-600" } },
  ],
  layers: {
    sky: [{ part: "planet", x: 76, y: 20, scale: 1, paint: "white" }],
    far: [
      { part: "viewport", x: 50, y: 30, scale: 2.1, paint: "slate-100" },
      { part: "room-wall", x: 50, y: 74, paint: "slate-400" },
    ],
    near: [
      { part: "wide-table", x: 50, y: 100, scale: 2.4, paint: "slate-100" },
      { part: "pages", x: 50, y: 76, scale: 1, paint: "white" },
    ],
    light: [
      { part: "light-shaft", x: 50, y: 20, scale: 1.4, paint: "white" },
      { part: "echo-rings", x: 50, y: 76, scale: 0.8, paint: "red-400" },
    ],
    particles: [{ part: "motes", x: 50, y: 50, scale: 1.4, paint: "white" }],
  },
};

export const STINKYSCAPE_91_RECIPES_2: Readonly<Record<string, SceneRecipe>> = {
  "stinkyscape-120": WILD_SYSTEM,
  "stinkyscape-121": SYSTEM_SCHEMATIC,
  "stinkyscape-123": GOKUGAINS,
  "stinkyscape-124": JLOCKEEDS_SHIPBUILDING_GRANT,
  "stinkyscape-125": JLOCKEEDS_SPACERACE_PROPAGANDA,
  "stinkyscape-126": JLOCKEEDS_PERMANENT_PROCUREMENT_LOBBYING,
  "stinkyscape-130": JLOCKEEDS_STARSHIP_RETROFITTING_PROGRAM,
  "stinkyscape-135": SACKED,
  "stinkyscape-135-1": SACK,
  "stinkyscape-137": STARFLEET_COMMANDER,
  "stinkyscape-137-1": LAND_DOCTRINE,
  "stinkyscape-137-2": SEA_DOCTRINE,
  "stinkyscape-137-3": AIR_DOCTRINE,
  "stinkyscape-137-4": SPACE_DOCTRINE,
  "stinkyscape-138": POCKET_PEPTIDES,
  "stinkyscape-139": WORMHOLE,
  "stinkyscape-144": PREPARE_FOR_LIFTOFF,
  "stinkyscape-145": FORTIFY_FLESH,
  "stinkyscape-146": FLASH_FREEZE,
  "stinkyscape-147": IN33DH3,
  "stinkyscape-147-1": IN33DH3_PRIME,
  "stinkyscape-147-2": WARDRUM_BLOB,
  "stinkyscape-148": ALTERAN_REACH,
  "stinkyscape-148-1": EDGE_OF_THE_UNIVERSE,
  "stinkyscape-149": WILDAR,
  "stinkyscape-150": CALL_TO_CHAOS_STINKY_EDITION,
  "stinkyscape-150-1": STINKYSCAPE_GOLEM,
  "stinkyscape-151": CALL_TO_CHAOS_SCAPE_EDITION,
  "stinkyscape-152": SANCTUM,
  "stinkyscape-153": STIMMYCHECK,
  "stinkyscape-155": COMET_CRASH,
  "stinkyscape-156": CATALYST_COMET,
  "stinkyscape-157": THE_GAUNTLET,
  "stinkyscape-158": STRATOS_SWARM,
  "stinkyscape-159": FACTION_ETHICS,
};
