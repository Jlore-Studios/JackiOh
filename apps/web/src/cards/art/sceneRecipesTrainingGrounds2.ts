// R3393000: the Training Grounds set's Units from #21 on and the Golem token's scene recipes, kept apart
// from sceneRecipesTrainingGrounds.ts, which spreads them into its table, as the Classic+ pass split its own. Each
// is drawn from its brief in issue #3393's table in the Training Grounds family's look (ART.md's art bible): carved
// wood, the yard's palisade and sand wherever the brief is set in it, the sun from the upper right but at night.
// A Unit is a figure-kit character.

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
/** A cat-folk recruit of the yard, in its green, empty-handed. */
const FELINOR: FigureLook = { ...figureLookFor(["Felinor"]), item: null, cloth: "green-700" };

/** Flame Tongue Totem (Training Grounds #21, Rare): a squat carved-cedar totem on the sparring row, its open mouth licking tongues of fire toward the posts either side, embers drifting; smoky dusk. */
const FLAME_TONGUE_TOTEM: SceneRecipe = {
  family: "training-grounds",
  sky: ["violet-700", "red-300"],
  light: { x: 50, y: 52, r: 32, paint: "orange-400", opacity: 0.45 },
  emblem: { x: 50, y: 70, size: 6, rotate: 0 },
  figure: { x: 50, y: 86, scale: 0.95, pose: "pose-post", look: { ...figureLookFor([]), head: "head-eyes", hair: null, outfit: "outfit-totem", item: null, skin: "red-600", cloth: "wood-600" } },
  layers: {
    sky: [{ part: "sun", x: 84, y: 40, scale: 0.6, paint: "red-300" }],
    far: palisadeAt(62, "wood-800"),
    mid: [
      { part: "yard-sand", x: 50, y: 64 },
      { part: "pell-post", x: 18, y: 86, scale: 1.3 },
      { part: "pell-post", x: 82, y: 86, scale: 1.3, flip: -1 },
      { part: "flames", x: 36, y: 62, scale: 1.1 },
      { part: "flames", x: 64, y: 62, scale: 1.1 },
      { part: "flames", x: 26, y: 66, scale: 0.7 },
      { part: "flames", x: 74, y: 66, scale: 0.7 },
    ],
    near: [{ part: "smoke", x: 10, y: 98, scale: 1.3, paint: "slate-700" }],
    light: [{ part: "halo", x: 50, y: 56, scale: 1.3, paint: "orange-400" }],
    particles: [{ part: "embers", x: 50, y: 46, scale: 2, count: 2, spread: [14, 4] }],
  },
};

/** Right-Hand Felinor (Training Grounds #22, Rare): a loyal cat-folk squire at a knight's right shoulder on the jousting rail, holding his lance steady and watching his side; soft morning light. */
const RIGHT_HAND_FELINOR: SceneRecipe = {
  family: "training-grounds",
  sky: ["sky-400", "blue-200"],
  light: { x: 70, y: 30, r: 34, paint: "white", opacity: 0.35 },
  emblem: { x: 36, y: 68, size: 6, rotate: 0 },
  figure: { x: 36, y: 86, scale: 0.75, pose: "pose-guard", look: { ...FELINOR, item: "item-lance", cloth: "blue-700" } },
  extras: [{ layer: "subject", x: 60, y: 85, scale: 0.85, flip: -1, pose: "pose-stand", look: { ...RECRUIT, hair: "helm", outfit: "outfit-plate", item: "item-shield", cloth: "slate-400" } }],
  layers: {
    sky: [
      { part: "sun", x: 84, y: 22, scale: 0.6 },
      { part: "cloud", x: 28, y: 22, scale: 0.8, paint: "white" },
    ],
    far: [
      ...palisadeAt(62, "wood-800"),
      { part: "banner", x: 12, y: 56, scale: 0.6, paint: "blue-700" },
      { part: "banner", x: 88, y: 56, scale: 0.6, flip: -1, paint: "red-600" },
    ],
    mid: [
      { part: "yard-sand", x: 50, y: 64 },
      { part: "fence", x: 50, y: 78, scale: 0.9, paint: "wood-600" },
    ],
    near: [
      { part: "grass-tuft", x: 12, y: 96, count: 2, spread: [6, 1], paint: "green-700" },
      { part: "fence-post", x: 90, y: 99, scale: 1 },
    ],
    light: [{ part: "rim-light", x: 50, y: 60, scale: 1.1, paint: "white" }],
  },
};

/** Quadrupling Felinors (Training Grounds #23, Rare): four identical cat-folk recruits mid-salute on the roll-call line by the flagpole, the last still flickering into being; crisp parade light. */
const QUADRUPLING_FELINORS: SceneRecipe = {
  family: "training-grounds",
  sky: ["blue-700", "sky-300"],
  light: { x: 50, y: 50, r: 34, paint: "white", opacity: 0.3 },
  emblem: { x: 36, y: 66, size: 5, rotate: 0 },
  figure: { x: 36, y: 84, scale: 0.55, pose: "pose-raise", look: FELINOR },
  extras: [
    { layer: "subject", x: 48, y: 84, scale: 0.55, pose: "pose-raise", look: FELINOR },
    { layer: "subject", x: 60, y: 84, scale: 0.55, pose: "pose-raise", look: FELINOR },
    { layer: "subject", x: 72, y: 84, scale: 0.55, pose: "pose-raise", look: FELINOR, fade: 0.35 },
  ],
  layers: {
    sky: [
      { part: "sun", x: 84, y: 22, scale: 0.6 },
      { part: "cloud", x: 40, y: 24, scale: 0.7, paint: "white" },
    ],
    far: [...palisadeAt(62, "wood-800"), { part: "barracks", x: 50, y: 64, scale: 0.9, paint: "wood-700" }],
    mid: [
      { part: "yard-sand", x: 50, y: 66 },
      { part: "banner", x: 18, y: 84, scale: 1.4, paint: "green-700" },
    ],
    subject: [{ part: "light-ribbon", x: 54, y: 84, scale: 0.4, paint: "white" }],
    near: [{ part: "paw-prints", x: 82, y: 96, scale: 1 }],
    particles: [{ part: "sparks", x: 72, y: 66, scale: 0.9, paint: "white" }],
  },
};

/** Trigor (Training Grounds #25, Rare): three recruits step into a chalk tribute circle behind the barracks and fuse into one bigger grinning fighter, a fourth outline looming; eerie violet dusk. */
const TRIGOR: SceneRecipe = {
  family: "training-grounds",
  sky: ["slate-900", "violet-500"],
  light: { x: 50, y: 56, r: 34, paint: "violet-300", opacity: 0.45 },
  emblem: { x: 50, y: 62, size: 6, rotate: 0 },
  figure: { x: 50, y: 85, scale: 0.85, pose: "pose-flex", look: { ...RECRUIT, outfit: "outfit-muscle", hair: "hair-spiky", cloth: "violet-700" } },
  extras: [
    { layer: "mid", x: 50, y: 80, scale: 1.25, pose: "pose-spread", look: { ...RECRUIT, cloth: "violet-500" }, fade: 0.2 },
    { layer: "subject", x: 30, y: 88, scale: 0.45, pose: "pose-run", look: { ...RECRUIT, cloth: "red-900" }, fade: 0.6 },
    { layer: "subject", x: 70, y: 88, scale: 0.45, flip: -1, pose: "pose-run", look: { ...RECRUIT, cloth: "green-700" }, fade: 0.6 },
    { layer: "subject", x: 62, y: 90, scale: 0.4, flip: -1, pose: "pose-stand", look: { ...RECRUIT, hair: "hair-long", cloth: "blue-800" }, fade: 0.6 },
  ],
  layers: {
    sky: [{ part: "moon", x: 82, y: 22, scale: 0.5 }],
    far: [{ part: "barracks", x: 50, y: 62, scale: 1.6, paint: "wood-800" }],
    mid: [
      { part: "yard-sand", x: 50, y: 64, paint: "wood-700" },
      { part: "rune-circle", x: 50, y: 84, scale: 1.5, paint: "white" },
    ],
    near: [{ part: "crate", x: 88, y: 99, scale: 1.1 }],
    light: [{ part: "halo", x: 50, y: 58, scale: 1.6, paint: "violet-300" }],
    particles: [{ part: "motes", x: 50, y: 60, scale: 1.3, paint: "violet-300" }],
  },
};

/** Occidental (Training Grounds #28, Epic): a lean drifter in a long dust coat and wide hat walks west out of the yard at sunset, dragging the whole right-hand plank lane behind him like a rug, dummies toppling off it; tumbleweeds before him, a red desert sky. */
const OCCIDENTAL: SceneRecipe = {
  family: "training-grounds",
  sky: ["red-900", "orange-400"],
  light: { x: 80, y: 46, r: 38, paint: "orange-400", opacity: 0.45 },
  emblem: { x: 60, y: 63, size: 6, rotate: 0 },
  figure: { x: 60, y: 84, scale: 0.8, pose: "pose-stand", look: { ...RECRUIT, hair: "hat-brim", outfit: "outfit-coat", cloth: "wood-600" } },
  layers: {
    sky: [
      { part: "sun", x: 84, y: 44, scale: 1, paint: "red-300" },
      { part: "cloud", x: 30, y: 24, scale: 0.9, paint: "red-300" },
    ],
    far: [...palisadeAt(62, "wood-900"), { part: "hills", x: 50, y: 62, paint: "red-900" }],
    mid: [
      { part: "lanes", x: 30, y: 64, paint: "wood-700" },
      { part: "straw-dummy", x: 16, y: 70, scale: 0.5 },
    ],
    subject: [
      { part: "plank-rug", x: 60, y: 82, scale: 1.2 },
      { part: "straw-dummy", x: 30, y: 78, scale: 0.55, flip: -1 },
      { part: "straw-dummy", x: 44, y: 79, scale: 0.5, fade: 0.8 },
    ],
    near: [
      { part: "tumbleweed", x: 80, y: 94, scale: 1.4 },
      { part: "tumbleweed", x: 18, y: 96, scale: 1 },
      { part: "dust", x: 40, y: 92, scale: 1.4, paint: "brass-500" },
    ],
    light: [{ part: "rim-light", x: 60, y: 60, scale: 1.2, paint: "orange-400" }],
    particles: [{ part: "dust", x: 28, y: 80, scale: 1.1, paint: "brass-500" }],
  },
};

/** Easterner (Training Grounds #29, Epic): a calm martial artist in a blue tunic steps through a carved wooden moon gate at dawn, sweeping the leftmost lane clean with one turn of his staff, posts scattering like leaves; plum blossom behind, pale jade light. */
const EASTERNER: SceneRecipe = {
  family: "training-grounds",
  sky: ["jade-400", "green-200"],
  light: { x: 50, y: 58, r: 34, paint: "white", opacity: 0.35 },
  emblem: { x: 50, y: 64, size: 6, rotate: 0 },
  figure: { x: 50, y: 86, scale: 0.8, pose: "pose-guard", look: { ...figureLookFor(["CN"]), outfit: "outfit-tunic", item: "item-staff", cloth: "blue-700" } },
  layers: {
    sky: [
      { part: "sun", x: 84, y: 30, scale: 0.6, paint: "white" },
      { part: "cloud", x: 26, y: 22, scale: 0.7, paint: "white" },
    ],
    far: [
      { part: "hills", x: 50, y: 58, paint: "green-700" },
      { part: "plum-branch", x: 4, y: 36, scale: 1 },
      { part: "plum-branch", x: 96, y: 40, scale: 0.9, flip: -1 },
    ],
    mid: [
      { part: "yard-sand", x: 50, y: 70 },
      { part: "moon-gate", x: 50, y: 84, scale: 1.5 },
      { part: "fence-post", x: 16, y: 80, scale: 0.7 },
      { part: "fence-post", x: 24, y: 72, scale: 0.5, flip: -1, fade: 0.8 },
    ],
    subject: [{ part: "light-ribbon", x: 32, y: 82, scale: 0.35, paint: "white" }],
    near: [
      { part: "lanes", x: 50, y: 90, paint: "wood-700" },
      { part: "grass-tuft", x: 86, y: 96, count: 2, spread: [5, 1], paint: "green-700" },
    ],
    light: [{ part: "halo", x: 50, y: 56, scale: 1.6, paint: "white" }],
    particles: [
      { part: "petals", x: 26, y: 62, scale: 1.1, paint: "red-300" },
      { part: "splinters", x: 18, y: 74, scale: 0.8 },
    ],
  },
};

/** Floodgates (Training Grounds #30, Epic): a living sluice gate of iron-banded timber stands in the yard's irrigation channel, straining to hold back a towering reservoir; before it its plates thicken as recruits arrive, cracks spurting jets; dark teal storm light. */
const FLOODGATES: SceneRecipe = {
  family: "training-grounds",
  sky: ["slate-900", "blue-800"],
  light: { x: 70, y: 30, r: 36, paint: "cyan-400", opacity: 0.3 },
  emblem: { x: 50, y: 66, size: 6, rotate: 0 },
  figure: { x: 50, y: 86, scale: 0.95, pose: "pose-post", look: { ...figureLookFor([]), head: "head-eyes", hair: null, outfit: "outfit-sluice", item: null, skin: "red-300", cloth: "wood-700" } },
  extras: [
    { layer: "mid", x: 20, y: 80, scale: 0.42, pose: "pose-run", look: { ...RECRUIT, cloth: "red-900" } },
    { layer: "mid", x: 80, y: 80, scale: 0.4, flip: -1, pose: "pose-run", look: { ...RECRUIT, cloth: "green-700" } },
  ],
  layers: {
    sky: [
      { part: "lightning", x: 78, y: 18, scale: 0.8 },
      { part: "cloud", x: 30, y: 22, scale: 1, count: 2, spread: [20, 4], paint: "slate-700" },
    ],
    far: [
      { part: "water", x: 50, y: 44, paint: "blue-900" },
      { part: "cascade", x: 50, y: 44, scale: 0.6 },
    ],
    mid: [
      { part: "yard-sand", x: 50, y: 70, paint: "wood-700" },
      { part: "palisade", x: 14, y: 76, scale: 0.8, paint: "wood-800" },
      { part: "palisade", x: 86, y: 76, scale: 0.8, flip: -1, paint: "wood-800" },
    ],
    near: [
      { part: "water", x: 50, y: 90, paint: "blue-800" },
      { part: "sandbags", x: 12, y: 98, scale: 1.2 },
    ],
    light: [{ part: "rim-light", x: 50, y: 60, scale: 1.4, paint: "cyan-400" }],
    particles: [{ part: "rain", x: 50, y: 46, scale: 1.4, paint: "blue-200" }],
  },
};

/** Selfless Protector (Training Grounds #32, Epic): a young guard falls to one knee on the battle-drill field, arms spread, as shimmering shields bloom over the comrades behind him; a broken spear before him, the barracks beyond, soft rose light of sacrifice. */
const SELFLESS_PROTECTOR: SceneRecipe = {
  family: "training-grounds",
  sky: ["violet-500", "red-300"],
  light: { x: 50, y: 56, r: 36, paint: "red-300", opacity: 0.4 },
  emblem: { x: 50, y: 70, size: 6, rotate: 0 },
  figure: { x: 50, y: 88, scale: 0.85, pose: "pose-kneel", look: { ...RECRUIT, outfit: "outfit-tabard", cloth: "blue-800" } },
  extras: [
    { layer: "mid", x: 28, y: 76, scale: 0.42, pose: "pose-guard", look: { ...RECRUIT, cloth: "red-900", item: "item-sword" } },
    { layer: "mid", x: 72, y: 76, scale: 0.42, flip: -1, pose: "pose-guard", look: { ...RECRUIT, hair: "hair-long", cloth: "green-700", item: "item-spear" } },
  ],
  layers: {
    sky: [
      { part: "sun", x: 84, y: 30, scale: 0.6, paint: "red-300" },
      { part: "cloud", x: 30, y: 22, scale: 0.8, paint: "red-300" },
    ],
    far: [{ part: "barracks", x: 50, y: 60, scale: 1.4, paint: "wood-700" }, ...palisadeAt(60, "wood-800")],
    mid: [
      { part: "yard-sand", x: 50, y: 64 },
      { part: "shield-dome", x: 28, y: 77, scale: 2 },
      { part: "shield-dome", x: 72, y: 77, scale: 2 },
    ],
    near: [
      { part: "broken-spear", x: 84, y: 96, scale: 1.2 },
      { part: "pebbles", x: 18, y: 96 },
    ],
    light: [
      { part: "halo", x: 50, y: 62, scale: 1.5, paint: "red-300" },
      { part: "rim-light", x: 50, y: 66, scale: 1.1, paint: "white" },
    ],
    particles: [{ part: "motes", x: 50, y: 60, scale: 1.4, paint: "cyan-400" }],
  },
};

/** Guerilla Felinor (Training Grounds #33, Epic): a cat-folk commando in leafy camouflage clings in the obstacle course's cargo net, flicking darts at enemies scattering below; tall grass before it, climbing walls behind, dappled jungle-green shade. */
const GUERILLA_FELINOR: SceneRecipe = {
  family: "training-grounds",
  sky: ["green-900", "green-500"],
  light: { x: 70, y: 30, r: 34, paint: "lime-200", opacity: 0.3 },
  emblem: { x: 50, y: 58, size: 6, rotate: 0 },
  figure: { x: 50, y: 74, scale: 0.6, pose: "pose-spread", look: { ...FELINOR, outfit: "outfit-leaves", item: "item-dagger", cloth: "green-700" } },
  extras: [
    { layer: "subject", x: 30, y: 90, scale: 0.36, flip: -1, pose: "pose-run", look: { ...RECRUIT, hair: "helm", cloth: "red-900" } },
    { layer: "subject", x: 72, y: 91, scale: 0.36, pose: "pose-run", look: { ...RECRUIT, hair: "helm", cloth: "red-900" } },
  ],
  layers: {
    sky: [{ part: "sun", x: 84, y: 20, scale: 0.5, paint: "white" }],
    far: [
      ...palisadeAt(60, "wood-800", 1.2),
      { part: "wood-ladder", x: 14, y: 62, scale: 0.8 },
      { part: "tree", x: 90, y: 60, scale: 1.1 },
    ],
    mid: [
      { part: "yard-sand", x: 50, y: 66, paint: "wood-600" },
      { part: "cargo-net", x: 50, y: 86, scale: 1.8 },
    ],
    subject: [
      { part: "knife", x: 40, y: 80, scale: 0.6, flip: -1 },
      { part: "knife", x: 62, y: 82, scale: 0.6 },
    ],
    near: [
      { part: "grass-tuft", x: 10, y: 98, scale: 1.8, count: 3, spread: [8, 1], paint: "green-700" },
      { part: "fern", x: 90, y: 98, scale: 1.6 },
    ],
    light: [{ part: "god-rays", x: 80, y: 10, scale: 1, paint: "lime-200" }],
    particles: [{ part: "petals", x: 50, y: 40, scale: 1.2, paint: "green-400" }],
  },
};

/** Training Dummy (Training Grounds #36, Epic): a battered straw dummy on its post alone on the hilltop practice green, patched and stuck with arrows, a glowing lesson card drifting from its chest; fences in the middle ground, dawn fog over the yard below, stoic. */
const TRAINING_DUMMY: SceneRecipe = {
  family: "training-grounds",
  sky: ["blue-700", "red-300"],
  light: { x: 58, y: 48, r: 32, paint: "cyan-400", opacity: 0.3 },
  emblem: { x: 48, y: 66, size: 6, rotate: 0 },
  figure: { x: 48, y: 86, scale: 0.95, pose: "pose-post", look: { ...figureLookFor([]), head: "head-sack", hair: null, outfit: "outfit-straw", item: null, skin: "brass-600", cloth: "brass-500" } },
  layers: {
    sky: [
      { part: "sun", x: 84, y: 34, scale: 0.6, paint: "red-300" },
      { part: "cloud", x: 26, y: 24, scale: 0.8, paint: "red-300" },
    ],
    far: [
      ...palisadeAt(66, "wood-800", 0.5),
      { part: "mist", x: 50, y: 64, scale: 1.4, paint: "white" },
      { part: "hills", x: 50, y: 70, paint: "green-700" },
    ],
    mid: [
      { part: "fence", x: 22, y: 78, scale: 0.7, paint: "wood-700" },
      { part: "fence", x: 80, y: 78, scale: 0.7, paint: "wood-700" },
    ],
    subject: [
      { part: "arrow", x: 40, y: 62, scale: 0.6, flip: -1 },
      { part: "arrow", x: 57, y: 71, scale: 0.55 },
      { part: "card", x: 62, y: 60, scale: 0.8 },
    ],
    near: [
      { part: "grass-tuft", x: 14, y: 96, count: 3, spread: [8, 1], paint: "green-700" },
      { part: "arrow", x: 86, y: 95, scale: 0.6 },
    ],
    light: [{ part: "halo", x: 62, y: 56, scale: 1, paint: "cyan-400" }],
    particles: [{ part: "motes", x: 60, y: 50, scale: 1, paint: "cyan-400" }],
  },
};

/** Tutorial Master (Training Grounds #38, Legendary): a towering white-bearded instructor in a patched quilted gambeson raps a long pointer on a chalkboard of arrows and stick figures in the lecture yard, explaining it again, slowly; yawning recruits on benches, chalk dust swirling in a beam of morning light, a fresh card sliding from his sleeve. */
const TUTORIAL_MASTER: SceneRecipe = {
  family: "training-grounds",
  sky: ["sky-400", "blue-200"],
  light: { x: 62, y: 40, r: 36, paint: "white", opacity: 0.35 },
  emblem: { x: 60, y: 66, size: 7, rotate: 0 },
  figure: { x: 60, y: 90, scale: 0.95, flip: -1, pose: "pose-guard", look: { ...RECRUIT, hair: "beard", outfit: "outfit-patched", item: "item-pointer", cloth: "wood-600" } },
  extras: [
    { layer: "near", x: 14, y: 100, scale: 0.42, pose: "pose-sit", look: { ...RECRUIT, cloth: "red-900" } },
    { layer: "near", x: 86, y: 100, scale: 0.42, flip: -1, pose: "pose-sit", look: { ...RECRUIT, hair: "hair-long", cloth: "green-700" } },
  ],
  layers: {
    sky: [
      { part: "sun", x: 86, y: 18, scale: 0.6 },
      { part: "cloud", x: 26, y: 22, scale: 0.8, paint: "white" },
    ],
    far: [
      ...palisadeAt(60, "wood-800"),
      { part: "barracks", x: 74, y: 62, scale: 0.9, paint: "wood-700" },
    ],
    mid: [
      { part: "yard-sand", x: 50, y: 64 },
      { part: "weapon-rack", x: 86, y: 74, scale: 0.7 },
    ],
    subject: [
      { part: "slate-board", x: 31, y: 72, scale: 1.9 },
      { part: "card", x: 69, y: 68, scale: 0.5 },
    ],
    near: [
      { part: "plank-bench", x: 14, y: 100, scale: 1.4 },
      { part: "plank-bench", x: 86, y: 100, scale: 1.4 },
    ],
    light: [
      { part: "light-shaft", x: 80, y: 10, scale: 1.2, paint: "white" },
      { part: "rim-light", x: 60, y: 60, scale: 1.1, paint: "white" },
    ],
    particles: [
      { part: "dust", x: 34, y: 46, scale: 1, paint: "white" },
      { part: "motes", x: 50, y: 48, scale: 1.3, paint: "white" },
    ],
  },
};

/** Training-Grounds Golem (Training Grounds #39 token): a giant golem built of lashed training posts, dummies and shields, a practice sword in one fist, stomping through the tilting lanes; storm-grey, unstoppable. */
const TRAINING_GROUNDS_GOLEM: SceneRecipe = {
  family: "training-grounds",
  sky: ["slate-800", "slate-400"],
  light: { x: 50, y: 46, r: 34, paint: "slate-100", opacity: 0.3 },
  emblem: { x: 50, y: 58, size: 6, rotate: 0 },
  figure: { x: 50, y: 88, scale: 1.05, pose: "pose-run", look: { ...figureLookFor(["Token"]), head: "head-sack", hair: null, outfit: "outfit-posts", item: "item-sword", skin: "brass-600", cloth: "wood-600" } },
  layers: {
    sky: [{ part: "cloud", x: 30, y: 22, scale: 1, count: 2, spread: [24, 4], paint: "slate-700" }],
    far: palisadeAt(60, "wood-800"),
    mid: [
      { part: "lanes", x: 50, y: 62, paint: "wood-700" },
      { part: "fence", x: 18, y: 76, scale: 0.6, paint: "wood-600" },
    ],
    near: [{ part: "splinters", x: 30, y: 94, scale: 1.2 }],
    particles: [{ part: "dust", x: 50, y: 88, scale: 1.4, paint: "stone-500" }],
  },
};

/** R3393000: Training Grounds #21 to #38's and the Golem token's recipes, by id. */
export const TRAINING_GROUNDS_RECIPES_2: Readonly<Record<string, SceneRecipe>> = {
  "traininggrounds-021": FLAME_TONGUE_TOTEM,
  "traininggrounds-022": RIGHT_HAND_FELINOR,
  "traininggrounds-023": QUADRUPLING_FELINORS,
  "traininggrounds-025": TRIGOR,
  "traininggrounds-028": OCCIDENTAL,
  "traininggrounds-029": EASTERNER,
  "traininggrounds-030": FLOODGATES,
  "traininggrounds-032": SELFLESS_PROTECTOR,
  "traininggrounds-033": GUERILLA_FELINOR,
  "traininggrounds-036": TRAINING_DUMMY,
  "traininggrounds-038": TUTORIAL_MASTER,
  "traininggrounds-039-1": TRAINING_GROUNDS_GOLEM,
};
