// #2798: Stinkyscape #146 Flash Freeze (`stinkyscape-146`, bespoke "flashFreeze"), bespoke because it
// freezes the whole board in one breath and no motif reads `marked`: at its first freeze a wave of frost
// rolls across both sides lane by lane into one ring over the board, and each Unit it freezes takes a
// sheen and a frost burst, all in the freeze mark's own colors. It anchors on the lanes, the board and
// the shown ids alone, the same on both seats (R202); each mark's own brand row still plays.

import { locateInstance } from "../../../game/animations.ts";
import { LANES, testid } from "../../../game/contract.ts";
import { shakeCues, tid } from "../../build.ts";
import { FX_FLASH_FREEZE_TRAUMA } from "../../constants.ts";
import { isFirstEntry, MOTIF_TUNING, motifBurst, motifRing, timesOf, type CueSpec } from "../../motifs/build.ts";
import type { FxCue } from "../../types.ts";
import { flourishSpec } from "../core/flourish.ts";
import type { CardRecipe } from "../recipe.ts";
import { markLook } from "./markLook.ts";

const MARKED = ["marked"] as const;

export const flashFreeze: CardRecipe = (event, p, play) => {
  if (event.type !== "marked" || !event.added) return null;
  const base = flourishSpec(p, play);
  const spec: CueSpec = { ...base, look: markLook(event.color, base.look) };
  const t = timesOf(p.D);
  const cues: FxCue[] = [];
  const first = p.entry.events.find((candidate) => candidate.type === "marked") === event && isFirstEntry(p.env, p.entry, MARKED);
  if (first) {
    const last = Math.max(1, LANES.length - 1);
    for (const lane of LANES) {
      const at = t.T + Math.round(((lane - 1) * (t.L - t.T)) / last);
      for (const side of ["you", "opponent"] as const) cues.push(motifBurst(spec, tid(testid.zone(side, "units", lane)), "area", at, MOTIF_TUNING.wave));
    }
    cues.push(motifRing(spec, p.D, tid(testid.board), t.L), ...shakeCues(spec.scale, FX_FLASH_FREEZE_TRAUMA, t.L));
  }
  const at = locateInstance(p.view, event.instanceId);
  if (at !== null) cues.push({ kind: "sheen", at: tid(at), delayMs: t.T, durationMs: t.L - t.T }, motifBurst(spec, tid(at), "point", t.L, MOTIF_TUNING.small));
  return cues.length === 0 ? null : { cues, row: "keep" };
};
