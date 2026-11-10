// R3352000: Classic's scene recipes, part 1 of three (#1 to #30), one per entry, each drawn from its brief (#3352's
// table) in Classic's temple-stone look (ART.md, "Classic's scenes"): its subject within x 30 to 70 and y 25 to 85,
// its foreground near, its middle in the mid and subject layers and its background far, richer the rarer.
// Presentation only (CLAUDE.md rule 7): a recipe is read by the card's id alone.

import { figureLookFor, type FigureLook } from "./figureKit.ts";
import type { SceneRecipe } from "./scene.ts";

/** A plain hero of the Classic scenes, empty-handed. */
const HERO: FigureLook = { ...figureLookFor(["Human"]), item: null, cloth: "blue-800" };
/** A knight in mail and helm, his sword laid down. */
const KNIGHT: FigureLook = { ...figureLookFor(["Human"]), hair: "helm", outfit: "outfit-armor", item: null, cloth: "slate-700" };
/** A ghoul in rags, lurching. */
const GHOUL: FigureLook = { head: "head-ghoul", hair: null, outfit: "outfit-rags", item: null, skin: "green-700", cloth: "slate-700" };
/** A hooded caster in a long robe, empty-handed. */
const CASTER: FigureLook = { ...figureLookFor(["Human"]), hair: "hood", outfit: "outfit-robe", item: null, cloth: "violet-700" };

/** Curse of the Forgotten Classic (Classic #1, Rare): pale ghosts of forgotten cards rise from a drifting grey limbo and lash a cowering hero with violet curse-chains; starless exile realm, cold violet light. */
const CURSE_OF_THE_FORGOTTEN_CLASSIC: SceneRecipe = {
  family: "classic",
  sky: ["slate-900", "slate-700"],
  light: { x: 50, y: 52, r: 36, paint: "violet-500", opacity: 0.4 },
  emblem: { x: 50, y: 82, size: 5, rotate: 0 },
  figure: { x: 50, y: 80, scale: 0.72, pose: "pose-kneel", look: HERO },
  layers: {
    sky: [{ part: "mist", x: 50, y: 30, scale: 1.4, count: 3, spread: [30, 6], paint: "slate-400" }],
    far: [
      { part: "cloud", x: 24, y: 52, scale: 1.4, count: 2, spread: [12, 4], paint: "slate-700" },
      { part: "cloud", x: 78, y: 48, scale: 1.3, paint: "slate-700" },
      { part: "mist", x: 50, y: 64, scale: 1.6, count: 2, spread: [30, 4], paint: "slate-400" },
    ],
    mid: [
      { part: "playing-card", x: 26, y: 40, scale: 2.2, count: 3, spread: [10, 8], paint: "slate-100", fade: 0.5 },
      { part: "playing-card", x: 74, y: 36, scale: 2.2, count: 3, spread: [10, 8], paint: "slate-100", fade: 0.5 },
      { part: "ghost-figure", x: 22, y: 70, scale: 1.3, fade: 0.6 },
      { part: "ghost-figure", x: 78, y: 68, scale: 1.2, flip: -1, fade: 0.6 },
      { part: "ghost", x: 50, y: 30, scale: 1.2, count: 2, spread: [14, 4], fade: 0.5 },
    ],
    subject: [
      { part: "rock-floor", x: 50, y: 78, paint: "slate-700", fade: 0.6 },
      { part: "chain-length", x: 28, y: 52, scale: 1.1, paint: "violet-500" },
      { part: "chain-length", x: 72, y: 50, scale: 1.1, flip: -1, paint: "violet-500" },
    ],
    near: [
      { part: "mist", x: 24, y: 94, scale: 1.4, paint: "slate-400" },
      { part: "mist", x: 80, y: 92, scale: 1.2, paint: "slate-400" },
    ],
    light: [
      { part: "halo", x: 50, y: 58, scale: 2, paint: "violet-500" },
      { part: "rim-light", x: 50, y: 60, scale: 1.2, paint: "violet-300" },
    ],
    particles: [
      { part: "sparks", x: 50, y: 52, count: 2, spread: [16, 6], paint: "violet-300" },
      { part: "motes", x: 50, y: 40, scale: 1.2, paint: "slate-100" },
    ],
  },
};

/** The Trickster (Classic #2, Common): a grinning conjurer in a patched harlequin coat fans a hand of cards at you, one palmed up his sleeve; lantern-lit bazaar alley, sly warmth. */
const THE_TRICKSTER: SceneRecipe = {
  family: "classic",
  sky: ["slate-900", "violet-700"],
  light: { x: 50, y: 30, r: 36, paint: "orange-400", opacity: 0.4 },
  emblem: { x: 84, y: 72, size: 5, rotate: 0 },
  figure: {
    x: 50,
    y: 84,
    scale: 0.95,
    pose: "pose-hold",
    look: { ...figureLookFor(["Human"]), hair: "jester-cap", outfit: "outfit-harlequin", item: "item-card-fan", cloth: "violet-700" },
  },
  layers: {
    far: [
      { part: "townhouse", x: 14, y: 66, scale: 1.1 },
      { part: "townhouse", x: 86, y: 64, scale: 1, flip: -1 },
      { part: "lantern-string", x: 50, y: 22, scale: 1.4 },
    ],
    mid: [
      { part: "cobbles", x: 50, y: 66, paint: "stone-500" },
      { part: "awning", x: 22, y: 74, scale: 1.1 },
      { part: "crate", x: 84, y: 78, scale: 1.4, count: 2, spread: [3, 0] },
    ],
    near: [
      { part: "playing-card", x: 43, y: 64, scale: 0.45 },
      { part: "pebbles", x: 70, y: 98 },
    ],
    particles: [{ part: "motes", x: 50, y: 40, scale: 1.2, paint: "orange-400" }],
  },
};

/** Book of Heal (Classic #3, Epic): an open tome floats, its pages unfurling into bandages that wrap a wounded knight's arm as he kneels on a temple infirmary's flagstones under tall arched windows of soft green-white morning light; the bandages are a ribbon of light. */
const BOOK_OF_HEAL: SceneRecipe = {
  family: "classic",
  sky: ["slate-700", "stone-500"],
  light: { x: 30, y: 30, r: 40, paint: "green-200", opacity: 0.45 },
  emblem: { x: 36, y: 60, size: 5, rotate: -8 },
  figure: { x: 60, y: 80, scale: 0.82, flip: -1, pose: "pose-kneel", look: KNIGHT },
  layers: {
    far: [
      { part: "tile-wall", x: 50, y: 66, paint: "slate-700" },
      { part: "window", x: 18, y: 56, scale: 2.2, paint: "green-200" },
      { part: "window", x: 50, y: 54, scale: 2.4, paint: "green-200" },
      { part: "window", x: 82, y: 56, scale: 2.2, paint: "green-200" },
      { part: "column", x: 34, y: 66, scale: 2 },
      { part: "column", x: 66, y: 66, scale: 2 },
    ],
    mid: [
      { part: "tile-floor", x: 50, y: 66, paint: "stone-500" },
      { part: "cot", x: 16, y: 76, scale: 0.9 },
      { part: "cot", x: 86, y: 74, scale: 0.8, flip: -1 },
      { part: "jar", x: 26, y: 70, count: 2, spread: [3, 0] },
    ],
    subject: [
      { part: "heal-glow", x: 54, y: 56, scale: 1.2, paint: "green-200" },
      { part: "light-ribbon", x: 48, y: 56, scale: 0.7, paint: "white" },
    ],
    near: [
      { part: "tome", x: 38, y: 64, scale: 2.2, paint: "green-700" },
      { part: "candle", x: 12, y: 97, scale: 1.1 },
      { part: "vial", x: 86, y: 97, scale: 1.6, count: 2, spread: [4, 0] },
      { part: "pebbles", x: 50, y: 98 },
    ],
    light: [
      { part: "light-shaft", x: 22, y: 0, scale: 1.2, paint: "green-200" },
      { part: "light-shaft", x: 54, y: 0, scale: 1.1, paint: "white" },
      { part: "halo", x: 40, y: 58, scale: 1.6, paint: "green-200" },
    ],
    particles: [
      { part: "motes", x: 44, y: 46, scale: 1.2, paint: "green-200" },
      { part: "sparks", x: 50, y: 54, count: 2, spread: [10, 6], paint: "white" },
    ],
  },
};

/** Palantir (Classic #4, Legendary): a Jlockeed watchtower's summit chamber, a black seeing-orb on a carved plinth blazing cyan, its beam snatching a spell out of a rival caster's hands mid-cast; brass lenses and spying mirrors on the walls, storm clouds past the arches, cold prying cyan light. */
const PALANTIR: SceneRecipe = {
  family: "classic",
  sky: ["slate-900", "blue-900"],
  light: { x: 46, y: 52, r: 34, paint: "cyan-400", opacity: 0.45 },
  emblem: { x: 46, y: 72, size: 5, rotate: 0 },
  extras: [{ layer: "mid", x: 78, y: 78, scale: 0.6, flip: -1, pose: "pose-cast", look: CASTER }],
  layers: {
    sky: [
      { part: "cloud", x: 30, y: 26, scale: 1.4, count: 3, spread: [24, 6], paint: "slate-700" },
      { part: "lightning", x: 70, y: 18, scale: 0.8, paint: "sky-300" },
      { part: "rain", x: 50, y: 34, scale: 1.2, paint: "slate-400" },
    ],
    far: [
      { part: "arch", x: 26, y: 66, scale: 1.7, paint: "slate-700" },
      { part: "arch", x: 74, y: 66, scale: 1.7, paint: "slate-700" },
      { part: "column", x: 8, y: 68, scale: 2 },
      { part: "column", x: 50, y: 68, scale: 2 },
      { part: "column", x: 92, y: 68, scale: 2 },
      { part: "round-window", x: 50, y: 22, scale: 0.7, paint: "brass-600" },
      { part: "round-window", x: 26, y: 32, scale: 0.35, paint: "brass-600" },
      { part: "round-window", x: 74, y: 32, scale: 0.35, paint: "brass-600" },
    ],
    mid: [
      { part: "tile-floor", x: 50, y: 68, paint: "slate-700" },
      { part: "mirror", x: 16, y: 78, scale: 1.3 },
      { part: "telescope", x: 30, y: 76, scale: 1.2 },
      { part: "astrolabe", x: 62, y: 74, scale: 0.6 },
      { part: "globe", x: 92, y: 80, scale: 1.2 },
    ],
    subject: [
      { part: "pedestal", x: 46, y: 82, scale: 2.4, paint: "stone-500" },
      { part: "orb", x: 46, y: 56, scale: 2.4, paint: "slate-900" },
      { part: "eye", x: 46, y: 56, scale: 0.6 },
    ],
    near: [
      { part: "rune-circle", x: 46, y: 86, scale: 0.6, paint: "cyan-400" },
      { part: "closed-tome", x: 14, y: 97, scale: 1 },
      { part: "candle", x: 88, y: 97, scale: 1 },
    ],
    light: [
      { part: "halo", x: 46, y: 56, scale: 2.2, paint: "cyan-400" },
      { part: "zap", x: 50, y: 54, scale: 0.8, paint: "cyan-400" },
      { part: "rim-light", x: 46, y: 62, scale: 1.2, paint: "cyan-400" },
    ],
    particles: [
      { part: "fireball", x: 82, y: 52, scale: 0.4, flip: -1, paint: "violet-500" },
      { part: "sparks", x: 66, y: 52, count: 2, spread: [10, 4], paint: "cyan-400" },
      { part: "motes", x: 46, y: 40, scale: 1.2, paint: "white" },
    ],
  },
};

/** Tesla (Classic #5, Epic): a copper coil-tower automaton bursts up through the flagstones, arcs lashing a freshly summoned soldier on a stone lane scorched by sparks, in a dark forge-hall hung with chains; violet-white lightning glare, crackling menace. */
const TESLA: SceneRecipe = {
  family: "classic",
  sky: ["black", "violet-700"],
  light: { x: 44, y: 40, r: 36, paint: "violet-300", opacity: 0.45 },
  emblem: { x: 44, y: 74, size: 5, rotate: 0 },
  extras: [
    {
      layer: "subject",
      x: 74,
      y: 84,
      scale: 0.64,
      flip: -1,
      pose: "pose-block",
      look: { ...figureLookFor(["Human"]), hair: "steel-helmet", outfit: "outfit-armor", cloth: "blue-700" },
      fade: 0.85,
    },
  ],
  layers: {
    far: [
      { part: "tile-wall", x: 50, y: 64, paint: "slate-900" },
      { part: "forge", x: 84, y: 64, scale: 0.8 },
      { part: "chain-length", x: 8, y: 20, scale: 1.6 },
      { part: "chain-length", x: 58, y: 16, scale: 1.4 },
    ],
    mid: [
      { part: "street", x: 50, y: 64, paint: "slate-700" },
      { part: "anvil", x: 16, y: 72, scale: 1.2 },
      { part: "smoke", x: 24, y: 70, scale: 0.8, paint: "slate-700" },
    ],
    subject: [
      { part: "tesla-coil", x: 44, y: 84, scale: 1.1 },
      { part: "zap", x: 50, y: 50, scale: 0.6, paint: "violet-300" },
    ],
    near: [
      { part: "pebbles", x: 30, y: 98, count: 2, spread: [8, 1] },
      { part: "chain", x: 70, y: 96, scale: 1 },
    ],
    light: [
      { part: "halo", x: 44, y: 38, scale: 2, paint: "violet-300" },
      { part: "lightning", x: 64, y: 44, scale: 0.6, paint: "white" },
      { part: "rim-light", x: 44, y: 60, scale: 1.2, paint: "violet-300" },
    ],
    particles: [
      { part: "sparks", x: 50, y: 46, count: 3, spread: [20, 10], paint: "white" },
      { part: "sparks", x: 70, y: 60, count: 2, spread: [8, 6], paint: "violet-300" },
    ],
  },
};

/** Cloaked Toe Cracker (Classic #6, Common): a hooded, barefoot rogue in a charcoal cloak crouches by a spike-trap, cracking his toes as he slips it free; dim crypt corridor, blue shadows. */
const CLOAKED_TOE_CRACKER: SceneRecipe = {
  family: "classic",
  sky: ["slate-900", "blue-900"],
  light: { x: 50, y: 40, r: 34, paint: "blue-600", opacity: 0.35 },
  emblem: { x: 68, y: 80, size: 5, rotate: 0 },
  figure: {
    x: 44,
    y: 84,
    scale: 0.95,
    pose: "pose-crouch",
    look: { ...figureLookFor(["Human"]), hair: "hood", outfit: "outfit-travel-cloak", item: null, cloth: "slate-800" },
  },
  layers: {
    far: [
      { part: "arch", x: 50, y: 64, scale: 1.6, paint: "slate-700" },
      { part: "column", x: 16, y: 66, scale: 1.8 },
      { part: "column", x: 84, y: 66, scale: 1.8 },
    ],
    mid: [
      { part: "tile-floor", x: 50, y: 66, paint: "slate-800" },
      { part: "tombstone", x: 82, y: 72, scale: 1.6 },
    ],
    subject: [{ part: "spike-trap", x: 68, y: 84, scale: 1.1 }],
    near: [
      { part: "bones", x: 14, y: 97, scale: 1.2 },
      { part: "candle", x: 90, y: 97, scale: 0.9 },
    ],
    particles: [{ part: "dust", x: 50, y: 86, scale: 0.8, paint: "slate-400" }],
  },
};

/** InfiniScepter (Classic #7, Legendary): a domed sanctum of ribbed stone arches, a crystal sceptre hovering above a dais with a captured spell coiled in its looped head like a trapped comet, while a spectral copy of that spell bursts out across the chamber; twin figure-eight light trails circle the dais, violet and teal glow, a moment of release. */
const INFINISCEPTER: SceneRecipe = {
  family: "classic",
  sky: ["slate-900", "violet-700"],
  light: { x: 50, y: 36, r: 38, paint: "violet-500", opacity: 0.5 },
  emblem: { x: 50, y: 80, size: 5, rotate: 0 },
  layers: {
    sky: [
      { part: "dome", x: 50, y: 14, scale: 1.1, paint: "slate-700" },
      { part: "stars", x: 50, y: 14, scale: 1.2, paint: "violet-300" },
    ],
    far: [
      { part: "arch", x: 18, y: 66, scale: 1.7, paint: "slate-700" },
      { part: "arch", x: 82, y: 66, scale: 1.7, paint: "slate-700" },
      { part: "column", x: 4, y: 68, scale: 2.2 },
      { part: "column", x: 34, y: 68, scale: 2.2 },
      { part: "column", x: 66, y: 68, scale: 2.2 },
      { part: "column", x: 96, y: 68, scale: 2.2 },
    ],
    mid: [
      { part: "tile-floor", x: 50, y: 68, paint: "slate-800" },
      { part: "statue", x: 16, y: 78, scale: 1.1 },
      { part: "statue", x: 84, y: 78, scale: 1.1, flip: -1 },
    ],
    subject: [
      { part: "altar", x: 50, y: 88, scale: 1.8 },
      { part: "light-ribbon", x: 50, y: 74, scale: 1.1, paint: "violet-300" },
      { part: "light-ribbon", x: 50, y: 78, scale: 1.3, paint: "cyan-400" },
      { part: "sceptre", x: 50, y: 68, scale: 1 },
    ],
    near: [
      { part: "rune-circle", x: 50, y: 94, scale: 0.7, paint: "violet-300" },
      { part: "candle", x: 10, y: 97, scale: 1 },
      { part: "candle", x: 90, y: 97, scale: 1 },
    ],
    light: [
      { part: "halo", x: 50, y: 32, scale: 2.2, paint: "violet-500" },
      { part: "halo", x: 50, y: 76, scale: 1.8, paint: "cyan-400" },
      { part: "god-rays", x: 50, y: 4, scale: 0.8, paint: "violet-300" },
    ],
    particles: [
      { part: "fireball", x: 80, y: 40, scale: 0.8, paint: "cyan-400", fade: 0.55 },
      { part: "streaks", x: 64, y: 38, scale: 0.8, paint: "cyan-400" },
      { part: "sparks", x: 50, y: 40, count: 3, spread: [20, 12], paint: "violet-300" },
      { part: "motes", x: 50, y: 60, scale: 1.2, paint: "cyan-400" },
    ],
  },
};

/** Pickle (Classic #8, Rare): a sweating duelist backs against a pickling cellar's shelves as a huge brine-dripping pickle looms over three offered paths; sour green lantern light, cornered mood; the paths are the floor's three lanes. */
const PICKLE: SceneRecipe = {
  family: "classic",
  sky: ["slate-900", "green-900"],
  light: { x: 50, y: 24, r: 36, paint: "lime-400", opacity: 0.4 },
  emblem: { x: 50, y: 58, size: 5, rotate: 0 },
  figure: { x: 26, y: 84, scale: 0.7, pose: "pose-block", look: { ...HERO, hair: "hair-short", cloth: "red-900" } },
  layers: {
    sky: [{ part: "lantern", x: 50, y: 6, scale: 1.3, paint: "lime-400" }],
    far: [
      { part: "tile-wall", x: 50, y: 64, paint: "wood-800" },
      { part: "shelf", x: 14, y: 62, scale: 1.5 },
      { part: "shelf", x: 86, y: 62, scale: 1.5, flip: -1 },
      { part: "jar", x: 14, y: 44, scale: 1.2, count: 3, spread: [8, 0], paint: "green-400" },
    ],
    mid: [
      { part: "lanes", x: 50, y: 64, paint: "slate-700" },
      { part: "barrel", x: 84, y: 80, scale: 0.8 },
    ],
    subject: [
      { part: "pickle", x: 52, y: 76, scale: 1.3, flip: -1 },
      { part: "drops", x: 58, y: 80, scale: 0.6, paint: "green-400" },
    ],
    near: [
      { part: "jar", x: 12, y: 98, scale: 1.6, paint: "green-400" },
      { part: "pebbles", x: 72, y: 98 },
    ],
    light: [{ part: "halo", x: 50, y: 14, scale: 1.6, paint: "lime-400" }],
    particles: [
      { part: "drops", x: 30, y: 38, scale: 0.4, paint: "blue-200" },
      { part: "drops", x: 52, y: 88, scale: 0.5, count: 2, spread: [6, 2], paint: "green-400" },
    ],
  },
};

/** Income Tax (Classic #9, Legendary): a stone counting-house as the trap springs, an iron strongbox bursting open as a cloaked tax collector's long arm sweeps a rival's fanned hand of card backs across the table, one card left behind; copper coins scatter, candles gutter, hard crimson lamplight. */
const INCOME_TAX: SceneRecipe = {
  family: "classic",
  sky: ["wood-900", "red-900"],
  light: { x: 50, y: 30, r: 38, paint: "red-600", opacity: 0.45 },
  emblem: { x: 24, y: 58, size: 5, rotate: 0 },
  figure: { x: 42, y: 84, scale: 0.8, pose: "pose-lunge", look: { ...CASTER, outfit: "outfit-coat", cloth: "slate-900" } },
  layers: {
    sky: [{ part: "lantern", x: 50, y: 8, scale: 1.2 }],
    far: [
      { part: "tile-wall", x: 50, y: 62, paint: "slate-800" },
      { part: "shelf", x: 16, y: 50, scale: 1.2 },
      { part: "shelf", x: 84, y: 50, scale: 1.2, flip: -1 },
      { part: "arch", x: 50, y: 56, scale: 1.3, paint: "stone-500" },
    ],
    mid: [
      { part: "tile-floor", x: 50, y: 62, paint: "slate-700" },
      { part: "coin-stack", x: 82, y: 66, scale: 1.2, paint: "brass-600" },
      { part: "sack", x: 12, y: 70, scale: 1.2 },
    ],
    subject: [
      { part: "burst", x: 24, y: 52, scale: 1.2, paint: "orange-400" },
      { part: "card-fan", x: 62, y: 58, scale: 1.3, paint: "red-900" },
      { part: "streaks", x: 74, y: 56, scale: 0.8, paint: "white" },
    ],
    near: [
      { part: "wide-table", x: 50, y: 74, scale: 1.2 },
      { part: "chest", x: 26, y: 62, scale: 1.3, paint: "slate-700" },
      { part: "playing-card", x: 78, y: 60, scale: 1, paint: "red-900" },
      { part: "coins", x: 46, y: 62, scale: 1, paint: "brass-600" },
      { part: "candle", x: 88, y: 62, scale: 0.9 },
      { part: "open-hand", x: 92, y: 56, scale: 1, flip: -1 },
    ],
    light: [
      { part: "halo", x: 50, y: 14, scale: 2, paint: "red-500" },
      { part: "rim-light", x: 42, y: 56, scale: 1.2, paint: "red-500" },
      { part: "halo", x: 24, y: 54, scale: 1.2, paint: "orange-400" },
    ],
    particles: [
      { part: "coin", x: 30, y: 44, count: 5, spread: [14, 8], paint: "brass-600" },
      { part: "smoke", x: 88, y: 50, scale: 0.6, paint: "slate-400" },
      { part: "sparks", x: 50, y: 50, count: 2, spread: [18, 6], paint: "orange-400" },
    ],
  },
};

/** Exile (Classic #10, Common): a swirling indigo portal snaps open under a tiny scurrying imp, flinging it toward a far-off star; moss-cracked temple steps at dusk, sudden chill. */
const EXILE: SceneRecipe = {
  family: "classic",
  sky: ["blue-900", "violet-700"],
  light: { x: 50, y: 66, r: 30, paint: "violet-500", opacity: 0.4 },
  emblem: { x: 50, y: 74, size: 5, rotate: 0 },
  layers: {
    sky: [{ part: "sparks", x: 78, y: 22, scale: 1.4, paint: "white" }],
    far: [
      { part: "temple", x: 50, y: 52, scale: 1.1, paint: "slate-700" },
      { part: "column", x: 16, y: 58, scale: 1.4 },
      { part: "column", x: 84, y: 58, scale: 1.4 },
    ],
    mid: [
      { part: "stairs", x: 50, y: 60, paint: "stone-500" },
      { part: "vine", x: 16, y: 70, scale: 1.2 },
      { part: "grass-tuft", x: 80, y: 72, count: 3, spread: [6, 2], paint: "green-700" },
    ],
    subject: [
      { part: "void-swirl", x: 50, y: 74, scale: 1.4, paint: "blue-700" },
      { part: "card-imp", x: 56, y: 48, scale: 1.8 },
      { part: "streaks", x: 52, y: 54, scale: 0.6, paint: "violet-300" },
    ],
    near: [
      { part: "mossy-boulder", x: 12, y: 97, scale: 1.4 },
      { part: "grass-tuft", x: 88, y: 98, count: 2, spread: [5, 1], paint: "green-700" },
      { part: "pebbles", x: 60, y: 98 },
    ],
    particles: [{ part: "sparks", x: 50, y: 70, count: 2, spread: [14, 4], paint: "violet-300" }],
  },
};

/** Mind Melt (Classic #11, Common): a seer's hand passes through a rival's translucent skull and plucks one glowing memory, which melts into smoke; hushed reading room, violet candle haze. */
const MIND_MELT: SceneRecipe = {
  family: "classic",
  sky: ["slate-900", "violet-700"],
  light: { x: 54, y: 46, r: 30, paint: "violet-500", opacity: 0.45 },
  emblem: { x: 50, y: 28, size: 5, rotate: 0 },
  figure: { x: 34, y: 84, scale: 0.8, pose: "pose-cast", look: { ...CASTER, cloth: "slate-800" } },
  extras: [{ layer: "subject", x: 64, y: 80, scale: 0.8, flip: -1, pose: "pose-sit", look: { ...HERO, cloth: "red-900" }, fade: 0.5 }],
  layers: {
    far: [
      { part: "bookcase", x: 14, y: 70, scale: 1.4 },
      { part: "bookcase", x: 86, y: 70, scale: 1.4 },
      { part: "round-window", x: 50, y: 28, scale: 0.7, paint: "violet-300" },
    ],
    mid: [{ part: "tile-floor", x: 50, y: 70, paint: "slate-800" }],
    subject: [
      { part: "chair", x: 67, y: 82, scale: 1.2, flip: -1 },
      { part: "wisp", x: 54, y: 52, scale: 1.4, paint: "violet-300" },
      { part: "steam-curl", x: 58, y: 46, scale: 0.6, paint: "violet-300" },
    ],
    near: [
      { part: "candle", x: 12, y: 97, scale: 1.2 },
      { part: "candle", x: 90, y: 97, scale: 1 },
    ],
    light: [{ part: "halo", x: 54, y: 52, scale: 1.4, paint: "violet-500" }],
    particles: [{ part: "motes", x: 50, y: 40, scale: 1.2, paint: "violet-300" }],
  },
};

/** Book of Blood (Classic #12, Epic): a crimson-bound tome bleeds a ribbon of red light into a lurching ghoul, the stream curling back into the caster's palm in a ruined battlefield shrine, broken walls against a low blood-red sunset; grim, hungry mood. */
const BOOK_OF_BLOOD: SceneRecipe = {
  family: "classic",
  sky: ["slate-900", "red-600"],
  light: { x: 78, y: 64, r: 38, paint: "red-500", opacity: 0.45 },
  emblem: { x: 52, y: 48, size: 5, rotate: -6 },
  figure: { x: 32, y: 82, scale: 0.74, pose: "pose-cast", look: { ...CASTER, cloth: "wood-800" } },
  extras: [{ layer: "subject", x: 72, y: 80, scale: 0.7, flip: -1, pose: "pose-lunge", look: GHOUL }],
  layers: {
    sky: [
      { part: "sun-disc", x: 80, y: 62, scale: 1.2, paint: "red-500" },
      { part: "cloud", x: 30, y: 30, scale: 1.4, count: 2, spread: [16, 4], paint: "red-900" },
    ],
    far: [
      { part: "ruin-wall", x: 16, y: 64, scale: 1.8, paint: "slate-800" },
      { part: "ruin-wall", x: 86, y: 64, scale: 1.5, flip: -1, paint: "slate-800" },
      { part: "column", x: 50, y: 64, scale: 1.4, paint: "slate-700" },
    ],
    mid: [
      { part: "cracked-earth", x: 50, y: 64, paint: "wood-800" },
      { part: "altar", x: 50, y: 74, scale: 1.3 },
      { part: "broken-spear", x: 18, y: 74, scale: 0.6 },
      { part: "shield", x: 86, y: 74, scale: 1.2, paint: "red-900" },
    ],
    subject: [
      { part: "tome", x: 52, y: 52, scale: 1.6, paint: "red-900" },
      { part: "light-ribbon", x: 54, y: 58, scale: 0.8, paint: "red-500" },
    ],
    near: [
      { part: "bones", x: 14, y: 97, scale: 1.2 },
      { part: "helmet", x: 62, y: 98, scale: 1.4 },
      { part: "broken-spear", x: 84, y: 98, scale: 0.7, flip: -1 },
      { part: "pebbles", x: 38, y: 98 },
    ],
    light: [
      { part: "halo", x: 52, y: 52, scale: 1.6, paint: "red-500" },
      { part: "rim-light", x: 72, y: 60, scale: 1, paint: "red-400" },
    ],
    particles: [
      { part: "embers", x: 56, y: 50, count: 2, spread: [14, 6], paint: "red-500" },
      { part: "drops", x: 64, y: 62, scale: 0.7, paint: "red-600" },
    ],
  },
};

/** Boots on the Ground (Classic #13, Common): a mud-spattered foot soldier in hobnailed boots sprints out of a trench, spear forward, a card tucked in his belt; rainy battlefield dawn, urgent grey. */
const BOOTS_ON_THE_GROUND: SceneRecipe = {
  family: "classic",
  sky: ["slate-700", "stone-500"],
  light: { x: 18, y: 50, r: 34, paint: "slate-100", opacity: 0.3 },
  emblem: { x: 80, y: 70, size: 5, rotate: 0 },
  figure: {
    x: 50,
    y: 84,
    scale: 0.95,
    pose: "pose-run",
    look: { ...figureLookFor(["Human"]), hair: "steel-helmet", item: "item-spear", cloth: "wood-700" },
  },
  layers: {
    sky: [{ part: "cloud", x: 60, y: 24, scale: 1.6, count: 2, spread: [24, 4], paint: "slate-400" }],
    far: [
      { part: "hills", x: 50, y: 60, paint: "slate-700" },
      { part: "dead-tree", x: 16, y: 62, scale: 1.2 },
      { part: "palisade", x: 82, y: 64, scale: 0.8, paint: "wood-800" },
    ],
    mid: [
      { part: "furrows", x: 50, y: 66, paint: "wood-800" },
      { part: "sandbags", x: 20, y: 76, scale: 1 },
    ],
    near: [
      { part: "playing-card", x: 47, y: 62, scale: 0.55 },
      { part: "sandbags", x: 50, y: 98, scale: 1.6 },
      { part: "dust-cloud", x: 40, y: 88, scale: 1.2, paint: "wood-700" },
    ],
    particles: [{ part: "rain", x: 50, y: 40, scale: 1.4, count: 2, spread: [24, 10], paint: "slate-100" }],
  },
};

/** Shadowstep (Classic #14, Common): a falling swordsman dissolves into a black shadow that leaps back into his commander's waiting hand; moonless cloister arcade, sudden violet flicker. */
const SHADOWSTEP: SceneRecipe = {
  family: "classic",
  sky: ["black", "slate-900"],
  light: { x: 62, y: 50, r: 30, paint: "violet-500", opacity: 0.4 },
  emblem: { x: 50, y: 32, size: 5, rotate: 0 },
  figure: { x: 38, y: 80, scale: 0.72, pose: "pose-spread", look: { ...HERO, item: "item-sword", cloth: "slate-700" }, fade: 0.55 },
  extras: [{ layer: "subject", x: 70, y: 84, scale: 0.8, flip: -1, pose: "pose-hold", look: { ...KNIGHT, cloth: "red-900" } }],
  layers: {
    far: [
      { part: "arch", x: 18, y: 64, scale: 1.4, paint: "slate-700" },
      { part: "arch", x: 50, y: 64, scale: 1.4, paint: "slate-700" },
      { part: "arch", x: 82, y: 64, scale: 1.4, paint: "slate-700" },
    ],
    mid: [{ part: "tile-floor", x: 50, y: 64, paint: "slate-800" }],
    subject: [
      { part: "ghost-figure", x: 54, y: 68, scale: 1.1, paint: "slate-900", fade: 0.9 },
      { part: "streaks", x: 66, y: 58, scale: 0.7, paint: "violet-300" },
      { part: "smoke", x: 40, y: 64, scale: 0.8, paint: "slate-900" },
    ],
    near: [
      { part: "column", x: 6, y: 100, scale: 2.4 },
      { part: "pebbles", x: 60, y: 98 },
    ],
    light: [
      { part: "halo", x: 62, y: 58, scale: 1.2, paint: "violet-500" },
      { part: "rim-light", x: 54, y: 56, scale: 0.9, paint: "violet-500" },
    ],
    particles: [{ part: "sparks", x: 52, y: 56, count: 2, spread: [12, 6], paint: "violet-300" }],
  },
};

/** Nose Hunter (Classic #15, Common): a lanky tracker with an enormous twitching nose sniffs a trail of scattered cards, a gnawed one in his teeth; misty forest path, grey morning. */
const NOSE_HUNTER: SceneRecipe = {
  family: "classic",
  sky: ["slate-400", "slate-100"],
  light: { x: 28, y: 30, r: 34, paint: "white", opacity: 0.3 },
  emblem: { x: 74, y: 82, size: 5, rotate: 0 },
  figure: {
    x: 46,
    y: 84,
    scale: 1.02,
    pose: "pose-stride",
    look: { ...figureLookFor(["Human"]), head: "head-nose", outfit: "outfit-coat", item: "item-card", cloth: "green-900" },
  },
  layers: {
    far: [
      { part: "pine", x: 50, y: 60, scale: 1.4, count: 4, spread: [40, 2], paint: "green-900" },
      { part: "mist", x: 50, y: 58, scale: 1.8, count: 2, spread: [30, 2], paint: "slate-100" },
    ],
    mid: [
      { part: "road", x: 50, y: 64, paint: "wood-600" },
      { part: "tree", x: 12, y: 76, scale: 1.2 },
      { part: "pine", x: 88, y: 76, scale: 1.3 },
    ],
    near: [
      { part: "playing-card", x: 74, y: 86, scale: 0.8, count: 3, spread: [14, 3] },
      { part: "fern", x: 10, y: 98, scale: 1.3 },
    ],
    particles: [{ part: "mist", x: 50, y: 80, scale: 1.4, paint: "white" }],
  },
};

/** Book of Flame (Classic #16, Epic): a leather tome flung open, a torrent of fire roaring off its pages and punching through a shield-wall of skeletons, a scorched library tower collapsing behind; furious orange-red glare. */
const BOOK_OF_FLAME: SceneRecipe = {
  family: "classic",
  sky: ["red-900", "orange-400"],
  light: { x: 46, y: 62, r: 38, paint: "ember-500", opacity: 0.5 },
  emblem: { x: 26, y: 78, size: 5, rotate: -8 },
  layers: {
    sky: [{ part: "smoke-billow", x: 72, y: 26, scale: 2, count: 2, spread: [10, 4], paint: "slate-700" }],
    far: [
      { part: "tower", x: 74, y: 62, scale: 1.4 },
      { part: "ruin-wall", x: 20, y: 62, scale: 1.4, paint: "slate-800" },
      { part: "flames", x: 74, y: 30, scale: 0.8 },
    ],
    mid: [
      { part: "cracked-earth", x: 50, y: 64, paint: "slate-800" },
      { part: "bookcase", x: 90, y: 74, scale: 1, flip: -1 },
    ],
    subject: [
      { part: "skeleton", x: 56, y: 80, scale: 0.9, flip: -1 },
      { part: "skeleton", x: 66, y: 80, scale: 0.9, flip: -1 },
      { part: "skeleton", x: 76, y: 80, scale: 0.9, flip: -1 },
      { part: "fireball", x: 54, y: 62, scale: 1.4 },
    ],
    near: [
      { part: "tome", x: 26, y: 84, scale: 2.4, paint: "wood-700" },
      { part: "flames", x: 28, y: 76, scale: 0.9 },
      { part: "bones", x: 76, y: 97, scale: 1.2 },
    ],
    light: [
      { part: "halo", x: 46, y: 64, scale: 2.2, paint: "orange-400" },
      { part: "rim-light", x: 66, y: 64, scale: 1.2, paint: "orange-400" },
    ],
    particles: [
      { part: "embers", x: 56, y: 50, count: 3, spread: [24, 12] },
      { part: "sparks", x: 62, y: 62, count: 2, spread: [10, 8], paint: "orange-400" },
    ],
  },
};

/** Counterspell (Classic #17, Common): a robed warden's raised palm shatters an incoming fireball into harmless sparks mid-air; arched temple gate, crisp blue warding light, flat refusal. */
const COUNTERSPELL: SceneRecipe = {
  family: "classic",
  sky: ["slate-900", "blue-800"],
  light: { x: 58, y: 48, r: 30, paint: "sky-300", opacity: 0.4 },
  emblem: { x: 50, y: 30, size: 5, rotate: 0 },
  figure: { x: 40, y: 84, scale: 0.85, pose: "pose-block", look: { ...CASTER, cloth: "blue-800" } },
  layers: {
    far: [
      { part: "arch", x: 50, y: 70, scale: 2.4, paint: "stone-500" },
      { part: "column", x: 14, y: 70, scale: 2 },
      { part: "column", x: 86, y: 70, scale: 2 },
    ],
    mid: [{ part: "tile-floor", x: 50, y: 70, paint: "slate-700" }],
    subject: [
      { part: "shield-dome", x: 58, y: 52, scale: 0.8, paint: "sky-300" },
      { part: "fireball", x: 78, y: 50, scale: 0.7, flip: -1 },
      { part: "shards", x: 66, y: 50, scale: 0.8, paint: "orange-400" },
    ],
    near: [
      { part: "stone-lantern", x: 12, y: 97, scale: 1.2 },
      { part: "grass-tuft", x: 86, y: 98, count: 2, spread: [5, 1], paint: "green-700" },
    ],
    particles: [{ part: "sparks", x: 66, y: 50, count: 2, spread: [8, 6], paint: "orange-400" }],
  },
};

/** Glitch in the System (Classic #18, Epic): a carved war-table where every matching piece flickers and breaks into squares, twin statues across a stone plaza tearing away into static, a broken seam of static splitting the sky; eerie cyan and magenta flicker. */
const GLITCH_IN_THE_SYSTEM: SceneRecipe = {
  family: "classic",
  sky: ["slate-900", "violet-700"],
  light: { x: 50, y: 60, r: 34, paint: "cyan-400", opacity: 0.4 },
  emblem: { x: 50, y: 82, size: 5, rotate: 0 },
  layers: {
    sky: [
      { part: "rift", x: 50, y: 28, scale: 1, paint: "cyan-400" },
      { part: "glitch", x: 40, y: 24, scale: 1.2, count: 2, spread: [16, 4] },
    ],
    far: [
      { part: "statue", x: 20, y: 62, scale: 1.5 },
      { part: "statue", x: 80, y: 62, scale: 1.5, flip: -1 },
      { part: "pixels", x: 20, y: 46, scale: 0.8 },
      { part: "pixels", x: 80, y: 46, scale: 0.8 },
      { part: "temple", x: 50, y: 56, scale: 0.8, paint: "slate-700" },
    ],
    mid: [
      { part: "tile-floor", x: 50, y: 62, paint: "stone-500" },
      { part: "squares", x: 70, y: 40, scale: 0.6, paint: "cyan-400" },
    ],
    subject: [
      { part: "table", x: 50, y: 80, scale: 1.2 },
      { part: "chess-piece", x: 50, y: 72, scale: 1.4, count: 5, spread: [14, 1] },
      { part: "glitch", x: 50, y: 66, scale: 1.4 },
    ],
    near: [
      { part: "fallen-column", x: 16, y: 98, scale: 0.8 },
      { part: "pixels", x: 82, y: 92, scale: 0.8 },
      { part: "pebbles", x: 60, y: 98 },
    ],
    light: [
      { part: "halo", x: 50, y: 68, scale: 1.8, paint: "cyan-400" },
      { part: "rim-light", x: 50, y: 60, scale: 1.2, paint: "purple-400" },
    ],
    particles: [
      { part: "squares", x: 44, y: 60, scale: 0.5, paint: "purple-400" },
      { part: "pixels", x: 50, y: 40, scale: 1 },
    ],
  },
};

/** Lizard's Breath (Classic #19, Rare): a horned rock lizard on a toppled column breathes braided green, grey and violet flame at a fleeing raider; sunbaked canyon ruins, hot noon. */
const LIZARDS_BREATH: SceneRecipe = {
  family: "classic",
  sky: ["sky-400", "orange-400"],
  light: { x: 72, y: 20, r: 36, paint: "white", opacity: 0.4 },
  emblem: { x: 30, y: 80, size: 5, rotate: 0 },
  extras: [{ layer: "subject", x: 82, y: 84, scale: 0.55, pose: "pose-run", look: { ...KNIGHT, hair: "helm", item: "item-spear", cloth: "red-900" } }],
  layers: {
    sky: [{ part: "sun", x: 76, y: 18, scale: 0.8 }],
    far: [
      { part: "mesa", x: 18, y: 58, scale: 1.2 },
      { part: "mesa", x: 84, y: 58, scale: 1 },
      { part: "ruin-wall", x: 54, y: 60, scale: 1.2, paint: "stone-500" },
      { part: "column", x: 40, y: 60, scale: 1.2 },
    ],
    mid: [
      { part: "sand", x: 50, y: 60, paint: "brass-500" },
      { part: "cactus", x: 12, y: 74, scale: 0.8 },
    ],
    subject: [
      { part: "fallen-column", x: 40, y: 84, scale: 0.8 },
      { part: "lizard", x: 38, y: 80, scale: 1.1 },
      { part: "fireball", x: 66, y: 70, scale: 0.8, paint: "green-400" },
      { part: "fireball", x: 64, y: 74, scale: 0.6, paint: "violet-500" },
      { part: "smoke-billow", x: 72, y: 68, scale: 0.6, paint: "slate-400" },
    ],
    near: [
      { part: "pebbles", x: 20, y: 98, count: 2, spread: [8, 1] },
      { part: "tall-grass", x: 90, y: 98, scale: 1 },
    ],
    light: [{ part: "halo", x: 76, y: 18, scale: 1.6, paint: "white" }],
    particles: [{ part: "dust", x: 70, y: 86, scale: 1, paint: "stone-500" }],
  },
};

/** The Power to Punish (Classic #20, Rare): a stern tribunal hall, a giant balance on its dais holding a spear, a dropped card and an executioner's axe; crimson torchlight; the balance has two pans, the axe and the card in them and the spear laid at its foot. */
const THE_POWER_TO_PUNISH: SceneRecipe = {
  family: "classic",
  sky: ["wood-900", "red-900"],
  light: { x: 50, y: 34, r: 38, paint: "red-600", opacity: 0.45 },
  emblem: { x: 50, y: 74, size: 5, rotate: 0 },
  layers: {
    far: [
      { part: "tile-wall", x: 50, y: 66, paint: "slate-800" },
      { part: "column", x: 12, y: 66, scale: 2.2 },
      { part: "column", x: 88, y: 66, scale: 2.2 },
      { part: "banner", x: 28, y: 52, scale: 1.2, paint: "red-600" },
      { part: "banner", x: 68, y: 52, scale: 1.2, paint: "red-600" },
      { part: "throne", x: 50, y: 66, scale: 1.1 },
    ],
    mid: [
      { part: "tile-floor", x: 50, y: 66, paint: "slate-700" },
      { part: "pedestal", x: 50, y: 84, scale: 3, paint: "stone-500" },
      { part: "candle", x: 26, y: 76, scale: 0.8 },
      { part: "candle", x: 74, y: 76, scale: 0.8 },
    ],
    subject: [
      { part: "scales", x: 50, y: 76, scale: 2.6, paint: "brass-600" },
      { part: "axe", x: 22, y: 58, scale: 0.6 },
      { part: "playing-card", x: 76, y: 56, scale: 1 },
    ],
    near: [
      { part: "broken-spear", x: 50, y: 92, scale: 0.7 },
      { part: "fence-post", x: 6, y: 100, scale: 2.6 },
      { part: "fence-post", x: 92, y: 100, scale: 2.6, flip: -1 },
      { part: "flames", x: 8, y: 58, scale: 0.6, paint: "red-500" },
      { part: "flames", x: 94, y: 58, scale: 0.6, paint: "red-500" },
    ],
    light: [
      { part: "halo", x: 8, y: 54, scale: 1.4, paint: "red-500" },
      { part: "halo", x: 94, y: 54, scale: 1.4, paint: "red-500" },
      { part: "rim-light", x: 50, y: 60, scale: 1.2, paint: "red-400" },
    ],
    particles: [
      { part: "embers", x: 50, y: 40, count: 2, spread: [30, 8], paint: "orange-400" },
      { part: "sparks", x: 50, y: 50, count: 2, spread: [20, 6], paint: "red-400" },
    ],
  },
};

/** Turtinator (Classic #21, Common): a hulking armoured tortoise with a cannon turret bolted to its shell takes slow aim across a muddy riverbank; brown dusk, steady lamp glow. */
const TURTINATOR: SceneRecipe = {
  family: "classic",
  sky: ["slate-800", "brass-600"],
  light: { x: 20, y: 64, r: 30, paint: "orange-400", opacity: 0.4 },
  emblem: { x: 50, y: 64, size: 5, rotate: 0 },
  layers: {
    far: [
      { part: "hills", x: 50, y: 58, paint: "wood-800" },
      { part: "dead-tree", x: 84, y: 60, scale: 1.2 },
    ],
    mid: [
      { part: "water", x: 50, y: 62, paint: "slate-700" },
      { part: "sand", x: 50, y: 74, paint: "wood-700" },
      { part: "fence-post", x: 20, y: 80, scale: 1.1 },
      { part: "lantern", x: 22, y: 64, scale: 1 },
    ],
    subject: [{ part: "tortoise", x: 52, y: 82, scale: 1.3 }],
    near: [
      { part: "reed", x: 10, y: 98, scale: 1.2, count: 2, spread: [4, 1] },
      { part: "reed", x: 90, y: 98, scale: 1.1, flip: -1 },
      { part: "pebbles", x: 60, y: 98 },
    ],
    particles: [{ part: "motes", x: 24, y: 60, scale: 0.8, paint: "orange-400" }],
  },
};

/** Mid Runner (Classic #22, Common): a wiry courier in a red sash sprints down the middle lane of a three-lane stone causeway, glancing back mid-stride; windy afternoon, quick light; the sash is his red tunic. */
const MID_RUNNER: SceneRecipe = {
  family: "classic",
  sky: ["sky-400", "blue-200"],
  light: { x: 78, y: 28, r: 34, paint: "white", opacity: 0.35 },
  emblem: { x: 80, y: 64, size: 5, rotate: 0 },
  figure: { x: 50, y: 82, scale: 0.9, pose: "pose-run", look: { ...figureLookFor(["Human"]), item: null, skin: "wood-600", cloth: "red-600" } },
  layers: {
    sky: [{ part: "cloud", x: 40, y: 22, scale: 1.2, count: 3, spread: [30, 4], paint: "white" }],
    far: [
      { part: "hills", x: 50, y: 58, paint: "green-700" },
      { part: "tower", x: 86, y: 58, scale: 0.8 },
    ],
    mid: [
      { part: "lanes", x: 50, y: 58, paint: "stone-500" },
      { part: "banner", x: 16, y: 66, scale: 1, paint: "red-600" },
      { part: "flag", x: 80, y: 66, scale: 1, paint: "blue-700" },
    ],
    subject: [{ part: "dust", x: 40, y: 82, scale: 0.9, paint: "stone-500" }],
    near: [
      { part: "tall-grass", x: 8, y: 98, scale: 1.2 },
      { part: "tall-grass", x: 92, y: 98, scale: 1.1, flip: -1 },
    ],
    particles: [
      { part: "gust", x: 28, y: 44, scale: 1.2, paint: "white" },
      { part: "leaves", x: 72, y: 46, scale: 0.8 },
    ],
  },
};

/** Devil's Pact (Classic #23, Rare): an infernal notary's chamber, a horned shadow at a lectern, a contract burning in a brazier, cards on the desk becoming flaming books; ember gloom; the brazier is a crucible. */
const DEVILS_PACT: SceneRecipe = {
  family: "classic",
  sky: ["black", "red-900"],
  light: { x: 26, y: 70, r: 34, paint: "ember-500", opacity: 0.45 },
  emblem: { x: 50, y: 74, size: 5, rotate: 0 },
  figure: { x: 52, y: 78, scale: 0.85, pose: "pose-hold", look: { head: "head-void", hair: "horns", outfit: "outfit-robe", item: null, skin: "slate-900", cloth: "black" } },
  layers: {
    far: [
      { part: "tile-wall", x: 50, y: 66, paint: "wood-900" },
      { part: "bookcase", x: 12, y: 66, scale: 1.3 },
      { part: "bookcase", x: 88, y: 66, scale: 1.3 },
    ],
    mid: [{ part: "tile-floor", x: 50, y: 66, paint: "slate-800" }],
    subject: [
      { part: "crucible", x: 26, y: 84, scale: 1 },
      { part: "flames", x: 26, y: 76, scale: 0.7, paint: "ember-500" },
      { part: "scroll", x: 26, y: 72, scale: 0.6 },
    ],
    near: [
      { part: "lectern", x: 52, y: 90, scale: 2.2 },
      { part: "desk", x: 80, y: 90, scale: 1.8 },
      { part: "playing-card", x: 74, y: 74, scale: 0.9, paint: "red-900" },
      { part: "closed-tome", x: 84, y: 76, scale: 0.6, paint: "red-900" },
      { part: "flame", x: 84, y: 72, scale: 1.2 },
    ],
    light: [
      { part: "halo", x: 26, y: 72, scale: 1.6, paint: "ember-500" },
      { part: "rim-light", x: 52, y: 56, scale: 1.1, paint: "red-500" },
    ],
    particles: [{ part: "embers", x: 40, y: 56, count: 3, spread: [24, 10], paint: "ember-500" }],
  },
};

/** Book of Knowledge (Classic #24, Epic): a vast tome opens and cards fly out like birds into the outstretched hand of a young scholar perched on a library ladder, endless shelves under a domed observatory roof; bright, curious blue daylight. */
const BOOK_OF_KNOWLEDGE: SceneRecipe = {
  family: "classic",
  sky: ["blue-700", "sky-300"],
  light: { x: 50, y: 18, r: 40, paint: "white", opacity: 0.45 },
  emblem: { x: 32, y: 80, size: 5, rotate: -8 },
  figure: { x: 60, y: 70, scale: 0.6, pose: "pose-raise", look: { ...figureLookFor(["Human"]), hair: "scholar-cap", outfit: "outfit-robe", item: null, cloth: "blue-800" } },
  layers: {
    sky: [{ part: "dome", x: 50, y: 16, scale: 1.1, paint: "sky-300" }],
    far: [
      { part: "bookcase", x: 10, y: 68, scale: 1.6 },
      { part: "bookcase", x: 30, y: 66, scale: 1.4 },
      { part: "bookcase", x: 70, y: 66, scale: 1.4 },
      { part: "bookcase", x: 90, y: 68, scale: 1.6 },
      { part: "star-map", x: 50, y: 42, scale: 0.8 },
    ],
    mid: [
      { part: "boards", x: 50, y: 68, paint: "wood-700" },
      { part: "ladder", x: 60, y: 90, scale: 1.4, paint: "wood-600" },
      { part: "telescope", x: 86, y: 78, scale: 1.2 },
      { part: "globe", x: 14, y: 78, scale: 1.4 },
    ],
    subject: [
      { part: "playing-card", x: 46, y: 44, scale: 1.2, count: 5, spread: [10, 10] },
      { part: "streaks", x: 48, y: 54, scale: 0.6, paint: "white" },
    ],
    near: [
      { part: "tome", x: 32, y: 88, scale: 2.6, paint: "blue-800" },
      { part: "closed-tome", x: 82, y: 98, scale: 1 },
      { part: "quill", x: 92, y: 96, scale: 1 },
    ],
    light: [
      { part: "god-rays", x: 50, y: 0, scale: 0.9, paint: "white" },
      { part: "halo", x: 34, y: 80, scale: 1.4, paint: "sky-300" },
    ],
    particles: [
      { part: "motes", x: 50, y: 34, scale: 1.2, paint: "white" },
      { part: "sparks", x: 40, y: 62, count: 2, spread: [8, 6], paint: "sky-300" },
    ],
  },
};

/** Lag in the System (Classic #25, Epic): small cheap creatures freeze mid-step, smearing into stuttering afterimages and dissolving, on a tourney ground where only the great armoured knights stand untouched, banners frozen mid-flap, rain hanging in the air; flat grey-blue, uncanny stillness. */
const LAG_IN_THE_SYSTEM: SceneRecipe = {
  family: "classic",
  sky: ["slate-700", "blue-200"],
  light: { x: 50, y: 40, r: 36, paint: "slate-100", opacity: 0.3 },
  emblem: { x: 50, y: 30, size: 5, rotate: 0 },
  extras: [
    { layer: "mid", x: 24, y: 74, scale: 0.62, pose: "pose-guard", look: { ...KNIGHT, outfit: "outfit-plate", item: "item-sword", cloth: "slate-400" } },
    { layer: "mid", x: 78, y: 72, scale: 0.6, flip: -1, pose: "pose-guard", look: { ...KNIGHT, outfit: "outfit-plate", item: "item-lance", cloth: "blue-800" } },
  ],
  layers: {
    sky: [{ part: "bunting", x: 50, y: 30, scale: 1.2 }],
    far: [
      { part: "stands", x: 18, y: 58, scale: 1.4 },
      { part: "banner", x: 86, y: 58, scale: 1.1, paint: "slate-700" },
      { part: "banner", x: 38, y: 56, scale: 1.1, paint: "red-600" },
      { part: "flag", x: 62, y: 56, scale: 1.1, paint: "blue-700" },
    ],
    mid: [
      { part: "sand", x: 50, y: 60, paint: "stone-500" },
      { part: "fence-post", x: 50, y: 68, scale: 0.9 },
    ],
    subject: [
      { part: "goblin", x: 38, y: 78, scale: 1.6, fade: 0.3 },
      { part: "goblin", x: 42, y: 78, scale: 1.6, fade: 0.55 },
      { part: "goblin", x: 46, y: 78, scale: 1.6 },
      { part: "bunny", x: 60, y: 82, scale: 1.4, fade: 0.4 },
      { part: "bunny", x: 64, y: 82, scale: 1.4 },
    ],
    near: [
      { part: "grass-tuft", x: 14, y: 98, count: 2, spread: [5, 1], paint: "slate-700" },
      { part: "shield", x: 84, y: 94, scale: 1.2, paint: "red-900" },
    ],
    light: [{ part: "rim-light", x: 50, y: 62, scale: 1.4, paint: "slate-100" }],
    particles: [
      { part: "drops", x: 50, y: 44, scale: 0.6, count: 2, spread: [36, 14], paint: "blue-200" },
      { part: "glitch", x: 42, y: 72, scale: 0.6 },
    ],
  },
};

/** Rapid Draw (Classic #26, Common): a card duelist whips three cards from a hip-holster deck and flicks the worst one away over his shoulder; dusty crossroads, sharp afternoon light. */
const RAPID_DRAW: SceneRecipe = {
  family: "classic",
  sky: ["blue-800", "brass-500"],
  light: { x: 80, y: 34, r: 34, paint: "orange-400", opacity: 0.4 },
  emblem: { x: 18, y: 70, size: 5, rotate: 0 },
  figure: { x: 50, y: 84, scale: 0.9, pose: "pose-cast", look: { ...figureLookFor(["Human"]), hair: "hat-brim", outfit: "outfit-coat", item: "item-card", cloth: "wood-700" } },
  layers: {
    far: [
      { part: "mesa", x: 78, y: 60, scale: 1 },
      { part: "mesa", x: 20, y: 60, scale: 0.7 },
    ],
    mid: [
      { part: "sand", x: 50, y: 60, paint: "brass-600" },
      { part: "cracked-earth", x: 50, y: 66, paint: "brass-500", fade: 0.5 },
      { part: "fence-post", x: 16, y: 74, scale: 1.2 },
      { part: "cactus", x: 86, y: 74, scale: 0.9 },
    ],
    subject: [{ part: "tumbleweed", x: 66, y: 84, scale: 0.9 }],
    near: [
      { part: "deck", x: 47, y: 64, scale: 1.2 },
      { part: "playing-card", x: 34, y: 34, scale: 1.1 },
      { part: "streaks", x: 46, y: 38, scale: 0.6, flip: -1, paint: "white" },
    ],
    particles: [{ part: "dust", x: 40, y: 86, scale: 1, paint: "stone-500" }],
  },
};

/** Pestilent Slime (Classic #27, Common): a bubbling olive-green slime, dotted with weeping plague pustules, hugs a cluster of smaller blobs to itself; dripping sewer cistern, sickly green glow. */
const PESTILENT_SLIME: SceneRecipe = {
  family: "classic",
  sky: ["slate-900", "green-900"],
  light: { x: 50, y: 70, r: 32, paint: "green-400", opacity: 0.4 },
  emblem: { x: 84, y: 52, size: 5, rotate: 0 },
  layers: {
    far: [
      { part: "tile-wall", x: 50, y: 70, paint: "slate-700" },
      { part: "arch", x: 50, y: 70, scale: 2, paint: "slate-800" },
      { part: "pipes", x: 84, y: 66, scale: 0.9 },
    ],
    mid: [{ part: "water", x: 50, y: 70, paint: "green-900" }],
    subject: [
      { part: "slime", x: 50, y: 82, scale: 1.5 },
      { part: "slime", x: 32, y: 84, scale: 0.6 },
      { part: "slime", x: 66, y: 84, scale: 0.5 },
    ],
    near: [{ part: "pebbles", x: 20, y: 98, count: 2, spread: [8, 1] }],
    particles: [
      { part: "drops", x: 30, y: 30, scale: 0.6, count: 2, spread: [20, 6], paint: "green-400" },
      { part: "bubbles", x: 50, y: 60, scale: 0.8, paint: "green-400" },
    ],
  },
};

/** Second Wind (Classic #28, Legendary): a wind-scoured cliff-top ruin, a gale tearing the last cards from a fallen hero's satchel and hurling them out over the abyss while old buried cards lift from cracked cairns behind her and ride the wind back into reach; torn banners, sky-blue gusts, pale dawn breaking through storm cloud. */
const SECOND_WIND: SceneRecipe = {
  family: "classic",
  sky: ["slate-700", "blue-200"],
  light: { x: 78, y: 38, r: 38, paint: "white", opacity: 0.45 },
  emblem: { x: 46, y: 76, size: 5, rotate: 0 },
  figure: { x: 40, y: 82, scale: 0.72, pose: "pose-kneel", look: { ...figureLookFor(["Human"]), hair: "hair-long", item: null, cloth: "green-700" } },
  layers: {
    sky: [
      { part: "sun-disc", x: 80, y: 40, scale: 0.8, paint: "slate-100" },
      { part: "cloud", x: 30, y: 22, scale: 1.8, count: 3, spread: [24, 4], paint: "slate-700" },
      { part: "cloud", x: 84, y: 28, scale: 1.2, count: 2, spread: [8, 4], paint: "slate-400" },
    ],
    far: [
      { part: "ruin-wall", x: 22, y: 62, scale: 1.4, paint: "slate-700" },
      { part: "column", x: 10, y: 64, scale: 1.6 },
      { part: "standing-stone", x: 34, y: 64, scale: 0.8 },
      { part: "banner", x: 58, y: 58, scale: 1.2, paint: "red-600" },
      { part: "flag", x: 4, y: 46, scale: 1, paint: "slate-400" },
    ],
    mid: [
      { part: "cliff", x: 72, y: 62, paint: "slate-700" },
      { part: "grave-mound", x: 20, y: 70, scale: 1.4, paint: "stone-500" },
      { part: "grave-mound", x: 32, y: 72, scale: 1.2, paint: "stone-500" },
      { part: "standing-stone", x: 26, y: 70, scale: 0.5, flip: -1 },
      { part: "playing-card", x: 24, y: 56, scale: 1.2, count: 3, spread: [8, 6], fade: 0.6 },
    ],
    subject: [
      { part: "pouch", x: 50, y: 82, scale: 1.2 },
      { part: "playing-card", x: 62, y: 52, scale: 1.4, count: 4, spread: [12, 10] },
    ],
    near: [
      { part: "fallen-column", x: 16, y: 98, scale: 0.9 },
      { part: "standing-stone", x: 6, y: 92, scale: 0.6 },
      { part: "tall-grass", x: 88, y: 98, scale: 1.2, count: 2, spread: [5, 1] },
      { part: "pebbles", x: 50, y: 98 },
    ],
    light: [
      { part: "god-rays", x: 82, y: 20, scale: 0.8, paint: "white" },
      { part: "halo", x: 80, y: 38, scale: 2, paint: "sky-300" },
      { part: "light-shaft", x: 74, y: 30, scale: 1, paint: "white" },
      { part: "rim-light", x: 40, y: 64, scale: 1, paint: "sky-300" },
    ],
    particles: [
      { part: "gust", x: 50, y: 44, scale: 1.4, count: 3, spread: [30, 12], paint: "sky-300" },
      { part: "streaks", x: 76, y: 50, scale: 0.8, count: 2, spread: [8, 8], paint: "white" },
      { part: "leaves", x: 30, y: 48, scale: 0.7 },
      { part: "playing-card", x: 86, y: 40, scale: 0.9, count: 2, spread: [6, 6] },
    ],
  },
};

/** Book of Vital Kill (Classic #29, Epic): a black tome snaps shut on a hero's glowing life-thread, cutting it short, as an hourglass shatters on the floor of a vaulted clock-crypt, rows of hourglasses all stopping at once and the candles dying; cold bone-white light; the hero is the thread's far end, half seen. */
const BOOK_OF_VITAL_KILL: SceneRecipe = {
  family: "classic",
  sky: ["slate-900", "slate-700"],
  light: { x: 50, y: 44, r: 36, paint: "slate-100", opacity: 0.4 },
  emblem: { x: 52, y: 54, size: 5, rotate: 0 },
  extras: [{ layer: "mid", x: 18, y: 76, scale: 0.55, pose: "pose-stand", look: { ...HERO, cloth: "slate-400" }, fade: 0.5 }],
  layers: {
    far: [
      { part: "arch", x: 50, y: 66, scale: 2.4, paint: "slate-700" },
      { part: "clock-face", x: 50, y: 30, scale: 0.6 },
      { part: "stone-wall", x: 50, y: 52, scale: 2, paint: "slate-700" },
      { part: "hourglass", x: 50, y: 44, scale: 0.8, count: 6, spread: [38, 0] },
      { part: "candle", x: 12, y: 44, scale: 0.6 },
      { part: "candle", x: 88, y: 44, scale: 0.6 },
    ],
    mid: [{ part: "tile-floor", x: 50, y: 66, paint: "slate-700" }],
    subject: [
      { part: "red-thread", x: 22, y: 60, scale: 0.9, paint: "slate-100" },
      { part: "closed-tome", x: 52, y: 62, scale: 1.8, paint: "black" },
      { part: "hourglass", x: 66, y: 82, scale: 1.4 },
      { part: "shards", x: 66, y: 76, scale: 0.8, paint: "white" },
    ],
    near: [
      { part: "candle", x: 12, y: 97, scale: 1.2 },
      { part: "bones", x: 86, y: 97, scale: 1.2 },
      { part: "pebbles", x: 40, y: 98 },
    ],
    light: [
      { part: "halo", x: 50, y: 58, scale: 1.6, paint: "slate-100" },
      { part: "rim-light", x: 50, y: 60, scale: 1, paint: "white" },
    ],
    particles: [
      { part: "smoke", x: 12, y: 86, scale: 0.6, paint: "slate-400" },
      { part: "dust", x: 66, y: 84, scale: 0.8, paint: "slate-100" },
    ],
  },
};

/** Recycle (Classic #30, Rare): a gardener monk rakes fallen leaf-shaped cards from a graveyard plot back onto a growing stack in his barrow; terraced temple garden, fresh green morning. */
const RECYCLE: SceneRecipe = {
  family: "classic",
  sky: ["sky-400", "green-200"],
  light: { x: 24, y: 26, r: 36, paint: "white", opacity: 0.4 },
  emblem: { x: 72, y: 80, size: 5, rotate: 0 },
  figure: {
    x: 42,
    y: 84,
    scale: 0.85,
    pose: "pose-hold",
    look: { ...figureLookFor(["Human"]), hair: "tonsure", outfit: "outfit-robe", item: "item-rake", cloth: "wood-700" },
  },
  layers: {
    far: [
      { part: "terraces", x: 50, y: 56, paint: "green-700" },
      { part: "temple", x: 76, y: 46, scale: 0.6, paint: "stone-500" },
      { part: "tree", x: 18, y: 52, scale: 0.9 },
    ],
    mid: [
      { part: "field", x: 50, y: 66, paint: "green-700" },
      { part: "tombstone", x: 16, y: 78, scale: 1.8 },
      { part: "tombstone", x: 28, y: 76, scale: 1.4 },
      { part: "grave-mound", x: 22, y: 82, scale: 1.4 },
    ],
    subject: [
      { part: "cart", x: 72, y: 86, scale: 1.4 },
      { part: "deck", x: 72, y: 72, scale: 1.8, count: 2, spread: [1, 2] },
    ],
    near: [
      { part: "playing-card", x: 26, y: 90, scale: 0.8, count: 3, spread: [10, 3], paint: "green-700" },
      { part: "leaves", x: 30, y: 88, scale: 0.8 },
      { part: "grass-tuft", x: 90, y: 98, count: 2, spread: [5, 1] },
    ],
    particles: [{ part: "leaves", x: 60, y: 40, scale: 0.8 }],
  },
};

export const CLASSIC_RECIPES_1: Readonly<Record<string, SceneRecipe>> = {
  "classic-001": CURSE_OF_THE_FORGOTTEN_CLASSIC,
  "classic-002": THE_TRICKSTER,
  "classic-003": BOOK_OF_HEAL,
  "classic-004": PALANTIR,
  "classic-005": TESLA,
  "classic-006": CLOAKED_TOE_CRACKER,
  "classic-007": INFINISCEPTER,
  "classic-008": PICKLE,
  "classic-009": INCOME_TAX,
  "classic-010": EXILE,
  "classic-011": MIND_MELT,
  "classic-012": BOOK_OF_BLOOD,
  "classic-013": BOOTS_ON_THE_GROUND,
  "classic-014": SHADOWSTEP,
  "classic-015": NOSE_HUNTER,
  "classic-016": BOOK_OF_FLAME,
  "classic-017": COUNTERSPELL,
  "classic-018": GLITCH_IN_THE_SYSTEM,
  "classic-019": LIZARDS_BREATH,
  "classic-020": THE_POWER_TO_PUNISH,
  "classic-021": TURTINATOR,
  "classic-022": MID_RUNNER,
  "classic-023": DEVILS_PACT,
  "classic-024": BOOK_OF_KNOWLEDGE,
  "classic-025": LAG_IN_THE_SYSTEM,
  "classic-026": RAPID_DRAW,
  "classic-027": PESTILENT_SLIME,
  "classic-028": SECOND_WIND,
  "classic-029": BOOK_OF_VITAL_KILL,
  "classic-030": RECYCLE,
};
