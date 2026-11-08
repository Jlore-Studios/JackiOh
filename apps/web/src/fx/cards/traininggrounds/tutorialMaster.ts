// R3400001: Training Grounds #38 Tutorial Master (`traininggrounds-038`, entrance "chalkboardLesson",
// bespoke "tutorialMaster", chalk dust on his tile), a Legendary Unit's signature. A chalkboard swings down
// behind his zone, his pointer raps it twice, and as he lands a chalk outline of his tile draws itself
// around him. On his Cry and on his Death each card he adds slides to the caster's hand pile as a card
// back, from his card while the board shows it (else from his hero), the first along a chalk arrow; at his
// Death his chalk outline erases first. It keys on his play or his Death's entry and how many cards each
// added (`seen`), never on an added card, which no cue names (R202).

import { animTestid, locateInstance } from "../../../game/animations.ts";
import { sideOf } from "../../../game/contract.ts";
import { frac, ringCue, shakeCues, tid, tunedBurst } from "../../build.ts";
import { FX_SLAM_AT } from "../../constants.ts";
import { figureCue, figureTint } from "../../figures.ts";
import { FLAVOR_LOOKS, paletteLook } from "../../flavors.ts";
import type { FxCue, FxPoint } from "../../types.ts";
import { ENTRANCE_TRAUMA, heroOf, type CardRecipe, type Entrance } from "../recipe.ts";

/** The look his chalk wears: Nerd's chalk sub-look. */
const CHALK_PALETTE = "Nerd/chalk";

/** Every particle count his lesson throws, at intensity "normal" (rule 9). */
const TUNING = { rap: { count: 6, power: 0.6 }, land: { count: 18, power: 1 } } as const;

/**
 * As fractions of the entry: his pointer's two raps; the board lasts the entry and `boardTailMs` more,
 * the outline from the slam to `outlineTailMs` past it; at his Death the erase runs until `eraseEnd`.
 */
const TIMING = { raps: [0.2, 0.35], boardTailMs: 300, outlineTailMs: 400, eraseEnd: 0.5 } as const;

/** Where his pointer raps the board: near its top. */
const RAP_AT: FxPoint = { x: 0.5, y: 0.15 };

export const chalkboardLesson: Entrance = ({ D, tgt, intensity: i, view, landing }) => {
  const look = paletteLook(CHALK_PALETTE) ?? FLAVOR_LOOKS.Nerd;
  const tint = figureTint(look);
  const at = tid(tgt);
  const slam = frac(FX_SLAM_AT, D);
  const side = view === undefined || landing === undefined ? {} : { side: sideOf(view, landing.player) };
  const cues: FxCue[] = [
    figureCue("chalkboard", at, tint, 0, D + TIMING.boardTailMs, side),
    ...TIMING.raps.map((rap) => ({ ...tunedBurst(i, look.particle, tid(tgt, RAP_AT), "point", frac(rap, D), TUNING.rap), colors: look.colors })),
    figureCue("chalkOutline", at, tint, slam, D - slam + TIMING.outlineTailMs),
    ringCue(D, look.particle, at, slam),
    { ...tunedBurst(i, look.particle, at, "ring", slam, TUNING.land), colors: look.colors },
    ...shakeCues(i, ENTRANCE_TRAUMA.chalkboardLesson, slam),
  ];
  return cues;
};

export const tutorialMaster: CardRecipe = (event, p, play) => {
  // His Cry is his play's own; his Death opens an entry of its own (R2797000).
  if (play.hook !== undefined && play.hook !== "death") return null;
  if (event.type !== "addedToHand" || event.player !== play.player) return null;
  const look = paletteLook(CHALK_PALETTE) ?? FLAVOR_LOOKS.Nerd;
  const tint = figureTint(look);
  const side = sideOf(p.view, play.player);
  const card = locateInstance(p.view, play.instanceId);
  const from = card === null ? heroOf(p.view, play) : tid(card);
  const hand = tid(animTestid.hand(side));
  if ((play.seen.addedToHand ?? 1) > 1) return { cues: [{ kind: "ghost", from, to: hand, delayMs: 0, durationMs: p.D }], row: "replace" };
  // The first card: at his Death his outline erases first; then it slides along a chalk arrow.
  const start = play.hook === "death" ? frac(TIMING.eraseEnd, p.D) : 0;
  const cues: FxCue[] = play.hook === "death" ? [figureCue("erase", from, tint, 0, start)] : [];
  cues.push(figureCue("chalkArrow", hand, tint, start, p.D - start, { side }), { kind: "ghost", from, to: hand, delayMs: start, durationMs: p.D - start });
  return { cues, row: "replace" };
};
