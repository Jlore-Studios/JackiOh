// R3395000: Stinkyscape #34 Drone Core VIII (`stinkyscape-034`, bespoke "droneConveyor"), a Legendary
// Spell's signature. As it is cast a cracked reactor glows over its caster's hero and its leftover energy
// pours out in a blue torrent, and a conveyor beam unrolls along the caster's unit row from its first zone
// to its last. Each Drone Interceptor it fills the board with is stamped off the line as its `summoned`
// lands: a press slams onto its zone, each stamp quicker than the last and each shake a little harder, the
// first two ringing with a weld spark. Every beat replaces its summon's own row, so the cast fits the
// Legendary Spell's ceiling (R3332000). It keys on its own play and how many summons it has seen, never on
// a card's definition (R202). Moved: "the last stamp shakes the board" is a shake rising stamp by stamp,
// since a beat cannot know which summon is the last.

import { sideOf, testid } from "../../../game/contract.ts";
import { ringCue, shakeCues, tid } from "../../build.ts";
import { figureCue, figureTint } from "../../figures.ts";
import { motifBurst, motifScale, motifShot, type CueSpec } from "../../motifs/build.ts";
import type { FxCue } from "../../types.ts";
import { heroOf, type CardRecipe } from "../recipe.ts";
import { lookOf, replace } from "./signatureKit.ts";

/** The reactor's blue and the line's rocket steel. */
const REACTOR = "Tech/starfield";
const LINE = "Tech/rocket";

/** The unit row's first and last lanes, the conveyor's two ends. */
const ROW_ENDS = [1, 5] as const;

/** Every particle count the cast throws at scale 1 (rule 9). */
const TUNING = {
  torrent: { count: 28, power: 1.2 },
} as const;

/** As shares of the entry: the conveyor unrolls from `lineAt` for `lineFlight`; each stamp is `stampStep` shorter, never under `stampMin`. */
const TIMING = { lineAt: 0.2, lineFlight: 0.3, stampStep: 0.15, stampMin: 0.4 } as const;

/** Each stamp's shake: the first's, and how much harder each next one lands (a Spell's at most 1). */
const STAMP_TRAUMA = { first: 0.2, step: 0.1 } as const;

/** How many stamps ring their zone with a weld spark; later ones stamp and shake alone, inside the ceiling. */
const RINGED_STAMPS = 2;

export const droneConveyor: CardRecipe = (event, p, play) => {
  const spec: CueSpec = { look: lookOf(LINE), scale: motifScale(play.defId, p.view, p.env.intensity), row: "replace" };
  if (event.type === "cardPlayed" && event.instanceId === play.instanceId) {
    const hero = heroOf(p.view, play);
    const side = sideOf(p.view, play.player);
    const [first, last] = ROW_ENDS;
    const line = motifShot(spec, "beam", tid(testid.zone(side, "units", first)), tid(testid.zone(side, "units", last)), Math.round(TIMING.lineAt * p.D), Math.round(TIMING.lineFlight * p.D));
    return replace([
      figureCue("reactor", hero, figureTint(lookOf(REACTOR)), 0, p.D),
      motifBurst({ ...spec, look: lookOf(REACTOR) }, hero, "area", 0, TUNING.torrent),
      line,
    ]);
  }
  if (event.type !== "summoned" || event.player !== play.player) return null;
  const k = Math.max(0, (play.seen.summoned ?? 1) - 1);
  const zone = tid(testid.zone(sideOf(p.view, event.player), event.row, event.lane));
  const stamp = Math.round(Math.max(TIMING.stampMin, 1 - k * TIMING.stampStep) * p.D);
  const cues: FxCue[] = [figureCue("press", zone, figureTint(spec.look), 0, stamp)];
  if (k < RINGED_STAMPS) cues.push(ringCue(p.D, "spark", zone, 0));
  cues.push(...shakeCues(spec.scale, STAMP_TRAUMA.first + k * STAMP_TRAUMA.step, 0));
  return replace(cues);
};
