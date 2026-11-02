// R502: #21 Hinder (`core-021`, bespoke "manaCrack") cracks the crystals of the victim's tray that its
// next refresh will not fill, as many as the view's rider badge says (manaMarks.ts).

import { animTestid } from "../../../game/animations.ts";
import { sideOf } from "../../../game/contract.ts";
import { frac, ringCue, shakeCues, tid, tunedBurst } from "../../build.ts";
import {
  FX_CRACK_HIT_AT,
  FX_CRACK_STAGGER_MS,
  FX_CRACK_TAIL_MS,
  FX_CRACK_TRAUMA,
  FX_FRACTURE_TAIL_MS,
  FX_NEXT_REFRESH_MODIFIER_ID,
} from "../../constants.ts";
import { lostCrystals } from "../../manaMarks.ts";
import type { FxAnchor, FxCue } from "../../types.ts";
import { heroOf, projectile, type CardRecipe } from "../recipe.ts";

/** Every particle count the crack throws, at intensity "normal" (rule 9). */
const TUNING = {
  crackFrost: { count: 22, power: 1.1 },
  crystalShard: { count: 10, power: 1 },
} as const;

/** #21 Hinder cracks next-refresh crystals using the view's count (manaMarks.ts). */
export const manaCrack: CardRecipe = (event, p) => {
  if (event.type !== "modifierChanged" || event.modifierId !== FX_NEXT_REFRESH_MODIFIER_ID || !event.added) return null;
  const i = p.env.intensity;
  const side = sideOf(p.view, event.player);
  const tray = tid(animTestid.mana(side));
  const hit = frac(FX_CRACK_HIT_AT, p.D);
  const caster = p.env.memory.resolving();
  const cues: FxCue[] = [];
  if (caster !== undefined) cues.push(projectile(i, "frost", heroOf(p.view, caster), tray, 0, hit));
  cues.push(ringCue(p.D, "frost", tray, hit), tunedBurst(i, "frost", tray, "area", hit, TUNING.crackFrost));
  const lost = lostCrystals(p.env.next, p.view, event.player);
  if (lost.count === 0) {
    cues.push({ kind: "crack", at: tray, delayMs: hit, durationMs: p.D - hit + FX_CRACK_TAIL_MS });
  }
  for (let k = 0; k < lost.count; k += 1) {
    const crystal: FxAnchor = { kind: "crystal", side, index: lost.from + k };
    const at = Math.min(p.D, hit + k * FX_CRACK_STAGGER_MS);
    cues.push(
      { kind: "fracture", at: crystal, delayMs: at, durationMs: p.D - at + FX_FRACTURE_TAIL_MS },
      { kind: "crack", at: crystal, delayMs: at, durationMs: p.D - at + FX_CRACK_TAIL_MS },
      tunedBurst(i, "frost", crystal, "point", at, TUNING.crystalShard),
    );
  }
  cues.push(...shakeCues(i, FX_CRACK_TRAUMA, hit));
  return { cues, row: "replace" };
};
