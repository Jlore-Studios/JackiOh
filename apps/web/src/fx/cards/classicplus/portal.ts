// R3399000: Classic+ Portal to the Past (`classicplus-029`, takeover "portal", bespoke "sepiaPortal"), its signature (#3399):
// Takeover (wired, fx/takeover.ts): the carved stone ring grinding open into a sepia vortex, frozen ruins drifting in.
// Text: on its owner's seat alone, as the Discover opens (`promptOpened`), the last game's board ghosts over
// this one in sepia (a sepia haze over every unit zone of both sides) and the options rise from the portal (a
// sepia ring at its mouth, three rising motes where the options stand). On every seat one card back flies out
// of the portal to the caster's hand pile for each card it gives (`addedToHand`: a sepia gust at the portal, a
// ghost, dust on the pile; a glint on it for its owner alone).
// Moved: the view carries no last board, so its ghost is the sepia haze over this board's zones, not its cards.
// R202: the options and the card stay hidden (the opponent sees no Discover, only the back and the pile), and
// no cue names a card (R2739000: the options' count is never drawn).

import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";

import { animTestid } from "../../../game/animations.ts";
import { LANES, sideOf, testid } from "../../../game/contract.ts";
import { frac, ringCue, tid, tunedBurst, type BurstTuning } from "../../build.ts";
import { FX_CENTER } from "../../constants.ts";
import { otherSide, timesOf } from "../../motifs/build.ts";
import type { FxAnchor, FxCue, FxPreset, FxSpread } from "../../types.ts";
import type { CardRecipe } from "../recipe.ts";

/** Every particle count the portal throws, at intensity "normal" (rule 9). */
const TUNING = {
  haze: { count: 3, power: 0.3 },
  mouth: { count: 10, power: 0.9 },
  rise: { count: 6, power: 0.5 },
  gust: { count: 8, power: 0.8 },
  land: { count: 6, power: 0.6 },
} as const;

/** The portal's sepia (its takeover's tint, hot first). */
const SEPIA = ["#fef3c7", "#d6a35c", "#a16207"] as const;

/** The portal's mouth, the middle of the screen, and where the Discover's three options stand over it. */
const PORTAL: FxAnchor = { kind: "viewport", at: { x: FX_CENTER.x, y: FX_CENTER.y } };
const OPTIONS: readonly FxAnchor[] = [
  { kind: "viewport", at: { x: 0.3, y: FX_CENTER.y } },
  { kind: "viewport", at: { x: 0.5, y: FX_CENTER.y } },
  { kind: "viewport", at: { x: 0.7, y: FX_CENTER.y } },
];

/** The haze sweeps the lanes this share of D apart; the options rise this share apart after Travel (R200). */
const HAZE_STEP = 0.06;
const RISE_STEP = 0.08;
/** Each card the portal gives leaves this share of D after the one before, at most STAGGER_MAX steps in. */
const STEP = 0.08;
const STAGGER_MAX = 5;

/** Classic+ #29 Portal to the Past: the old board's ghost and the options for its owner, a back out of the portal for all. */
export const sepiaPortal: CardRecipe = (event, p, play) => {
  if (play.hook !== undefined) return null;
  const i = p.env.intensity;
  const t = timesOf(p.D);
  const burst = (preset: FxPreset, at: FxAnchor, spread: FxSpread, delay: number, tuning: BurstTuning): FxCue => ({
    ...tunedBurst(i, preset, at, spread, delay, tuning),
    colors: SEPIA,
  });
  const mine = sideOf(p.view, play.player);
  if (event.type === "promptOpened" && event.player === play.player && play.player === p.view.viewer && play.seen.promptOpened === 1) {
    const cues: FxCue[] = [ringCue(p.D, "dust", PORTAL, t.G), burst("dust", PORTAL, "ring", t.G, TUNING.mouth)];
    LANES.forEach((lane, k) => {
      for (const side of [mine, otherSide(mine)]) cues.push(burst("smoke", tid(testid.zone(side, "units", lane)), "area", frac(k * HAZE_STEP, p.D), TUNING.haze));
    });
    OPTIONS.forEach((at, k) => cues.push(burst("sparkle", at, "point", t.T + frac(k * RISE_STEP, p.D), TUNING.rise)));
    return { cues, row: "keep" };
  }
  if (event.type === "addedToHand" && event.player === play.player) {
    const same = p.entry.events.filter((e) => e.type === "addedToHand");
    const k = Math.min(Math.max(0, same.indexOf(event)), STAGGER_MAX);
    const at = frac(k * STEP, p.D);
    const hand = tid(animTestid.hand(mine));
    const cues: FxCue[] = [
      burst("dust", PORTAL, "point", at, TUNING.gust),
      { kind: "ghost", from: PORTAL, to: hand, delayMs: at, durationMs: p.D - at },
      burst("dust", hand, "point", t.L, TUNING.land),
    ];
    if (play.player === p.view.viewer) cues.push(burst("sparkle", hand, "point", t.L, TUNING.land));
    return { cues, row: "keep" };
  }
  return null;
};

/** The gallery's Portal: its Discover opens, its player picks, and the card arrives in their hand. */
export function sepiaPortalFixture(_view: PlayerView, _instanceId: string, player: PlayerId): GameEvent[] {
  return [
    { type: "promptOpened", player, choiceId: "fx-portal-choice", kind: "discover" },
    { type: "promptAnswered", player, choiceId: "fx-portal-choice" },
    { type: "addedToHand", player, instanceId: "fx-portal", defId: "core-012" },
  ];
}
