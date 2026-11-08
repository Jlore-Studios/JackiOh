// R3392000: Core's scenes, part 1 of 3: Core #1 to #37, each drawn from its brief in the
// issue's table (#3392) in Core's family, the glass orb (ART.md, "Core" and "Core's scenes"). Each recipe keeps
// its subject within x 30 to 70 and y 25 to 85, its light from the upper left unless its brief names another
// source, and its emblem small on a prop or a chest; a Unit's character is a figure built on its tags' look, or
// a creature part in its subject layer. Each doc comment gives the brief in a line and any change made to fit
// the kits.

import type { SceneRecipe } from "./scene.ts";
import { figureLookFor, KY_TUTOR_LOOK } from "./figureKit.ts";

/** Big D-fender (Core #1, Common Unit): a huge calm man in layered plate behind a tower shield on a lighthouse jetty, gulls on his helmet, crisp morning light. */
const BIG_D_FENDER: SceneRecipe = {
  family: "core",
  sky: ["sky-300", "slate-100"],
  light: { x: 24, y: 22, r: 40, paint: "white", opacity: 0.55 },
  emblem: { x: 50, y: 58, size: 6, rotate: 0 },
  figure: {
    x: 50,
    y: 86,
    scale: 1.05,
    pose: "pose-guard",
    look: { ...figureLookFor(["Human"]), hair: "steel-helmet", outfit: "outfit-layered-plate", item: "item-pavise", cloth: "slate-400" },
  },
  layers: {
    far: [{ part: "sea", x: 50, y: 62 }],
    mid: [
      { part: "lighthouse", x: 82, y: 68, scale: 1.1 },
      { part: "boards", x: 50, y: 72 },
    ],
    near: [
      { part: "fence-post", x: 10, y: 96 },
      { part: "gull", x: 50, y: 28, scale: 1.3, count: 2, spread: [4, 1], props: true },
    ],
  },
};

/** Speciesist (Core #2, Common Unit): a sneering man in a starched collar flings a crackling bolt at a fleeing lizard-folk traveller at a whitewashed checkpoint, flat noon glare. */
const SPECIESIST: SceneRecipe = {
  family: "core",
  sky: ["sky-400", "blue-200"],
  light: { x: 30, y: 12, r: 48, paint: "white", opacity: 0.6 },
  emblem: { x: 31, y: 58, size: 5, rotate: 0 },
  figure: {
    x: 30,
    y: 84,
    scale: 1,
    pose: "pose-cast",
    look: { ...figureLookFor(["Human"]), hair: "hair-short", outfit: "outfit-blazer", item: null, cloth: "slate-700" },
  },
  extras: [
    {
      x: 72,
      y: 84,
      scale: 0.9,
      pose: "pose-stride",
      look: { head: "head-lizard", hair: null, outfit: "outfit-rags", item: null, skin: "green-500", cloth: "blue-800" },
      layer: "subject",
    },
  ],
  layers: {
    far: [{ part: "skyline", x: 36, y: 62, scale: 0.8 }],
    mid: [
      { part: "street", x: 60, y: 64 },
      { part: "whitewashed-house", x: 84, y: 62, scale: 1.4, paint: "white" },
    ],
    near: [{ part: "zap", x: 38, y: 50, scale: 1.3 }],
  },
};

/** Right-house defender (Core #3, Common Unit): a small earnest guard in a bubble of pale shield-light on a suburban doorstep, map clutched tight, bright afternoon. */
const RIGHT_HOUSE_DEFENDER: SceneRecipe = {
  family: "core",
  sky: ["sky-300", "blue-200"],
  light: { x: 26, y: 20, r: 40, paint: "white", opacity: 0.5 },
  emblem: { x: 44, y: 63, size: 5, rotate: 0 },
  figure: {
    x: 42,
    y: 84,
    scale: 0.85,
    pose: "pose-guard",
    look: { ...figureLookFor(["Human"]), hair: "steel-helmet", outfit: "outfit-jacket", item: "item-map", cloth: "blue-700" },
  },
  layers: {
    far: [{ part: "hills", x: 50, y: 56 }],
    mid: [
      { part: "field", x: 50, y: 78 },
      { part: "whitewashed-house", x: 66, y: 78, scale: 2, paint: "blue-200" },
      { part: "hedge", x: 22, y: 80, scale: 1.2 },
      { part: "tree", x: 90, y: 76, scale: 1.5 },
    ],
    light: [
      { part: "halo", x: 42, y: 62, scale: 1.3, paint: "sky-300" },
      { part: "halo", x: 42, y: 62, scale: 0.6, paint: "white" },
    ],
    particles: [{ part: "motes", x: 42, y: 62, scale: 1, paint: "white" }],
  },
};

/** Gary the Gambler (Core #4, Common Unit): a grinning gambler in a loud checked waistcoat flipping silver coins high above a riverboat card table, warm lamplit cabin. */
const GARY_THE_GAMBLER: SceneRecipe = {
  family: "core",
  sky: ["slate-800", "wood-700"],
  light: { x: 50, y: 24, r: 40, paint: "orange-400", opacity: 0.45 },
  emblem: { x: 70, y: 66, size: 5, rotate: 0 },
  figure: {
    x: 36,
    y: 84,
    scale: 1,
    pose: "pose-raise",
    look: { ...figureLookFor(["Human"]), hair: "fedora", outfit: "outfit-waistcoat", item: "item-silver-coin", cloth: "red-500" },
  },
  layers: {
    sky: [{ part: "room-wall", x: 50, y: 78 }],
    far: [{ part: "window", x: 82, y: 56, scale: 1.4 }],
    mid: [
      { part: "wide-table", x: 70, y: 84, scale: 1.4 },
      { part: "card-fan", x: 70, y: 70, scale: 1.1 },
    ],
    near: [{ part: "coin", x: 44, y: 36, scale: 1.4, count: 3, spread: [8, 8], props: true }],
    light: [{ part: "rim-light", x: 36, y: 56, scale: 1.2, paint: "orange-400" }],
  },
};

/** Stockpile (Core #5, Common Spell): two cards float off a pantry shelf of hoarded crates into a resting hand as a soft green healing glow spreads, sunlit storeroom. */
const STOCKPILE: SceneRecipe = {
  family: "core",
  sky: ["slate-100", "stone-500"],
  light: { x: 24, y: 22, r: 40, paint: "white", opacity: 0.5 },
  emblem: { x: 28, y: 70, size: 5, rotate: 0 },
  layers: {
    sky: [{ part: "room-wall", x: 50, y: 76 }],
    far: [{ part: "shelf", x: 34, y: 76, scale: 1.8 }],
    mid: [
      { part: "crate", x: 20, y: 80, scale: 2.2, count: 2, spread: [6, 0] },
      { part: "sack", x: 84, y: 82, scale: 1.6 },
    ],
    subject: [
      { part: "playing-card", x: 52, y: 50, scale: 1.8 },
      { part: "playing-card", x: 60, y: 42, scale: 1.6, flip: -1 },
      { part: "open-hand", x: 70, y: 78, scale: 2.4, flip: -1 },
    ],
    light: [{ part: "heal-glow", x: 64, y: 62, scale: 2.4 }],
    particles: [{ part: "motes", x: 56, y: 56, scale: 1.2, paint: "green-200" }],
  },
};

/** Mana Well (Core #6, Common Field Spell): a stone wishing well in a glass-domed courtyard, blue mana bubbling up from its depths, silver coins glinting below, ivy on its rim, clear daylight. */
const MANA_WELL: SceneRecipe = {
  family: "core",
  sky: ["sky-400", "blue-200"],
  light: { x: 28, y: 20, r: 42, paint: "white", opacity: 0.5 },
  emblem: { x: 50, y: 66, size: 5, rotate: 0 },
  layers: {
    sky: [{ part: "cloud", x: 70, y: 22, scale: 1.2 }],
    far: [{ part: "glass-dome", x: 50, y: 74, scale: 1.8 }],
    mid: [
      { part: "cobbles", x: 50, y: 74 },
      { part: "bush", x: 14, y: 78, scale: 1.4 },
      { part: "bush", x: 88, y: 78, scale: 1.4 },
    ],
    subject: [
      { part: "well", x: 50, y: 82, scale: 2 },
      { part: "vine", x: 38, y: 80, scale: 1.2 },
    ],
    near: [{ part: "coin", x: 36, y: 92, scale: 1.3, count: 4, spread: [18, 3] }],
    light: [{ part: "halo", x: 50, y: 52, scale: 1.1, paint: "cyan-400" }],
    particles: [{ part: "bubble-column", x: 50, y: 44, scale: 2, count: 2, spread: [6, 4], paint: "cyan-400" }],
  },
};

/** Jewelosco Scarab (Core #7, Rare Unit): a jewel-shelled scarab rolling a neat ball of gems across rippled desert dunes, sunlight sparkling off its emerald carapace. */
const JEWELOSCO_SCARAB: SceneRecipe = {
  family: "core",
  sky: ["sky-400", "blue-200"],
  light: { x: 22, y: 22, r: 44, paint: "white", opacity: 0.55 },
  emblem: { x: 50, y: 58, size: 5, rotate: 0 },
  layers: {
    sky: [{ part: "sun-disc", x: 20, y: 20, scale: 2 }],
    far: [
      { part: "mesa", x: 80, y: 64, scale: 1.3 },
      { part: "mesa", x: 14, y: 66, scale: 0.8 },
    ],
    mid: [
      { part: "sand", x: 50, y: 64 },
      { part: "dust-cloud", x: 20, y: 76, scale: 1.6 },
    ],
    subject: [
      { part: "scarab", x: 38, y: 80, scale: 2 },
      { part: "gem-ball", x: 66, y: 68, scale: 2 },
    ],
    near: [{ part: "grass-tuft", x: 12, y: 92, scale: 1.6, count: 2, spread: [70, 2] }],
    light: [{ part: "rim-light", x: 40, y: 70, scale: 1.2, paint: "white" }],
    particles: [{ part: "sparks", x: 52, y: 62, scale: 1.8, count: 3, spread: [18, 10], paint: "white" }],
  },
};

/** Mr. Vanilla (Core #8, Common Unit): a plain sturdy man in a cream sweater holding a vanilla ice-cream cone in an empty white gallery, soft even light. */
const MR_VANILLA: SceneRecipe = {
  family: "core",
  sky: ["white", "slate-100"],
  light: { x: 50, y: 20, r: 46, paint: "white", opacity: 0.6 },
  emblem: { x: 50, y: 60, size: 5, rotate: 0 },
  figure: {
    x: 50,
    y: 86,
    scale: 1,
    pose: "pose-hold",
    look: { ...figureLookFor(["Human"]), hair: "hair-short", outfit: "outfit-sweater", item: "item-cone", cloth: "slate-100" },
  },
  layers: {
    sky: [{ part: "tile-wall", x: 50, y: 74 }],
    far: [
      { part: "window", x: 18, y: 56, scale: 1.4 },
      { part: "window", x: 82, y: 56, scale: 1.4 },
    ],
    mid: [{ part: "pedestal", x: 18, y: 84, scale: 1.4 }, { part: "bench", x: 80, y: 84, scale: 1.2 }],
    near: [{ part: "light-shaft", x: 20, y: 0, scale: 1.4, props: true }],
    light: [{ part: "halo", x: 50, y: 56, scale: 1.2, paint: "white" }],
    particles: [{ part: "motes", x: 50, y: 52, scale: 1.8, paint: "white" }],
  },
};

/** Moths to the Flame (Core #9, Rare Unit): a towering lantern-bodied creature with a blazing heart, giant moths bouncing off its glass chest, in a dusky reed marsh. */
const MOTHS_TO_THE_FLAME: SceneRecipe = {
  family: "core",
  sky: ["blue-800", "orange-400"],
  light: { x: 50, y: 56, r: 34, paint: "orange-400", opacity: 0.5 },
  emblem: { x: 50, y: 24, size: 5, rotate: 0 },
  figure: {
    x: 50,
    y: 87,
    scale: 1.1,
    pose: "pose-stand",
    look: { head: "head-lantern", hair: null, outfit: "outfit-lantern", item: null, skin: "brass-500", cloth: "brass-600" },
  },
  layers: {
    far: [{ part: "hills", x: 50, y: 60 }],
    mid: [
      { part: "sea", x: 50, y: 76 },
      { part: "reed", x: 16, y: 80, scale: 2, count: 3, spread: [8, 3] },
      { part: "reed", x: 84, y: 80, scale: 2, count: 3, spread: [8, 3] },
    ],
    subject: [
      { part: "moth", x: 30, y: 52, scale: 1.8 },
      { part: "moth", x: 70, y: 44, scale: 1.6, flip: -1 },
    ],
    near: [{ part: "reed", x: 8, y: 96, scale: 2.4 }],
    light: [{ part: "halo", x: 50, y: 58, scale: 1.4, paint: "orange-400" }],
    particles: [{ part: "embers", x: 50, y: 46, scale: 2, count: 2, spread: [10, 8] }],
  },
};

/** Rapid Replenish (Core #10, Common Spell): cards whirl out of a deck in a fast spiral into a cyclist's outstretched hand on a bright seaside promenade, motion streaks behind. */
const RAPID_REPLENISH: SceneRecipe = {
  family: "core",
  sky: ["sky-300", "slate-100"],
  light: { x: 22, y: 20, r: 40, paint: "white", opacity: 0.5 },
  emblem: { x: 22, y: 66, size: 5, rotate: 0 },
  figure: {
    x: 62,
    y: 78,
    scale: 0.9,
    pose: "pose-block",
    look: { ...figureLookFor(["Human"]), hair: "headband", outfit: "outfit-hoodie", item: null, cloth: "green-500" },
  },
  layers: {
    far: [{ part: "sea", x: 50, y: 56 }],
    mid: [
      { part: "boards", x: 50, y: 70 },
      { part: "fence", x: 50, y: 70, scale: 1.2 },
      { part: "deck", x: 22, y: 82, scale: 1.8 },
    ],
    subject: [{ part: "bicycle", x: 62, y: 86, scale: 2.2 }],
    near: [{ part: "streaks", x: 94, y: 68, scale: 1.6 }],
    light: [{ part: "card-ring", x: 48, y: 50, scale: 1.2 }],
    particles: [{ part: "gust", x: 40, y: 40, scale: 1.4 }],
  },
};

/** Tempo Timmy (Core #11, Common Unit): a scrappy boy with a stopwatch skids first over the starting line of a sunny village footrace, dust flying. The wooden sword is dropped: a figure holds one item, so the stopwatch is the one in his hand. */
const TEMPO_TIMMY: SceneRecipe = {
  family: "core",
  sky: ["sky-400", "blue-200"],
  light: { x: 24, y: 20, r: 40, paint: "white", opacity: 0.5 },
  emblem: { x: 56, y: 62, size: 5, rotate: 0 },
  figure: {
    x: 56,
    y: 82,
    scale: 0.9,
    pose: "pose-stride",
    look: { ...figureLookFor(["Human"]), hair: "headband", outfit: "outfit-tunic", item: "item-stopwatch", cloth: "red-500" },
  },
  layers: {
    far: [
      { part: "hills", x: 50, y: 56 },
      { part: "whitewashed-house", x: 22, y: 60, scale: 1.2 },
      { part: "barn", x: 84, y: 60, scale: 1.2 },
    ],
    mid: [
      { part: "road", x: 50, y: 62 },
      { part: "bunting", x: 50, y: 40, scale: 1 },
      { part: "crowd", x: 78, y: 68, scale: 0.9 },
    ],
    near: [{ part: "dust-cloud", x: 44, y: 86, scale: 1.5, count: 2, spread: [8, 2], props: true }],
    light: [{ part: "rim-light", x: 56, y: 56, scale: 1.1, paint: "white" }],
  },
};

/** Duplicating Felinors (Core #12, Rare Unit): two identical cat-folk in matching striped scarves high-five before a tall dressing-room mirror, a third copy climbing out of the glass, bright vanity bulbs. */
const DUPLICATING_FELINORS: SceneRecipe = {
  family: "core",
  sky: ["slate-100", "stone-500"],
  light: { x: 50, y: 20, r: 40, paint: "orange-400", opacity: 0.35 },
  emblem: { x: 76, y: 50, size: 5, rotate: 0 },
  figure: {
    x: 26,
    y: 84,
    scale: 1,
    pose: "pose-point",
    look: { ...figureLookFor(["Felinor"]), outfit: "outfit-stripes", item: null, cloth: "blue-600" },
  },
  extras: [
    {
      x: 52,
      y: 84,
      scale: 1,
      flip: -1,
      pose: "pose-point",
      look: { ...figureLookFor(["Felinor"]), outfit: "outfit-stripes", item: null, cloth: "blue-600" },
      layer: "subject",
    },
    {
      x: 78,
      y: 80,
      scale: 0.75,
      flip: -1,
      pose: "pose-lunge",
      look: { ...figureLookFor(["Felinor"]), outfit: "outfit-stripes", item: null, cloth: "blue-600" },
      layer: "subject",
    },
  ],
  layers: {
    sky: [{ part: "room-wall", x: 50, y: 82 }],
    far: [{ part: "bulbs", x: 50, y: 20, scale: 1.8 }],
    mid: [{ part: "mirror", x: 78, y: 82, scale: 2.4 }],
    near: [{ part: "sparks", x: 39, y: 46, scale: 1.4, count: 2, spread: [3, 3], paint: "white", props: true }],
    light: [{ part: "halo", x: 39, y: 46, scale: 0.7, paint: "white" }],
  },
};

/** Jlockeed Shredder-10 (Core #13, Common Unit): a hulking four-armed war machine with spinning saw-blades rolls off a spotless factory assembly line, sparks spraying, cold white floodlights. Its four arms are two saws and the figure's own two. */
const JLOCKEED_SHREDDER_10: SceneRecipe = {
  family: "core",
  sky: ["slate-100", "slate-400"],
  light: { x: 28, y: 16, r: 42, paint: "white", opacity: 0.6 },
  emblem: { x: 50, y: 56, size: 5, rotate: 0 },
  figure: {
    x: 50,
    y: 80,
    scale: 1.05,
    pose: "pose-guard",
    look: { head: "head-robot", hair: "visor", outfit: "outfit-suit", item: null, skin: "slate-400", cloth: "slate-700" },
  },
  layers: {
    sky: [{ part: "tile-wall", x: 50, y: 78 }],
    far: [{ part: "spotlight", x: 24, y: 2, scale: 1.2, flip: 1 }],
    mid: [{ part: "conveyor", x: 50, y: 82, scale: 1.7 }],
    subject: [
      { part: "saw", x: 29, y: 54, scale: 2.4 },
      { part: "saw", x: 71, y: 54, scale: 2.4 },
    ],
    near: [{ part: "cog", x: 90, y: 92, scale: 1.2 }],
    light: [{ part: "rim-light", x: 50, y: 56, scale: 1.2, paint: "white" }],
    particles: [{ part: "sparks", x: 30, y: 58, scale: 1.6, count: 2, spread: [40, 4] }],
  },
};

/** Jlockeed's Weapons (Core #14, Common Field Spell): a gleaming arms showroom of rifles racked under glass, a tiny pistol clipped to each, spotlights on polished white floors and empty display stands. The pistols are small rifles. */
const JLOCKEEDS_WEAPONS: SceneRecipe = {
  family: "core",
  sky: ["slate-100", "slate-400"],
  light: { x: 30, y: 20, r: 44, paint: "white", opacity: 0.55 },
  emblem: { x: 50, y: 84, size: 5, rotate: 0 },
  layers: {
    sky: [{ part: "tile-wall", x: 50, y: 72 }],
    far: [{ part: "spotlight", x: 50, y: 2, scale: 1.2 }],
    mid: [
      { part: "tile-floor", x: 50, y: 72 },
      { part: "shelf", x: 50, y: 74, scale: 2.4 },
      { part: "pedestal", x: 12, y: 84, scale: 1.5 },
    ],
    subject: [
      { part: "rifle", x: 50, y: 40, scale: 1.7, paint: "slate-700" },
      { part: "rifle", x: 50, y: 55, scale: 1.7, paint: "slate-700" },
      { part: "rifle", x: 50, y: 70, scale: 1.7, paint: "slate-700" },
      { part: "rifle", x: 74, y: 40, scale: 0.5 },
      { part: "rifle", x: 74, y: 55, scale: 0.5 },
      { part: "rifle", x: 74, y: 70, scale: 0.5 },
    ],
    near: [{ part: "glass-wall", x: 50, y: 82, scale: 2.4 }],
  },
};

/** Me and Mr Token (Core #15, Common Unit): a lazy boy lounges in a deckchair on a shop porch while his round little token-buddy sprints a heavy crate up a busy market lane. */
const ME_AND_MR_TOKEN: SceneRecipe = {
  family: "core",
  sky: ["sky-300", "blue-200"],
  light: { x: 24, y: 18, r: 40, paint: "white", opacity: 0.5 },
  emblem: { x: 30, y: 60, size: 5, rotate: 0 },
  figure: {
    x: 30,
    y: 80,
    scale: 0.8,
    pose: "pose-perch",
    look: { ...figureLookFor(["Human"]), hair: "cap-feather", outfit: "outfit-hoodie", item: null, cloth: "green-500" },
  },
  layers: {
    far: [{ part: "townhouse", x: 62, y: 58, scale: 1.4 }],
    mid: [
      { part: "street", x: 70, y: 62 },
      { part: "counter", x: 20, y: 82, scale: 1.4 },
      { part: "bunting", x: 50, y: 34, scale: 1.2 },
    ],
    subject: [
      { part: "deckchair", x: 30, y: 82, scale: 2.2 },
      { part: "rush-token", x: 66, y: 82, scale: 1.8, flip: 1 },
      { part: "crate", x: 70, y: 62, scale: 3.4 },
    ],
    near: [{ part: "streaks", x: 52, y: 84, scale: 1.4, count: 2, spread: [0, 4] }],
    particles: [{ part: "dust-cloud", x: 56, y: 86, scale: 1.2 }],
  },
};

/** Hit Job (Core #16, Common Spell): a gloved assassin in a grey suit lunges from a marble hotel corridor, dagger flashing, the victim's hat tumbling, cold clinical white light. */
const HIT_JOB: SceneRecipe = {
  family: "core",
  sky: ["slate-100", "slate-400"],
  light: { x: 30, y: 16, r: 42, paint: "white", opacity: 0.6 },
  emblem: { x: 40, y: 58, size: 5, rotate: 0 },
  figure: {
    x: 40,
    y: 84,
    scale: 1,
    pose: "pose-lunge",
    look: { ...figureLookFor(["Human"]), hair: "hair-short", outfit: "outfit-suit", item: "item-knife", cloth: "slate-700" },
  },
  layers: {
    sky: [{ part: "room-wall", x: 50, y: 74 }],
    far: [
      { part: "framed-door", x: 82, y: 74, scale: 2 },
      { part: "column", x: 12, y: 78, scale: 1.4 },
    ],
    mid: [{ part: "checker-floor", x: 50, y: 74 }],
    near: [{ part: "hat", x: 76, y: 44, scale: 2 }],
    light: [{ part: "burst", x: 60, y: 54, scale: 1, paint: "white" }],
    particles: [{ part: "sparks", x: 60, y: 54, scale: 1.2, paint: "white" }],
  },
};

/** Flood (Core #17, Rare Spell): a huge clear wave crashes across a sun-baked fortress courtyard of lanes, tossing soldiers and beasts back the way they came, spray sparkling. */
const FLOOD: SceneRecipe = {
  family: "core",
  sky: ["sky-400", "blue-200"],
  light: { x: 24, y: 20, r: 42, paint: "white", opacity: 0.5 },
  emblem: { x: 36, y: 74, size: 5, rotate: 0 },
  layers: {
    sky: [{ part: "cloud", x: 72, y: 22, scale: 1.2 }],
    far: [
      { part: "banner", x: 14, y: 68, scale: 1.4 },
      { part: "fortress", x: 66, y: 62, scale: 1.8 },
      { part: "battlement", x: 70, y: 66, scale: 1.4 },
    ],
    mid: [{ part: "cobbles", x: 50, y: 68 }],
    subject: [
      { part: "wave", x: 22, y: 84, scale: 1.6 },
      { part: "soldier-fallen", x: 74, y: 56, scale: 1.8 },
      { part: "beast", x: 86, y: 70, scale: 1.2, flip: -1 },
      { part: "soldier", x: 66, y: 76, scale: 1.4 },
    ],
    near: [{ part: "spray", x: 54, y: 68, scale: 1.6, count: 2, spread: [10, 8] }],
    light: [{ part: "rim-light", x: 40, y: 58, scale: 1.4, paint: "white" }],
    particles: [{ part: "drops", x: 60, y: 50, scale: 2, count: 2, spread: [14, 8] }],
  },
};

/** Bread and Butter (Core #18, Epic Field Trap): foreground, a toaster-shaped trap snapping open on a farmhouse table as a big loaf-creature leaps out; middle, a butter dish and scattered crumbs; background, a sunny kitchen window over swaying wheat. The butter dish is a small brass crate. */
const BREAD_AND_BUTTER: SceneRecipe = {
  family: "core",
  sky: ["sky-300", "blue-200"],
  light: { x: 26, y: 18, r: 44, paint: "white", opacity: 0.55 },
  emblem: { x: 38, y: 62, size: 5, rotate: 0 },
  layers: {
    sky: [{ part: "room-wall", x: 50, y: 74 }],
    far: [
      { part: "window", x: 50, y: 52, scale: 2.8 },
      { part: "wheat", x: 40, y: 56, scale: 1.6, count: 4, spread: [12, 2] },
    ],
    mid: [
      { part: "wide-table", x: 50, y: 92, scale: 2.2 },
      { part: "crate", x: 80, y: 76, scale: 2.6, paint: "brass-500", props: true },
      { part: "motes", x: 62, y: 72, scale: 1.2, paint: "brass-500", props: true },
    ],
    subject: [{ part: "loaf", x: 48, y: 50, scale: 2 }],
    near: [{ part: "toaster", x: 36, y: 76, scale: 1.9 }],
    light: [{ part: "halo", x: 48, y: 58, scale: 1.1, paint: "white" }],
    particles: [{ part: "motes", x: 52, y: 60, scale: 1.6, count: 2, spread: [14, 8], paint: "brass-500" }],
  },
};

/** Midrange Menace (Core #19, Common Unit): a broad horned brute planted mid-span on a narrow rope bridge over a misty gorge, arms spread to block, bandages knitting shut in green light. */
const MIDRANGE_MENACE: SceneRecipe = {
  family: "core",
  sky: ["sky-300", "slate-100"],
  light: { x: 26, y: 20, r: 40, paint: "white", opacity: 0.5 },
  emblem: { x: 50, y: 58, size: 5, rotate: 0 },
  figure: {
    x: 50,
    y: 80,
    scale: 1,
    pose: "pose-block",
    look: { head: "head-human", hair: "horns", outfit: "outfit-rags", item: null, skin: "orange-400", cloth: "slate-100" },
  },
  layers: {
    far: [{ part: "hills", x: 50, y: 60 }],
    mid: [
      { part: "cliff", x: 18, y: 88, scale: 1.4 },
      { part: "cliff", x: 82, y: 88, scale: 1.4, flip: -1 },
      { part: "mist", x: 50, y: 88, scale: 1.4, count: 2, spread: [20, 3] },
    ],
    subject: [{ part: "rope-bridge", x: 50, y: 80, scale: 1.6 }],
    light: [{ part: "heal-glow", x: 50, y: 58, scale: 2.6 }],
    particles: [{ part: "motes", x: 50, y: 56, scale: 1.4, paint: "green-200" }],
  },
};

/** Pointmaster (Core #20, Common Unit): a lanky fencer in white lunges, a needle-thin rapier tip just touching a straw target's centre, in a bright fencing hall with a chequered floor. */
const POINTMASTER: SceneRecipe = {
  family: "core",
  sky: ["slate-100", "blue-200"],
  light: { x: 24, y: 18, r: 42, paint: "white", opacity: 0.55 },
  emblem: { x: 32, y: 58, size: 5, rotate: 0 },
  figure: {
    x: 32,
    y: 82,
    scale: 1,
    pose: "pose-lunge",
    look: { ...figureLookFor(["Human"]), hair: "face-mask", outfit: "outfit-jacket", item: "item-rapier", cloth: "white" },
  },
  layers: {
    sky: [{ part: "room-wall", x: 50, y: 68 }],
    mid: [
      { part: "checker-floor", x: 50, y: 68 },
      { part: "banner", x: 86, y: 70, scale: 1.4 },
    ],
    subject: [{ part: "target", x: 72, y: 82, scale: 2.2 }],
    light: [{ part: "light-shaft", x: 20, y: 0, scale: 1.2 }],
    particles: [{ part: "sparks", x: 66, y: 62, scale: 0.8, paint: "white" }],
  },
};

/** Hinder (Core #21, Common Spell): a blue mana crystal cracks in a rival mage's hands as a card slips from his grip, in a quiet chess salon, startled cool light. */
const HINDER: SceneRecipe = {
  family: "core",
  sky: ["slate-800", "slate-400"],
  light: { x: 50, y: 40, r: 36, paint: "cyan-400", opacity: 0.4 },
  emblem: { x: 36, y: 62, size: 5, rotate: 0 },
  figure: {
    x: 36,
    y: 86,
    scale: 1,
    pose: "pose-hold",
    look: { ...figureLookFor(["Human"]), hair: "hat-pointed", outfit: "outfit-gown", item: null, cloth: "violet-700" },
  },
  layers: {
    sky: [{ part: "room-wall", x: 50, y: 74 }],
    far: [{ part: "bookcase", x: 84, y: 74, scale: 1.8 }],
    mid: [
      { part: "wide-table", x: 78, y: 86, scale: 1.2 },
      { part: "chess-piece", x: 74, y: 72, scale: 2, count: 3, spread: [8, 0] },
    ],
    subject: [
      { part: "crystal", x: 42, y: 56, scale: 2.2, paint: "cyan-400" },
      { part: "playing-card", x: 62, y: 62, scale: 2 },
    ],
    near: [{ part: "shards", x: 44, y: 54, scale: 1.6 }],
    light: [{ part: "burst", x: 42, y: 54, scale: 1.2, paint: "cyan-400" }],
  },
};

/** Carnivorous Cube (Core #22, Epic Unit): foreground, a translucent green gelatin cube with jagged teeth and a startled knight floating inside; middle, dropped helmets on a crystal cavern floor; background, glittering quartz arches and a second knight backing away. */
const CARNIVOROUS_CUBE: SceneRecipe = {
  family: "core",
  sky: ["slate-900", "blue-900"],
  light: { x: 28, y: 22, r: 44, paint: "cyan-400", opacity: 0.4 },
  emblem: { x: 22, y: 76, size: 5, rotate: 0 },
  extras: [
    {
      x: 50,
      y: 72,
      scale: 0.6,
      pose: "pose-raise",
      look: { ...figureLookFor(["Human"]), hair: "steel-helmet", outfit: "outfit-layered-plate", item: null, cloth: "slate-400" },
      layer: "subject",
    },
    {
      x: 82,
      y: 66,
      scale: 0.55,
      flip: -1,
      pose: "pose-guard",
      look: { ...figureLookFor(["Human"]), hair: "steel-helmet", outfit: "outfit-layered-plate", item: "item-sword", cloth: "slate-400" },
      layer: "far",
    },
  ],
  layers: {
    far: [
      { part: "arch", x: 24, y: 66, scale: 1.8, paint: "slate-100" },
      { part: "arch", x: 78, y: 66, scale: 1.8, paint: "slate-100" },
      { part: "crystal", x: 50, y: 66, scale: 1.8, count: 4, spread: [32, 2] },
    ],
    mid: [
      { part: "cavern", x: 50, y: 74 },
      { part: "helmet", x: 20, y: 86, scale: 2.4, count: 2, spread: [6, 3] },
      { part: "helmet", x: 80, y: 88, scale: 2.4, flip: -1 },
    ],
    subject: [{ part: "cube", x: 50, y: 84, scale: 2 }],
    near: [{ part: "drops", x: 36, y: 90, scale: 1.8, paint: "green-400" }],
    light: [{ part: "rim-light", x: 50, y: 58, scale: 1.4, paint: "green-200" }],
  },
};

/** Reoccurring Dream (Core #23, Rare Spell): the same bedroom door floats in a starry dream sky beside a sleeper's pillow, one card in her hand starting to shimmer, soft violet moonlight. The sleeper is a bed and her pillow, her hand a card above it. */
const REOCCURRING_DREAM: SceneRecipe = {
  family: "core",
  sky: ["slate-900", "violet-700"],
  light: { x: 26, y: 20, r: 40, paint: "violet-300", opacity: 0.45 },
  emblem: { x: 36, y: 58, size: 5, rotate: 0 },
  layers: {
    sky: [
      { part: "stars", x: 50, y: 24, scale: 2.2, count: 2, spread: [12, 6] },
      { part: "moon", x: 82, y: 24, scale: 1.4 },
    ],
    far: [{ part: "framed-door", x: 60, y: 60, scale: 1.4 }],
    mid: [
      { part: "cloud", x: 30, y: 82, scale: 2, count: 2, spread: [30, 3] },
      { part: "framed-door", x: 36, y: 76, scale: 2.5 },
    ],
    subject: [
      { part: "bed", x: 72, y: 82, scale: 1.7 },
      { part: "playing-card", x: 74, y: 56, scale: 2.2 },
    ],
    light: [{ part: "halo", x: 74, y: 58, scale: 0.9, paint: "violet-300" }],
    particles: [{ part: "sparks", x: 74, y: 56, scale: 1.6, count: 2, spread: [6, 6], paint: "white" }],
  },
};

/** Efficiency Dividend (Core #24, Epic Spell): foreground, an office worker wringing the last blue mana from a sponge into three cups, one burning, one healing, one glowing; middle, a tidy glass cubicle; background, high-rise towers in morning sun. The sponge is a cyan crate. */
const EFFICIENCY_DIVIDEND: SceneRecipe = {
  family: "core",
  sky: ["sky-300", "slate-100"],
  light: { x: 24, y: 18, r: 44, paint: "white", opacity: 0.55 },
  emblem: { x: 38, y: 62, size: 5, rotate: 0 },
  figure: {
    x: 32,
    y: 86,
    scale: 0.95,
    pose: "pose-hold",
    look: { ...figureLookFor(["Human"]), hair: "hair-short", outfit: "outfit-blazer", item: null, cloth: "blue-700" },
  },
  layers: {
    sky: [{ part: "skyline", x: 50, y: 58, scale: 1.3 }],
    far: [
      { part: "tower", x: 88, y: 58, scale: 1.2 },
      { part: "tower", x: 12, y: 60, scale: 1 },
    ],
    mid: [
      { part: "glass-wall", x: 64, y: 78, scale: 2.4 },
      { part: "chair", x: 88, y: 86, scale: 2 },
          ],
    subject: [
      { part: "crate", x: 38, y: 58, scale: 1.5, paint: "cyan-400" },
      { part: "drops", x: 46, y: 70, scale: 1.6, paint: "cyan-400" },
    ],
    near: [
      { part: "cup", x: 56, y: 84, scale: 4 },
      { part: "cup", x: 68, y: 84, scale: 4 },
      { part: "cup", x: 80, y: 84, scale: 4 },
    ],
    light: [
      { part: "heal-glow", x: 68, y: 76, scale: 1.8 },
      { part: "halo", x: 80, y: 76, scale: 0.8, paint: "cyan-400" },
    ],
    particles: [{ part: "flame", x: 56, y: 80, scale: 2 }],
  },
};
/** 4-mana 7/7 (Core #25, Common Unit): a massive armoured stone giant steps out of a huge plain shipping crate on a loading dock, packing straw falling, clear daylight. */
const FOUR_MANA_SEVEN_SEVEN: SceneRecipe = {
  family: "core",
  sky: ["sky-400", "blue-200"],
  light: { x: 24, y: 20, r: 42, paint: "white", opacity: 0.5 },
  emblem: { x: 62, y: 56, size: 6, rotate: 0 },
  figure: {
    x: 64,
    y: 86,
    scale: 1.05,
    pose: "pose-lunge",
    look: { head: "head-stone", hair: null, outfit: "outfit-rock", item: null, skin: "slate-400", cloth: "stone-500" },
  },
  layers: {
    far: [{ part: "sea", x: 50, y: 58 }],
    mid: [
      { part: "boards", x: 50, y: 70 },
      { part: "crate-big", x: 34, y: 82, scale: 1.4 },
      { part: "crate", x: 90, y: 74, scale: 2.4 },
    ],
    near: [{ part: "motes", x: 50, y: 70, scale: 1.8, count: 2, spread: [14, 8], paint: "brass-500" }],
    light: [{ part: "rim-light", x: 64, y: 58, scale: 1.4, paint: "white" }],
  },
};

/** Glowy Jelly Bean (Core #26, Rare Spell): a single glowing lime-green jelly bean drops onto a card held in a palm, the card lighting up, on a sparkling candy-shop counter in clean bright light. */
const GLOWY_JELLY_BEAN: SceneRecipe = {
  family: "core",
  sky: ["sky-300", "slate-100"],
  light: { x: 26, y: 20, r: 40, paint: "white", opacity: 0.55 },
  emblem: { x: 50, y: 82, size: 5, rotate: 0 },
  layers: {
    sky: [{ part: "room-wall", x: 50, y: 74 }],
    far: [
      { part: "shelf", x: 20, y: 70, scale: 1.6 },
      { part: "bunting", x: 66, y: 30, scale: 1.2 },
    ],
    mid: [
      { part: "counter", x: 50, y: 92, scale: 2.4 },
      { part: "jar", x: 18, y: 78, scale: 2.6 },
      { part: "jar", x: 84, y: 78, scale: 2.6 },
    ],
    subject: [
      { part: "open-hand", x: 52, y: 84, scale: 3 },
      { part: "playing-card", x: 52, y: 66, scale: 2.6 },
      { part: "jelly-bean", x: 52, y: 38, scale: 3.6, paint: "lime-400" },
    ],
    light: [{ part: "halo", x: 52, y: 52, scale: 1, paint: "lime-400" }],
    particles: [{ part: "sparks", x: 52, y: 54, scale: 1.8, count: 3, spread: [14, 10], paint: "white" }],
  },
};

/** Blood Ridden Glowy Jelly Bean (Core #27, Rare Spell): a crimson jelly bean pulses inside a glass apothecary jar, red drops seeping through the glass as a card beside it brightens, on a spotless white shelf. */
const BLOOD_RIDDEN_GLOWY_JELLY_BEAN: SceneRecipe = {
  family: "core",
  sky: ["white", "slate-100"],
  light: { x: 30, y: 20, r: 40, paint: "white", opacity: 0.6 },
  emblem: { x: 50, y: 82, size: 5, rotate: 0 },
  layers: {
    sky: [{ part: "tile-wall", x: 50, y: 82 }],
    far: [
      { part: "shelf", x: 50, y: 84, scale: 3.4 },
      { part: "vial", x: 74, y: 62, scale: 3, count: 2, spread: [8, 0] },
    ],
    mid: [
      { part: "jar", x: 16, y: 80, scale: 3 },
      { part: "jar", x: 28, y: 80, scale: 2.4 },
    ],
    subject: [
      { part: "jar", x: 46, y: 82, scale: 6 },
      { part: "jelly-bean", x: 46, y: 62, scale: 4, paint: "red-500" },
      { part: "drops", x: 58, y: 74, scale: 1.4, count: 2, spread: [4, 6], paint: "red-500" },
      { part: "playing-card", x: 80, y: 70, scale: 3 },
    ],
    light: [
      { part: "halo", x: 46, y: 62, scale: 0.9, paint: "red-400" },
      { part: "halo", x: 80, y: 68, scale: 0.8, paint: "white" },
    ],
  },
};

/** Knockoff Temu Glowy Jelly Bean (Core #28, Rare Spell): a lumpy two-toned jelly bean glows patchily as it pops from a battered over-taped parcel on a post-office counter, the cards beside it flickering. */
const KNOCKOFF_TEMU_GLOWY_JELLY_BEAN: SceneRecipe = {
  family: "core",
  sky: ["slate-100", "stone-500"],
  light: { x: 28, y: 20, r: 38, paint: "white", opacity: 0.5 },
  emblem: { x: 50, y: 82, size: 5, rotate: 0 },
  layers: {
    sky: [{ part: "room-wall", x: 50, y: 76 }],
    far: [{ part: "window", x: 80, y: 58, scale: 1.6 }],
    mid: [
      { part: "counter", x: 50, y: 92, scale: 2.4 },
      { part: "envelope", x: 80, y: 80, scale: 2.4 },
      { part: "bin", x: 12, y: 84, scale: 1.6 },
    ],
    subject: [
      { part: "parcel", x: 48, y: 82, scale: 3.6 },
      { part: "jelly-bean", x: 46, y: 52, scale: 4, paint: "lime-400" },
      { part: "jelly-bean", x: 52, y: 55, scale: 3.4, paint: "orange-400" },
      { part: "playing-card", x: 20, y: 66, scale: 2.4 },
      { part: "playing-card", x: 28, y: 72, scale: 2, flip: -1 },
    ],
    light: [{ part: "burst", x: 49, y: 54, scale: 1, paint: "lime-200" }],
    particles: [{ part: "sparks", x: 24, y: 64, scale: 1.4, count: 2, spread: [6, 6], paint: "white" }],
  },
};

/** GIGA Glowy Jelly Bean (Core #29, Rare Spell): a jelly bean as big as a boulder rolls down a candy-striped mountain toward a fan of five cards, each lighting up, under a bright blue sky. */
const GIGA_GLOWY_JELLY_BEAN: SceneRecipe = {
  family: "core",
  sky: ["sky-400", "blue-200"],
  light: { x: 24, y: 18, r: 42, paint: "white", opacity: 0.5 },
  emblem: { x: 28, y: 80, size: 5, rotate: 0 },
  layers: {
    sky: [{ part: "cloud", x: 30, y: 24, scale: 1.4 }],
    far: [
      { part: "tree", x: 90, y: 78, scale: 1.4 },
      { part: "candy-mountain", x: 66, y: 78, scale: 1.6 },
    ],
    mid: [{ part: "field", x: 50, y: 78 }],
    subject: [
      { part: "jelly-bean", x: 52, y: 54, scale: 8, paint: "lime-400" },
      { part: "card-fan", x: 24, y: 86, scale: 3.2 },
    ],
    near: [{ part: "streaks", x: 76, y: 46, scale: 1.4 }],
    light: [{ part: "heal-glow", x: 24, y: 78, scale: 3, }],
    particles: [{ part: "dust-cloud", x: 70, y: 70, scale: 1.4 }, { part: "sparks", x: 24, y: 74, scale: 2, count: 2, spread: [10, 4], paint: "white" }],
  },
};

/** Archivist (Core #30, Rare Unit): a bespectacled archivist atop a tall rolling ladder hugs a huge tome in one arm in an airy white library. The tiny booklet in his other hand is dropped: one item slot, so the tome is a part in front of him. */
const ARCHIVIST: SceneRecipe = {
  family: "core",
  sky: ["white", "slate-100"],
  light: { x: 26, y: 18, r: 42, paint: "white", opacity: 0.6 },
  emblem: { x: 50, y: 46, size: 5, rotate: 0 },
  figure: {
    x: 50,
    y: 66,
    scale: 0.65,
    pose: "pose-hold",
    look: { ...figureLookFor(["Human"]), hair: "spectacles", outfit: "outfit-tutor-cardigan", item: null, cloth: "green-700" },
  },
  layers: {
    sky: [{ part: "room-wall", x: 50, y: 82 }],
    far: [{ part: "window", x: 50, y: 40, scale: 1.6 }],
    mid: [
      { part: "bookcase", x: 16, y: 84, scale: 2.1 },
      { part: "bookcase", x: 84, y: 84, scale: 2.1 },
      { part: "tile-floor", x: 50, y: 82 },
    ],
    subject: [{ part: "ladder", x: 50, y: 88, scale: 1.9 }],
    near: [{ part: "closed-tome", x: 52, y: 58, scale: 2.6 }],
    light: [{ part: "light-shaft", x: 24, y: 0, scale: 1.2 }],
  },
};

/** KY's Math Equation (Core #31, Rare Spell): KY, a chalk-dusted tutor in a cardigan, flings a spiral of ever-larger squares off a blackboard on a breezy rooftop, striking in a white burst. */
const KYS_MATH_EQUATION: SceneRecipe = {
  family: "core",
  sky: ["sky-400", "blue-200"],
  light: { x: 24, y: 18, r: 42, paint: "white", opacity: 0.5 },
  emblem: { x: 20, y: 62, size: 5, rotate: 0 },
  figure: { x: 38, y: 84, scale: 1, pose: "pose-cast", look: KY_TUTOR_LOOK },
  layers: {
    sky: [{ part: "cloud", x: 62, y: 24, scale: 1.4 }],
    far: [{ part: "skyline", x: 58, y: 66, scale: 1.1 }],
    mid: [
      { part: "stone-wall", x: 50, y: 82, scale: 2 },
      { part: "blackboard", x: 20, y: 76, scale: 1.4 },
    ],
    subject: [{ part: "squares", x: 52, y: 50, scale: 1.2 }],
    near: [{ part: "gust", x: 80, y: 30, scale: 1.4 }],
    light: [{ part: "burst", x: 84, y: 54, scale: 1.8, paint: "white" }],
    particles: [{ part: "sparks", x: 82, y: 52, scale: 2, count: 2, spread: [6, 6], paint: "white" }],
  },
};

/** Prem Panther (Core #32, Rare Unit): a sleek black panther bounds through tall swaying savanna grass, eyes gleaming green, a blur of motion, in crisp late-afternoon light. */
const PREM_PANTHER: SceneRecipe = {
  family: "core",
  sky: ["sky-300", "orange-400"],
  light: { x: 20, y: 24, r: 44, paint: "orange-400", opacity: 0.45 },
  emblem: { x: 50, y: 66, size: 5, rotate: 0 },
  layers: {
    sky: [{ part: "sun-disc", x: 18, y: 24, scale: 1.8 }],
    far: [{ part: "hills", x: 50, y: 62 }, { part: "tree", x: 82, y: 64, scale: 1.4 }],
    mid: [
      { part: "field", x: 50, y: 66 },
      { part: "bush", x: 90, y: 72, scale: 1.4 },
      { part: "tall-grass", x: 20, y: 76, scale: 2, count: 5, spread: [20, 3] },
    ],
    subject: [{ part: "panther", x: 52, y: 56, scale: 2 }],
    near: [{ part: "tall-grass", x: 12, y: 94, scale: 3, count: 5, spread: [80, 3] }],
    light: [{ part: "streaks", x: 76, y: 56, scale: 1.4 }],
    particles: [{ part: "leaves", x: 40, y: 54, scale: 1.6 }],
  },
};

/** Unstable Clone Machine (Core #33, Rare Field Spell): a glass-walled lab with a humming twin-chamber cloning pod, flickering duplicate cards spilling from its hatch across a gleaming tiled floor. */
const UNSTABLE_CLONE_MACHINE: SceneRecipe = {
  family: "core",
  sky: ["white", "blue-200"],
  light: { x: 40, y: 30, r: 40, paint: "cyan-400", opacity: 0.35 },
  emblem: { x: 34, y: 66, size: 5, rotate: 0 },
  layers: {
    sky: [{ part: "tile-wall", x: 50, y: 74 }],
    far: [
      { part: "glass-wall", x: 12, y: 76, scale: 1.8 },
      { part: "glass-wall", x: 88, y: 76, scale: 1.8, flip: -1 },
    ],
    mid: [{ part: "tile-floor", x: 50, y: 74 }],
    subject: [
      { part: "pod", x: 34, y: 80, scale: 1.7 },
      { part: "hatch", x: 74, y: 80, scale: 1.4 },
      { part: "playing-card", x: 70, y: 66, scale: 2.2, count: 4, spread: [14, 8] },
    ],
    light: [{ part: "zap", x: 36, y: 52, scale: 0.7 }],
    particles: [{ part: "sparks", x: 66, y: 62, scale: 1.8, count: 2, spread: [10, 6], paint: "cyan-400" }],
  },
};

/** Collateral Damage (Core #34, Rare Spell): a wrecking-ball blast erases a watchtower into blue sparks, its shockwave tearing one card off a nearby deck, on a dusty bright construction site. */
const COLLATERAL_DAMAGE: SceneRecipe = {
  family: "core",
  sky: ["sky-300", "slate-100"],
  light: { x: 24, y: 18, r: 40, paint: "white", opacity: 0.5 },
  emblem: { x: 84, y: 82, size: 5, rotate: 0 },
  layers: {
    far: [{ part: "hills", x: 50, y: 62 }],
    mid: [
      { part: "ramp", x: 14, y: 84, scale: 1.4 },
      { part: "fence", x: 40, y: 80, scale: 1.2 },
      { part: "tower", x: 62, y: 80, scale: 1.8 },
      { part: "crate", x: 36, y: 84, scale: 2.4, count: 2, spread: [8, 0] },
    ],
    subject: [
      { part: "wrecking-ball", x: 26, y: 12, scale: 2.1 },
      { part: "deck", x: 86, y: 84, scale: 3 },
      { part: "playing-card", x: 84, y: 62, scale: 2.4 },
    ],
    light: [{ part: "blast-rings", x: 50, y: 56, scale: 2.2, paint: "cyan-400" }],
    particles: [
      { part: "sparks", x: 62, y: 54, scale: 2.4, count: 3, spread: [10, 14], paint: "cyan-400" },
      { part: "dust-cloud", x: 54, y: 78, scale: 1.8 },
    ],
  },
};

/** Lunar Eclipse (Core #35, Rare Spell): the moon slides into shadow above a hilltop stone circle, a thin silver beam striking a soldier as the field dims, under a crisp starry sky. */
const LUNAR_ECLIPSE: SceneRecipe = {
  family: "core",
  sky: ["slate-900", "blue-900"],
  light: { x: 28, y: 22, r: 40, paint: "slate-100", opacity: 0.35 },
  emblem: { x: 52, y: 66, size: 5, rotate: 0 },
  layers: {
    sky: [
      { part: "stars", x: 50, y: 26, scale: 2.2, count: 2, spread: [14, 6] },
      { part: "moon", x: 28, y: 24, scale: 1.8 },
      { part: "eclipse", x: 28, y: 24, scale: 1.8 },
    ],
    far: [{ part: "hills", x: 50, y: 66 }],
    mid: [
      { part: "field", x: 50, y: 76 },
      { part: "ring-stone", x: 22, y: 76, scale: 2.4, count: 5, spread: [32, 3] },
    ],
    subject: [{ part: "soldier", x: 54, y: 82, scale: 1.8 }],
    light: [{ part: "light-shaft", x: 36, y: 28, scale: 1.4, paint: "slate-100" }],
    particles: [{ part: "motes", x: 54, y: 62, scale: 1.2, paint: "slate-100" }],
  },
};

/** Magic Jammed (Core #36, Rare Spell): a spellbook slammed shut and padlocked in crackling purple chains, sparks spitting from its jammed pages, on a cluttered wizard's-tower workbench. */
const MAGIC_JAMMED: SceneRecipe = {
  family: "core",
  sky: ["slate-800", "violet-700"],
  light: { x: 50, y: 50, r: 36, paint: "violet-500", opacity: 0.45 },
  emblem: { x: 36, y: 74, size: 5, rotate: 0 },
  layers: {
    sky: [{ part: "room-wall", x: 50, y: 72 }],
    far: [{ part: "window", x: 80, y: 50, scale: 1.8 }],
    mid: [
      { part: "wide-table", x: 50, y: 92, scale: 2.6 },
      { part: "taper", x: 14, y: 70, scale: 3 },
      { part: "sand-glass", x: 82, y: 70, scale: 3 },
      { part: "vial", x: 88, y: 70, scale: 3 },
    ],
    subject: [
      { part: "closed-tome", x: 50, y: 70, scale: 4.4 },
      { part: "chain-length", x: 28, y: 62, scale: 2.2, paint: "purple-400" },
      { part: "chain-length", x: 72, y: 74, scale: 2.2, flip: -1, paint: "purple-400" },
      { part: "lock", x: 50, y: 64, scale: 4 },
    ],
    light: [{ part: "zap", x: 50, y: 56, scale: 0.9, paint: "violet-300" }],
    particles: [{ part: "sparks", x: 62, y: 62, scale: 2, count: 3, spread: [14, 6], paint: "purple-400" }],
  },
};

/** Gravedigger (Core #37, Rare Unit): a lanky gravedigger leans on his spade as a skeletal hand pops up from a fresh mound, offering him a card, on a tidy hilltop churchyard in morning light. */
const GRAVEDIGGER: SceneRecipe = {
  family: "core",
  sky: ["sky-300", "slate-100"],
  light: { x: 24, y: 20, r: 40, paint: "white", opacity: 0.5 },
  emblem: { x: 34, y: 60, size: 5, rotate: 0 },
  figure: {
    x: 34,
    y: 84,
    scale: 1,
    pose: "pose-hold",
    look: { ...figureLookFor(["Human"]), hair: "fedora", outfit: "outfit-waistcoat", item: "item-spade", cloth: "wood-700" },
  },
  layers: {
    sky: [{ part: "sun-disc", x: 18, y: 18, scale: 1.4 }],
    far: [
      { part: "hills", x: 50, y: 62 },
      { part: "tombstone", x: 62, y: 66, scale: 1.4, count: 3, spread: [24, 2] },
    ],
    mid: [
      { part: "field", x: 50, y: 74 },
      { part: "fence", x: 82, y: 74, scale: 1.4 },
      { part: "dead-tree", x: 90, y: 74, scale: 1.4 },
    ],
    subject: [
      { part: "grave-mound", x: 68, y: 84, scale: 2 },
      { part: "bone-hand", x: 68, y: 80, scale: 3 },
      { part: "playing-card", x: 70, y: 56, scale: 2.4 },
    ],
    near: [{ part: "mist", x: 50, y: 92, scale: 1.4 }],
    particles: [{ part: "motes", x: 62, y: 62, scale: 1.2, paint: "green-200" }],
  },
};

export const CORE_SCENES_1: Readonly<Record<string, SceneRecipe>> = {
  "core-001": BIG_D_FENDER,
  "core-002": SPECIESIST,
  "core-003": RIGHT_HOUSE_DEFENDER,
  "core-004": GARY_THE_GAMBLER,
  "core-005": STOCKPILE,
  "core-006": MANA_WELL,
  "core-007": JEWELOSCO_SCARAB,
  "core-008": MR_VANILLA,
  "core-009": MOTHS_TO_THE_FLAME,
  "core-010": RAPID_REPLENISH,
  "core-011": TEMPO_TIMMY,
  "core-012": DUPLICATING_FELINORS,
  "core-013": JLOCKEED_SHREDDER_10,
  "core-014": JLOCKEEDS_WEAPONS,
  "core-015": ME_AND_MR_TOKEN,
  "core-016": HIT_JOB,
  "core-017": FLOOD,
  "core-018": BREAD_AND_BUTTER,
  "core-019": MIDRANGE_MENACE,
  "core-020": POINTMASTER,
  "core-021": HINDER,
  "core-022": CARNIVOROUS_CUBE,
  "core-023": REOCCURRING_DREAM,
  "core-024": EFFICIENCY_DIVIDEND,
  "core-025": FOUR_MANA_SEVEN_SEVEN,
  "core-026": GLOWY_JELLY_BEAN,
  "core-027": BLOOD_RIDDEN_GLOWY_JELLY_BEAN,
  "core-028": KNOCKOFF_TEMU_GLOWY_JELLY_BEAN,
  "core-029": GIGA_GLOWY_JELLY_BEAN,
  "core-030": ARCHIVIST,
  "core-031": KYS_MATH_EQUATION,
  "core-032": PREM_PANTHER,
  "core-033": UNSTABLE_CLONE_MACHINE,
  "core-034": COLLATERAL_DAMAGE,
  "core-035": LUNAR_ECLIPSE,
  "core-036": MAGIC_JAMMED,
  "core-037": GRAVEDIGGER,
};
