// R3403000: the scene kit's gear in space (ART.md, "Scene recipes and kits"), which Stinkyscape's later cards
// (#3403) added: the devices and props of its stations and ships (a console, a hologram, a light diagram, a cable,
// ore, wreckage, credit chips, a roulette table, a capsule, a sack, loose pages, a blueprint, a reset lever, a
// poster, pennants, an emblem, a pickaxe, a mining drill, the chaos egg and orb, a game board, a floating island
// station) and the beams and energies between them (an energy beam, a magic bolt, a laser, a blast, a tractor
// beam, an energy wall, a shield bubble, a thruster's flame, scan rings, a warp flash). sceneKit.ts spreads them into
// `SCENE_KIT`. Every colour is a palette.css name, none in Radiant's gold window; a glow is a stack of translucent
// shapes, never a gradient; shapes are filled even-odd, so a hole is a second sub-path and no two sub-paths of one
// shape overlap otherwise.

import type { KitPart } from "./scene.ts";

export const GEAR_KIT = {} as const satisfies Readonly<Record<string, KitPart>>;
