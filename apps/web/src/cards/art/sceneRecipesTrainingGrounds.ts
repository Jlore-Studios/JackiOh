// R3393000: the Training Grounds set's scene recipes (ART.md, "Scene recipes and kits"), #1 to #40 and the Golem
// token, each drawn from its brief in issue #3393's table in the Training Grounds family's look (ART.md's art
// bible): carved wood, the yard's palisade and sand wherever the brief is set in it, the sun from the upper right
// but at night. A Unit is a figure-kit character. Each is placed in the 100-unit box as sceneRecipes.ts's header
// says, and reaches its rarity's floor (richness.ts `ART_DETAIL_MIN`) with its placements' counts. The yard
// painter (traininggrounds.ts) draws only a card with no recipe.

import { figureLookFor, type FigureLook } from "./figureKit.ts";
import type { PaletteName, ScenePlacement, SceneRecipe } from "./scene.ts";

/** The yard's spiked palisade across the back of a scene, its foot at `y`, in two runs. */
function palisadeAt(y: number, paint: PaletteName = "wood-700", scale = 1): ScenePlacement[] {
  return [
    { part: "palisade", x: 24, y, scale, paint },
    { part: "palisade", x: 76, y, scale, flip: -1, paint },
  ];
}

/** A recruit of the yard: short hair, a blue tunic, no weapon. */
const RECRUIT: FigureLook = { ...figureLookFor(["Human"]), item: null };

/** Fan of Knives (Training Grounds #10, Common): a dozen knives fan out from a spinning blur on the knife-throwing range, every target along the fence struck at once; steel-blue morning. */
const FAN_OF_KNIVES: SceneRecipe = {
  family: "training-grounds",
  sky: ["blue-800", "sky-300"],
  light: { x: 50, y: 50, r: 30, paint: "white", opacity: 0.35 },
  emblem: { x: 50, y: 50, size: 6, rotate: 0 },
  layers: {
    sky: [{ part: "sun", x: 82, y: 25, scale: 0.7 }],
    far: palisadeAt(60, "wood-800"),
    mid: [
      { part: "sand", x: 50, y: 60 },
      { part: "target", x: 14, y: 70, scale: 0.9 },
      { part: "target", x: 86, y: 70, scale: 0.9, flip: -1 },
    ],
    subject: [
      { part: "light-ribbon", x: 50, y: 50, scale: 0.45, paint: "slate-100" },
      { part: "knife", x: 36, y: 42, scale: 1.6, flip: -1 },
      { part: "knife", x: 64, y: 42, scale: 1.6 },
      { part: "knife", x: 34, y: 58, scale: 1.6, flip: -1 },
      { part: "knife", x: 66, y: 58, scale: 1.6 },
    ],
    near: [{ part: "fence-post", x: 8, y: 98, scale: 1.1 }],
    particles: [{ part: "sparks", x: 50, y: 50, scale: 1.2 }],
  },
};

/** Thwomp (Training Grounds #11, Common): a huge faceless iron-banded oak block plunges from the drill shed's doorway onto a startled recruit, dust bursting; heavy brown shadows. */
const THWOMP: SceneRecipe = {
  family: "training-grounds",
  sky: ["wood-900", "wood-700"],
  light: { x: 50, y: 34, r: 30, paint: "orange-400", opacity: 0.25 },
  emblem: { x: 50, y: 40, size: 6, rotate: 0 },
  extras: [{ layer: "subject", x: 50, y: 86, scale: 0.42, pose: "pose-spread", look: RECRUIT }],
  layers: {
    far: [{ part: "barracks", x: 50, y: 66, scale: 1.9, paint: "wood-800" }],
    mid: [{ part: "sand", x: 50, y: 66, paint: "wood-600" }],
    subject: [{ part: "oak-block", x: 50, y: 50, scale: 1.4 }],
    near: [{ part: "crate", x: 10, y: 98, scale: 1.3 }],
    particles: [{ part: "dust", x: 50, y: 86, scale: 1.4, paint: "stone-500" }],
  },
};

/** Fortify (Training Grounds #12, Common): sandbags stack themselves round a recruit on the timber rampart as a helmet drops onto his head and a sergeant wags a finger; overcast grey. */
const FORTIFY: SceneRecipe = {
  family: "training-grounds",
  sky: ["slate-400", "slate-100"],
  light: { x: 46, y: 40, r: 30, paint: "white", opacity: 0.3 },
  emblem: { x: 46, y: 64, size: 5, rotate: 0 },
  figure: { x: 44, y: 84, scale: 0.6, pose: "pose-guard", look: { ...RECRUIT, hair: "helm" } },
  extras: [{ layer: "subject", x: 72, y: 84, scale: 0.55, flip: -1, pose: "pose-raise", look: { ...RECRUIT, cloth: "slate-700" } }],
  layers: {
    sky: [{ part: "cloud", x: 30, y: 26, scale: 0.9, count: 2, spread: [20, 4], paint: "slate-400" }],
    far: [{ part: "hills", x: 50, y: 62, paint: "slate-700" }],
    mid: palisadeAt(78, "wood-800"),
    subject: [
      { part: "sandbags", x: 34, y: 86, scale: 1.1 },
      { part: "sandbags", x: 50, y: 88, scale: 1.3 },
    ],
    near: [{ part: "sandbags", x: 88, y: 98, scale: 1.4 }],
  },
};

/** Explosive Runes (Training Grounds #13, Common): scorched glyphs flare red on the plank path to the commander's tent as an attacker's boot lands, the blast bursting up; smoky night. */
const EXPLOSIVE_RUNES: SceneRecipe = {
  family: "training-grounds",
  sky: ["slate-900", "blue-900"],
  light: { x: 50, y: 70, r: 30, paint: "ember-500", opacity: 0.45 },
  emblem: { x: 50, y: 58, size: 6, rotate: 0 },
  extras: [{ layer: "subject", x: 40, y: 80, scale: 0.5, pose: "pose-run", look: { ...RECRUIT, hair: "hood", cloth: "red-900", item: "item-dagger" } }],
  layers: {
    sky: [{ part: "stars", x: 50, y: 24, scale: 1.2 }],
    far: [...palisadeAt(58, "wood-900"), { part: "tent", x: 50, y: 60, scale: 1.2 }],
    mid: [{ part: "lanes", x: 50, y: 60, paint: "wood-700" }],
    subject: [
      { part: "glyph-mark", x: 52, y: 80, scale: 1.4 },
      { part: "glyph-mark", x: 56, y: 68, scale: 0.9 },
      { part: "flames", x: 52, y: 80, scale: 1.3 },
    ],
    near: [{ part: "smoke", x: 16, y: 96, scale: 1.4, paint: "slate-700" }],
    light: [{ part: "halo", x: 52, y: 78, scale: 1.4, paint: "ember-500" }],
  },
};

/** Explosive Trap (Training Grounds #14, Common): a welcome mat at the barracks door, a lit fuse curling out from under it and a newcomer's boot hovering above it; tense red lamplight. */
const EXPLOSIVE_TRAP: SceneRecipe = {
  family: "training-grounds",
  sky: ["wood-900", "red-900"],
  light: { x: 70, y: 40, r: 30, paint: "red-500", opacity: 0.4 },
  emblem: { x: 52, y: 66, size: 5, rotate: 0 },
  extras: [{ layer: "subject", x: 30, y: 84, scale: 0.6, pose: "pose-run", look: { ...RECRUIT, cloth: "green-700" } }],
  layers: {
    far: [{ part: "barracks", x: 46, y: 74, scale: 2.4 }],
    mid: [{ part: "sand", x: 50, y: 74, paint: "wood-700" }],
    subject: [{ part: "fuse-mat", x: 52, y: 82, scale: 2 }],
    near: [
      { part: "fence-post", x: 84, y: 98, scale: 1.4, flip: -1 },
      { part: "lantern", x: 77.3, y: 81.5, scale: 1.4 },
    ],
    light: [{ part: "halo", x: 77.3, y: 86, scale: 1.4, paint: "red-500" }],
  },
};

/** In a hurry (Training Grounds #15, Common): four wooden runner tokens burst out of the yard's stable doors in a flat sprint, dust trails behind, no finish line in sight; windy and frantic. */
const IN_A_HURRY: SceneRecipe = {
  family: "training-grounds",
  sky: ["sky-400", "blue-200"],
  light: { x: 80, y: 24, r: 30, paint: "white", opacity: 0.3 },
  emblem: { x: 50, y: 52, size: 5, rotate: 0 },
  layers: {
    sky: [
      { part: "sun", x: 82, y: 24, scale: 0.7 },
      { part: "cloud", x: 30, y: 26, scale: 0.8, paint: "white" },
    ],
    far: [{ part: "barracks", x: 30, y: 62, scale: 1.3, paint: "wood-800" }],
    mid: [
      { part: "sand", x: 50, y: 62 },
      { part: "dust", x: 30, y: 72, scale: 1.2, paint: "stone-500" },
    ],
    subject: [
      { part: "runner-token", x: 44, y: 76, scale: 2.2 },
      { part: "runner-token", x: 58, y: 80, scale: 2.4 },
      { part: "runner-token", x: 66, y: 72, scale: 2 },
      { part: "runner-token", x: 52, y: 70, scale: 1.8 },
    ],
    near: [{ part: "dust", x: 30, y: 94, scale: 1.6, paint: "stone-500" }],
  },
};

/** Extra Rations (Training Grounds #19, Rare): the yard's mess hall: long carved tables, steaming pots, a ladle mid-pour and towers of bread for second helpings; warm hearth glow, cheerful clatter. */
const EXTRA_RATIONS: SceneRecipe = {
  family: "training-grounds",
  sky: ["wood-900", "wood-700"],
  light: { x: 50, y: 52, r: 40, paint: "orange-400", opacity: 0.4 },
  emblem: { x: 50, y: 70, size: 5, rotate: 0 },
  extras: [{ layer: "subject", x: 64, y: 84, scale: 0.5, flip: -1, pose: "pose-raise", look: { ...RECRUIT, outfit: "outfit-apron", item: "item-spoon", cloth: "white" } }],
  layers: {
    far: [
      { part: "room-wall", x: 50, y: 64, paint: "wood-700" },
      { part: "hearth", x: 50, y: 64, scale: 1.1 },
      { part: "banner", x: 16, y: 60, scale: 0.7 },
      { part: "banner", x: 84, y: 60, scale: 0.7, flip: -1 },
    ],
    mid: [
      { part: "tile-floor", x: 50, y: 64, paint: "wood-800" },
      { part: "table", x: 24, y: 76, scale: 1 },
      { part: "table", x: 76, y: 76, scale: 1, flip: -1 },
      { part: "bread", x: 20, y: 68.5, count: 2, spread: [5, 0] },
      { part: "bread", x: 80, y: 68.5, count: 2, spread: [5, 0] },
    ],
    subject: [
      { part: "cauldron", x: 46, y: 82, scale: 1.1 },
      { part: "steam", x: 46, y: 72, scale: 0.9 },
    ],
    near: [
      { part: "table", x: 50, y: 102, scale: 1.8 },
      { part: "bread", x: 34, y: 88, scale: 1.4 },
      { part: "bread", x: 34, y: 84.4, scale: 1.2 },
      { part: "bread", x: 66, y: 88, scale: 1.4 },
    ],
    light: [{ part: "halo", x: 50, y: 58, scale: 2, paint: "orange-400" }],
    particles: [{ part: "motes", x: 50, y: 50, scale: 1.2, paint: "orange-400" }],
  },
};

/** Recruitment Posters (Training Grounds #20, Rare): the recruiting wall outside the yard, layered with wordless posters of flexing arms and pointing hands, a free-lunch tray on a stool; bright breezy noon. */
const RECRUITMENT_POSTERS: SceneRecipe = {
  family: "training-grounds",
  sky: ["sky-400", "blue-200"],
  light: { x: 80, y: 22, r: 30, paint: "white", opacity: 0.35 },
  emblem: { x: 50, y: 50, size: 6, rotate: 0 },
  layers: {
    sky: [
      { part: "sun", x: 84, y: 22, scale: 0.7 },
      { part: "cloud", x: 26, y: 24, scale: 0.8, count: 2, spread: [12, 3], paint: "white" },
    ],
    far: [{ part: "hills", x: 50, y: 56 }],
    mid: [...palisadeAt(72, "wood-700", 1.5), { part: "sand", x: 50, y: 72 }],
    subject: [
      { part: "poster", x: 38, y: 64, scale: 1.3 },
      { part: "poster", x: 50, y: 60, scale: 1.4 },
      { part: "poster", x: 62, y: 64, scale: 1.3 },
      { part: "poster", x: 44, y: 70, scale: 1 },
      { part: "poster", x: 58, y: 69, scale: 1 },
      { part: "bench", x: 72, y: 86, scale: 0.9 },
      { part: "bread", x: 70, y: 81.4, scale: 0.8 },
      { part: "apple", x: 75, y: 81.6, scale: 0.8 },
    ],
    near: [{ part: "grass-tuft", x: 14, y: 96, count: 2, spread: [6, 1], paint: "green-700" }],
    particles: [{ part: "petals", x: 50, y: 40, scale: 1, paint: "white" }],
  },
};

/** Goliath Situation (Training Grounds #24, Rare): a giant ducking under the covered walkway's low beams trips a hidden lever, and a tiny recruit's pile of slingstones glows ready; a dramatic low angle. */
const GOLIATH_SITUATION: SceneRecipe = {
  family: "training-grounds",
  sky: ["slate-800", "blue-800"],
  light: { x: 76, y: 80, r: 24, paint: "cyan-400", opacity: 0.35 },
  emblem: { x: 44, y: 62, size: 6, rotate: 0 },
  extras: [
    { layer: "subject", x: 42, y: 90, scale: 1.05, pose: "pose-kneel", look: { ...RECRUIT, hair: "hair-long", outfit: "outfit-fur", cloth: "wood-700", skin: "stone-500" } },
    { layer: "subject", x: 74, y: 88, scale: 0.34, flip: -1, pose: "pose-stand", look: { ...RECRUIT, item: "item-sling" } },
  ],
  layers: {
    sky: [{ part: "cloud", x: 70, y: 26, scale: 0.9, paint: "slate-700" }],
    far: palisadeAt(66, "wood-900"),
    mid: [{ part: "sand", x: 50, y: 82 }],
    subject: [
      { part: "walkway", x: 46, y: 86, scale: 2 },
      { part: "pebbles", x: 80, y: 88.6, scale: 0.5 },
    ],
    near: [{ part: "pebbles", x: 24, y: 97, scale: 1.2 }],
    light: [{ part: "halo", x: 80, y: 87, scale: 0.8, paint: "cyan-400" }],
    particles: [{ part: "sparks", x: 80, y: 86, scale: 0.8 }],
  },
};

/** Path of Exile (Training Grounds #26, Rare): a training catapult hauled on ropes down a one-way road out of the yard's back gate, fading into dust; lonely orange sunset. */
const PATH_OF_EXILE: SceneRecipe = {
  family: "training-grounds",
  sky: ["violet-700", "orange-400"],
  light: { x: 76, y: 48, r: 34, paint: "orange-400", opacity: 0.45 },
  emblem: { x: 50, y: 56, size: 5, rotate: 0 },
  layers: {
    sky: [
      { part: "sun", x: 76, y: 46, scale: 0.9 },
      { part: "cloud", x: 28, y: 28, scale: 0.8, paint: "violet-500" },
    ],
    far: [{ part: "hills", x: 50, y: 60, paint: "wood-800" }],
    mid: [
      { part: "street", x: 50, y: 62, paint: "stone-500" },
      ...palisadeAt(70, "wood-900"),
      { part: "gate", x: 50, y: 70, scale: 1.4 },
    ],
    subject: [
      { part: "catapult", x: 50, y: 66, scale: 0.9, fade: 0.75 },
      { part: "dust", x: 50, y: 66, scale: 1.1, paint: "orange-400" },
    ],
    near: [
      { part: "fence-post", x: 12, y: 98, scale: 1.2 },
      { part: "fence-post", x: 88, y: 98, scale: 1.2, flip: -1 },
    ],
    particles: [{ part: "dust", x: 50, y: 60, scale: 1.4, paint: "orange-400" }],
  },
};

/** Hopeful Jelly Bean (Training Grounds #27, Rare): a pale lemon jelly bean tossed onto the carved siege tower at the yard's edge, the tower shining pearl-white from its base; hopeful pink dawn. */
const HOPEFUL_JELLY_BEAN: SceneRecipe = {
  family: "training-grounds",
  sky: ["violet-300", "red-300"],
  light: { x: 50, y: 78, r: 30, paint: "white", opacity: 0.45 },
  emblem: { x: 50, y: 60, size: 5, rotate: 0 },
  layers: {
    sky: [
      { part: "sun", x: 80, y: 28, scale: 0.7 },
      { part: "cloud", x: 26, y: 26, scale: 0.8, paint: "white" },
    ],
    far: palisadeAt(66, "wood-800"),
    mid: [{ part: "sand", x: 50, y: 68 }],
    subject: [
      { part: "siege-tower", x: 50, y: 84, scale: 1.4, paint: "slate-100" },
      { part: "jelly-bean", x: 62, y: 34, scale: 1.8 },
    ],
    near: [{ part: "grass-tuft", x: 86, y: 97, count: 2, spread: [6, 1], paint: "green-700" }],
    light: [
      { part: "halo", x: 50, y: 82, scale: 1.8, paint: "white" },
      { part: "light-shaft", x: 50, y: 30, paint: "white" },
    ],
    particles: [{ part: "sparks", x: 62, y: 34, scale: 1 }],
  },
};

/** Level Up! (Training Grounds #31, Epic): the sparring arena at the yard's heart: chalk-scuffed sand before it, a verdigris bell on a tall pole ringing as a recruit's blow lands, plain banners over the wooden stands behind; bright triumphant afternoon. */
const LEVEL_UP: SceneRecipe = {
  family: "training-grounds",
  sky: ["sky-400", "blue-200"],
  light: { x: 62, y: 44, r: 30, paint: "white", opacity: 0.4 },
  emblem: { x: 44, y: 56, size: 6, rotate: 0 },
  extras: [
    { layer: "subject", x: 40, y: 82, scale: 0.55, pose: "pose-strike", look: { ...RECRUIT, item: "item-mallet" } },
    { layer: "subject", x: 30, y: 84, scale: 0.5, flip: -1, pose: "pose-guard", look: { ...RECRUIT, item: "item-shield", cloth: "red-900" } },
  ],
  layers: {
    sky: [
      { part: "sun", x: 82, y: 22, scale: 0.7 },
      { part: "cloud", x: 30, y: 24, scale: 0.8, paint: "white" },
    ],
    far: [
      { part: "stands", x: 22, y: 60, scale: 1.4, paint: "wood-700" },
      { part: "stands", x: 78, y: 60, scale: 1.4, flip: -1, paint: "wood-700" },
      { part: "banner", x: 10, y: 50, scale: 0.6 },
      { part: "banner", x: 36, y: 49, scale: 0.6, paint: "blue-700" },
      { part: "banner", x: 64, y: 49, scale: 0.6, flip: -1, paint: "blue-700" },
      { part: "banner", x: 90, y: 50, scale: 0.6, flip: -1 },
    ],
    mid: [
      { part: "sand", x: 50, y: 62 },
      { part: "rune-circle", x: 46, y: 80, scale: 1.4, paint: "white" },
    ],
    subject: [{ part: "bell", x: 58, y: 84, scale: 1.3 }],
    near: [
      { part: "pebbles", x: 20, y: 96 },
      { part: "weapon-rack", x: 88, y: 99, scale: 0.8 },
    ],
    light: [
      { part: "halo", x: 63, y: 46, scale: 1.2, paint: "white" },
      { part: "rim-light", x: 40, y: 64, scale: 1, paint: "white" },
    ],
    particles: [{ part: "sparks", x: 63, y: 46, scale: 1.2, count: 2, spread: [5, 4] }],
  },
};

/** Dino-size (Training Grounds #34, Epic): in the yard's sand pit a startled recruit swells into a towering dinosaur mid-roar, his tunic bursting; sand erupts under a stomp before it, onlookers dive behind the fence; a scorching orange sky. */
const DINO_SIZE: SceneRecipe = {
  family: "training-grounds",
  sky: ["red-600", "orange-400"],
  light: { x: 60, y: 44, r: 36, paint: "orange-400", opacity: 0.45 },
  emblem: { x: 46, y: 64, size: 6, rotate: 0 },
  extras: [
    { layer: "mid", x: 16, y: 74, scale: 0.36, flip: -1, pose: "pose-run", look: { ...RECRUIT, cloth: "green-700" } },
    { layer: "mid", x: 84, y: 73, scale: 0.34, pose: "pose-spread", look: { ...RECRUIT, cloth: "red-900" } },
  ],
  layers: {
    sky: [
      { part: "sun", x: 84, y: 22, scale: 0.8 },
      { part: "cloud", x: 26, y: 26, scale: 0.8, paint: "red-300" },
    ],
    far: palisadeAt(62, "wood-800"),
    mid: [
      { part: "sand", x: 50, y: 64 },
      { part: "fence", x: 22, y: 76, scale: 0.6 },
      { part: "fence", x: 80, y: 76, scale: 0.6 },
    ],
    subject: [{ part: "dino", x: 46, y: 86, scale: 1.3 }],
    near: [
      { part: "dust", x: 42, y: 92, scale: 1.8, paint: "brass-500" },
      { part: "pebbles", x: 66, y: 96, scale: 1.2 },
    ],
    light: [{ part: "rim-light", x: 50, y: 60, scale: 1.6, paint: "orange-400" }],
    particles: [{ part: "dust", x: 50, y: 80, scale: 1.4, paint: "brass-500" }],
  },
};

/** Corpse Recycler (Training Grounds #35, Epic): a ramshackle shed behind the infirmary: fallen dummies ride a creaking conveyor into a wooden mill and fresh runner tokens sprint out the other end; bone-grey smoke from its chimney, sickly green lantern light. */
const CORPSE_RECYCLER: SceneRecipe = {
  family: "training-grounds",
  sky: ["slate-900", "green-900"],
  light: { x: 62, y: 60, r: 34, paint: "green-400", opacity: 0.35 },
  emblem: { x: 56, y: 68, size: 5, rotate: 0 },
  layers: {
    sky: [{ part: "moon", x: 80, y: 24, scale: 0.7 }],
    far: [
      { part: "hills", x: 50, y: 60, paint: "slate-800" },
      { part: "barracks", x: 16, y: 62, scale: 0.9, paint: "stone-500" },
      { part: "palisade", x: 80, y: 62, scale: 0.9, paint: "wood-900" },
    ],
    mid: [{ part: "sand", x: 50, y: 64, paint: "wood-700" }],
    subject: [
      { part: "mill", x: 58, y: 82, scale: 1.3 },
      { part: "straw-dummy", x: 39, y: 81, scale: 0.42 },
      { part: "straw-dummy", x: 46, y: 77.6, scale: 0.42 },
      { part: "runner-token", x: 80, y: 84, scale: 1.2 },
      { part: "runner-token", x: 86, y: 88, scale: 1.3 },
    ],
    near: [
      { part: "bones", x: 22, y: 96, scale: 1.2 },
      { part: "fence-post", x: 8, y: 98, scale: 1.2 },
      { part: "lantern", x: 13.8, y: 83.8, scale: 1.2, paint: "green-400" },
    ],
    light: [
      { part: "halo", x: 13.8, y: 88, scale: 1.2, paint: "green-400" },
      { part: "smoke", x: 70, y: 52, scale: 1.2, paint: "stone-500" },
    ],
    particles: [{ part: "motes", x: 50, y: 70, scale: 1.2, paint: "green-200" }],
  },
};

/** Seance (Training Grounds #37, Legendary): in the dusty attic of the instructors' hall at midnight, recruits clasp hands round a table of candles; above it a fallen siege tower's spirit rises and pours itself into an exact ghostly copy settling into one recruit's open hand; curtains billow, flames lean, pale cyan ectoplasm against plum shadow. */
const SEANCE: SceneRecipe = {
  family: "training-grounds",
  sky: ["slate-900", "violet-700"],
  light: { x: 50, y: 42, r: 40, paint: "cyan-400", opacity: 0.4 },
  emblem: { x: 50, y: 46, size: 6, rotate: 0 },
  extras: [
    { layer: "subject", x: 30, y: 90, scale: 0.5, pose: "pose-spread", look: { ...RECRUIT, cloth: "violet-500" } },
    { layer: "subject", x: 70, y: 90, scale: 0.5, flip: -1, pose: "pose-spread", look: { ...RECRUIT, cloth: "blue-800" } },
    { layer: "subject", x: 58, y: 86, scale: 0.42, flip: -1, pose: "pose-kneel", look: { ...RECRUIT, hair: "hair-long", cloth: "slate-700" } },
  ],
  layers: {
    sky: [{ part: "stars", x: 50, y: 22, scale: 1 }],
    far: [
      { part: "room-wall", x: 50, y: 70, paint: "violet-700" },
      { part: "banner", x: 14, y: 66, scale: 1.1, paint: "violet-500" },
      { part: "banner", x: 86, y: 66, scale: 1.1, flip: -1, paint: "violet-500" },
      { part: "bookshelf", x: 30, y: 70, scale: 0.8, paint: "wood-800" },
    ],
    mid: [
      { part: "tile-floor", x: 50, y: 70, paint: "wood-800" },
      { part: "table", x: 46, y: 82, scale: 1.2 },
      { part: "candle", x: 42, y: 74, count: 3, spread: [8, 0.4] },
    ],
    subject: [
      { part: "siege-tower", x: 50, y: 60, scale: 0.9, fade: 0.45, paint: "cyan-400" },
      { part: "siege-tower", x: 70.6, y: 60, scale: 0.18, fade: 0.7, paint: "cyan-400" },
    ],
    near: [
      { part: "candle", x: 12, y: 96, scale: 1.4 },
      { part: "bones", x: 86, y: 97, scale: 1 },
      { part: "dust", x: 50, y: 96, scale: 1.4, paint: "violet-300" },
    ],
    light: [
      { part: "halo", x: 50, y: 44, scale: 2, paint: "cyan-400" },
      { part: "light-ribbon", x: 60, y: 50, scale: 0.4, paint: "cyan-400" },
      { part: "rim-light", x: 50, y: 74, scale: 1.2, paint: "violet-300" },
    ],
    particles: [
      { part: "motes", x: 50, y: 50, scale: 1.4, paint: "cyan-400" },
      { part: "sparks", x: 70, y: 56, scale: 0.8 },
    ],
  },
};

/** Call to Chaos (Training-Grounds Edition) (Training Grounds #39, Legendary): exam day on the yard's centre ground: an instructor hurls a pair of giant carved dice, and each tumbling face bursts into its own outcome, runners sprinting, shields blooming, a golem rising, cards raining; recruits scatter across the sand; lurid magenta and teal storm light. */
const CALL_TO_CHAOS: SceneRecipe = {
  family: "training-grounds",
  sky: ["violet-700", "cyan-400"],
  light: { x: 50, y: 44, r: 40, paint: "purple-400", opacity: 0.45 },
  emblem: { x: 50, y: 40, size: 6, rotate: 0 },
  extras: [
    { layer: "subject", x: 50, y: 92, scale: 0.6, pose: "pose-raise", look: { ...RECRUIT, hair: "hood", outfit: "outfit-robe", cloth: "violet-500" } },
    { layer: "subject", x: 70, y: 82, scale: 0.5, flip: -1, pose: "pose-raise", look: { ...figureLookFor([]), head: "head-golem", outfit: "outfit-stone", hair: null, cloth: "wood-600" }, fade: 0.85 },
    { layer: "mid", x: 16, y: 72, scale: 0.34, flip: -1, pose: "pose-run", look: { ...RECRUIT, cloth: "green-700" } },
    { layer: "mid", x: 86, y: 72, scale: 0.34, pose: "pose-run", look: RECRUIT },
  ],
  layers: {
    sky: [
      { part: "cloud", x: 30, y: 24, scale: 1, count: 2, spread: [16, 3], paint: "slate-800" },
      { part: "lightning", x: 76, y: 18, scale: 0.8 },
    ],
    far: palisadeAt(60, "slate-800"),
    mid: [
      { part: "sand", x: 50, y: 62 },
      { part: "rune-circle", x: 50, y: 78, scale: 1.4, paint: "cyan-400" },
    ],
    subject: [
      { part: "dice", x: 48, y: 42, scale: 2.4 },
      { part: "runner-token", x: 30, y: 80, scale: 1.4 },
      { part: "runner-token", x: 36, y: 84, scale: 1.5 },
      { part: "dome", x: 30, y: 62, scale: 0.3, paint: "cyan-400" },
    ],
    near: [
      { part: "card", x: 16, y: 96, scale: 1.2 },
      { part: "card", x: 84, y: 98, scale: 1.3 },
      { part: "dust", x: 50, y: 96, scale: 1.6, paint: "violet-300" },
    ],
    light: [
      { part: "halo", x: 48, y: 40, scale: 2, paint: "purple-400" },
      { part: "rim-light", x: 50, y: 66, scale: 1.4, paint: "cyan-400" },
    ],
    particles: [
      { part: "card", x: 50, y: 30, scale: 0.6, count: 4, spread: [36, 8] },
      { part: "sparks", x: 48, y: 40, scale: 1.2, count: 2, spread: [10, 6] },
    ],
  },
};

/** Goetzmann Game Design (Training Grounds #40, Mythic): at the canteen window after closing a designer's pencil sketches on a napkin by a coffee ring; one card unfolds up from it and behind it every card of the yard rises in a towering spiral, dummies, Felinors, golems and spells taking form; lantern light below, cyan and magenta pixel sparks above, wonder. */
const GOETZMANN_GAME_DESIGN: SceneRecipe = {
  family: "training-grounds",
  sky: ["slate-900", "violet-700"],
  light: { x: 50, y: 84, r: 34, paint: "orange-400", opacity: 0.45 },
  emblem: { x: 50, y: 66, size: 6, rotate: 0 },
  extras: [
    { layer: "subject", x: 64, y: 44, scale: 0.26, pose: "pose-spread", look: figureLookFor(["Felinor"]), fade: 0.7 },
    { layer: "subject", x: 36, y: 34, scale: 0.26, flip: -1, pose: "pose-raise", look: { ...figureLookFor([]), head: "head-golem", outfit: "outfit-stone", hair: null, cloth: "wood-600" }, fade: 0.6 },
  ],
  layers: {
    sky: [
      { part: "stars", x: 50, y: 24, scale: 1.4 },
      { part: "pixels", x: 50, y: 26, scale: 1.2, count: 2, spread: [24, 4] },
    ],
    far: [
      ...palisadeAt(60, "wood-900"),
      { part: "carved-pole", x: 84, y: 62, scale: 0.6 },
      { part: "barracks", x: 18, y: 62, scale: 0.8, paint: "wood-800" },
    ],
    mid: [
      { part: "sand", x: 50, y: 62, paint: "wood-700" },
      { part: "straw-dummy", x: 72, y: 74, scale: 0.6 },
      { part: "target", x: 28, y: 74, scale: 0.6 },
    ],
    subject: [
      { part: "card", x: 50, y: 74, scale: 1.4 },
      { part: "card", x: 42, y: 60, scale: 0.9, fade: 0.8 },
      { part: "card", x: 58, y: 52, scale: 0.8, fade: 0.75 },
      { part: "card", x: 44, y: 44, scale: 0.7, fade: 0.7 },
      { part: "card", x: 54, y: 36, scale: 0.6, fade: 0.65 },
      { part: "card", x: 48, y: 29, scale: 0.5, fade: 0.6 },
      { part: "straw-dummy", x: 34, y: 54, scale: 0.4, fade: 0.7 },
      { part: "fireball", x: 64, y: 58, scale: 0.6 },
      { part: "orb", x: 58, y: 30, scale: 0.5 },
    ],
    near: [
      { part: "table", x: 50, y: 104, scale: 3 },
      { part: "napkin", x: 50, y: 90, scale: 1.8 },
      { part: "fence-post", x: 8, y: 98, scale: 1.4 },
      { part: "lantern", x: 14.7, y: 81.5, scale: 1.4 },
    ],
    light: [
      { part: "halo", x: 14.7, y: 86, scale: 1.6, paint: "orange-400" },
      { part: "halo", x: 50, y: 50, scale: 2, paint: "cyan-400" },
      { part: "light-ribbon", x: 50, y: 50, scale: 0.6, paint: "violet-300" },
    ],
    particles: [
      { part: "pixels", x: 50, y: 44, scale: 1, count: 2, spread: [16, 10] },
      { part: "sparks", x: 50, y: 70, scale: 1 },
    ],
  },
};

/** R3393000: the Training Grounds cards' recipes, by id, the Legacy cards' aside (sceneRecipesLegacy.ts). */
export const TRAINING_GROUNDS_RECIPES: Readonly<Record<string, SceneRecipe>> = {
  "traininggrounds-010": FAN_OF_KNIVES,
  "traininggrounds-011": THWOMP,
  "traininggrounds-012": FORTIFY,
  "traininggrounds-013": EXPLOSIVE_RUNES,
  "traininggrounds-014": EXPLOSIVE_TRAP,
  "traininggrounds-015": IN_A_HURRY,
  "traininggrounds-019": EXTRA_RATIONS,
  "traininggrounds-020": RECRUITMENT_POSTERS,
  "traininggrounds-024": GOLIATH_SITUATION,
  "traininggrounds-026": PATH_OF_EXILE,
  "traininggrounds-027": HOPEFUL_JELLY_BEAN,
  "traininggrounds-031": LEVEL_UP,
  "traininggrounds-034": DINO_SIZE,
  "traininggrounds-035": CORPSE_RECYCLER,
  "traininggrounds-037": SEANCE,
  "traininggrounds-039": CALL_TO_CHAOS,
  "traininggrounds-040": GOETZMANN_GAME_DESIGN,
};
