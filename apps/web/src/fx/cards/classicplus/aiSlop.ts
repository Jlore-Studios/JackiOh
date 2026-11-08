// R3399000: Classic+ AI Slop (`classicplus-043`, bespoke "holoLoom"), its signature (#3399):
// Cast: a holographic loom flickers up at the board's centre (a ring and a hologram burst) and two
// half-rendered cards scroll into it from the board's flanks (two ghosts), glitching at the edges (circuit
// flecks). Text: they smear together in cyan and magenta (`fused`: glitch bursts, a little off one another),
// the result jitters and drops to the caster's hand pile as a card back (`addedToHand`: a ghost), its cost
// ticking to zero (`costChanged`: a spark at the hand pile, unless the view hides it).
// R202: no cue names a card; the fused cards and the result are never drawn, ghosts are backs.

import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";

import { animTestid } from "../../../game/animations.ts";
import { sideOf } from "../../../game/contract.ts";
import { frac, ringCue, tid, tunedBurst } from "../../build.ts";
import { FLAVOR_LOOKS, paletteLook } from "../../flavors.ts";
import type { FxAnchor, FxCue } from "../../types.ts";
import type { CardRecipe } from "../recipe.ts";

/** Every particle count the loom throws, at intensity "normal" (rule 9). */
const TUNING = {
  loom: { count: 14, power: 0.8 },
  glitch: { count: 6, power: 0.6 },
  smear: { count: 14, power: 0.9 },
  jitter: { count: 8, power: 0.5 },
  tick: { count: 8, power: 0.7 },
} as const;

const LOOM: FxAnchor = { kind: "viewport", at: { x: 0.5, y: 0.5 } };
const FLANKS = [
  { kind: "viewport", at: { x: 0.15, y: 0.5 } },
  { kind: "viewport", at: { x: 0.85, y: 0.5 } },
] as const satisfies readonly FxAnchor[];
const JITTERS = [
  { kind: "viewport", at: { x: 0.48, y: 0.48 } },
  { kind: "viewport", at: { x: 0.52, y: 0.52 } },
] as const satisfies readonly FxAnchor[];
const LOOM_AT = 0;
const SCROLL_AT = 0.05;
const SCROLL_LAND_AT = 0.3;
const SMEAR_AT = 0.05;
const JITTER_AT = 0.2;
const DROP_AT = 0.3;
const DROP_LAND_AT = 0.45;
const TICK_AT = 0.1;
const HOLOGRAM = "Tech/hologram";
const GLITCH = "Tech/glitch";

/** Classic+ #43 AI Slop: the loom on its cast, the smear on its fusion, the drop on its add. */
export const holoLoom: CardRecipe = (event, p, play) => {
  if (play.hook !== undefined) return null;
  const i = p.env.intensity;
  const holo = paletteLook(HOLOGRAM) ?? FLAVOR_LOOKS.Tech;
  const glitch = paletteLook(GLITCH) ?? FLAVOR_LOOKS.Tech;
  if (event.type === "cardPlayed" && event.instanceId === play.instanceId) {
    const cues: FxCue[] = [
      ringCue(p.D, "sparkle", LOOM, frac(LOOM_AT, p.D)),
      { ...tunedBurst(i, "sparkle", LOOM, "area", frac(LOOM_AT, p.D), TUNING.loom), colors: holo.colors },
    ];
    for (const flank of FLANKS) {
      cues.push(
        { kind: "ghost", from: flank, to: LOOM, delayMs: frac(SCROLL_AT, p.D), durationMs: frac(SCROLL_LAND_AT - SCROLL_AT, p.D) },
        { ...tunedBurst(i, "circuit", flank, "area", frac(SCROLL_AT, p.D), TUNING.glitch), colors: glitch.colors },
      );
    }
    return { cues, row: "keep" };
  }
  if (event.type === "fused") {
    const cues: FxCue[] = [{ ...tunedBurst(i, "circuit", LOOM, "area", frac(SMEAR_AT, p.D), TUNING.smear), colors: glitch.colors }];
    for (const jitter of JITTERS) cues.push({ ...tunedBurst(i, "spark", jitter, "point", frac(JITTER_AT, p.D), TUNING.jitter), colors: glitch.colors });
    return { cues, row: "keep" };
  }
  if (event.type === "addedToHand") {
    const hand = tid(animTestid.hand(sideOf(p.view, event.player)));
    const cues: FxCue[] = [{ kind: "ghost", from: LOOM, to: hand, delayMs: frac(DROP_AT, p.D), durationMs: frac(DROP_LAND_AT - DROP_AT, p.D) }];
    return { cues, row: "keep" };
  }
  if (event.type === "costChanged") {
    if (event.hiddenFrom?.includes(p.view.viewer) === true) return null;
    const hand = tid(animTestid.hand(sideOf(p.view, play.player)));
    return { cues: [{ ...tunedBurst(i, "spark", hand, "point", frac(TICK_AT + DROP_LAND_AT, p.D), TUNING.tick), colors: holo.colors }], row: "keep" };
  }
  return null;
};

/** The gallery's Slop: two cards fuse into one that reaches the hand at cost (0). */
export function holoLoomFixture(_view: PlayerView, _instanceId: string, player: PlayerId): GameEvent[] {
  return [
    { type: "fused", instanceIds: ["fx-slop-a", "fx-slop-b"], resultInstanceId: "fx-slop-r", defId: "core-005" },
    { type: "addedToHand", player, instanceId: "fx-slop-r", defId: "core-005" },
    { type: "costChanged", instanceId: "fx-slop-r", cost: 0 },
  ];
}
