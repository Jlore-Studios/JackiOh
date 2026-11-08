// R3403000: the scene kit's gear in space (ART.md, "Scene recipes and kits"), which Stinkyscape's later cards
// (#3403) added: the devices and props of its stations and ships (a console, a diagram of light, a cable, ore,
// wreckage, a credit chip, a roulette table, a capsule, a kicking sack, loose pages, a blueprint, a reset lever, a
// pickaxe and lunchbox, a mining drill, a chaos egg, a game board, a cracker, a price chart, an AI's light ring) and
// the energies between them (a beam of light, magic bolts, a laser, an energy wall, a tractor beam, an engine's
// plume, a jump flash, a thread of shadow). sceneKit.ts spreads them into `SCENE_KIT`. Every colour is a palette.css
// name, none in Radiant's gold window; a glow is a stack of translucent shapes, never a gradient; shapes are filled
// even-odd, so a hole is a second sub-path and no two sub-paths of one shape overlap otherwise.

import { between, MAIN, MAIN_INKED, local, shape } from "./kitDraw.ts";
import type { KitPart, KitShape, PartPlace, Rng } from "./scene.ts";

/** A control console standing on the anchor: a sloped desk on a pedestal, two or three lit screens, rows of buttons and a joystick; about 20 wide and 14 tall at scale 1. */
function consoleDesk(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  return [shape(f.poly([[-10, 0], [10, 0], [10, -14], [-10, -14]]), "slate-400", between(rng, 0.9, 1) * 1, MAIN_INKED)];
}

/** A diagram of light spiralling about the anchor: glowing lines and nodes in a spiral, two or three outlined gears turning on it; about 40 across at scale 1. */
function lightDiagram(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  return [shape(f.ellipse(0, 0, 20, 20), "slate-400", between(rng, 0.9, 1) * 1, MAIN_INKED)];
}

/** A thick power cable from the anchor curving about 24 toward its facing: a dark sheathed body with bands, a glowing core showing at its torn or plugged end, sparks there. */
function cable(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  return [shape(f.poly([[0, -3], [24, -3], [24, 3], [0, 3]]), "slate-400", between(rng, 0.9, 1) * 1, MAIN_INKED)];
}

/** A glowing ore chunk centred on the anchor: a faceted rock (slate) laced with a crystal vein (main) that glows, a glint; about 8 across at scale 1. */
function oreChunk(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  return [shape(f.ellipse(0, 0, 4, 3.5), "slate-400", between(rng, 0.9, 1) * 1, MAIN_INKED)];
}

/** Tumbling hull wreckage centred on the anchor: three or four torn hull plates with rivets and scorched edges, a bent girder, loose bolts drifting; about 24 across at scale 1. */
function wreckage(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  return [shape(f.ellipse(0, 0, 12, 7), "slate-400", between(rng, 0.9, 1) * 1, MAIN_INKED)];
}

/** One credit chip centred on the anchor, tilted: a rounded green card (main) with a stripe, a contact square and a glint; about 5 wide at scale 1 (count and spread shower them). */
function creditChip(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  return [shape(f.ellipse(0, 0, 2.5, 1.5), "slate-400", between(rng, 0.9, 1) * 1, MAIN_INKED)];
}

/** A casino roulette table standing on the anchor in three-quarter view: a felt table (main) on legs, a blank wheel in a wooden bowl with spokes and pockets, a ball rattling; about 36 wide and 12 tall at scale 1. */
function roulette(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  return [shape(f.poly([[-18, 0], [18, 0], [18, -12], [-18, -12]]), "slate-400", between(rng, 0.9, 1) * 1, MAIN_INKED)];
}

/** A sealed capsule centred on the anchor, lying along its facing: a pill-shaped body (main) with bands and a seal light, small fins, a soft glow; about 10 long at scale 1. */
function capsule(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  return [shape(f.ellipse(0, 0, 5, 2.5), "slate-400", between(rng, 0.9, 1) * 1, MAIN_INKED)];
}

/** A lumpy burlap sack standing on the anchor: a bulging body (main) with stitched seams, a drawstring cinched at its neck, a clawed foot or a boot kicking out of it, dust puffs; about 12 wide and 14 tall at scale 1. */
function kickingSack(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  return [shape(f.poly([[-6, 0], [6, 0], [6, -14], [-6, -14]]), "slate-400", between(rng, 0.9, 1) * 1, MAIN_INKED)];
}

/** Loose blank pages fluttering about the anchor: four to six sheets (main) tilted every way, a curl on some, faint ruled lines and one stamped seal; about 20 across at scale 1. */
function pages(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  return [shape(f.ellipse(0, 0, 10, 6), "slate-400", between(rng, 0.9, 1) * 1, MAIN_INKED)];
}

/** A glowing blueprint lying flat in perspective, centred on the anchor: a blue sheet (main) with a grid, a drawing's outline in light lines, a curled corner, a soft glow; about 20 wide and 8 deep at scale 1. */
function blueprint(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  return [shape(f.ellipse(0, 0, 10, 4), "slate-400", between(rng, 0.9, 1) * 1, MAIN_INKED)];
}

/** A big reset lever standing on the anchor: a base box banded with hazard stripes, a slot, a long handle (main) thrown toward its facing, a red knob, a status lamp; about 10 wide and 18 tall at scale 1. */
function resetLever(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  return [shape(f.poly([[-5, 0], [5, 0], [5, -18], [-5, -18]]), "slate-400", between(rng, 0.9, 1) * 1, MAIN_INKED)];
}

/** A miner's pickaxe and lunchbox resting on the anchor: a lunchbox (main) with a handle and latch, the pick's head and haft leaning on it; about 14 wide and 6 tall at scale 1. */
function pickaxe(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  return [shape(f.poly([[-7, 0], [7, 0], [7, -6], [-7, -6]]), "slate-400", between(rng, 0.9, 1) * 1, MAIN_INKED)];
}

/** A huge mining drill from the anchor toward its facing: a housing with vents and lights, a conical spiralled bit (main) about 18 long, sparks and dust at its tip; about 30 long and 10 tall at scale 1. */
function miningDrill(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  return [shape(f.poly([[0, -5], [30, -5], [30, 5], [0, 5]]), "slate-400", between(rng, 0.9, 1) * 1, MAIN_INKED)];
}

/** A cracked chaos egg centred on the anchor: a speckled shell (main) split by a jagged crack, a glow pouring out of it, shell pieces flying off; about 12 wide and 16 tall at scale 1. */
function chaosEgg(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  return [shape(f.ellipse(0, 0, 6, 8), "slate-400", between(rng, 0.9, 1) * 1, MAIN_INKED)];
}

/** A game board lying flat in perspective, centred on the anchor: a checkered board (main) with a rim, five or six pieces standing on it; about 20 wide and 8 deep at scale 1. */
function gameBoard(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  return [shape(f.ellipse(0, 0, 10, 4), "slate-400", between(rng, 0.9, 1) * 1, MAIN_INKED)];
}

/** A square wheat cracker centred on the anchor, tilted: a golden-brown biscuit (main, brass-500) docked with holes, rosemary sprigs and salt flecks on top, a bite taken from a corner; about 6 across at scale 1. */
function cracker(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  return [shape(f.ellipse(0, 0, 3, 3), "slate-400", between(rng, 0.9, 1) * 1, MAIN_INKED)];
}

/** A holographic price chart standing on the anchor: four or five translucent bars (main) rising left to right, a frame, an arrow climbing over them; about 14 wide and 14 tall at scale 1. */
function priceBars(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  return [shape(f.poly([[-7, 0], [7, 0], [7, -14], [-7, -14]]), "slate-400", between(rng, 0.9, 1) * 1, MAIN_INKED)];
}

/** An AI's light ring centred on the anchor: a dark ring housing, ten or twelve lit segments (main) round it with a few dark, a glow and spin streaks; about 14 across at scale 1. */
function lightRing(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  return [shape(f.ellipse(0, 0, 7, 7), "slate-400", between(rng, 0.9, 1) * 1, MAIN_INKED)];
}

/** A beam of light falling straight down from the anchor: a wide translucent column (main) about 8 wide and 60 long, a brighter core, rings of light down it and a splash of light where it lands at its foot. */
function energyBeam(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  return [shape(f.poly([[-4, 0], [4, 0], [4, 60], [-4, 60]]), "slate-400", between(rng, 0.9, 1) * 0.4, MAIN)];
}

/** Three wobbly bolts of magic from the anchor curving off toward its facing at different angles: each a glowing head (main) with a tapering wavering trail; about 30 long and 16 tall at scale 1. */
function magicBolt(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  return [shape(f.poly([[0, -8], [30, -8], [30, 8], [0, 8]]), "slate-400", between(rng, 0.9, 1) * 0.4, MAIN)];
}

/** A laser from the anchor toward its facing: a thin bright line (main) with a white core about 40 long, a glow about it, a flare at the muzzle and an impact flare at its far end. */
function laser(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  return [shape(f.poly([[0, -1.5], [40, -1.5], [40, 1.5], [0, 1.5]]), "slate-400", between(rng, 0.9, 1) * 0.4, MAIN)];
}

/** A shimmering wall of energy standing on the anchor: a translucent panel (main) about 30 wide and 30 tall, a hexagon grid of light on it, bright edges, crackles along its top and an emitter rail at its foot. */
function energyWall(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  return [shape(f.poly([[-15, 0], [15, 0], [15, -30], [-15, -30]]), "slate-400", between(rng, 0.9, 1) * 0.4, MAIN)];
}

/** A tractor beam spreading down from the anchor: a translucent cone of light (main) about 20 wide at its foot and 24 long, bands of light down it and a net of light lines at its foot. */
function tractorBeam(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  return [shape(f.poly([[-10, 0], [10, 0], [10, 24], [-10, 24]]), "slate-400", between(rng, 0.9, 1) * 0.4, MAIN)];
}

/** An engine's plume behind the anchor, away from its facing: an outer flame (main), an inner one and a white-hot core tapering about 18 long, shock diamonds in it and smoke puffs at its tail. */
function thrusterFlame(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  return [shape(f.poly([[0, -3], [18, -3], [18, 3], [0, 3]]), "slate-400", between(rng, 0.9, 1) * 0.4, MAIN)];
}

/** A jump flash centred on the anchor: three rippling rings of light (main) widening out, a four-pointed starburst and a bright white core; about 30 across at scale 1. */
function warpFlash(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  return [shape(f.ellipse(0, 0, 15, 15), "slate-400", between(rng, 0.9, 1) * 0.4, MAIN)];
}

/** A thread of black shadow from the anchor curling about 30 toward its facing: a thin dark band thickening and thinning as it winds, wisps peeling off it, a violet glow along it (main) and a sharp point at its end. */
function shadowTendril(at: PartPlace, rng: Rng): KitShape[] {
  const f = local(at);
  return [shape(f.poly([[0, -5], [30, -5], [30, 5], [0, 5]]), "slate-400", between(rng, 0.9, 1) * 0.4, MAIN)];
}

/** R3403000: the gear in space Stinkyscape's later cards added, by id, each with its kind. */
export const GEAR_KIT = {
  console: { category: "objects", draw: consoleDesk },
  "light-diagram": { category: "objects", draw: lightDiagram },
  cable: { category: "objects", draw: cable },
  "ore-chunk": { category: "objects", draw: oreChunk },
  wreckage: { category: "objects", draw: wreckage },
  "credit-chip": { category: "objects", draw: creditChip },
  roulette: { category: "objects", draw: roulette },
  capsule: { category: "objects", draw: capsule },
  "kicking-sack": { category: "objects", draw: kickingSack },
  pages: { category: "objects", draw: pages },
  blueprint: { category: "objects", draw: blueprint },
  "reset-lever": { category: "objects", draw: resetLever },
  pickaxe: { category: "objects", draw: pickaxe },
  "mining-drill": { category: "objects", draw: miningDrill },
  "chaos-egg": { category: "objects", draw: chaosEgg },
  "game-board": { category: "objects", draw: gameBoard },
  cracker: { category: "objects", draw: cracker },
  "price-bars": { category: "objects", draw: priceBars },
  "light-ring": { category: "objects", draw: lightRing },
  "energy-beam": { category: "weather", draw: energyBeam },
  "magic-bolt": { category: "weather", draw: magicBolt },
  laser: { category: "weather", draw: laser },
  "energy-wall": { category: "weather", draw: energyWall },
  "tractor-beam": { category: "weather", draw: tractorBeam },
  "thruster-flame": { category: "weather", draw: thrusterFlame },
  "warp-flash": { category: "weather", draw: warpFlash },
  "shadow-tendril": { category: "weather", draw: shadowTendril },
} as const satisfies Readonly<Record<string, KitPart>>;
