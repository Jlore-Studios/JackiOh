// R3395000: Stinkyscape #86 Prototype-X2 (`stinkyscape-086`, entrance "forgeBurst", bespoke "spineWeld",
// takeover "forge", embers on its tile), a Mythic Unit's signature. Its takeover is a forge-sphere filling the
// dark, five ships spiralling into it until it cracks open in molten orange and the hull bursts out
// (fx/takeoverArt.ts); then a molten shell splits over its zone and it snarls out in fire. Its Cry Equips the
// Epic Weapons it Discovers: each is hammered onto its spine as its `fused` lands, clamps closing and a weld
// flashing, the first in a spray of sparks and a shake, the second ringing, a third welding alone inside the
// Mythic's ceiling. It keys on its play and the fused card's public id, never on a Weapon's definition; the
// Discover's offer stays its caster's (R202). Moved: the tributes sinking into its welds are paid before its
// play opens; the cooling welds and cockpit lights on its tile are rising embers.

import { ringCue, shakeCues, tid, tunedBurst } from "../../build.ts";
import type { FxCue } from "../../types.ts";
import { FOOT, type CardRecipe, type Entrance } from "../recipe.ts";
import { cardAt, entranceFigure, lookOf, placeOf, replace, slamAt, slamShake } from "./signatureKit.ts";

/** Its molten forge. */
const FORGE = "Fire";

/** How hard each weld hums (fx.css's `--fx-hum`), and the first weld's shake (a Unit's at most 0.8). */
const WELD_HUM = 1;
const WELD_TRAUMA = 0.45;

/** As a share of the entry, how long a third or later weld lasts, inside the Mythic's ceiling. */
const TIMING = { lateWeld: 0.4 } as const;

/** Every particle count it throws, at intensity "normal" (rule 9). */
const TUNING = {
  molten: { count: 28, power: 1.3 },
  sparks: { count: 18, power: 1.1 },
} as const;

export const forgeBurst: Entrance = (plan) => {
  const at = tid(plan.tgt);
  const slam = slamAt(plan);
  return [
    entranceFigure("forgeShell", FORGE, plan),
    { ...tunedBurst(plan.intensity, "fire", at, "area", slam, TUNING.molten), colors: lookOf(FORGE).colors },
    tunedBurst(plan.intensity, "ember", tid(plan.tgt, FOOT), "ring", slam, TUNING.sparks),
    ringCue(plan.D, "fire", at, slam),
    ...slamShake(plan, "forgeBurst"),
  ];
};

export const spineWeld: CardRecipe = (event, p, play) => {
  if (play.hook !== undefined || event.type !== "fused") return null;
  const host = placeOf(p.view, event.resultInstanceId);
  const spine = host === null ? cardAt(p.view, play) : tid(host);
  const n = play.seen.fused ?? 1;
  const weldMs = n > 2 ? Math.round(TIMING.lateWeld * p.D) : p.D;
  const cues: FxCue[] = [{ kind: "weld", at: spine, hum: WELD_HUM, delayMs: 0, durationMs: weldMs }];
  if (n <= 2) cues.push(ringCue(p.D, "fire", spine, 0));
  if (n <= 1) cues.push({ ...tunedBurst(p.env.intensity, "spark", spine, "area", 0, TUNING.sparks), colors: lookOf(FORGE).colors }, ...shakeCues(p.env.intensity, WELD_TRAUMA, 0));
  return replace(cues);
};
