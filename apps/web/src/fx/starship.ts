// R2801000: the Starship's moment, Stinkyscape's signature system (R1852000, R1852003). Pieces fusing
// into one Dormant Starship play as one fuse run (game/runs.ts `fuseRunAt`). A launch is the Starship's
// set piece (`starshipClaim`, R2785000), and the ship lifts off and lands as its launched self as the
// hold ends. The Starship and its launch are public (R202): no cue names a card's definition, and a
// Starship the seat cannot place (R97's sentinel) plans nothing. Everything starts inside its entry and
// ends within FX_MAX_TAIL_MS of it (R200); the effects speed scales it with the entry (R201).
//
// R3334000–R3334003 (issue #3334) make each moment the ship's own, drawn from its picture (R3333000):
// - a fused piece's module flies to its own hardpoint, clamps on and welds, its hum rising with the count;
// - held at Big moments "full", the launch is a cinematic: the board dims, the ship's picture fills the
//   screen, the camera passes each module, its engines light in turn, its name stamps across the hull in
//   its design's lettering and it climbs out of frame; otherwise (and under Reduce Motion) a still title
//   card of its picture and name. A War Doctrine it casts is announced over it;
// - a launched Starship's attack is a broadside, each mounted weapon firing by its kind, or, unarmed, a
//   ram prow first;
// - a destroyed Starship is a wreck: its newest modules blow off, the hull breaks in two, debris drifts
//   over its backrow and a scorch settles on its zone.
// Each grows with the pieces it holds (`STARSHIP_TUNING`), and a phone throws fewer particles.

import type { GameEvent, PlayerView, Row, UnitView } from "@jackioh/shared";

import { STARSHIP_DESIGNS, starshipArtOf, type StarshipArt, type StarshipDesignId } from "../cards/art/starshipHulls.ts";
import { MODULE_KINDS, type ModuleKind } from "../cards/art/starshipModules.ts";
import {
  starshipCenterPoint,
  starshipEnginePoints,
  starshipMountPoints,
  starshipProwPoint,
  type StarshipMountPoint,
  type StarshipPoint,
} from "../cards/art/stinkyscape.ts";
import { starshipArtUri, starshipModuleUri } from "../cards/art/svg.ts";
import {
  STARSHIP_FUSE_MS,
  STARSHIP_FUSE_STAGGERED_MAX,
  STARSHIP_LAUNCH_HOLD_MS,
  STARSHIP_SET_PIECE,
  animTestid,
  starshipFuseMs,
  type AnimationEntry,
  type SetPieceClaim,
} from "../game/animations.ts";
import { sideOf, testid, type Side } from "../game/contract.ts";
import type { StarshipFuse, StarshipLaunch } from "../game/runs.ts";
import { CATALOG } from "../wire/cards.ts";
import { frac, ringCue, shakeCues, tid, tunedBurst, type BurstTuning } from "./build.ts";
import {
  FX_CENTER,
  FX_LUNGE_CONTACT_AT,
  FX_MAX_TAIL_MS,
  FX_STARSHIP_BADGE_AT,
  FX_STARSHIP_FUSE_LAND_AT,
  FX_STARSHIP_FUSE_STAGGER_MS,
  FX_STARSHIP_FUSE_TRAUMA,
  FX_STARSHIP_IGNITE_AT,
  FX_STARSHIP_LAND_AT,
  FX_STARSHIP_LIFT_AT,
  FX_STARSHIP_STILL_MS,
  FX_STARSHIP_TAIL_MS,
} from "./constants.ts";
import { FLAVOR_LOOKS, paletteLook, type FlavorLook } from "./flavors.ts";
import { MOTIF_FOOT, MOTIF_TUNING } from "./motifs/build.ts";
import { tintOf, type SetPiecePlan } from "./setPieces.ts";
import type {
  FxAnchor,
  FxCinematicCue,
  FxCue,
  FxLaunchCue,
  FxPlanEnv,
  FxPreset,
  FxShot,
  FxShotSprite,
  FxTimedPoint,
  FxTint,
  StarshipLettering,
  StarshipSize,
  WarDoctrine,
} from "./types.ts";

/**
 * One size of Starship: the fewest pieces it takes, its exhaust burst, how many card heights its contrail
 * climbs, the contrail's density and the shake; R3334001: how close its cinematic's camera draws its
 * picture, and the sparks each of its engines throws there.
 */
export type StarshipTuning = { pieces: number; exhaust: BurstTuning; contrail: number; density: number; trauma: number; zoom: number; sparks: number };

/** R2801000, R3334001: a bigger ship, more exhaust, a longer contrail, a heavier shake and a closer, longer pan, at intensity "normal" (rule 9). */
export const STARSHIP_TUNING: { readonly [S in StarshipSize]: StarshipTuning } = {
  small: { pieces: 0, exhaust: { count: 24, power: 0.9 }, contrail: 1.5, density: 0.6, trauma: 0.45, zoom: 1.5, sparks: 6 },
  medium: { pieces: 3, exhaust: { count: 44, power: 1.15 }, contrail: 2.5, density: 0.85, trauma: 0.65, zoom: 1.8, sparks: 9 },
  large: { pieces: 6, exhaust: { count: 72, power: 1.4 }, contrail: 4, density: 1.1, trauma: 0.9, zoom: 2.2, sparks: 12 },
};

/** The sizes, smallest first. */
const SIZES: readonly StarshipSize[] = ["small", "medium", "large"];

/** R2801000: every burst a fuse run throws, at intensity "normal" (rule 9). */
export const STARSHIP_FUSE_TUNING = { spark: { count: 18, power: 1.1 } } as const;

/** R3334000: a docking module is drawn this share of its ship's art wide; its weld lasts `weldMs`; its hum is full at `humFull` pieces. */
export const STARSHIP_DOCK = { moduleShare: 0.22, weldMs: 320, humFull: 8 } as const;

/**
 * R3334001: the cinematic, as shares of its hold: the camera pans stern to bow from `panFrom` to `panTo`,
 * the engines light one after another from `enginesFrom` to `enginesTo` (at most `enginesMax`), the name
 * stamps at `stampAt` and the ship climbs out at `climbAt`; the cue outlasts the hold by `tailMs`, into the
 * lift-off. A phone throws `phoneShare` of every particle.
 */
export const STARSHIP_CINEMATIC = {
  panFrom: 0.06,
  panTo: 0.55,
  enginesFrom: 0.55,
  enginesTo: 0.7,
  stampAt: 0.68,
  climbAt: 0.86,
  enginesMax: 4,
  tailMs: 300,
  phoneShare: 0.5,
} as const;

/**
 * R3334002: a broadside, as shares of the attack's row: the guns fire one after another from `fireAt`
 * (jk-lunge's 24% keyframe) to `lastAt`, at most `gunsMax`; a beam sweeps `beamSweep` strokes
 * `beamStep` apart, a repeater stutters `repeaterShots` bolts `repeaterStep` apart, flak bursts in
 * `flakClouds` clouds, spread across the target at `aimY` of its height; a cannon kicks back for
 * `recoilMs` and shakes the board `cannonTrauma`, a ram `ramTrauma`.
 */
export const STARSHIP_BROADSIDE = {
  fireAt: 0.24,
  aimY: 0.5,
  lastAt: 0.5,
  gunsMax: 8,
  beamSweep: 3,
  beamStep: 0.05,
  repeaterShots: 4,
  repeaterStep: 0.05,
  flakClouds: 3,
  recoilMs: 180,
  cannonTrauma: 0.3,
  ramTrauma: 0.5,
} as const;

/**
 * R3334003: a wreck, as shares of its `destroyed` row: its newest `modulesMax` modules blow off one at a
 * time from `blowFrom` to `blowTo`, each flung `fling` of its picture away over `flingMs`; the hull breaks
 * at `breakAt`, each half turning `halfTurnDeg` away, the wreck gone `tailMs` after the row; debris drifts
 * over its backrow at `debrisAt`, and a scorch settles on its zone at `scorchAt`, gone `scorchTailMs`
 * after the row. The break shakes the board `trauma`.
 */
export const STARSHIP_WRECK = {
  modulesMax: 6,
  blowFrom: 0.05,
  blowTo: 0.5,
  breakAt: 0.6,
  flingMs: 700,
  fling: 0.35,
  halfTurnDeg: 12,
  tailMs: 800,
  debrisAt: 0.8,
  scorchAt: 0.9,
  scorchTailMs: 850,
  trauma: 0.45,
} as const;

/** R3334001: a War Doctrine's announcement outlasts its `cardPlayed` by `tailMs`; its burst lands at `stampAt` of it and shakes `trauma`. */
export const STARSHIP_DOCTRINE = { tailMs: 700, stampAt: 0.3, trauma: 0.4 } as const;

/** R3334000–R3334003: every burst the Starship's moments throw, at intensity "normal" (rule 9). */
export const STARSHIP_MOMENT_TUNING = {
  weld: { count: 10, power: 0.9 },
  muzzle: { count: 12, power: 1 },
  flak: { count: 14, power: 0.6 },
  ramExhaust: { count: 16, power: 1 },
  ramSpark: { count: 18, power: 1.2 },
  blowOff: { count: 10, power: 1.1 },
  wreckSmoke: { count: 30, power: 0.8 },
  debris: { count: 24, power: 0.7 },
  doctrine: { count: 28, power: 1 },
} as const;

/** A Starship's picture's side, in its own units (stinkyscape.ts `BOX`): its points are shares of it. */
const PICTURE_UNITS = 100;

/** The middle of a Starship's picture, as a share of its side. */
const PICTURE_MIDDLE = 0.5;

/** R3334001: each launched design's lettering, as its name suits (`StarshipLettering`). */
export const STARSHIP_LETTERING: { readonly [D in Exclude<StarshipDesignId, "shipyard">]: StarshipLettering } = {
  "ninth-life": { font: "signature", weight: 400, tracking: 0, italic: false },
  "frontiers-promise": { font: "display", weight: 800, tracking: 0.12, italic: false },
  "celestial-archive": { font: "display", weight: 600, tracking: 0.04, italic: true },
  "lantern-ferry": { font: "brush", weight: 400, tracking: 0, italic: false },
  "jlockeed-skylark": { font: "text", weight: 800, tracking: 0.02, italic: true },
  "ember-of-departure": { font: "marker", weight: 400, tracking: 0.02, italic: false },
  "silent-choir": { font: "copperplate", weight: 400, tracking: 0.06, italic: false },
};

/** The design whose lettering a name no launched design has would fall back on (`starshipDesignOf` gives every launch one). */
const LETTERING_FALLBACK = "ember-of-departure" satisfies keyof typeof STARSHIP_LETTERING;

/** R3334001: Stinkyscape's four War Doctrines, by their tokens' ids. */
export const WAR_DOCTRINES: Readonly<Record<string, WarDoctrine>> = {
  "stinkyscape-137-1": "land",
  "stinkyscape-137-2": "sea",
  "stinkyscape-137-3": "air",
  "stinkyscape-137-4": "space",
};

/** R3334001: each doctrine's look: its insignia's tint and the burst it lands with. */
export const WAR_DOCTRINE_LOOKS: { readonly [D in WarDoctrine]: { tint: FxTint; preset: FxPreset } } = {
  land: { tint: { rim: "#6b4f2a", core: "#f2d49b", glow: "#c8923a" }, preset: "dust" },
  sea: { tint: { rim: "#0f3a6b", core: "#bfe9ff", glow: "#2f8fd8" }, preset: "frost" },
  air: { tint: { rim: "#5a6f8a", core: "#ffffff", glow: "#9fd2ff" }, preset: "sparkle" },
  space: { tint: { rim: "#2a1a5a", core: "#e8d8ff", glow: "#8a5aff" }, preset: "star" },
};

/** R3334002: how a mounted weapon fires: its motion, what a launcher lobs, and the burst it lands in. */
export type BroadsideFire = "beam" | "repeater" | "cannon" | "flak" | "launcher";
export type BroadsideWeapon = { fire: BroadsideFire; preset: FxPreset; sprite?: FxShotSprite };

/** R3334002: the modules that fire in a broadside, each as its nearest kind (the odd weapons among them). */
export const BROADSIDE_WEAPONS: Readonly<Partial<Record<ModuleKind, BroadsideWeapon>>> = {
  beam: { fire: "beam", preset: "spark" },
  disintegrex: { fire: "beam", preset: "poison" },
  trila: { fire: "beam", preset: "fire" },
  repeater: { fire: "repeater", preset: "spark" },
  "ap-rounds": { fire: "repeater", preset: "shard" },
  "vamp-rounds": { fire: "repeater", preset: "blood" },
  cannon: { fire: "cannon", preset: "fire" },
  rail: { fire: "cannon", preset: "arcane" },
  yamato: { fire: "cannon", preset: "prismatic" },
  flak: { fire: "flak", preset: "smoke" },
  "rush-launcher": { fire: "launcher", preset: "spark", sprite: "token" },
  "fruit-launcher": { fire: "launcher", preset: "leaf", sprite: "fruit" },
  "jade-launcher": { fire: "launcher", preset: "prismatic", sprite: "jade" },
  "book-launcher": { fire: "launcher", preset: "arcane", sprite: "book" },
  "missile-pod": { fire: "launcher", preset: "fire", sprite: "arrow" },
};

/** R2801000: the size of a Starship of `pieces` pieces: the largest step it reaches. */
export function starshipSizeOf(pieces: number): StarshipSize {
  let size: StarshipSize = "small";
  for (const step of SIZES) if (pieces >= STARSHIP_TUNING[step].pieces) size = step;
  return size;
}

/** R2801000, R2785000: a launch the seat can place claims its entry as the Starship's set piece. */
export function starshipClaim(entry: AnimationEntry): SetPieceClaim | null {
  const anchor = entry.launch?.anchor;
  return anchor === undefined || anchor === null ? null : { key: STARSHIP_SET_PIECE, holdMs: STARSHIP_LAUNCH_HOLD_MS };
}

/** A unit the board shows on `side`: each of its units, then each Unit standing face up in its backrow (R1081000). */
function shownUnits(view: PlayerView, side: Side): { unit: UnitView; row: Row; lane: number }[] {
  const sv = view[side];
  const out: { unit: UnitView; row: Row; lane: number }[] = [];
  sv.units.forEach((unit, k) => {
    if (unit !== null) out.push({ unit, row: "units", lane: k + 1 });
  });
  sv.backrow.forEach((slot, k) => {
    const held = slot !== null && !slot.faceDown ? slot.unit : undefined;
    if (held !== undefined && held !== null) out.push({ unit: held, row: "backrow", lane: k + 1 });
  });
  return out;
}

/** A unit that is a Starship: its memory set. */
export type StarshipUnit = UnitView & { starship: NonNullable<UnitView["starship"]> };

/** R3334000: the Starship `id` as `view` shows it, with its side, row and lane; null for any other card, or one it does not show (R97). */
export function starshipUnitOf(view: PlayerView, id: string): { unit: StarshipUnit; side: Side; row: Row; lane: number } | null {
  for (const side of ["you", "opponent"] as const) {
    for (const shown of shownUnits(view, side)) {
      const ship = shown.unit.starship;
      if (shown.unit.instanceId === id && ship !== undefined) return { unit: { ...shown.unit, starship: ship }, side, row: shown.row, lane: shown.lane };
    }
  }
  return null;
}

/** R3334002: whether `view` shows `id` as a launched Starship. */
export function isLaunchedStarship(view: PlayerView, id: string): boolean {
  return starshipUnitOf(view, id)?.unit.starship.launched !== undefined;
}

/** R2801000: `side`'s Dormant Starship as the board draws it (a unit, else a backrow Unit), or null. */
export function dormantStarshipOf(view: PlayerView, side: Side): string | null {
  const sv = view[side];
  const unit = sv.units.find((u) => u !== null && u.starship !== undefined && u.starship.launched === undefined);
  if (unit != null) return testid.card(unit.instanceId);
  for (const slot of sv.backrow) {
    const held = slot !== null && !slot.faceDown ? slot.unit : undefined;
    if (held?.starship !== undefined && held.starship.launched === undefined) return testid.card(held.instanceId);
  }
  return null;
}

/** R3334001: `side`'s launched Starship as the board draws it, the latest launched (R1852005's "the launched Starship"), or null. */
export function launchedStarshipOf(view: PlayerView, side: Side): string | null {
  let best: { id: string; seq: number } | null = null;
  for (const { unit } of shownUnits(view, side)) {
    const seq = unit.starship?.launched;
    if (seq !== undefined && (best === null || seq > best.seq)) best = { id: unit.instanceId, seq };
  }
  return best === null ? null : testid.card(best.id);
}

/** R3334000: a Starship's picture's inputs as `unit` shows it, `launched` under `name` or Dormant. */
export function shipArtOf(unit: UnitView, launched: boolean, name: string | undefined): StarshipArt {
  const pieces = (unit.starship?.pieces ?? []).map((piece) => ({ defId: piece.defId, radiant: piece.radiant === true }));
  return starshipArtOf(pieces, launched, name, unit.radiant);
}

/** R3334002: one gun of a broadside: the module that fires, how it fires and where it sits on the picture. */
export type BroadsideGun = StarshipMountPoint & { weapon: BroadsideWeapon };

/** R3334002: the weapons a Starship's picture mounts, list order, at most `gunsMax`. */
export function broadsideGuns(ship: StarshipArt): BroadsideGun[] {
  const guns: BroadsideGun[] = [];
  for (const mount of starshipMountPoints(ship)) {
    const weapon = BROADSIDE_WEAPONS[mount.kind];
    if (weapon !== undefined && guns.length < STARSHIP_BROADSIDE.gunsMax) guns.push({ ...mount, weapon });
  }
  return guns;
}

/** R3334002: the broadside `attackerId` fires as `view` shows it (none: it rams), or null when it is no launched Starship. */
export function starshipBroadside(view: PlayerView, attackerId: string): BroadsideGun[] | null {
  const found = starshipUnitOf(view, attackerId);
  if (found === null || found.unit.starship.launched === undefined) return null;
  return broadsideGuns(shipArtOf(found.unit, true, found.unit.starship.name));
}

/** R3334002: when gun `j` of `G` fires, in an attack row of `D`. */
export function broadsideFireMs(j: number, G: number, D: number): number {
  const { fireAt, lastAt } = STARSHIP_BROADSIDE;
  return frac(fireAt + ((lastAt - fireAt) * j) / Math.max(1, G - 1), D);
}

/** R2801000: when piece `k` of a fuse run of `n` leaves, in a run of `D` (the budget may have squeezed it: every beat keeps its share). */
export function starshipFuseStartMs(k: number, n: number, D: number): number {
  const s = D / starshipFuseMs(n);
  return Math.round(Math.min(k, STARSHIP_FUSE_STAGGERED_MAX - 1) * FX_STARSHIP_FUSE_STAGGER_MS * s);
}

/** R2801000, R3334000: when piece `k` of a fuse run of `n` docks, in a run of `D`. */
export function starshipFuseLandMs(k: number, n: number, D: number): number {
  const s = D / starshipFuseMs(n);
  return starshipFuseStartMs(k, n, D) + Math.round(STARSHIP_FUSE_MS * FX_STARSHIP_FUSE_LAND_AT * s);
}

/** R3334000: a point of a Starship's picture on the card `testid` shows it on. */
function picture(at: string, p: StarshipPoint): FxAnchor {
  return { kind: "testid", testid: at, at: { x: p.x / PICTURE_UNITS, y: p.y / PICTURE_UNITS }, art: "picture" };
}

/** Tech's rocket look (#2792): the Starship's exhaust and sparks wear it. */
function rocketLook(): FlavorLook {
  return paletteLook("Tech/rocket") ?? FLAVOR_LOOKS.Tech;
}

type Ctx = { i: number; D: number; share: number };

/** A burst of the Starship's moments: a phone (`share`) throws fewer particles, never none. */
function burst(c: Ctx, preset: FxPreset, at: FxAnchor, spread: "point" | "area" | "ring", delayMs: number, tuning: BurstTuning, colors?: readonly string[]): FxCue {
  const cue = tunedBurst(c.i * c.share, preset, at, spread, delayMs, tuning);
  return colors === undefined ? cue : { ...cue, colors };
}

function planFuse(fuse: StarshipFuse, entry: AnimationEntry, view: PlayerView, c: Ctx): SetPiecePlan | null {
  if (fuse.anchor === null || fuse.pieces.length === 0) return null;
  const { i, D } = c;
  const look = rocketLook();
  const to = tid(fuse.anchor);
  const n = fuse.pieces.length;
  const graveyard = tid(animTestid.graveyard(sideOf(view, fuse.player)));
  // The list the ship holds after the run: the view's pieces, then each fusion's, in order.
  const fusions = entry.events.filter((event): event is Extract<GameEvent, { type: "starshipFused" }> => event.type === "starshipFused");
  const held = starshipUnitOf(view, fuse.shipId)?.unit;
  const before = (held?.starship.pieces ?? []).map((record) => ({ defId: record.defId, radiant: record.radiant === true }));
  const list = [...before, ...fusions.map((event) => ({ defId: event.defId, radiant: false }))];
  const ship = starshipArtOf(list, false, undefined, held?.radiant ?? false);
  const mounts = starshipMountPoints(ship);
  const center = starshipCenterPoint(ship);
  const cues: FxCue[] = [];
  fuse.pieces.forEach((piece, k) => {
    const start = starshipFuseStartMs(k, n, D);
    const land = starshipFuseLandMs(k, n, D);
    const index = before.length + k;
    const point = mounts[index] ?? center;
    const defId = fusions[k]?.defId ?? list[index]?.defId ?? "";
    const dock = picture(fuse.anchor ?? "", point);
    const weldMs = Math.min(STARSHIP_DOCK.weldMs, D - land + FX_MAX_TAIL_MS);
    cues.push(
      {
        kind: "module",
        uri: starshipModuleUri(defId, false),
        motion: "dock",
        share: STARSHIP_DOCK.moduleShare,
        from: piece.from === null ? graveyard : tid(piece.from),
        to: dock,
        landMs: land - start,
        spin: 0,
        delayMs: start,
        durationMs: land - start + weldMs,
      },
      { kind: "weld", at: dock, hum: Math.min(1, (fuse.count - n + k + 1) / STARSHIP_DOCK.humFull), delayMs: land, durationMs: weldMs },
      burst(c, "spark", dock, "point", land, STARSHIP_MOMENT_TUNING.weld, look.colors),
      { ...tunedBurst(i, "spark", to, "point", land, STARSHIP_FUSE_TUNING.spark), colors: look.colors },
      ringCue(D, "spark", to, land),
      ...shakeCues(i, FX_STARSHIP_FUSE_TRAUMA, land),
    );
  });
  const replaces = new Set<GameEvent>(entry.events.filter((event) => event.type === "fused" || event.type === "starshipFused"));
  return { cues, replaces };
}

/** The launch cue: the ignition and the flame and contrail of a ship of `size` (R3334001: its countdown is gone). */
function launchCue(at: FxLaunchCue["at"], size: StarshipSize, igniteMs: number, durationMs: number): FxLaunchCue {
  return { kind: "launch", at, tint: tintOf(rocketLook()), size, countsMs: [], igniteMs, delayMs: 0, durationMs };
}

/** R3334001: the art, the name and its lettering a launch's cinematic or title card shows, from the launch and the ship as the view shows it. */
function launchArt(launch: StarshipLaunch, entry: AnimationEntry, view: PlayerView): { ship: StarshipArt; name: string } {
  const event = entry.events.find((e): e is Extract<GameEvent, { type: "starshipLaunched" }> => e.type === "starshipLaunched");
  const held = starshipUnitOf(view, launch.shipId)?.unit;
  const ship =
    held !== undefined
      ? shipArtOf(held, true, launch.name)
      : starshipArtOf((event?.pieces ?? []).map((defId) => ({ defId, radiant: false })), true, launch.name, false);
  return { ship, name: launch.name };
}

/** R3334001: the lettering and the letters' colors of a launched ship's design. */
function letteringOf(ship: StarshipArt): Pick<FxCinematicCue, "lettering" | "letters"> {
  // A launched ship never wears the shipyard's design (`starshipDesignOf`); the fallback only types it.
  const design = ship.design === "shipyard" ? LETTERING_FALLBACK : ship.design;
  const livery = ship.livery ?? STARSHIP_DESIGNS[design].livery;
  return { lettering: STARSHIP_LETTERING[design], letters: { fill: livery.window, edge: livery.hull } };
}

const share = (p: StarshipPoint): { x: number; y: number } => ({ x: p.x / PICTURE_UNITS, y: p.y / PICTURE_UNITS });

/**
 * R3334001: where the camera may look on the picture, `zoom` close: no farther from its middle than keeps
 * the zoomed picture filling the screen (fx.css draws it at least as wide and as tall as the screen, so a
 * share of 1/zoom of it is in frame and its middle may move half the rest either way).
 */
export function cameraPoint(p: { x: number; y: number }, zoom: number): { x: number; y: number } {
  const reach = PICTURE_MIDDLE * (1 - 1 / Math.max(1, zoom));
  const keep = (v: number): number => Math.min(PICTURE_MIDDLE + reach, Math.max(PICTURE_MIDDLE - reach, v));
  return { x: keep(p.x), y: keep(p.y) };
}

/**
 * R3334001: the launch's cinematic over the hold `H` (or, `still`, its title card): the camera's pan from
 * stern to bow, a stop at each module timed by where it stands, the engines lit in turn, the name's stamp
 * and the climb.
 */
function cinematicCue(launch: StarshipLaunch, entry: AnimationEntry, view: PlayerView, H: number, durationMs: number, c: Ctx | null): FxCinematicCue {
  const { ship, name } = launchArt(launch, entry, view);
  const size = starshipSizeOf(launch.pieces);
  const tune = STARSHIP_TUNING[size];
  const C = STARSHIP_CINEMATIC;
  const stern = share(starshipEnginePoints(ship).reduce((a, b) => (b.x < a.x ? b : a), starshipProwPoint(ship)));
  const bow = share(starshipProwPoint(ship));
  const span = bow.x - stern.x;
  const along = (x: number): number => (span > 0 ? Math.max(0, Math.min(1, (x - stern.x) / span)) : 0);
  const stops: FxTimedPoint[] = starshipMountPoints(ship).map((m) => {
    const p = share(m);
    return { ...p, atMs: frac(C.panFrom + (C.panTo - C.panFrom) * along(p.x), H) };
  });
  const lit = starshipEnginePoints(ship).slice(0, C.enginesMax);
  const engines: FxTimedPoint[] = lit.map((p, k) => ({ ...share(p), atMs: frac(C.enginesFrom + ((C.enginesTo - C.enginesFrom) * k) / Math.max(1, lit.length - 1), H) }));
  const sparks = c === null ? 0 : Math.max(1, Math.round(tune.sparks * c.share));
  return {
    kind: "cinematic",
    art: starshipArtUri(ship),
    name,
    ...letteringOf(ship),
    size,
    tint: tintOf(rocketLook()),
    at: tid(launch.anchor ?? ""),
    cameraFrom: cameraPoint(stern, tune.zoom),
    cameraTo: cameraPoint(bow, tune.zoom),
    panAtMs: frac(C.panFrom, H),
    panEndMs: frac(C.panTo, H),
    zoom: tune.zoom,
    stops,
    engines,
    sparks,
    stampMs: frac(C.stampAt, H),
    climbMs: frac(C.climbAt, H),
    delayMs: 0,
    durationMs,
    ...(c === null ? { still: true as const } : {}),
  };
}

function planLaunch(launch: StarshipLaunch, entry: AnimationEntry, view: PlayerView, env: Pick<FxPlanEnv, "bigMoments">, c: Ctx): SetPiecePlan | null {
  if (launch.anchor === null) return null;
  const { i, D } = c;
  const look = rocketLook();
  const size = starshipSizeOf(launch.pieces);
  const tune = STARSHIP_TUNING[size];
  // Held, the cinematic fills the hold and the ship lifts off as it ends; unheld (Big moments "off", or a
  // second set piece in the action), it lifts off at once. Only Big moments "full" plays the cinematic:
  // "short" and "off" show the title card through the entry.
  const H = entry.setPiece?.key === STARSHIP_SET_PIECE ? entry.setPiece.holdMs : 0;
  const M = Math.max(0, D - H);
  const at = tid(launch.anchor);
  const foot = tid(launch.anchor, MOTIF_FOOT);
  const igniteMs = frac(FX_STARSHIP_IGNITE_AT, H);
  const landMs = H + frac(FX_STARSHIP_LAND_AT, M);
  const exhaust = (delayMs: number, spread: "point" | "area"): FxCue => ({ ...tunedBurst(i, "exhaust", foot, spread, delayMs, tune.exhaust), colors: look.colors });
  const cinematic = H > 0 && (env.bigMoments ?? "full") === "full";
  const cues: FxCue[] = [launchCue(at, size, igniteMs, D + FX_STARSHIP_TAIL_MS)];
  if (cinematic) {
    const cue = cinematicCue(launch, entry, view, H, H + STARSHIP_CINEMATIC.tailMs, c);
    cues.push(cue);
    // Each engine shakes the board as it lights, the last as hard as the climb.
    cue.engines.forEach((engine, k) => cues.push(...shakeCues(i, (tune.trauma * (k + 1)) / cue.engines.length, engine.atMs)));
    cues.push(...shakeCues(i, tune.trauma, cue.climbMs));
  } else {
    cues.push(cinematicCue(launch, entry, view, H, D + FX_STARSHIP_TAIL_MS, null));
  }
  cues.push(
    exhaust(igniteMs, "point"),
    // Lift-off: the engines' full burn, the contrail it climbs on, and the Dormant badge burning away.
    exhaust(H, "area"),
    {
      kind: "projectile",
      preset: "exhaust",
      from: foot,
      to: tid(launch.anchor, { x: MOTIF_FOOT.x, y: -tune.contrail }),
      delayMs: H,
      flightMs: frac(FX_STARSHIP_LIFT_AT, M),
      density: tune.density * i * c.share,
    },
    { ...tunedBurst(i, "ember", tid(launch.anchor, FX_STARSHIP_BADGE_AT), "point", H, MOTIF_TUNING.ember), colors: look.colors },
    ...shakeCues(i, tune.trauma, H),
    // The landing, as its launched self.
    ringCue(D, "dust", at, landMs),
    tunedBurst(i, "dust", foot, "area", landMs, MOTIF_TUNING.land),
    ...shakeCues(i, tune.trauma, landMs),
  );
  const replaces = new Set<GameEvent>(entry.events.filter((event) => event.type === "starshipLaunched"));
  return { cues, replaces };
}

/** R3334002: one gun's shots, each landing on `to` as the attack's row ends (R200). */
function gunCues(gun: BroadsideGun, card: string, to: FxAnchor, fireMs: number, c: Ctx): FxCue[] {
  const { i, D } = c;
  const B = STARSHIP_BROADSIDE;
  const from = picture(card, gun);
  const colors = [MODULE_KINDS[gun.kind].energy, ...rocketLook().colors];
  const density = i * c.share;
  const shot = (motion: FxShot, delayMs: number, target: FxAnchor = to): FxCue => ({
    kind: "shot",
    shot: motion,
    colors,
    preset: gun.weapon.preset,
    from,
    to: target,
    delayMs,
    flightMs: Math.max(0, D - delayMs),
    density,
  });
  // A sweep's strokes and flak's clouds spread evenly across the target, at its middle's height.
  const aim = (k: number, n: number): FxAnchor => (to.kind === "testid" ? { ...to, at: { x: (k + 1) / (n + 1), y: B.aimY } } : to);
  switch (gun.weapon.fire) {
    case "beam":
      // A beam sweeps across its target, stroke after stroke.
      return Array.from({ length: B.beamSweep }, (_, k) => shot({ motion: "beam" }, fireMs + frac(B.beamStep * k, D), aim(k, B.beamSweep)));
    case "repeater":
      return Array.from({ length: B.repeaterShots }, (_, k) => shot({ motion: "bolt", trail: "spark" }, fireMs + frac(B.repeaterStep * k, D)));
    case "cannon":
      // A cannon kicks back where it stands as it fires its shell.
      return [
        {
          kind: "module",
          uri: starshipModuleUri(gun.defId, gun.radiant),
          motion: "recoil",
          share: STARSHIP_DOCK.moduleShare,
          from,
          to: from,
          landMs: Math.min(B.recoilMs, D - fireMs + FX_MAX_TAIL_MS),
          spin: 0,
          delayMs: fireMs,
          durationMs: Math.min(B.recoilMs, D - fireMs + FX_MAX_TAIL_MS),
        },
        shot({ motion: "bolt", trail: "smoke" }, fireMs),
        burst(c, "smoke", from, "point", fireMs, STARSHIP_MOMENT_TUNING.muzzle, colors),
        ...shakeCues(i, B.cannonTrauma, fireMs),
      ];
    case "flak":
      return [
        shot({ motion: "bolt", trail: "smoke" }, fireMs),
        ...Array.from({ length: B.flakClouds }, (_, k) => burst(c, "smoke", aim(k, B.flakClouds), "area", D, STARSHIP_MOMENT_TUNING.flak, colors)),
      ];
    case "launcher":
      return [shot({ motion: "arc", sprites: [gun.weapon.sprite ?? "arrow"], turn: "spin", count: 1 }, fireMs)];
  }
}

/**
 * R3334002: a launched Starship's attack: each mounted weapon fires by its kind, one after another, every
 * shot landing as the row ends; a ship with no weapons rams prow first, its exhaust at the start and its
 * prow sparking at the contact. The lunge and its turn are the stage's (stage.ts).
 */
function planBroadside(event: Extract<GameEvent, { type: "attackDeclared" }>, entry: AnimationEntry, view: PlayerView, c: Ctx): SetPiecePlan | null {
  const found = starshipUnitOf(view, event.attackerId);
  if (found === null || found.unit.starship.launched === undefined) return null;
  const card = testid.card(event.attackerId);
  const hero = /^hero-(p1|p2)$/.exec(event.targetId)?.[1];
  const targetId = hero === "p1" || hero === "p2" ? testid.hero(sideOf(view, hero)) : testid.card(event.targetId);
  const to = tid(targetId);
  const ship = shipArtOf(found.unit, true, found.unit.starship.name);
  const guns = broadsideGuns(ship);
  const cues: FxCue[] = [];
  if (guns.length === 0) {
    const contact = frac(FX_LUNGE_CONTACT_AT, c.D);
    const look = rocketLook();
    for (const engine of starshipEnginePoints(ship)) cues.push(burst(c, "exhaust", picture(card, engine), "point", 0, STARSHIP_MOMENT_TUNING.ramExhaust, look.colors));
    cues.push(
      burst(c, "spark", picture(card, starshipProwPoint(ship)), "point", contact, STARSHIP_MOMENT_TUNING.ramSpark, look.colors),
      ringCue(c.D, "shard", to, contact),
      ...shakeCues(c.i, STARSHIP_BROADSIDE.ramTrauma, contact),
    );
  } else {
    guns.forEach((gun, j) => cues.push(...gunCues(gun, card, to, broadsideFireMs(j, guns.length, c.D), c)));
  }
  return { cues, replaces: new Set<GameEvent>([event]) };
}

/**
 * R3334003: a Starship's wreck: its newest modules blow off one by one, the hull breaks in two as the
 * card is hidden, debris drifts over its side's backrow and a scorch settles on its zone. Never inside a
 * wipe, whose set piece swallows it.
 */
function planWreck(event: Extract<GameEvent, { type: "destroyed" }>, entry: AnimationEntry, view: PlayerView, c: Ctx): SetPiecePlan | null {
  if (entry.wipe !== undefined) return null;
  const found = starshipUnitOf(view, event.instanceId);
  if (found === null) return null;
  const { i, D } = c;
  const W = STARSHIP_WRECK;
  const card = testid.card(event.instanceId);
  const ship = shipArtOf(found.unit, found.unit.starship.launched !== undefined, found.unit.starship.name);
  const center = starshipCenterPoint(ship);
  const off = starshipMountPoints(ship).slice(-W.modulesMax).reverse();
  const cues: FxCue[] = [];
  off.forEach((m, k) => {
    const delayMs = frac(W.blowFrom + ((W.blowTo - W.blowFrom) * k) / Math.max(1, off.length - 1), D);
    const dx = m.x - center.x;
    const dy = m.y - center.y;
    const length = Math.hypot(dx, dy);
    const away = length > 0 ? { x: dx / length, y: dy / length } : { x: 0, y: -1 };
    const reach = W.fling * PICTURE_UNITS;
    cues.push(
      {
        kind: "module",
        uri: starshipModuleUri(m.defId, m.radiant),
        motion: "blowOff",
        share: STARSHIP_DOCK.moduleShare,
        from: picture(card, m),
        to: picture(card, { x: m.x + away.x * reach, y: m.y + away.y * reach }),
        landMs: Math.min(W.flingMs, D - delayMs + FX_MAX_TAIL_MS),
        spin: k % 2 === 0 ? 1 : -1,
        delayMs,
        durationMs: Math.min(W.flingMs, D - delayMs + FX_MAX_TAIL_MS),
      },
      burst(c, "spark", picture(card, m), "point", delayMs, STARSHIP_MOMENT_TUNING.blowOff),
    );
  });
  const breakMs = frac(W.breakAt, D);
  const debrisMs = frac(W.debrisAt, D);
  const scorchMs = frac(W.scorchAt, D);
  const zone = testid.zone(found.side, found.row, found.lane);
  cues.push(
    { kind: "wreck", art: starshipArtUri(ship), at: { kind: "testid", testid: card, art: "frame" }, breakMs, turn: W.halfTurnDeg, delayMs: 0, durationMs: D + W.tailMs },
    burst(c, "smoke", tid(card), "area", breakMs, STARSHIP_MOMENT_TUNING.wreckSmoke),
    ...shakeCues(i, W.trauma, breakMs),
    { kind: "conceal", testid: card, mode: "after", delayMs: breakMs, durationMs: D - breakMs + W.tailMs },
    burst(c, "dust", tid(testid.zone(found.side, "backrow", found.lane)), "area", debrisMs, STARSHIP_MOMENT_TUNING.debris),
    { kind: "scorch", at: tid(zone), delayMs: scorchMs, durationMs: D - scorchMs + W.scorchTailMs },
  );
  return { cues, replaces: new Set<GameEvent>([event]) };
}

/** R3334001: a War Doctrine cast, announced over its caster's launched Starship (else the board's middle) in its own look. */
function planDoctrine(event: Extract<GameEvent, { type: "cardPlayed" }>, view: PlayerView, c: Ctx): SetPiecePlan | null {
  const doctrine = WAR_DOCTRINES[event.defId];
  if (doctrine === undefined) return null;
  const { i, D } = c;
  const look = WAR_DOCTRINE_LOOKS[doctrine];
  const ship = launchedStarshipOf(view, sideOf(view, event.player));
  const at: FxAnchor = ship === null ? { kind: "viewport", at: { x: FX_CENTER.x, y: FX_CENTER.y } } : tid(ship);
  const stampMs = frac(STARSHIP_DOCTRINE.stampAt, D);
  const colors = [look.tint.core, look.tint.glow, look.tint.rim];
  return {
    cues: [
      { kind: "doctrine", doctrine, text: CATALOG[event.defId]?.name ?? "", tint: look.tint, at, delayMs: 0, durationMs: D + STARSHIP_DOCTRINE.tailMs },
      burst(c, look.preset, at, "area", stampMs, STARSHIP_MOMENT_TUNING.doctrine, colors),
      ringCue(D, look.preset, at, stampMs),
      ...shakeCues(i, STARSHIP_DOCTRINE.trauma, stampMs),
    ],
    replaces: new Set<GameEvent>(),
  };
}

type StarshipEnv = Pick<FxPlanEnv, "intensity"> & Partial<Pick<FxPlanEnv, "bigMoments" | "compact">>;

/**
 * R2801000, R3334000–R3334003: the Starship's cues for a fuse run, a launch, a launched Starship's
 * attack, a Starship's death or a War Doctrine's cast, and the events they draw in place of their own
 * rows; null for any other entry, the effects off, an entry of no duration (Reduce Motion) or a Starship
 * the seat cannot place.
 */
export function planStarship(entry: AnimationEntry, view: PlayerView, env: StarshipEnv): SetPiecePlan | null {
  const i = env.intensity;
  const D = entry.durationMs;
  if (!(i > 0) || !(D > 0)) return null;
  const c: Ctx = { i, D, share: env.compact === true ? STARSHIP_CINEMATIC.phoneShare : 1 };
  if (entry.fuse !== undefined) return planFuse(entry.fuse, entry, view, c);
  if (entry.launch !== undefined) return planLaunch(entry.launch, entry, view, env, c);
  const first = entry.events[0];
  if (first?.type === "attackDeclared") return planBroadside(first, entry, view, c);
  const plans = entry.events.flatMap((event) => {
    const plan = event.type === "destroyed" ? planWreck(event, entry, view, c) : event.type === "cardPlayed" ? planDoctrine(event, view, c) : null;
    return plan === null ? [] : [plan];
  });
  if (plans.length === 0) return null;
  return { cues: plans.flatMap((plan) => plan.cues), replaces: new Set<GameEvent>(plans.flatMap((plan) => [...plan.replaces])) };
}

/**
 * R2801000, R3334001: a launch under Reduce Motion (an entry of no duration): the title card, the ship's
 * picture and its name, nothing moving. It holds nothing: the queue has already moved on, as a Mythic's
 * still card does (R2722001, R2785000).
 */
export function planStarshipStill(entry: AnimationEntry, view: PlayerView, env: Pick<FxPlanEnv, "intensity">): FxCue[] {
  const launch = entry.launch;
  if (launch === undefined || launch.anchor === null || !(env.intensity > 0)) return [];
  return [cinematicCue(launch, entry, view, 0, FX_STARSHIP_STILL_MS, null)];
}
