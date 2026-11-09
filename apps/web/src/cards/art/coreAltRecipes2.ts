// R3398000: Core's alt scene recipes, part 2 of three (#37 to #65), one per card offered Alt art, each drawn from its
// brief (ART.md, "Core's Alt art restaged"): the card's own subject in another place, time or moment, under a sky
// sharing no colour with its normal picture's and a new camera (a close-up, a wide or distant view, or one from
// above), its figure the normal picture's person (coreAltRecipes1.ts `normalCoreLookOf`), in Core's glass-orb look
// and at or above its rarity's floor. scene.ts `sceneRecipeOf` reads one under the card's Alt art seed alone, at every
// Alt art level. Presentation only (CLAUDE.md rule 7): a recipe is read by the card's id alone.

import { normalCoreExtraLookOf, normalCoreLookOf } from "./coreAltRecipes1.ts";
import { figureLookFor, type FigureLook } from "./figureKit.ts";
import type { SceneRecipe } from "./scene.ts";

/** The runaway skeleton of the orchard cemetery: a bare skull in rags. */
const SKELETON: FigureLook = { ...figureLookFor(["Human"]), head: "head-skull", hair: null, outfit: "outfit-rags", item: null, skin: "slate-100", cloth: "stone-500" };
/** A fan queueing on the pavement before the show. */
const QUEUER: FigureLook = { ...figureLookFor(["Human"]), hair: "hair-long", outfit: "outfit-hoodie", item: null, cloth: "blue-700" };
/** A student of the lecture hall's rows. */
const STUDENT: FigureLook = { ...figureLookFor(["Human"]), hair: "hair-short", outfit: "outfit-sweater", item: null, cloth: "green-700" };
/** A member of the sceptical committee on the university lawn. */
const COMMITTEE: FigureLook = { ...figureLookFor(["Human"]), hair: "spectacles", outfit: "outfit-blazer", item: null, cloth: "slate-700" };
/** A prodigy of the program's summer camp. */
const PRODIGY: FigureLook = { ...figureLookFor(["Human"]), hair: "hair-short", outfit: "outfit-sweater", item: "item-card", cloth: "blue-700" };
/** The lone wanderer of the thorn desert, hooded against the sun, her satchel at her hand. */
const WANDERER: FigureLook = { ...figureLookFor(["Human"]), hair: "hood", outfit: "outfit-poncho", item: "item-sack", cloth: "wood-700" };

/** Gravedigger (Core #37, Rare), Alt art: a wide shot of the gravedigger chasing a runaway skeleton through red leaves in an autumn orchard cemetery, spade raised; the orchard is the kit's `fruit-tree`. */
const GRAVEDIGGER_ALT: SceneRecipe = {
  family: "core",
  sky: ["orange-400", "red-300"],
  light: { x: 76, y: 26, r: 40, paint: "orange-400", opacity: 0.4 },
  emblem: { x: 22, y: 30, size: 5, rotate: 0 },
  figure: { x: 36, y: 80, scale: 0.6, pose: "pose-run", look: { ...normalCoreLookOf("core-037"), cloth: "red-900" } },
  extras: [{ layer: "subject", x: 66, y: 78, scale: 0.5, pose: "pose-run", look: SKELETON }],
  layers: {
    sky: [
      { part: "sun-disc", x: 76, y: 26, scale: 1.2 },
      { part: "cloud", x: 30, y: 20, scale: 0.9, paint: "red-300" },
    ],
    far: [
      { part: "hills", x: 50, y: 56, paint: "red-900" },
      { part: "fruit-tree", x: 14, y: 66, scale: 1, paint: "orange-400" },
      { part: "fruit-tree", x: 86, y: 64, scale: 0.9, flip: -1, paint: "red-600" },
      { part: "fruit-tree", x: 54, y: 62, scale: 0.6, paint: "red-600" },
    ],
    mid: [
      { part: "field", x: 50, y: 66, paint: "brass-600" },
      { part: "fence", x: 50, y: 70, scale: 0.7 },
      { part: "tombstone", x: 22, y: 76, scale: 1.2, count: 2, spread: [10, 2] },
      { part: "tombstone", x: 82, y: 76, scale: 1.3, count: 2, spread: [8, 2] },
    ],
    subject: [
      { part: "grave-mound", x: 50, y: 84, scale: 1.6 },
      { part: "tombstone", x: 50, y: 80, scale: 1.4 },
      { part: "leaves", x: 52, y: 74, scale: 1.2 },
    ],
    near: [
      { part: "leaves", x: 50, y: 92, scale: 1.4, count: 2, spread: [30, 3] },
      { part: "grass-tuft", x: 14, y: 96, count: 2, spread: [6, 1], paint: "brass-600" },
      { part: "grass-tuft", x: 88, y: 97, count: 2, spread: [6, 1], paint: "red-900" },
    ],
    light: [
      { part: "halo", x: 76, y: 26, scale: 1.6, paint: "orange-400" },
      { part: "rim-light", x: 36, y: 60, scale: 0.8, paint: "orange-400" },
    ],
    particles: [
      { part: "petals", x: 50, y: 50, scale: 1.4, paint: "red-600" },
      { part: "leaves", x: 62, y: 40, scale: 1.2 },
    ],
  },
};

/** Quickstriker (Core #38, Epic), Alt art: the platform at dusk under drifting cherry petals, a single blade left on the floor, seen from a high corner; soft pink. The cherry trees are the kit's `fruit-tree` and `tree` painted pink. */
const QUICKSTRIKER_ALT: SceneRecipe = {
  family: "core",
  sky: ["violet-500", "red-300"],
  light: { x: 78, y: 24, r: 40, paint: "red-300", opacity: 0.4 },
  emblem: { x: 22, y: 26, size: 5, rotate: 0 },
  layers: {
    sky: [
      { part: "sun-disc", x: 78, y: 24, scale: 0.9 },
      { part: "cloud", x: 40, y: 20, scale: 1, count: 2, spread: [16, 3], paint: "red-300" },
    ],
    far: [
      { part: "fruit-tree", x: 10, y: 40, scale: 0.9, paint: "red-300" },
      { part: "tree", x: 90, y: 40, scale: 1, flip: -1, paint: "red-300" },
      { part: "tree", x: 62, y: 38, scale: 0.6, paint: "violet-300" },
    ],
    mid: [
      { part: "boards", x: 50, y: 38, paint: "wood-700" },
      { part: "fence-post", x: 16, y: 54, scale: 1.2 },
      { part: "fence-post", x: 86, y: 50, scale: 1.1, flip: -1 },
      { part: "petals", x: 30, y: 72, scale: 1.2, paint: "red-300" },
    ],
    subject: [
      { part: "blade", x: 56, y: 62, scale: 1.4 },
      { part: "gust", x: 56, y: 66, scale: 1 },
    ],
    near: [
      { part: "fence-post", x: 8, y: 99, scale: 1.8 },
      { part: "petals", x: 72, y: 88, scale: 1.4, paint: "red-300" },
      { part: "petals", x: 24, y: 94, scale: 1.2, paint: "violet-300" },
    ],
    light: [
      { part: "halo", x: 56, y: 62, scale: 1.4, paint: "red-300" },
      { part: "rim-light", x: 56, y: 62, scale: 0.8, paint: "white" },
    ],
    particles: [{ part: "petals", x: 50, y: 40, scale: 1.6, count: 2, spread: [20, 10], paint: "red-300" }],
  },
};

/** Recycling Initiative (Core #39, Epic), Alt art: a rooftop recycling garden on a breezy afternoon, copies fluttering out of the chute into a waiting hand among planter boxes; bird's-eye view, leafy green. The planters are a `crate` under a `bush`. */
const RECYCLING_INITIATIVE_ALT: SceneRecipe = {
  family: "core",
  sky: ["green-200", "lime-200"],
  light: { x: 24, y: 20, r: 44, paint: "white", opacity: 0.4 },
  emblem: { x: 80, y: 28, size: 5, rotate: 0 },
  layers: {
    sky: [{ part: "cloud", x: 70, y: 16, scale: 0.8, paint: "white" }],
    far: [{ part: "skyline", x: 50, y: 26, scale: 0.5 }],
    mid: [
      { part: "tile-floor", x: 50, y: 26, paint: "slate-700" },
      { part: "crate", x: 14, y: 50, scale: 1.4, paint: "wood-600" },
      { part: "bush", x: 14, y: 44, scale: 1.2, paint: "green-500" },
      { part: "crate", x: 86, y: 46, scale: 1.3, paint: "wood-600" },
      { part: "bush", x: 86, y: 40, scale: 1.1, paint: "green-700" },
      { part: "fern-sprig", x: 30, y: 40, scale: 1.4 },
    ],
    subject: [
      { part: "glow-chute", x: 58, y: 42, scale: 1.3 },
      { part: "playing-card", x: 52, y: 62, scale: 0.9, count: 3, spread: [12, 10] },
      { part: "open-hand", x: 30, y: 78, scale: 1.6 },
    ],
    near: [
      { part: "crate", x: 12, y: 99, scale: 1.8, paint: "wood-600" },
      { part: "bush", x: 12, y: 90, scale: 1.8, paint: "green-700" },
      { part: "crate", x: 88, y: 99, scale: 1.7, paint: "wood-600" },
      { part: "bush", x: 88, y: 90, scale: 1.6, paint: "green-500" },
      { part: "leaves", x: 70, y: 84, scale: 1.2 },
    ],
    light: [{ part: "halo", x: 58, y: 48, scale: 1.4, paint: "green-200" }],
    particles: [
      { part: "leaves", x: 50, y: 40, scale: 1.4, count: 2, spread: [20, 10] },
      { part: "gust", x: 40, y: 56, scale: 1.2 },
    ],
  },
};

/** Selective Breeding (Core #42, Common), Alt art: a wide view of the breeder sorting seedlings by size into baskets on a windy hillside allotment at harvest, shimmering pods glinting; earthy brown. The pods' glints are `sparks`. */
const SELECTIVE_BREEDING_ALT: SceneRecipe = {
  family: "core",
  sky: ["blue-200", "orange-400"],
  light: { x: 76, y: 24, r: 38, paint: "orange-400", opacity: 0.35 },
  emblem: { x: 22, y: 30, size: 5, rotate: 0 },
  figure: { x: 40, y: 78, scale: 0.55, pose: "pose-kneel", look: { ...normalCoreLookOf("core-042"), item: null } },
  layers: {
    sky: [{ part: "cloud", x: 64, y: 22, scale: 1, count: 2, spread: [20, 4], paint: "white" }],
    far: [{ part: "hills", x: 50, y: 54, paint: "wood-700" }],
    mid: [
      { part: "furrows", x: 70, y: 62, paint: "wood-700" },
      { part: "fence", x: 50, y: 64, scale: 0.6 },
    ],
    subject: [
      { part: "sprout", x: 56, y: 82, scale: 1.2, count: 3, spread: [10, 2] },
      { part: "basket", x: 68, y: 84, scale: 0.9 },
      { part: "basket", x: 22, y: 82, scale: 0.7 },
    ],
    near: [
      { part: "grass-tuft", x: 50, y: 97, count: 3, spread: [40, 2], paint: "wood-600" },
      { part: "sprout", x: 88, y: 96, scale: 1.6 },
    ],
    light: [{ part: "heal-glow", x: 56, y: 76, scale: 1 }],
    particles: [
      { part: "leaves", x: 50, y: 40, scale: 1.3 },
      { part: "sparks", x: 58, y: 74, scale: 1.2, count: 2, spread: [10, 4], paint: "green-200" },
    ],
  },
};

/** Big Felinor (Core #43, Rare), Alt art: a close-up of the big Felinor napping in a beach hammock, tiny Felinors curled on his chest; sandy turquoise. He sits in it (the kit has no lying pose) and the tiny Felinors are `house-cat`. */
const BIG_FELINOR_ALT: SceneRecipe = {
  family: "core",
  sky: ["cyan-400", "blue-200"],
  light: { x: 80, y: 20, r: 40, paint: "white", opacity: 0.45 },
  emblem: { x: 82, y: 32, size: 5, rotate: 0 },
  figure: { x: 50, y: 96, scale: 1.6, pose: "pose-sit", look: { ...normalCoreLookOf("core-043") } },
  layers: {
    sky: [
      { part: "sun", x: 80, y: 18, scale: 0.6 },
      { part: "cloud", x: 30, y: 22, scale: 0.8, paint: "white" },
    ],
    far: [{ part: "sea", x: 50, y: 56, paint: "cyan-400" }],
    mid: [
      { part: "sand", x: 50, y: 66 },
      { part: "palm", x: 10, y: 80, scale: 1.6 },
      { part: "palm", x: 92, y: 78, scale: 1.4, flip: -1 },
    ],
    near: [
      { part: "beach-hammock", x: 50, y: 88, scale: 2.6 },
      { part: "house-cat", x: 44, y: 66, scale: 1.3, paint: "orange-400" },
      { part: "house-cat", x: 56, y: 65, scale: 1.1, flip: -1, paint: "slate-100" },
    ],
    light: [{ part: "halo", x: 80, y: 18, scale: 1.6, paint: "white" }],
    particles: [{ part: "motes", x: 50, y: 40, scale: 1.4, paint: "white" }],
  },
};

/** True Strike (Core #44, Common), Alt art: an extreme close-up of the arrowhead just leaving the bowstring, the archer's eye in focus, in a misty mountain pine forest; cool green-grey. */
const TRUE_STRIKE_ALT: SceneRecipe = {
  family: "core",
  sky: ["slate-400", "green-200"],
  light: { x: 70, y: 20, r: 40, paint: "white", opacity: 0.35 },
  emblem: { x: 82, y: 76, size: 5, rotate: 0 },
  extras: [{ layer: "mid", x: 30, y: 116, scale: 1.7, pose: "pose-point", look: { ...normalCoreExtraLookOf("core-044", 0), item: "item-bow", cloth: "slate-700" } }],
  layers: {
    sky: [{ part: "mist", x: 50, y: 30, scale: 1.4, count: 2, spread: [20, 4], paint: "white" }],
    far: [
      { part: "mountains", x: 50, y: 56, paint: "slate-700" },
      { part: "pine", x: 66, y: 66, scale: 1.6, count: 3, spread: [20, 3], paint: "green-900" },
      { part: "pine", x: 90, y: 70, scale: 2, paint: "green-900" },
    ],
    mid: [
      { part: "mist", x: 70, y: 66, scale: 1.4, paint: "slate-100" },
      { part: "pine", x: 8, y: 80, scale: 2.4, paint: "green-900" },
    ],
    subject: [
      { part: "flying-arrow", x: 72, y: 46, scale: 1.6 },
      { part: "streaks", x: 58, y: 46, scale: 1, paint: "white" },
    ],
    near: [
      { part: "fern", x: 92, y: 99, scale: 1.6 },
      { part: "mist", x: 50, y: 92, scale: 1.4, paint: "slate-100" },
    ],
    light: [{ part: "rim-light", x: 30, y: 40, scale: 1.4, paint: "white" }],
    particles: [{ part: "motes", x: 50, y: 50, scale: 1.4, paint: "white" }],
  },
};

/** Fig of Life (Core #47, Common), Alt art: a close-up of the fig still on its branch in a whitewashed courtyard after rain, one dewdrop glowing with healing light; cool blue-white. The branch is a `vine`. */
const FIG_OF_LIFE_ALT: SceneRecipe = {
  family: "core",
  sky: ["blue-200", "white"],
  light: { x: 30, y: 22, r: 40, paint: "white", opacity: 0.5 },
  emblem: { x: 20, y: 76, size: 5, rotate: 0 },
  layers: {
    sky: [{ part: "cloud", x: 70, y: 20, scale: 1, count: 2, spread: [16, 3], paint: "slate-400" }],
    far: [
      { part: "whitewashed-house", x: 16, y: 66, scale: 1.6 },
      { part: "whitewashed-house", x: 84, y: 64, scale: 1.4, flip: -1 },
    ],
    mid: [{ part: "cobbles", x: 50, y: 66, paint: "slate-400" }],
    subject: [
      { part: "vine", x: 52, y: 38, scale: 2.2 },
      { part: "leaves", x: 50, y: 30, scale: 1.2 },
      { part: "fig", x: 52, y: 54, scale: 4 },
      { part: "drops", x: 54, y: 70, scale: 0.5, paint: "cyan-400" },
    ],
    near: [
      { part: "drops", x: 20, y: 86, scale: 1, paint: "blue-200" },
      { part: "drops", x: 82, y: 40, scale: 0.8, paint: "blue-200" },
    ],
    light: [
      { part: "heal-glow", x: 54, y: 72, scale: 0.9 },
      { part: "halo", x: 52, y: 54, scale: 1.4, paint: "white" },
    ],
    particles: [{ part: "motes", x: 50, y: 50, scale: 1.4, paint: "white" }],
  },
};

/** 5pek Controller (Core #48, Common), Alt art: a bird's-eye view of a child's bedroom floor, the controller dropped on a striped rug as toy soldiers flip positions; warm orange. */
const FIVE_PEK_CONTROLLER_ALT: SceneRecipe = {
  family: "core",
  sky: ["orange-400", "red-300"],
  light: { x: 30, y: 30, r: 44, paint: "orange-400", opacity: 0.4 },
  emblem: { x: 84, y: 30, size: 5, rotate: 0 },
  layers: {
    far: [{ part: "boards", x: 50, y: 12, paint: "wood-600" }],
    mid: [
      { part: "die", x: 82, y: 30, scale: 2.4 },
      { part: "closed-tome", x: 16, y: 32, scale: 1.4 },
      { part: "die", x: 16, y: 74, scale: 2 },
    ],
    subject: [
      { part: "rug", x: 50, y: 58, scale: 2.6 },
      { part: "controller", x: 50, y: 48, scale: 1.6 },
      { part: "soldier", x: 34, y: 70, scale: 1.2, count: 2, spread: [8, 2] },
      { part: "soldier", x: 66, y: 68, scale: 1.2, count: 2, spread: [8, 2], flip: -1 },
    ],
    near: [
      { part: "soldier-fallen", x: 76, y: 88, scale: 1.3 },
      { part: "die", x: 30, y: 90, scale: 2.2 },
    ],
    light: [{ part: "halo", x: 50, y: 48, scale: 1.8, paint: "orange-400" }],
    particles: [
      { part: "motes", x: 50, y: 50, scale: 1.4, paint: "white" },
      { part: "gust", x: 50, y: 70, scale: 1 },
    ],
  },
};

/** Snom Bunny Mind Control (Core #49, Rare), Alt art: a close-up of the bunny's swirling eyes reflected in a dazed knight's visor, in a spring meadow of clover; pastel pink. The reflection is a small `vortex` at the visor and the clover the kit's `flowers`. */
const SNOM_BUNNY_MIND_CONTROL_ALT: SceneRecipe = {
  family: "core",
  sky: ["red-300", "green-200"],
  light: { x: 24, y: 22, r: 40, paint: "white", opacity: 0.45 },
  emblem: { x: 18, y: 30, size: 5, rotate: 0 },
  figure: { x: 64, y: 118, scale: 1.8, flip: -1, pose: "pose-stand", look: { ...normalCoreLookOf("core-049"), item: null } },
  layers: {
    sky: [{ part: "cloud", x: 26, y: 24, scale: 0.9, paint: "white" }],
    far: [
      { part: "hills", x: 50, y: 58, paint: "green-400" },
      { part: "fruit-tree", x: 14, y: 62, scale: 0.7, paint: "red-300" },
    ],
    mid: [
      { part: "field", x: 50, y: 64, paint: "green-400" },
      { part: "flowers", x: 20, y: 74, count: 3, spread: [14, 3] },
    ],
    subject: [
      { part: "bunny", x: 28, y: 80, scale: 1.6 },
      { part: "vortex", x: 30, y: 62, scale: 0.6, paint: "violet-300" },
    ],
    near: [
      { part: "vortex", x: 62, y: 30, scale: 0.3, paint: "violet-300" },
      { part: "flowers", x: 40, y: 97, count: 3, spread: [20, 2] },
      { part: "grass-tuft", x: 14, y: 97, count: 2, spread: [6, 1], paint: "green-500" },
    ],
    light: [{ part: "halo", x: 62, y: 32, scale: 1, paint: "violet-300" }],
    particles: [
      { part: "petals", x: 50, y: 40, scale: 1.4, paint: "red-300" },
      { part: "motes", x: 40, y: 60, scale: 1.2, paint: "white" },
    ],
  },
};

/** K-Pop Fanatic (Core #50, Epic), Alt art: before the show, a wide view of the fan in a sleeping bag in a pavement queue in the noon heat, light stick hugged, a rival's lantern already floating to her; cool blue. */
const K_POP_FANATIC_ALT: SceneRecipe = {
  family: "core",
  sky: ["sky-400", "blue-200"],
  light: { x: 76, y: 16, r: 40, paint: "white", opacity: 0.5 },
  emblem: { x: 18, y: 30, size: 5, rotate: 0 },
  figure: { x: 40, y: 80, scale: 0.6, pose: "pose-sit", look: { ...normalCoreLookOf("core-050") } },
  extras: [
    { layer: "mid", x: 62, y: 72, scale: 0.4, flip: -1, pose: "pose-stand", look: QUEUER },
    { layer: "mid", x: 72, y: 71, scale: 0.38, flip: -1, pose: "pose-raise", look: { ...QUEUER, hair: "hair-short", cloth: "red-600" } },
    { layer: "mid", x: 82, y: 70, scale: 0.36, flip: -1, pose: "pose-stand", look: { ...QUEUER, hair: "bucket-hat", cloth: "green-700" } },
  ],
  layers: {
    sky: [{ part: "sun", x: 76, y: 16, scale: 0.7 }],
    far: [
      { part: "skyline", x: 30, y: 60, scale: 0.6 },
      { part: "stadium", x: 70, y: 58, scale: 0.5 },
    ],
    mid: [
      { part: "street", x: 50, y: 60, paint: "slate-400" },
      { part: "streetlamp", x: 90, y: 74, scale: 0.9 },
    ],
    subject: [{ part: "lantern", x: 56, y: 56, scale: 1.2 }],
    near: [
      { part: "sleeping-bag", x: 43, y: 81, scale: 1.1 },
      { part: "streaks", x: 64, y: 54, scale: 0.8, paint: "white" },
      { part: "bin", x: 88, y: 98, scale: 1.2 },
    ],
    light: [{ part: "halo", x: 56, y: 58, scale: 1.2, paint: "orange-400" }],
    particles: [{ part: "motes", x: 50, y: 50, scale: 1.6, paint: "white" }],
  },
};

/** Silly Silas (Core #52, Legendary), Alt art: a wide shot of a sunrise rehearsal on an empty pier, Silas alone practising his spin as the planks tilt under him, gulls circling; soft coral. The tilted planks are a wooden `ramp`. */
const SILLY_SILAS_ALT: SceneRecipe = {
  family: "core",
  sky: ["violet-300", "red-300"],
  light: { x: 24, y: 56, r: 40, paint: "orange-400", opacity: 0.45 },
  emblem: { x: 80, y: 30, size: 5, rotate: 0 },
  figure: { x: 58, y: 70, scale: 0.55, pose: "pose-spread", look: { ...normalCoreLookOf("core-052") } },
  layers: {
    sky: [
      { part: "sun", x: 24, y: 56, scale: 0.9 },
      { part: "cloud", x: 70, y: 26, scale: 1, count: 2, spread: [16, 4], paint: "red-300" },
    ],
    far: [
      { part: "sea", x: 50, y: 56, paint: "blue-800" },
      { part: "lighthouse", x: 88, y: 56, scale: 0.4 },
    ],
    mid: [
      { part: "boards", x: 58, y: 64, paint: "wood-600" },
      { part: "fence-post", x: 20, y: 72, scale: 0.9 },
      { part: "fence-post", x: 90, y: 70, scale: 0.8, flip: -1 },
    ],
    subject: [
      { part: "ramp", x: 58, y: 74, scale: 0.7, paint: "wood-700" },
      { part: "gust", x: 58, y: 56, scale: 1.2 },
      { part: "echo-rings", x: 58, y: 58, scale: 0.8 },
    ],
    near: [
      { part: "fence-post", x: 8, y: 99, scale: 1.6 },
      { part: "fence-post", x: 94, y: 99, scale: 1.5, flip: -1 },
      { part: "gull", x: 84, y: 82, scale: 1.2 },
    ],
    light: [
      { part: "god-rays", x: 24, y: 56, scale: 1.4, paint: "orange-400" },
      { part: "halo", x: 24, y: 56, scale: 1.8, paint: "red-300" },
      { part: "rim-light", x: 58, y: 56, scale: 0.8, paint: "orange-400" },
    ],
    particles: [
      { part: "gull", x: 50, y: 30, scale: 1, count: 3, spread: [24, 8] },
      { part: "spray", x: 30, y: 66, scale: 1.2 },
      { part: "motes", x: 50, y: 50, scale: 1.4, paint: "white" },
    ],
  },
};

/** Reno (Core #53, Common), Alt art: a wide view of Reno kneeling in a sunny vineyard, coaxing a wilted vine back upright with the same green light; warm terracotta. */
const RENO_ALT: SceneRecipe = {
  family: "core",
  sky: ["sky-400", "orange-400"],
  light: { x: 76, y: 22, r: 40, paint: "orange-400", opacity: 0.4 },
  emblem: { x: 20, y: 30, size: 5, rotate: 0 },
  figure: { x: 40, y: 78, scale: 0.6, pose: "pose-kneel", look: { ...normalCoreLookOf("core-053") } },
  layers: {
    sky: [{ part: "sun", x: 78, y: 20, scale: 0.6 }],
    far: [{ part: "hills", x: 50, y: 56, paint: "red-600" }],
    mid: [
      { part: "furrows", x: 50, y: 62, paint: "red-900" },
      { part: "vine", x: 18, y: 72, scale: 1, count: 3, spread: [10, 2] },
      { part: "vine", x: 82, y: 70, scale: 1, count: 3, spread: [10, 2] },
    ],
    subject: [
      { part: "vine", x: 56, y: 80, scale: 1.4 },
      { part: "zap", x: 52, y: 60, scale: 0.5, paint: "green-400" },
      { part: "grapes", x: 58, y: 70, scale: 0.9 },
    ],
    near: [
      { part: "grapes", x: 88, y: 84, scale: 1.4 },
      { part: "grass-tuft", x: 14, y: 97, count: 2, spread: [6, 1], paint: "red-900" },
    ],
    light: [{ part: "heal-glow", x: 56, y: 72, scale: 1.2 }],
    particles: [{ part: "motes", x: 50, y: 50, scale: 1.2, paint: "white" }],
  },
};

/** Lava Golem (Core #55, Rare), Alt art: cooled, a wide shot of the golem standing steaming in a glacier valley, its lava seams dimmed under falling snow; ice blue. */
const LAVA_GOLEM_ALT: SceneRecipe = {
  family: "core",
  sky: ["blue-200", "white"],
  light: { x: 30, y: 22, r: 40, paint: "white", opacity: 0.45 },
  emblem: { x: 80, y: 30, size: 5, rotate: 0 },
  figure: { x: 50, y: 76, scale: 0.6, pose: "pose-stand", look: { ...normalCoreLookOf("core-055") } },
  layers: {
    sky: [{ part: "cloud", x: 30, y: 22, scale: 1.2, count: 2, spread: [30, 4], paint: "slate-100" }],
    far: [
      { part: "mountains", x: 50, y: 58, paint: "slate-100" },
      { part: "pine", x: 12, y: 66, scale: 1, count: 3, spread: [8, 2], paint: "green-900" },
    ],
    mid: [
      { part: "ice-floor", x: 50, y: 64 },
      { part: "crystal", x: 80, y: 74, scale: 1 },
    ],
    subject: [
      { part: "steam", x: 50, y: 44, scale: 1.4, paint: "white" },
      { part: "smoke", x: 54, y: 40, scale: 0.8, paint: "slate-100" },
    ],
    near: [
      { part: "grass-tuft", x: 20, y: 97, count: 3, spread: [12, 2], paint: "slate-100" },
      { part: "crystal", x: 88, y: 99, scale: 1.4 },
    ],
    light: [{ part: "halo", x: 50, y: 60, scale: 1.2, paint: "orange-400" }],
    particles: [
      { part: "snow", x: 50, y: 40, scale: 1.4 },
      { part: "snow", x: 50, y: 70, scale: 1.2 },
    ],
  },
};

/** Jilliax (Core #56, Common), Alt art: off duty, a close-up of Jilliax hunched under a tarp in a scrapyard at dusk, sharpening its teeth on a bolt; rust and steel blue. The tarp is an `awning` and the bolt a `cog`. */
const JILLIAX_ALT: SceneRecipe = {
  family: "core",
  sky: ["orange-400", "slate-700"],
  light: { x: 80, y: 60, r: 36, paint: "orange-400", opacity: 0.4 },
  emblem: { x: 82, y: 30, size: 5, rotate: 0 },
  figure: { x: 48, y: 108, scale: 1.6, pose: "pose-sit", look: { ...normalCoreLookOf("core-056"), item: null } },
  layers: {
    sky: [{ part: "sun-disc", x: 84, y: 56, scale: 1 }],
    far: [
      { part: "cog", x: 14, y: 52, scale: 1.4, paint: "slate-700" },
      { part: "wrecking-ball", x: 86, y: 10, scale: 1 },
    ],
    mid: [
      { part: "rock-floor", x: 50, y: 70, paint: "wood-800" },
      { part: "bin", x: 14, y: 76, scale: 1.4 },
      { part: "crate", x: 88, y: 76, scale: 1.4, paint: "red-900" },
    ],
    subject: [{ part: "awning", x: 48, y: 100, scale: 2.8, paint: "red-900" }],
    near: [
      { part: "cog", x: 60, y: 54, scale: 0.4, paint: "slate-400" },
      { part: "sparks", x: 62, y: 52, scale: 1, paint: "orange-400" },
    ],
    light: [{ part: "rim-light", x: 48, y: 60, scale: 1.2, paint: "orange-400" }],
    particles: [{ part: "sparks", x: 58, y: 58, scale: 1, paint: "orange-400" }],
  },
};

/** Conjure KY (Core #57, Common), Alt art: a wide shot from inside the lecture hall, rows of startled students watching KY's chalk and empty lectern vanish into the portal; warm wood brown. KY has gone through it, so no figure; the chalk is a puff of `shards`. */
const CONJURE_KY_ALT: SceneRecipe = {
  family: "core",
  sky: ["wood-700", "brass-600"],
  light: { x: 54, y: 50, r: 40, paint: "cyan-400", opacity: 0.35 },
  emblem: { x: 82, y: 30, size: 5, rotate: 0 },
  extras: [
    { layer: "mid", x: 16, y: 80, scale: 0.45, pose: "pose-spread", look: STUDENT },
    { layer: "mid", x: 28, y: 78, scale: 0.42, pose: "pose-raise", look: { ...STUDENT, hair: "hair-long", cloth: "red-600" } },
    { layer: "mid", x: 84, y: 80, scale: 0.45, flip: -1, pose: "pose-raise", look: { ...STUDENT, hair: "spectacles", cloth: "blue-700" } },
  ],
  layers: {
    far: [
      { part: "room-wall", x: 50, y: 62, paint: "wood-600" },
      { part: "blackboard", x: 54, y: 50, scale: 1.2 },
    ],
    mid: [
      { part: "boards", x: 50, y: 62, paint: "wood-700" },
      { part: "bench", x: 22, y: 84, scale: 1.2 },
      { part: "bench", x: 82, y: 84, scale: 1.2 },
    ],
    subject: [
      { part: "portal", x: 54, y: 66, scale: 1.4, paint: "cyan-400" },
      { part: "lectern", x: 56, y: 70, scale: 1.4 },
      { part: "shards", x: 50, y: 56, scale: 0.8 },
    ],
    near: [{ part: "bench", x: 50, y: 99, scale: 2.6 }],
    light: [{ part: "halo", x: 54, y: 60, scale: 1.6, paint: "cyan-400" }],
    particles: [
      { part: "vortex", x: 54, y: 60, scale: 0.6, paint: "cyan-400" },
      { part: "motes", x: 50, y: 50, scale: 1.2, paint: "white" },
    ],
  },
};

/** Rush Token Farm (Core #58, Common), Alt art: a wide view of the farm in deep winter, Rush Tokens bursting out of snow-covered rows, steam rising off them, the barn frosted; cold white. */
const RUSH_TOKEN_FARM_ALT: SceneRecipe = {
  family: "core",
  sky: ["slate-400", "white"],
  light: { x: 74, y: 22, r: 38, paint: "white", opacity: 0.4 },
  emblem: { x: 82, y: 30, size: 5, rotate: 0 },
  layers: {
    sky: [{ part: "cloud", x: 30, y: 22, scale: 1.3, count: 2, spread: [30, 4], paint: "slate-100" }],
    far: [
      { part: "hills", x: 50, y: 54, paint: "slate-100" },
      { part: "barn", x: 26, y: 60, scale: 0.8, paint: "red-900" },
      { part: "pine", x: 86, y: 60, scale: 0.9, count: 2, spread: [6, 2], paint: "green-900" },
    ],
    mid: [
      { part: "snowfield", x: 50, y: 60 },
      { part: "fence", x: 50, y: 64, scale: 0.6 },
    ],
    subject: [
      { part: "rush-token", x: 56, y: 72, scale: 1.3, count: 3, spread: [22, 6] },
      { part: "steam", x: 56, y: 62, scale: 1, count: 3, spread: [22, 6], paint: "white" },
      { part: "dust-cloud", x: 56, y: 74, scale: 0.8 },
    ],
    near: [{ part: "fence", x: 50, y: 94, scale: 1.2 }],
    particles: [
      { part: "snow", x: 50, y: 40, scale: 1.4 },
      { part: "snow", x: 50, y: 70, scale: 1.2 },
    ],
  },
};

/** Unbiased Immigration (Core #59, Rare), Alt art: a wide view of a desert caravanserai gate at dawn, one traveller leading a camel up the road with a gift box; pale violet and sand. The gate is a stone `arch` in a `battlement`, the gift tied on the camel's saddle. */
const UNBIASED_IMMIGRATION_ALT: SceneRecipe = {
  family: "core",
  sky: ["violet-300", "orange-400"],
  light: { x: 80, y: 54, r: 40, paint: "orange-400", opacity: 0.45 },
  emblem: { x: 20, y: 30, size: 5, rotate: 0 },
  figure: { x: 48, y: 78, scale: 0.55, pose: "pose-stride", look: { ...normalCoreLookOf("core-059"), cloth: "wood-700" } },
  layers: {
    sky: [{ part: "sun", x: 80, y: 54, scale: 0.8 }],
    far: [
      { part: "sand", x: 50, y: 58 },
      { part: "mesa", x: 20, y: 60, scale: 0.5 },
      { part: "battlement", x: 76, y: 66, scale: 0.6, paint: "brass-600" },
      { part: "arch", x: 76, y: 66, scale: 0.7 },
    ],
    mid: [
      { part: "road", x: 76, y: 66 },
      { part: "palm", x: 92, y: 72, scale: 0.8, flip: -1 },
      { part: "cactus", x: 12, y: 76, scale: 0.8 },
    ],
    subject: [
      { part: "camel", x: 30, y: 78, scale: 0.9 },
      { part: "gift", x: 30, y: 63, scale: 1.4 },
    ],
    near: [
      { part: "pebbles", x: 20, y: 96 },
      { part: "cactus", x: 90, y: 99, scale: 1.2 },
    ],
    light: [{ part: "halo", x: 80, y: 54, scale: 1.8, paint: "orange-400" }],
    particles: [{ part: "dust", x: 40, y: 80, scale: 1.2 }],
  },
};

/** Prejudiced Postdoc (Core #61, Rare), Alt art: a wide shot of the postdoc presenting the blank-faced clone to a small, sceptical committee on a sunny university lawn; cool white and green. The university is a `rotunda` and the clone the kit's `soldier`. */
const PREJUDICED_POSTDOC_ALT: SceneRecipe = {
  family: "core",
  sky: ["sky-400", "white"],
  light: { x: 76, y: 20, r: 40, paint: "white", opacity: 0.45 },
  emblem: { x: 20, y: 30, size: 5, rotate: 0 },
  figure: { x: 28, y: 80, scale: 0.6, pose: "pose-point", look: { ...normalCoreLookOf("core-061") } },
  extras: [
    { layer: "mid", x: 64, y: 74, scale: 0.42, flip: -1, pose: "pose-sit", look: COMMITTEE },
    { layer: "mid", x: 74, y: 74, scale: 0.42, flip: -1, pose: "pose-sit", look: { ...COMMITTEE, hair: "hair-short", cloth: "green-900" } },
    { layer: "mid", x: 84, y: 74, scale: 0.42, flip: -1, pose: "pose-sit", look: { ...COMMITTEE, hair: "hair-long", cloth: "blue-900" } },
  ],
  layers: {
    sky: [
      { part: "sun", x: 76, y: 18, scale: 0.6 },
      { part: "cloud", x: 30, y: 22, scale: 0.9, paint: "white" },
    ],
    far: [
      { part: "rotunda", x: 50, y: 58, scale: 0.5 },
      { part: "tree", x: 10, y: 60, scale: 1, paint: "green-700" },
      { part: "tree", x: 92, y: 58, scale: 0.9, flip: -1, paint: "green-700" },
    ],
    mid: [{ part: "field", x: 50, y: 62, paint: "green-500" }],
    subject: [
      { part: "soldier", x: 44, y: 80, scale: 1.3, paint: "slate-100" },
      { part: "wide-table", x: 74, y: 80, scale: 0.9, paint: "white" },
    ],
    near: [
      { part: "grass-tuft", x: 50, y: 97, count: 3, spread: [40, 2], paint: "green-700" },
      { part: "flowers", x: 12, y: 96, scale: 1.2 },
    ],
    light: [{ part: "halo", x: 44, y: 70, scale: 1, paint: "white" }],
    particles: [{ part: "motes", x: 50, y: 50, scale: 1.2, paint: "white" }],
  },
};

/** Gifted Program (Core #64, Rare), Alt art: a wide view of the program's summer camp at noon, prodigies on a lake jetty holding up shimmering cards, one diving in; saturated lake blue. The jetty is a long `table` on its legs in the water. */
const GIFTED_PROGRAM_ALT: SceneRecipe = {
  family: "core",
  sky: ["sky-400", "cyan-400"],
  light: { x: 50, y: 14, r: 44, paint: "white", opacity: 0.5 },
  emblem: { x: 18, y: 30, size: 5, rotate: 0 },
  figure: { x: 40, y: 70, scale: 0.5, pose: "pose-raise", look: { ...normalCoreLookOf("core-064") } },
  extras: [
    { layer: "subject", x: 22, y: 71, scale: 0.48, pose: "pose-raise", look: PRODIGY },
    { layer: "subject", x: 74, y: 58, scale: 0.42, flip: -1, pose: "pose-spread", look: { ...PRODIGY, item: null, cloth: "orange-400" } },
  ],
  layers: {
    sky: [
      { part: "sun", x: 50, y: 14, scale: 0.6 },
      { part: "cloud", x: 80, y: 24, scale: 0.8, paint: "white" },
    ],
    far: [
      { part: "hills", x: 50, y: 50, paint: "green-700" },
      { part: "tree", x: 14, y: 54, scale: 0.7, count: 3, spread: [10, 2], paint: "green-700" },
      { part: "tent", x: 84, y: 54, scale: 0.4 },
    ],
    mid: [{ part: "water", x: 50, y: 56, paint: "blue-700" }],
    subject: [
      { part: "table", x: 20, y: 80, scale: 1.2, paint: "wood-600" },
      { part: "table", x: 44, y: 80, scale: 1.2, paint: "wood-600" },
      { part: "spray", x: 78, y: 76, scale: 1.2 },
    ],
    near: [{ part: "spray", x: 80, y: 86, scale: 1 }],
    light: [{ part: "halo", x: 40, y: 44, scale: 1, paint: "white" }],
    particles: [{ part: "sparks", x: 40, y: 42, scale: 1.2, paint: "cyan-400" }],
  },
};

/** Masochism Mask (Core #65, Epic), Alt art: a wide vista of the mask worn by a lone wanderer crossing a sunlit thorn desert, cards glowing in her satchel; pale sand and sky. The thorns are a `dead-tree` and `cactus`. */
const MASOCHISM_MASK_ALT: SceneRecipe = {
  family: "core",
  sky: ["sky-300", "white"],
  light: { x: 78, y: 18, r: 44, paint: "white", opacity: 0.5 },
  emblem: { x: 20, y: 30, size: 5, rotate: 0 },
  extras: [{ layer: "mid", x: 44, y: 76, scale: 0.5, pose: "pose-stride", look: WANDERER }],
  layers: {
    sky: [{ part: "sun", x: 78, y: 18, scale: 0.7 }],
    far: [
      { part: "mesa", x: 24, y: 58, scale: 0.6 },
      { part: "mesa", x: 80, y: 56, scale: 0.4 },
    ],
    mid: [
      { part: "sand", x: 50, y: 58 },
      { part: "cactus", x: 14, y: 72, scale: 1 },
      { part: "dead-tree", x: 84, y: 70, scale: 1 },
    ],
    subject: [
      { part: "mask", x: 44, y: 51.3, scale: 0.45 },
      { part: "playing-card", x: 48, y: 64, scale: 0.4, count: 2, spread: [2, 1] },
    ],
    near: [
      { part: "cactus", x: 90, y: 99, scale: 1.8 },
      { part: "dead-tree", x: 8, y: 99, scale: 1.4 },
      { part: "pebbles", x: 50, y: 96 },
    ],
    light: [
      { part: "halo", x: 48, y: 64, scale: 0.5, paint: "red-300" },
      { part: "halo", x: 78, y: 18, scale: 1.6, paint: "white" },
    ],
    particles: [{ part: "dust", x: 50, y: 74, scale: 1.2 }],
  },
};

/** R3398000: Core's alt recipes #37 to #65, by card id. */
export const CORE_ALT_RECIPES_2: Readonly<Record<string, SceneRecipe>> = {
  "core-037": GRAVEDIGGER_ALT,
  "core-038": QUICKSTRIKER_ALT,
  "core-039": RECYCLING_INITIATIVE_ALT,
  "core-042": SELECTIVE_BREEDING_ALT,
  "core-043": BIG_FELINOR_ALT,
  "core-044": TRUE_STRIKE_ALT,
  "core-047": FIG_OF_LIFE_ALT,
  "core-048": FIVE_PEK_CONTROLLER_ALT,
  "core-049": SNOM_BUNNY_MIND_CONTROL_ALT,
  "core-050": K_POP_FANATIC_ALT,
  "core-052": SILLY_SILAS_ALT,
  "core-053": RENO_ALT,
  "core-055": LAVA_GOLEM_ALT,
  "core-056": JILLIAX_ALT,
  "core-057": CONJURE_KY_ALT,
  "core-058": RUSH_TOKEN_FARM_ALT,
  "core-059": UNBIASED_IMMIGRATION_ALT,
  "core-061": PREJUDICED_POSTDOC_ALT,
  "core-064": GIFTED_PROGRAM_ALT,
  "core-065": MASOCHISM_MASK_ALT,
};
