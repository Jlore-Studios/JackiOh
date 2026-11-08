// R3399000: Classic+ League of Losers (`classicplus-019`, bespoke "lineup"), its signature (#3399):
// Cast: a referee's whistle (a spark burst and a ring at the caster's hero), and five lane markers light up
// one after another across the caster's unit zones, each in its own colour, like a team lining up (the
// `cardPlayed` claims them). Text: each Loser drops into its zone in turn (`summoned`; moved: each Loser's
// landing is its own signature entrance, which planCardFx plays on its `summoned` before any recipe), then Mid
// Loser's coin spins over him and lands (`rolled`, the shared `coinSpin`), swelling or draining him (`buffed`).
// R202: only zones, heroes and public board targets anchor; every cue starts within 300 ms of its entry (a
// Legendary Spell's ceiling).

import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";

import { targetFor } from "../../../game/animations.ts";
import { LANES, sideOf, testid } from "../../../game/contract.ts";
import { frac, ringCue, tid, tunedBurst } from "../../build.ts";
import type { FxCue } from "../../types.ts";
import { FLAVOR_LOOKS, paletteLook } from "../../flavors.ts";
import { hitAnchor } from "../../motifs/build.ts";
import type { CardRecipe } from "../recipe.ts";
import { coinSpin } from "./midLoser.ts";

/** Every particle count the line-up throws, at intensity "normal" (rule 9). */
const TUNING = {
  whistle: { count: 10, power: 0.9 },
  marker: { count: 6, power: 0.6 },
  swell: { count: 14, power: 0.8 },
} as const;

const LOOK = paletteLook("Physical/sport") ?? FLAVOR_LOOKS.Physical;
/** Each lane's own marker colours, lane 1 to 5 (a team's kits). */
const LANE_COLORS = [
  ["#ffffff", "#e63946", "#8d1b24"],
  ["#ffffff", "#3fbf7f", "#0e6b44"],
  ["#ffffff", "#ffd23f", "#9a7b00"],
  ["#ffffff", "#bfe3ff", "#4a8fd6"],
  ["#ffffff", "#ff7a2e", "#a13d00"],
] as const;

/** The whistle blows first; the markers follow MARKER_STEP of the entry apart, the last by MARKER_LATEST_MS. */
const WHISTLE_AT = 0;
const MARKER_AT = 0.04;
const MARKER_STEP = 0.07;
const MARKER_LATEST_MS = 300;
const COIN_AT = 0;
const SWELL_AT = 0.1;
const ARROWS_MS = 500;
/** The Loser's lane the coin falls over when the board does not show him: Mid Loser's. */
const MID_LANE = 3;

export const lineup: CardRecipe = (event, p, play) => {
  if (play.hook !== undefined) return null;
  const i = p.env.intensity;
  const side = sideOf(p.view, play.player);
  switch (event.type) {
    case "cardPlayed": {
      if (event.instanceId !== play.instanceId) return null;
      const hero = tid(testid.hero(side));
      const cues: FxCue[] = [
        { ...tunedBurst(i, "spark", hero, "point", frac(WHISTLE_AT, p.D), TUNING.whistle), colors: LOOK.colors },
        ringCue(p.D, "spark", hero, frac(WHISTLE_AT, p.D)),
      ];
      LANES.forEach((lane, k) => {
        const zone = tid(testid.zone(side, "units", lane));
        const delay = Math.min(frac(MARKER_AT + k * MARKER_STEP, p.D), MARKER_LATEST_MS);
        cues.push({ ...tunedBurst(i, "sparkle", zone, "ring", delay, TUNING.marker), colors: LANE_COLORS[k] ?? LOOK.colors }, ringCue(p.D, "sparkle", zone, delay));
      });
      return { cues, row: "keep" };
    }
    case "rolled": {
      const tgt = targetFor(event, p.view);
      const zone = tgt !== null ? tid(tgt) : (hitAnchor(p.view, event.instanceId) ?? tid(testid.zone(side, "units", MID_LANE)));
      return { cues: coinSpin(p.D, i, zone, frac(COIN_AT, p.D)), row: "keep" };
    }
    case "buffed": {
      const tgt = targetFor(event, p.view);
      const zone = tgt !== null ? tid(tgt) : (hitAnchor(p.view, event.instanceId) ?? tid(testid.zone(side, "units", MID_LANE)));
      const start = frac(SWELL_AT, p.D);
      const direction = event.attack < 0 || event.health < 0 ? "down" : "up";
      const cues: FxCue[] = [
        { kind: "arrows", direction, at: zone, delayMs: start, durationMs: ARROWS_MS },
        { ...tunedBurst(i, direction === "up" ? "sparkle" : "smoke", zone, "area", start, TUNING.swell), colors: LOOK.colors },
      ];
      return { cues, row: "keep" };
    }
    default:
      return null;
  }
};

/** The five Losers League of Losers summons, then Mid Loser's heads: the engine's order (summon, summon, summon, Cry, summon, summon). */
const LOSERS = ["classicplus-019-1", "classicplus-019-2", "classicplus-019-3", "classicplus-019-4", "classicplus-019-5"] as const;
const MID_AT = 2;
const FIXTURE_BUFF = 6;

export function lineupFixture(_view: PlayerView, instanceId: string, player: PlayerId): GameEvent[] {
  const ids = LOSERS.map((_, k) => `${instanceId}-loser-${String(k + 1)}`);
  const summoned = LOSERS.map((defId, k): GameEvent => ({ type: "summoned", player, instanceId: ids[k] ?? instanceId, defId, row: "units", lane: k + 1 }));
  const mid = ids[MID_AT] ?? instanceId;
  const cry: GameEvent[] = [
    { type: "rolled", player, instanceId: mid, defId: LOSERS[MID_AT], kind: "coin", sides: 2, rolls: [{ faces: [1], kept: 0 }] },
    { type: "buffed", instanceId: mid, attack: FIXTURE_BUFF, health: FIXTURE_BUFF },
  ];
  return [...summoned.slice(0, MID_AT + 1), ...cry, ...summoned.slice(MID_AT + 1)];
}
