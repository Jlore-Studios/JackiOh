// R3402001: #98 Heroic Power (`core-098`), a Mythic Field Spell's signature (R3332000): takeover "rotunda"
// (a white marble rotunda of thirteen glowing doors, takeoverArt.ts), entrance "heroicPedestal", recipe
// "heroicDoors" and a `pedestal` aura on its zone (R3402000). As the hall folds down it lands as a crystal
// pedestal in its zone: holy rays, a ring and a burst of light, and a diamond glint for its Indestructible.
// Its Discover's doors open in the prompt, for the caster alone. Each Activate is a beat of its own (a later
// action than its play): on its `activated`, which the engine sends before the Activate's hook opens, the
// pedestal flares, and inside the hook the power's own events launch it from the pedestal, each kind at
// most FX_SIGNATURE_BURSTS_MAX times an entry: a fire dart to a `damage`'s target, a dust sprinter to a
// `summoned` Unit's zone and a card back into the hand pile on a `drawn` or an `addedToHand`. It anchors on
// its zone, the piles and readable targets and names no card (R202).

import { animTestid, locateInstance, targetFor } from "../../../game/animations.ts";
import { sideOf } from "../../../game/contract.ts";
import { frac, raysCue, ringCue, shakeCues, tid, tunedBurst } from "../../build.ts";
import { FX_SLAM_AT } from "../../constants.ts";
import { FLAVOR_LOOKS } from "../../flavors.ts";
import { hitAnchor, timesOf } from "../../motifs/build.ts";
import type { FxAnchor, FxCue, FxPoint, FxShotCue } from "../../types.ts";
import { ENTRANCE_TRAUMA, heroOf, type CardRecipe, type Entrance } from "../recipe.ts";
import { withinBursts } from "./flourish.ts";

/** Every particle count the pedestal throws, at intensity "normal" (rule 9). */
const TUNING = {
  light: { count: 30, power: 1.2 },
  diamond: { count: 8, power: 0.7 },
  flare: { count: 18, power: 1 },
} as const;

/** Where the Indestructible diamond glints: the pedestal's top. */
const DIAMOND_AT: FxPoint = { x: 0.5, y: 0.2 };

const HOLY = FLAVOR_LOOKS.Holy;

export const heroicPedestal: Entrance = ({ D, tgt, intensity: i, leadMs }) => {
  const at = tid(tgt);
  // R3405000: a Mythic's entrance starts after its entry's hold, so its rotunda takeover plays first and the
  // pedestal lands as it ends.
  const lead = leadMs ?? 0;
  const slam = lead + frac(FX_SLAM_AT, D - lead);
  return [
    raysCue(D, "holy", at, lead),
    ringCue(D, "holy", at, slam),
    { ...tunedBurst(i, "holy", at, "area", slam, TUNING.light), colors: HOLY.colors },
    tunedBurst(i, "sparkle", tid(tgt, DIAMOND_AT), "point", slam, TUNING.diamond),
    ...shakeCues(i, ENTRANCE_TRAUMA.heroicPedestal, slam),
  ];
};

/** A power's flight from the pedestal, in `preset`'s trail. */
function launch(from: FxAnchor, to: FxAnchor, preset: "fire" | "dust", delayMs: number, flightMs: number, density: number): FxShotCue {
  return { kind: "shot", shot: { motion: "bolt", trail: preset }, colors: HOLY.colors, preset, from, to, delayMs, flightMs, density };
}

export const heroicDoors: CardRecipe = (event, p, play) => {
  const i = p.env.intensity;
  const t = timesOf(p.D);
  const own = locateInstance(p.view, play.instanceId);
  const pedestal = own === null ? heroOf(p.view, play) : tid(own);
  // The engine sends `activated` before the Activate's `hookResolving` marker opens its hook, so the flare
  // keys on the card's own `activated` whichever entry is open: a game's reaches it as the card's signature
  // claim (cardFx.ts `signatureFallback`, `sourceIdOf`), the gallery's inside its play. It replaces the
  // shared arcane flare an Activate draws on its card (fx/cues.ts).
  if (event.type === "activated" && event.instanceId === play.instanceId) {
    return { cues: [ringCue(p.D, "holy", pedestal, t.G), { ...tunedBurst(i, "holy", pedestal, "ring", t.T, TUNING.flare), colors: HOLY.colors }], row: "replace" };
  }
  if (play.hook !== "activate" || !withinBursts(event, p)) return null;
  const cues = ((): FxCue[] => {
    switch (event.type) {
      case "damage": {
        const to = hitAnchor(p.view, event.targetId);
        return to === null ? [] : [launch(pedestal, to, "fire", t.G, t.L - t.G, i)];
      }
      case "summoned": {
        const zone = targetFor(event, p.view);
        return zone === null ? [] : [launch(pedestal, tid(zone), "dust", t.G, t.L - t.G, i)];
      }
      case "drawn":
      case "addedToHand":
        return [{ kind: "ghost", from: pedestal, to: tid(animTestid.hand(sideOf(p.view, event.player))), delayMs: t.G, durationMs: p.D }];
      default:
        return [];
    }
  })();
  return cues.length === 0 ? null : { cues, row: "keep" };
};
