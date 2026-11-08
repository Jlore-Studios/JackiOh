// R3395000: Stinkyscape #85 Prototype-X1 (`stinkyscape-085`, entrance "testLurch", bespoke "wingMount", a
// hologram grid's motes on its tile), a Legendary Unit's signature. Telemetry towers flash around its zone
// as an unpainted test ship lurches in trailing loose cables, in a spray of sparks. Its Cry Equips the Epic
// Weapons it Discovers: each one's hologram outline fills in solid on a wing mount as its `fused` lands (a
// hologram brand and sheen, a ring, the first with a crackle of circuitry). It keys on its play and the fused
// card's public id, never on a Weapon's definition; the Discover's offer stays its caster's (R202). Moved:
// its two tributes' deaths are paid before its play opens; the loose cables and test grid on its tile are
// hologram motes.

import { ringCue, tid, tunedBurst } from "../../build.ts";
import type { FxCue } from "../../types.ts";
import { FOOT, type CardRecipe, type Entrance } from "../recipe.ts";
import { cardAt, entranceFigure, lookOf, placeOf, replace, slamAt, slamShake } from "./signatureKit.ts";

/** Its hologram. */
const HOLOGRAM = "Tech/hologram";

/** A Weapon's outline as it fills in. */
const OUTLINE = { rim: "#7dffe0", core: "#f0fffb", glow: "#b48cff" } as const;

/** Every particle count it throws, at intensity "normal" (rule 9). */
const TUNING = {
  sparks: { count: 22, power: 1 },
  circuit: { count: 14, power: 0.8 },
} as const;

export const testLurch: Entrance = (plan) => {
  const at = tid(plan.tgt);
  const slam = slamAt(plan);
  return [
    entranceFigure("telemetry", HOLOGRAM, plan),
    { ...tunedBurst(plan.intensity, "spark", tid(plan.tgt, FOOT), "area", slam, TUNING.sparks), colors: lookOf(HOLOGRAM).colors },
    ringCue(plan.D, "circuit", at, slam),
    ...slamShake(plan, "testLurch"),
  ];
};

export const wingMount: CardRecipe = (event, p, play) => {
  if (play.hook !== undefined || event.type !== "fused") return null;
  const host = placeOf(p.view, event.resultInstanceId);
  const mount = host === null ? cardAt(p.view, play) : tid(host);
  const cues: FxCue[] = [{ kind: "brand", at: mount, tint: OUTLINE, delayMs: 0, durationMs: p.D }, { kind: "sheen", at: mount, delayMs: 0, durationMs: p.D }, ringCue(p.D, "sparkle", mount, 0)];
  if ((play.seen.fused ?? 1) <= 1) cues.push({ ...tunedBurst(p.env.intensity, "circuit", mount, "area", 0, TUNING.circuit), colors: lookOf(HOLOGRAM).colors });
  return replace(cues);
};
