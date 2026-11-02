// R502: #27 Blood Ridden Glowy Jelly Bean (`core-027`, bespoke "bloodDrain") runs a crimson stream from
// the caster's hero into the card made Radiant; on the other seat into a back its play's own count picks (R202).

import type { GameEvent } from "@jackioh/shared";

import { HIDDEN_ID, locateInstance } from "../../../game/animations.ts";
import { sideOf } from "../../../game/contract.ts";
import { frac, shakeCues, tid, tunedBurst } from "../../build.ts";
import { FX_BLOOD_FLIGHT_FRACTION, FX_BLOOD_PICK_BASE, FX_BLOOD_PICK_STRIDE, FX_BLOOD_TRAUMA, FX_RAYS_TAIL_MS } from "../../constants.ts";
import type { FxAnchor, FxPlay } from "../../types.ts";
import { heroOf, projectile, type CardFxPlan, type CardRecipe } from "../recipe.ts";

/** Every particle count the drain throws, at intensity "normal" (rule 9). */
const TUNING = {
  bloodSpurt: { count: 18, power: 1 },
  bloodLanding: { count: 14, power: 0.9 },
  radiantGold: { count: 30, power: 1.1 },
  bloodPool: { count: 26, power: 1 },
  bloodRing: { count: 16, power: 1.2 },
} as const;

/** Where a card made Radiant in the caster's hand is: the viewer's own hand card, else a back (R202). */
export function radiantTarget(event: Extract<GameEvent, { type: "radiantSet" }>, p: CardFxPlan, play: FxPlay): FxAnchor {
  const side = sideOf(p.view, play.player);
  const named = event.instanceId === HIDDEN_ID ? null : locateInstance(p.view, event.instanceId);
  if (named !== null && named.startsWith("hand-card-")) return tid(named);
  return { kind: "handCard", side, pick: FX_BLOOD_PICK_BASE + (play.step - 1) * FX_BLOOD_PICK_STRIDE };
}

/** #27 Blood Ridden Glowy Jelly Bean targets its Radiant hand card, or an opponent's back (R97). */
export const bloodDrain: CardRecipe = (event, p, play) => {
  const i = p.env.intensity;
  if (event.type === "radiantSet") {
    if (event.zone.z !== "hand" || event.zone.player !== play.player) return null;
    const hero = heroOf(p.view, play);
    const target = radiantTarget(event, p, play);
    const flight = frac(FX_BLOOD_FLIGHT_FRACTION, p.D);
    return {
      cues: [
        tunedBurst(i, "blood", hero, "point", 0, TUNING.bloodSpurt),
        projectile(i, "blood", hero, target, 0, flight),
        tunedBurst(i, "blood", target, "point", flight, TUNING.bloodLanding),
        tunedBurst(i, "gold", target, "area", flight, TUNING.radiantGold),
        { kind: "sheen", at: target, delayMs: flight, durationMs: p.D - flight + FX_RAYS_TAIL_MS },
      ],
      row: "replace",
    };
  }
  if (event.type === "healthLost" && event.player === play.player) {
    const hero = heroOf(p.view, play);
    return {
      cues: [
        tunedBurst(i, "blood", hero, "area", 0, TUNING.bloodPool),
        tunedBurst(i, "blood", hero, "ring", 0, TUNING.bloodRing),
        ...shakeCues(i, FX_BLOOD_TRAUMA, 0),
      ],
      row: "keep",
    };
  }
  return null;
};
