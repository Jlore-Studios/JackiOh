// R4577009: the held Infinite Assault set's placeholder art, one scene recipe per entry (24 cards and their two
// Drone tokens), in the machine war's look (ART.md, "The Infinite Assault set's placeholder scenes"). Every entry
// draws its scene through the procedural painter (`artSpec` reads a card's recipe by its id), so no dispatch site
// changes and no painter of its own is written: each entry clears its rarity's detail floor (richness.ts), the two
// Legendaries in every layer. A scene is its type's template (a unit's drone bay, a spell's or a trap's drone swarm,
// a field's production line) built from the space, craft and gear kits, and `hashId(id)` varies its offsets, flips
// and counts so no two are alike. No part turns (`rotate`) and no paint is gold. Presentation only (CLAUDE.md rule
// 7): a recipe is read by the card's id alone, and only ids under `INFINITE_ASSAULT_PREFIX` are keys here, so no
// shipped card's picture moves (R503). A later brief pass replaces them.

import { hashId, round2, seededRandom } from "./hash.ts";
import type { SceneLayer, ScenePlacement, SceneRecipe } from "./scene.ts";

export const INFINITE_ASSAULT_PREFIX = "infiniteassault-";

/** The template a placeholder scene is drawn from, by the card's type: a Trap is drawn as a spell. */
export type AssaultTemplate = "unit" | "swarm" | "factory";

/** Every entry of Infinite Assault, its tokens included, with its template, in card order. */
export const INFINITE_ASSAULT_SCENE_CARDS: readonly (readonly [string, AssaultTemplate])[] = [
  ["infiniteassault-001", "unit"], // Seed Drone
  ["infiniteassault-002", "unit"], // Scrap Prototype
  ["infiniteassault-003", "unit"], // Assault Trooper
  ["infiniteassault-004", "unit"], // Wave Spawner
  ["infiniteassault-004-1", "unit"], // Drone
  ["infiniteassault-005", "swarm"], // Generated Filler
  ["infiniteassault-006", "swarm"], // Hotfix
  ["infiniteassault-007", "swarm"], // Bulk Order
  ["infiniteassault-008", "swarm"], // Firing Solution
  ["infiniteassault-009", "unit"], // Heavy Lifter Mk.II
  ["infiniteassault-010", "unit"], // Assault Carrier
  ["infiniteassault-010-1", "unit"], // Drone Escort
  ["infiniteassault-011", "unit"], // Quickgen Turret
  ["infiniteassault-012", "swarm"], // Recompile
  ["infiniteassault-013", "swarm"], // Training Run
  ["infiniteassault-014", "swarm"], // Overflow Error
  ["infiniteassault-015", "factory"], // Drone Foundry
  ["infiniteassault-016", "swarm"], // Benchmark Run
  ["infiniteassault-017", "unit"], // Cluster Drone
  ["infiniteassault-018", "unit"], // Prompt Engineer
  ["infiniteassault-019", "swarm"], // Infinite Scroll
  ["infiniteassault-020", "swarm"], // Synthetic Data
  ["infiniteassault-021", "factory"], // Endless Production Line
  ["infiniteassault-022", "unit"], // Assault Mainframe
  ["infiniteassault-023", "unit"], // The Generator
  ["infiniteassault-024", "swarm"], // Mass Deployment
];

/** Salts the stream that varies a scene, so it shares no draw with the scene's own (scene.ts `SCENE_SALT`). */
const PLACEHOLDER_SALT = 0x45770901;
/** How far a placement's anchor strays from its template's, in the box's 100 units, either way. */
const OFFSET = 6;
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

type Layers = Readonly<Record<SceneLayer, readonly ScenePlacement[]>>;

/** A template's scene: its sky and light, and its layers, each from the stream `v`. */
type Template = Pick<SceneRecipe, "sky" | "light"> & { layers(v: Vary): Layers };

const TEMPLATES: Readonly<Record<AssaultTemplate, Template>> = {
  // A war machine stepping off its rack in a drone bay, alarms red and sparks flying.
  unit: {
    sky: ["slate-900", "red-900"],
    light: { x: 50, y: 54, r: 36, paint: "red-400", opacity: 0.34 },
    layers: (v) => ({
      sky: [
        { part: "ceiling-lights", x: 50, y: v.move(12), scale: 1.2 },
        { part: "alarm-light", x: v.move(14), y: 24, scale: 1, flip: v.flip() },
      ],
      far: [
        { part: "drone-rack", x: v.move(20), y: v.move(56), scale: 1.2 },
        { part: "drone-rack", x: v.move(80), y: v.move(56), scale: 1.2, flip: -1 },
      ],
      mid: [
        { part: "robot-arm", x: v.move(26), y: v.move(74), scale: 0.9, flip: v.flip(), fade: 0.8 },
        { part: "drone", x: v.move(72), y: v.move(36), scale: 0.7, count: v.count(1, 2), spread: [10, 6], fade: 0.75 },
      ],
      subject: [
        { part: "combat-robot", x: v.move(50), y: v.move(80), scale: 1.25, flip: v.flip() },
        { part: "ai-eye", x: v.move(50), y: v.move(40), scale: 0.6, paint: "red-400" },
      ],
      near: [
        { part: "cable", x: v.move(28), y: 94, scale: 1.1 },
        { part: "robot-wreck", x: v.move(82), y: v.move(90), scale: 0.9, flip: v.flip() },
      ],
      light: [
        { part: "halo", x: v.move(50), y: v.move(58), scale: 2, paint: "red-400" },
        { part: "energy-beam", x: v.move(50), y: 20, scale: 0.8, paint: "cyan-400" },
      ],
      particles: [
        { part: "sparks", x: 50, y: 52, scale: 1.4, spread: [16, 12], paint: "orange-400" },
        { part: "motes", x: v.move(50), y: 40, scale: 1.2, paint: "cyan-400" },
      ],
    }),
  },
  // A swarm of drones and interceptors pouring through a gate over a burning city, the machine's eye behind.
  swarm: {
    sky: ["black", "blue-900"],
    light: { x: 50, y: 42, r: 38, paint: "cyan-400", opacity: 0.32 },
    layers: (v) => ({
      sky: [
        { part: "stars", x: v.move(50), y: 24, scale: 2, spread: [30, 10] },
        { part: "nebula", x: v.move(66), y: v.move(36), scale: 1.3, paint: "red-900" },
      ],
      far: [
        { part: "warp-gate", x: v.move(50), y: v.move(42), scale: 1.3, fade: 0.8 },
        { part: "city-skyline", x: 50, y: v.move(84), scale: 1.1, paint: "slate-800", fade: 0.7 },
      ],
      mid: [
        { part: "interceptor", x: v.move(24), y: v.move(34), scale: 0.8, flip: v.flip(), fade: 0.8 },
        { part: "escort-disc", x: v.move(76), y: v.move(62), scale: 0.8, flip: v.flip(), fade: 0.75 },
      ],
      subject: [
        { part: "drone", x: v.move(50), y: v.move(54), scale: 1.3, flip: v.flip() },
        { part: "drone", x: v.move(32), y: v.move(62), scale: 0.9, count: v.count(1, 2), spread: [12, 8] },
        { part: "ai-eye", x: v.move(50), y: v.move(36), scale: 0.7, paint: "cyan-400" },
      ],
      near: [
        { part: "missile", x: v.move(20), y: v.move(80), scale: 1, flip: v.flip() },
        { part: "speed-streaks", x: v.move(60), y: v.move(56), scale: 1.2, paint: "sky-300" },
      ],
      light: [
        { part: "laser", x: v.move(70), y: v.move(70), scale: 1, paint: "red-400" },
        { part: "halo", x: v.move(50), y: v.move(50), scale: 2, paint: "cyan-400" },
      ],
      particles: [
        { part: "sparks", x: 50, y: 50, scale: 1.3, spread: [14, 12], paint: "sky-300" },
        { part: "motes", x: v.move(50), y: 40, scale: 1.2, paint: "blue-200" },
      ],
    }),
  },
  // A production line without end: printers and arms feeding an AI core under a crane, drones rolling off it.
  factory: {
    sky: ["slate-900", "wood-900"],
    light: { x: 50, y: 50, r: 36, paint: "orange-400", opacity: 0.36 },
    layers: (v) => ({
      sky: [
        { part: "ceiling-lights", x: 50, y: v.move(12), scale: 1.2 },
      ],
      far: [
        { part: "bulkhead", x: 50, y: v.move(50), scale: 1.2 },
        { part: "shipyard-crane", x: v.move(80), y: v.move(76), scale: 1.2, flip: -1 },
      ],
      mid: [
        { part: "printer", x: v.move(24), y: v.move(70), scale: 0.9, flip: v.flip() },
        { part: "drone-rack", x: v.move(76), y: v.move(60), scale: 0.9, fade: 0.8 },
      ],
      subject: [
        { part: "ai-core", x: v.move(50), y: v.move(62), scale: 1.2 },
        { part: "robot-arm", x: v.move(32), y: v.move(70), scale: 0.9 },
        { part: "robot-arm", x: v.move(68), y: v.move(70), scale: 0.9, flip: -1 },
      ],
      near: [
        { part: "console", x: v.move(16), y: 100, scale: 1, flip: v.flip() },
        { part: "drone", x: v.move(70), y: v.move(88), scale: 0.8, count: v.count(1, 2), spread: [10, 4] },
      ],
      light: [
        { part: "halo", x: v.move(50), y: v.move(56), scale: 2, paint: "orange-400" },
        { part: "light-ring", x: v.move(50), y: v.move(60), scale: 1, paint: "cyan-400" },
      ],
      particles: [
        { part: "sparks", x: 50, y: 52, scale: 1.4, spread: [16, 12], paint: "orange-400" },
        { part: "embers", x: v.move(50), y: 56, scale: 1.2, count: 2, spread: [14, 10] },
      ],
    }),
  },
};

/** The placeholder scene of `id`: its template, varied by `hashId(id)`, family `stinkyscape`, all seven layers. */
export function assaultRecipe(id: string, template: AssaultTemplate): SceneRecipe {
  const { sky, light, layers } = TEMPLATES[template];
  return {
    family: "stinkyscape",
    sky,
    light,
    emblem: { x: hashId(id) % 2 === 0 ? EMBLEM_LEFT : EMBLEM_RIGHT, y: EMBLEM_TOP, size: EMBLEM_SIZE, rotate: 0 },
    layers: layers(varyFor(id)),
  };
}

/** R4577009: every Infinite Assault entry's scene, by id. */
export const INFINITE_ASSAULT_RECIPES: Readonly<Record<string, SceneRecipe>> = Object.fromEntries(
  INFINITE_ASSAULT_SCENE_CARDS.map(([id, template]) => [id, assaultRecipe(id, template)] as const),
);
