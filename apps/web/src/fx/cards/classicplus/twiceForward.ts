// R3399000: Classic+ Twice Forward One Step Backwards (`classicplus-074`, takeover "stair", bespoke "stairGlue"), its signature (#3399):
// Takeover (wired, fx/takeover.ts): when it first turns face up (its first `trapFired`, R687), the vast stone
// stair turning on itself.
// Text, on each firing (hook "cast"): a ghost of the opponent's last card is dragged back two steps and glued
// into it (`fused`: a card back hauled from the opponent's hand pile to a step at the board's middle, then on
// to the trap, each step landing in violet dust, and a cyan glue ring and splash on the trap); its cyan
// cracks spread as it gains Brittle (`counterChanged` of its Brittle: a crack and cyan shards on the trap).
// Nothing moves from the concept: the engine emits `fused` and its Brittle's `counterChanged` (then
// `animated` when the copy was a Unit, whose own row plays). R202: the copy flies as a back and no cue
// names a card; the trap is drawn where it stands, else at the zone it fired from.

import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";

import { animTestid, targetFor } from "../../../game/animations.ts";
import { sideOf, testid } from "../../../game/contract.ts";
import { frac, ringCue, tid, tunedBurst, type BurstTuning } from "../../build.ts";
import { FX_CENTER, FX_CRACK_TAIL_MS } from "../../constants.ts";
import { otherSide } from "../../motifs/build.ts";
import type { FxAnchor, FxCue, FxPlay, FxPreset, FxSpread } from "../../types.ts";
import type { CardFxPlan, CardRecipe } from "../recipe.ts";

/** Every particle count the stair throws, at intensity "normal" (rule 9). */
const TUNING = {
  step: { count: 8, power: 0.6 },
  glue: { count: 14, power: 0.9 },
  shards: { count: 12, power: 1 },
} as const;

/** The stair's violet and its cracks' cyan (its takeover's tint), hot first. */
const VIOLET = ["#ede9fe", "#a78bfa", "#7c3aed"] as const;
const CYAN = ["#ecfeff", "#67e8f9", "#0891b2"] as const;

/** The step the copy is dragged back onto halfway: the board's middle. */
const STEP: FxAnchor = { kind: "viewport", at: { x: FX_CENTER.x, y: FX_CENTER.y } };

/** The two steps back as shares of D: the first hop lands at FIRST_STEP_AT, the second (on the trap) at GLUE_AT. */
const DRAG_AT = 0;
const FIRST_STEP_AT = 0.3;
const SECOND_STEP_AT = 0.35;
const GLUE_AT = 0.65;
/** Its cracks spread from the start of the Brittle's entry. */
const CRACK_AT = 0;

/** Where the trap stands: its own card on the board, else the zone it fired from, else its controller's hero. */
function trapAnchor(event: GameEvent, p: CardFxPlan, play: FxPlay): FxAnchor {
  const own = targetFor(event, p.view);
  if (own !== null) return tid(own);
  const zone = p.env.memory.trapZoneOf(play.instanceId);
  if (zone !== undefined) return tid(testid.zone(sideOf(p.view, zone.player), zone.row, zone.lane));
  return tid(testid.hero(sideOf(p.view, play.player)));
}

/** Classic+ #74 Twice Forward: the copy dragged back two steps and glued in, then the cracks of its Brittle. */
export const stairGlue: CardRecipe = (event, p, play) => {
  if (play.hook !== "cast") return null;
  const i = p.env.intensity;
  const burst = (preset: FxPreset, at: FxAnchor, spread: FxSpread, delay: number, tuning: BurstTuning, colors: readonly string[]): FxCue => ({
    ...tunedBurst(i, preset, at, spread, delay, tuning),
    colors,
  });
  if (event.type === "fused" && event.resultInstanceId === play.instanceId) {
    const trap = trapAnchor(event, p, play);
    const from = tid(animTestid.hand(otherSide(sideOf(p.view, play.player))));
    const drag = frac(DRAG_AT, p.D);
    const first = frac(FIRST_STEP_AT, p.D);
    const second = frac(SECOND_STEP_AT, p.D);
    const glue = frac(GLUE_AT, p.D);
    const cues: FxCue[] = [
      { kind: "ghost", from, to: STEP, delayMs: drag, durationMs: first - drag },
      burst("dust", STEP, "point", first, TUNING.step, VIOLET),
      { kind: "ghost", from: STEP, to: trap, delayMs: second, durationMs: glue - second },
      burst("dust", trap, "point", glue, TUNING.step, VIOLET),
      ringCue(p.D, "arcane", trap, glue),
      burst("arcane", trap, "area", glue, TUNING.glue, CYAN),
    ];
    return { cues, row: "keep" };
  }
  if (event.type === "counterChanged" && event.counter === "brittle" && event.instanceId === play.instanceId) {
    const trap = trapAnchor(event, p, play);
    const at = frac(CRACK_AT, p.D);
    return { cues: [{ kind: "crack", at: trap, delayMs: at, durationMs: p.D - at + FX_CRACK_TAIL_MS }, burst("shard", trap, "ring", at, TUNING.shards, CYAN)], row: "keep" };
  }
  return null;
};

/** The fused card the gallery's firing makes: the trap kept, a copy of a Spell fused in (R831011). */
const FIXTURE_FUSED = "t-1:classicplus-074+core-005";
/** Its Brittle after the firing: its printed 2, started at the reveal (R687), and the +1 it gains. */
const FIXTURE_BRITTLE = 3;

/** The gallery's Twice Forward: it fires from its zone, fuses in a copy of a Spell the opponent played and gains Brittle. */
export function stairGlueFixture(_view: PlayerView, instanceId: string, player: PlayerId): GameEvent[] {
  return [
    { type: "trapFired", instanceId, defId: "classicplus-074", controller: player, row: "backrow", lane: 1 },
    { type: "fused", instanceIds: [instanceId, "fx-stair-copy"], resultInstanceId: instanceId, defId: FIXTURE_FUSED },
    { type: "counterChanged", instanceId, counter: "brittle", value: FIXTURE_BRITTLE },
  ];
}
