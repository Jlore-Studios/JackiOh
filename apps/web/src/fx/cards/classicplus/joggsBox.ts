// R3399000: Classic+ Jogg's Box (`classicplus-047`, bespoke "puzzleBox"), its signature (#3399):
// Cast: a carved puzzle box drops onto the board's centre (dust, a ring and a shake) and rattles, its panels
// slide open in a spiral (three smoke and confetti bursts around the centre, each a step round) spilling
// smoke in many colours. Text: on every random cast it makes (`cardAnnounced`, the first event of that
// cast, while Jogg's Box is still the resolving play) a sigil spins out of the box like a roulette ball (a
// ring and an arcane burst at the centre) and launches at the opponent's side (a bolt). The lid's snap and the
// board dimming come with the turn's end, which closes the play before its events arrive (memory.ts), so
// they are not drawn here (moved: dropped, the engine's turn-end events fall outside the play).
// R202: no cue names a card; the sigil is a bolt, never the random spell's face.

import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";

import { sideOf, testid } from "../../../game/contract.ts";
import { frac, ringCue, shakeCues, tid, tunedBurst } from "../../build.ts";
import { FLAVOR_LOOKS } from "../../flavors.ts";
import { motifShot, otherSide } from "../../motifs/build.ts";
import type { FxAnchor, FxCue } from "../../types.ts";
import type { CardRecipe } from "../recipe.ts";

/** Every particle count the box throws, at intensity "normal" (rule 9). */
const TUNING = {
  drop: { count: 14, power: 1 },
  panel: { count: 8, power: 0.7 },
  sigil: { count: 10, power: 0.8 },
  launch: { count: 8, power: 0.7 },
} as const;

const CENTER: FxAnchor = { kind: "viewport", at: { x: 0.5, y: 0.5 } };
/** The panels' spiral: each opens a step round the centre, in these smoke and confetti turns. */
const SPIRAL = [
  { at: { x: 0.56, y: 0.5 }, preset: "smoke" },
  { at: { x: 0.5, y: 0.56 }, preset: "confetti" },
  { at: { x: 0.44, y: 0.5 }, preset: "smoke" },
  { at: { x: 0.5, y: 0.44 }, preset: "confetti" },
] as const;
const DROP_AT = 0;
const DROP_SHAKE = 0.5;
const SPIRAL_AT = 0.1;
const SPIRAL_STEP = 0.05;
const SIGIL_AT = 0.05;
const LAUNCH_AT = 0.15;
const LAUNCH_LAND_AT = 0.4;
const MANY_COLORS = ["#ffd166", "#ef476f", "#06d6a0"] as const;
const LAUNCH_LOOK = FLAVOR_LOOKS.Chaos;

/** Classic+ #47 Jogg's Box: the drop and spiral on its cast, the roulette sigil on each random cast. */
export const puzzleBox: CardRecipe = (event, p, play) => {
  if (play.hook !== undefined) return null;
  const i = p.env.intensity;
  if (event.type === "cardPlayed" && event.instanceId === play.instanceId) {
    const cues: FxCue[] = [
      ringCue(p.D, "dust", CENTER, frac(DROP_AT, p.D)),
      { ...tunedBurst(i, "dust", CENTER, "area", frac(DROP_AT, p.D), TUNING.drop), colors: MANY_COLORS },
    ];
    SPIRAL.forEach((panel, k) => {
      cues.push({ ...tunedBurst(i, panel.preset, { kind: "viewport", at: panel.at }, "area", frac(SPIRAL_AT + k * SPIRAL_STEP, p.D), TUNING.panel), colors: MANY_COLORS });
    });
    cues.push(...shakeCues(i, DROP_SHAKE, frac(DROP_AT, p.D)));
    return { cues, row: "keep" };
  }
  if (event.type === "cardAnnounced") {
    const foe = tid(testid.hero(otherSide(sideOf(p.view, event.player))));
    const cues: FxCue[] = [
      ringCue(p.D, "arcane", CENTER, frac(SIGIL_AT, p.D)),
      { ...tunedBurst(i, "arcane", CENTER, "ring", frac(SIGIL_AT, p.D), TUNING.sigil), colors: LAUNCH_LOOK.colors },
      motifShot({ look: LAUNCH_LOOK, scale: i, row: "keep" }, "bolt", CENTER, foe, frac(LAUNCH_AT, p.D), frac(LAUNCH_LAND_AT - LAUNCH_AT, p.D)),
      { ...tunedBurst(i, "arcane", foe, "point", frac(LAUNCH_LAND_AT, p.D), TUNING.launch), colors: LAUNCH_LOOK.colors },
    ];
    return { cues, row: "keep" };
  }
  return null;
};

/** The gallery's Box: one random Spell is announced, whose cast then plays under its own rows. */
export function puzzleBoxFixture(_view: PlayerView, _instanceId: string, player: PlayerId): GameEvent[] {
  return [{ type: "cardAnnounced", player, instanceId: "fx-box-cast", defId: "core-005", cardType: "Spell", costPaid: 0, targets: [] }];
}
