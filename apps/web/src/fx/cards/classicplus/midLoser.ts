// R3399000: Classic+ Mid Loser (`classicplus-019-3`, entrance "spotlightPose", bespoke "coinCall"), its signature (#3399):
// Entrance: he strides to the centre of his zone (a puff of dust at his foot), flicks a coin off his thumb
// (it spins up and lands, `coinSpin`) and poses as a spotlight falls on him at the slam (rays, a gold burst
// and a ring). Text: on his Cry the coin spins up over him and lands (`rolled`); heads, he swells in a bright
// fed glow (`buffed`, a rising arrow); tails, he droops grey (a falling arrow) and a mana gem drifts across
// to the opponent's hero (`modifierChanged`, a gold projectile). The coin's panel (the roll row) still plays.
// `coinSpin` is shared with League of Losers. R202: only zones, heroes and the board's public targets anchor.

import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";

import { targetFor } from "../../../game/animations.ts";
import { sideOf, testid } from "../../../game/contract.ts";
import { frac, raysCue, ringCue, shakeCues, tid, tunedBurst } from "../../build.ts";
import { FX_SLAM_AT } from "../../constants.ts";
import { FLAVOR_LOOKS, paletteLook } from "../../flavors.ts";
import { hitAnchor } from "../../motifs/build.ts";
import type { FxAnchor, FxCue, FxPoint, FxShotCue } from "../../types.ts";
import { ENTRANCE_TRAUMA, FOOT, heroOf, projectile, type CardRecipe, type Entrance } from "../recipe.ts";

/** Every particle count the pose and the coin throw, at intensity "normal" (rule 9). */
const TUNING = {
  stride: { count: 6, power: 0.6 },
  pose: { count: 14, power: 1 },
  land: { count: 6, power: 0.6 },
  fed: { count: 16, power: 0.9 },
  droop: { count: 12, power: 0.5 },
  gem: { count: 8, power: 0.5 },
} as const;

const LOOK = paletteLook("Hustle") ?? FLAVOR_LOOKS.Hustle;
/** The grey a drooping Loser wears. */
const GREY_COLORS = ["#e6e6e6", "#9a9a9a", "#5a5a5a"] as const;

/** The entrance: the stride, the flick, then the spotlight at the slam. */
const STRIDE_AT = 0.1;
const COIN_AT = 0.2;
/** The Cry's coin leaves at the start of its entry; the glow comes with the `buffed` entry's own start. */
const CRY_COIN_AT = 0;
const BUFF_AT = 0.1;
const ARROWS_MS = 500;
const GEM_FLIGHT = 0.4;

/** A coin's two legs: its toss and its fall, each this fraction of the entry, at most COIN_LEG_MAX_MS (a Spell's 300 ms burst start). */
const COIN_LEG = 0.15;
const COIN_LEG_MAX_MS = 140;
/** Where the coin leaves from, tops out and lands, in fractions of its zone. */
const COIN_FROM: FxPoint = { x: 0.5, y: 0.6 };
const COIN_TOP: FxPoint = { x: 0.5, y: -0.4 };
const COIN_BACK: FxPoint = { x: 0.55, y: 0.7 };

/** The unit zone of Mid Loser's lane, where League of Losers puts him. */
const MID_LANE = 3;

function at(anchor: FxAnchor, point: FxPoint): FxAnchor {
  return anchor.kind === "testid" ? tid(anchor.testid, point) : anchor;
}

function coinLeg(from: FxAnchor, to: FxAnchor, delayMs: number, flightMs: number, intensity: number): FxShotCue {
  return {
    kind: "shot",
    shot: { motion: "arc", sprites: ["coin"], turn: "spin", count: 1 },
    colors: LOOK.colors,
    preset: "gold",
    from,
    to,
    delayMs,
    flightMs,
    density: intensity,
  };
}

/**
 * A coin spinning up over `zone` and landing in it: two arcs of a spinning coin (the toss, the fall), a
 * gold burst and a ring where it lands. Every cue starts within 300 ms of `delayMs` clamped into the entry
 * `D`, so a Spell's ceiling holds.
 */
export function coinSpin(D: number, intensity: number, zone: FxAnchor, delayMs: number): FxCue[] {
  const start = Math.min(delayMs, D);
  const leg = Math.min(frac(COIN_LEG, D), COIN_LEG_MAX_MS);
  const land = Math.min(start + 2 * leg, D);
  return [
    coinLeg(at(zone, COIN_FROM), at(zone, COIN_TOP), start, leg, intensity),
    coinLeg(at(zone, COIN_TOP), at(zone, COIN_BACK), start + leg, leg, intensity),
    { ...tunedBurst(intensity, "gold", at(zone, COIN_BACK), "point", land, TUNING.land), colors: LOOK.colors },
    ringCue(D, "gold", at(zone, COIN_BACK), land),
  ];
}

export const spotlightPose: Entrance = ({ D, tgt, intensity: i }) => {
  const zone = tid(tgt);
  const slam = frac(FX_SLAM_AT, D);
  return [
    tunedBurst(i, "dust", tid(tgt, FOOT), "point", frac(STRIDE_AT, D), TUNING.stride),
    ...coinSpin(D, i, zone, frac(COIN_AT, D)),
    raysCue(D, "legendary", zone, slam),
    { ...tunedBurst(i, "star", zone, "area", slam, TUNING.pose), colors: LOOK.colors },
    ringCue(D, "gold", zone, slam),
    ...shakeCues(i, ENTRANCE_TRAUMA.spotlightPose, slam),
  ];
};

/** Where the rolling or buffed Loser stands: the board's public card, else Mid Loser's own lane of the caster's side. */
function loserAnchor(event: GameEvent, view: PlayerView, player: PlayerId, id: string): FxAnchor {
  const tgt = targetFor(event, view);
  return tgt !== null ? tid(tgt) : (hitAnchor(view, id) ?? tid(testid.zone(sideOf(view, player), "units", MID_LANE)));
}

/** Mid Loser's Cry: the coin, then the swell or the droop, and the mana gem a tails gives away. */
export const coinCall: CardRecipe = (event, p, play) => {
  if (play.hook !== undefined) return null;
  const i = p.env.intensity;
  switch (event.type) {
    case "rolled":
      return { cues: coinSpin(p.D, i, loserAnchor(event, p.view, event.player, event.instanceId), frac(CRY_COIN_AT, p.D)), row: "keep" };
    case "buffed": {
      const zone = loserAnchor(event, p.view, play.player, event.instanceId);
      const start = frac(BUFF_AT, p.D);
      if (event.attack < 0 || event.health < 0) {
        const droop: FxCue[] = [
          { kind: "arrows", direction: "down", at: zone, delayMs: start, durationMs: ARROWS_MS },
          { ...tunedBurst(i, "smoke", zone, "area", start, TUNING.droop), colors: GREY_COLORS },
        ];
        return { cues: droop, row: "keep" };
      }
      const fed: FxCue[] = [
        { kind: "arrows", direction: "up", at: zone, delayMs: start, durationMs: ARROWS_MS },
        { ...tunedBurst(i, "sparkle", zone, "area", start, TUNING.fed), colors: LOOK.colors },
      ];
      return { cues: fed, row: "keep" };
    }
    case "modifierChanged": {
      if (event.player === play.player) return null;
      const from = heroOf(p.view, play);
      const to = tid(testid.hero(sideOf(p.view, event.player)));
      const start = frac(BUFF_AT, p.D);
      const flight = frac(GEM_FLIGHT, p.D);
      const gem: FxCue[] = [
        projectile(i, "gold", from, to, start, flight),
        { ...tunedBurst(i, "gold", to, "point", start + flight, TUNING.gem), colors: LOOK.colors },
      ];
      return { cues: gem, row: "keep" };
    }
    default:
      return null;
  }
};

/** The gallery's Cry: a heads. His own lane is the board's first Unit, where the gallery stands him. */
export function coinCallFixture(view: PlayerView, instanceId: string, player: PlayerId): GameEvent[] {
  const own = player === view.viewer ? view.you : view.opponent;
  const unit = own.units.find((candidate) => candidate !== null);
  const id = unit?.instanceId ?? instanceId;
  return [
    { type: "rolled", player, instanceId: id, defId: "classicplus-019-3", kind: "coin", sides: 2, rolls: [{ faces: [1], kept: 0 }] },
    { type: "buffed", instanceId: id, attack: 6, health: 6 },
  ];
}
