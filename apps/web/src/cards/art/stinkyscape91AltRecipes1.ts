// R3406000: the alt scene recipes of Stinkyscape's later cards (#3406), one per card of its table offered Alt art, cards
// 93 to 115 here and the rest in stinkyscape91AltRecipes2.ts and stinkyscape91AltRecipes3.ts, each drawn from its brief
// (ART.md, "Stinkyscape's later Alt art restaged"): the card's own subject in another place, time or moment, under a
// sky sharing no colour with its normal picture's and a new camera (a close-up, a wide or distant view, or one from
// above), its figure the normal picture's person (`normalLookOf`) and a person its brief keeps the normal picture's
// (`normalExtraLookOf`), in Stinkyscape's fine black line, at or above its rarity's floor. A Starship Piece, a Jlockeed
// weapon and Dice of Doom, whose normal face their own painter still draws (R3333001, R3338001), stand their piece in
// the scene as a part the pass added (sceneKitStinkyAlt.ts), under a sky that is neither black nor slate-900. scene.ts
// `sceneRecipeOf` reads one under the card's Alt art seed alone, at every Alt art level. Presentation only (CLAUDE.md
// rule 7): a recipe is read by the card's id alone.

import { figureLookFor, type FigureLook } from "./figureKit.ts";
import type { SceneRecipe } from "./scene.ts";
import { STINKYSCAPE_91_RECIPES_1 } from "./stinkyscape91Recipes1.ts";
import { STINKYSCAPE_91_RECIPES_2 } from "./stinkyscape91Recipes2.ts";
import { STINKYSCAPE_91_RECIPES_3 } from "./stinkyscape91Recipes3.ts";

/** #3403's normal recipes, by id (sceneRecipes.ts `STINKYSCAPE_91_RECIPES`, read here from its files so no import goes round). */
const NORMAL: Readonly<Record<string, SceneRecipe>> = { ...STINKYSCAPE_91_RECIPES_1, ...STINKYSCAPE_91_RECIPES_2, ...STINKYSCAPE_91_RECIPES_3 };

/** The look of the card's normal figure, so its Alt picture draws the same person; with none, the default person. */
export function normalLookOf(id: string): FigureLook {
  return NORMAL[id]?.figure?.look ?? figureLookFor([]);
}

/** The look of the card's normal `k`th extra figure, so a person its brief keeps is the same one; with none, the default person. */
export function normalExtraLookOf(id: string, k: number): FigureLook {
  return NORMAL[id]?.extras?.[k]?.look ?? figureLookFor([]);
}

/** Patcher Nanobots (Stinkyscape #93, Common), Alt art: close-up of the nanobot cloud swarming over a crash-landed pilot on a pink desert dune, welding her suit's tear shut instead of the wreck behind; hot coral afternoon. */
const PATCHER_NANOBOTS_ALT: SceneRecipe = {
  family: "stinkyscape",
  sky: ["red-300", "orange-400"],
  light: { x: 76, y: 24, r: 34, paint: "white", opacity: 0.35 },
  emblem: { x: 82, y: 72, size: 5, rotate: 0 },
  extras: [
    {
      layer: "mid",
      x: 44,
      y: 98,
      scale: 1.25,
      pose: "pose-sit",
      look: { ...figureLookFor(["Human"]), hair: "hair-long", outfit: "outfit-flightsuit", item: null, skin: "wood-700", cloth: "slate-100" },
    },
  ],
  layers: {
    sky: [{ part: "sun", x: 76, y: 24, scale: 0.8 }],
    far: [
      { part: "sand", x: 50, y: 60, paint: "red-300" },
      { part: "wreckage", x: 82, y: 58, scale: 0.7 },
      { part: "smoke", x: 84, y: 50, scale: 0.8, paint: "slate-400" },
    ],
    mid: [{ part: "sand", x: 50, y: 78, paint: "orange-400" }],
    subject: [{ part: "nanobot-swarm", x: 46, y: 64, scale: 1.05 }],
    near: [{ part: "pebbles", x: 16, y: 96, scale: 1.2 }],
    light: [{ part: "halo", x: 46, y: 66, scale: 1.1, paint: "cyan-400" }],
    particles: [
      { part: "sparks", x: 44, y: 74, scale: 0.8, paint: "white" },
      { part: "dust", x: 62, y: 90, scale: 1, paint: "red-300" },
    ],
  },
};

/** Tmack Trav (Stinkyscape #94, Epic), Alt art: after the fight: Tmack Trav sits on a heap of beaten robots atop a snowy moon ridge, axe over his shoulder; low-angle wide shot in pale lilac snowfall. */
const TMACK_TRAV_ALT: SceneRecipe = {
  family: "stinkyscape",
  sky: ["violet-300", "slate-100"],
  light: { x: 26, y: 24, r: 34, paint: "white", opacity: 0.3 },
  emblem: { x: 82, y: 30, size: 5, rotate: 0 },
  figure: { x: 52, y: 64, scale: 0.72, pose: "pose-sit", look: normalLookOf("stinkyscape-094") },
  layers: {
    sky: [{ part: "planet", x: 22, y: 24, scale: 0.9, paint: "violet-500" }],
    far: [{ part: "mountains", x: 50, y: 68, paint: "slate-100" }],
    mid: [
      { part: "snowfield", x: 50, y: 76 },
      { part: "robot-wreck", x: 20, y: 82, scale: 0.9 },
    ],
    subject: [
      { part: "robot-wreck", x: 40, y: 74, scale: 1.2, flip: -1 },
      { part: "robot-wreck", x: 64, y: 74, scale: 1.1 },
      { part: "robot-wreck", x: 52, y: 66, scale: 1.3 },
    ],
    near: [
      { part: "robot-wreck", x: 86, y: 96, scale: 1.7, flip: -1 },
      { part: "frost", x: 24, y: 98, scale: 1.4 },
    ],
    light: [{ part: "rim-light", x: 52, y: 50, scale: 0.9, paint: "white" }],
    particles: [
      { part: "snow", x: 50, y: 46, scale: 1.4, paint: "white" },
      { part: "sparks", x: 62, y: 70, scale: 0.6, paint: "cyan-400" },
    ],
  },
};

/** Ocean Tsunami (Stinkyscape #96, Rare), Alt art: Ocean Tsunami surfs a real tidal wave on a stormy ocean planet, ore crystals tumbling in the foam; wide vista under a slate-grey sky. */
const OCEAN_TSUNAMI_ALT: SceneRecipe = {
  family: "stinkyscape",
  sky: ["slate-700", "slate-400"],
  light: { x: 72, y: 28, r: 36, paint: "white", opacity: 0.25 },
  emblem: { x: 80, y: 72, size: 5, rotate: 0 },
  figure: { x: 50, y: 58, scale: 0.55, pose: "pose-lunge", look: normalLookOf("stinkyscape-096") },
  layers: {
    sky: [
      { part: "cloud", x: 30, y: 22, scale: 1.4, paint: "slate-400" },
      { part: "lightning", x: 82, y: 18, scale: 0.6, paint: "white" },
    ],
    far: [{ part: "sea", x: 50, y: 64, paint: "blue-800" }],
    mid: [{ part: "wave", x: 22, y: 76, scale: 0.8, paint: "blue-600" }],
    subject: [
      { part: "surf-wave", x: 54, y: 78, scale: 1.8 },
      { part: "crystal", x: 40, y: 72, scale: 0.8, count: 3, spread: [12, 4], paint: "sky-300" },
    ],
    near: [{ part: "spray", x: 74, y: 84, scale: 1.4 }],
    light: [{ part: "rim-light", x: 50, y: 48, scale: 0.8, paint: "white" }],
    particles: [{ part: "rain", x: 50, y: 40, scale: 1.3, paint: "slate-100" }],
  },
};

/** Asteroid Field (Stinkyscape #97, Common), Alt art: moments after: the scout ship limps out of the field's edge, hull dented and smoking, a net of glittering ore in tow, seen from far behind against a rose nebula. */
const ASTEROID_FIELD_ALT: SceneRecipe = {
  family: "stinkyscape",
  sky: ["violet-500", "red-300"],
  light: { x: 62, y: 40, r: 34, paint: "red-300", opacity: 0.35 },
  emblem: { x: 18, y: 74, size: 5, rotate: 0 },
  layers: {
    sky: [
      { part: "nebula", x: 62, y: 36, scale: 1.6, paint: "red-300" },
      { part: "stars", x: 50, y: 28, scale: 1.6 },
    ],
    far: [{ part: "asteroid", x: 18, y: 40, scale: 0.6, count: 4, spread: [14, 10] }],
    mid: [{ part: "asteroid", x: 10, y: 80, scale: 1.4 }],
    subject: [
      { part: "fighter", x: 62, y: 42, scale: 1.1 },
      { part: "cargo-net", x: 42, y: 62, scale: 0.4 },
      { part: "ore-chunk", x: 42, y: 56, scale: 0.7, count: 3, spread: [3, 2], paint: "sky-400" },
    ],
    near: [{ part: "asteroid", x: 88, y: 94, scale: 1.8 }],
    particles: [
      { part: "smoke", x: 56, y: 40, scale: 0.6, paint: "slate-400" },
      { part: "sparks", x: 64, y: 44, scale: 0.6, paint: "orange-400" },
    ],
  },
};

/** Hull Booster (Stinkyscape #98, Common), Alt art: the extra plating bolted onto a lumbering freighter mid-voyage, its engines straining in grey exhaust; side-on view above a teal ice giant. */
const HULL_BOOSTER_ALT: SceneRecipe = {
  family: "stinkyscape",
  sky: ["blue-800", "cyan-400"],
  light: { x: 30, y: 30, r: 30, paint: "white", opacity: 0.25 },
  emblem: { x: 18, y: 30, size: 5, rotate: 0 },
  layers: {
    sky: [{ part: "stars", x: 50, y: 24, scale: 1.4 }],
    far: [{ part: "planet-limb", x: 50, y: 80, paint: "jade-400" }],
    subject: [
      { part: "freighter", x: 54, y: 50, scale: 1.3 },
      { part: "hull-plate", x: 50, y: 48, scale: 0.8 },
      { part: "hull-plate", x: 64, y: 50, scale: 0.7 },
    ],
    light: [{ part: "rim-light", x: 54, y: 52, scale: 1.2, paint: "cyan-400" }],
    particles: [{ part: "smoke", x: 22, y: 52, scale: 1, paint: "slate-400" }],
  },
};

/** No-No Words (Stinkyscape #99, Common), Alt art: the aftermath close-up: a shocked grandmother claps a hand over a young pilot's mouth at a sunny family dinner on a garden station, a dented robot beside them. The figure kit has no hand-over-mouth gesture, so the grandmother raises her hand. */
const NO_NO_WORDS_ALT: SceneRecipe = {
  family: "stinkyscape",
  sky: ["sky-300", "green-200"],
  light: { x: 76, y: 22, r: 36, paint: "white", opacity: 0.4 },
  emblem: { x: 84, y: 34, size: 5, rotate: 0 },
  figure: { x: 42, y: 102, scale: 1.3, pose: "pose-stand", look: normalLookOf("stinkyscape-099") },
  extras: [
    {
      layer: "subject",
      x: 64,
      y: 102,
      scale: 1.22,
      flip: -1,
      pose: "pose-raise",
      look: { ...figureLookFor(["Human"]), hair: "hair-bun", outfit: "outfit-cardigan", item: null, skin: "red-300", cloth: "violet-500" },
    },
  ],
  layers: {
    sky: [{ part: "sun", x: 76, y: 22, scale: 0.7 }],
    far: [
      { part: "hedge", x: 50, y: 68, scale: 1.6, paint: "green-700" },
      { part: "fruit-tree", x: 14, y: 70, scale: 1.1 },
    ],
    subject: [{ part: "robot-wreck", x: 16, y: 90, scale: 0.9 }],
    near: [
      { part: "wide-table", x: 52, y: 106, scale: 2.2, paint: "white" },
      { part: "cup", x: 36, y: 94, scale: 1.2 },
    ],
    particles: [{ part: "petals", x: 50, y: 40, scale: 1, paint: "red-300" }],
  },
};

/** AP Rounds (Stinkyscape #100, Common), Alt art: a single armour-piercing round punches clean through three stacked hull plates on a white salt-flat firing range; slow-motion side view, bright noon glare. */
const AP_ROUNDS_ALT: SceneRecipe = {
  family: "stinkyscape",
  sky: ["sky-300", "white"],
  light: { x: 50, y: 16, r: 40, paint: "white", opacity: 0.5 },
  emblem: { x: 16, y: 30, size: 5, rotate: 0 },
  layers: {
    sky: [{ part: "sun-disc", x: 50, y: 14, scale: 0.7 }],
    far: [
      { part: "cracked-earth", x: 50, y: 64, paint: "slate-100" },
      { part: "range-flags", x: 84, y: 64, scale: 0.8 },
    ],
    subject: [
      { part: "hull-plate", x: 40, y: 54, scale: 0.8 },
      { part: "hull-plate", x: 48, y: 54, scale: 0.8 },
      { part: "hull-plate", x: 56, y: 54, scale: 0.8 },
      { part: "cartridge", x: 74, y: 53, scale: 1.4, paint: "slate-400" },
    ],
    light: [{ part: "lens-flare", x: 50, y: 14, scale: 0.7 }],
    particles: [
      { part: "speed-streaks", x: 26, y: 53, scale: 0.8, paint: "slate-400" },
      { part: "sparks", x: 62, y: 53, scale: 0.8, paint: "orange-400" },
      { part: "shards", x: 64, y: 56, scale: 0.6 },
    ],
  },
};

/** Overdrive Module (Stinkyscape #101, Rare), Alt art: the module overheats on a racing ship in a canyon race on a red rock planet, its engine flaring then sputtering; low chase-camera angle. */
const OVERDRIVE_MODULE_ALT: SceneRecipe = {
  family: "stinkyscape",
  sky: ["orange-400", "red-300"],
  light: { x: 30, y: 30, r: 30, paint: "white", opacity: 0.3 },
  emblem: { x: 82, y: 30, size: 5, rotate: 0 },
  layers: {
    far: [
      { part: "mesa", x: 50, y: 58, paint: "red-600" },
      { part: "cliff", x: 8, y: 70, paint: "red-900" },
      { part: "cliff", x: 92, y: 70, flip: -1, paint: "red-900" },
    ],
    mid: [
      { part: "cracked-earth", x: 50, y: 76, paint: "red-600" },
      { part: "fighter", x: 78, y: 42, scale: 0.6, paint: "sky-400" },
    ],
    subject: [
      { part: "fighter", x: 52, y: 58, scale: 2, paint: "red-500" },
      { part: "overdrive-engine", x: 32, y: 60, scale: 0.75 },
    ],
    near: [{ part: "pebbles", x: 80, y: 96, scale: 1.4 }],
    light: [{ part: "halo", x: 28, y: 60, scale: 1, paint: "ember-500" }],
    particles: [
      { part: "smoke", x: 14, y: 58, scale: 1, paint: "slate-400" },
      { part: "embers", x: 20, y: 62, scale: 1 },
      { part: "speed-streaks", x: 50, y: 74, scale: 1, paint: "white" },
    ],
  },
};

/** Vxyl (Stinkyscape #103, Rare), Alt art: Vxyl sits innocently in a sunlit orbital cafe, cloak folded on the chair, a venom dart hidden in a sugar spoon; close-up portrait in warm peach daylight. The figure kit dresses a figure in one outfit, so Vxyl keeps the cloak on, hood down, and holds the spoon. */
const VXYL_ALT: SceneRecipe = {
  family: "stinkyscape",
  sky: ["red-300", "white"],
  light: { x: 76, y: 30, r: 36, paint: "orange-400", opacity: 0.35 },
  emblem: { x: 82, y: 66, size: 5, rotate: 0 },
  figure: { x: 38, y: 106, scale: 1.45, pose: "pose-sit", look: { ...normalLookOf("stinkyscape-103"), hair: "hair-long", item: "item-spoon" } },
  layers: {
    sky: [{ part: "planet", x: 74, y: 38, scale: 0.5, paint: "orange-400" }],
    far: [
      { part: "room-wall", x: 50, y: 82, paint: "red-300" },
      { part: "window", x: 74, y: 56, scale: 1.6 },
    ],
    mid: [{ part: "chair", x: 20, y: 98, scale: 1.4, paint: "wood-700" }],
    near: [
      { part: "wide-table", x: 66, y: 106, scale: 1.6, paint: "white" },
      { part: "cup", x: 64, y: 90, scale: 1.2 },
    ],
    light: [{ part: "light-shaft", x: 74, y: 30, scale: 1, paint: "white" }],
    particles: [
      { part: "steam", x: 64, y: 82, scale: 0.6, paint: "white" },
      { part: "motes", x: 50, y: 50, scale: 1, paint: "white" },
    ],
  },
};

/** Reflective Plating (Stinkyscape #104, Rare), Alt art: the mirror-polished plate strapped to a lone sentry ship in an ice canyon, enemy lasers bouncing back off it at their shooters; wide shot in pale blue. */
const REFLECTIVE_PLATING_ALT: SceneRecipe = {
  family: "stinkyscape",
  sky: ["blue-200", "white"],
  light: { x: 50, y: 24, r: 36, paint: "white", opacity: 0.35 },
  emblem: { x: 16, y: 30, size: 5, rotate: 0 },
  layers: {
    far: [
      { part: "ice-floor", x: 50, y: 72 },
      { part: "cliff", x: 6, y: 62, paint: "blue-200" },
      { part: "cliff", x: 94, y: 62, flip: -1, paint: "blue-200" },
    ],
    mid: [
      { part: "fighter", x: 84, y: 30, scale: 0.7, flip: -1, paint: "red-600" },
      { part: "fighter", x: 86, y: 52, scale: 0.6, flip: -1, paint: "red-600" },
    ],
    subject: [
      { part: "fighter", x: 36, y: 46, scale: 1.6 },
      { part: "hull-plate", x: 48, y: 46, scale: 0.55, paint: "slate-100" },
    ],
    near: [
      { part: "ice-shard", x: 16, y: 88, scale: 1.4 },
      { part: "ice-shard", x: 84, y: 90, scale: 1.2, flip: -1 },
    ],
    light: [
      { part: "laser", x: 54, y: 42, scale: 0.7, paint: "red-500" },
      { part: "laser", x: 54, y: 48, scale: 0.6, paint: "red-500" },
    ],
    particles: [
      { part: "sparks", x: 54, y: 45, scale: 0.8, paint: "white" },
      { part: "snow", x: 50, y: 40, scale: 1, paint: "white" },
    ],
  },
};

/** Warpfield Amplifier (Stinkyscape #106, Rare), Alt art: the amplifier hums on a battleship's spine at a fleet review, every gun along the hull swinging to point forward; bird's-eye view over a jade planet. */
const WARPFIELD_AMPLIFIER_ALT: SceneRecipe = {
  family: "stinkyscape",
  sky: ["jade-400", "green-700"],
  light: { x: 50, y: 42, r: 40, paint: "violet-300", opacity: 0.3 },
  emblem: { x: 84, y: 78, size: 5, rotate: 0 },
  layers: {
    sky: [
      { part: "cloud", x: 22, y: 26, scale: 1.2, paint: "white" },
      { part: "cloud", x: 80, y: 82, scale: 1, paint: "white" },
    ],
    far: [
      { part: "planet-limb", x: 50, y: 92, paint: "green-700" },
      { part: "starship", x: 18, y: 26, scale: 0.4 },
      { part: "starship", x: 82, y: 22, scale: 0.35 },
    ],
    subject: [
      { part: "starship", x: 50, y: 56, scale: 1.5 },
      { part: "warp-ring", x: 46, y: 46, scale: 0.7 },
      { part: "turret", x: 30, y: 50, scale: 0.35 },
      { part: "turret", x: 62, y: 50, scale: 0.35 },
      { part: "turret", x: 70, y: 52, scale: 0.3 },
    ],
    light: [{ part: "halo", x: 46, y: 38, scale: 1.4, paint: "violet-300" }],
    particles: [{ part: "motes", x: 46, y: 40, scale: 1, paint: "violet-300" }],
  },
};

/** RR-5000 (Stinkyscape #107, Rare), Alt art: RR-5000 rebuilds itself in a junkyard moon at dusk, robot arms snapping new parts onto its rail gun beside a line of its discarded older shells. */
const RR_5000_ALT: SceneRecipe = {
  family: "stinkyscape",
  sky: ["violet-500", "orange-400"],
  light: { x: 80, y: 60, r: 34, paint: "orange-400", opacity: 0.35 },
  emblem: { x: 16, y: 30, size: 5, rotate: 0 },
  layers: {
    sky: [{ part: "planet", x: 22, y: 22, scale: 0.8, paint: "red-300" }],
    far: [
      { part: "scrap-heap", x: 50, y: 68 },
      { part: "robot-wreck", x: 14, y: 74, scale: 0.6 },
      { part: "robot-wreck", x: 26, y: 75, scale: 0.6 },
      { part: "robot-wreck", x: 38, y: 76, scale: 0.6 },
    ],
    mid: [{ part: "regolith", x: 50, y: 80, paint: "stone-500" }],
    subject: [
      { part: "rail-gun", x: 58, y: 86, scale: 1.4 },
      { part: "robot-arm", x: 88, y: 90, scale: 0.75, flip: -1 },
      { part: "robot-arm", x: 30, y: 92, scale: 0.65 },
    ],
    near: [{ part: "cable", x: 8, y: 98, scale: 1 }],
    light: [{ part: "rim-light", x: 58, y: 78, scale: 1, paint: "orange-400" }],
    particles: [{ part: "sparks", x: 66, y: 76, scale: 1, count: 2, spread: [6, 4], paint: "orange-400" }],
  },
};

/** xxFreshJack (Stinkyscape #109, Mythic), Alt art: before the match: xxFreshJack lounges on a hangar ramp in pale sky-blue morning light, visor up, spinning a dormant glowing card on one finger, his fighter cooling behind. */
const XXFRESHJACK_ALT: SceneRecipe = {
  family: "stinkyscape",
  sky: ["sky-300", "blue-200"],
  light: { x: 22, y: 28, r: 40, paint: "white", opacity: 0.4 },
  emblem: { x: 84, y: 74, size: 5, rotate: 0 },
  figure: { x: 34, y: 94, scale: 1.15, pose: "pose-sit", look: { ...normalLookOf("stinkyscape-109"), item: "item-card" } },
  layers: {
    sky: [
      { part: "cloud", x: 30, y: 18, scale: 1.2, paint: "white" },
      { part: "cloud", x: 78, y: 26, scale: 0.9, paint: "white" },
      { part: "sun", x: 20, y: 28, scale: 0.6 },
    ],
    far: [
      { part: "bulkhead", x: 50, y: 72, paint: "slate-400" },
      { part: "gantry", x: 90, y: 76, scale: 1.2 },
      { part: "ceiling-lights", x: 50, y: 4, scale: 1 },
      { part: "fighter", x: 86, y: 40, scale: 0.6, flip: -1 },
    ],
    mid: [
      { part: "tile-floor", x: 50, y: 76, paint: "slate-100" },
      { part: "fighter", x: 70, y: 60, scale: 2.2 },
    ],
    subject: [{ part: "ramp", x: 40, y: 96, scale: 1.4, paint: "slate-400" }],
    near: [
      { part: "crate", x: 88, y: 100, scale: 1.4, count: 2, spread: [6, 2], paint: "slate-400" },
      { part: "cable", x: 6, y: 96, scale: 1 },
      { part: "helmet", x: 58, y: 96, scale: 0.9 },
    ],
    light: [
      { part: "halo", x: 46, y: 54, scale: 0.8, paint: "cyan-400" },
      { part: "light-shaft", x: 22, y: 6, scale: 1.2, paint: "white" },
      { part: "rim-light", x: 34, y: 72, scale: 1, paint: "white" },
    ],
    particles: [
      { part: "motes", x: 46, y: 50, scale: 0.8, paint: "cyan-400" },
      { part: "steam", x: 66, y: 50, scale: 0.8, paint: "white" },
      { part: "dust", x: 50, y: 84, scale: 1, paint: "white" },
    ],
  },
};

/** iKwzki (Stinkyscape #110, Mythic), Alt art: a close-up portrait of iKwzki in a crowded docking hall, laughing as the shy engineer beside her starts to glow; deep sapphire hall lighting. */
const IKWZKI_ALT: SceneRecipe = {
  family: "stinkyscape",
  sky: ["blue-700", "blue-600"],
  light: { x: 70, y: 52, r: 40, paint: "cyan-400", opacity: 0.4 },
  emblem: { x: 84, y: 30, size: 5, rotate: 0 },
  figure: { x: 34, y: 112, scale: 1.6, pose: "pose-stand", look: normalLookOf("stinkyscape-110") },
  extras: [
    {
      layer: "subject",
      x: 72,
      y: 110,
      scale: 1.35,
      flip: -1,
      pose: "pose-hold",
      look: { ...figureLookFor(["Human"]), hair: "goggles", outfit: "outfit-overalls", item: null, skin: "wood-700", cloth: "slate-400" },
    },
  ],
  layers: {
    sky: [{ part: "ceiling-lights", x: 50, y: 4, scale: 1 }],
    far: [
      { part: "bulkhead", x: 50, y: 64, paint: "blue-800" },
      { part: "shuttle", x: 54, y: 28, scale: 0.8 },
      { part: "shuttle", x: 16, y: 34, scale: 0.45, flip: -1 },
      { part: "pipes", x: 90, y: 36, scale: 0.8 },
    ],
    mid: [
      { part: "crew-crowd", x: 52, y: 72, scale: 1.8 },
      { part: "crew-crowd", x: 18, y: 78, scale: 1.4 },
      { part: "console", x: 92, y: 82, scale: 0.9 },
    ],
    near: [
      { part: "crate", x: 6, y: 102, scale: 1.6, count: 3, spread: [4, 3], paint: "blue-800" },
      { part: "cable", x: 92, y: 98, scale: 0.9, flip: -1 },
    ],
    light: [
      { part: "halo", x: 72, y: 62, scale: 1.8, paint: "cyan-400" },
      { part: "rim-light", x: 34, y: 54, scale: 1.2, paint: "sky-300" },
      { part: "god-rays", x: 54, y: 0, scale: 1, paint: "sky-300" },
    ],
    particles: [
      { part: "motes", x: 72, y: 56, scale: 1.2, paint: "cyan-400" },
      { part: "sparks", x: 72, y: 48, scale: 1, paint: "white" },
    ],
  },
};

/** Shockwave447 (Stinkyscape #111, Mythic), Alt art: Shockwave447 alone in a cramped, tool-strewn workshop on a rust-red mining moon, sketching diagrams of light on a fogged window; over-the-shoulder view. */
const SHOCKWAVE447_ALT: SceneRecipe = {
  family: "stinkyscape",
  sky: ["red-900", "red-600"],
  light: { x: 62, y: 40, r: 40, paint: "sky-300", opacity: 0.35 },
  emblem: { x: 16, y: 30, size: 5, rotate: 0 },
  figure: { x: 24, y: 118, scale: 1.7, pose: "pose-raise", look: normalLookOf("stinkyscape-111") },
  layers: {
    sky: [{ part: "planet", x: 70, y: 28, scale: 0.6, paint: "red-300" }],
    far: [
      { part: "room-wall", x: 50, y: 92, paint: "wood-800" },
      { part: "window", x: 64, y: 70, scale: 2.4 },
    ],
    mid: [
      { part: "pipes", x: 14, y: 40, scale: 0.8 },
      { part: "shelf", x: 92, y: 62, scale: 0.9 },
      { part: "lantern", x: 40, y: 24, scale: 0.7, count: 2, spread: [10, 1] },
      { part: "console", x: 86, y: 84, scale: 0.8, paint: "wood-700" },
    ],
    subject: [{ part: "light-diagram", x: 64, y: 50, scale: 0.6, paint: "sky-300" }],
    near: [
      { part: "hammer", x: 80, y: 94, scale: 0.7 },
      { part: "pages", x: 48, y: 100, scale: 0.8 },
      { part: "cable", x: 6, y: 98, scale: 1 },
      { part: "gear", x: 64, y: 98, scale: 0.8, count: 3, spread: [6, 2], paint: "slate-400" },
      { part: "cog", x: 92, y: 86, scale: 0.8, paint: "slate-400" },
    ],
    light: [
      { part: "halo", x: 64, y: 50, scale: 1.4, paint: "sky-300" },
      { part: "rim-light", x: 28, y: 64, scale: 1.4, paint: "sky-300" },
    ],
    particles: [
      { part: "mist", x: 64, y: 54, scale: 0.6, paint: "white" },
      { part: "dust", x: 50, y: 76, scale: 1.2, paint: "red-300" },
      { part: "sparks", x: 64, y: 48, scale: 0.8, paint: "sky-300" },
    ],
  },
};

/** A Very Dandy Lad (Stinkyscape #113, Epic), Alt art: A Very Dandy Lad strolls a snowy promenade at an ice-moon resort, parasol raised, cheap souvenir drones winking out as he passes; crisp white and navy, wide tracking shot. The figure kit holds no parasol, so he strolls without one. */
const A_VERY_DANDY_LAD_ALT: SceneRecipe = {
  family: "stinkyscape",
  sky: ["blue-900", "blue-200"],
  light: { x: 70, y: 30, r: 34, paint: "white", opacity: 0.3 },
  emblem: { x: 84, y: 34, size: 5, rotate: 0 },
  figure: { x: 40, y: 84, scale: 0.6, pose: "pose-stride", look: normalLookOf("stinkyscape-113") },
  layers: {
    sky: [
      { part: "stars", x: 50, y: 18, scale: 1.2 },
      { part: "planet", x: 80, y: 20, scale: 0.7, paint: "blue-200" },
    ],
    far: [
      { part: "snowfield", x: 50, y: 64 },
      { part: "glass-dome", x: 78, y: 66, scale: 0.6 },
    ],
    mid: [
      { part: "street", x: 50, y: 74, paint: "slate-100" },
      { part: "streetlamp", x: 14, y: 82, scale: 0.9 },
      { part: "streetlamp", x: 86, y: 82, scale: 0.9 },
    ],
    subject: [
      { part: "drone", x: 56, y: 56, scale: 0.6, fade: 0.4 },
      { part: "drone", x: 66, y: 50, scale: 0.6 },
      { part: "drone", x: 26, y: 54, scale: 0.6, fade: 0.2 },
    ],
    near: [{ part: "fence", x: 50, y: 96, paint: "white" }],
    light: [
      { part: "halo", x: 14, y: 54, scale: 0.6, paint: "sky-300" },
      { part: "halo", x: 86, y: 54, scale: 0.6, paint: "sky-300" },
    ],
    particles: [
      { part: "snow", x: 50, y: 40, scale: 1.2, paint: "white" },
      { part: "sparks", x: 26, y: 54, scale: 0.6, paint: "cyan-400" },
    ],
  },
};

/** Arctic Citadel (Stinkyscape #114, Epic), Alt art: Arctic Citadel in a sweltering jungle-moon salvage camp, parka tied at his waist, heaving a wrecked hull plate onto his shoulder; humid green haze, low angle. The figure kit dresses a figure in one outfit, so he keeps the parka on, his helmet off. */
const ARCTIC_CITADEL_ALT: SceneRecipe = {
  family: "stinkyscape",
  sky: ["green-500", "lime-200"],
  light: { x: 30, y: 30, r: 36, paint: "lime-200", opacity: 0.35 },
  emblem: { x: 84, y: 30, size: 5, rotate: 0 },
  figure: { x: 58, y: 98, scale: 1.45, pose: "pose-lift", look: { ...normalLookOf("stinkyscape-114"), hair: "hair-short", item: null } },
  layers: {
    sky: [{ part: "planet", x: 22, y: 20, scale: 0.8, paint: "green-200" }],
    far: [
      { part: "tree", x: 12, y: 68, scale: 2.2, paint: "green-700" },
      { part: "tree", x: 88, y: 66, scale: 2.4, paint: "green-900" },
      { part: "wreckage", x: 34, y: 60, scale: 0.8 },
    ],
    mid: [
      { part: "rock-floor", x: 50, y: 80, paint: "green-900" },
      { part: "camp-tent", x: 22, y: 84, scale: 1.2 },
    ],
    subject: [{ part: "hull-plate", x: 62, y: 30, scale: 1.4 }],
    near: [
      { part: "fern", x: 10, y: 100, scale: 1.6 },
      { part: "fern", x: 92, y: 100, scale: 1.4, flip: -1 },
    ],
    light: [{ part: "light-shaft", x: 30, y: 8, scale: 1.2, paint: "lime-200" }],
    particles: [
      { part: "mist", x: 50, y: 70, scale: 1.4, paint: "green-200" },
      { part: "motes", x: 50, y: 50, scale: 1, paint: "lime-200" },
    ],
  },
};

/** Scornboi (Stinkyscape #115, Legendary), Alt art: Scornboi in a quiet ivory-walled office aboard his flagship, striking one more line off his blank list by a window onto a pale planet; one human aide waits, close-up. */
const SCORNBOI_ALT: SceneRecipe = {
  family: "stinkyscape",
  sky: ["slate-100", "white"],
  light: { x: 74, y: 36, r: 36, paint: "white", opacity: 0.4 },
  emblem: { x: 16, y: 30, size: 5, rotate: 0 },
  figure: { x: 40, y: 110, scale: 1.45, pose: "pose-hold", look: normalLookOf("stinkyscape-115") },
  extras: [{ layer: "mid", x: 86, y: 92, scale: 0.9, flip: -1, pose: "pose-stand", look: normalExtraLookOf("stinkyscape-115", 0) }],
  layers: {
    sky: [{ part: "stars", x: 70, y: 30, scale: 0.8 }],
    far: [
      { part: "room-wall", x: 50, y: 84, paint: "slate-100" },
      { part: "window", x: 72, y: 62, scale: 2 },
      { part: "planet", x: 72, y: 42, scale: 0.5, paint: "blue-200" },
    ],
    mid: [{ part: "bookshelf", x: 12, y: 86, scale: 1.2, paint: "wood-700" }],
    near: [
      { part: "desk", x: 68, y: 108, scale: 2.4, paint: "wood-700" },
      { part: "pages", x: 66, y: 90, scale: 0.6 },
    ],
    light: [
      { part: "light-shaft", x: 72, y: 34, scale: 1.2, paint: "white" },
      { part: "rim-light", x: 40, y: 70, scale: 1.2, paint: "white" },
    ],
    particles: [{ part: "motes", x: 60, y: 50, scale: 1, paint: "white" }],
  },
};

/** R3406000: the alt recipes of #3406's cards 93 to 115, by id. */
export const STINKYSCAPE_91_ALT_RECIPES_1: Readonly<Record<string, SceneRecipe>> = {
  "stinkyscape-093": PATCHER_NANOBOTS_ALT,
  "stinkyscape-094": TMACK_TRAV_ALT,
  "stinkyscape-096": OCEAN_TSUNAMI_ALT,
  "stinkyscape-097": ASTEROID_FIELD_ALT,
  "stinkyscape-098": HULL_BOOSTER_ALT,
  "stinkyscape-099": NO_NO_WORDS_ALT,
  "stinkyscape-100": AP_ROUNDS_ALT,
  "stinkyscape-101": OVERDRIVE_MODULE_ALT,
  "stinkyscape-103": VXYL_ALT,
  "stinkyscape-104": REFLECTIVE_PLATING_ALT,
  "stinkyscape-106": WARPFIELD_AMPLIFIER_ALT,
  "stinkyscape-107": RR_5000_ALT,
  "stinkyscape-109": XXFRESHJACK_ALT,
  "stinkyscape-110": IKWZKI_ALT,
  "stinkyscape-111": SHOCKWAVE447_ALT,
  "stinkyscape-113": A_VERY_DANDY_LAD_ALT,
  "stinkyscape-114": ARCTIC_CITADEL_ALT,
  "stinkyscape-115": SCORNBOI_ALT,
};
