// R3337002: Stinkyscape #166 and #167 Scanning Probe Mk1 and Mk2 (`stinkyscape-166`, `-167`, bespoke
// "probeScan"), whose scan may find an Anomaly Found! (R1855003). At the probe's own `cardPlayed` a radar
// sweep crosses the board from its caster's hand, on both seats (the play is public). When the scan adds
// an Anomaly Found! its owner's seat reads it (`addedToHand` names it): a ping on the board, the card
// flying from the ping into the hand, and a strong flash as it lands when it is Radiant (the owner's
// next view says so); that draws in place of the card's own row. The other seat's `addedToHand` names R97's
// sentinel, so it keys nothing here and plays its own row, as any card added to a hand does (R202). A scan
// that finds nothing adds nothing: the sweep alone, which fades.

import type { PlayerView } from "@jackioh/shared";

import { ANOMALY_FOUND_ID } from "../../../cards/anomalies.ts";
import { animTestid } from "../../../game/animations.ts";
import { sideOf, testid } from "../../../game/contract.ts";
import { raysCue, ringCue, shakeCues, tid } from "../../build.ts";
import { FLAVOR_LOOKS, paletteLook, type FlavorLook } from "../../flavors.ts";
import { motifBurst, motifScale, timesOf, type CueSpec } from "../../motifs/build.ts";
import type { FxCue, FxTint } from "../../types.ts";
import type { CardRecipe } from "../recipe.ts";

/** R3337002: the sweep outlasts the probe's entry by this much, inside FX_MAX_TAIL_MS (R200). */
export const PROBE_RADAR_TAIL_MS = 500;
/** R3337002: the shake of a Radiant find's flash. */
export const PROBE_RADIANT_TRAUMA = 0.35;

/** Every particle count the scan throws at scale 1 (rule 9). */
const TUNING = {
  ping: { count: 14, power: 0.9 },
  arrive: { count: 12, power: 0.9 },
  radiant: { count: 28, power: 1.3 },
} as const;

/** The scan's look: Tech's starfield, the scan of space. */
function scanLook(): FlavorLook {
  return paletteLook("Tech/starfield") ?? FLAVOR_LOOKS.Tech;
}

/** The look's colors as the sweep's tint, as a set piece's (setPieces.ts `tintOf`, which the table's own import chain cannot reach). */
function tintOf(look: FlavorLook): FxTint {
  const [glow, rim, core] = look.colors;
  return { glow, rim, core: core ?? glow };
}

/** Whether the viewer's own next view shows `instanceId` in their hand as Radiant. */
function radiantInHand(next: PlayerView | undefined, instanceId: string): boolean {
  const hand = next?.you.hand;
  return Array.isArray(hand) && hand.some((card) => card.instanceId === instanceId && card.radiant);
}

/** R3337002: the Scanning Probes' sweep at their own play, and a find's ping and flight on its owner's seat. */
export const probeScan: CardRecipe = (event, p, play) => {
  const spec: CueSpec = { look: scanLook(), scale: motifScale(play.defId, p.view, p.env.intensity), row: "keep" };
  if (event.type === "cardPlayed" && event.instanceId === play.instanceId) {
    const side = sideOf(p.view, event.player);
    const radar: FxCue = { kind: "radar", at: tid(animTestid.hand(side)), side, tint: tintOf(spec.look), delayMs: 0, durationMs: p.D + PROBE_RADAR_TAIL_MS };
    return { cues: [radar], row: "keep" };
  }
  // R202: the other seat's find names the sentinel, never Anomaly Found!.
  if (event.type !== "addedToHand" || event.defId !== ANOMALY_FOUND_ID) return null;
  const t = timesOf(p.D);
  const ping = tid(testid.board);
  const hand = tid(animTestid.hand(sideOf(p.view, event.player)));
  const cues: FxCue[] = [
    ringCue(t.D, "star", ping, t.G),
    motifBurst(spec, ping, "point", t.G, TUNING.ping, "sparkle"),
    { kind: "ghost", from: ping, to: hand, delayMs: t.T, durationMs: t.L - t.T },
  ];
  if (radiantInHand(p.env.next, event.instanceId)) {
    cues.push(raysCue(t.D, "radiant", hand, t.L), motifBurst(spec, hand, "area", t.L, TUNING.radiant, "prismatic"), ...shakeCues(spec.scale, PROBE_RADIANT_TRAUMA, t.L));
  } else {
    cues.push(motifBurst(spec, hand, "point", t.L, TUNING.arrive, "star"));
  }
  return { cues, row: "replace" };
};
