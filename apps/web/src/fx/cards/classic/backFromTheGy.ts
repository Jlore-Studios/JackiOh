// R3389001: Classic #44 Back from the GY (`classic-044`), a Spell's signature: bespoke "graveRise", a cast
// of its own in place of the shared Legendary rays (R3389000). As it is played (its own `cardPlayed`), a
// violet lantern flare bursts over its caster's graveyard pile and the ground cracks open along the
// caster's unit row, the board rumbling; each Unit it summons claws up through its zone from below, the
// soil ringing out and cracking (in place of the summon's own row); then the Spell crumbles to violet ash
// and drifts into the exile pile (its own `exiled`, in place of that row). A Spell wears no aura. It
// keeps a Spell's shorter budget (SIGNATURE_SPELL_SHARE), so every burst is its play's own entry's.

import { animTestid, targetFor } from "../../../game/animations.ts";
import { LANES, sideOf, testid } from "../../../game/contract.ts";
import { shakeCues, tid } from "../../build.ts";
import { FX_CRACK_TAIL_MS } from "../../constants.ts";
import type { FxPalette } from "../../flavors.ts";
import { motifBurst, motifRing, timesOf } from "../../motifs/build.ts";
import type { FxCue } from "../../types.ts";
import { cardAnchor, heroOf, ownWindow, SIGNATURE_TRAUMA, stepMs, type CardRecipe } from "../recipe.ts";
import { flourishSpec, type FlourishFixture } from "./flourish.ts";

/** The look its lantern and its ash glow in. */
const PALETTE: FxPalette = "Dark/curse";

/** Every particle count it throws, at scale 1 (rule 9). */
const TUNING = {
  flare: { count: 18, power: 1.2 },
  soil: { count: 12, power: 1 },
} as const;

/** #44 Back from the GY opens the graveyard and raises what it summons, then crumbles into exile. */
export const graveRise: CardRecipe = (event, p, play) => {
  if (!ownWindow(p, play)) return null;
  const spec = flourishSpec(PALETTE, play, p.view, p.env);
  if (!(p.D > 0) || !(spec.scale > 0)) return null;
  const t = timesOf(p.D);
  const side = sideOf(p.view, play.player);
  if (event.type === "cardPlayed" && event.instanceId === play.instanceId) {
    const grave = tid(animTestid.graveyard(side));
    const cues: FxCue[] = [
      motifBurst(spec, grave, "area", t.G, TUNING.flare),
      motifRing(spec, p.D, grave, t.G),
      motifBurst(spec, grave, "ring", t.T, TUNING.soil, "dust"),
    ];
    // The ground cracks open along the caster's unit row, lane by lane.
    LANES.forEach((lane, k) => {
      const at = stepMs(p.D, t.T, k);
      cues.push({ kind: "crack", at: tid(testid.zone(side, "units", lane)), delayMs: at, durationMs: p.D - at + FX_CRACK_TAIL_MS });
    });
    cues.push(...shakeCues(p.env.intensity, SIGNATURE_TRAUMA.graveRise, t.T));
    return { cues, row: "keep" };
  }
  if (event.type === "summoned" && event.player === play.player && event.row === "units" && event.instanceId !== play.instanceId) {
    const tgt = targetFor(event, p.view);
    if (tgt === null) return null;
    const at = tid(tgt);
    return {
      cues: [motifRing(spec, p.D, at, t.G), { kind: "crack", at, delayMs: t.G, durationMs: p.D - t.G + FX_CRACK_TAIL_MS }],
      row: "replace",
    };
  }
  if (event.type === "exiled" && event.instanceId === play.instanceId) {
    const from = cardAnchor(p, play.instanceId) ?? heroOf(p.view, play);
    return { cues: [{ kind: "ghost", from, to: tid(animTestid.exile(side)), delayMs: t.G, durationMs: p.D - t.G }], row: "replace" };
  }
  return null;
};

/** The gallery's Back from the GY: it summons a Unit from the graveyard, then exiles itself. */
export const graveRiseFixture: FlourishFixture = (_view, instanceId, player) => [
  { type: "summoned", player, instanceId: `${instanceId}-risen`, defId: "token-sheep", row: "units", lane: 1 },
  { type: "exiled", instanceId, defId: "classic-044", owner: player },
];
