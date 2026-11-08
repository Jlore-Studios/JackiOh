// R3399000: Classic+ KY's Test (`classicplus-042`, bespoke "slateTest"), its signature (#3399):
// Cast: a slate board slides down over the caster's half (a sheen on their middle unit zone) and chalk squeaks
// out three boxes of rising size across lanes 1, 3 and 5 (dust, a bigger puff each); a red pen circles the
// biggest one (a red ring). Text: the three problems hang over the caster's hand, for the caster alone (a
// sheen and chalk dust at their hand pile on the mode prompt); after the pick a tick stamps the slate and the
// reward drops from it to the hand pile as a ghost, a back to the opponent too.
// R202: no cue names a card; the prompt's options are never drawn, the hand pile and the zones are anchors.

import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";

import { animTestid } from "../../../game/animations.ts";
import { sideOf, testid } from "../../../game/contract.ts";
import { frac, ringCue, shakeCues, tid, tunedBurst } from "../../build.ts";
import { FLAVOR_LOOKS, paletteLook } from "../../flavors.ts";
import type { FxCue } from "../../types.ts";
import type { CardRecipe } from "../recipe.ts";

/** Every particle count the slate throws, at intensity "normal" (rule 9). */
const TUNING = {
  boxSmall: { count: 6, power: 0.5 },
  boxMid: { count: 9, power: 0.6 },
  boxBig: { count: 14, power: 0.7 },
  pen: { count: 10, power: 0.8 },
  problems: { count: 8, power: 0.5 },
  tick: { count: 10, power: 0.8 },
  reward: { count: 8, power: 0.6 },
} as const;

/** The three boxes' lanes, small to big, and where each chalks (fractions of the entry). */
const BOX_LANES = [1, 3, 5] as const;
const BOX_TUNING = [TUNING.boxSmall, TUNING.boxMid, TUNING.boxBig] as const;
const SLATE_LANE = 3;
const BIGGEST_LANE = 5;
const SLATE_AT = 0;
const CHALK_AT = 0.1;
const CHALK_STEP = 0.1;
const PEN_AT = 0.45;
const SHAKE_AT = 0.05;
const SLATE_SHAKE = 0.15;
const PROBLEMS_AT = 0.1;
const TICK_AT = 0.1;
const REWARD_AT = 0.1;
const REWARD_LAND_AT = 0.4;
const PEN_COLORS = ["#ffe5e5", "#ff4d4d", "#a30d1f"] as const;
const PALETTE = "Nerd/chalk";

/** Classic+ #42 KY's Test: the slate on its cast, the problems on its prompt, the stamp on its reward. */
export const slateTest: CardRecipe = (event, p, play) => {
  if (play.hook !== undefined) return null;
  const look = paletteLook(PALETTE) ?? FLAVOR_LOOKS.Nerd;
  const i = p.env.intensity;
  const mine = sideOf(p.view, play.player);
  const slate = tid(testid.zone(mine, "units", SLATE_LANE));
  if (event.type === "cardPlayed" && event.instanceId === play.instanceId) {
    const cues: FxCue[] = [{ kind: "sheen", at: slate, delayMs: frac(SLATE_AT, p.D), durationMs: p.D }];
    BOX_LANES.forEach((lane, k) => {
      const at = tid(testid.zone(mine, "units", lane));
      cues.push({ ...tunedBurst(i, "dust", at, "area", frac(CHALK_AT + k * CHALK_STEP, p.D), BOX_TUNING[k] ?? TUNING.boxSmall), colors: look.colors });
    });
    const biggest = tid(testid.zone(mine, "units", BIGGEST_LANE));
    cues.push(ringCue(p.D, "blood", biggest, frac(PEN_AT, p.D)), { ...tunedBurst(i, "blood", biggest, "ring", frac(PEN_AT, p.D), TUNING.pen), colors: PEN_COLORS });
    cues.push(...shakeCues(i, SLATE_SHAKE, frac(SHAKE_AT, p.D)));
    return { cues, row: "keep" };
  }
  if (event.type === "promptOpened") {
    if (play.player !== p.view.viewer || event.player !== p.view.viewer || event.kind !== "mode") return null;
    const hand = tid(animTestid.hand(mine));
    const cues: FxCue[] = [
      { kind: "sheen", at: hand, delayMs: frac(PROBLEMS_AT, p.D), durationMs: p.D },
      { ...tunedBurst(i, "dust", hand, "area", frac(PROBLEMS_AT, p.D), TUNING.problems), colors: look.colors },
    ];
    return { cues, row: "keep" };
  }
  if (event.type === "addedToHand") {
    const hand = tid(animTestid.hand(sideOf(p.view, event.player)));
    const cues: FxCue[] = [
      { ...tunedBurst(i, "sparkle", slate, "point", frac(TICK_AT, p.D), TUNING.tick), colors: look.colors },
      { kind: "ghost", from: slate, to: hand, delayMs: frac(REWARD_AT, p.D), durationMs: frac(REWARD_LAND_AT - REWARD_AT, p.D) },
    ];
    if (event.player === p.view.viewer) cues.push({ ...tunedBurst(i, "sparkle", hand, "area", frac(REWARD_LAND_AT, p.D), TUNING.reward), colors: look.colors });
    return { cues, row: "keep" };
  }
  return null;
};

/** The gallery's Test: its player takes the mode prompt, answers, and a reward reaches their hand. */
export function slateTestFixture(_view: PlayerView, _instanceId: string, player: PlayerId): GameEvent[] {
  return [
    { type: "promptOpened", player, choiceId: "fx-test-mode", kind: "mode" },
    { type: "promptAnswered", player, choiceId: "fx-test-mode" },
    { type: "addedToHand", player, instanceId: "fx-test-reward", defId: "core-t-coin" },
  ];
}
