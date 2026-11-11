// R3394000: the alt scene recipes of Stinkyscape's cards 1 to 90 and the Dormant Starship (#3394), one per card of its
// table offered Alt art, cards 1 to 26 here and the rest in stinkyscapeAltRecipes2.ts and stinkyscapeAltRecipes3.ts,
// each drawn from its brief (ART.md, "Stinkyscape's early Alt art restaged"): the card's own subject in another place,
// time or moment, under a sky sharing no colour with its normal picture's and a new camera (a close-up, a wide or
// distant view, or one from above), its figure the normal picture's person (`normalLookOf`), in Stinkyscape's fine
// black line, at or above its rarity's floor. Olivian Expanse, the early Starship Pieces and the Dormant Starship, whose
// normal face their own painter still draws (R3338001, R3333001, R3333000), stand their piece in the scene as a part of
// the kit (sceneKitStinkyAlt.ts, sceneKitStinkyEarlyAlt.ts) under a sky that is neither black nor slate-900. scene.ts
// `sceneRecipeOf` reads one under the card's Alt art seed alone, at every Alt art level. Presentation only (CLAUDE.md
// rule 7): a recipe is read by the card's id alone.

import { figureLookFor, type FigureLook } from "./figureKit.ts";
import type { SceneRecipe } from "./scene.ts";
import { STINKYSCAPE_RECIPES } from "./stinkyscapeRecipes.ts";

/** The look of the card's normal figure, so its Alt picture draws the same person; with none, the default person. */
export function normalLookOf(id: string): FigureLook {
  return STINKYSCAPE_RECIPES[id]?.figure?.look ?? figureLookFor([]);
}

/** Freeze Ray (Stinkyscape #1, Common), Alt art: before the shot: a close-up of the ray gun's coil charging in a gloved hand, frost creeping over the knuckles, warm orange cockpit light behind. */
const FREEZE_RAY_ALT: SceneRecipe = {
  family: "stinkyscape",
  sky: ["red-600", "orange-400"],
  light: { x: 66, y: 46, r: 32, paint: "orange-400", opacity: 0.4 },
  emblem: { x: 82, y: 28, size: 5, rotate: 0 },
  layers: {
    far: [{ part: "cockpit-frame", x: 50, y: 86 }],
    subject: [
      { part: "ray-gun", x: 50, y: 52, scale: 2.6 },
      { part: "frost-shard", x: 40, y: 62, scale: 1.3 },
      { part: "frost-shard", x: 46, y: 66, scale: 0.9 },
    ],
    near: [{ part: "gloved-hand", x: 40, y: 78, scale: 2.2 }],
    light: [{ part: "halo", x: 66, y: 50, scale: 1.1, paint: "cyan-400" }],
    particles: [
      { part: "sparks", x: 68, y: 48, scale: 0.8, paint: "sky-300" },
      { part: "snow", x: 44, y: 60, scale: 0.7, paint: "white" },
    ],
  },
};

/** Olivian Expanse (Stinkyscape #4, Mythic), Alt art: from a survey ship's cockpit: the last star shrinking behind, instruments dimming one by one, and ahead only a flat olive-grey nothing. Close interior view, hushed. The cockpit has no rear-view mirror, so the last star is a small glare high in the canopy. */
const OLIVIAN_EXPANSE_ALT: SceneRecipe = {
  family: "stinkyscape",
  sky: ["stone-500", "grey-500"],
  light: { x: 50, y: 44, r: 40, paint: "stone-500", opacity: 0.3 },
  emblem: { x: 80, y: 30, size: 5, rotate: 0 },
  layers: {
    sky: [{ part: "star-glare", x: 22, y: 26, scale: 0.3, paint: "white" }],
    far: [
      { part: "mist", x: 50, y: 50, scale: 2, paint: "stone-500" },
      { part: "mist", x: 30, y: 62, scale: 1.4, paint: "grey-500" },
    ],
    mid: [{ part: "mist", x: 70, y: 58, scale: 1.2, count: 2, spread: [20, 6], paint: "slate-400" }],
    subject: [
      { part: "cockpit-frame", x: 50, y: 70 },
      { part: "console", x: 50, y: 84, scale: 1.4 },
    ],
    near: [
      { part: "monitors", x: 20, y: 96, scale: 1, count: 2, spread: [8, 2], fade: 0.6 },
      { part: "monitors", x: 80, y: 96, scale: 1, count: 2, spread: [8, 2], fade: 0.35 },
      { part: "bulbs", x: 50, y: 92, scale: 0.9, fade: 0.5 },
      { part: "cable", x: 6, y: 90, scale: 0.8 },
      { part: "cable", x: 94, y: 90, scale: 0.8, flip: -1 },
      { part: "gloved-hand", x: 64, y: 100, scale: 1.4 },
    ],
    light: [
      { part: "rim-light", x: 50, y: 74, scale: 1.4, paint: "slate-100" },
      { part: "halo", x: 22, y: 26, scale: 0.5, paint: "white" },
      { part: "lens-flare", x: 22, y: 26, scale: 0.4, paint: "white" },
    ],
    particles: [
      { part: "motes", x: 50, y: 50, scale: 1.4, count: 2, spread: [24, 10], paint: "slate-100" },
      { part: "dust", x: 40, y: 80, scale: 1, count: 2, spread: [30, 6], paint: "stone-500" },
    ],
  },
};

/** Masochism Unmasked (Stinkyscape #5, Rare), Alt art: the trap set and waiting: a pristine chrome mask on a medical tray in a pale-blue infirmary, faint bruise-shaped reflections in it. Still life, clinical. The kit has no instrument tray, so the mask stands on a steel table. */
const MASOCHISM_UNMASKED_ALT: SceneRecipe = {
  family: "stinkyscape",
  sky: ["blue-200", "slate-100"],
  light: { x: 52, y: 40, r: 36, paint: "white", opacity: 0.4 },
  emblem: { x: 82, y: 30, size: 4, rotate: 0 },
  layers: {
    far: [
      { part: "tile-wall", x: 50, y: 72 },
      { part: "cabinet", x: 14, y: 72, scale: 1.2, paint: "slate-400" },
      { part: "cot", x: 84, y: 74, scale: 1.1 },
    ],
    mid: [{ part: "table", x: 52, y: 84, scale: 1.8, paint: "slate-400" }],
    subject: [{ part: "chrome-mask", x: 52, y: 56, scale: 2.1 }],
    near: [
      { part: "syringe", x: 76, y: 82, scale: 0.9 },
      { part: "vial", x: 28, y: 78, scale: 1, count: 2, spread: [6, 1] },
    ],
    light: [
      { part: "ceiling-lights", x: 50, y: 14, scale: 0.9 },
      { part: "halo", x: 52, y: 56, scale: 1.3, paint: "blue-200" },
    ],
    particles: [{ part: "motes", x: 52, y: 46, scale: 1, paint: "white" }],
  },
};

/** Tylannt (Stinkyscape #6, Mythic), Alt art: before the job: a close-up portrait of Tylannt in a quiet green-lit casino bar, calmly turning a lockpick in his fingers over a small hologram of a vault. The kit has no lockpick, so he turns a key; the hologram is a faint vault door. */
const TYLANNT_ALT: SceneRecipe = {
  family: "stinkyscape",
  sky: ["green-900", "jade-400"],
  light: { x: 62, y: 62, r: 30, paint: "jade-400", opacity: 0.4 },
  emblem: { x: 84, y: 30, size: 5, rotate: 0 },
  figure: { x: 44, y: 106, scale: 1.5, pose: "pose-hold", look: { ...normalLookOf("stinkyscape-006"), item: "item-key" } },
  layers: {
    sky: [{ part: "lantern-string", x: 50, y: 22, scale: 1.4, paint: "green-500" }],
    far: [
      { part: "room-wall", x: 50, y: 80, paint: "green-900" },
      { part: "shelf", x: 18, y: 58, scale: 1.1, paint: "wood-700" },
      { part: "roulette", x: 82, y: 70, scale: 0.7 },
    ],
    mid: [
      { part: "bottle", x: 18, y: 46, scale: 0.6, count: 2, spread: [8, 2] },
      { part: "credit-chips", x: 80, y: 62, scale: 0.6, count: 2, spread: [8, 2] },
    ],
    subject: [{ part: "vault-door", x: 66, y: 70, scale: 0.45, paint: "cyan-400", fade: 0.55 }],
    near: [
      { part: "counter", x: 50, y: 100, scale: 2.2, paint: "wood-800" },
      { part: "cup", x: 22, y: 90, scale: 1.4, count: 2, spread: [6, 1] },
      { part: "credit-chip", x: 80, y: 90, scale: 1, count: 3, spread: [6, 2] },
    ],
    light: [
      { part: "halo", x: 66, y: 70, scale: 1.2, paint: "cyan-400" },
      { part: "rim-light", x: 44, y: 70, scale: 1.4, paint: "jade-400" },
    ],
    particles: [
      { part: "smoke", x: 30, y: 50, scale: 0.8, paint: "slate-400" },
      { part: "motes", x: 60, y: 50, scale: 1.2, count: 2, spread: [20, 10], paint: "green-200" },
    ],
  },
};

/** Otherworldly Endurance (Stinkyscape #10, Epic), Alt art: the three prayer-lights arriving at sunrise on a rocky moon's open-air altar, a lone pilgrim in orange robes kneeling. Wide vista under a peach sky. */
const OTHERWORLDLY_ENDURANCE_ALT: SceneRecipe = {
  family: "stinkyscape",
  sky: ["red-300", "orange-400"],
  light: { x: 80, y: 64, r: 36, paint: "white", opacity: 0.35 },
  emblem: { x: 18, y: 30, size: 5, rotate: 0 },
  figure: {
    x: 34,
    y: 82,
    scale: 0.55,
    pose: "pose-kneel",
    look: { ...figureLookFor(["Human"]), hair: "hood", outfit: "outfit-robe", item: null, skin: "wood-700", cloth: "orange-400" },
  },
  layers: {
    sky: [
      { part: "sun", x: 80, y: 64, scale: 1 },
      { part: "cloud", x: 30, y: 24, scale: 1.2, paint: "red-300" },
    ],
    far: [
      { part: "mesa", x: 82, y: 70, scale: 0.8, paint: "wood-700" },
      { part: "cliff", x: 16, y: 56, scale: 1.2, paint: "wood-700" },
    ],
    mid: [
      { part: "moon-surface", x: 50, y: 76 },
      { part: "altar", x: 58, y: 80, scale: 1.3 },
    ],
    subject: [
      { part: "flame-column", x: 48, y: 64, scale: 0.4 },
      { part: "wisp", x: 58, y: 54, scale: 0.9 },
      { part: "hex-shield", x: 68, y: 62, scale: 0.32 },
    ],
    near: [
      { part: "pebbles", x: 16, y: 96, scale: 1.4 },
      { part: "pebbles", x: 82, y: 94, scale: 1.2 },
    ],
    light: [
      { part: "god-rays", x: 80, y: 64, scale: 1.2, paint: "white" },
      { part: "halo", x: 58, y: 58, scale: 1.4, paint: "white" },
    ],
    particles: [{ part: "motes", x: 56, y: 56, scale: 1.2, paint: "white" }],
  },
};

/** Otherworldly Magic (Stinkyscape #11, Epic), Alt art: afterwards: the student alone in a frost- and scorch-marked library aisle, a blank book smoking open, one frost shard still melting. Low angle, warm brown lamplight. */
const OTHERWORLDLY_MAGIC_ALT: SceneRecipe = {
  family: "stinkyscape",
  sky: ["wood-800", "wood-600"],
  light: { x: 50, y: 16, r: 34, paint: "orange-400", opacity: 0.4 },
  emblem: { x: 82, y: 30, size: 5, rotate: 0 },
  figure: {
    x: 50,
    y: 94,
    scale: 0.9,
    pose: "pose-stand",
    look: { ...figureLookFor(["Human"]), hair: "hair-short", outfit: "outfit-sweater", item: null, skin: "red-300", cloth: "blue-800" },
  },
  layers: {
    far: [
      { part: "aisle", x: 50, y: 78 },
      { part: "bookcase", x: 12, y: 86, scale: 1.4, paint: "wood-700" },
      { part: "bookcase", x: 88, y: 86, scale: 1.4, flip: -1, paint: "wood-700" },
    ],
    mid: [{ part: "bookshelf", x: 26, y: 80, scale: 1.1, count: 2, spread: [48, 2] }],
    subject: [
      { part: "frost-shard", x: 34, y: 82, scale: 1 },
      { part: "tome", x: 64, y: 86, scale: 0.9 },
    ],
    near: [
      { part: "frost", x: 18, y: 98, scale: 1.4 },
      { part: "pages", x: 80, y: 94, scale: 0.8 },
    ],
    light: [
      { part: "lantern", x: 50, y: 12, scale: 1.2 },
      { part: "halo", x: 50, y: 18, scale: 1.4, paint: "orange-400" },
    ],
    particles: [
      { part: "smoke", x: 64, y: 74, scale: 0.7, paint: "slate-400" },
      { part: "embers", x: 70, y: 80, scale: 0.8, paint: "orange-400" },
    ],
  },
};

/** Arm of the Jempire (Stinkyscape #14, Common), Alt art: off duty: a close-up of the enforcer's huge hydraulic arm as a mechanic oils it in a sea-green maintenance bay, the rest of him in shadow. The kit has no oil can, so the mechanic holds a flask. */
const ARM_OF_THE_JEMPIRE_ALT: SceneRecipe = {
  family: "stinkyscape",
  sky: ["green-900", "jade-400"],
  light: { x: 64, y: 50, r: 34, paint: "jade-400", opacity: 0.4 },
  emblem: { x: 84, y: 30, size: 4, rotate: 0 },
  figure: { x: 34, y: 128, scale: 2.1, pose: "pose-flex", look: { ...normalLookOf("stinkyscape-014"), cloth: "slate-800" } },
  extras: [
    {
      layer: "subject",
      x: 74,
      y: 100,
      scale: 1.1,
      flip: -1,
      pose: "pose-reach",
      look: { ...figureLookFor(["Human"]), hair: "beanie", outfit: "outfit-overalls", item: "item-flask", skin: "wood-700", cloth: "jade-400" },
    },
  ],
  layers: {
    far: [{ part: "hangar-wall", x: 50, y: 86, paint: "green-700" }],
    mid: [{ part: "pipes", x: 84, y: 60, scale: 1.2 }],
    near: [{ part: "crate", x: 88, y: 98, scale: 1.4 }],
    light: [{ part: "light-shaft", x: 66, y: 0, scale: 1, paint: "jade-400" }],
    particles: [
      { part: "drops", x: 58, y: 66, scale: 0.7, paint: "slate-400" },
      { part: "sparks", x: 50, y: 56, scale: 0.6, paint: "white" },
    ],
  },
};

/** Imitated Instrumentality (Stinkyscape #18, Legendary), Alt art: the vote's aftermath: three cores glowing a unanimous red over an empty commander's chair, seen from high above in a vast hall. Long shadows, ominous crimson. */
const IMITATED_INSTRUMENTALITY_ALT: SceneRecipe = {
  family: "stinkyscape",
  sky: ["red-900", "red-600"],
  light: { x: 50, y: 56, r: 40, paint: "red-500", opacity: 0.4 },
  emblem: { x: 82, y: 30, size: 4, rotate: 0 },
  layers: {
    sky: [{ part: "ceiling-lights", x: 50, y: 10, scale: 1.2 }],
    far: [
      { part: "tile-floor", x: 50, y: 30 },
      { part: "column", x: 10, y: 60, scale: 1.2 },
      { part: "column", x: 90, y: 60, scale: 1.2 },
    ],
    mid: [{ part: "light-grid", x: 50, y: 70, scale: 1.2, paint: "red-900" }],
    subject: [
      { part: "server-core", x: 50, y: 52, scale: 0.7, paint: "red-900" },
      { part: "server-core", x: 32, y: 64, scale: 0.7, paint: "red-900" },
      { part: "server-core", x: 68, y: 64, scale: 0.7, paint: "red-900" },
      { part: "pilot-seat", x: 50, y: 80, scale: 0.6 },
    ],
    near: [
      { part: "control-desk", x: 14, y: 96, scale: 1.2 },
      { part: "control-desk", x: 86, y: 96, scale: 1.2, flip: -1 },
    ],
    light: [
      { part: "halo", x: 50, y: 34, scale: 1.2, paint: "red-500" },
      { part: "halo", x: 32, y: 46, scale: 1, paint: "red-500" },
      { part: "halo", x: 68, y: 46, scale: 1, paint: "red-500" },
    ],
    particles: [
      { part: "embers", x: 50, y: 50, scale: 1.4, paint: "red-400" },
      { part: "sparks", x: 50, y: 40, scale: 1, paint: "red-300" },
      { part: "motes", x: 50, y: 70, scale: 1.2, paint: "red-300" },
    ],
  },
};

/** Tnack Trav (Stinkyscape #20, Epic), Alt art: Tnack Trav in a hall of mirrors at a station fairground, a dozen reflections each wearing a different stolen hero's outline. Warm magenta-orange carnival light. The kit has no hero outlines, so the reflections are faint ghost figures. */
const TNACK_TRAV_ALT: SceneRecipe = {
  family: "stinkyscape",
  sky: ["purple-400", "orange-400"],
  light: { x: 52, y: 52, r: 36, paint: "orange-400", opacity: 0.4 },
  emblem: { x: 16, y: 30, size: 4, rotate: 0 },
  figure: { x: 52, y: 90, scale: 0.8, pose: "pose-hips", look: { ...normalLookOf("stinkyscape-020"), item: null } },
  layers: {
    sky: [{ part: "bunting", x: 50, y: 22, scale: 1.4 }],
    far: [
      { part: "mirror", x: 18, y: 82, scale: 1.8 },
      { part: "mirror", x: 50, y: 74, scale: 1.4 },
      { part: "mirror", x: 82, y: 82, scale: 1.8 },
    ],
    mid: [
      { part: "ghost-figure", x: 14, y: 78, scale: 1.2, count: 6, spread: [16, 4] },
      { part: "ghost-figure", x: 70, y: 78, scale: 1.2, count: 6, spread: [16, 4], flip: -1 },
    ],
    subject: [{ part: "stage", x: 50, y: 90 }],
    near: [
      { part: "bulbs", x: 50, y: 98, scale: 1.6 },
      { part: "ticket-stub", x: 80, y: 94, scale: 1 },
    ],
    light: [
      { part: "rim-light", x: 52, y: 64, scale: 1.2, paint: "purple-400" },
      { part: "halo", x: 52, y: 60, scale: 1.4, paint: "orange-400" },
    ],
    particles: [{ part: "confetti", x: 50, y: 40, scale: 1.2 }],
  },
};

/** Moonberry (Stinkyscape #21, Common), Alt art: a wide orchard of moonberry vines climbing a crater wall at bright alien noon, pickers with baskets, the berries dusky against an orange sky. */
const MOONBERRY_ALT: SceneRecipe = {
  family: "stinkyscape",
  sky: ["orange-400", "red-300"],
  light: { x: 70, y: 18, r: 34, paint: "white", opacity: 0.4 },
  emblem: { x: 84, y: 34, size: 4, rotate: 0 },
  extras: [
    {
      layer: "mid",
      x: 70,
      y: 76,
      scale: 0.4,
      pose: "pose-reach",
      look: { ...figureLookFor(["Human"]), hair: "bucket-hat", outfit: "outfit-overalls", item: "item-sack", skin: "red-300", cloth: "jade-400" },
    },
    {
      layer: "mid",
      x: 24,
      y: 80,
      scale: 0.42,
      flip: -1,
      pose: "pose-reach",
      look: { ...figureLookFor(["Human"]), hair: "kerchief", outfit: "outfit-overalls", item: "item-sack", skin: "wood-700", cloth: "blue-800" },
    },
  ],
  layers: {
    sky: [{ part: "sun", x: 70, y: 18, scale: 0.7 }],
    far: [
      { part: "cliff", x: 40, y: 46, scale: 1.8, paint: "wood-700" },
      { part: "moon-surface", x: 50, y: 70 },
    ],
    subject: [{ part: "berry-vine", x: 40, y: 66, scale: 1.2, count: 4, spread: [24, 8] }],
    near: [
      { part: "basket", x: 80, y: 94, scale: 1.4 },
      { part: "berry-vine", x: 10, y: 102, scale: 2 },
    ],
  },
};

/** KY's Tricknology (Stinkyscape #25, Epic), Alt art: before the proof: KY at a cluttered desk in a warm green study pod, a tiny hologram grid twisting above a teacup. Close, cosy, overhead angle. */
const KYS_TRICKNOLOGY_ALT: SceneRecipe = {
  family: "stinkyscape",
  sky: ["green-700", "lime-200"],
  light: { x: 62, y: 64, r: 30, paint: "lime-200", opacity: 0.4 },
  emblem: { x: 16, y: 30, size: 4, rotate: 0 },
  figure: { x: 44, y: 104, scale: 1.3, pose: "pose-sit", look: normalLookOf("stinkyscape-025") },
  layers: {
    far: [
      { part: "room-wall", x: 50, y: 70, paint: "green-700" },
      { part: "round-window", x: 80, y: 40, scale: 0.8 },
    ],
    mid: [{ part: "bookshelf", x: 14, y: 76, scale: 1.2, paint: "wood-700" }],
    subject: [{ part: "light-grid", x: 64, y: 70, scale: 0.3, paint: "cyan-400" }],
    near: [
      { part: "desk", x: 56, y: 108, scale: 3.4 },
      { part: "cup", x: 64, y: 84, scale: 1.4 },
      { part: "notebook", x: 30, y: 88, scale: 1 },
      { part: "closed-tome", x: 82, y: 86, scale: 1, count: 2, spread: [2, 3] },
      { part: "quill", x: 74, y: 86, scale: 0.8 },
    ],
    light: [{ part: "halo", x: 64, y: 72, scale: 0.9, paint: "cyan-400" }],
    particles: [{ part: "motes", x: 50, y: 50, scale: 1.2, paint: "lime-200" }],
  },
};

/** ZolarKeth (Stinkyscape #26, Legendary), Alt art: finally aboard: ZolarKeth strapped into a cramped launch capsule, medals floating loose, grinning as red ignition light floods the cabin. Tight close-up portrait. The kit has no medals or harness, so the medals are small discs and he sits without straps. */
const ZOLARKETH_ALT: SceneRecipe = {
  family: "stinkyscape",
  sky: ["red-600", "ember-500"],
  light: { x: 50, y: 50, r: 40, paint: "red-500", opacity: 0.4 },
  emblem: { x: 82, y: 30, size: 4, rotate: 0 },
  figure: { x: 50, y: 112, scale: 1.6, pose: "pose-sit", look: { ...normalLookOf("stinkyscape-026"), item: null } },
  layers: {
    sky: [{ part: "riveted-porthole", x: 82, y: 42, scale: 0.8 }],
    far: [{ part: "bulkhead", x: 50, y: 86 }],
    mid: [
      { part: "pilot-seat", x: 54, y: 112, scale: 1.8 },
      { part: "monitors", x: 18, y: 70, scale: 0.8 },
    ],
    subject: [{ part: "jade-disc", x: 30, y: 40, scale: 0.4, count: 4, spread: [40, 14], paint: "brass-600" }],
    near: [
      { part: "console", x: 86, y: 100, scale: 1.2, flip: -1 },
      { part: "cable", x: 8, y: 92, scale: 1 },
    ],
    light: [
      { part: "alarm-light", x: 16, y: 30, scale: 1.2 },
      { part: "halo", x: 50, y: 60, scale: 1.6, paint: "red-500" },
    ],
    particles: [
      { part: "sparks", x: 50, y: 40, scale: 1, paint: "orange-400" },
      { part: "embers", x: 70, y: 70, scale: 1, paint: "red-400" },
    ],
  },
};

/** R3394000: the alt recipes of #3394's cards 1 to 26, by id. */
export const STINKYSCAPE_ALT_RECIPES_1: Readonly<Record<string, SceneRecipe>> = {
  "stinkyscape-001": FREEZE_RAY_ALT,
  "stinkyscape-004": OLIVIAN_EXPANSE_ALT,
  "stinkyscape-005": MASOCHISM_UNMASKED_ALT,
  "stinkyscape-006": TYLANNT_ALT,
  "stinkyscape-010": OTHERWORLDLY_ENDURANCE_ALT,
  "stinkyscape-011": OTHERWORLDLY_MAGIC_ALT,
  "stinkyscape-014": ARM_OF_THE_JEMPIRE_ALT,
  "stinkyscape-018": IMITATED_INSTRUMENTALITY_ALT,
  "stinkyscape-020": TNACK_TRAV_ALT,
  "stinkyscape-021": MOONBERRY_ALT,
  "stinkyscape-025": KYS_TRICKNOLOGY_ALT,
  "stinkyscape-026": ZOLARKETH_ALT,
};
