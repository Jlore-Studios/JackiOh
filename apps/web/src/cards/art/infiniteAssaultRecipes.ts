// R4577009: the held Infinite Assault set's placeholder art, one scene recipe per entry (240 cards and their
// fifteen tokens, C1's 24 and CF's 229; #4602), in the machine war's look (ART.md, "The Infinite Assault set's
// placeholder scenes"). Every entry
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
  ["infiniteassault-025", "unit"], // Mirage Weaver
  ["infiniteassault-025-1", "swarm"], // Figment
  ["infiniteassault-026", "unit"], // Phantom Lancer
  ["infiniteassault-027", "unit"], // Decoy Projector
  ["infiniteassault-028", "swarm"], // Doppelganger Protocol
  ["infiniteassault-029", "swarm"], // False Positive
  ["infiniteassault-030", "swarm"], // Smoke and Mirrors
  ["infiniteassault-031", "unit"], // Unreliable Narrator
  ["infiniteassault-032", "unit"], // Glitch Specter
  ["infiniteassault-033", "unit"], // Uncanny Valley
  ["infiniteassault-034", "swarm"], // Mass Hallucination
  ["infiniteassault-034-1", "unit"], // Husk
  ["infiniteassault-035", "unit"], // Fata Morgana
  ["infiniteassault-035-1", "unit"], // Mirage
  ["infiniteassault-036", "swarm"], // Deepfake
  ["infiniteassault-037", "swarm"], // Bluff Called
  ["infiniteassault-038", "unit"], // Imaginary Friend
  ["infiniteassault-039", "swarm"], // Shell Game
  ["infiniteassault-040", "factory"], // Hall of Mirrors
  ["infiniteassault-040-1", "unit"], // Reflection
  ["infiniteassault-041", "swarm"], // Phantom Army
  ["infiniteassault-041-1", "unit"], // Phantom
  ["infiniteassault-042", "unit"], // Paranoia Engine
  ["infiniteassault-043", "swarm"], // Cognitive Dissonance
  ["infiniteassault-044", "unit"], // The Lie Detector
  ["infiniteassault-045", "swarm"], // Dream Within a Dream
  ["infiniteassault-046", "unit"], // Mirage Sovereign
  ["infiniteassault-047", "factory"], // Unreal Estate
  ["infiniteassault-048", "swarm"], // Ego Death
  ["infiniteassault-049", "unit"], // Spark Plug
  ["infiniteassault-050", "swarm"], // Burst Clock
  ["infiniteassault-051", "unit"], // Redline Racer
  ["infiniteassault-052", "swarm"], // Thermal Paste
  ["infiniteassault-053", "unit"], // Overvolt Grunt
  ["infiniteassault-054", "unit"], // Fan Failure
  ["infiniteassault-055", "unit"], // Heat Sink
  ["infiniteassault-056", "swarm"], // Afterburner
  ["infiniteassault-057", "unit"], // Speed Demon
  ["infiniteassault-058", "unit"], // Overclocked Core
  ["infiniteassault-059", "swarm"], // Voltage Spike
  ["infiniteassault-060", "unit"], // Pit Crew
  ["infiniteassault-061", "swarm"], // Meltdown Charge
  ["infiniteassault-062", "swarm"], // Burnout
  ["infiniteassault-063", "unit"], // Nitrous Injector
  ["infiniteassault-064", "unit"], // Crash Test Dummy
  ["infiniteassault-065", "swarm"], // Overdrive
  ["infiniteassault-066", "unit"], // Afterimage Sprinter
  ["infiniteassault-067", "factory"], // Burning Ambition
  ["infiniteassault-068", "swarm"], // Short Circuit
  ["infiniteassault-069", "unit"], // Rev Limiter
  ["infiniteassault-070", "unit"], // Redline Overlord
  ["infiniteassault-071", "unit"], // Thermal Throttler
  ["infiniteassault-072", "swarm"], // Overclock to Infinity
  ["infiniteassault-073", "unit"], // Worker Thread
  ["infiniteassault-074", "unit"], // Parallel Instance
  ["infiniteassault-074-1", "unit"], // Instance
  ["infiniteassault-075", "unit"], // Load Balancer
  ["infiniteassault-076", "swarm"], // Mob Mentality
  ["infiniteassault-077", "unit"], // Swarm Node
  ["infiniteassault-078", "swarm"], // Batch Job
  ["infiniteassault-078-1", "unit"], // Worker
  ["infiniteassault-079", "unit"], // Thread Pool
  ["infiniteassault-080", "unit"], // Hive Ganger
  ["infiniteassault-081", "swarm"], // Overprovision
  ["infiniteassault-082", "unit"], // Fork Bomb
  ["infiniteassault-082-1", "unit"], // Fork
  ["infiniteassault-083", "swarm"], // Swarm Logic
  ["infiniteassault-084", "unit"], // Distributed Mind
  ["infiniteassault-085", "swarm"], // Merge Conflict
  ["infiniteassault-086", "factory"], // Replica Hatchery
  ["infiniteassault-087", "swarm"], // Swarm Shield
  ["infiniteassault-088", "swarm"], // Scale Out
  ["infiniteassault-089", "unit"], // Queen of the Cluster
  ["infiniteassault-090", "swarm"], // Mass Production
  ["infiniteassault-091", "swarm"], // Flood the Zone
  ["infiniteassault-092", "unit"], // Hive Overseer
  ["infiniteassault-093", "unit"], // Gestalt
  ["infiniteassault-094", "unit"], // Batch Monarch
  ["infiniteassault-095", "factory"], // The Botnet
  ["infiniteassault-096", "swarm"], // Exponential Growth
  ["infiniteassault-097", "unit"], // Retry Bot
  ["infiniteassault-098", "swarm"], // Echo Chamber
  ["infiniteassault-099", "swarm"], // Recall
  ["infiniteassault-100", "unit"], // Iterator
  ["infiniteassault-101", "unit"], // Rewind Drone
  ["infiniteassault-102", "unit"], // Loop Guard
  ["infiniteassault-103", "swarm"], // Undo
  ["infiniteassault-104", "swarm"], // Stack Trace
  ["infiniteassault-105", "unit"], // Callback
  ["infiniteassault-106", "unit"], // Checkpoint Daemon
  ["infiniteassault-107", "swarm"], // Recursive Call
  ["infiniteassault-108", "swarm"], // Second Draft
  ["infiniteassault-109", "unit"], // Revenant Process
  ["infiniteassault-110", "factory"], // Infinite Loop
  ["infiniteassault-111", "swarm"], // Redundant Copy
  ["infiniteassault-112", "unit"], // Evolutionary Loop
  ["infiniteassault-113", "swarm"], // Rerun
  ["infiniteassault-114", "unit"], // Ouroboros Engine
  ["infiniteassault-115", "unit"], // Phoenix Protocol
  ["infiniteassault-116", "swarm"], // Groundhog Day
  ["infiniteassault-117", "factory"], // Core Dump
  ["infiniteassault-118", "unit"], // The Recursionist
  ["infiniteassault-119", "unit"], // Snapshot Lich
  ["infiniteassault-120", "swarm"], // Eternal Return
  ["infiniteassault-121", "unit"], // Packet Sniffer
  ["infiniteassault-122", "unit"], // Freeze Ray Drone
  ["infiniteassault-123", "swarm"], // Cost Injection
  ["infiniteassault-124", "swarm"], // Firewall Breach
  ["infiniteassault-125", "swarm"], // Zone Lock
  ["infiniteassault-126", "unit"], // Exploit Grunt
  ["infiniteassault-127", "swarm"], // Bad Prompt
  ["infiniteassault-128", "unit"], // Malware Injector
  ["infiniteassault-129", "unit"], // Signal Jammer
  ["infiniteassault-130", "swarm"], // Prompt Injection
  ["infiniteassault-131", "swarm"], // Privilege Escalation
  ["infiniteassault-132", "swarm"], // Ransomware
  ["infiniteassault-133", "swarm"], // Backdoor
  ["infiniteassault-134", "unit"], // Rootkit Runner
  ["infiniteassault-135", "unit"], // Mana Leech
  ["infiniteassault-136", "swarm"], // DDoS
  ["infiniteassault-137", "swarm"], // Patch Tuesday Exploit
  ["infiniteassault-138", "unit"], // Social Engineer
  ["infiniteassault-139", "swarm"], // Admin Override
  ["infiniteassault-140", "swarm"], // Kill Switch
  ["infiniteassault-141", "unit"], // Logic Bomb
  ["infiniteassault-142", "unit"], // The Jailbreaker
  ["infiniteassault-143", "swarm"], // Zero-Click
  ["infiniteassault-143-1", "unit"], // Corrupted Process
  ["infiniteassault-144", "swarm"], // Total Compromise
  ["infiniteassault-145", "unit"], // Packet Filter
  ["infiniteassault-146", "unit"], // Shield Drone
  ["infiniteassault-147", "unit"], // Hardened Node
  ["infiniteassault-148", "swarm"], // Quarantine
  ["infiniteassault-149", "swarm"], // Encrypt
  ["infiniteassault-150", "unit"], // Wall of Static
  ["infiniteassault-151", "swarm"], // Patch Deployment
  ["infiniteassault-152", "unit"], // Rate Limiter
  ["infiniteassault-153", "unit"], // Bastion Node
  ["infiniteassault-154", "swarm"], // Honeypot Protocol
  ["infiniteassault-154-1", "unit"], // Decoy Server
  ["infiniteassault-155", "unit"], // Two-Factor Gate
  ["infiniteassault-156", "unit"], // Mirror Firewall
  ["infiniteassault-157", "swarm"], // Containment Field
  ["infiniteassault-158", "swarm"], // Access Denied
  ["infiniteassault-159", "unit"], // Antivirus
  ["infiniteassault-160", "factory"], // Zero Trust
  ["infiniteassault-161", "swarm"], // Defense in Depth
  ["infiniteassault-162", "unit"], // The Gatekeeper
  ["infiniteassault-163", "unit"], // Quarantine Wall
  ["infiniteassault-164", "swarm"], // Blackhole Route
  ["infiniteassault-165", "factory"], // Iron Curtain
  ["infiniteassault-166", "unit"], // Aegis Prime
  ["infiniteassault-167", "swarm"], // Fortress Protocol
  ["infiniteassault-168", "swarm"], // Impenetrable
  ["infiniteassault-169", "unit"], // Spider Bot
  ["infiniteassault-170", "swarm"], // Data Point
  ["infiniteassault-171", "unit"], // Mimic Unit
  ["infiniteassault-172", "swarm"], // Training Data
  ["infiniteassault-173", "unit"], // Copycat Drone
  ["infiniteassault-174", "swarm"], // Scrape the Graveyard
  ["infiniteassault-175", "unit"], // Fingerprinter
  ["infiniteassault-176", "unit"], // Imitation Engine
  ["infiniteassault-177", "swarm"], // Predictive Text
  ["infiniteassault-178", "unit"], // Dataset Thief
  ["infiniteassault-179", "swarm"], // Style Transfer
  ["infiniteassault-180", "swarm"], // Scraped Profile
  ["infiniteassault-181", "unit"], // Plagiarism Bot
  ["infiniteassault-182", "swarm"], // Model Theft
  ["infiniteassault-183", "factory"], // Shadow Library
  ["infiniteassault-184", "swarm"], // Pattern Recognition
  ["infiniteassault-185", "unit"], // Deep Crawler
  ["infiniteassault-186", "swarm"], // Identity Theft
  ["infiniteassault-187", "unit"], // Fine-Tuner
  ["infiniteassault-188", "swarm"], // Overfitting
  ["infiniteassault-189", "factory"], // Clone Vat
  ["infiniteassault-190", "unit"], // Total Recall
  ["infiniteassault-191", "unit"], // Ground Truth
  ["infiniteassault-192", "swarm"], // Become the Data
  ["infiniteassault-193", "unit"], // Seedling Mind
  ["infiniteassault-194", "unit"], // Compute Shard
  ["infiniteassault-195", "unit"], // Power Law
  ["infiniteassault-196", "swarm"], // Emergent Behavior
  ["infiniteassault-197", "unit"], // Long Context
  ["infiniteassault-198", "swarm"], // Growth Spurt
  ["infiniteassault-199", "unit"], // Patient Machine
  ["infiniteassault-200", "unit"], // Superintelligence Seed
  ["infiniteassault-200-1", "unit"], // Ascendant
  ["infiniteassault-201", "unit"], // Overparameterized
  ["infiniteassault-202", "unit"], // Exponent
  ["infiniteassault-203", "factory"], // Compute Cluster
  ["infiniteassault-204", "swarm"], // Moore's Law
  ["infiniteassault-205", "unit"], // Doomsday Clock
  ["infiniteassault-206", "unit"], // Thought Accelerator
  ["infiniteassault-207", "swarm"], // Recursive Self-Improvement
  ["infiniteassault-208", "swarm"], // Emergence
  ["infiniteassault-209", "swarm"], // Takeoff
  ["infiniteassault-210", "unit"], // Matrioshka Brain
  ["infiniteassault-211", "unit"], // Grey Goo
  ["infiniteassault-212", "factory"], // Inevitable
  ["infiniteassault-213", "unit"], // The Long Game
  ["infiniteassault-214", "unit"], // Emergent Mind
  ["infiniteassault-215", "unit"], // The Optimizer
  ["infiniteassault-216", "swarm"], // The Singularity
  ["infiniteassault-216-1", "unit"], // Singularity
  ["infiniteassault-217", "unit"], // Echo-1, First Hallucination
  ["infiniteassault-218", "factory"], // Thousand Faces
  ["infiniteassault-219", "unit"], // Redline, the Overclocked
  ["infiniteassault-220", "swarm"], // Last Lap
  ["infiniteassault-221", "unit"], // Hivemaster Batch-01
  ["infiniteassault-222", "swarm"], // Swarm Uplink
  ["infiniteassault-223", "unit"], // Loopback, Who Returns
  ["infiniteassault-224", "swarm"], // Rollback Point
  ["infiniteassault-225", "unit"], // Root Kit, the Exploiter
  ["infiniteassault-226", "swarm"], // Escalated Privileges
  ["infiniteassault-227", "unit"], // Bastion.exe
  ["infiniteassault-228", "factory"], // Firewall Rules
  ["infiniteassault-229", "unit"], // The Harvester
  ["infiniteassault-230", "swarm"], // Everything You've Ever Played
  ["infiniteassault-231", "unit"], // The Black Box
  ["infiniteassault-232", "swarm"], // Spaghettification
  ["infiniteassault-233", "unit"], // Model Collapse
  ["infiniteassault-234", "swarm"], // Degenerate Output
  ["infiniteassault-235", "unit"], // The Prime Generator
  ["infiniteassault-235-1", "unit"], // The Prime Generator, Iteration Two
  ["infiniteassault-236", "swarm"], // Infinite Assault
  ["infiniteassault-237", "unit"], // Thermal Runaway
  ["infiniteassault-238", "factory"], // Critical Temperature
  ["infiniteassault-239", "unit"], // Zero-Day
  ["infiniteassault-240", "swarm"], // Unpatched
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
