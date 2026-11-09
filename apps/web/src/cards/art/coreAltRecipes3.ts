// R3398000: Core's alt scene recipes, part 3 of three (#68 to #100), one per card offered Alt art, each drawn from its
// brief (ART.md, "Core's Alt art restaged"): the card's own subject in another place, time or moment, under a sky
// sharing no colour with its normal picture's and a new camera (a close-up, a wide or distant view, or one from
// above), its figure the normal picture's person (coreAltRecipes1.ts `normalCoreLookOf`), in Core's glass-orb look
// and at or above its rarity's floor. scene.ts `sceneRecipeOf` reads one under the card's Alt art seed alone, at every
// Alt art level. Presentation only (CLAUDE.md rule 7): a recipe is read by the card's id alone.

import { normalCoreExtraLookOf, normalCoreLookOf } from "./coreAltRecipes1.ts";
import { figureLookFor, type FigureLook } from "./figureKit.ts";
import type { SceneRecipe } from "./scene.ts";

/** The startled delivery golem the clamps seize on the wet street. */
const GOLEM: FigureLook = { ...figureLookFor([]), head: "head-golem", hair: "bucket-hat", outfit: "outfit-jacket", item: null, skin: "stone-500", cloth: "orange-400" };

/** The stadium's dawn cleaner, broom in hand. */
const CLEANER: FigureLook = { ...figureLookFor(["Human"]), hair: "kerchief", outfit: "outfit-work-apron", item: "item-broom", cloth: "blue-800" };

/** Twisted Sorcerer (Core #68, Common), Alt art: years before, the sorcerer as a young, unbent scholar reading in a sunny cloister garden, his staff still straight beside him; warm green, a close-up. */
const TWISTED_SORCERER_ALT: SceneRecipe = {
  family: "core",
  sky: ["green-200", "lime-200"],
  light: { x: 24, y: 20, r: 40, paint: "white", opacity: 0.45 },
  emblem: { x: 82, y: 30, size: 5, rotate: 0 },
  figure: { x: 44, y: 116, scale: 1.7, pose: "pose-hold", look: { ...normalCoreLookOf("core-068"), hair: null } },
  layers: {
    sky: [{ part: "cloud", x: 70, y: 18, scale: 1.1, paint: "white" }],
    far: [
      { part: "arch", x: 14, y: 70, scale: 1.4, paint: "stone-500" },
      { part: "arch", x: 86, y: 70, scale: 1.4, paint: "stone-500" },
      { part: "column", x: 50, y: 72, scale: 1.2, paint: "slate-100" },
    ],
    mid: [
      { part: "field", x: 50, y: 70, paint: "green-500" },
      { part: "hedge", x: 16, y: 78, scale: 1.3, paint: "green-700" },
      { part: "hedge", x: 86, y: 78, scale: 1.3, paint: "green-700" },
      { part: "flowers", x: 72, y: 80, count: 3, spread: [12, 3] },
    ],
    near: [
      { part: "tome", x: 54, y: 70, scale: 1 },
      { part: "flowers", x: 10, y: 98, scale: 1.6 },
      { part: "fern", x: 90, y: 99, scale: 1.6, paint: "green-700" },
    ],
    light: [{ part: "light-shaft", x: 24, y: 0, paint: "white" }],
    particles: [{ part: "motes", x: 50, y: 40, scale: 1.2, paint: "white" }],
  },
};

/** Reminisce (Core #72, Rare), Alt art: the old soldier on a park bench in autumn, the memory card drifting down with falling leaves into his hands; wide, rust red. */
const REMINISCE_ALT: SceneRecipe = {
  family: "core",
  sky: ["red-600", "orange-400"],
  light: { x: 76, y: 24, r: 38, paint: "orange-400", opacity: 0.4 },
  emblem: { x: 20, y: 34, size: 5, rotate: 0 },
  figure: { x: 50, y: 80, scale: 0.55, pose: "pose-sit", look: normalCoreLookOf("core-072") },
  layers: {
    sky: [{ part: "cloud", x: 30, y: 20, scale: 1.2, paint: "red-300" }],
    far: [
      { part: "hills", x: 50, y: 58, paint: "red-900" },
      { part: "tree", x: 14, y: 66, scale: 1.8, paint: "orange-400" },
      { part: "tree", x: 86, y: 64, scale: 2, paint: "red-600" },
    ],
    mid: [
      { part: "field", x: 50, y: 68, paint: "brass-600" },
      { part: "road", x: 50, y: 68, paint: "wood-600" },
      { part: "streetlamp", x: 78, y: 80, scale: 0.7 },
    ],
    subject: [
      { part: "bench", x: 50, y: 77, scale: 1.4 },
      { part: "playing-card", x: 57, y: 62, scale: 0.9, paint: "white" },
    ],
    near: [
      { part: "leaves", x: 24, y: 92, scale: 1.2 },
      { part: "tall-grass", x: 88, y: 99, scale: 1.4, paint: "brass-600" },
    ],
    light: [{ part: "halo", x: 57, y: 58, scale: 1, paint: "white" }],
    particles: [
      { part: "leaves", x: 50, y: 40, scale: 1.6, count: 2, spread: [30, 10] },
      { part: "petals", x: 50, y: 50, scale: 1.2, paint: "orange-400" },
    ],
  },
};

/** Anti-oneshot Armor (Core #73, Rare), Alt art: the bastion seen far off on a green coastal headland at noon, waves breaking harmlessly on its plated seawall; wide, sea green. */
const ANTI_ONESHOT_ARMOR_ALT: SceneRecipe = {
  family: "core",
  sky: ["sky-400", "green-200"],
  light: { x: 50, y: 16, r: 40, paint: "white", opacity: 0.45 },
  emblem: { x: 22, y: 30, size: 5, rotate: 0 },
  layers: {
    sky: [
      { part: "cloud", x: 30, y: 20, scale: 1.1, paint: "white" },
      { part: "gull", x: 40, y: 30, scale: 1, count: 2, spread: [10, 4] },
    ],
    far: [{ part: "sea", x: 50, y: 56, paint: "jade-400" }],
    mid: [
      { part: "cliff", x: 56, y: 62, scale: 1.4, flip: -1, paint: "green-700" },
      { part: "hills", x: 50, y: 88, paint: "green-500" },
    ],
    subject: [
      { part: "bastion", x: 74, y: 62, scale: 0.45 },
      { part: "stone-wall", x: 56, y: 70, scale: 0.5, paint: "slate-400" },
      { part: "wave", x: 44, y: 74, scale: 0.4, paint: "cyan-400" },
    ],
    near: [
      { part: "tall-grass", x: 8, y: 99, scale: 1.4, paint: "green-500" },
      { part: "flowers", x: 90, y: 98, scale: 1.4 },
    ],
    light: [{ part: "rim-light", x: 74, y: 56, scale: 0.8, paint: "white" }],
    particles: [{ part: "spray", x: 50, y: 70, scale: 1.2, count: 2, spread: [6, 3] }],
  },
};

/** Field of Dreams (Core #76, Rare), Alt art: a midday cloud-field, soft cards drifting down like dandelion seeds onto a picnic blanket, a hand reaching up; wide, sky blue. */
const FIELD_OF_DREAMS_ALT: SceneRecipe = {
  family: "core",
  sky: ["sky-400", "slate-100"],
  light: { x: 50, y: 14, r: 44, paint: "white", opacity: 0.5 },
  emblem: { x: 82, y: 66, size: 5, rotate: 0 },
  layers: {
    sky: [
      { part: "cloud", x: 22, y: 22, scale: 1.4, paint: "white" },
      { part: "cloud", x: 78, y: 16, scale: 1.2, paint: "white" },
    ],
    far: [
      { part: "hills", x: 50, y: 60, paint: "green-400" },
      { part: "cloud", x: 14, y: 60, scale: 1.6, paint: "white" },
      { part: "cloud", x: 86, y: 58, scale: 1.5, paint: "white" },
    ],
    mid: [
      { part: "field", x: 50, y: 66, paint: "lime-400" },
      { part: "flowers", x: 20, y: 74, count: 3, spread: [10, 3] },
      { part: "gingham-blanket", x: 50, y: 82, scale: 1.4 },
    ],
    subject: [
      { part: "open-hand", x: 42, y: 80, scale: 1.4 },
      { part: "playing-card", x: 50, y: 46, scale: 0.8, count: 4, spread: [30, 14], paint: "white" },
      { part: "playing-card", x: 52, y: 70, scale: 1, paint: "blue-200" },
    ],
    near: [
      { part: "flowers", x: 10, y: 97, scale: 1.6 },
      { part: "tall-grass", x: 90, y: 99, scale: 1.4, paint: "green-500" },
    ],
    light: [{ part: "halo", x: 50, y: 50, scale: 1.6, paint: "white" }],
    particles: [
      { part: "spores", x: 50, y: 40, scale: 1.4, count: 2, spread: [30, 10], paint: "white" },
      { part: "motes", x: 50, y: 56, scale: 1.4, paint: "white" },
    ],
  },
};

/** /fullsend (Core #78, Epic), Alt art: the aftermath at dusk, the cart crashed on a quiet beach below the cliff, empty hands raised, ash flakes falling; wide, purple-pink. */
const FULLSEND_ALT: SceneRecipe = {
  family: "core",
  sky: ["violet-500", "red-300"],
  light: { x: 74, y: 56, r: 40, paint: "red-300", opacity: 0.4 },
  emblem: { x: 82, y: 30, size: 5, rotate: 0 },
  figure: { x: 60, y: 80, scale: 0.5, pose: "pose-raise", look: { ...normalCoreLookOf("core-078"), item: null } },
  layers: {
    sky: [
      { part: "cloud", x: 30, y: 20, scale: 1.2, count: 2, spread: [30, 4], paint: "violet-300" },
      { part: "sun-disc", x: 74, y: 54, scale: 0.9 },
    ],
    far: [
      { part: "sea", x: 50, y: 60, paint: "violet-700" },
      { part: "cliff", x: 22, y: 40, scale: 1.4, paint: "slate-800" },
    ],
    mid: [
      { part: "sand", x: 50, y: 70, paint: "red-300" },
      { part: "pebbles", x: 80, y: 74, scale: 1 },
    ],
    subject: [
      { part: "rocket-cart", x: 38, y: 82, scale: 1.1, flip: -1 },
      { part: "smoke", x: 32, y: 74, scale: 0.8, paint: "slate-400" },
    ],
    near: [
      { part: "shards", x: 28, y: 90, scale: 1 },
      { part: "playing-card", x: 18, y: 94, scale: 1, count: 2, spread: [8, 2], paint: "orange-400" },
      { part: "pebbles", x: 78, y: 96, scale: 1.4 },
    ],
    light: [
      { part: "halo", x: 74, y: 54, scale: 1.6, paint: "red-300" },
      { part: "rim-light", x: 60, y: 62, scale: 0.8, paint: "red-300" },
    ],
    particles: [
      { part: "embers", x: 50, y: 40, scale: 1.6, count: 3, spread: [30, 12], paint: "slate-400" },
      { part: "motes", x: 50, y: 50, scale: 1.4, paint: "slate-100" },
    ],
  },
};

/** Twinspell (Core #79, Rare), Alt art: inside the belfry, a close-up of the twin bells mid-swing, sunlight through dusty slats, echo rings rippling; warm rose. */
const TWINSPELL_ALT: SceneRecipe = {
  family: "core",
  sky: ["red-300", "wood-800"],
  light: { x: 24, y: 24, r: 38, paint: "orange-400", opacity: 0.4 },
  emblem: { x: 50, y: 80, size: 5, rotate: 0 },
  layers: {
    far: [
      { part: "column", x: 8, y: 92, scale: 3, paint: "wood-700" },
      { part: "column", x: 92, y: 92, scale: 3, paint: "wood-700" },
      { part: "fence", x: 50, y: 30, scale: 1.6, paint: "wood-700" },
    ],
    mid: [{ part: "boards", x: 50, y: 86, paint: "wood-700" }],
    subject: [
      { part: "bell", x: 34, y: 30, scale: 2.8 },
      { part: "bell", x: 66, y: 34, scale: 2.8 },
    ],
    near: [{ part: "crate", x: 12, y: 98, scale: 1.4 }],
    light: [
      { part: "light-shaft", x: 20, y: 0, scale: 1.4, paint: "white" },
      { part: "echo-rings", x: 34, y: 44, scale: 1.6, paint: "red-300" },
      { part: "echo-rings", x: 66, y: 48, scale: 1.6, paint: "red-300" },
    ],
    particles: [{ part: "motes", x: 40, y: 56, scale: 1.6, paint: "white" }],
  },
};

/** KY's Trial (Core #82, Rare), Alt art: KY on a windy hilltop holding out three drifting envelopes for a student to choose; wide, lavender sky. */
const KYS_TRIAL_ALT: SceneRecipe = {
  family: "core",
  sky: ["violet-300", "blue-200"],
  light: { x: 72, y: 22, r: 38, paint: "white", opacity: 0.4 },
  emblem: { x: 82, y: 32, size: 5, rotate: 0 },
  figure: { x: 40, y: 78, scale: 0.5, pose: "pose-hold", look: { ...normalCoreLookOf("core-082"), item: null } },
  extras: [{ layer: "subject", x: 66, y: 80, scale: 0.42, flip: -1, pose: "pose-stand", look: normalCoreExtraLookOf("core-082", 0) }],
  layers: {
    sky: [{ part: "cloud", x: 24, y: 20, scale: 1.2, paint: "white" }],
    far: [{ part: "hills", x: 50, y: 60, paint: "violet-500" }],
    mid: [
      { part: "field", x: 50, y: 72, paint: "green-500" },
      { part: "gust", x: 30, y: 40, scale: 1.2, paint: "white" },
      { part: "tree", x: 88, y: 74, scale: 1.2, paint: "green-700" },
    ],
    subject: [
      { part: "envelope", x: 50, y: 54, scale: 0.8 },
      { part: "envelope", x: 56, y: 47, scale: 0.8 },
      { part: "envelope", x: 62, y: 53, scale: 0.8 },
    ],
    near: [{ part: "grass-tuft", x: 50, y: 99, scale: 1.4, count: 3, spread: [40, 1], paint: "green-700" }],
    light: [{ part: "halo", x: 56, y: 50, scale: 1, paint: "white" }],
    particles: [{ part: "leaves", x: 50, y: 40, scale: 1.4 }],
  },
};

/** Transmogulate (Core #83, Legendary), Alt art: before the wave, the alchemist alone on a windswept salt flat, the crucible cooling, one ordinary card pinched in tongs; a vast pale-blue vista. The tongs are the card held in her hand (`item-card`), the salt flat an `ice-floor`. */
const TRANSMOGULATE_ALT: SceneRecipe = {
  family: "core",
  sky: ["blue-200", "white"],
  light: { x: 70, y: 30, r: 44, paint: "white", opacity: 0.5 },
  emblem: { x: 82, y: 30, size: 5, rotate: 0 },
  figure: { x: 42, y: 78, scale: 0.45, pose: "pose-hold", look: { ...normalCoreLookOf("core-083"), item: "item-card" } },
  layers: {
    sky: [{ part: "cloud", x: 30, y: 20, scale: 1.4, count: 3, spread: [40, 6], paint: "white" }],
    far: [{ part: "mountains", x: 50, y: 62, paint: "slate-400" }],
    mid: [
      { part: "ice-floor", x: 50, y: 68 },
      { part: "mist", x: 50, y: 70, scale: 1.2, paint: "white" },
    ],
    subject: [
      { part: "crucible", x: 54, y: 78, scale: 0.7 },
      { part: "steam", x: 54, y: 72, scale: 0.6, paint: "white" },
    ],
    near: [
      { part: "pebbles", x: 20, y: 94, scale: 1.4 },
      { part: "crystal", x: 84, y: 96, scale: 1.2, count: 3, spread: [6, 2], paint: "white" },
    ],
    light: [
      { part: "halo", x: 54, y: 74, scale: 0.6, paint: "red-300" },
      { part: "rim-light", x: 42, y: 62, scale: 0.8, paint: "white" },
    ],
    particles: [
      { part: "dust", x: 30, y: 76, scale: 1.4, paint: "white" },
      { part: "motes", x: 50, y: 50, scale: 1.4, paint: "white" },
    ],
  },
};

/** Unlicensed Experimentation (Core #85, Legendary), Alt art: the clamps springing from a manhole on a rain-washed city street, seizing a startled delivery golem and dragging it toward a sparking welding van; low angle, slate and safety orange. The manhole is the `hatch`; the experimenter steps out for the golem. */
const UNLICENSED_EXPERIMENTATION_ALT: SceneRecipe = {
  family: "core",
  sky: ["slate-700", "slate-400"],
  light: { x: 76, y: 70, r: 36, paint: "orange-400", opacity: 0.45 },
  emblem: { x: 20, y: 30, size: 5, rotate: 0 },
  extras: [{ layer: "subject", x: 44, y: 80, scale: 0.7, pose: "pose-spread", look: GOLEM }],
  layers: {
    sky: [{ part: "cloud", x: 40, y: 18, scale: 1.4, count: 2, spread: [30, 4], paint: "slate-800" }],
    far: [{ part: "skyline", x: 50, y: 62, scale: 1.1, paint: "slate-800" }],
    mid: [
      { part: "street", x: 50, y: 62, paint: "slate-800" },
      { part: "van", x: 78, y: 80, scale: 1.3, flip: -1, paint: "orange-400" },
      { part: "streetlamp", x: 12, y: 82, scale: 1 },
    ],
    subject: [
      { part: "hatch", x: 42, y: 90, scale: 1.4 },
      { part: "clamp", x: 32, y: 88, scale: 1.1 },
      { part: "clamp", x: 54, y: 88, scale: 1.1, flip: -1 },
    ],
    near: [{ part: "bin", x: 90, y: 98, scale: 1.2 }],
    light: [
      { part: "halo", x: 42, y: 86, scale: 1.2, paint: "green-400" },
      { part: "halo", x: 70, y: 74, scale: 1, paint: "orange-400" },
    ],
    particles: [
      { part: "rain", x: 50, y: 40, scale: 1.4, count: 2, spread: [20, 20], paint: "slate-100" },
      { part: "sparks", x: 68, y: 74, scale: 1.2, count: 3, spread: [6, 4], paint: "orange-400" },
    ],
  },
};

/** "Miss" Mrow (Core #86, Epic), Alt art: teatime before the fight, Miss Mrow pouring tea in a sunny fern-filled conservatory, claws politely tucked; a close-up portrait, fern green. */
const MISS_MROW_ALT: SceneRecipe = {
  family: "core",
  sky: ["green-200", "lime-200"],
  light: { x: 76, y: 20, r: 40, paint: "white", opacity: 0.5 },
  emblem: { x: 82, y: 30, size: 5, rotate: 0 },
  figure: { x: 40, y: 118, scale: 1.75, pose: "pose-hold", look: normalCoreLookOf("core-086") },
  layers: {
    sky: [{ part: "sun", x: 80, y: 14, scale: 0.6 }],
    far: [{ part: "greenhouse", x: 50, y: 74, scale: 1.4 }],
    mid: [
      { part: "tile-floor", x: 50, y: 76, paint: "green-900" },
      { part: "fern", x: 12, y: 80, scale: 2.2, paint: "green-700" },
      { part: "fern", x: 88, y: 78, scale: 2.4, paint: "green-500" },
    ],
    near: [
      { part: "pouring-teapot", x: 60, y: 70, scale: 1.4 },
      { part: "cup", x: 74, y: 84, scale: 2.2, props: true },
      { part: "fern-sprig", x: 90, y: 99, scale: 2, paint: "green-700" },
    ],
    light: [{ part: "light-shaft", x: 76, y: 0, scale: 1.4, paint: "white" }],
    particles: [{ part: "motes", x: 50, y: 40, scale: 1.4, paint: "white" }],
  },
};

/** Pocket Chaos (Core #87, Legendary), Alt art: later, the pocket lying alone on a cafe table between two empty chairs, one die still spinning; seen from high above, cherry red. */
const POCKET_CHAOS_ALT: SceneRecipe = {
  family: "core",
  sky: ["red-600", "red-900"],
  light: { x: 50, y: 30, r: 40, paint: "orange-400", opacity: 0.4 },
  emblem: { x: 82, y: 30, size: 5, rotate: 0 },
  layers: {
    sky: [
      { part: "bulbs", x: 50, y: 10, scale: 1.6 },
      { part: "bunting", x: 50, y: 20, scale: 1.4, paint: "white" },
    ],
    far: [
      { part: "window", x: 20, y: 46, scale: 1.4 },
      { part: "window", x: 80, y: 46, scale: 1.4 },
      { part: "flowers", x: 20, y: 54, scale: 0.8, count: 2, spread: [8, 0], paint: "red-300" },
      { part: "flowers", x: 80, y: 54, scale: 0.8, count: 2, spread: [8, 0], paint: "red-300" },
      { part: "shelf", x: 50, y: 40, scale: 0.9 },
    ],
    mid: [
      { part: "checker-floor", x: 50, y: 48, paint: "red-300" },
      { part: "chair", x: 22, y: 74, scale: 1.6 },
      { part: "chair", x: 78, y: 74, scale: 1.6 },
      { part: "wide-table", x: 50, y: 78, scale: 1.3 },
      { part: "cup", x: 34, y: 62, scale: 0.9 },
    ],
    subject: [
      { part: "pocket", x: 46, y: 60, scale: 1.4 },
      { part: "die", x: 60, y: 60, scale: 1.2 },
      { part: "streaks", x: 60, y: 58, scale: 0.6, paint: "white" },
    ],
    near: [
      { part: "playing-card", x: 70, y: 94, scale: 1.2, count: 2, spread: [8, 2], paint: "white" },
      { part: "die", x: 18, y: 94, scale: 1, count: 2, spread: [8, 2] },
    ],
    light: [
      { part: "halo", x: 46, y: 58, scale: 1.4, paint: "white" },
      { part: "echo-rings", x: 60, y: 62, scale: 0.6, paint: "white" },
    ],
    particles: [
      { part: "motes", x: 50, y: 50, scale: 1.4, paint: "white" },
      { part: "sparks", x: 60, y: 56, scale: 0.8, count: 2, spread: [10, 4], paint: "white" },
    ],
  },
};

/** Twisting Nether (Core #88, Epic), Alt art: a quiet countryside village at noon, its houses peeling up into a fold in a clear sky; wide, green fields and pale blue. */
const TWISTING_NETHER_ALT: SceneRecipe = {
  family: "core",
  sky: ["sky-300", "blue-200"],
  light: { x: 50, y: 30, r: 40, paint: "white", opacity: 0.5 },
  emblem: { x: 82, y: 72, size: 5, rotate: 0 },
  layers: {
    sky: [{ part: "cloud", x: 20, y: 20, scale: 1.2, paint: "white" }],
    far: [{ part: "hills", x: 50, y: 58, paint: "green-500" }],
    mid: [
      { part: "field", x: 50, y: 68, paint: "green-400" },
      { part: "house", x: 20, y: 76, scale: 0.6 },
      { part: "barn", x: 80, y: 74, scale: 0.6 },
    ],
    subject: [
      { part: "paper-tower", x: 40, y: 64, scale: 0.9 },
      { part: "paper-tower", x: 60, y: 62, scale: 0.8, flip: -1 },
      { part: "fold-seam", x: 50, y: 50 },
    ],
    near: [{ part: "sheep", x: 14, y: 94, scale: 1.2 }],
    light: [{ part: "halo", x: 50, y: 40, scale: 1.6, paint: "white" }],
    particles: [{ part: "motes", x: 50, y: 44, scale: 1.4, paint: "white" }],
  },
};

/** Corpse Eater (Core #89, Epic), Alt art: a close-up of the Corpse Eater dozing, belly swollen, in a mushroom cave lit by teal fungus; low angle, teal. */
const CORPSE_EATER_ALT: SceneRecipe = {
  family: "core",
  sky: ["slate-900", "green-900"],
  light: { x: 50, y: 60, r: 40, paint: "cyan-400", opacity: 0.45 },
  emblem: { x: 82, y: 30, size: 5, rotate: 0 },
  figure: { x: 50, y: 110, scale: 1.7, pose: "pose-sit", look: normalCoreLookOf("core-089") },
  layers: {
    far: [{ part: "cavern", x: 50, y: 80, paint: "slate-800" }],
    mid: [
      { part: "giant-mushroom", x: 16, y: 84, scale: 0.9, paint: "jade-400" },
      { part: "mushroom", x: 84, y: 80, scale: 2, count: 3, spread: [8, 3], paint: "cyan-400" },
    ],
    near: [
      { part: "mushroom", x: 10, y: 98, scale: 2.4, paint: "cyan-400" },
      { part: "mushroom", x: 90, y: 98, scale: 2.2, paint: "jade-400" },
    ],
    light: [
      { part: "halo", x: 16, y: 64, scale: 1.2, paint: "cyan-400" },
      { part: "halo", x: 84, y: 70, scale: 1, paint: "cyan-400" },
    ],
    particles: [{ part: "spore-motes", x: 50, y: 50, scale: 1.6, count: 2, spread: [30, 10], paint: "cyan-400" }],
  },
};

/** Felinor Fiender (Core #92, Legendary), Alt art: daylight, the Fiender on a park bench buried under a heap of napping Felinors, binoculars forgotten; wide, sunny grass green. */
const FELINOR_FIENDER_ALT: SceneRecipe = {
  family: "core",
  sky: ["sky-300", "lime-200"],
  light: { x: 20, y: 18, r: 40, paint: "white", opacity: 0.5 },
  emblem: { x: 82, y: 30, size: 5, rotate: 0 },
  figure: { x: 50, y: 80, scale: 0.6, pose: "pose-sit", look: { ...normalCoreLookOf("core-092"), item: null } },
  layers: {
    sky: [
      { part: "sun", x: 18, y: 16, scale: 0.7 },
      { part: "cloud", x: 70, y: 18, scale: 1.1, paint: "white" },
    ],
    far: [
      { part: "hills", x: 50, y: 58, paint: "green-500" },
      { part: "tree", x: 14, y: 64, scale: 2, paint: "green-700" },
      { part: "tree", x: 88, y: 62, scale: 1.8, paint: "green-700" },
    ],
    mid: [{ part: "field", x: 50, y: 68, paint: "green-400" }],
    subject: [{ part: "bench", x: 50, y: 77, scale: 1.5 }],
    near: [
      { part: "house-cat", x: 44, y: 74, scale: 1.6, paint: "orange-400" },
      { part: "house-cat", x: 56, y: 72, scale: 1.6, flip: -1, paint: "stone-500" },
      { part: "house-cat", x: 50, y: 64, scale: 1.4, paint: "white" },
      { part: "house-cat", x: 64, y: 77, scale: 1.4, paint: "wood-600" },
      { part: "house-cat", x: 36, y: 77, scale: 1.4, flip: -1, paint: "slate-700" },
      { part: "house-cat", x: 46, y: 57, scale: 1.2, paint: "orange-400" },
    ],
    light: [{ part: "halo", x: 50, y: 60, scale: 1.4, paint: "white" }],
    particles: [{ part: "petals", x: 50, y: 40, scale: 1.2, paint: "white" }],
  },
};

/** Combo-Index (Core #93, Legendary), Alt art: the stadium empty at first light, a cleaner sweeping cards off the lowest tier, the crown unlit; wide, cool dawn blue. The showman steps out for the cleaner. */
const COMBO_INDEX_ALT: SceneRecipe = {
  family: "core",
  sky: ["blue-600", "sky-300"],
  light: { x: 20, y: 64, r: 40, paint: "white", opacity: 0.35 },
  emblem: { x: 82, y: 28, size: 5, rotate: 0 },
  extras: [{ layer: "subject", x: 60, y: 86, scale: 0.4, pose: "pose-hold", look: CLEANER }],
  layers: {
    sky: [{ part: "cloud", x: 24, y: 20, scale: 1.2, paint: "slate-100" }],
    far: [{ part: "skyline", x: 50, y: 66, scale: 1.1, paint: "slate-700" }],
    mid: [{ part: "tile-floor", x: 50, y: 80, paint: "slate-700" }],
    subject: [
      { part: "stadium", x: 50, y: 76, scale: 0.8, paint: "slate-400" },
      { part: "crown", x: 50, y: 30, scale: 1.4, paint: "slate-400" },
      { part: "playing-card", x: 44, y: 86, scale: 0.6, count: 5, spread: [14, 2], paint: "white" },
    ],
    near: [{ part: "deck", x: 14, y: 96, scale: 1.2 }],
    light: [{ part: "halo", x: 20, y: 62, scale: 1.4, paint: "white" }],
    particles: [{ part: "dust", x: 56, y: 86, scale: 0.8, paint: "slate-100" }],
  },
};

/** Genn's Greed (Core #94, Epic), Alt art: afterwards, the noble asleep on mountains of paired silver in a sunlit tower bedroom, one odd coin rolling out of the window; seen from above, deep teal. */
const GENNS_GREED_ALT: SceneRecipe = {
  family: "core",
  sky: ["green-900", "jade-400"],
  light: { x: 78, y: 40, r: 38, paint: "white", opacity: 0.45 },
  emblem: { x: 20, y: 30, size: 5, rotate: 0 },
  figure: { x: 44, y: 74, scale: 0.7, pose: "pose-sit", look: normalCoreLookOf("core-094") },
  layers: {
    far: [
      { part: "window", x: 78, y: 52, scale: 1.6 },
      { part: "bookcase", x: 14, y: 62, scale: 1 },
    ],
    mid: [{ part: "boards", x: 50, y: 60, paint: "wood-600" }],
    subject: [
      { part: "coin-stack", x: 30, y: 82, scale: 1.6, count: 3, spread: [8, 3], paint: "slate-100" },
      { part: "coin-stack", x: 58, y: 84, scale: 1.8, count: 3, spread: [8, 3], paint: "slate-100" },
      { part: "coins", x: 44, y: 80, scale: 1.6, paint: "slate-100" },
      { part: "coin", x: 80, y: 50, scale: 1 },
    ],
    near: [
      { part: "coins", x: 16, y: 96, scale: 1.6 },
      { part: "coin-stack", x: 88, y: 98, scale: 1.6, count: 2, spread: [5, 2] },
    ],
    light: [{ part: "light-shaft", x: 78, y: 40, scale: 1.2, paint: "white" }],
    particles: [{ part: "sparks", x: 44, y: 76, scale: 1.2, count: 2, spread: [16, 6], paint: "white" }],
  },
};

/** My Pawn (Core #96, Mythic), Alt art: before the game, the mastermind's gloved hand resting on a single pawn in a quiet study, a sliver of afternoon sun; a close-up, warm brown and green. */
const MY_PAWN_ALT: SceneRecipe = {
  family: "core",
  sky: ["wood-700", "green-900"],
  light: { x: 76, y: 30, r: 36, paint: "orange-400", opacity: 0.4 },
  emblem: { x: 18, y: 30, size: 5, rotate: 0 },
  layers: {
    sky: [
      { part: "light-shaft", x: 72, y: 0, paint: "orange-400" },
      { part: "chandelier", x: 34, y: 6, scale: 0.8 },
    ],
    far: [
      { part: "room-wall", x: 50, y: 70, paint: "green-900" },
      { part: "bookcase", x: 14, y: 70, scale: 1.6 },
      { part: "bookcase", x: 86, y: 70, scale: 1.6 },
      { part: "window", x: 60, y: 44, scale: 1.2 },
      { part: "globe", x: 34, y: 62, scale: 0.9 },
      { part: "clock-face", x: 40, y: 30, scale: 0.6 },
    ],
    mid: [
      { part: "rug", x: 50, y: 92, scale: 1.6, paint: "green-700" },
      { part: "wide-table", x: 50, y: 98, scale: 2.4 },
      { part: "chessboard", x: 50, y: 74, scale: 0.6 },
      { part: "closed-tome", x: 26, y: 76, scale: 1, count: 2, spread: [4, 3] },
      { part: "hourglass", x: 72, y: 74, scale: 0.9 },
      { part: "quill", x: 80, y: 76, scale: 0.9 },
    ],
    subject: [
      { part: "chess-piece", x: 50, y: 82, scale: 3.4, paint: "white" },
      { part: "puppet-hand", x: 50, y: 27, scale: 1.4 },
    ],
    near: [
      { part: "chess-piece", x: 20, y: 94, scale: 2 },
      { part: "chess-piece", x: 10, y: 98, scale: 1.8, paint: "white" },
      { part: "taper", x: 84, y: 84, scale: 2 },
      { part: "scroll", x: 70, y: 96, scale: 1.2 },
    ],
    light: [
      { part: "rim-light", x: 50, y: 60, scale: 1.2, paint: "orange-400" },
      { part: "halo", x: 84, y: 70, scale: 0.7, paint: "orange-400" },
    ],
    particles: [
      { part: "motes", x: 64, y: 40, scale: 1.4, paint: "orange-400" },
      { part: "dust", x: 70, y: 30, scale: 1.2, count: 2, spread: [14, 16], paint: "orange-400" },
    ],
  },
};

/** Zephrys (Core #97, Mythic), Alt art: the wind spirit pondering atop a red sandstone canyon at sunset, cheek on fist, one card fluttering in a gust; a close-up portrait, deep red. */
const ZEPHRYS_ALT: SceneRecipe = {
  family: "core",
  sky: ["red-900", "orange-400"],
  light: { x: 76, y: 60, r: 40, paint: "orange-400", opacity: 0.5 },
  emblem: { x: 18, y: 30, size: 5, rotate: 0 },
  figure: { x: 46, y: 112, scale: 1.7, pose: "pose-perch", look: normalCoreLookOf("core-097") },
  layers: {
    sky: [
      { part: "cloud", x: 24, y: 22, scale: 1.2, count: 3, spread: [24, 6], paint: "orange-400" },
      { part: "cloud", x: 80, y: 14, scale: 0.9, count: 2, spread: [14, 4], paint: "red-600" },
      { part: "sun-disc", x: 76, y: 62, scale: 1 },
      { part: "crow", x: 30, y: 36, scale: 0.6, count: 3, spread: [16, 6], paint: "red-900" },
    ],
    far: [
      { part: "mountains", x: 50, y: 66, paint: "red-600" },
      { part: "mesa", x: 16, y: 72, scale: 1.4, paint: "red-600" },
      { part: "mesa", x: 86, y: 68, scale: 1.2, flip: -1, paint: "red-900" },
      { part: "mesa", x: 64, y: 70, scale: 0.8, paint: "ember-500" },
    ],
    mid: [
      { part: "sand", x: 50, y: 76, paint: "red-900" },
      { part: "cactus", x: 14, y: 84, scale: 0.9, paint: "green-900" },
      { part: "standing-stone", x: 88, y: 86, scale: 0.6, paint: "ember-500" },
      { part: "dead-tree", x: 78, y: 82, scale: 0.7, paint: "red-900" },
      { part: "mesa", x: 46, y: 112, scale: 1.4, paint: "red-600" },
    ],
    subject: [
      { part: "playing-card", x: 76, y: 40, scale: 1.2, paint: "white" },
      { part: "gust", x: 72, y: 44, scale: 1, paint: "white" },
    ],
    near: [
      { part: "dust-cloud", x: 82, y: 92, scale: 1.4 },
      { part: "pebbles", x: 18, y: 96, scale: 1.6, paint: "red-600" },
      { part: "tall-grass", x: 8, y: 98, scale: 1.2, paint: "brass-600" },
    ],
    light: [
      { part: "rim-light", x: 46, y: 50, scale: 1.6, paint: "orange-400" },
      { part: "god-rays", x: 76, y: 62, scale: 1.2, paint: "orange-400" },
    ],
    particles: [
      { part: "dust", x: 50, y: 60, scale: 1.4, paint: "orange-400" },
      { part: "streaks", x: 70, y: 30, scale: 1, count: 2, spread: [16, 12], paint: "white" },
      { part: "leaves", x: 84, y: 48, scale: 0.8, paint: "red-300" },
    ],
  },
};

/** Heroic Power (Core #98, Mythic), Alt art: outside the rotunda, a lone hero choosing a door on a misty cliff stair, the doors glowing far above; wide, cool slate and moss. */
const HEROIC_POWER_ALT: SceneRecipe = {
  family: "core",
  sky: ["slate-700", "green-900"],
  light: { x: 50, y: 20, r: 40, paint: "white", opacity: 0.4 },
  emblem: { x: 82, y: 72, size: 5, rotate: 0 },
  figure: { x: 50, y: 84, scale: 0.4, pose: "pose-stand", look: normalCoreLookOf("core-098") },
  layers: {
    sky: [
      { part: "cloud", x: 30, y: 14, scale: 1.2, count: 2, spread: [40, 4], paint: "slate-400" },
      { part: "crow", x: 74, y: 12, scale: 0.5, count: 3, spread: [12, 4], paint: "slate-900" },
    ],
    far: [
      { part: "cliff", x: 24, y: 36, scale: 1.6, paint: "green-900" },
      { part: "cliff", x: 76, y: 36, scale: 1.6, flip: -1, paint: "slate-700" },
      { part: "pine", x: 12, y: 40, scale: 0.7, count: 2, spread: [8, 3], paint: "green-900" },
      { part: "pine", x: 88, y: 40, scale: 0.7, count: 2, spread: [8, 3], paint: "green-900" },
      { part: "door", x: 38, y: 32, scale: 0.6 },
      { part: "doorway", x: 50, y: 28, scale: 0.6 },
      { part: "door", x: 62, y: 32, scale: 0.6 },
    ],
    mid: [
      { part: "stairs", x: 50, y: 40, paint: "slate-400" },
      { part: "lantern", x: 34, y: 62, scale: 0.6 },
      { part: "lantern", x: 66, y: 62, scale: 0.6 },
      { part: "grass-tuft", x: 50, y: 70, count: 3, spread: [34, 10], paint: "green-700" },
      { part: "mist", x: 50, y: 60, scale: 1.4, paint: "white" },
    ],
    near: [
      { part: "fern", x: 10, y: 98, scale: 2, paint: "green-700" },
      { part: "fern", x: 90, y: 98, scale: 2, paint: "green-700" },
      { part: "standing-stone", x: 22, y: 98, scale: 0.8, paint: "slate-700" },
    ],
    light: [
      { part: "halo", x: 38, y: 28, scale: 0.6, paint: "orange-400" },
      { part: "halo", x: 50, y: 24, scale: 0.6, paint: "orange-400" },
      { part: "halo", x: 62, y: 28, scale: 0.6, paint: "orange-400" },
    ],
    particles: [{ part: "motes", x: 50, y: 50, scale: 1.4, paint: "white" }],
  },
};

/** Craft a Card (Core #99, Mythic), Alt art: after the forge, the craftsman asleep at a cluttered attic workbench, the fused card propped against a glue pot; a close-up, deep plum and grey. The glue pot is a `jar`. */
const CRAFT_A_CARD_ALT: SceneRecipe = {
  family: "core",
  sky: ["violet-700", "slate-700"],
  light: { x: 64, y: 70, r: 36, paint: "orange-400", opacity: 0.35 },
  emblem: { x: 18, y: 30, size: 5, rotate: 0 },
  figure: { x: 34, y: 110, scale: 1.6, pose: "pose-sit", look: normalCoreLookOf("core-099") },
  layers: {
    sky: [
      { part: "light-shaft", x: 60, y: 30, scale: 1.2, paint: "violet-300" },
      { part: "stars", x: 60, y: 40, scale: 0.6, paint: "white" },
    ],
    far: [
      { part: "room-wall", x: 50, y: 78, paint: "violet-700" },
      { part: "window", x: 60, y: 44, scale: 1.2 },
      { part: "shelf", x: 84, y: 44, scale: 1 },
      { part: "shelf", x: 84, y: 62, scale: 1 },
      { part: "cog", x: 12, y: 30, scale: 1.2, count: 2, spread: [8, 8], paint: "slate-400" },
      { part: "lantern", x: 40, y: 20, scale: 0.8 },
    ],
    mid: [
      { part: "boards", x: 50, y: 78, paint: "wood-800" },
      { part: "crate", x: 88, y: 84, scale: 1, count: 2, spread: [6, 6] },
      { part: "anvil", x: 12, y: 84, scale: 0.8 },
    ],
    near: [
      { part: "wide-table", x: 72, y: 100, scale: 1.8 },
      { part: "jar", x: 66, y: 80, scale: 1.4 },
      { part: "playing-card", x: 72, y: 80, scale: 1.4, paint: "white" },
      { part: "taper", x: 56, y: 80, scale: 1.6 },
      { part: "hammer", x: 84, y: 82, scale: 1 },
      { part: "card", x: 90, y: 84, scale: 0.9, count: 2, spread: [6, 2] },
      { part: "quill", x: 78, y: 80, scale: 1 },
    ],
    light: [
      { part: "halo", x: 56, y: 72, scale: 0.8, paint: "orange-400" },
      { part: "rim-light", x: 34, y: 80, scale: 1.4, paint: "violet-300" },
    ],
    particles: [
      { part: "motes", x: 50, y: 50, scale: 1.4, paint: "white" },
      { part: "dust", x: 62, y: 44, scale: 1.2, count: 2, spread: [10, 14], paint: "violet-300" },
      { part: "sparks", x: 72, y: 76, scale: 0.8, paint: "orange-400" },
    ],
  },
};

/** Ceaseless Void (Core #100, Mythic), Alt art: before it wakes, a tiny black pearl in a sunlit tide pool, the water around it bending inward; an extreme close-up, bright coral and sea green. The giant stays in its normal picture; the pearl is an `orb` painted black and the coral is ferns and bushes in coral paint. */
const CEASELESS_VOID_ALT: SceneRecipe = {
  family: "core",
  sky: ["cyan-400", "jade-400"],
  light: { x: 30, y: 20, r: 44, paint: "white", opacity: 0.55 },
  emblem: { x: 82, y: 30, size: 5, rotate: 0 },
  layers: {
    sky: [
      { part: "god-rays", x: 30, y: 0, scale: 2, paint: "white" },
      { part: "sun", x: 22, y: 10, scale: 0.6 },
    ],
    far: [
      { part: "water", x: 50, y: 36, paint: "cyan-400" },
      { part: "mesa", x: 8, y: 42, scale: 0.6, paint: "stone-500" },
      { part: "mesa", x: 92, y: 44, scale: 0.6, flip: -1, paint: "stone-500" },
      { part: "reed", x: 30, y: 48, scale: 1, count: 3, spread: [10, 4], paint: "green-700" },
    ],
    mid: [
      { part: "fern-sprig", x: 14, y: 72, scale: 2.4, paint: "orange-400" },
      { part: "fern-sprig", x: 74, y: 44, scale: 1.4, paint: "red-300" },
      { part: "bush", x: 86, y: 74, scale: 1.6, paint: "red-300" },
      { part: "bush", x: 24, y: 46, scale: 1, paint: "orange-400" },
      { part: "pebbles", x: 64, y: 78, scale: 1.4, paint: "jade-400" },
      { part: "echo-rings", x: 50, y: 56, scale: 1.4, paint: "cyan-400" },
    ],
    subject: [
      { part: "void-swirl", x: 50, y: 56, scale: 1.1, paint: "jade-400" },
      { part: "orb", x: 50, y: 56, scale: 0.5, paint: "black" },
    ],
    near: [
      { part: "pebbles", x: 20, y: 94, scale: 2, paint: "red-300" },
      { part: "pebbles", x: 86, y: 96, scale: 2.2, paint: "stone-500" },
      { part: "bush", x: 8, y: 98, scale: 1.4, paint: "red-300" },
      { part: "reed", x: 70, y: 98, scale: 1.4, count: 2, spread: [6, 2], paint: "jade-400" },
    ],
    light: [
      { part: "rim-light", x: 50, y: 56, scale: 1, paint: "white" },
      { part: "light-shaft", x: 34, y: 0, scale: 1.2, paint: "white" },
    ],
    particles: [
      { part: "bubbles", x: 62, y: 40, scale: 1.2, count: 3, spread: [20, 10] },
      { part: "sparks", x: 40, y: 30, scale: 1, count: 3, spread: [30, 10], paint: "white" },
      { part: "motes", x: 50, y: 60, scale: 1.2, paint: "white" },
    ],
  },
};

/** R3398000: Core's alt recipes #68 to #100, by card id. */
export const CORE_ALT_RECIPES_3: Readonly<Record<string, SceneRecipe>> = {
  "core-068": TWISTED_SORCERER_ALT,
  "core-072": REMINISCE_ALT,
  "core-073": ANTI_ONESHOT_ARMOR_ALT,
  "core-076": FIELD_OF_DREAMS_ALT,
  "core-078": FULLSEND_ALT,
  "core-079": TWINSPELL_ALT,
  "core-082": KYS_TRIAL_ALT,
  "core-083": TRANSMOGULATE_ALT,
  "core-085": UNLICENSED_EXPERIMENTATION_ALT,
  "core-086": MISS_MROW_ALT,
  "core-087": POCKET_CHAOS_ALT,
  "core-088": TWISTING_NETHER_ALT,
  "core-089": CORPSE_EATER_ALT,
  "core-092": FELINOR_FIENDER_ALT,
  "core-093": COMBO_INDEX_ALT,
  "core-094": GENNS_GREED_ALT,
  "core-096": MY_PAWN_ALT,
  "core-097": ZEPHRYS_ALT,
  "core-098": HEROIC_POWER_ALT,
  "core-099": CRAFT_A_CARD_ALT,
  "core-100": CEASELESS_VOID_ALT,
};
