// R3389001: Classic #33 Joro (`classic-033`), a Unit's signature: entrance "silkDrop", bespoke
// "webRedirect" and its aura "silkWeb" on its own zone (R3389000). Nothing shows while it waits in a hand
// (R202). When an enemy Spell targets a friendly unit, a silver thread drops from above into its zone
// and it swings down on it, upside-down, landing with a ring of silk; then a web strand snaps from the
// Spell's aim to it (`redirected`, by it): the Spell's light bends off its first target and strikes
// Joro, who flips upright with a wink of sparkle. Standing, a faint web shimmers in its zone.

import { frac, ringCue, shakeCues, tid, tunedBurst } from "../../build.ts";
import { FX_SLAM_AT } from "../../constants.ts";
import { FLAVOR_LOOKS, paletteLook, type FxPalette } from "../../flavors.ts";
import { hitAnchor, motifBurst, motifShot, timesOf } from "../../motifs/build.ts";
import type { FxCue, FxPoint } from "../../types.ts";
import { cardAnchor, ENTRANCE_TRAUMA, FOOT, type CardRecipe, type Entrance } from "../recipe.ts";
import { flourishSpec, type FlourishFixture } from "./flourish.ts";

/** The look its silk is drawn in. */
const PALETTE: FxPalette = "Physical/steel";

/** Every particle count it throws, the entrance's at intensity "normal", the recipe's at scale 1 (rule 9). */
const TUNING = {
  thread: { count: 8, power: 0.4 },
  swing: { count: 16, power: 0.9 },
  snap: { count: 10, power: 0.8 },
  wink: { count: 12, power: 0.9 },
} as const;

/** Where its thread hangs from: three of its zone's heights above it. */
const THREAD_TOP: FxPoint = { x: 0.5, y: -3 };

export const silkDrop: Entrance = ({ D, tgt, intensity: i, landMs }) => {
  const at = tid(tgt);
  const land = landMs ?? frac(FX_SLAM_AT, D);
  const look = paletteLook(PALETTE) ?? FLAVOR_LOOKS.Physical;
  const cues: FxCue[] = [
    { kind: "shot", shot: { motion: "beam" }, colors: look.colors, preset: "sparkle", from: tid(tgt, THREAD_TOP), to: at, delayMs: 0, flightMs: land, density: i },
    tunedBurst(i, "sparkle", tid(tgt, THREAD_TOP), "point", 0, TUNING.thread),
    ringCue(D, "sparkle", at, land),
    { ...tunedBurst(i, "sparkle", at, "ring", land, TUNING.swing), colors: look.colors },
    tunedBurst(i, "dust", tid(tgt, FOOT), "point", land, TUNING.thread),
    ...shakeCues(i, ENTRANCE_TRAUMA.silkDrop, land),
  ];
  return cues;
};

/** #33 Joro snaps a web from the Spell's aim to itself as it takes the Spell's target. */
export const webRedirect: CardRecipe = (event, p, play) => {
  if (event.type !== "redirected" || event.byInstanceId !== play.instanceId) return null;
  const spec = flourishSpec(PALETTE, play, p.view, p.env);
  if (!(p.D > 0) || !(spec.scale > 0)) return null;
  const t = timesOf(p.D);
  const joro = cardAnchor(p, play.instanceId) ?? hitAnchor(p.view, event.toId);
  const aim = hitAnchor(p.view, event.fromId);
  if (joro === null) return null;
  const cues: FxCue[] = [];
  if (aim !== null) cues.push(motifBurst(spec, aim, "point", t.G, TUNING.snap), motifShot(spec, "beam", aim, joro, t.T, t.L - t.T));
  cues.push(motifBurst(spec, joro, "ring", t.L, TUNING.wink, "sparkle"));
  return { cues, row: "keep" };
};

/** The gallery's Joro: the Spell aimed at its player's first unit is redirected to it. */
export const webRedirectFixture: FlourishFixture = (view, instanceId, player) => {
  const own = player === view.viewer ? view.you : view.opponent;
  const aimed = own.units.find((unit) => unit !== null && unit.instanceId !== instanceId);
  return aimed === undefined || aimed === null ? [] : [{ type: "redirected", what: "target", fromId: aimed.instanceId, toId: instanceId, byInstanceId: instanceId }];
};
