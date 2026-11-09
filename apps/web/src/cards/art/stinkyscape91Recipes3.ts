// R3403000: Stinkyscape's later scene recipes, part 3 of three (#160 to #176, the drones', the resources' and T-AI-1 to 9), one per entry of
// #3403's table, each drawn from its brief in Stinkyscape's deep-space look (ART.md, "Stinkyscape's later cards drawn
// as scenes"): its subject within x 30 to 70 and y 25 to 85, its foreground near, its middle in the mid and subject
// layers and its background far, richer the rarer. Presentation only (CLAUDE.md rule 7): a recipe is read by the
// card's id alone.

import { figureLookFor, type FigureLook } from "./figureKit.ts";
import type { SceneRecipe } from "./scene.ts";

/** The look every card of this file starts from: none of them carries a tag with people of its own, so each is the hooded traveller. */
const BASE: FigureLook = figureLookFor([]);

/** Faction: Golden Accord (Stinkyscape #160, Rare): four warship captains press seals onto a blank treaty scroll on a white flagship's marble table, three Paragons and a Falcon outside; the Paragons are starships and the Falcon a fighter beyond the viewport, the wax seals red coins. */
const FACTION_GOLDEN_ACCORD: SceneRecipe = {
  family: "stinkyscape",
  sky: ["black", "blue-900"],
  light: { x: 50, y: 30, r: 36, paint: "sky-300", opacity: 0.28 },
  emblem: { x: 40, y: 70, size: 5, rotate: 0 },
  figure: { x: 40, y: 100, scale: 0.86, pose: "pose-stand", look: { ...BASE, hair: "hair-silver", outfit: "outfit-greatcoat", item: null, skin: "red-300", cloth: "white" } },
  extras: [
    { layer: "subject", x: 60, y: 100, scale: 0.86, flip: -1, pose: "pose-stand", look: { ...BASE, hair: "hair-long", outfit: "outfit-suit", item: null, skin: "wood-700", cloth: "blue-800" } },
    { layer: "mid", x: 26, y: 96, scale: 0.78, pose: "pose-hold", look: { ...BASE, hair: "conductor-cap", outfit: "outfit-coat", item: null, skin: "brass-600", cloth: "red-900" } },
    { layer: "mid", x: 74, y: 96, scale: 0.78, flip: -1, pose: "pose-hold", look: { ...BASE, hair: "hair-short", outfit: "outfit-coat", item: null, skin: "orange-400", cloth: "slate-700" } },
  ],
  layers: {
    sky: [
      { part: "starship", x: 34, y: 26, scale: 0.34, paint: "white" },
      { part: "starship", x: 60, y: 22, scale: 0.4, paint: "white" },
      { part: "starship", x: 72, y: 38, scale: 0.3, paint: "white" },
      { part: "fighter", x: 46, y: 40, scale: 0.9, paint: "brass-500" },
    ],
    far: [{ part: "viewport", x: 50, y: 33, scale: 1.7, paint: "slate-100" }],
    near: [
      { part: "wide-table", x: 50, y: 100, scale: 1.8, paint: "slate-100" },
      { part: "scroll", x: 50, y: 82, scale: 1.5, paint: "white" },
      { part: "coin", x: 44, y: 79.5, scale: 0.6, paint: "red-600" },
      { part: "coin", x: 56, y: 80, scale: 0.6, paint: "red-600" },
    ],
    light: [{ part: "light-shaft", x: 50, y: 30, scale: 1.4, paint: "sky-300" }],
    particles: [{ part: "motes", x: 50, y: 60, scale: 1.4, paint: "sky-300" }],
  },
};

/** Faction: Pegasus (Stinkyscape #161, Rare): a tailor fits a winged-horse emblem cape onto a stylish recruit in a sky-yacht boutique, sleek winged ships gliding past the window; soft lilac light; the cape is the stealth cloak with the card's emblem on it, the tailor's pin her chalk. */
const FACTION_PEGASUS: SceneRecipe = {
  family: "stinkyscape",
  sky: ["violet-700", "violet-300"],
  light: { x: 64, y: 32, r: 38, paint: "violet-300", opacity: 0.35 },
  emblem: { x: 44, y: 57, size: 5, rotate: 0 },
  figure: { x: 44, y: 90, scale: 0.95, pose: "pose-hips", look: { ...BASE, hair: "hair-spiky", outfit: "outfit-cloak", item: null, skin: "brass-600", cloth: "violet-500" } },
  extras: [
    { layer: "subject", x: 64, y: 90, scale: 0.84, flip: -1, pose: "pose-hold", look: { ...BASE, hair: "spectacles", outfit: "outfit-frock", item: "item-chalk", skin: "red-300", cloth: "slate-700" } },
  ],
  layers: {
    sky: [
      { part: "stars", x: 64, y: 30, scale: 1.2, paint: "white" },
      { part: "fighter", x: 54, y: 28, scale: 1, flip: -1, paint: "white" },
      { part: "fighter", x: 76, y: 38, scale: 0.8, flip: -1, paint: "slate-100" },
      { part: "speed-streaks", x: 60, y: 28, scale: 0.6, flip: -1, paint: "white" },
    ],
    far: [
      { part: "viewport", x: 64, y: 34, scale: 1.1, paint: "slate-100" },
      { part: "drapes", x: 64, y: 12, scale: 1.1, paint: "purple-400" },
      { part: "shelf", x: 16, y: 58, scale: 0.9, paint: "violet-500" },
    ],
    mid: [
      { part: "checker-floor", x: 50, y: 78, paint: "violet-700" },
      { part: "mirror", x: 86, y: 88, scale: 1.2, paint: "slate-100" },
    ],
    near: [
      { part: "chair", x: 14, y: 98, scale: 1.3, paint: "purple-400" },
      { part: "hat", x: 22, y: 86, scale: 1.1 },
    ],
    light: [
      { part: "spotlight", x: 44, y: 4, scale: 1.1, paint: "violet-300" },
      { part: "rim-light", x: 44, y: 62, scale: 1.4, paint: "violet-300" },
    ],
    particles: [
      { part: "motes", x: 50, y: 56, scale: 1.4, paint: "violet-300" },
      { part: "sparks", x: 46, y: 58, scale: 0.8, paint: "white" },
    ],
  },
};

/** Faction: Star-Jumper Core (Stinkyscape #162, Rare): five mismatched ships warp out together in formation from a glowing star-jump core gate, a blue jump-flash rippling around them. */
const FACTION_STAR_JUMPER_CORE: SceneRecipe = {
  family: "stinkyscape",
  sky: ["black", "blue-900"],
  light: { x: 42, y: 46, r: 38, paint: "sky-400", opacity: 0.32 },
  emblem: { x: 57, y: 47, size: 5, rotate: 0 },
  layers: {
    sky: [
      { part: "stars", x: 50, y: 24, scale: 2 },
      { part: "stars", x: 50, y: 76, scale: 2 },
      { part: "galaxy", x: 84, y: 18, scale: 0.6 },
    ],
    far: [
      { part: "nebula", x: 22, y: 70, scale: 1.2, paint: "blue-600" },
      { part: "planet", x: 86, y: 80, scale: 1.3 },
    ],
    mid: [{ part: "warp-gate", x: 40, y: 46, scale: 1.4 }],
    subject: [
      { part: "starship", x: 58, y: 46, scale: 0.5 },
      { part: "fighter", x: 52, y: 31, scale: 1.1, paint: "red-600" },
      { part: "shuttle", x: 52, y: 61, scale: 1 },
      { part: "freighter", x: 72, y: 36, scale: 0.36, paint: "wood-600" },
      { part: "fighter", x: 72, y: 56, scale: 0.9, paint: "jade-400" },
    ],
    light: [
      { part: "warp-flash", x: 40, y: 46, scale: 1.5, paint: "sky-300" },
      { part: "blast-rings", x: 40, y: 46, scale: 2, paint: "blue-200" },
    ],
    particles: [
      { part: "speed-streaks", x: 46, y: 46, scale: 0.6, paint: "sky-300" },
      { part: "sparks", x: 40, y: 46, scale: 1.2, count: 3, spread: [16, 14], paint: "sky-300" },
      { part: "motes", x: 50, y: 50, scale: 1.4, paint: "blue-200" },
    ],
  },
};

/** Swear Loyalities (Stinkyscape #163, Rare): a pilot kneels on a canal moon's mooring pier, pledging to one bright banner as four other faction banners burn to ash behind; red dusk. */
const SWEAR_LOYALITIES: SceneRecipe = {
  family: "stinkyscape",
  sky: ["violet-700", "red-600"],
  light: { x: 58, y: 56, r: 34, paint: "ember-500", opacity: 0.3 },
  emblem: { x: 61, y: 57, size: 5, rotate: 0 },
  figure: { x: 38, y: 90, scale: 0.9, pose: "pose-genuflect", look: { ...BASE, hair: "goggles", outfit: "outfit-flightsuit", item: null, skin: "wood-600", cloth: "blue-800" } },
  layers: {
    sky: [
      { part: "stars", x: 50, y: 14, scale: 1.6, fade: 0.6 },
      { part: "planet", x: 80, y: 20, scale: 1.2, paint: "red-900" },
      { part: "sun-disc", x: 20, y: 56, scale: 1.1, paint: "red-500" },
    ],
    far: [{ part: "water", x: 50, y: 68, paint: "blue-900" }],
    mid: [
      { part: "boards", x: 50, y: 80, paint: "wood-700" },
      { part: "banner", x: 14, y: 80, scale: 1.2, paint: "slate-800" },
      { part: "banner", x: 26, y: 76, scale: 1, paint: "slate-800" },
      { part: "banner", x: 76, y: 76, scale: 1, flip: -1, paint: "slate-800" },
      { part: "banner", x: 88, y: 80, scale: 1.2, flip: -1, paint: "slate-800" },
      { part: "flames", x: 18, y: 66, scale: 0.9 },
      { part: "flames", x: 29, y: 63, scale: 0.8 },
      { part: "flames", x: 73, y: 63, scale: 0.8 },
      { part: "flames", x: 84, y: 66, scale: 0.9 },
    ],
    subject: [{ part: "banner", x: 54, y: 84, scale: 1.7, paint: "sky-400" }],
    near: [
      { part: "fence-post", x: 8, y: 98, scale: 1.2 },
      { part: "fence-post", x: 92, y: 98, scale: 1.2 },
    ],
    light: [
      { part: "halo", x: 60, y: 56, scale: 2, paint: "sky-300" },
      { part: "rim-light", x: 40, y: 64, scale: 1.2, paint: "orange-400" },
    ],
    particles: [
      { part: "embers", x: 50, y: 56, scale: 1.6, count: 3, spread: [34, 6] },
      { part: "smoke", x: 22, y: 60, scale: 0.9, paint: "slate-400" },
      { part: "smoke", x: 80, y: 60, scale: 0.9, paint: "slate-400" },
    ],
  },
};

/** The Syndicate (Stinkyscape #164, Epic): foreground, a smoky back-room card table in a smugglers' asteroid den, crates sliding in and out of a pneumatic chute; middle, masked brokers in emerald coats; background, one shipment vanishing into a dark airlock while another glows bright; green lamplight; the brokers' masks are visors, one a top hat's monocle. */
const THE_SYNDICATE: SceneRecipe = {
  family: "stinkyscape",
  sky: ["black", "green-900"],
  light: { x: 50, y: 22, r: 34, paint: "green-400", opacity: 0.26 },
  emblem: { x: 50, y: 64, size: 5, rotate: 0 },
  figure: { x: 50, y: 90, scale: 0.78, pose: "pose-hold", look: { ...BASE, hair: "visor", outfit: "outfit-coat", item: "item-card", skin: "red-300", cloth: "green-700" } },
  extras: [
    { layer: "mid", x: 30, y: 86, scale: 0.68, pose: "pose-hips", look: { ...BASE, hair: "top-hat", outfit: "outfit-coat", item: null, skin: "brass-600", cloth: "green-900" } },
    { layer: "mid", x: 70, y: 86, scale: 0.68, flip: -1, pose: "pose-stand", look: { ...BASE, hair: "visor", outfit: "outfit-tailcoat", item: null, skin: "wood-600", cloth: "jade-400" } },
  ],
  layers: {
    sky: [{ part: "stars", x: 50, y: 14, scale: 1.4, fade: 0.5 }],
    far: [
      { part: "cavern", x: 50, y: 76, paint: "slate-800" },
      { part: "airlock", x: 22, y: 42, scale: 0.9 },
      { part: "crate", x: 22, y: 46, scale: 0.8, fade: 0.3 },
      { part: "crate", x: 78, y: 52, scale: 1, paint: "green-400" },
    ],
    mid: [
      { part: "lantern", x: 50, y: 8, scale: 1.2 },
      { part: "halo", x: 78, y: 46, scale: 1.5, paint: "green-400" },
    ],
    near: [
      { part: "wide-table", x: 50, y: 100, scale: 1.6, paint: "green-900" },
      { part: "card-fan", x: 44, y: 83, scale: 0.6 },
      { part: "coin-stack", x: 60, y: 83, scale: 0.8 },
      { part: "intake-tube", x: 90, y: 100, scale: 1, flip: -1, paint: "slate-400" },
      { part: "crate", x: 82, y: 76, scale: 0.6 },
      { part: "crate", x: 8, y: 100, scale: 1.3 },
    ],
    light: [
      { part: "light-shaft", x: 50, y: 12, scale: 1.2, paint: "green-400" },
      { part: "rim-light", x: 50, y: 68, scale: 1.4, paint: "green-400" },
    ],
    particles: [
      { part: "smoke", x: 36, y: 82, scale: 1, paint: "slate-400" },
      { part: "smoke-billow", x: 62, y: 30, scale: 1.2, fade: 0.4 },
      { part: "motes", x: 50, y: 50, scale: 1.4, paint: "green-200" },
    ],
  },
};

/** The Trade Union (Stinkyscape #165, Rare): a busy dockworkers' union hall on a cargo station, crates swapped across a turnstile counter under plain pennants, tea urns steaming; steady blue light; the turnstile is a cog on the counter, the urn a teapot. */
const THE_TRADE_UNION: SceneRecipe = {
  family: "stinkyscape",
  sky: ["slate-900", "blue-800"],
  light: { x: 50, y: 20, r: 38, paint: "blue-200", opacity: 0.22 },
  emblem: { x: 50, y: 84, size: 5, rotate: 0 },
  figure: { x: 36, y: 88, scale: 0.82, pose: "pose-hold", look: { ...BASE, hair: "beanie", outfit: "outfit-overalls", item: null, skin: "wood-600", cloth: "blue-800" } },
  extras: [
    { layer: "subject", x: 64, y: 88, scale: 0.8, flip: -1, pose: "pose-hold", look: { ...BASE, hair: "kerchief", outfit: "outfit-work-apron", item: null, skin: "red-300", cloth: "slate-700" } },
    { layer: "mid", x: 16, y: 80, scale: 0.6, pose: "pose-lift", look: { ...BASE, hair: "hat-brim", outfit: "outfit-vest", item: "item-sack", skin: "brass-600", cloth: "wood-700" } },
  ],
  layers: {
    far: [
      { part: "room-wall", x: 50, y: 74, paint: "slate-700" },
      { part: "ceiling-lights", x: 50, y: 4 },
      { part: "shelf", x: 84, y: 66, scale: 0.9, paint: "slate-800" },
    ],
    mid: [{ part: "bunting", x: 50, y: 20, scale: 1.5, paint: "blue-600" }],
    near: [
      { part: "counter", x: 50, y: 92, scale: 1.2, paint: "slate-400" },
      { part: "cog", x: 50, y: 74, scale: 0.8, paint: "slate-400" },
      { part: "crate", x: 42, y: 80.5, scale: 0.9 },
      { part: "crate", x: 58, y: 80.5, scale: 0.8, paint: "blue-800" },
      { part: "table", x: 88, y: 100, scale: 0.8, paint: "wood-700" },
      { part: "teapot", x: 86, y: 93, scale: 0.8, paint: "slate-400" },
      { part: "cup", x: 94, y: 93, scale: 0.9 },
    ],
    light: [{ part: "rim-light", x: 50, y: 64, scale: 1.6, paint: "blue-200" }],
    particles: [
      { part: "steam", x: 86, y: 86, scale: 0.9 },
      { part: "motes", x: 50, y: 46, scale: 1.4, paint: "blue-200" },
    ],
  },
};

/** Scanning Probe Mk1 (Stinkyscape #166, Common): a small spindly probe drifts through empty space, a sonar ring pulsing from its antenna toward a faint shimmering distortion far off; dim teal light; the distortion is a faint rift. */
const SCANNING_PROBE_MK1: SceneRecipe = {
  family: "stinkyscape",
  sky: ["black", "slate-900"],
  light: { x: 42, y: 50, r: 26, paint: "cyan-400", opacity: 0.2 },
  emblem: { x: 42, y: 58, size: 5, rotate: 0 },
  layers: {
    sky: [
      { part: "stars", x: 50, y: 28, scale: 2 },
      { part: "stars", x: 50, y: 72, scale: 2 },
    ],
    far: [{ part: "space-rift", x: 80, y: 30, scale: 0.5, fade: 0.4 }],
    subject: [{ part: "probe", x: 42, y: 56, scale: 2.4 }],
    light: [
      { part: "echo-rings", x: 58, y: 42, scale: 1.4, paint: "cyan-400" },
      { part: "halo", x: 42, y: 56, scale: 1.6, paint: "cyan-400" },
    ],
    particles: [
      { part: "motes", x: 50, y: 50, scale: 1.6, paint: "cyan-400" },
      { part: "sparks", x: 80, y: 30, scale: 0.8, paint: "sky-300" },
    ],
  },
};

/** Scanning Probe Mk2 (Stinkyscape #167, Rare): a bigger twin-antenna probe hurtles through a dust nebula, two overlapping scan rings sweeping ahead toward a lucky sparkle of signal; dusty lavender light; the kit's probe carries one antenna, so its twin goes unseen. */
const SCANNING_PROBE_MK2: SceneRecipe = {
  family: "stinkyscape",
  sky: ["slate-900", "violet-700"],
  light: { x: 80, y: 32, r: 30, paint: "violet-300", opacity: 0.32 },
  emblem: { x: 44, y: 60, size: 5, rotate: 0 },
  layers: {
    sky: [
      { part: "stars", x: 50, y: 24, scale: 2 },
      { part: "stars", x: 50, y: 76, scale: 2, fade: 0.6 },
      { part: "nebula", x: 28, y: 40, scale: 1.4, paint: "violet-500" },
      { part: "nebula", x: 74, y: 72, scale: 1.2, paint: "purple-400" },
    ],
    far: [
      { part: "dust-cloud", x: 20, y: 72, scale: 2, paint: "violet-300" },
      { part: "dust-cloud", x: 66, y: 22, scale: 1.6, paint: "purple-400" },
      { part: "asteroid", x: 50, y: 50, scale: 0.6, count: 5, spread: [40, 32] },
      { part: "comet", x: 86, y: 78, scale: 0.6, flip: -1 },
    ],
    subject: [
      { part: "thruster-flame", x: 26, y: 60, scale: 0.9 },
      { part: "probe", x: 44, y: 58, scale: 3.2 },
    ],
    light: [
      { part: "echo-rings", x: 62, y: 50, scale: 1.3, paint: "cyan-400" },
      { part: "echo-rings", x: 68, y: 44, scale: 1.6, paint: "violet-300" },
      { part: "halo", x: 80, y: 32, scale: 1.4, paint: "violet-300" },
      { part: "burst", x: 80, y: 32, scale: 0.5, paint: "white" },
    ],
    particles: [
      { part: "speed-streaks", x: 24, y: 58, scale: 0.9, paint: "violet-300" },
      { part: "sparks", x: 80, y: 32, scale: 1, count: 2, spread: [4, 4], paint: "white" },
      { part: "motes", x: 50, y: 54, scale: 1.6, paint: "violet-300" },
    ],
  },
};

/** Drone Hanger (Stinkyscape #168, Rare): a cavernous drone hangar inside a carrier, rows of interceptor drones docked on racks, one launching through a blue force-field door; strobing work lights; the strobes are alarm beacons. */
const DRONE_HANGER: SceneRecipe = {
  family: "stinkyscape",
  sky: ["slate-900", "slate-800"],
  light: { x: 76, y: 60, r: 30, paint: "sky-400", opacity: 0.3 },
  emblem: { x: 52, y: 57, size: 5, rotate: 0 },
  layers: {
    far: [
      { part: "stars", x: 76, y: 60, scale: 0.6 },
      { part: "ceiling-lights", x: 50, y: 4, scale: 1.1 },
      { part: "gantry", x: 94, y: 80, scale: 1.1 },
    ],
    mid: [
      { part: "deck-floor", x: 50, y: 78, paint: "slate-700" },
      { part: "drone-rack", x: 18, y: 54, scale: 1.2 },
      { part: "drone-rack", x: 18, y: 78, scale: 1.2 },
      { part: "energy-wall", x: 76, y: 78, scale: 1, paint: "sky-400" },
    ],
    subject: [{ part: "interceptor", x: 58, y: 56, scale: 1.8 }],
    near: [
      { part: "crate", x: 6, y: 100, scale: 1.3 },
      { part: "alarm-light", x: 30, y: 97, scale: 1.2 },
      { part: "alarm-light", x: 92, y: 97, scale: 1.2, flip: -1 },
    ],
    light: [
      { part: "light-shaft", x: 30, y: 6, paint: "white" },
      { part: "rim-light", x: 58, y: 56, scale: 1.2, paint: "sky-400" },
    ],
    particles: [
      { part: "speed-streaks", x: 46, y: 56, scale: 0.7, paint: "sky-300" },
      { part: "sparks", x: 76, y: 62, scale: 1, paint: "sky-300" },
      { part: "motes", x: 50, y: 60, scale: 1.4, paint: "slate-100" },
    ],
  },
};

/** Tiny Asteroid (Stinkyscape #169, Common): a potato-sized asteroid with one glinting ore vein, a miner's lunchbox and pickaxe resting on top, floating in a sunny mining belt. */
const TINY_ASTEROID: SceneRecipe = {
  family: "stinkyscape",
  sky: ["slate-900", "blue-800"],
  light: { x: 78, y: 22, r: 32, paint: "orange-400", opacity: 0.35 },
  emblem: { x: 42, y: 66, size: 5, rotate: 0 },
  layers: {
    sky: [
      { part: "stars", x: 40, y: 30, scale: 1.8 },
      { part: "sun", x: 80, y: 20, scale: 1.1 },
    ],
    far: [{ part: "asteroid", x: 50, y: 46, scale: 0.8, count: 6, spread: [42, 32] }],
    subject: [
      { part: "asteroid", x: 50, y: 61, scale: 3.6, paint: "stone-500" },
      { part: "ore-chunk", x: 55, y: 63, scale: 0.9, paint: "sky-400" },
      { part: "pickaxe", x: 50, y: 48, scale: 1.1, paint: "red-600" },
    ],
    light: [{ part: "rim-light", x: 50, y: 60, scale: 1.3, paint: "orange-400" }],
    particles: [
      { part: "sparks", x: 56, y: 62, scale: 0.7, paint: "white" },
      { part: "motes", x: 50, y: 50, scale: 1.4, paint: "orange-400" },
    ],
  },
};

/** Medium Asteroid (Stinkyscape #170, Common): a house-sized grey asteroid laced with glowing blue ore veins, mining lasers just starting to score its surface, drifting beside a ringed planet; the veins are ore chunks set in its face. */
const MEDIUM_ASTEROID: SceneRecipe = {
  family: "stinkyscape",
  sky: ["black", "blue-900"],
  light: { x: 46, y: 56, r: 30, paint: "sky-400", opacity: 0.25 },
  emblem: { x: 38, y: 66, size: 5, rotate: 0 },
  layers: {
    sky: [
      { part: "stars", x: 50, y: 30, scale: 2 },
      { part: "planet", x: 80, y: 24, scale: 1.8 },
    ],
    subject: [
      { part: "asteroid", x: 46, y: 58, scale: 6, paint: "grey-500" },
      { part: "ore-chunk", x: 46, y: 56, scale: 1.2, count: 3, spread: [12, 10], paint: "sky-400" },
    ],
    light: [
      { part: "laser", x: 96, y: 40, scale: 0.9, flip: -1, paint: "red-500" },
      { part: "laser", x: 96, y: 70, scale: 0.8, flip: -1, paint: "red-500" },
    ],
    particles: [
      { part: "sparks", x: 62, y: 42, scale: 0.8 },
      { part: "sparks", x: 66, y: 70, scale: 0.8 },
      { part: "motes", x: 46, y: 56, scale: 1.2, paint: "sky-300" },
    ],
  },
};

/** Large Asteroid (Stinkyscape #171, Rare): a mountain-sized asteroid dwarfs a mining ship whose huge drill bites into its flank, spraying sparks and violet crystal dust; harsh white sunlight; the mining ship is a freighter with the drill on its nose. */
const LARGE_ASTEROID: SceneRecipe = {
  family: "stinkyscape",
  sky: ["black", "slate-900"],
  light: { x: 86, y: 14, r: 36, paint: "white", opacity: 0.3 },
  emblem: { x: 32, y: 46, size: 6, rotate: 0 },
  layers: {
    sky: [
      { part: "stars", x: 50, y: 30, scale: 2 },
      { part: "sun-disc", x: 88, y: 12, scale: 1.2, paint: "white" },
    ],
    subject: [
      { part: "asteroid", x: 40, y: 54, scale: 9.5 },
      { part: "bowl-crater", x: 30, y: 66, scale: 0.5 },
      { part: "crystal", x: 26, y: 34, scale: 1.2, paint: "violet-500" },
      { part: "crystal", x: 52, y: 76, scale: 1, paint: "violet-500" },
      { part: "freighter", x: 90, y: 66, scale: 0.42, flip: -1 },
      { part: "mining-drill", x: 82, y: 66, scale: 0.7, flip: -1 },
    ],
    light: [
      { part: "god-rays", x: 88, y: 10, scale: 1.2, paint: "white" },
      { part: "rim-light", x: 42, y: 52, scale: 2.8, paint: "white" },
    ],
    particles: [
      { part: "sparks", x: 64, y: 66, scale: 1, count: 3, spread: [6, 6] },
      { part: "dust-cloud", x: 66, y: 70, scale: 1, paint: "violet-300" },
      { part: "shards", x: 62, y: 62, scale: 0.8 },
    ],
  },
};

/** Huge Asteroid (Stinkyscape #172, Epic): foreground, a tiny mining crew on a rocky ridge; middle, a moon-sized asteroid with a scowling cliff face, its magenta core glowing through a crack; background, mining ships circling it like gulls round a whale; deep blue space. */
const HUGE_ASTEROID: SceneRecipe = {
  family: "stinkyscape",
  sky: ["black", "blue-900"],
  light: { x: 34, y: 40, r: 38, paint: "purple-400", opacity: 0.24 },
  emblem: { x: 74, y: 84, size: 5, rotate: 0 },
  extras: [
    { layer: "near", x: 16, y: 88, scale: 0.3, pose: "pose-point", look: { ...BASE, hair: "space-helmet", outfit: "outfit-jacket", item: null, skin: "red-300", cloth: "orange-400" } },
    { layer: "near", x: 23, y: 89, scale: 0.28, pose: "pose-hold", look: { ...BASE, hair: "visor", outfit: "outfit-jacket", item: "item-spade", skin: "wood-600", cloth: "orange-400" } },
    { layer: "near", x: 30, y: 88, scale: 0.29, flip: -1, pose: "pose-stand", look: { ...BASE, hair: "visor", outfit: "outfit-tunic", item: null, skin: "brass-600", cloth: "red-600" } },
  ],
  layers: {
    sky: [
      { part: "stars", x: 50, y: 40, scale: 2.4 },
      { part: "nebula", x: 82, y: 72, scale: 1.2, paint: "blue-600" },
    ],
    far: [
      { part: "freighter", x: 14, y: 24, scale: 0.26 },
      { part: "shuttle", x: 88, y: 30, scale: 0.6, flip: -1 },
      { part: "fighter", x: 86, y: 64, scale: 0.6, flip: -1 },
    ],
    mid: [
      { part: "halo", x: 50, y: 46, scale: 4.2, paint: "purple-400" },
      { part: "asteroid", x: 50, y: 46, scale: 0.7, count: 4, spread: [44, 34] },
    ],
    subject: [{ part: "giant-asteroid", x: 50, y: 46 }],
    near: [{ part: "regolith", x: 50, y: 84, paint: "stone-500" }],
    light: [{ part: "rim-light", x: 50, y: 46, scale: 3.6, paint: "purple-400" }],
    particles: [
      { part: "dust-cloud", x: 34, y: 36, scale: 1.2, paint: "purple-400" },
      { part: "sparks", x: 32, y: 42, scale: 1, paint: "purple-400" },
      { part: "motes", x: 50, y: 50, scale: 1.8, paint: "violet-300" },
    ],
  },
};

/** Abandoned Spice Tower (Stinkyscape #173, Rare): a crumbling trading spire on a dust asteroid, torn awnings flapping, overflowing sacks of red spice spilling down its stairs; low rusty sunlight. */
const ABANDONED_SPICE_TOWER: SceneRecipe = {
  family: "stinkyscape",
  sky: ["slate-900", "red-900"],
  light: { x: 84, y: 64, r: 34, paint: "orange-400", opacity: 0.32 },
  emblem: { x: 50, y: 58, size: 5, rotate: 0 },
  layers: {
    sky: [
      { part: "stars", x: 50, y: 18, scale: 2, fade: 0.7 },
      { part: "planet", x: 18, y: 26, scale: 1.2, paint: "red-600" },
      { part: "sun-disc", x: 86, y: 68, scale: 1.3, paint: "orange-400" },
    ],
    far: [{ part: "asteroid", x: 50, y: 40, scale: 0.8, count: 4, spread: [40, 16], paint: "wood-700" }],
    mid: [
      { part: "sand", x: 50, y: 80, paint: "wood-700" },
      { part: "banner", x: 76, y: 84, scale: 1, flip: -1, paint: "red-600" },
    ],
    subject: [{ part: "spice-tower", x: 50, y: 84 }],
    near: [
      { part: "stairs", x: 50, y: 86, scale: 0.5 },
      { part: "sacks", x: 30, y: 95, scale: 1.1 },
      { part: "sacks", x: 70, y: 97, scale: 0.9, flip: -1 },
      { part: "crate", x: 88, y: 100, scale: 1.1 },
    ],
    light: [
      { part: "rim-light", x: 50, y: 56, scale: 1.6, paint: "orange-400" },
      { part: "god-rays", x: 86, y: 66, scale: 0.7, paint: "orange-400" },
    ],
    particles: [
      { part: "dust", x: 50, y: 92, scale: 1, paint: "red-600" },
      { part: "gust", x: 36, y: 46, scale: 1, paint: "red-300" },
      { part: "motes", x: 50, y: 60, scale: 1.6, paint: "red-400" },
    ],
  },
};

/** Aerelret (Stinkyscape #174, Epic): foreground, an enemy fighter mid-charge; middle, space folds open like paper in front of it as a warp-disruptor buoy and a Paragon battleship unfold into place; background, a quiet patrol lane over a frosty planet; cyan warp light. */
const AERELRET: SceneRecipe = {
  family: "stinkyscape",
  sky: ["black", "blue-900"],
  light: { x: 54, y: 44, r: 36, paint: "cyan-400", opacity: 0.3 },
  emblem: { x: 60, y: 44, size: 5, rotate: 0 },
  layers: {
    sky: [
      { part: "stars", x: 50, y: 22, scale: 2.2 },
      { part: "stars", x: 50, y: 60, scale: 2.2, fade: 0.6 },
    ],
    far: [
      { part: "planet-limb", x: 60, y: 84, paint: "blue-200" },
      { part: "buoy", x: 16, y: 66, scale: 0.4, fade: 0.6 },
      { part: "buoy", x: 86, y: 62, scale: 0.4, fade: 0.6 },
    ],
    mid: [
      { part: "fold-seam", x: 52, y: 50, paint: "cyan-400" },
      { part: "squares", x: 52, y: 46, scale: 1, paint: "cyan-400" },
    ],
    subject: [
      { part: "starship", x: 60, y: 42, scale: 0.8, flip: -1 },
      { part: "buoy", x: 38, y: 40, scale: 1.4 },
    ],
    near: [
      { part: "fighter", x: 22, y: 76, scale: 2.4, paint: "red-600" },
      { part: "speed-streaks", x: 12, y: 78, scale: 0.9, paint: "red-300" },
    ],
    light: [
      { part: "warp-flash", x: 52, y: 44, scale: 1.4, paint: "cyan-400" },
      { part: "rim-light", x: 60, y: 42, scale: 1.4, paint: "sky-300" },
    ],
    particles: [
      { part: "shards", x: 50, y: 54, scale: 1.2 },
      { part: "motes", x: 50, y: 48, scale: 1.6, paint: "cyan-400" },
    ],
  },
};

/** Preemptive Strike (Stinkyscape #176, Legendary): in a hidden asteroid dock a dormant starship's engines blaze to life, tearing it loose of its scaffolds as it fires a broadside at an enemy fleet whose guns are only just turning, the enemy flagship's charging cannon still dark; cold blue engine light against searing white blasts; the dock is a cave's mouth, the broadside a laser and a missile. */
const PREEMPTIVE_STRIKE: SceneRecipe = {
  family: "stinkyscape",
  sky: ["black", "blue-900"],
  light: { x: 26, y: 58, r: 40, paint: "sky-300", opacity: 0.34 },
  emblem: { x: 42, y: 58, size: 6, rotate: 0 },
  layers: {
    sky: [{ part: "stars", x: 60, y: 34, scale: 1.6 }],
    far: [
      { part: "starship", x: 82, y: 34, scale: 0.42, flip: -1, paint: "slate-400" },
      { part: "turret", x: 76, y: 30, scale: 0.8, flip: -1, paint: "slate-700" },
      { part: "fighter", x: 68, y: 22, scale: 0.6, flip: -1 },
      { part: "fighter", x: 88, y: 48, scale: 0.55, flip: -1 },
    ],
    mid: [
      { part: "cave", x: 50, y: 22, scale: 1.4, paint: "slate-800" },
      { part: "gantry", x: 18, y: 86, scale: 1.2 },
      { part: "gantry", x: 64, y: 88, scale: 1.1 },
    ],
    subject: [
      { part: "thruster-flame", x: 22, y: 58, scale: 1.3, paint: "sky-300" },
      { part: "starship", x: 44, y: 56, scale: 0.9 },
      { part: "laser", x: 56, y: 52, scale: 0.9, paint: "white" },
      { part: "missile", x: 66, y: 60, scale: 0.9 },
    ],
    near: [
      { part: "asteroid", x: 6, y: 96, scale: 3.4, paint: "slate-800" },
      { part: "asteroid", x: 94, y: 98, scale: 3, paint: "slate-800" },
    ],
    light: [
      { part: "burst", x: 80, y: 32, scale: 1.1, paint: "white" },
      { part: "blast-rings", x: 80, y: 32, scale: 1, paint: "white" },
      { part: "rim-light", x: 44, y: 56, scale: 2, flip: -1, paint: "sky-300" },
    ],
    particles: [
      { part: "sparks", x: 80, y: 32, scale: 1.2, count: 2, spread: [6, 6], paint: "white" },
      { part: "smoke-billow", x: 20, y: 76, scale: 0.9, fade: 0.5 },
      { part: "motes", x: 34, y: 60, scale: 1.6, paint: "sky-300" },
    ],
  },
};

/** Drone Interceptor (Stinkyscape #T-DRONE-INTERCEPTOR, Token): a small triangular interceptor drone with one red sensor eye swaps between blade, shield and gun attachments as it darts through a rust-moon canyon; the swap shows as a faded afterimage thrusting another. */
const DRONE_INTERCEPTOR: SceneRecipe = {
  family: "stinkyscape",
  sky: ["slate-900", "red-900"],
  light: { x: 56, y: 30, r: 30, paint: "orange-400", opacity: 0.28 },
  emblem: { x: 46, y: 54, size: 5, rotate: 0 },
  layers: {
    sky: [
      { part: "stars", x: 50, y: 16, scale: 1.6 },
      { part: "planet", x: 52, y: 24, scale: 1.1, paint: "red-600" },
    ],
    far: [
      { part: "cliff", x: 24, y: 30, scale: 1.3, paint: "red-900" },
      { part: "cliff", x: 76, y: 34, scale: 1.2, flip: -1, paint: "red-900" },
    ],
    mid: [{ part: "rock-floor", x: 50, y: 84, paint: "red-600" }],
    subject: [
      { part: "interceptor", x: 32, y: 60, scale: 1.7, fade: 0.3 },
      { part: "interceptor", x: 50, y: 52, scale: 2.4 },
    ],
    light: [{ part: "rim-light", x: 50, y: 52, scale: 1.4, paint: "orange-400" }],
    particles: [
      { part: "speed-streaks", x: 38, y: 52, scale: 0.9, paint: "red-300" },
      { part: "dust", x: 50, y: 86, scale: 1.2, paint: "red-300" },
      { part: "motes", x: 50, y: 50, scale: 1.4, paint: "orange-400" },
    ],
  },
};

/** Drone Corvette (Stinkyscape #T-DRONE-CORVETTE, Token): a chunky corvette-sized drone bristling with turrets and missile racks, its sensor eye sweeping as it cruises above a frozen ocean moon; hard rim light. */
const DRONE_CORVETTE: SceneRecipe = {
  family: "stinkyscape",
  sky: ["black", "blue-900"],
  light: { x: 74, y: 28, r: 30, paint: "blue-200", opacity: 0.3 },
  emblem: { x: 42, y: 50, size: 5, rotate: 0 },
  layers: {
    sky: [
      { part: "stars", x: 50, y: 20, scale: 2 },
      { part: "planet", x: 80, y: 20, scale: 1.4, paint: "blue-600" },
    ],
    far: [
      { part: "ice-floor", x: 50, y: 76 },
      { part: "ice-block", x: 16, y: 82, scale: 0.6 },
      { part: "ice-block", x: 86, y: 80, scale: 0.5 },
    ],
    subject: [{ part: "corvette", x: 48, y: 50, scale: 1.4 }],
    light: [{ part: "rim-light", x: 48, y: 50, scale: 1.8, paint: "white" }],
    particles: [
      { part: "snow", x: 50, y: 60, scale: 1, fade: 0.5 },
      { part: "motes", x: 50, y: 50, scale: 1.4, paint: "blue-200" },
    ],
  },
};

/** Reknite (Stinkyscape #T-REKNITE, Token): on an ore barge's deck, a dull grey ore lump in a miner's glove flows into armour plates over her chest as a laser strikes; her armour is the plated parka. */
const REKNITE: SceneRecipe = {
  family: "stinkyscape",
  sky: ["black", "slate-900"],
  light: { x: 62, y: 50, r: 28, paint: "red-500", opacity: 0.24 },
  emblem: { x: 18, y: 80, size: 5, rotate: 0 },
  figure: { x: 46, y: 88, scale: 0.95, pose: "pose-hold", look: { ...BASE, hair: "hair-long", outfit: "outfit-plated-parka", item: null, skin: "red-300", cloth: "slate-400" } },
  layers: {
    sky: [
      { part: "stars", x: 50, y: 22, scale: 2 },
      { part: "asteroid", x: 70, y: 30, scale: 0.8, count: 3, spread: [24, 10] },
    ],
    far: [{ part: "freighter", x: 20, y: 40, scale: 0.4, fade: 0.8 }],
    mid: [
      { part: "deck-floor", x: 50, y: 80, paint: "slate-700" },
      { part: "crate", x: 18, y: 86, scale: 1.2, paint: "slate-700" },
    ],
    subject: [{ part: "ore-chunk", x: 52, y: 60, scale: 0.8, paint: "slate-400" }],
    light: [
      { part: "laser", x: 98, y: 54, scale: 1.1, flip: -1, paint: "red-500" },
      { part: "burst", x: 54, y: 54, scale: 0.5, paint: "red-300" },
      { part: "rim-light", x: 46, y: 64, scale: 1.4, paint: "slate-100" },
    ],
    particles: [
      { part: "sparks", x: 56, y: 54, scale: 0.9, paint: "white" },
      { part: "motes", x: 48, y: 58, scale: 0.8, paint: "slate-100" },
    ],
  },
};

/** Korrelite (Stinkyscape #T-KORRELITE, Token): a crackling orange-red korrelite shard hurled down a mine shaft pierces a raider's shield in a spray of sparks; the shaft is a rock-lined duct, the shard an ice shard painted hot. */
const KORRELITE: SceneRecipe = {
  family: "stinkyscape",
  sky: ["black", "wood-900"],
  light: { x: 68, y: 52, r: 26, paint: "ember-500", opacity: 0.34 },
  emblem: { x: 46, y: 58, size: 5, rotate: 0 },
  figure: { x: 46, y: 90, scale: 0.9, pose: "pose-block", look: { ...BASE, hair: "wild-goggles", outfit: "outfit-sleeveless", item: "item-shield", skin: "brass-600", cloth: "red-900" } },
  layers: {
    far: [
      { part: "vent-duct", x: 50, y: 36, paint: "wood-700" },
      { part: "lantern", x: 22, y: 20, scale: 0.9 },
    ],
    subject: [{ part: "ice-shard", x: 74, y: 52, scale: 1.3, flip: -1, paint: "ember-500" }],
    light: [
      { part: "zap", x: 78, y: 50, scale: 0.6, paint: "orange-400" },
      { part: "burst", x: 63, y: 56, scale: 0.7, paint: "orange-400" },
    ],
    particles: [
      { part: "sparks", x: 63, y: 56, scale: 1.2, count: 3, spread: [5, 6], paint: "orange-400" },
      { part: "embers", x: 70, y: 46, scale: 1.2, paint: "ember-500" },
      { part: "dust", x: 46, y: 90, scale: 1, paint: "wood-600" },
    ],
  },
};

/** Gellium (Stinkyscape #T-GELLIUM, Token): on a lifeboat bench, a wobbling blob of cool blue gel pressed to a pilot's wound spreads soothing light over her skin; the gel is a small orb. */
const GELLIUM: SceneRecipe = {
  family: "stinkyscape",
  sky: ["slate-900", "blue-800"],
  light: { x: 54, y: 62, r: 26, paint: "sky-300", opacity: 0.3 },
  emblem: { x: 82, y: 82, size: 5, rotate: 0 },
  figure: { x: 46, y: 88, scale: 0.95, pose: "pose-sit", look: { ...BASE, hair: "hair-bun", outfit: "outfit-flightsuit", item: null, skin: "wood-600", cloth: "orange-400" } },
  layers: {
    far: [
      { part: "bulkhead", x: 50, y: 82, paint: "slate-700" },
      { part: "stars", x: 76, y: 34, scale: 0.3 },
      { part: "porthole", x: 76, y: 34, scale: 1.4 },
    ],
    mid: [
      { part: "obs-bench", x: 46, y: 88, scale: 1.6, paint: "slate-400" },
      { part: "crate", x: 82, y: 90, scale: 1.2, paint: "slate-100" },
    ],
    subject: [{ part: "orb", x: 55, y: 66, scale: 0.35, paint: "sky-400" }],
    light: [
      { part: "heal-glow", x: 54, y: 64, scale: 1.1, paint: "sky-300" },
      { part: "rim-light", x: 46, y: 66, scale: 1.3, paint: "sky-300" },
    ],
    particles: [
      { part: "motes", x: 50, y: 60, scale: 1, paint: "sky-300" },
      { part: "bubbles", x: 58, y: 58, scale: 0.6, paint: "sky-300" },
    ],
  },
};

/** Axnit (Stinkyscape #T-AXNIT, Token): on a pirate skiff's deck a violet crystal blade slices a pirate, red life-light streaming back up the crystal into the wielder's hand; the wielder is the gloved hand reaching in from the foreground. */
const AXNIT: SceneRecipe = {
  family: "stinkyscape",
  sky: ["black", "violet-700"],
  light: { x: 52, y: 58, r: 28, paint: "red-500", opacity: 0.28 },
  emblem: { x: 84, y: 80, size: 5, rotate: 0 },
  figure: { x: 60, y: 88, scale: 0.9, flip: -1, pose: "pose-spread", look: { ...BASE, hair: "kerchief", outfit: "outfit-vest", item: "item-knife", skin: "red-300", cloth: "red-900" } },
  extras: [{ layer: "mid", x: 80, y: 82, scale: 0.6, flip: -1, pose: "pose-guard", look: { ...BASE, hair: "bucket-hat", outfit: "outfit-rags", item: "item-dagger", skin: "wood-600", cloth: "slate-700" } }],
  layers: {
    sky: [
      { part: "stars", x: 50, y: 24, scale: 2 },
      { part: "planet", x: 20, y: 22, scale: 1, paint: "violet-500" },
    ],
    mid: [
      { part: "boards", x: 50, y: 80, paint: "wood-700" },
      { part: "crate", x: 84, y: 88, scale: 1.1 },
    ],
    subject: [{ part: "blade", x: 42, y: 66, scale: 1.6, paint: "violet-500" }],
    near: [{ part: "gloved-hand", x: 12, y: 80, scale: 1.1 }],
    light: [
      { part: "zap", x: 52, y: 62, scale: 0.8, flip: -1, paint: "red-400" },
      { part: "rim-light", x: 60, y: 64, scale: 1.3, flip: -1, paint: "red-500" },
    ],
    particles: [
      { part: "motes", x: 40, y: 70, scale: 1, paint: "red-400" },
      { part: "shards", x: 50, y: 62, scale: 0.8 },
    ],
  },
};

/** Narcor (Stinkyscape #T-NARCOR, Token): a pinch of shimmering silver-blue narcor dust traded under a cloak in a deep-belt smugglers' cave, a hidden glow forming in the buyer's palm. */
const NARCOR: SceneRecipe = {
  family: "stinkyscape",
  sky: ["black", "slate-900"],
  light: { x: 50, y: 62, r: 22, paint: "sky-300", opacity: 0.3 },
  emblem: { x: 82, y: 82, size: 5, rotate: 0 },
  figure: { x: 40, y: 88, scale: 0.9, pose: "pose-hold", look: { ...BASE, hair: "hood", outfit: "outfit-cloak", item: null, skin: "slate-400", cloth: "slate-700" } },
  extras: [{ layer: "subject", x: 60, y: 88, scale: 0.88, flip: -1, pose: "pose-hold", look: { ...BASE, hair: "fedora", outfit: "outfit-coat", item: null, skin: "red-300", cloth: "blue-800" } }],
  layers: {
    sky: [
      { part: "stars", x: 50, y: 40, scale: 1.4 },
      { part: "asteroid", x: 50, y: 44, scale: 0.7, count: 3, spread: [20, 8] },
    ],
    mid: [
      { part: "cave", x: 50, y: 44, scale: 1.2, paint: "slate-800" },
      { part: "crate", x: 82, y: 90, scale: 1.2 },
    ],
    light: [
      { part: "halo", x: 50, y: 58, scale: 0.9, paint: "sky-300" },
      { part: "rim-light", x: 50, y: 62, scale: 1.2, paint: "blue-200" },
    ],
    particles: [
      { part: "sparks", x: 50, y: 58, scale: 0.8, paint: "slate-100" },
      { part: "motes", x: 50, y: 58, scale: 0.6, paint: "sky-300" },
      { part: "pebbles", x: 50, y: 92, scale: 1.2 },
    ],
  },
};

/** Red Narcor (Stinkyscape #T-RED-NARCOR, Token): in a prospector's tent on a red dune asteroid, a softly humming crimson narcor crystal spills mana sparks into her cupped hands; she stands at the lamp-lit mouth of her tent. */
const RED_NARCOR: SceneRecipe = {
  family: "stinkyscape",
  sky: ["slate-900", "red-900"],
  light: { x: 58, y: 54, r: 26, paint: "red-500", opacity: 0.32 },
  emblem: { x: 22, y: 76, size: 5, rotate: 0 },
  figure: { x: 54, y: 88, scale: 0.92, pose: "pose-hold", look: { ...BASE, hair: "hat-brim", outfit: "outfit-overalls", item: null, skin: "red-300", cloth: "wood-600" } },
  layers: {
    sky: [
      { part: "stars", x: 50, y: 22, scale: 2 },
      { part: "planet", x: 82, y: 24, scale: 1, paint: "red-600" },
    ],
    far: [{ part: "sand", x: 50, y: 76, paint: "red-600" }],
    mid: [{ part: "lamp-tent", x: 24, y: 84, scale: 1.7 }],
    subject: [{ part: "crystal", x: 60, y: 57, scale: 1.1, paint: "red-500" }],
    light: [
      { part: "halo", x: 60, y: 54, scale: 1, paint: "red-500" },
      { part: "echo-rings", x: 60, y: 54, scale: 0.8, paint: "red-300" },
    ],
    particles: [
      { part: "sparks", x: 59, y: 60, scale: 0.8, paint: "red-300" },
      { part: "motes", x: 56, y: 58, scale: 0.7, paint: "red-400" },
    ],
  },
};

/** Escort Protocol (Stinkyscape #T-AI-1, Token): a polite hovering escort AI, a white disc with a soft blue face-light and courtesy arm lights, ushers a drone into formation beside a shuttle. */
const ESCORT_PROTOCOL: SceneRecipe = {
  family: "stinkyscape",
  sky: ["black", "blue-900"],
  light: { x: 46, y: 50, r: 28, paint: "sky-300", opacity: 0.28 },
  emblem: { x: 72, y: 70, size: 5, rotate: 0 },
  layers: {
    sky: [
      { part: "stars", x: 50, y: 24, scale: 2 },
      { part: "stars", x: 50, y: 76, scale: 2, fade: 0.6 },
      { part: "planet", x: 16, y: 80, scale: 1.4, paint: "blue-600" },
    ],
    mid: [
      { part: "shuttle", x: 72, y: 70, scale: 1.8 },
      { part: "drone", x: 26, y: 36, scale: 1.2 },
    ],
    subject: [{ part: "escort-disc", x: 48, y: 50, scale: 2 }],
    light: [
      { part: "halo", x: 48, y: 50, scale: 1.8, paint: "sky-300" },
      { part: "rim-light", x: 48, y: 50, scale: 1.3, paint: "blue-200" },
    ],
    particles: [
      { part: "speed-streaks", x: 22, y: 36, scale: 0.6, paint: "sky-300" },
      { part: "motes", x: 50, y: 50, scale: 1.4, paint: "sky-300" },
    ],
  },
};

/** Mining Subroutine (Stinkyscape #T-AI-2, Token): an autonomous mining arm patiently chips at an endless rock face, its progress glow never filling, one ore chunk finally popping free; the progress glow is a light ring with dark segments. */
const MINING_SUBROUTINE: SceneRecipe = {
  family: "stinkyscape",
  sky: ["black", "slate-900"],
  light: { x: 68, y: 46, r: 26, paint: "orange-400", opacity: 0.28 },
  emblem: { x: 36, y: 80, size: 5, rotate: 0 },
  layers: {
    sky: [{ part: "stars", x: 40, y: 22, scale: 1.8 }],
    far: [
      { part: "cliff", x: 76, y: 14, scale: 1.6, flip: -1, paint: "stone-500" },
      { part: "rock-floor", x: 50, y: 84, paint: "slate-700" },
    ],
    subject: [
      { part: "robot-arm", x: 40, y: 84, scale: 1.2 },
      { part: "ore-chunk", x: 66, y: 50, scale: 0.9, paint: "sky-400" },
    ],
    light: [
      { part: "light-ring", x: 22, y: 58, scale: 0.8, paint: "orange-400" },
      { part: "burst", x: 70, y: 56, scale: 0.6, paint: "orange-400" },
    ],
    particles: [
      { part: "sparks", x: 70, y: 56, scale: 1, count: 2, spread: [4, 4], paint: "orange-400" },
      { part: "dust-cloud", x: 72, y: 62, scale: 0.9, paint: "stone-500" },
      { part: "pebbles", x: 66, y: 86, scale: 1 },
    ],
  },
};

/** Route Optimizer (Stinkyscape #T-AI-3, Token): a navigation hologram draws a bright straight line right through an asteroid, a ship following it toward a starship part waiting on the far side; the line is a cyan laser, the ship a shuttle. */
const ROUTE_OPTIMIZER: SceneRecipe = {
  family: "stinkyscape",
  sky: ["black", "blue-900"],
  light: { x: 54, y: 50, r: 30, paint: "cyan-400", opacity: 0.26 },
  emblem: { x: 32, y: 50, size: 5, rotate: 0 },
  layers: {
    sky: [
      { part: "stars", x: 50, y: 22, scale: 2 },
      { part: "stars", x: 50, y: 78, scale: 2, fade: 0.6 },
      { part: "nebula", x: 74, y: 30, scale: 1.1, paint: "blue-600" },
    ],
    mid: [
      { part: "asteroid", x: 54, y: 50, scale: 3.6, paint: "stone-500" },
      { part: "ship-module", x: 82, y: 50, scale: 1.2, flip: -1 },
      { part: "hologram", x: 20, y: 80, scale: 0.6, fade: 0.8 },
    ],
    subject: [{ part: "shuttle", x: 32, y: 50, scale: 1.4 }],
    light: [
      { part: "laser", x: 40, y: 50, scale: 1.1, paint: "cyan-400" },
      { part: "halo", x: 82, y: 50, scale: 1.2, paint: "cyan-400" },
    ],
    particles: [
      { part: "sparks", x: 54, y: 50, scale: 0.9, paint: "cyan-400" },
      { part: "motes", x: 54, y: 50, scale: 1.4, paint: "sky-300" },
    ],
  },
};

/** Predictive Shield (Stinkyscape #T-AI-4, Token): on a catwalk a pale blue predictive shield snaps up before a raider's blade lands, frost crawling up his arm as an AI eye watches; the shield is an energy wall, his blade a shock-axe. */
const PREDICTIVE_SHIELD: SceneRecipe = {
  family: "stinkyscape",
  sky: ["slate-900", "slate-800"],
  light: { x: 40, y: 56, r: 28, paint: "blue-200", opacity: 0.3 },
  emblem: { x: 60, y: 54, size: 5, rotate: 0 },
  figure: { x: 60, y: 84, scale: 0.9, flip: -1, pose: "pose-strike", look: { ...BASE, hair: "steel-helmet", outfit: "outfit-harness", item: "item-shock-axe", skin: "brass-600", cloth: "red-900" } },
  layers: {
    far: [
      { part: "reactor", x: 84, y: 76, scale: 1.1 },
      { part: "ai-eye", x: 22, y: 30, scale: 1.6 },
    ],
    mid: [{ part: "catwalk", x: 50, y: 84, paint: "slate-700" }],
    subject: [{ part: "energy-wall", x: 42, y: 84, scale: 0.8, paint: "blue-200" }],
    light: [{ part: "rim-light", x: 60, y: 62, scale: 1.3, flip: -1, paint: "blue-200" }],
    particles: [
      { part: "frost", x: 58, y: 40, scale: 0.4 },
      { part: "snow", x: 50, y: 46, scale: 0.5, fade: 0.6 },
      { part: "sparks", x: 46, y: 50, scale: 0.8, paint: "blue-200" },
    ],
  },
};

/** Launch Sequencer (Stinkyscape #T-AI-5, Token): in a launch control bunker an AI's light ring spins fast as its countdown lamps skip ahead and, through the window, a rocket's clamps release; the clamps are the gantry beside it, the lamps a strip of bulbs. */
const LAUNCH_SEQUENCER: SceneRecipe = {
  family: "stinkyscape",
  sky: ["slate-900", "blue-800"],
  light: { x: 36, y: 62, r: 26, paint: "sky-300", opacity: 0.28 },
  emblem: { x: 50, y: 82, size: 5, rotate: 0 },
  layers: {
    far: [
      { part: "bulkhead", x: 50, y: 100, paint: "slate-800" },
      { part: "stars", x: 50, y: 28, scale: 0.8 },
      { part: "gantry", x: 58, y: 44, scale: 0.6 },
      { part: "rocket", x: 64, y: 44, scale: 0.6 },
      { part: "smoke-billow", x: 64, y: 42, scale: 0.7 },
      { part: "viewport", x: 50, y: 32, scale: 1.3, paint: "slate-700" },
    ],
    mid: [{ part: "console", x: 50, y: 92, scale: 1.6 }],
    subject: [
      { part: "light-ring", x: 36, y: 62, scale: 1.4, paint: "sky-300" },
      { part: "bulbs", x: 64, y: 62, scale: 0.8 },
    ],
    light: [{ part: "halo", x: 36, y: 62, scale: 1.3, paint: "sky-300" }],
    particles: [
      { part: "sparks", x: 36, y: 62, scale: 1, paint: "sky-300" },
      { part: "motes", x: 50, y: 60, scale: 1.2, paint: "orange-400" },
    ],
  },
};

/** Probe Relay (Stinkyscape #T-AI-6, Token): a relay satellite spits out two little probes in opposite directions as one returning signal beam lights its dish; bright starlight. */
const PROBE_RELAY: SceneRecipe = {
  family: "stinkyscape",
  sky: ["black", "blue-900"],
  light: { x: 64, y: 40, r: 30, paint: "white", opacity: 0.22 },
  emblem: { x: 46, y: 52, size: 5, rotate: 0 },
  layers: {
    sky: [
      { part: "stars", x: 50, y: 22, scale: 2.2 },
      { part: "stars", x: 50, y: 70, scale: 2.2 },
      { part: "galaxy", x: 16, y: 72, scale: 0.6 },
    ],
    far: [{ part: "planet", x: 84, y: 84, scale: 1.6, paint: "blue-600" }],
    subject: [
      { part: "satellite", x: 50, y: 50, scale: 2 },
      { part: "probe", x: 20, y: 28, scale: 1.2 },
      { part: "probe", x: 82, y: 66, scale: 1.2, flip: -1 },
    ],
    light: [
      { part: "energy-beam", x: 68, y: 0, scale: 0.6, paint: "white" },
      { part: "lens-flare", x: 66, y: 44, scale: 0.5 },
    ],
    particles: [
      { part: "speed-streaks", x: 24, y: 30, scale: 0.5, flip: -1, paint: "white" },
      { part: "speed-streaks", x: 78, y: 64, scale: 0.5, paint: "white" },
      { part: "motes", x: 50, y: 50, scale: 1.4, paint: "white" },
    ],
  },
};

/** Combat Subroutine (Stinkyscape #T-AI-7, Token): a boxy combat robot with a red visor and four mismatched weapon arms tosses glowing upgrade chips to nearby drones in a scuffed sparring pit; the chips are credit chips painted cyan, the pit a sunken ring of steps. */
const COMBAT_SUBROUTINE: SceneRecipe = {
  family: "stinkyscape",
  sky: ["slate-900", "slate-800"],
  light: { x: 50, y: 36, r: 30, paint: "red-500", opacity: 0.2 },
  emblem: { x: 50, y: 66, size: 5, rotate: 0 },
  layers: {
    far: [
      { part: "ceiling-lights", x: 50, y: 4 },
      { part: "amphitheatre", x: 50, y: 82, scale: 1.1, paint: "slate-700" },
    ],
    mid: [
      { part: "drone", x: 18, y: 44, scale: 1.2 },
      { part: "drone", x: 84, y: 50, scale: 1.1 },
    ],
    subject: [
      { part: "combat-robot", x: 50, y: 84, scale: 1.5 },
      { part: "credit-chip", x: 50, y: 34, scale: 0.9, count: 3, spread: [24, 6], paint: "cyan-400" },
    ],
    light: [
      { part: "halo", x: 50, y: 34, scale: 1.4, paint: "cyan-400" },
      { part: "rim-light", x: 50, y: 62, scale: 1.6, paint: "red-500" },
    ],
    particles: [
      { part: "dust", x: 50, y: 86, scale: 1.4, paint: "stone-500" },
      { part: "sparks", x: 50, y: 36, scale: 0.8, paint: "cyan-400" },
    ],
  },
};

/** Drone Uplink (Stinkyscape #T-AI-8, Token): a mountaintop uplink array on a jungle moon, its giant dish beaming down to a pad where a fresh drone materialises; misty emerald light. */
const DRONE_UPLINK: SceneRecipe = {
  family: "stinkyscape",
  sky: ["slate-900", "green-700"],
  light: { x: 60, y: 44, r: 34, paint: "green-400", opacity: 0.3 },
  emblem: { x: 62, y: 84, size: 5, rotate: 0 },
  layers: {
    sky: [
      { part: "stars", x: 50, y: 14, scale: 1.8, fade: 0.6 },
      { part: "planet", x: 82, y: 18, scale: 1.1, paint: "jade-400" },
    ],
    far: [
      { part: "mountains", x: 50, y: 70, paint: "green-900" },
      { part: "mesa", x: 32, y: 66, scale: 0.8, paint: "green-900" },
    ],
    mid: [
      { part: "satellite-dish", x: 32, y: 50, scale: 0.9 },
      { part: "launch-pad", x: 62, y: 86, scale: 0.6 },
    ],
    subject: [{ part: "drone", x: 62, y: 72, scale: 1.4, fade: 0.7 }],
    near: [
      { part: "fern", x: 8, y: 100, scale: 1.6, paint: "green-700" },
      { part: "fern", x: 92, y: 100, scale: 1.4, paint: "green-700" },
    ],
    light: [{ part: "energy-beam", x: 62, y: 22, scale: 0.8, paint: "green-400" }],
    particles: [
      { part: "mist", x: 50, y: 74, scale: 1.4 },
      { part: "pixels", x: 62, y: 72, scale: 0.5 },
      { part: "motes", x: 50, y: 50, scale: 1.6, paint: "green-200" },
    ],
  },
};

/** Honeypot (Stinkyscape #T-AI-9, Token): an unguarded data vault glows invitingly on a derelict ship while a drone interceptor waits hidden in the ceiling shadows; green lure light; the vault is a server obelisk. */
const HONEYPOT: SceneRecipe = {
  family: "stinkyscape",
  sky: ["black", "green-900"],
  light: { x: 50, y: 66, r: 28, paint: "green-400", opacity: 0.32 },
  emblem: { x: 50, y: 74, size: 5, rotate: 0 },
  layers: {
    far: [
      { part: "bulkhead", x: 50, y: 84, paint: "slate-800" },
      { part: "pipe", x: 4, y: 16, scale: 1.2 },
    ],
    mid: [
      { part: "smoke-billow", x: 76, y: 16, scale: 1.4, paint: "slate-900" },
      { part: "interceptor", x: 78, y: 16, scale: 1.3, flip: -1, fade: 0.7 },
    ],
    subject: [{ part: "server-rack", x: 50, y: 86, scale: 1.5 }],
    light: [
      { part: "halo", x: 50, y: 64, scale: 2, paint: "green-400" },
      { part: "light-shaft", x: 50, y: 40, scale: 0.8, paint: "green-400" },
    ],
    particles: [
      { part: "motes", x: 50, y: 64, scale: 1.4, paint: "green-200" },
      { part: "glitch", x: 50, y: 50, scale: 0.5 },
      { part: "pebbles", x: 30, y: 88, scale: 1 },
    ],
  },
};

export const STINKYSCAPE_91_RECIPES_3: Readonly<Record<string, SceneRecipe>> = {
  "stinkyscape-160": FACTION_GOLDEN_ACCORD,
  "stinkyscape-161": FACTION_PEGASUS,
  "stinkyscape-162": FACTION_STAR_JUMPER_CORE,
  "stinkyscape-163": SWEAR_LOYALITIES,
  "stinkyscape-164": THE_SYNDICATE,
  "stinkyscape-165": THE_TRADE_UNION,
  "stinkyscape-166": SCANNING_PROBE_MK1,
  "stinkyscape-167": SCANNING_PROBE_MK2,
  "stinkyscape-168": DRONE_HANGER,
  "stinkyscape-169": TINY_ASTEROID,
  "stinkyscape-170": MEDIUM_ASTEROID,
  "stinkyscape-171": LARGE_ASTEROID,
  "stinkyscape-172": HUGE_ASTEROID,
  "stinkyscape-173": ABANDONED_SPICE_TOWER,
  "stinkyscape-174": AERELRET,
  "stinkyscape-176": PREEMPTIVE_STRIKE,
  "stinkyscape-t-drone-interceptor": DRONE_INTERCEPTOR,
  "stinkyscape-t-drone-corvette": DRONE_CORVETTE,
  "stinkyscape-t-reknite": REKNITE,
  "stinkyscape-t-korrelite": KORRELITE,
  "stinkyscape-t-gellium": GELLIUM,
  "stinkyscape-t-axnit": AXNIT,
  "stinkyscape-t-narcor": NARCOR,
  "stinkyscape-t-red-narcor": RED_NARCOR,
  "stinkyscape-t-ai-01": ESCORT_PROTOCOL,
  "stinkyscape-t-ai-02": MINING_SUBROUTINE,
  "stinkyscape-t-ai-03": ROUTE_OPTIMIZER,
  "stinkyscape-t-ai-04": PREDICTIVE_SHIELD,
  "stinkyscape-t-ai-05": LAUNCH_SEQUENCER,
  "stinkyscape-t-ai-06": PROBE_RELAY,
  "stinkyscape-t-ai-07": COMBAT_SUBROUTINE,
  "stinkyscape-t-ai-08": DRONE_UPLINK,
  "stinkyscape-t-ai-09": HONEYPOT,
};
