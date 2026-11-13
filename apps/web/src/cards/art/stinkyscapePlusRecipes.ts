// R3572001: the held Stinkyscape+ set's placeholder scenes, one per Legendary and Mythic (40 of them), in
// Stinkyscape's deep-space look (ART.md, "The Stinkyscape+ set's placeholder scenes"). A card of the set is
// drawn by stinkyscapeplus.ts's painter until a later brief pass gives it a scene of its own; a Legendary or
// Mythic must clear its rarity's detail floor and use every layer (richness.ts), so each draws a scene from
// its family's template: the first of Terminus, Starship, Swarm, Mindustrial and Human it carries, else
// "spell". Every part is of the space, craft, gear and cosmos kits, each Radiant face keeps the base face's
// shapes, and `hashId(id)` varies each scene's offsets, flips and counts so no two are alike. A Mythic draws
// its sky, near and particle layers twice. No part turns (`rotate`), none is Stinkyscape's Mythic parts or an Alt
// art's, and no paint is gold. Presentation only (CLAUDE.md rule 7): a recipe is read by the card's id alone.

import { hashId, round2, seededRandom } from "./hash.ts";
import type { PaletteName, SceneLayer, SceneRecipe, ScenePlacement } from "./scene.ts";

/** The family a placeholder scene's template is, by the tag a card carries first of these. */
export type PlusFamily = "terminus" | "swarm" | "mindustrial" | "human" | "spell";

export type PlusRarity = "Legendary" | "Mythic";

/**
 * The 40 Legendaries and Mythics of Stinkyscape+ (no token: a token is a Terminus Starship, an Automa-ζ or drawn by the
 * painter): `[id, family, rarity]`, in card order. No Legendary or Mythic carries Starship.
 */
export const STINKY_PLUS_SCENE_CARDS: readonly (readonly [string, PlusFamily, PlusRarity])[] = [
  ["stinkyscapeplus-003", "swarm", "Legendary"], // High Evolutionary
  ["stinkyscapeplus-004", "human", "Legendary"], // Profe Yates
  ["stinkyscapeplus-005", "human", "Legendary"], // Dingleberry Droat
  ["stinkyscapeplus-007", "spell", "Legendary"], // Churnwalker
  ["stinkyscapeplus-010", "spell", "Legendary"], // Drone Assembler v5
  ["stinkyscapeplus-033", "human", "Legendary"], // True Conservative
  ["stinkyscapeplus-045", "mindustrial", "Legendary"], // Surge Alloy
  ["stinkyscapeplus-052", "mindustrial", "Legendary"], // Airblast Drill
  ["stinkyscapeplus-056", "mindustrial", "Legendary"], // Impact Reactor
  ["stinkyscapeplus-064", "mindustrial", "Legendary"], // Surge Wall
  ["stinkyscapeplus-072", "mindustrial", "Mythic"], // Foreshadow
  ["stinkyscapeplus-075", "swarm", "Legendary"], // J-lease S-Space EX-plorer
  ["stinkyscapeplus-077", "terminus", "Mythic"], // Black Hole
  ["stinkyscapeplus-078", "terminus", "Mythic"], // Slashed Dream Cries in Red
  ["stinkyscapeplus-079", "terminus", "Mythic"], // A Glimpse into the Endless Sea
  ["stinkyscapeplus-080", "terminus", "Mythic"], // The Last Voyage
  ["stinkyscapeplus-081", "spell", "Mythic"], // Doomsayer
  ["stinkyscapeplus-082", "spell", "Mythic"], // Endless Expedition
  ["stinkyscapeplus-084", "spell", "Legendary"], // Bogos Binted
  ["stinkyscapeplus-089", "terminus", "Mythic"], // Animosity
  ["stinkyscapeplus-090", "mindustrial", "Legendary"], // Meltdown
  ["stinkyscapeplus-095", "spell", "Legendary"], // JC Backed
  ["stinkyscapeplus-098", "spell", "Legendary"], // Doomed Ship
  ["stinkyscapeplus-106", "mindustrial", "Legendary"], // Spectre
  ["stinkyscapeplus-114", "mindustrial", "Legendary"], // Reign
  ["stinkyscapeplus-119", "mindustrial", "Legendary"], // Eclipse
  ["stinkyscapeplus-121", "mindustrial", "Mythic"], // Toxopid
  ["stinkyscapeplus-123", "mindustrial", "Legendary"], // Oct
  ["stinkyscapeplus-125", "mindustrial", "Legendary"], // Omura
  ["stinkyscapeplus-127", "mindustrial", "Legendary"], // Corvus
  ["stinkyscapeplus-131", "mindustrial", "Legendary"], // Tetrative Reconstructor
  ["stinkyscapeplus-132", "mindustrial", "Mythic"], // Great Paperclip Theory II
  ["stinkyscapeplus-133", "mindustrial", "Mythic"], // Great Paperclip Theory III
  ["stinkyscapeplus-139", "mindustrial", "Legendary"], // Operation Omega
  ["stinkyscapeplus-142", "spell", "Legendary"], // Call to Chaos (Mindustrial-Swarm Edition)
  ["stinkyscapeplus-145", "spell", "Legendary"], // Fire Beam v5
  ["stinkyscapeplus-148", "spell", "Legendary"], // Scourge Mortar v5
  ["stinkyscapeplus-155", "spell", "Mythic"], // The Far Vagabond
  ["stinkyscapeplus-157", "spell", "Legendary"], // Phantom Shipyard
  ["stinkyscapeplus-160", "spell", "Legendary"], // Icarus
];

/** Salts the stream that varies a scene, so it shares no draw with the scene's own (scene.ts `SCENE_SALT`). */
const PLACEHOLDER_SALT = 0x35721e01;
/** How far a placement's anchor strays from its template's, in the box's 100 units, either way. */
const OFFSET = 6;
/** How far a Mythic's second copy of its near and particle layers is moved from the first. */
const MYTHIC_SHIFT = 9;
/** Where the emblem stands: the box's upper corners, inside the band every face's window shows. */
const EMBLEM_LEFT = 18;
const EMBLEM_RIGHT = 82;
const EMBLEM_TOP = 28;
const EMBLEM_SIZE = 5;

/** What a template draws with: its offsets, flips and counts, seeded by the card's id. */
type Vary = { move(at: number): number; flip(): 1 | -1; count(low: number, high: number): number };

function varyFor(id: string): Vary {
  const rng = seededRandom((hashId(id) ^ PLACEHOLDER_SALT) >>> 0);
  return {
    move: (at) => round2(at + (rng() * 2 - 1) * OFFSET),
    flip: () => (rng() < 0.5 ? 1 : -1),
    count: (low, high) => low + Math.floor(rng() * (high - low + 1)),
  };
}

/** A family's scene: its sky and light, and its layers, each from the stream `v`. */
type Template = {
  sky: readonly [PaletteName, PaletteName];
  light: { x: number; y: number; r: number; paint: PaletteName; opacity: number };
  layers(v: Vary): Readonly<Record<SceneLayer, readonly ScenePlacement[]>>;
};

const TEMPLATES: Readonly<Record<PlusFamily, Template>> = {
  // A wormhole or a rift in a violet void, a galaxy behind and a comet across.
  terminus: {
    sky: ["black", "violet-700"],
    light: { x: 50, y: 50, r: 38, paint: "purple-400", opacity: 0.36 },
    layers: (v) => ({
      sky: [
        { part: "galaxy", x: v.move(26), y: v.move(32), scale: 1, flip: v.flip() },
        { part: "stars", x: v.move(50), y: 26, scale: 2.2, spread: [30, 12] },
        { part: "nebula", x: v.move(60), y: v.move(42), scale: 1.4, paint: "violet-500" },
      ],
      far: [
        { part: "space-rift", x: v.move(70), y: v.move(46), scale: 1.6, paint: "purple-400", flip: v.flip() },
      ],
      mid: [
        { part: "probe", x: v.move(78), y: v.move(30), scale: 0.8, flip: v.flip(), fade: 0.8 },
        { part: "wreckage", x: v.move(24), y: v.move(66), scale: 0.9, fade: 0.7 },
      ],
      subject: [
        { part: "wormhole", x: v.move(48), y: v.move(54), scale: 1.1 },
        { part: "warp-flash", x: v.move(46), y: v.move(50), scale: 1.2, paint: "sky-300" },
      ],
      near: [
        { part: "ice-shard", x: v.move(18), y: v.move(84), scale: 1.2, paint: "violet-300" },
        { part: "comet", x: v.move(72), y: v.move(30), scale: 1.1, flip: v.flip(), paint: "cyan-400" },
      ],
      light: [
        { part: "lens-flare", x: v.move(20), y: 18, scale: 0.7, paint: "white" },
        { part: "halo", x: v.move(48), y: v.move(54), scale: 2, paint: "purple-400" },
      ],
      particles: [
        { part: "sparks", x: 50, y: 50, scale: 1.4, spread: [14, 12], paint: "violet-300" },
        { part: "motes", x: v.move(50), y: 40, scale: 1.2, paint: "blue-200" },
      ],
    }),
  },
  // A hive's honeycomb and its wasps in a green dark.
  swarm: {
    sky: ["black", "green-900"],
    light: { x: 50, y: 46, r: 36, paint: "lime-400", opacity: 0.32 },
    layers: (v) => ({
      sky: [
        { part: "stars", x: v.move(40), y: 28, scale: 2, spread: [28, 10] },
        { part: "nebula", x: v.move(32), y: v.move(40), scale: 1.3, paint: "green-500" },
        { part: "planet", x: v.move(76), y: v.move(28), scale: 0.9, paint: "lime-200" },
      ],
      far: [
        { part: "honeycomb", x: 50, y: v.move(70) },
      ],
      mid: [
        { part: "space-wasp", x: v.move(30), y: v.move(46), scale: 1.2, flip: v.flip(), fade: 0.7 },
      ],
      subject: [
        { part: "space-wasp", x: v.move(50), y: v.move(52), scale: 1.8, flip: v.flip() },
        { part: "jelly-blob", x: v.move(62), y: v.move(70), scale: 1, paint: "lime-400", fade: 0.8 },
      ],
      near: [
        { part: "alien", x: v.move(20), y: v.move(86), scale: 1.2, flip: v.flip(), fade: 0.85 },
        { part: "ice-shard", x: v.move(82), y: v.move(84), scale: 1, paint: "lime-200" },
      ],
      light: [
        { part: "halo", x: v.move(50), y: v.move(50), scale: 2, paint: "lime-400" },
        { part: "lens-flare", x: v.move(78), y: 20, scale: 0.7, paint: "lime-200" },
      ],
      particles: [
        { part: "sparks", x: 50, y: 50, scale: 1.3, spread: [14, 12], paint: "lime-400" },
        { part: "motes", x: v.move(50), y: 40, scale: 1.2, paint: "green-200" },
      ],
    }),
  },
  // A foundry deck under gantries, a reactor and a mech, sparks and embers rising.
  mindustrial: {
    sky: ["slate-900", "wood-900"],
    light: { x: 50, y: 56, r: 36, paint: "orange-400", opacity: 0.38 },
    layers: (v) => ({
      sky: [
        { part: "stars", x: v.move(50), y: 24, scale: 1.8, spread: [30, 8] },
        { part: "nebula", x: v.move(70), y: v.move(30), scale: 1.2, paint: "red-900" },
        { part: "alarm-light", x: v.move(14), y: 26, scale: 1, flip: v.flip() },
      ],
      far: [
        { part: "gantry", x: v.move(16), y: 80, scale: 1.6 },
        { part: "gantry", x: v.move(84), y: 80, scale: 1.6, flip: -1 },
      ],
      mid: [
        { part: "reactor", x: v.move(50), y: v.move(62), scale: 1.1, paint: "orange-400" },
        { part: "shipyard-crane", x: v.move(82), y: v.move(80), scale: 1.2, flip: -1 },
      ],
      subject: [
        { part: "mech", x: v.move(52), y: v.move(80), scale: 1.3, flip: v.flip() },
        { part: "mining-drill", x: v.move(26), y: v.move(70), scale: 0.8 },
      ],
      near: [
        { part: "cable", x: v.move(30), y: 94, scale: 1.1 },
        { part: "ore-chunk", x: v.move(80), y: v.move(86), scale: 1.3, paint: "cyan-400" },
      ],
      light: [
        { part: "halo", x: v.move(50), y: v.move(58), scale: 2, paint: "orange-400" },
        { part: "energy-beam", x: v.move(50), y: 22, scale: 0.85, paint: "orange-400" },
      ],
      particles: [
        { part: "sparks", x: 50, y: 50, scale: 1.4, spread: [16, 12], paint: "orange-400" },
        { part: "embers", x: v.move(50), y: 56, scale: 1.2, count: 2, spread: [14, 10] },
      ],
    }),
  },
  // A crew under a viewport on a station's deck, a holo table before them.
  human: {
    sky: ["blue-900", "slate-900"],
    light: { x: 50, y: 36, r: 36, paint: "sky-300", opacity: 0.32 },
    layers: (v) => ({
      sky: [
        { part: "stars", x: v.move(50), y: 26, scale: 2.2, spread: [30, 10] },
        { part: "planet", x: v.move(74), y: v.move(32), scale: 0.9, paint: "blue-200" },
        { part: "galaxy", x: v.move(26), y: v.move(34), scale: 0.8, flip: v.flip() },
      ],
      far: [
        { part: "viewport", x: 50, y: v.move(40), scale: 2.1 },
        { part: "city-skyline", x: 50, y: v.move(78), scale: 1.1, paint: "slate-800", fade: 0.7 },
      ],
      mid: [
        { part: "console", x: v.move(18), y: v.move(66), scale: 0.8, flip: v.flip() },
        { part: "catwalk", x: 50, y: v.move(82) },
      ],
      subject: [
        { part: "holo-table", x: v.move(50), y: v.move(84), scale: 1.3 },
        { part: "robot", x: v.move(70), y: v.move(62), scale: 0.8, flip: -1 },
      ],
      near: [
        { part: "crew-crowd", x: v.move(50), y: 100, scale: 1.5 },
        { part: "console", x: v.move(86), y: 100, scale: 1, flip: -1 },
      ],
      light: [
        { part: "halo", x: v.move(50), y: v.move(52), scale: 2, paint: "sky-300" },
        { part: "rim-light", x: v.move(48), y: 62, scale: 1, paint: "blue-200" },
      ],
      particles: [
        { part: "sparks", x: 50, y: 50, scale: 1.3, spread: [14, 12], paint: "sky-300" },
        { part: "motes", x: v.move(50), y: 40, scale: 1.2, paint: "blue-200" },
      ],
    }),
  },
  // A starship before a warp gate over a planet's limb, a lens flare across.
  spell: {
    sky: ["black", "blue-900"],
    light: { x: 50, y: 44, r: 38, paint: "cyan-400", opacity: 0.34 },
    layers: (v) => ({
      sky: [
        { part: "galaxy", x: v.move(24), y: v.move(32), scale: 1, flip: v.flip() },
        { part: "stars", x: v.move(52), y: 26, scale: 2.2, spread: [30, 10] },
        { part: "nebula", x: v.move(66), y: v.move(40), scale: 1.3, paint: "blue-600" },
      ],
      far: [
        { part: "warp-gate", x: v.move(44), y: v.move(44), scale: 1.4, fade: 0.8 },
      ],
      mid: [
        { part: "satellite", x: v.move(80), y: v.move(30), scale: 0.9, flip: v.flip() },
        { part: "freighter", x: v.move(78), y: v.move(62), scale: 0.7, flip: -1, paint: "slate-400", fade: 0.7 },
      ],
      subject: [
        { part: "starship", x: v.move(50), y: v.move(54), scale: 1.3, flip: v.flip() },
        { part: "thruster-flame", x: v.move(34), y: v.move(54), scale: 1.1, paint: "sky-300" },
      ],
      near: [
        { part: "ore-chunk", x: v.move(20), y: v.move(84), scale: 1.2, paint: "cyan-400" },
        { part: "speed-streaks", x: v.move(40), y: v.move(52), scale: 1.2, paint: "sky-300" },
      ],
      light: [
        { part: "lens-flare", x: v.move(20), y: 18, scale: 0.8, paint: "white" },
        { part: "halo", x: v.move(50), y: v.move(52), scale: 2, paint: "cyan-400" },
      ],
      particles: [
        { part: "sparks", x: 50, y: 50, scale: 1.4, spread: [14, 12], paint: "sky-300" },
        { part: "comet", x: v.move(72), y: v.move(34), scale: 1, flip: v.flip(), paint: "cyan-400" },
      ],
    }),
  },
};

/** A Mythic's layer drawn twice: the copy moved by `MYTHIC_SHIFT` and flipped, so the layer reads richer, never a repeat. */
function doubled(layer: readonly ScenePlacement[]): readonly ScenePlacement[] {
  return [...layer, ...layer.map((placed) => ({ ...placed, x: round2(placed.x + MYTHIC_SHIFT), flip: placed.flip === -1 ? (1 as const) : (-1 as const) }))];
}

/**
 * The placeholder scene of `id`: its family's template, varied by `hashId(id)`, family `stinkyscape`, all seven layers;
 * a Mythic doubles its sky, near and particle layers.
 */
export function placeholderRecipe(id: string, family: PlusFamily, rarity: PlusRarity): SceneRecipe {
  const template = TEMPLATES[family];
  const layers = template.layers(varyFor(id));
  const left = hashId(id) % 2 === 0;
  return {
    family: "stinkyscape",
    sky: template.sky,
    light: template.light,
    emblem: { x: left ? EMBLEM_LEFT : EMBLEM_RIGHT, y: EMBLEM_TOP, size: EMBLEM_SIZE, rotate: 0 },
    layers:
      rarity === "Mythic"
        ? { ...layers, sky: doubled(layers.sky), near: doubled(layers.near), particles: doubled(layers.particles) }
        : layers,
  };
}

/** R3572001: every Stinkyscape+ Legendary and Mythic's scene, by id. */
export const STINKY_PLUS_RECIPES: Readonly<Record<string, SceneRecipe>> = Object.fromEntries(
  STINKY_PLUS_SCENE_CARDS.map(([id, family, rarity]) => [id, placeholderRecipe(id, family, rarity)] as const),
);
