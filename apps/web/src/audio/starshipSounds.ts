// R3334000–R3334003 (issue #3334): each of the Starship's moments sounds its own, timed to what the
// effects layer draws (fx/starship.ts) and sent as the director starts its entry (director.ts), as a set
// piece's bed is: a clank and weld as each piece of a fuse run docks, its hum a step higher for each piece
// the ship then holds; a launched Starship's broadside, one sound for each kind of weapon it fires, as the
// first gun of that kind fires, or, unarmed, its ram at the contact; and a destroyed Starship's groan as
// its hull breaks, never inside a wipe, whose set piece swallows it. The launch's are its bed's
// (setPieceBeds.ts). It reads only the view the board shows through the entry, so a Starship R97 hides
// (the sentinel, a fuse run with no anchor) sounds nothing (R202), and an entry of no duration (Reduce
// Motion) sounds nothing either. cues.ts drops the rows' sounds these stand in for.

import type { GameEvent, PlayerView } from "@jackioh/shared";

import { FX_LUNGE_CONTACT_AT } from "../fx/constants.ts";
import {
  STARSHIP_WRECK,
  broadsideFireMs,
  starshipBroadside,
  starshipFuseLandMs,
  starshipUnitOf,
  type BroadsideFire,
} from "../fx/starship.ts";
import type { AnimationEntry } from "../game/animations.ts";
import type { StarshipFuse } from "../game/runs.ts";
import type { SfxId, SoundCue } from "./types.ts";

/** R3334002: the sound of each way a mounted weapon fires. */
export const BROADSIDE_SOUNDS: { readonly [F in BroadsideFire]: SfxId } = {
  beam: "starshipBeam",
  repeater: "starshipRepeater",
  cannon: "starshipCannon",
  flak: "starshipFlak",
  launcher: "starshipLauncher",
};

/** R3334000: a clank and weld as each piece docks, `amount` the pieces the ship holds once it has. */
function dockCues(fuse: StarshipFuse, D: number): SoundCue[] {
  if (fuse.anchor === null) return [];
  const n = fuse.pieces.length;
  return fuse.pieces.map((_, k) => ({ kind: "sfx", id: "starshipDock", params: { amount: fuse.count - n + k + 1 }, delayMs: starshipFuseLandMs(k, n, D) }));
}

/** R3334002: each kind of weapon a launched Starship fires, as its first gun of that kind fires; unarmed, its ram at the contact. */
function broadsideCues(event: Extract<GameEvent, { type: "attackDeclared" }>, view: PlayerView, D: number): SoundCue[] {
  const guns = starshipBroadside(view, event.attackerId);
  if (guns === null) return [];
  if (guns.length === 0) return [{ kind: "sfx", id: "starshipRam", delayMs: Math.round(FX_LUNGE_CONTACT_AT * D) }];
  const heard = new Set<BroadsideFire>();
  const cues: SoundCue[] = [];
  guns.forEach((gun, j) => {
    const fire = gun.weapon.fire;
    if (heard.has(fire)) return;
    heard.add(fire);
    cues.push({ kind: "sfx", id: BROADSIDE_SOUNDS[fire], delayMs: broadsideFireMs(j, guns.length, D) });
  });
  return cues;
}

/** R3334003: the groan as a destroyed Starship's hull breaks, once for the entry; none inside a wipe. */
function wreckCues(entry: AnimationEntry, view: PlayerView, D: number): SoundCue[] {
  if (entry.wipe !== undefined) return [];
  const wrecked = entry.events.some((event) => event.type === "destroyed" && starshipUnitOf(view, event.instanceId) !== null);
  return wrecked ? [{ kind: "sfx", id: "starshipWreck", delayMs: Math.round(STARSHIP_WRECK.breakAt * D) }] : [];
}

/**
 * R3334000–R3334003: the Starship's own sounds for `entry`, as `view` (the board through it) shows the
 * ship, each at its beat of the entry; none for any other entry, a launch (its bed's), a Starship the
 * seat cannot place or an entry of no duration.
 */
export function starshipBed(entry: AnimationEntry, view: PlayerView): SoundCue[] {
  const D = entry.durationMs;
  if (!(D > 0)) return [];
  if (entry.fuse !== undefined) return dockCues(entry.fuse, D);
  if (entry.launch !== undefined) return [];
  const first = entry.events[0];
  if (first?.type === "attackDeclared") return broadsideCues(first, view, D);
  return wreckCues(entry, view, D);
}
