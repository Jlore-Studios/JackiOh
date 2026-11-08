// R3405001: Meditative #47 饕餮 Taotie (`meditative-047`, entrance "bronzeCauldron", bespoke "taotie", a
// toxic steam on its tile), a Legendary Unit's signature. A verdigris bronze cauldron drops and slams onto
// its zone in a gout of steam, and the taotie's jawless horned face rises from it, its eyes kindling green.
// At the end of each turn its maw yawns: a green-bronze vortex opens under the enemy Unit it eats and drags
// its card back, tumbling, off its zone into the face, whose brow spirals flare as its stats swell. A card
// the board does not show (the Radiant face's second course, from the opponent's deck) is dragged from the
// opponent's deck pile instead. It keys on a fusion its own card is in (the turn hook's in a game, its play's
// in the gallery), never on a card's definition, and draws card backs alone (R202).

import { HIDDEN_ID, animTestid, locateInstance } from "../../../game/animations.ts";
import { sideOf } from "../../../game/contract.ts";
import { frac, ringCue, shakeCues, tid, tunedBurst } from "../../build.ts";
import { FX_SLAM_AT } from "../../constants.ts";
import { figureCue, figureTint } from "../../figures.ts";
import { FLAVOR_LOOKS, paletteLook } from "../../flavors.ts";
import { otherSide } from "../../motifs/build.ts";
import type { FxCue, FxPoint } from "../../types.ts";
import { ENTRANCE_TRAUMA, FOOT, type CardRecipe, type Entrance, type RecipeFixture } from "../recipe.ts";
import { standingAt } from "./knowledgeBreaker.ts";

/** The look its steam and vortex wear: Toxic's. */
const TAOTIE_PALETTE = "Toxic";

/** Its own definition, which the gallery's fusion keeps (R77: the kept card keeps its instance). */
const TAOTIE_ID = "meditative-047";

/** Every particle count its signature throws, at intensity "normal" (rule 9). */
const TUNING = {
  steam: { count: 24, power: 0.8 },
  eyes: { count: 6, power: 0.5 },
  vortex: { count: 16, power: 1 },
  swell: { count: 14, power: 1 },
} as const;

/**
 * As fractions of the entry: its eyes kindle at `eyesAt`; the eaten card is dragged from `dragAt` and
 * swallowed at `swallowAt`. The cauldron lasts its entry and `cauldronTailMs` more, the face from the slam
 * to `faceTailMs` past its entry, the yawning maw its fusion's entry and `mawTailMs` more.
 */
const TIMING = { eyesAt: 0.85, dragAt: 0.15, swallowAt: 0.55, cauldronTailMs: 300, faceTailMs: 400, mawTailMs: 300 } as const;

/** Where its eyes kindle on its zone: on the face risen over the cauldron. */
const EYES: FxPoint = { x: 0.5, y: 0.4 };

export const bronzeCauldron: Entrance = ({ D, tgt, intensity: i, view, landing, leadMs }) => {
  const look = paletteLook(TAOTIE_PALETTE) ?? FLAVOR_LOOKS.Toxic;
  const tint = figureTint(look);
  const lead = leadMs ?? 0;
  const slam = lead + frac(FX_SLAM_AT, D - lead);
  const eyes = lead + frac(TIMING.eyesAt, D - lead);
  const at = tid(tgt);
  const side = view === undefined || landing === undefined ? {} : { side: sideOf(view, landing.player) };
  return [
    figureCue("cauldron", at, tint, lead, D - lead + TIMING.cauldronTailMs, side),
    ringCue(D, look.particle, at, slam),
    { ...tunedBurst(i, "smoke", tid(tgt, FOOT), "area", slam, TUNING.steam), colors: look.colors },
    ...shakeCues(i, ENTRANCE_TRAUMA.bronzeCauldron, slam),
    figureCue("taotieMaw", at, tint, slam, D - slam + TIMING.faceTailMs, side),
    { ...tunedBurst(i, look.particle, tid(tgt, EYES), "point", eyes, TUNING.eyes), colors: look.colors },
  ];
};

export const taotie: CardRecipe = (event, p, play) => {
  if (event.type !== "fused" || (event.resultInstanceId !== play.instanceId && !event.instanceIds.includes(play.instanceId))) return null;
  const look = paletteLook(TAOTIE_PALETTE) ?? FLAVOR_LOOKS.Toxic;
  const i = p.env.intensity;
  const side = sideOf(p.view, play.player);
  const maw = standingAt(p, play);
  const eaten = event.instanceIds.find((id) => id !== play.instanceId && id !== event.resultInstanceId);
  const card = eaten === undefined || eaten === HIDDEN_ID ? null : locateInstance(p.view, eaten);
  const from = card === null ? tid(animTestid.library(otherSide(side))) : tid(card);
  const drag = frac(TIMING.dragAt, p.D);
  const swallow = frac(TIMING.swallowAt, p.D);
  const cues: FxCue[] = [
    figureCue("taotieMaw", maw, figureTint(look), 0, p.D + TIMING.mawTailMs, { side }),
    ringCue(p.D, look.particle, from, 0),
    { ...tunedBurst(i, look.particle, from, "ring", 0, TUNING.vortex), colors: look.colors },
    { kind: "ghost", from, to: maw, delayMs: drag, durationMs: swallow - drag, spin: true },
    ringCue(p.D, look.particle, maw, swallow),
    { ...tunedBurst(i, look.particle, maw, "area", swallow, TUNING.swell), colors: look.colors },
  ];
  return { cues, row: "replace" };
};

/** Its end of turn as `player` sees it: the first enemy Unit fused into it. */
export const taotieFixture: RecipeFixture = (view, instanceId, player) => {
  const foe = (player === view.viewer ? view.opponent : view.you).units.find((unit) => unit !== null);
  return foe === undefined || foe === null ? [] : [{ type: "fused", instanceIds: [instanceId, foe.instanceId], resultInstanceId: instanceId, defId: TAOTIE_ID }];
};
