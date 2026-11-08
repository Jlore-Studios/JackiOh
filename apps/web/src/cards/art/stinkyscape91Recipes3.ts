// R3403000: Stinkyscape's later scene recipes, part 3 of three (#160 to #176, the drones', the resources' and T-AI-1 to 9), one per entry of
// #3403's table, each drawn from its brief in Stinkyscape's deep-space look (ART.md, "Stinkyscape's later cards drawn
// as scenes"): its subject within x 30 to 70 and y 25 to 85, its foreground near, its middle in the mid and subject
// layers and its background far, richer the rarer. Presentation only (CLAUDE.md rule 7): a recipe is read by the
// card's id alone.

import { figureLookFor, type FigureLook } from "./figureKit.ts";
import type { SceneRecipe } from "./scene.ts";

/** The look every card of this file starts from: none of them carries a tag with people of its own, so each is the hooded traveller. */
const BASE: FigureLook = figureLookFor([]);

/** Faction: Golden Accord (Stinkyscape #160, Rare): four warship captains press seals onto a blank treaty scroll on a white flagship's marble table, three Paragons and a Falcon outside; the Paragons are starships and the Falcon a fighter beyond the viewport. */
const FACTION_GOLDEN_ACCORD: SceneRecipe = {
  family: "stinkyscape",
  sky: ["black", "blue-900"],
  light: { x: 50, y: 30, r: 36, paint: "sky-300", opacity: 0.28 },
  emblem: { x: 40, y: 70, size: 5, rotate: 0 },
  figure: { x: 40, y: 100, scale: 0.86, pose: "pose-stand", look: { ...BASE, hair: "hair-silver", outfit: "outfit-greatcoat", item: null, skin: "red-300", cloth: "white" } },
  extras: [
    { layer: "subject", x: 60, y: 100, scale: 0.86, flip: -1, pose: "pose-stand", look: { ...BASE, hair: "beard", outfit: "outfit-greatcoat", item: null, skin: "wood-700", cloth: "blue-800" } },
    { layer: "mid", x: 26, y: 96, scale: 0.78, pose: "pose-hold", look: { ...BASE, hair: "headset", outfit: "outfit-greatcoat", item: null, skin: "brass-600", cloth: "red-900" } },
    { layer: "mid", x: 74, y: 96, scale: 0.78, flip: -1, pose: "pose-hold", look: { ...BASE, hair: "steel-helmet", outfit: "outfit-greatcoat", item: null, skin: "orange-400", cloth: "slate-700" } },
  ],
  layers: {
    sky: [
      { part: "stars", x: 50, y: 26, scale: 1.8 },
      { part: "nebula", x: 30, y: 30, scale: 0.9, paint: "blue-600" },
      { part: "starship", x: 34, y: 26, scale: 0.34, paint: "white" },
      { part: "starship", x: 60, y: 22, scale: 0.4, paint: "white" },
      { part: "starship", x: 72, y: 38, scale: 0.3, paint: "white" },
      { part: "fighter", x: 46, y: 40, scale: 0.9, paint: "brass-500" },
    ],
    far: [
      { part: "viewport", x: 50, y: 33, scale: 1.7, paint: "slate-100" },
      { part: "column", x: 8, y: 92, scale: 1.1, paint: "white" },
      { part: "column", x: 92, y: 92, scale: 1.1, paint: "white" },
    ],
    near: [
      { part: "wide-table", x: 50, y: 100, scale: 1.8, paint: "slate-100" },
      { part: "scroll", x: 50, y: 82, scale: 1.5, paint: "white" },
      { part: "seal", x: 44, y: 80, scale: 0.7 },
      { part: "seal", x: 56, y: 80.5, scale: 0.7 },
    ],
    light: [
      { part: "light-shaft", x: 50, y: 30, scale: 1.4, paint: "sky-300" },
      { part: "rim-light", x: 50, y: 66, scale: 1.6, paint: "blue-200" },
    ],
    particles: [
      { part: "motes", x: 50, y: 60, scale: 1.4, paint: "sky-300" },
      { part: "sparks", x: 50, y: 78, scale: 0.8, count: 2, spread: [8, 1], paint: "white" },
    ],
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
      { part: "deck-floor", x: 50, y: 78, paint: "violet-700" },
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
    far: [{ part: "rift", x: 80, y: 30, scale: 0.5, fade: 0.4 }],
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
      { part: "crater", x: 30, y: 66, scale: 0.5 },
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

export const STINKYSCAPE_91_RECIPES_3: Readonly<Record<string, SceneRecipe>> = {
  "stinkyscape-160": FACTION_GOLDEN_ACCORD,
  "stinkyscape-161": FACTION_PEGASUS,
  "stinkyscape-162": FACTION_STAR_JUMPER_CORE,
  "stinkyscape-163": SWEAR_LOYALITIES,
  "stinkyscape-166": SCANNING_PROBE_MK1,
  "stinkyscape-169": TINY_ASTEROID,
  "stinkyscape-170": MEDIUM_ASTEROID,
  "stinkyscape-171": LARGE_ASTEROID,
};
