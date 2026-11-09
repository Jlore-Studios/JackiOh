// R3399000: Classic+ Bot Loser (`classicplus-019-5`, entrance "flexBurst", bespoke "killNotch"), his signature
// (#3399): Entrance: he bursts into the zone flexing (steel shards and a ring at the slam, the one shake),
// bow raised (a rising arrow) and one boot planted on the zone's edge (dust at its foot). Text: each kill
// cuts a notch in his bow (a shard point at the bow) and a red gleam into his arms (a red spark burst,
// `buffed`, a rising arrow); while Berserk his eyes flare red (a red burst over him) as he turns on his own
// hero (`damage` from him, claimed on his turn hooks). R202: zones, heroes and public board targets anchor.

import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";

import { targetFor } from "../../../game/animations.ts";
import { frac, ringCue, shakeCues, tid, tunedBurst } from "../../build.ts";
import { FX_SLAM_AT } from "../../constants.ts";
import { FLAVOR_LOOKS, paletteLook } from "../../flavors.ts";
import { hitAnchor } from "../../motifs/build.ts";
import type { FxCue, FxPoint } from "../../types.ts";
import { ENTRANCE_TRAUMA, type CardRecipe, type Entrance } from "../recipe.ts";

/** Every particle count the burst and the notches throw, at intensity "normal" (rule 9). */
const TUNING = {
  burst: { count: 16, power: 1.2 },
  boot: { count: 8, power: 0.6 },
  notch: { count: 8, power: 0.6 },
  gleam: { count: 14, power: 0.8 },
  eyes: { count: 10, power: 0.7 },
} as const;

const LOOK = paletteLook("Physical/blade") ?? FLAVOR_LOOKS.Physical;
const RED_COLORS = ["#ffd6d6", "#e0202e", "#6b0a12"] as const;

const BOW_AT = 0.7;
const BOW_MS = 500;
const ARROWS_MS = 500;
const NOTCH_AT = 0.05;
const GLEAM_AT = 0.2;
const EYES_AT = 0.05;
/** The zone's edge where his boot lands, and the bow's and the eyes' places on him. */
const EDGE: FxPoint = { x: 0.9, y: 1 };
const BOW: FxPoint = { x: 0.25, y: 0.4 };
const EYES: FxPoint = { x: 0.5, y: 0.2 };

export const flexBurst: Entrance = ({ D, tgt, intensity: i }) => {
  const zone = tid(tgt);
  const slam = frac(FX_SLAM_AT, D);
  return [
    { ...tunedBurst(i, LOOK.particle, zone, "area", slam, TUNING.burst), colors: LOOK.colors },
    ringCue(D, LOOK.particle, zone, slam),
    tunedBurst(i, "dust", tid(tgt, EDGE), "point", slam, TUNING.boot),
    { kind: "arrows", direction: "up", at: zone, delayMs: frac(BOW_AT, D), durationMs: BOW_MS },
    ...shakeCues(i, ENTRANCE_TRAUMA.flexBurst, slam),
  ];
};

/** His own hooks: a kill's `buffed` notch and gleam, and a Berserk turn's `damage` from him with red eyes. */
export const killNotch: CardRecipe = (event, p, play) => {
  if (play.hook !== "trigger") return null;
  const i = p.env.intensity;
  if (event.type === "buffed") {
    const tgt = targetFor(event, p.view);
    if (tgt === null) return null;
    const cues: FxCue[] = [
      { ...tunedBurst(i, "shard", tid(tgt, BOW), "point", frac(NOTCH_AT, p.D), TUNING.notch), colors: LOOK.colors },
      { ...tunedBurst(i, "spark", tid(tgt), "area", frac(GLEAM_AT, p.D), TUNING.gleam), colors: RED_COLORS },
      { kind: "arrows", direction: "up", at: tid(tgt), delayMs: frac(GLEAM_AT, p.D), durationMs: ARROWS_MS },
    ];
    return { cues, row: "keep" };
  }
  if (event.type === "damage" && event.sourceId === play.instanceId) {
    const source = hitAnchor(p.view, event.sourceId);
    if (source === null || source.kind !== "testid") return null;
    return { cues: [{ ...tunedBurst(i, "blood", tid(source.testid, EYES), "point", frac(EYES_AT, p.D), TUNING.eyes), colors: RED_COLORS }], row: "keep" };
  }
  return null;
};

/** The gallery's whenever: a kill buffs the caster's first Unit. */
const FIXTURE_ATTACK = 2;

export function killNotchFixture(view: PlayerView, instanceId: string, player: PlayerId): GameEvent[] {
  const own = player === view.viewer ? view.you : view.opponent;
  const unit = own.units.find((candidate) => candidate !== null);
  if (unit === undefined || unit === null) return [];
  return [
    { type: "hookResolving", sourceId: instanceId, defId: "classicplus-019-5", controller: player, hook: "whenever" },
    { type: "buffed", instanceId: unit.instanceId, attack: FIXTURE_ATTACK, health: 0 },
  ];
}
