// R3336000: the Drone Battleship's sounds, beside the sound table's own (cues.ts `cuesFor` appends them
// after the event's row, unpanned: the ship is the whole board's, not one lane's). A row reads the event
// and the viewer's `PlayerView` alone (CLAUDE.md rule 7): a layer is found by the card the view shows on
// the field, or by the `defId` the event itself carries, and an id the view cannot place, or the R97
// sentinel, sounds nothing, so a hidden card never shapes a sound (R203).

import type { GameEvent, PlayerView, UnitView } from "@jackioh/shared";

import { SHIP_DRONE_IDS, shipLayerOf, type ShipLayer } from "../cards/art/stinkyscape.ts";
import { armorTookHalf } from "../game/damageFeel.ts";
import { HIDDEN_DEF_ID, IMPACT_AMOUNT_CAP } from "./constants.ts";
import type { CueContext } from "./cues.ts";
import type { SfxId, SfxParams, SoundCue } from "./types.ts";

/** The softest `reactorPulse` (types.ts `amount`): a tick on a core that has lost next to nothing. */
export const REACTOR_PULSE_MIN = 1;
/** The hardest: a Meltdown layer's blast, and a tick that takes the last of its health. */
export const REACTOR_PULSE_MAX = IMPACT_AMOUNT_CAP;
/** The slam tier (types.ts `slamTier`) a Corvette's landing thud is weighed at, whatever its stats. */
const CORVETTE_SLAM_TIER = "large";

const NONE: readonly SoundCue[] = [];

function sfx(id: SfxId, params?: SfxParams): readonly SoundCue[] {
  return [params === undefined ? { kind: "sfx", id, delayMs: 0 } : { kind: "sfx", id, params, delayMs: 0 }];
}

/** The Battleship layer a public `defId` names; the sentinel names none (R97). */
function layerOfDef(defId: string): ShipLayer | null {
  return defId === HIDDEN_DEF_ID ? null : shipLayerOf(defId);
}

/** The unit the view shows on the field under `instanceId` and the layer it is, or null when it is no layer (R669's `panOf` scan). */
function layerAt(view: PlayerView, instanceId: string): { unit: UnitView; layer: ShipLayer } | null {
  if (instanceId === HIDDEN_DEF_ID) return null;
  for (const side of [view.you, view.opponent]) {
    const unit = side.units.find((u) => u !== null && u.instanceId === instanceId);
    if (unit === undefined || unit === null) continue;
    const layer = layerOfDef(unit.defId);
    return layer === null ? null : { unit, layer };
  }
  return null;
}

/**
 * How near a Meltdown layer is to its end after a hit of `amount`, 0 through 1: the health it has lost of its
 * maximum, as the view the batch was planned against shows it (the same view `healthBefore` reads).
 */
export function reactorLevel(unit: UnitView, amount: number): number {
  if (unit.maxHealth <= 0) return 1;
  return Math.min(1, Math.max(0, (unit.maxHealth - unit.health + amount) / unit.maxHealth));
}

/** The `reactorPulse` a hit of `amount` on the Meltdown layer gives: `REACTOR_PULSE_MIN` through `REACTOR_PULSE_MAX`, growing with the damage taken. */
export function reactorPulseAmount(unit: UnitView, amount: number): number {
  return Math.round(REACTOR_PULSE_MIN + (REACTOR_PULSE_MAX - REACTOR_PULSE_MIN) * reactorLevel(unit, amount));
}

/**
 * The cues the Battleship's three layers and its drones add to one event, or none for any other card or
 * event. Each row names the layer the event is about and the sound the screen's beat has (fx/battleship.ts).
 */
export function battleshipCues(event: GameEvent, ctx: CueContext): readonly SoundCue[] {
  switch (event.type) {
    case "summoned": {
      if (event.row !== "units" || event.defId === HIDDEN_DEF_ID) return NONE;
      switch (layerOfDef(event.defId)) {
        case "meltdown":
          return sfx("horn");
        case "propagation":
          return sfx("lock");
        case "shield":
          return sfx("armorRing");
        case null:
          break;
      }
      if (event.defId === SHIP_DRONE_IDS.interceptor) return sfx("whoosh");
      if (event.defId === SHIP_DRONE_IDS.corvette) return sfx("summon", { slamTier: CORVETTE_SLAM_TIER });
      return NONE;
    }
    case "redirected":
      return layerAt(ctx.view, event.toId)?.layer === "shield" ? sfx("domeFlare") : NONE;
    case "damage": {
      if (event.amount <= 0 || event.targetId === HIDDEN_DEF_ID) return NONE;
      const hit = layerAt(ctx.view, event.targetId);
      if (hit === null) return NONE;
      // A layer's own damage to itself is the reactor's tick; a hit on the dome clanks, unless Armor took half
      // of it, which cues.ts' damage row already clanks.
      if (hit.layer === "meltdown" && event.sourceId === event.targetId) {
        return sfx("reactorPulse", { amount: reactorPulseAmount(hit.unit, event.amount) });
      }
      return hit.layer === "shield" && !armorTookHalf(event.absorbed, event.amount) ? sfx("armorClank") : NONE;
    }
    case "activated":
      return layerOfDef(event.defId) === "propagation" ? sfx("bayDoor") : NONE;
    case "attackDeclared":
      return layerAt(ctx.view, event.attackerId)?.layer === "meltdown" ? sfx("burn") : NONE;
    case "destroyed": {
      const layer = layerAt(ctx.view, event.instanceId);
      if (layer === null) return NONE;
      if (layer.layer === "meltdown") return sfx("reactorPulse", { amount: REACTOR_PULSE_MAX });
      return layer.unit.buried > 0 ? sfx("deckShear") : NONE;
    }
    default:
      return NONE;
  }
}
