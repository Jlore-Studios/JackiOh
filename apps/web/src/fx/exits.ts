// #1446: one death exit per flavor, appended by the `death` recipe (cues.ts) over today's
// crack, embers and smoke for a `destroyed` card the view showed face-up. Every delay is 0 or
// one of the death's beats (FX_DEATH_EMBER_AT, FX_DEATH_SMOKE_AT) and every tail is an existing
// one, so an exit paces nothing (R200). Only a face-up card takes an exit: a face-down card
// (R371) or a hidden or unknown one keeps today's death (R202, CLAUDE.md rule 7).

import type { Flavor, PlayerView } from "@jackioh/shared";

import { frac, raysCue, ringCue, tid, tunedBurst } from "./build.ts";
import {
  FX_DEATH_BILL_COLORS,
  FX_DEATH_BOLT_FLIGHT_FRACTION,
  FX_DEATH_EMBER_AT,
  FX_DEATH_FLAVOR_TUNING,
  FX_DEATH_LIGHTS_OUT_COLORS,
  FX_DEATH_SMOKE_AT,
  FX_FLAVOR_LIVE,
} from "./constants.ts";
import { chaosFlavor, FLAVOR_LOOKS } from "./flavors.ts";
import { flavorLookOf } from "./flavorSource.ts";
import type { FxCue, FxPoint } from "./types.ts";

const FOOT: FxPoint = { x: 0.5, y: 1 };
const TOP: FxPoint = { x: 0.5, y: 0 };
const T = FX_DEATH_FLAVOR_TUNING;

/**
 * What a flavor exit plans against: the entry's duration, the card's testid, the intensity, the
 * embers' and the smoke's delays (`fallAt`, `settleAt`), the card's instance id, and the owner's
 * graveyard testid (`pile`).
 */
export type FlavorExitPlan = { D: number; tgt: string; intensity: number; fallAt: number; settleAt: number; instanceId: string; pile: string };

export type FlavorExit = (p: FlavorExitPlan) => FxCue[];

/** Physical: it slumps, and dust settles. */
const physical: FlavorExit = ({ tgt, intensity: i, fallAt, settleAt }) => [
  tunedBurst(i, "dust", tid(tgt, FOOT), "ring", fallAt, T.physicalSlump),
  tunedBurst(i, "dust", tid(tgt, FOOT), "area", settleAt, T.physicalDust),
];

/** Brute: it crumbles into rubble. */
const brute: FlavorExit = ({ D, tgt, intensity: i, fallAt, settleAt }) => {
  const at = tid(tgt);
  return [
    tunedBurst(i, "shard", at, "area", fallAt, T.bruteRubble),
    tunedBurst(i, "dust", tid(tgt, FOOT), "ring", settleAt, T.bruteDust),
    ringCue(D, "dust", at, settleAt),
  ];
};

/** Beast: it bolts off in a streak. */
const beast: FlavorExit = ({ D, tgt, intensity: i, fallAt, pile }) => {
  const at = tid(tgt);
  return [
    { kind: "projectile", preset: "shard", from: at, to: tid(pile), delayMs: fallAt, flightMs: frac(FX_DEATH_BOLT_FLIGHT_FRACTION, D), density: i },
    tunedBurst(i, "dust", tid(tgt, FOOT), "point", fallAt, T.beastKick),
  ];
};

/** Fire: it burns down to ash. */
const fire: FlavorExit = ({ tgt, intensity: i, settleAt }) => [
  tunedBurst(i, "fire", tid(tgt, FOOT), "area", 0, T.fireFlame),
  tunedBurst(i, "dust", tid(tgt, FOOT), "area", settleAt, T.fireAsh),
];

/** Dark: it sinks into smoke. */
const dark: FlavorExit = ({ tgt, intensity: i, fallAt, settleAt }) => [
  tunedBurst(i, "void", tid(tgt, FOOT), "ring", fallAt, T.darkSink),
  tunedBurst(i, "smoke", tid(tgt, FOOT), "area", settleAt, T.darkSmoke),
];

/** Holy: it rises in light. */
const holy: FlavorExit = ({ D, tgt, intensity: i, fallAt }) => {
  const at = tid(tgt);
  return [raysCue(D, "holy", at, fallAt), tunedBurst(i, "holy", tid(tgt, TOP), "area", fallAt, T.holyLight)];
};

/** Nerd: it closes like a book. */
const nerd: FlavorExit = ({ tgt, intensity: i, fallAt, settleAt }) => {
  const at = tid(tgt);
  return [
    tunedBurst(i, "arcane", at, "ring", fallAt, T.nerdPages),
    tunedBurst(i, "sparkle", at, "point", settleAt, T.nerdSnap),
  ];
};

/** Tech: it powers down with sparks. */
const tech: FlavorExit = ({ tgt, intensity: i, fallAt, settleAt }) => {
  const at = tid(tgt);
  return [
    tunedBurst(i, "spark", at, "area", fallAt, T.techSparks),
    tunedBurst(i, "frost", at, "point", settleAt, T.techFade),
  ];
};

/** Nature: it wilts. */
const nature: FlavorExit = ({ tgt, intensity: i, fallAt, settleAt }) => {
  const at = tid(tgt);
  return [
    tunedBurst(i, "poison", at, "area", fallAt, T.natureWilt),
    tunedBurst(i, "dust", tid(tgt, FOOT), "point", settleAt, T.natureFall),
  ];
};

/** Toxic: it melts into a puddle. */
const toxic: FlavorExit = ({ D, tgt, intensity: i, fallAt, settleAt }) => {
  const at = tid(tgt);
  const foot = tid(tgt, FOOT);
  return [
    tunedBurst(i, "poison", at, "point", fallAt, T.toxicDrip),
    tunedBurst(i, "poison", foot, "ring", settleAt, T.toxicPuddle),
    ringCue(D, "poison", foot, settleAt),
  ];
};

/** Snack: it is eaten, leaving crumbs. */
const snack: FlavorExit = ({ tgt, intensity: i, fallAt, settleAt }) => {
  const at = tid(tgt);
  return [
    tunedBurst(i, "ember", at, "ring", fallAt, T.snackBite),
    tunedBurst(i, "ember", tid(tgt, FOOT), "area", settleAt, T.snackCrumbs),
  ];
};

/** Hustle: it scatters as bills. */
const hustle: FlavorExit = ({ tgt, intensity: i, fallAt, settleAt }) => {
  const at = tid(tgt);
  return [
    { ...tunedBurst(i, "confetti", at, "area", fallAt, T.hustleBills), colors: FX_DEATH_BILL_COLORS },
    tunedBurst(i, "gold", tid(tgt, FOOT), "ring", settleAt, T.hustleCoins),
  ];
};

/** Clout: the lights go out, a "disconnected" flash. */
const clout: FlavorExit = ({ tgt, intensity: i, fallAt, settleAt }) => {
  const at = tid(tgt);
  return [
    tunedBurst(i, "spark", at, "point", fallAt, T.cloutFlash),
    { ...tunedBurst(i, "smoke", at, "area", settleAt, T.cloutDark), colors: FX_DEATH_LIGHTS_OUT_COLORS },
  ];
};

/** Spirit: it floats up. */
const spirit: FlavorExit = ({ D, tgt, intensity: i, fallAt, settleAt }) => {
  const at = tid(tgt);
  return [
    { kind: "sheen", at, delayMs: fallAt, durationMs: D - fallAt },
    tunedBurst(i, "sparkle", tid(tgt, TOP), "area", fallAt, T.spiritRise),
    tunedBurst(i, "sparkle", at, "ring", settleAt, T.spiritWisps),
  ];
};

/** Chaos: another flavor's exit, picked from the instance id as #1442's Chaos picks. */
const chaos: FlavorExit = (p) => FLAVOR_EXITS[chaosFlavor(p.instanceId)](p);

/** One exit per flavor; the `Record<Flavor, …>` type makes a missing one fail `typecheck`. */
export const FLAVOR_EXITS: Readonly<Record<Flavor, FlavorExit>> = {
  Physical: physical,
  Brute: brute,
  Beast: beast,
  Fire: fire,
  Dark: dark,
  Holy: holy,
  Nerd: nerd,
  Tech: tech,
  Nature: nature,
  Toxic: toxic,
  Snack: snack,
  Hustle: hustle,
  Clout: clout,
  Spirit: spirit,
  Chaos: chaos,
};

/** True when `view` draws `instanceId` face-up: a Unit on either board, or a public backrow card that is not its controller's own unrevealed Trap (R371). */
function shownFaceUp(view: PlayerView, instanceId: string): boolean {
  for (const side of [view.you, view.opponent]) {
    if (side.units.some((unit) => unit !== null && unit.instanceId === instanceId)) return true;
    if (side.backrow.some((slot) => slot !== null && slot.faceDown === false && slot.unrevealed !== true && slot.instanceId === instanceId)) return true;
  }
  return false;
}

/**
 * #1446: the flavor exit a `destroyed` card the view showed face-up adds over today's death, or
 * nothing: a face-down card, a hidden or unknown `defId` (`flavorLookOf`, CLAUDE.md rule 7) and
 * reduced motion (every duration is 0) keep today's cue alone.
 */
export function flavorExitCues(opts: { defId: string; instanceId: string; view: PlayerView; D: number; tgt: string; intensity: number; pile: string }): FxCue[] {
  if (!FX_FLAVOR_LIVE || !(opts.D > 0) || !shownFaceUp(opts.view, opts.instanceId)) return [];
  const look = flavorLookOf(opts.defId, opts.instanceId, opts.view);
  if (look === null) return [];
  const flavor = (Object.keys(FLAVOR_LOOKS) as Flavor[]).find((key) => FLAVOR_LOOKS[key] === look);
  if (flavor === undefined) return [];
  const { D, tgt, intensity, instanceId, pile } = opts;
  return FLAVOR_EXITS[flavor]({ D, tgt, intensity, instanceId, pile, fallAt: frac(FX_DEATH_EMBER_AT, D), settleAt: frac(FX_DEATH_SMOKE_AT, D) });
}
