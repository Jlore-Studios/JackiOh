// R2801000: the Starship's moment, Stinkyscape's signature system (R1852000, R1852003). Pieces fusing
// into one Dormant Starship play as one fuse run (game/runs.ts `fuseRunAt`): each dead piece's card back
// flies from where it stood (else its graveyard) to the Starship, a stagger apart, and bolts on with a
// spark, a clank ring and a short shake. A launch is the Starship's set piece (`starshipClaim`, R2785000):
// a 3-2-1 countdown over the ship through its hold, the board shaking harder at each count, its engines
// igniting in Tech's rocket exhaust (#2792), then, as the hold ends, the lift-off with a contrail, its
// Dormant badge burning away, and the landing as its launched self. Its exhaust, contrail and shake grow
// with the pieces it holds (`STARSHIP_TUNING`). The Starship and its launch are public (R202): no cue
// names a card's definition, and a Starship the seat cannot place (R97's sentinel) plans nothing.
// Everything starts inside its entry and ends within FX_MAX_TAIL_MS of it (R200); the effects speed
// scales it with the entry (R201). Under Reduce Motion the launch is one still frame, which holds nothing.

import type { GameEvent, PlayerView } from "@jackioh/shared";

import { STARSHIP_FUSE_MS, STARSHIP_FUSE_STAGGERED_MAX, STARSHIP_LAUNCH_HOLD_MS, STARSHIP_SET_PIECE, animTestid, starshipFuseMs, type AnimationEntry, type SetPieceClaim } from "../game/animations.ts";
import { sideOf, testid, type Side } from "../game/contract.ts";
import type { StarshipFuse, StarshipLaunch } from "../game/runs.ts";
import { frac, ringCue, shakeCues, tid, tunedBurst, type BurstTuning } from "./build.ts";
import {
  FX_STARSHIP_BADGE_AT,
  FX_STARSHIP_COUNT_AT,
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
import type { FxCue, FxLaunchCue, FxPlanEnv, StarshipSize } from "./types.ts";

/** One size of Starship: the fewest pieces it takes, its exhaust burst, how many card heights its contrail climbs, the contrail's density and the shake. */
export type StarshipTuning = { pieces: number; exhaust: BurstTuning; contrail: number; density: number; trauma: number };

/** R2801000: a bigger ship, more exhaust, a longer contrail and a heavier shake, at intensity "normal" (rule 9). */
export const STARSHIP_TUNING: { readonly [S in StarshipSize]: StarshipTuning } = {
  small: { pieces: 0, exhaust: { count: 24, power: 0.9 }, contrail: 1.5, density: 0.6, trauma: 0.45 },
  medium: { pieces: 3, exhaust: { count: 44, power: 1.15 }, contrail: 2.5, density: 0.85, trauma: 0.65 },
  large: { pieces: 6, exhaust: { count: 72, power: 1.4 }, contrail: 4, density: 1.1, trauma: 0.9 },
};

/** The sizes, smallest first. */
const SIZES: readonly StarshipSize[] = ["small", "medium", "large"];

/** R2801000: every burst a fuse run throws, at intensity "normal" (rule 9). */
export const STARSHIP_FUSE_TUNING = { spark: { count: 18, power: 1.1 } } as const;

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

/** Tech's rocket look (#2792): the Starship's exhaust, sparks and countdown wear it. */
function rocketLook(): FlavorLook {
  return paletteLook("Tech/rocket") ?? FLAVOR_LOOKS.Tech;
}

function planFuse(fuse: StarshipFuse, entry: AnimationEntry, view: PlayerView, i: number, D: number): SetPiecePlan | null {
  if (fuse.anchor === null || fuse.pieces.length === 0) return null;
  const look = rocketLook();
  const to = tid(fuse.anchor);
  // The budget may have squeezed the run: every beat keeps its share of it.
  const s = D / starshipFuseMs(fuse.pieces.length);
  const graveyard = tid(animTestid.graveyard(sideOf(view, fuse.player)));
  const cues: FxCue[] = [];
  fuse.pieces.forEach((piece, k) => {
    const slot = Math.min(k, STARSHIP_FUSE_STAGGERED_MAX - 1);
    const start = Math.round(slot * FX_STARSHIP_FUSE_STAGGER_MS * s);
    const land = start + Math.round(STARSHIP_FUSE_MS * FX_STARSHIP_FUSE_LAND_AT * s);
    cues.push(
      { kind: "ghost", from: piece.from === null ? graveyard : tid(piece.from), to, delayMs: start, durationMs: land - start },
      { ...tunedBurst(i, "spark", to, "point", land, STARSHIP_FUSE_TUNING.spark), colors: look.colors },
      ringCue(D, "spark", to, land),
      ...shakeCues(i, FX_STARSHIP_FUSE_TRAUMA, land),
    );
  });
  const replaces = new Set<GameEvent>(entry.events.filter((event) => event.type === "fused" || event.type === "starshipFused"));
  return { cues, replaces };
}

/** The launch cue: the countdown, the ignition and the flame and contrail of a ship of `size`. */
function launchCue(at: FxLaunchCue["at"], size: StarshipSize, countsMs: readonly number[], igniteMs: number, durationMs: number): FxLaunchCue {
  return { kind: "launch", at, tint: tintOf(rocketLook()), size, countsMs, igniteMs, delayMs: 0, durationMs };
}

function planLaunch(launch: StarshipLaunch, entry: AnimationEntry, i: number, D: number): SetPiecePlan | null {
  if (launch.anchor === null) return null;
  const look = rocketLook();
  const size = starshipSizeOf(launch.pieces);
  const tune = STARSHIP_TUNING[size];
  // Held, the countdown and the ignition fill the hold and the ship lifts off as it ends; unheld (Big
  // moments "off", or a second set piece in the action), there is no countdown and it lifts off at once.
  const H = entry.setPiece?.key === STARSHIP_SET_PIECE ? entry.setPiece.holdMs : 0;
  const M = Math.max(0, D - H);
  const at = tid(launch.anchor);
  const foot = tid(launch.anchor, MOTIF_FOOT);
  const countsMs = H > 0 ? FX_STARSHIP_COUNT_AT.map((share) => frac(share, H)) : [];
  const igniteMs = frac(FX_STARSHIP_IGNITE_AT, H);
  const landMs = H + frac(FX_STARSHIP_LAND_AT, M);
  const exhaust = (delayMs: number, spread: "point" | "area"): FxCue => ({ ...tunedBurst(i, "exhaust", foot, spread, delayMs, tune.exhaust), colors: look.colors });
  const cues: FxCue[] = [launchCue(at, size, countsMs, igniteMs, D + FX_STARSHIP_TAIL_MS)];
  // Each count shakes the board harder, the last as hard as the lift-off.
  countsMs.forEach((delayMs, k) => cues.push(...shakeCues(i, (tune.trauma * (k + 1)) / countsMs.length, delayMs)));
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
      density: tune.density * i,
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

/**
 * R2801000: the Starship's cues for a fuse run or a launch entry, and the events they draw in place of
 * their own rows; null for any other entry, the effects off, an entry of no duration (Reduce Motion) or
 * a Starship the seat cannot place.
 */
export function planStarship(entry: AnimationEntry, view: PlayerView, env: Pick<FxPlanEnv, "intensity">): SetPiecePlan | null {
  const i = env.intensity;
  const D = entry.durationMs;
  if (!(i > 0) || !(D > 0)) return null;
  if (entry.fuse !== undefined) return planFuse(entry.fuse, entry, view, i, D);
  if (entry.launch !== undefined) return planLaunch(entry.launch, entry, i, D);
  return null;
}

/**
 * R2801000: a launch under Reduce Motion (an entry of no duration): one still frame of the Starship's
 * launch, its flame and contrail standing, no count and nothing moving. It holds nothing: the queue has
 * already moved on, as a Mythic's still card does (R2722001, R2785000).
 */
export function planStarshipStill(entry: AnimationEntry, _view: PlayerView, env: Pick<FxPlanEnv, "intensity">): FxCue[] {
  const launch = entry.launch;
  if (launch === undefined || launch.anchor === null || !(env.intensity > 0)) return [];
  return [{ ...launchCue(tid(launch.anchor), starshipSizeOf(launch.pieces), [], 0, FX_STARSHIP_STILL_MS), still: true }];
}
