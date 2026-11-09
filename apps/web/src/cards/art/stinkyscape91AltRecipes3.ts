// R3406000: the alt scene recipes of Stinkyscape's later cards (#3406), cards 147 to 177 (stinkyscape91AltRecipes1.ts
// says what each is held to). Presentation only (CLAUDE.md rule 7): a recipe is read by the card's id alone.

import { figureLookFor, type FigureLook } from "./figureKit.ts";
import type { SceneRecipe } from "./scene.ts";
import { normalExtraLookOf, normalLookOf } from "./stinkyscape91AltRecipes1.ts";

/** IN33DH3 (Stinkyscape #147, Legendary), Alt art: IN33DH3 off duty in a cluttered dorm pod, cross-legged on his bunk asking his AI companion which snack to eat; tangerine lamp light, overhead view. The figure kit has no cross-legged pose, so he sits. */
const IN33DH3_ALT: SceneRecipe = {
  family: "stinkyscape",
  sky: ["orange-400", "brass-600"],
  light: { x: 70, y: 22, r: 36, paint: "orange-400", opacity: 0.45 },
  emblem: { x: 84, y: 76, size: 5, rotate: 0 },
  figure: { x: 46, y: 76, scale: 0.8, pose: "pose-sit", look: normalLookOf("stinkyscape-147") },
  layers: {
    sky: [{ part: "porthole", x: 16, y: 26, scale: 1 }],
    far: [
      { part: "room-wall", x: 50, y: 62, paint: "wood-700" },
      { part: "shelf", x: 82, y: 50, scale: 0.9 },
    ],
    mid: [{ part: "bed", x: 46, y: 82, scale: 1.6 }],
    subject: [{ part: "spirit", x: 72, y: 76, scale: 0.5, paint: "cyan-400" }],
    near: [
      { part: "cracker", x: 22, y: 90, scale: 1.2 },
      { part: "apple", x: 32, y: 94, scale: 1.2 },
      { part: "controller", x: 80, y: 96, scale: 0.6 },
    ],
    light: [
      { part: "lantern", x: 70, y: 10, scale: 1 },
      { part: "halo", x: 70, y: 18, scale: 1.2, paint: "orange-400" },
      { part: "halo", x: 72, y: 58, scale: 0.8, paint: "cyan-400" },
    ],
    particles: [
      { part: "motes", x: 50, y: 50, scale: 1, paint: "orange-400" },
      { part: "thought-bubble", x: 62, y: 38, scale: 0.7 },
    ],
  },
};

/** Alteran Reach (Stinkyscape #148, Mythic), Alt art: close on one stop of the route: the traveller's ship landing on a tiny seafoam moon, a glowing souvenir shell in her hand, the next node a speck far above. */
const ALTERAN_REACH_ALT: SceneRecipe = {
  family: "stinkyscape",
  sky: ["sky-300", "green-200"],
  light: { x: 66, y: 20, r: 40, paint: "white", opacity: 0.4 },
  emblem: { x: 84, y: 78, size: 5, rotate: 0 },
  figure: {
    x: 34,
    y: 100,
    scale: 1.3,
    pose: "pose-hold",
    look: { ...figureLookFor(["Human"]), hair: "hair-long", outfit: "outfit-flightsuit", item: null, skin: "wood-700", cloth: "jade-400" },
  },
  layers: {
    sky: [
      { part: "moon", x: 72, y: 12, scale: 0.25 },
      { part: "stars", x: 50, y: 20, scale: 0.8 },
      { part: "cloud", x: 24, y: 30, scale: 1, paint: "white" },
      { part: "cloud", x: 88, y: 36, scale: 0.7, paint: "white" },
    ],
    far: [
      { part: "planet-limb", x: 50, y: 64, paint: "green-400" },
      { part: "sea", x: 50, y: 72, paint: "jade-400" },
    ],
    mid: [
      { part: "sand", x: 50, y: 82, paint: "green-200" },
      { part: "crystal", x: 88, y: 84, scale: 0.8, count: 2, spread: [4, 2], paint: "cyan-400" },
    ],
    subject: [{ part: "shuttle", x: 70, y: 50, scale: 1.4 }],
    near: [
      { part: "seashell", x: 48, y: 64, scale: 0.7 },
      { part: "crystal", x: 10, y: 98, scale: 1, count: 2, spread: [4, 2], paint: "cyan-400" },
      { part: "pebbles", x: 80, y: 98, scale: 1.2 },
      { part: "pebbles", x: 20, y: 92, scale: 0.9 },
    ],
    light: [
      { part: "halo", x: 48, y: 64, scale: 0.8, paint: "cyan-400" },
      { part: "thruster-flame", x: 70, y: 50, scale: 0.6, paint: "sky-300" },
      { part: "rim-light", x: 34, y: 60, scale: 1, paint: "white" },
      { part: "god-rays", x: 72, y: 12, scale: 0.6, paint: "white" },
    ],
    particles: [
      { part: "spray", x: 24, y: 76, scale: 1, count: 2, spread: [10, 3] },
      { part: "motes", x: 50, y: 50, scale: 1.2, paint: "white" },
      { part: "dust", x: 70, y: 76, scale: 1, paint: "white" },
      { part: "sparks", x: 48, y: 60, scale: 0.6, paint: "cyan-400" },
    ],
  },
};

/** Call to Chaos (Stinky Edition) (Stinkyscape #150, Legendary), Alt art: after the roll: a lone captain's bridge drenched in pink stink-smoke, alarms flashing, a Stinkyscape Golem waving cheerfully through the viewport; tight interior close-up. */
const CALL_TO_CHAOS_STINKY_ALT: SceneRecipe = {
  family: "stinkyscape",
  sky: ["red-300", "purple-400"],
  light: { x: 50, y: 40, r: 40, paint: "red-300", opacity: 0.45 },
  emblem: { x: 16, y: 30, size: 5, rotate: 0 },
  figure: { x: 34, y: 110, scale: 1.5, pose: "pose-guard", look: normalLookOf("stinkyscape-150") },
  layers: {
    sky: [{ part: "stars", x: 62, y: 30, scale: 1 }],
    far: [
      { part: "viewport", x: 62, y: 40, scale: 1.6 },
      { part: "scrap-golem", x: 62, y: 44, scale: 0.5 },
    ],
    mid: [{ part: "console", x: 84, y: 86, scale: 1.2 }],
    subject: [{ part: "stink-cloud", x: 48, y: 62, scale: 1.6, paint: "red-300" }],
    near: [{ part: "console", x: 12, y: 102, scale: 1.4 }],
    light: [
      { part: "alarm-light", x: 14, y: 22, scale: 1.2 },
      { part: "alarm-light", x: 88, y: 22, scale: 1.2 },
      { part: "halo", x: 50, y: 50, scale: 1.6, paint: "red-300" },
    ],
    particles: [
      { part: "smoke", x: 50, y: 72, scale: 1.4, paint: "red-300" },
      { part: "motes", x: 50, y: 50, scale: 1, paint: "purple-400" },
    ],
  },
};

/** Call to Chaos (Scape Edition) (Stinkyscape #151, Legendary), Alt art: the same roll from a god's-eye view: the starscape as a vast round tabletop, fleets and rifts scattering like spilled pieces; warm crimson and white light. The kit's game board is checkered, so the tabletop is a board seen from above. */
const CALL_TO_CHAOS_SCAPE_ALT: SceneRecipe = {
  family: "stinkyscape",
  sky: ["red-600", "white"],
  light: { x: 50, y: 30, r: 44, paint: "white", opacity: 0.4 },
  emblem: { x: 84, y: 78, size: 5, rotate: 0 },
  layers: {
    sky: [{ part: "stars", x: 50, y: 26, scale: 1.4 }],
    far: [{ part: "game-board", x: 50, y: 62, scale: 3.4, paint: "slate-800" }],
    mid: [
      { part: "space-rift", x: 26, y: 54, scale: 0.45 },
      { part: "space-rift", x: 74, y: 70, scale: 0.4 },
    ],
    subject: [
      { part: "interceptor", x: 38, y: 58, scale: 0.6, count: 3, spread: [12, 6] },
      { part: "starship", x: 64, y: 52, scale: 0.4 },
      { part: "fighter", x: 58, y: 70, scale: 0.6 },
    ],
    near: [{ part: "die", x: 84, y: 92, scale: 2.4 }],
    light: [
      { part: "god-rays", x: 50, y: 0, scale: 1.2, paint: "white" },
      { part: "halo", x: 50, y: 58, scale: 2, paint: "red-300" },
    ],
    particles: [
      { part: "sparks", x: 50, y: 60, scale: 1, paint: "white" },
      { part: "motes", x: 50, y: 50, scale: 1.2, paint: "red-300" },
    ],
  },
};

/** Sanctum (Stinkyscape #152, Legendary), Alt art: Sanctum from outside: the hollowed asteroid drifting in a sunlit emerald nebula, its rose-window porthole glowing, a pilgrim shuttle approaching; wide exterior vista. */
const SANCTUM_ALT: SceneRecipe = {
  family: "stinkyscape",
  sky: ["green-700", "jade-400"],
  light: { x: 78, y: 22, r: 40, paint: "white", opacity: 0.4 },
  emblem: { x: 16, y: 78, size: 5, rotate: 0 },
  layers: {
    sky: [
      { part: "nebula", x: 30, y: 30, scale: 1.6, paint: "green-400" },
      { part: "nebula", x: 78, y: 76, scale: 1.2, paint: "jade-400" },
      { part: "stars", x: 50, y: 50, scale: 1.4 },
      { part: "sun", x: 80, y: 20, scale: 0.6 },
    ],
    far: [{ part: "asteroid", x: 18, y: 30, scale: 0.6, count: 5, spread: [12, 12] }],
    mid: [{ part: "asteroid", x: 54, y: 52, scale: 3.4, paint: "stone-500" }],
    subject: [{ part: "porthole", x: 54, y: 52, scale: 1.2 }],
    near: [
      { part: "shuttle", x: 22, y: 80, scale: 1 },
      { part: "asteroid", x: 88, y: 92, scale: 0.9, count: 2, spread: [6, 4] },
    ],
    light: [
      { part: "halo", x: 54, y: 52, scale: 1, paint: "red-300" },
      { part: "god-rays", x: 80, y: 20, scale: 1, paint: "white" },
    ],
    particles: [
      { part: "motes", x: 50, y: 50, scale: 1.2, paint: "green-200" },
      { part: "sparks", x: 54, y: 52, scale: 0.8, paint: "red-300" },
    ],
  },
};

/** Stimmycheck (Stinkyscape #153, Legendary), Alt art: weeks later: the same market stall in grey rain, prices towering as rising bars, the vendor counting a thin stack of chips under an awning, a new drone high above. */
const STIMMYCHECK_ALT: SceneRecipe = {
  family: "stinkyscape",
  sky: ["slate-400", "grey-500"],
  light: { x: 50, y: 30, r: 36, paint: "slate-100", opacity: 0.3 },
  emblem: { x: 16, y: 30, size: 5, rotate: 0 },
  figure: { x: 48, y: 102, scale: 1.2, pose: "pose-hold", look: normalLookOf("stinkyscape-153") },
  layers: {
    sky: [
      { part: "cloud", x: 40, y: 18, scale: 1.6, paint: "slate-700" },
      { part: "drone", x: 80, y: 14, scale: 0.5 },
    ],
    far: [
      { part: "price-bars", x: 78, y: 58, scale: 1.6, paint: "red-400" },
      { part: "price-bars", x: 18, y: 60, scale: 1.3, paint: "red-300" },
    ],
    mid: [{ part: "awning", x: 48, y: 78, scale: 2.2, paint: "red-400" }],
    subject: [{ part: "credit-chip", x: 62, y: 76, scale: 0.8, count: 2, spread: [1.5, 1] }],
    near: [{ part: "counter", x: 50, y: 106, scale: 1.4, paint: "wood-700" }],
    light: [{ part: "halo", x: 48, y: 56, scale: 1, paint: "slate-100" }],
    particles: [
      { part: "rain", x: 50, y: 40, scale: 1.4, paint: "slate-100" },
      { part: "drops", x: 30, y: 60, scale: 1, count: 2, spread: [16, 6], paint: "slate-100" },
    ],
  },
};

/** Dice of Doom (Stinkyscape #154, Mythic), Alt art: the hundred-sided die tumbles down the steps of a ruined observatory on a crimson desert world, a dust trail behind it; low-angle shot. */
const DICE_OF_DOOM_ALT: SceneRecipe = {
  family: "stinkyscape",
  sky: ["red-600", "orange-400"],
  light: { x: 76, y: 28, r: 40, paint: "orange-400", opacity: 0.4 },
  emblem: { x: 16, y: 30, size: 5, rotate: 0 },
  layers: {
    sky: [
      { part: "sun", x: 76, y: 28, scale: 0.8 },
      { part: "planet", x: 22, y: 20, scale: 0.6, paint: "red-300" },
      { part: "stars", x: 50, y: 14, scale: 1 },
    ],
    far: [
      { part: "mesa", x: 30, y: 58, paint: "red-900" },
      { part: "dome", x: 70, y: 52, scale: 0.6, paint: "slate-400" },
      { part: "telescope", x: 72, y: 42, scale: 0.9 },
    ],
    mid: [
      { part: "stairs", x: 50, y: 56, scale: 1.2, paint: "stone-500" },
      { part: "ruin-wall", x: 16, y: 72, scale: 1 },
      { part: "ruin-wall", x: 86, y: 74, scale: 0.9, flip: -1 },
      { part: "fallen-column", x: 72, y: 84, scale: 0.9 },
      { part: "fallen-column", x: 30, y: 66, scale: 0.6, flip: -1 },
      { part: "ruin-wall", x: 60, y: 46, scale: 0.6 },
    ],
    subject: [{ part: "d100", x: 44, y: 72, scale: 1.7 }],
    near: [
      { part: "pebbles", x: 82, y: 98, scale: 1.4, count: 2, spread: [6, 2] },
      { part: "pebbles", x: 18, y: 96, scale: 1.2 },
      { part: "fallen-column", x: 8, y: 100, scale: 1.1 },
    ],
    light: [
      { part: "god-rays", x: 76, y: 28, scale: 1, paint: "orange-400" },
      { part: "rim-light", x: 44, y: 66, scale: 1, paint: "orange-400" },
      { part: "halo", x: 44, y: 72, scale: 1, paint: "red-300" },
    ],
    particles: [
      { part: "dust-cloud", x: 56, y: 60, scale: 1.2, paint: "red-300" },
      { part: "dust", x: 50, y: 86, scale: 1.2, paint: "red-300" },
      { part: "sparks", x: 44, y: 70, scale: 0.8, paint: "white" },
    ],
  },
};

/** Comet Crash (Stinkyscape #155, Rare), Alt art: the descent, moments before impact: a blazing blue-white comet streaks low over a grassy green planet's plains as herds scatter; wide sky view. */
const COMET_CRASH_ALT: SceneRecipe = {
  family: "stinkyscape",
  sky: ["sky-400", "blue-200"],
  light: { x: 46, y: 44, r: 40, paint: "white", opacity: 0.4 },
  emblem: { x: 84, y: 30, size: 5, rotate: 0 },
  layers: {
    sky: [{ part: "cloud", x: 72, y: 24, scale: 1.4, paint: "white" }],
    far: [{ part: "hills", x: 50, y: 64, paint: "green-500" }],
    mid: [
      { part: "field", x: 50, y: 74 },
      { part: "deer", x: 30, y: 84, scale: 0.5, count: 3, spread: [12, 3] },
      { part: "deer", x: 74, y: 82, scale: 0.45, flip: -1 },
    ],
    subject: [
      { part: "comet", x: 50, y: 46, scale: 1.4 },
      { part: "ice-shard", x: 62, y: 52, scale: 0.8 },
    ],
    near: [{ part: "tall-grass", x: 10, y: 98, scale: 1.4 }],
    light: [{ part: "halo", x: 50, y: 46, scale: 1.2, paint: "sky-300" }],
    particles: [{ part: "sparks", x: 50, y: 46, scale: 1, paint: "white" }],
  },
};

/** The Gauntlet (Stinkyscape #157, Epic), Alt art: The Gauntlet seen from outside: three glowing tubes of force threaded across a sunlit canyon on a rust-red planet, tiny figures duelling within; wide aerial vista. Its tubes are the kit's walls of energy, its duellists the normal picture's mechs, far below. */
const THE_GAUNTLET_ALT: SceneRecipe = {
  family: "stinkyscape",
  sky: ["sky-300", "red-300"],
  light: { x: 76, y: 20, r: 36, paint: "white", opacity: 0.35 },
  emblem: { x: 16, y: 30, size: 5, rotate: 0 },
  layers: {
    far: [
      { part: "mesa", x: 50, y: 50, paint: "red-600" },
      { part: "cliff", x: 8, y: 58, paint: "red-900" },
      { part: "cliff", x: 92, y: 58, flip: -1, paint: "red-900" },
    ],
    mid: [{ part: "cracked-earth", x: 50, y: 66, paint: "red-600" }],
    subject: [
      { part: "energy-wall", x: 26, y: 74, scale: 0.6, paint: "purple-400" },
      { part: "energy-wall", x: 50, y: 78, scale: 0.6, paint: "purple-400" },
      { part: "energy-wall", x: 74, y: 74, scale: 0.6, paint: "purple-400" },
      { part: "mech", x: 47, y: 76, scale: 0.2 },
      { part: "mech", x: 53, y: 76, scale: 0.2, flip: -1, paint: "red-600" },
    ],
    near: [
      { part: "pebbles", x: 18, y: 98, scale: 1.4 },
      { part: "pebbles", x: 82, y: 96, scale: 1.2 },
    ],
    light: [{ part: "halo", x: 50, y: 66, scale: 1.4, paint: "violet-300" }],
    particles: [
      { part: "dust", x: 50, y: 84, scale: 1, paint: "red-300" },
      { part: "sparks", x: 50, y: 66, scale: 0.6, paint: "violet-300" },
    ],
  },
};

/** Stratos Swarm (Stinkyscape #158, Rare), Alt art: the Stratos assembly line: hundreds of identical fighters roll off a conveyor in a white clean room as an accountant beams beside it; bright clinical white. */
const STRATOS_SWARM_ALT: SceneRecipe = {
  family: "stinkyscape",
  sky: ["white", "slate-100"],
  light: { x: 50, y: 20, r: 44, paint: "white", opacity: 0.5 },
  emblem: { x: 84, y: 30, size: 5, rotate: 0 },
  figure: {
    x: 18,
    y: 96,
    scale: 0.9,
    pose: "pose-hold",
    look: { ...figureLookFor(["Human"]), hair: "glasses", outfit: "outfit-suit", item: "item-clipboard", skin: "red-300", cloth: "slate-700" },
  },
  layers: {
    far: [
      { part: "tile-wall", x: 50, y: 70 },
      { part: "ceiling-lights", x: 50, y: 4, scale: 1 },
    ],
    mid: [{ part: "robot-arm", x: 88, y: 74, scale: 0.6, flip: -1 }],
    subject: [
      { part: "conveyor", x: 60, y: 80, scale: 1.3 },
      { part: "fighter", x: 40, y: 74, scale: 0.7, paint: "sky-400" },
      { part: "fighter", x: 56, y: 74, scale: 0.7, paint: "sky-400" },
      { part: "fighter", x: 72, y: 74, scale: 0.7, paint: "sky-400" },
      { part: "fighter", x: 88, y: 74, scale: 0.7, paint: "sky-400" },
    ],
    particles: [{ part: "motes", x: 50, y: 50, scale: 1.2, paint: "sky-300" }],
  },
};

/** Faction: ETHICS (Stinkyscape #159, Rare), Alt art: after the meeting: six silhouettes cross a skybridge between tower rings at violet dusk, papers tucked under their arms; wide view. Every other member carries the papers, a scroll, so six held scrolls do not take the picture past the byte ceiling. */
const FACTION_ETHICS_ALT: SceneRecipe = {
  family: "stinkyscape",
  sky: ["violet-700", "red-300"],
  light: { x: 50, y: 72, r: 40, paint: "red-300", opacity: 0.35 },
  emblem: { x: 84, y: 30, size: 5, rotate: 0 },
  figure: { x: 22, y: 60, scale: 0.3, pose: "pose-stride", look: { ...normalLookOf("stinkyscape-159"), item: "item-scroll" } },
  extras: [0, 1, 2, 3, 4].map((k) => ({
    layer: "subject" as const,
    x: 33 + k * 11,
    y: 60,
    scale: 0.3,
    pose: "pose-stride" as const,
    look: { ...normalExtraLookOf("stinkyscape-159", k), item: k % 2 === 1 ? ("item-scroll" as const) : null },
  })),
  layers: {
    sky: [{ part: "stars", x: 50, y: 16, scale: 1 }],
    far: [{ part: "tower", x: 50, y: 92, scale: 1.2, count: 3, spread: [36, 3], paint: "violet-500" }],
    mid: [
      { part: "glass-spire", x: 6, y: 100, scale: 1.4 },
      { part: "glass-spire", x: 94, y: 100, scale: 1.4 },
    ],
    subject: [{ part: "bridge", x: 50, y: 60, scale: 1.6, paint: "slate-400" }],
    particles: [{ part: "motes", x: 50, y: 40, scale: 1.2, paint: "red-300" }],
  },
};

/** Faction: Golden Accord (Stinkyscape #160, Rare), Alt art: the accord's fleet: three Paragon warships and a Falcon fighter fly in tight formation through a teal ocean world's cloudbanks, an emblem pennant streaming; bright midday. The pennant is the kit's flag. */
const FACTION_GOLDEN_ACCORD_ALT: SceneRecipe = {
  family: "stinkyscape",
  sky: ["sky-400", "cyan-400"],
  light: { x: 24, y: 20, r: 40, paint: "white", opacity: 0.4 },
  emblem: { x: 16, y: 78, size: 5, rotate: 0 },
  layers: {
    sky: [{ part: "sun", x: 20, y: 18, scale: 0.6 }],
    far: [
      { part: "sea", x: 50, y: 82, paint: "jade-400" },
      { part: "cloud", x: 28, y: 62, scale: 1.6, paint: "white" },
    ],
    mid: [{ part: "cloud", x: 78, y: 70, scale: 1.4, paint: "white" }],
    subject: [
      { part: "starship", x: 44, y: 34, scale: 0.6 },
      { part: "starship", x: 62, y: 46, scale: 0.6 },
      { part: "starship", x: 40, y: 56, scale: 0.6 },
      { part: "fighter", x: 76, y: 32, scale: 0.8, paint: "brass-500" },
    ],
    near: [
      { part: "cloud", x: 84, y: 94, scale: 1.6, paint: "white" },
      { part: "flag", x: 26, y: 31, scale: 0.6 },
    ],
    particles: [{ part: "speed-streaks", x: 50, y: 44, scale: 1, paint: "white" }],
  },
};

/** Scanning Probe Mk2 (Stinkyscape #167, Rare), Alt art: close-up of the probe's impatient twin antennae twitching above a frozen comet's surface as a strange shimmer rises from a crevasse; pale mint light. The kit's probe has one antenna. */
const SCANNING_PROBE_MK2_ALT: SceneRecipe = {
  family: "stinkyscape",
  sky: ["green-200", "slate-100"],
  light: { x: 62, y: 72, r: 36, paint: "green-200", opacity: 0.4 },
  emblem: { x: 84, y: 30, size: 5, rotate: 0 },
  layers: {
    sky: [{ part: "stars", x: 50, y: 18, scale: 1 }],
    far: [{ part: "ice-floor", x: 50, y: 70 }],
    mid: [
      { part: "frost", x: 28, y: 78, scale: 1.4 },
      { part: "frost", x: 76, y: 82, scale: 1.2 },
    ],
    subject: [{ part: "probe", x: 44, y: 42, scale: 4.2 }],
    near: [{ part: "ice-shard", x: 86, y: 92, scale: 1.2, flip: -1 }],
    light: [{ part: "light-ribbon", x: 64, y: 72, scale: 0.6, paint: "green-400" }],
    particles: [
      { part: "motes", x: 64, y: 70, scale: 1, paint: "green-200" },
      { part: "sparks", x: 64, y: 72, scale: 0.8, paint: "white" },
    ],
  },
};

/** Tiny Asteroid (Stinkyscape #169, Common), Alt art: the tiny asteroid as a paperweight on a miner's cluttered desk in a station cabin, a sliver of ore glinting under a desk lamp; cosy brown interior. */
const TINY_ASTEROID_ALT: SceneRecipe = {
  family: "stinkyscape",
  sky: ["wood-700", "wood-600"],
  light: { x: 30, y: 34, r: 34, paint: "orange-400", opacity: 0.4 },
  emblem: { x: 84, y: 30, size: 5, rotate: 0 },
  layers: {
    far: [
      { part: "room-wall", x: 50, y: 70, paint: "wood-800" },
      { part: "shelf", x: 80, y: 50, scale: 1 },
    ],
    mid: [{ part: "desk", x: 50, y: 100, scale: 5 }],
    subject: [
      { part: "asteroid", x: 54, y: 72, scale: 1.4, paint: "stone-500" },
      { part: "ore-chunk", x: 58, y: 72, scale: 0.6, paint: "sky-400" },
    ],
    near: [
      { part: "pages", x: 24, y: 80, scale: 0.8 },
      { part: "cup", x: 78, y: 80, scale: 1.4 },
    ],
    light: [
      { part: "lantern", x: 30, y: 30, scale: 1 },
      { part: "halo", x: 30, y: 40, scale: 1, paint: "orange-400" },
    ],
    particles: [{ part: "sparks", x: 58, y: 70, scale: 0.6, paint: "white" }],
  },
};

/** Huge Asteroid (Stinkyscape #172, Epic), Alt art: inside the huge asteroid: a cavernous mined-out hollow round a glowing core chamber, miners' lamps strung along a narrow catwalk; warm red rock. */
const HUGE_ASTEROID_ALT: SceneRecipe = {
  family: "stinkyscape",
  sky: ["wood-700", "red-600"],
  light: { x: 50, y: 56, r: 40, paint: "orange-400", opacity: 0.45 },
  emblem: { x: 16, y: 30, size: 5, rotate: 0 },
  layers: {
    far: [{ part: "cave", x: 50, y: 54 }],
    mid: [{ part: "rock-floor", x: 50, y: 84, paint: "red-900" }],
    subject: [{ part: "reactor", x: 50, y: 86, scale: 1.4 }],
    near: [{ part: "catwalk", x: 50, y: 40, scale: 1 }],
    light: [
      { part: "lantern", x: 30, y: 31, scale: 0.6, count: 3, spread: [24, 0.5] },
      { part: "halo", x: 50, y: 64, scale: 1.8, paint: "orange-400" },
    ],
    particles: [
      { part: "dust", x: 50, y: 80, scale: 1.2, paint: "red-300" },
      { part: "embers", x: 50, y: 60, scale: 1, paint: "orange-400" },
    ],
  },
};

/** A trader of #173's tower in its heyday, weighing spice. */
const TRADER: FigureLook = { ...figureLookFor(["Human"]), hair: "kerchief", outfit: "outfit-kaftan", item: null, skin: "wood-700", cloth: "red-400" };

/** Abandoned Spice Tower (Stinkyscape #173, Rare), Alt art: the tower in its busy heyday before the traders fled: bright awnings, cargo ships lined up at its docks, merchants weighing spice; cheerful turquoise noon. */
const ABANDONED_SPICE_TOWER_ALT: SceneRecipe = {
  family: "stinkyscape",
  sky: ["sky-400", "green-200"],
  light: { x: 30, y: 16, r: 40, paint: "white", opacity: 0.4 },
  emblem: { x: 16, y: 30, size: 5, rotate: 0 },
  extras: [{ layer: "near", x: 46, y: 100, scale: 0.55, flip: -1, pose: "pose-hold", look: TRADER }],
  layers: {
    sky: [{ part: "sun", x: 30, y: 16, scale: 0.6 }],
    far: [
      { part: "sand", x: 50, y: 74, paint: "orange-400" },
      { part: "freighter", x: 24, y: 40, scale: 0.5 },
      { part: "freighter", x: 30, y: 54, scale: 0.4 },
    ],
    mid: [
      { part: "awning", x: 22, y: 84, scale: 1, paint: "red-400" },
      { part: "awning", x: 42, y: 86, scale: 0.9, paint: "jade-400" },
    ],
    subject: [{ part: "spice-tower", x: 66, y: 86, scale: 0.9 }],
    near: [
      { part: "sacks", x: 18, y: 98, scale: 1.2 },
      { part: "scales", x: 32, y: 100, scale: 0.7 },
    ],
    particles: [{ part: "motes", x: 50, y: 50, scale: 1.2, paint: "white" }],
  },
};

/** Aerelret (Stinkyscape #174, Epic), Alt art: the wait: a single folded crystal drifting in a quiet violet asteroid lane, faint outlines of a sleeping fleet inside it; still, close framing. */
const AERELRET_ALT: SceneRecipe = {
  family: "stinkyscape",
  sky: ["violet-500", "violet-300"],
  light: { x: 50, y: 52, r: 40, paint: "violet-300", opacity: 0.4 },
  emblem: { x: 84, y: 30, size: 5, rotate: 0 },
  layers: {
    sky: [{ part: "stars", x: 50, y: 30, scale: 1 }],
    far: [{ part: "asteroid", x: 18, y: 28, scale: 0.5, count: 4, spread: [12, 10] }],
    mid: [{ part: "asteroid", x: 86, y: 72, scale: 1.2 }],
    subject: [
      { part: "crystal", x: 50, y: 86, scale: 3, paint: "violet-300" },
      { part: "starship", x: 48, y: 60, scale: 0.35, fade: 0.35 },
      { part: "starship", x: 53, y: 70, scale: 0.3, fade: 0.3 },
    ],
    near: [{ part: "asteroid", x: 10, y: 92, scale: 1.6 }],
    light: [{ part: "halo", x: 50, y: 64, scale: 1.4, paint: "violet-300" }],
    particles: [{ part: "motes", x: 50, y: 50, scale: 1.2, paint: "white" }],
  },
};

/** A dockhand of #177's spaceport, doing a double take. */
const DOCKHAND: FigureLook = { ...figureLookFor(["Human"]), hair: "flap-cap", outfit: "outfit-overalls", item: null, skin: "red-300", cloth: "orange-400" };

/** Cloaking Module (Stinkyscape #177, Rare), Alt art: a ship half-vanished at a busy sunny spaceport dock, its cloaking module's shimmer peeling the hull into the scenery while dockhands do a double take. */
const CLOAKING_MODULE_ALT: SceneRecipe = {
  family: "stinkyscape",
  sky: ["sky-400", "slate-100"],
  light: { x: 76, y: 24, r: 36, paint: "white", opacity: 0.4 },
  emblem: { x: 16, y: 30, size: 5, rotate: 0 },
  extras: [
    { layer: "near", x: 14, y: 102, scale: 0.7, pose: "pose-point", look: DOCKHAND },
    { layer: "near", x: 86, y: 102, scale: 0.66, flip: -1, pose: "pose-spread", look: { ...DOCKHAND, hair: "beanie", cloth: "blue-600" } },
  ],
  layers: {
    sky: [{ part: "sun", x: 76, y: 22, scale: 0.6 }],
    far: [{ part: "gantry", x: 88, y: 78, scale: 1.2 }],
    mid: [{ part: "launch-pad", x: 50, y: 86, scale: 1.4 }],
    subject: [
      { part: "starship", x: 50, y: 56, scale: 1.2, fade: 0.5 },
      { part: "pod-capsule", x: 52, y: 50, scale: 1, fade: 0.5 },
    ],
    light: [{ part: "warp-flash", x: 50, y: 54, scale: 0.8, paint: "cyan-400" }],
    particles: [
      { part: "glitch", x: 62, y: 56, scale: 1 },
      { part: "motes", x: 50, y: 56, scale: 1, paint: "cyan-400" },
    ],
  },
};

/** R3406000: the alt recipes of #3406's cards 147 to 177, by id. */
export const STINKYSCAPE_91_ALT_RECIPES_3: Readonly<Record<string, SceneRecipe>> = {
  "stinkyscape-147": IN33DH3_ALT,
  "stinkyscape-148": ALTERAN_REACH_ALT,
  "stinkyscape-150": CALL_TO_CHAOS_STINKY_ALT,
  "stinkyscape-151": CALL_TO_CHAOS_SCAPE_ALT,
  "stinkyscape-152": SANCTUM_ALT,
  "stinkyscape-153": STIMMYCHECK_ALT,
  "stinkyscape-154": DICE_OF_DOOM_ALT,
  "stinkyscape-155": COMET_CRASH_ALT,
  "stinkyscape-157": THE_GAUNTLET_ALT,
  "stinkyscape-158": STRATOS_SWARM_ALT,
  "stinkyscape-159": FACTION_ETHICS_ALT,
  "stinkyscape-160": FACTION_GOLDEN_ACCORD_ALT,
  "stinkyscape-167": SCANNING_PROBE_MK2_ALT,
  "stinkyscape-169": TINY_ASTEROID_ALT,
  "stinkyscape-172": HUGE_ASTEROID_ALT,
  "stinkyscape-173": ABANDONED_SPICE_TOWER_ALT,
  "stinkyscape-174": AERELRET_ALT,
  "stinkyscape-177": CLOAKING_MODULE_ALT,
};
