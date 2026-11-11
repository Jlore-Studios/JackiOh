// R3394000: the alt scene recipes of Stinkyscape's cards 1 to 90 (#3394), cards 30 to 56 (stinkyscapeAltRecipes1.ts
// says what each is held to). Presentation only (CLAUDE.md rule 7): a recipe is read by the card's id alone.

import { figureLookFor } from "./figureKit.ts";
import type { SceneRecipe } from "./scene.ts";
import { normalLookOf } from "./stinkyscapeAltRecipes1.ts";

/** Drone Core IV (Stinkyscape #30, Rare), Alt art: the three interceptors returning at dusk to a floating buoy above a violet ocean world, seen from below, their lights reflected on the water. */
const DRONE_CORE_IV_ALT: SceneRecipe = {
  family: "stinkyscape",
  sky: ["violet-700", "red-300"],
  light: { x: 50, y: 66, r: 36, paint: "red-300", opacity: 0.4 },
  emblem: { x: 18, y: 30, size: 5, rotate: 0 },
  layers: {
    sky: [
      { part: "stars", x: 50, y: 18, scale: 1.2 },
      { part: "moon", x: 82, y: 22, scale: 0.6 },
    ],
    far: [{ part: "sea", x: 50, y: 70, paint: "violet-500" }],
    mid: [{ part: "buoy", x: 50, y: 62, scale: 1.3 }],
    subject: [
      { part: "eye-drone", x: 50, y: 40, scale: 1.6 },
      { part: "eye-drone", x: 34, y: 30, scale: 1.2, flip: -1 },
      { part: "eye-drone", x: 66, y: 32, scale: 1.2 },
    ],
    near: [{ part: "ripples", x: 50, y: 84, scale: 1.4, paint: "cyan-400" }],
    light: [
      { part: "halo", x: 50, y: 60, scale: 1.2, paint: "cyan-400" },
      { part: "halo", x: 50, y: 40, scale: 0.8, paint: "red-300" },
    ],
    particles: [{ part: "motes", x: 50, y: 50, scale: 1.2, paint: "violet-300" }],
  },
};

/** Drone Core VII (Stinkyscape #33, Epic), Alt art: the corvette and its two bodyguards parked in a green-lit station wash bay, cleaner drones scrubbing their hulls; overhead view. The kit has no hose or brush, so the cleaner drones' spray and bubbles stand for the scrubbing. */
const DRONE_CORE_VII_ALT: SceneRecipe = {
  family: "stinkyscape",
  sky: ["jade-400", "green-200"],
  light: { x: 50, y: 50, r: 42, paint: "lime-200", opacity: 0.35 },
  emblem: { x: 82, y: 76, size: 5, rotate: 0 },
  layers: {
    sky: [{ part: "ceiling-lights", x: 50, y: 4, scale: 1 }],
    far: [{ part: "deck-floor", x: 50, y: 22, paint: "green-700" }],
    mid: [
      { part: "launch-pad", x: 46, y: 70, scale: 1.4 },
      { part: "bucket", x: 84, y: 34, scale: 1 },
    ],
    subject: [
      { part: "drone-corvette", x: 46, y: 62, scale: 1.8, paint: "slate-100" },
      { part: "eye-drone", x: 26, y: 40, scale: 1.4 },
      { part: "eye-drone", x: 70, y: 38, scale: 1.4, flip: -1 },
      { part: "drone", x: 38, y: 46, scale: 0.7 },
      { part: "drone", x: 60, y: 50, scale: 0.7, flip: -1 },
      { part: "drone", x: 50, y: 76, scale: 0.6 },
    ],
    near: [
      { part: "pipes", x: 12, y: 92, scale: 1.2 },
      { part: "cable", x: 88, y: 90, scale: 1.1 },
    ],
    light: [{ part: "light-shaft", x: 50, y: 6, scale: 1.2, paint: "lime-200" }],
    particles: [
      { part: "bubbles", x: 46, y: 56, scale: 1.4, paint: "green-200" },
      { part: "spray", x: 40, y: 50, scale: 0.8 },
      { part: "drops", x: 60, y: 56, scale: 0.6, paint: "sky-300" },
    ],
  },
};

/** Drone Core VIII (Stinkyscape #34, Legendary), Alt art: after the shift: the silent assembly line at rest, one last drone dangling from a robotic arm, dust settling in red emergency light; low, lonely angle. */
const DRONE_CORE_VIII_ALT: SceneRecipe = {
  family: "stinkyscape",
  sky: ["red-900", "red-600"],
  light: { x: 70, y: 30, r: 34, paint: "red-500", opacity: 0.4 },
  emblem: { x: 82, y: 70, size: 5, rotate: 0 },
  layers: {
    sky: [{ part: "ceiling-lights", x: 50, y: 4, scale: 1 }],
    far: [
      { part: "hangar-wall", x: 50, y: 66, paint: "red-900" },
      { part: "drone-rack", x: 18, y: 64, scale: 1 },
    ],
    mid: [
      { part: "conveyor", x: 50, y: 80, scale: 1.4 },
      { part: "eye-drone", x: 26, y: 74, scale: 0.7, fade: 0.7 },
      { part: "eye-drone", x: 72, y: 76, scale: 0.6, count: 2, spread: [10, 0], fade: 0.7 },
    ],
    subject: [
      { part: "welding-arm", x: 38, y: 76, scale: 1.4 },
      { part: "eye-drone", x: 58, y: 54, scale: 1.3 },
    ],
    near: [
      { part: "welding-arm", x: 90, y: 100, scale: 1.6, flip: -1 },
      { part: "crate", x: 10, y: 96, scale: 1.3 },
    ],
    light: [
      { part: "alarm-light", x: 72, y: 26, scale: 1.2 },
      { part: "halo", x: 58, y: 54, scale: 1, paint: "red-400" },
    ],
    particles: [
      { part: "dust", x: 50, y: 50, scale: 1.4, paint: "red-300" },
      { part: "motes", x: 46, y: 40, scale: 1.2, paint: "red-300" },
    ],
  },
};

/** Drone Core IX (Stinkyscape #35, Legendary), Alt art: the three corvettes lined up on a snowy launch field at a frozen pole, crews in parkas loading them; pale blue overcast, wide vista. */
const DRONE_CORE_IX_ALT: SceneRecipe = {
  family: "stinkyscape",
  sky: ["slate-400", "blue-200"],
  light: { x: 50, y: 30, r: 40, paint: "white", opacity: 0.3 },
  emblem: { x: 16, y: 30, size: 5, rotate: 0 },
  extras: [
    { layer: "subject", x: 40, y: 74, scale: 0.18, pose: "pose-lift", look: { ...figureLookFor(["Human"]), hair: "beanie", outfit: "outfit-parka", item: null, cloth: "red-500" } },
    { layer: "subject", x: 62, y: 74, scale: 0.18, flip: -1, pose: "pose-hold", look: { ...figureLookFor(["Human"]), hair: "beanie", outfit: "outfit-parka", item: null, cloth: "orange-400" } },
  ],
  layers: {
    sky: [{ part: "cloud", x: 30, y: 20, scale: 1.4, count: 3, spread: [30, 4], paint: "slate-100" }],
    far: [{ part: "mountains", x: 50, y: 58, paint: "slate-100" }],
    mid: [
      { part: "snowfield", x: 50, y: 64 },
      { part: "launch-pad", x: 50, y: 74, scale: 1.4 },
    ],
    subject: [
      { part: "drone-corvette", x: 50, y: 62, scale: 1.1 },
      { part: "drone-corvette", x: 30, y: 58, scale: 0.8 },
      { part: "drone-corvette", x: 70, y: 58, scale: 0.8, flip: -1 },
      { part: "crate", x: 46, y: 76, scale: 0.5, count: 2, spread: [3, 1] },
    ],
    near: [
      { part: "snowfield", x: 50, y: 90 },
      { part: "crate", x: 86, y: 94, scale: 1.2 },
    ],
    light: [{ part: "rim-light", x: 50, y: 58, scale: 1.2, paint: "white" }],
    particles: [{ part: "snow", x: 50, y: 40, scale: 1.4, paint: "white" }],
  },
};

/** The Narcor Project (Stinkyscape #39, Legendary), Alt art: outside the project: the cracked asteroid seen from far off, red-violet light leaking from its fissures into a calm blue-black belt; wide, quiet vista. */
const THE_NARCOR_PROJECT_ALT: SceneRecipe = {
  family: "stinkyscape",
  sky: ["black", "blue-900"],
  light: { x: 50, y: 52, r: 30, paint: "purple-400", opacity: 0.4 },
  emblem: { x: 82, y: 76, size: 5, rotate: 0 },
  layers: {
    sky: [
      { part: "stars", x: 50, y: 24, scale: 1.8 },
      { part: "galaxy", x: 20, y: 26, scale: 0.7 },
    ],
    far: [
      { part: "nebula", x: 76, y: 30, scale: 1.2, paint: "blue-800" },
      { part: "asteroid", x: 18, y: 64, scale: 0.5, count: 4, spread: [14, 8] },
    ],
    mid: [
      { part: "asteroid", x: 80, y: 66, scale: 0.8, count: 3, spread: [10, 6] },
      { part: "asteroid", x: 24, y: 40, scale: 0.6 },
      { part: "asteroid", x: 30, y: 82, scale: 0.5, count: 2, spread: [8, 4] },
    ],
    subject: [
      { part: "asteroid", x: 50, y: 54, scale: 3, paint: "slate-700" },
      { part: "crystal", x: 46, y: 50, scale: 0.9, count: 3, spread: [10, 6], paint: "purple-400" },
      { part: "crystal", x: 56, y: 58, scale: 0.7, paint: "red-400" },
    ],
    near: [{ part: "asteroid", x: 88, y: 94, scale: 2 }],
    light: [
      { part: "halo", x: 50, y: 52, scale: 1.6, paint: "violet-500" },
      { part: "halo", x: 54, y: 56, scale: 0.9, paint: "red-400" },
    ],
    particles: [
      { part: "motes", x: 50, y: 50, scale: 1.4, paint: "violet-300" },
      { part: "sparks", x: 52, y: 52, scale: 0.8, paint: "red-300" },
    ],
  },
};

/** Drone Sympathizer (Stinkyscape #41, Common), Alt art: the helper drone at the kitchen window of a domed colony home in a snowstorm, cheerfully holding a drone core up to the glass; cool white-blue light. */
const DRONE_SYMPATHIZER_ALT: SceneRecipe = {
  family: "stinkyscape",
  sky: ["slate-100", "blue-200"],
  light: { x: 50, y: 48, r: 30, paint: "white", opacity: 0.35 },
  emblem: { x: 82, y: 76, size: 5, rotate: 0 },
  layers: {
    far: [
      { part: "snowfield", x: 50, y: 66 },
      { part: "colony-dome", x: 22, y: 64, scale: 0.6 },
    ],
    subject: [
      { part: "helper-drone", x: 46, y: 52, scale: 2 },
      { part: "drone-core", x: 57, y: 50, scale: 0.6 },
    ],
    near: [
      { part: "viewport", x: 50, y: 52, scale: 1.7 },
      { part: "counter", x: 50, y: 98, scale: 1.6, paint: "slate-400" },
      { part: "cup", x: 26, y: 84, scale: 1 },
    ],
    particles: [{ part: "snow", x: 50, y: 40, scale: 1.4, paint: "white" }],
  },
};

/** Core Researcher (Stinkyscape #42, Rare), Alt art: the small explosion itself: a wide shot of her lab module puffing orange smoke into a peaceful green nebula, her goggles spinning away. The kit has no loose goggles, so she tumbles out bare-headed and a spark spins where they fly. */
const CORE_RESEARCHER_ALT: SceneRecipe = {
  family: "stinkyscape",
  sky: ["green-900", "jade-400"],
  light: { x: 40, y: 48, r: 34, paint: "orange-400", opacity: 0.35 },
  emblem: { x: 18, y: 76, size: 5, rotate: 0 },
  figure: { x: 62, y: 52, scale: 0.3, pose: "pose-spread", look: { ...normalLookOf("stinkyscape-042"), hair: null } },
  layers: {
    sky: [
      { part: "stars", x: 50, y: 24, scale: 1.4 },
      { part: "nebula", x: 70, y: 30, scale: 1.4, paint: "green-400" },
    ],
    far: [{ part: "nebula", x: 24, y: 70, scale: 1.1, paint: "green-200" }],
    mid: [{ part: "debris", x: 76, y: 66, scale: 0.8 }],
    subject: [
      { part: "ship-module", x: 42, y: 48, scale: 1.7 },
      { part: "smoke-billow", x: 32, y: 40, scale: 1.2, paint: "orange-400" },
    ],
    near: [{ part: "debris", x: 14, y: 88, scale: 1.2 }],
    light: [{ part: "halo", x: 40, y: 46, scale: 1.2, paint: "orange-400" }],
    particles: [
      { part: "sparks", x: 70, y: 40, scale: 0.7, paint: "white" },
      { part: "embers", x: 40, y: 44, scale: 1, paint: "orange-400" },
    ],
  },
};

/** Healing Capsule (Stinkyscape #43, Rare), Alt art: the capsule clamped to a battered hull mid-fight, washing green light over dented plates as they pop back out; wide view against a crimson nebula. */
const HEALING_CAPSULE_ALT: SceneRecipe = {
  family: "stinkyscape",
  sky: ["red-900", "red-500"],
  light: { x: 52, y: 50, r: 30, paint: "green-400", opacity: 0.35 },
  emblem: { x: 18, y: 76, size: 5, rotate: 0 },
  layers: {
    sky: [
      { part: "nebula", x: 30, y: 28, scale: 1.4, paint: "red-400" },
      { part: "stars", x: 50, y: 20, scale: 1.2 },
    ],
    far: [
      { part: "fighter", x: 80, y: 26, scale: 0.6, flip: -1, paint: "red-600" },
      { part: "laser", x: 76, y: 30, scale: 0.5, flip: -1, paint: "red-500" },
    ],
    mid: [{ part: "starship", x: 50, y: 62, scale: 1.6, paint: "slate-700" }],
    subject: [
      { part: "pod-capsule", x: 52, y: 50, scale: 1.4 },
      { part: "hull-plate", x: 36, y: 58, scale: 0.7 },
      { part: "hull-plate", x: 66, y: 60, scale: 0.6 },
    ],
    near: [{ part: "debris", x: 86, y: 90, scale: 1.2 }],
    light: [
      { part: "heal-glow", x: 52, y: 54, scale: 1.8 },
      { part: "halo", x: 52, y: 50, scale: 1.2, paint: "green-400" },
    ],
    particles: [{ part: "sparks", x: 40, y: 60, scale: 0.8, paint: "green-200" }],
  },
};

/** Great Paperclip Theory (Stinkyscape #44, Mythic), Alt art: far in the future: a whole planet tiled with nested drone cores, seen from orbit at sunset, a rust-orange terminator line crossing it; vast and still. */
const GREAT_PAPERCLIP_THEORY_ALT: SceneRecipe = {
  family: "stinkyscape",
  sky: ["violet-700", "orange-400"],
  light: { x: 78, y: 40, r: 36, paint: "orange-400", opacity: 0.45 },
  emblem: { x: 16, y: 28, size: 5, rotate: 0 },
  layers: {
    sky: [
      { part: "stars", x: 40, y: 18, scale: 1.6 },
      { part: "sun", x: 84, y: 38, scale: 0.7 },
    ],
    far: [{ part: "planet-limb", x: 50, y: 40, paint: "slate-700" }],
    mid: [
      { part: "drone-core", x: 22, y: 70, scale: 0.5, count: 5, spread: [16, 12] },
      { part: "drone-core", x: 80, y: 74, scale: 0.5, count: 4, spread: [14, 12] },
    ],
    subject: [
      { part: "drone-core", x: 50, y: 60, scale: 1.4 },
      { part: "drone-core", x: 50, y: 60, scale: 0.6, count: 8, spread: [18, 16] },
      { part: "drone-core", x: 50, y: 60, scale: 0.4 },
    ],
    near: [{ part: "drone-core", x: 50, y: 92, scale: 0.8, count: 5, spread: [40, 4] }],
    light: [
      { part: "rim-light", x: 50, y: 44, scale: 2.2, paint: "orange-400" },
      { part: "halo", x: 84, y: 38, scale: 1.2, paint: "red-300" },
    ],
    particles: [{ part: "motes", x: 50, y: 30, scale: 1.4, paint: "orange-400" }],
  },
};

/** Drone Commander (Stinkyscape #45, Epic), Alt art: a close-up portrait of the commander eating alone at a mess-hall table, her nameless drones hovering in a neat row behind her chair; warm brown light. */
const DRONE_COMMANDER_ALT: SceneRecipe = {
  family: "stinkyscape",
  sky: ["wood-700", "orange-400"],
  light: { x: 44, y: 50, r: 34, paint: "orange-400", opacity: 0.35 },
  emblem: { x: 84, y: 30, size: 5, rotate: 0 },
  figure: { x: 42, y: 110, scale: 1.5, pose: "pose-sit", look: { ...normalLookOf("stinkyscape-045"), item: "item-spoon" } },
  layers: {
    sky: [{ part: "ceiling-lights", x: 50, y: 4, scale: 1 }],
    far: [{ part: "room-wall", x: 50, y: 70, paint: "wood-700" }],
    mid: [
      { part: "eye-drone", x: 20, y: 36, scale: 1 },
      { part: "eye-drone", x: 34, y: 34, scale: 1 },
      { part: "eye-drone", x: 48, y: 34, scale: 1 },
      { part: "eye-drone", x: 62, y: 34, scale: 1 },
      { part: "eye-drone", x: 76, y: 36, scale: 1 },
      { part: "chair", x: 30, y: 92, scale: 1.6 },
    ],
    near: [
      { part: "wide-table", x: 52, y: 106, scale: 2.4, paint: "wood-600" },
      { part: "cup", x: 74, y: 92, scale: 1.2 },
      { part: "bread", x: 60, y: 94, scale: 1 },
    ],
    light: [{ part: "halo", x: 44, y: 60, scale: 1.4, paint: "orange-400" }],
    particles: [{ part: "steam-curl", x: 74, y: 84, scale: 0.6, paint: "white" }],
  },
};

/** Ultralight Chassis (Stinkyscape #48, Common), Alt art: the ultralight chassis racing through a sunlit canyon on a red-rock moon, kicking up a long dust plume; low, wide action angle. */
const ULTRALIGHT_CHASSIS_ALT: SceneRecipe = {
  family: "stinkyscape",
  sky: ["orange-400", "red-300"],
  light: { x: 76, y: 22, r: 30, paint: "white", opacity: 0.4 },
  emblem: { x: 82, y: 72, size: 5, rotate: 0 },
  layers: {
    sky: [{ part: "sun", x: 78, y: 20, scale: 0.8 }],
    far: [
      { part: "mesa", x: 18, y: 58, scale: 0.7, paint: "red-600" },
      { part: "mesa", x: 82, y: 56, scale: 0.6, paint: "red-600" },
    ],
    mid: [{ part: "rock-floor", x: 50, y: 70, paint: "red-900" }],
    subject: [{ part: "chassis-frame", x: 54, y: 62, scale: 1.8 }],
    near: [
      { part: "cliff", x: 10, y: 60, scale: 1.2, paint: "red-600" },
      { part: "pebbles", x: 80, y: 94, scale: 1.2 },
    ],
    particles: [
      { part: "dust-cloud", x: 26, y: 66, scale: 1.2 },
      { part: "speed-streaks", x: 36, y: 62, scale: 0.8, paint: "white" },
    ],
  },
};

/** Beam v3 (Stinkyscape #50, Rare), Alt art: Beam v3 on a patrol frigate's bow, firing one wide beam that clips three pirate ships at once over a blue ice planet; long side view. */
const BEAM_V3_ALT: SceneRecipe = {
  family: "stinkyscape",
  sky: ["blue-800", "sky-300"],
  light: { x: 70, y: 46, r: 30, paint: "cyan-400", opacity: 0.35 },
  emblem: { x: 16, y: 30, size: 5, rotate: 0 },
  layers: {
    sky: [{ part: "stars", x: 50, y: 20, scale: 1.2 }],
    far: [{ part: "planet-limb", x: 50, y: 84, paint: "blue-200" }],
    subject: [
      { part: "beam-emitter", x: 46, y: 57, scale: 0.8 },
      { part: "starship", x: 36, y: 64, scale: 0.9 },
      { part: "laser", x: 68, y: 52, scale: 0.8, paint: "cyan-400" },
      { part: "fighter", x: 68, y: 44, scale: 0.55, flip: -1, paint: "red-600" },
      { part: "fighter", x: 76, y: 52, scale: 0.55, flip: -1, paint: "red-600" },
      { part: "fighter", x: 70, y: 60, scale: 0.55, flip: -1, paint: "red-600" },
    ],
    light: [{ part: "halo", x: 70, y: 52, scale: 1.2, paint: "sky-300" }],
    particles: [{ part: "sparks", x: 72, y: 52, scale: 0.8, paint: "white" }],
  },
};

/** Repeater v3 (Stinkyscape #51, Rare), Alt art: Repeater v3 bolted to a freighter's flank, spraying short bursts at pirate skiffs in an asteroid lane as the quartermaster leans from a hatch, arguing; orange tracer light. */
const REPEATER_V3_ALT: SceneRecipe = {
  family: "stinkyscape",
  sky: ["red-900", "orange-400"],
  light: { x: 66, y: 40, r: 32, paint: "orange-400", opacity: 0.4 },
  emblem: { x: 16, y: 76, size: 5, rotate: 0 },
  extras: [{ layer: "subject", x: 32, y: 62, scale: 0.2, pose: "pose-point", look: { ...figureLookFor(["Human"]), hair: "flap-cap", outfit: "outfit-coat", item: null, cloth: "blue-600" } }],
  layers: {
    sky: [{ part: "stars", x: 50, y: 18, scale: 1.2 }],
    far: [{ part: "asteroid", x: 80, y: 24, scale: 0.6, count: 3, spread: [12, 8] }],
    mid: [{ part: "asteroid", x: 14, y: 30, scale: 1.2 }],
    subject: [
      { part: "repeater-gun", x: 50, y: 52, scale: 1.1 },
      { part: "freighter", x: 42, y: 62, scale: 1.2 },
      { part: "scout", x: 76, y: 34, scale: 0.6, flip: -1 },
      { part: "scout", x: 82, y: 48, scale: 0.5, flip: -1 },
    ],
    near: [{ part: "asteroid", x: 88, y: 92, scale: 1.8 }],
    light: [
      { part: "laser", x: 66, y: 44, scale: 0.4, paint: "orange-400" },
      { part: "laser", x: 66, y: 46, scale: 0.35, paint: "orange-400" },
    ],
    particles: [{ part: "sparks", x: 76, y: 40, scale: 0.8, paint: "orange-400" }],
  },
};

/** Beam v4 (Stinkyscape #54, Epic), Alt art: Beam v4 behind glass in a hushed museum gallery, the first three models on plinths beside it; cool marble-white light, visitors in silhouette. */
const BEAM_V4_ALT: SceneRecipe = {
  family: "stinkyscape",
  sky: ["slate-100", "white"],
  light: { x: 50, y: 30, r: 40, paint: "white", opacity: 0.45 },
  emblem: { x: 84, y: 30, size: 5, rotate: 0 },
  layers: {
    sky: [{ part: "ceiling-lights", x: 50, y: 4, scale: 1 }],
    far: [
      { part: "room-wall", x: 50, y: 72, paint: "slate-100" },
      { part: "column", x: 8, y: 100, scale: 1.4 },
      { part: "column", x: 92, y: 100, scale: 1.4 },
    ],
    mid: [
      { part: "tile-floor", x: 50, y: 74, paint: "slate-400" },
      { part: "pedestal", x: 16, y: 80, scale: 1 },
      { part: "beam-emitter", x: 14, y: 70, scale: 0.3 },
      { part: "pedestal", x: 30, y: 82, scale: 0.9 },
      { part: "beam-emitter", x: 28, y: 73, scale: 0.28 },
      { part: "pedestal", x: 84, y: 80, scale: 1 },
      { part: "beam-emitter", x: 82, y: 70, scale: 0.3 },
    ],
    subject: [
      { part: "beam-emitter", x: 46, y: 64, scale: 0.7 },
      { part: "glass-case", x: 52, y: 76, scale: 2.4 },
    ],
    near: [{ part: "crowd", x: 50, y: 100, scale: 0.9, paint: "slate-800" }],
    light: [
      { part: "light-shaft", x: 52, y: 6, scale: 1.2, paint: "white" },
      { part: "rim-light", x: 52, y: 56, scale: 1, paint: "white" },
    ],
    particles: [{ part: "motes", x: 50, y: 40, scale: 1, paint: "white" }],
  },
};

/** Repeater v4 (Stinkyscape #55, Epic), Alt art: Repeater v4 firing six short bursts from a frigate's flank above a teal gas giant, tracer trails fanning out, an unread manual drifting away. */
const REPEATER_V4_ALT: SceneRecipe = {
  family: "stinkyscape",
  sky: ["blue-900", "cyan-400"],
  light: { x: 66, y: 40, r: 32, paint: "orange-400", opacity: 0.35 },
  emblem: { x: 16, y: 28, size: 5, rotate: 0 },
  layers: {
    sky: [{ part: "stars", x: 50, y: 18, scale: 1.4 }],
    far: [{ part: "gas-giant", x: 50, y: 96, scale: 2.4, paint: "jade-400" }],
    mid: [{ part: "fighter", x: 82, y: 70, scale: 0.5, flip: -1, paint: "red-600" }],
    subject: [
      { part: "repeater-gun", x: 46, y: 50, scale: 1.2 },
      { part: "starship", x: 40, y: 58, scale: 1.1 },
    ],
    near: [{ part: "notebook", x: 84, y: 28, scale: 1.1 }],
    light: [
      { part: "laser", x: 64, y: 38, scale: 0.4, paint: "orange-400" },
      { part: "laser", x: 66, y: 42, scale: 0.45, paint: "orange-400" },
      { part: "laser", x: 64, y: 46, scale: 0.4, paint: "orange-400" },
    ],
    particles: [
      { part: "speed-streaks", x: 74, y: 42, scale: 0.8, paint: "orange-400" },
      { part: "sparks", x: 62, y: 42, scale: 0.8, paint: "white" },
    ],
  },
};

/** Cannon v4 (Stinkyscape #56, Epic), Alt art: Cannon v4 recoiling on a siege barge's deck at dawn above a cloud planet, a heavy shell streaking toward a distant fortress; pink sky. */
const CANNON_V4_ALT: SceneRecipe = {
  family: "stinkyscape",
  sky: ["violet-300", "red-300"],
  light: { x: 20, y: 60, r: 34, paint: "white", opacity: 0.35 },
  emblem: { x: 16, y: 28, size: 5, rotate: 0 },
  layers: {
    sky: [{ part: "sun", x: 16, y: 62, scale: 0.8 }],
    far: [
      { part: "cloud", x: 30, y: 76, scale: 1.6, count: 3, spread: [30, 4], paint: "white" },
      { part: "floating-island", x: 84, y: 40, scale: 0.5 },
      { part: "fortress", x: 84, y: 36, scale: 0.4, paint: "slate-700" },
    ],
    mid: [{ part: "cloud", x: 60, y: 88, scale: 1.4, count: 2, spread: [24, 4], paint: "violet-300" }],
    subject: [
      { part: "great-cannon", x: 36, y: 58, scale: 0.75 },
      { part: "barge", x: 46, y: 66, scale: 1.4 },
    ],
    near: [{ part: "cloud", x: 14, y: 96, scale: 1.6, paint: "white" }],
    light: [{ part: "halo", x: 70, y: 52, scale: 1, paint: "white" }],
    particles: [
      { part: "meteor", x: 62, y: 46, scale: 0.6 },
      { part: "smoke-billow", x: 66, y: 52, scale: 0.6, paint: "slate-400" },
    ],
  },
};

/** R3394000: the alt recipes of #3394's cards 30 to 56, by id. */
export const STINKYSCAPE_ALT_RECIPES_2: Readonly<Record<string, SceneRecipe>> = {
  "stinkyscape-030": DRONE_CORE_IV_ALT,
  "stinkyscape-033": DRONE_CORE_VII_ALT,
  "stinkyscape-034": DRONE_CORE_VIII_ALT,
  "stinkyscape-035": DRONE_CORE_IX_ALT,
  "stinkyscape-039": THE_NARCOR_PROJECT_ALT,
  "stinkyscape-041": DRONE_SYMPATHIZER_ALT,
  "stinkyscape-042": CORE_RESEARCHER_ALT,
  "stinkyscape-043": HEALING_CAPSULE_ALT,
  "stinkyscape-044": GREAT_PAPERCLIP_THEORY_ALT,
  "stinkyscape-045": DRONE_COMMANDER_ALT,
  "stinkyscape-048": ULTRALIGHT_CHASSIS_ALT,
  "stinkyscape-050": BEAM_V3_ALT,
  "stinkyscape-051": REPEATER_V3_ALT,
  "stinkyscape-054": BEAM_V4_ALT,
  "stinkyscape-055": REPEATER_V4_ALT,
  "stinkyscape-056": CANNON_V4_ALT,
};
