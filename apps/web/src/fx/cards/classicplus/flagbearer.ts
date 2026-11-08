// R3399000: Classic+ Felinor Flagbearer (`classicplus-046`, entrance "bannerPlant", bespoke "bannerFold"), its
// signature (#3399): his entrance and his death.
// Entrance: he sprints in from his hero (a bolt) and plants his banner at the landing (dust, ring, shake); it
// unfurls with a snap (a spark burst); a gust of paw prints races along the caster's side, a shot to each end lane;
// and a ribbon (a beam) from the banner links to each other Felinor the side shows face up (`felinorZones`), which
// perks in a sparkle. The gallery board holds Felinors already.
// Death (`shuffledIn`): the banner falls in a puff of dust where he stood (his old zone if the view still shows it,
// else the owner's graveyard), folds, and flies as a ghost to the deck pile, where it sinks in with a sparkle.
// R202: nothing here names a card; the ghost is a card back.

import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";

import { animTestid } from "../../../game/animations.ts";
import { LANES, sideOf, testid } from "../../../game/contract.ts";
import { frac, shakeCues, tid } from "../../build.ts";
import { FX_FELINOR_STREAKS_MAX, FX_SLAM_AT } from "../../constants.ts";
import { FLAVOR_LOOKS, paletteLook } from "../../flavors.ts";
import { hitAnchor, motifBurst, motifRing, motifShot, timesOf, type CueSpec } from "../../motifs/build.ts";
import type { FxCue } from "../../types.ts";
import { felinorZones } from "../core/felinorFiender.ts";
import { ENTRANCE_TRAUMA, FOOT, type CardRecipe, type Entrance } from "../recipe.ts";

/** Every particle count the banner throws, at intensity "normal" (rule 9). */
const TUNING = {
  plant: { count: 22, power: 1.1 },
  snap: { count: 10, power: 1.2 },
  paws: { count: 6, power: 0.7 },
  perk: { count: 8, power: 0.8 },
  fall: { count: 12, power: 0.8 },
  sink: { count: 12, power: 0.8 },
} as const;

/** Entrance beats, as fractions of D. */
const SPRINT_AT = 0.05;
const SNAP_AT = 0.68;
const PAWS_AT = 0.7;
const PAWS_FLIGHT = 0.3;
const RIBBON_AT = 0.76;
const RIBBON_FLIGHT = 0.12;
const PERK_AT = 0.95;

const look = paletteLook("Beast/felinor") ?? FLAVOR_LOOKS.Beast;

export const bannerPlant: Entrance = ({ D, tgt, intensity: i, view, landing }) => {
  const spec: CueSpec = { look, scale: i, row: "keep" };
  const at = tid(tgt);
  const slam = frac(FX_SLAM_AT, D);
  const sprint = frac(SPRINT_AT, D);
  const cues: FxCue[] = [
    motifRing(spec, D, at, slam),
    motifBurst(spec, tid(tgt, FOOT), "area", slam, TUNING.plant, "dust"),
    motifBurst(spec, at, "point", frac(SNAP_AT, D), TUNING.snap, "spark"),
    ...shakeCues(i, ENTRANCE_TRAUMA.bannerPlant, slam),
  ];
  if (view === undefined || landing === undefined) return cues;
  const side = sideOf(view, landing.player);
  cues.push(motifShot(spec, "bolt", tid(testid.hero(side)), at, sprint, slam - sprint));
  for (const lane of [Math.min(...LANES), Math.max(...LANES)]) {
    const zone = testid.zone(side, "units", lane);
    if (zone !== tgt) cues.push(motifShot(spec, "bolt", at, tid(zone), frac(PAWS_AT, D), frac(PAWS_FLIGHT, D)));
  }
  cues.push(motifBurst(spec, at, "area", frac(PAWS_AT, D), TUNING.paws, "dust"));
  for (const zone of felinorZones(view, landing).slice(0, FX_FELINOR_STREAKS_MAX)) {
    cues.push(motifShot(spec, "beam", at, tid(zone), frac(RIBBON_AT, D), frac(RIBBON_FLIGHT, D)), motifBurst(spec, tid(zone), "area", frac(PERK_AT, D), TUNING.perk, "sparkle"));
  }
  return cues;
};

/** #46: his banner falls where he stood and flies into the deck pile. */
export const bannerFold: CardRecipe = (event, p, play) => {
  if (play.hook !== "death" || event.type !== "shuffledIn") return null;
  const t = timesOf(p.D);
  const spec: CueSpec = { look, scale: p.env.intensity, row: "keep" };
  const side = sideOf(p.view, event.player);
  const from = hitAnchor(p.view, play.instanceId) ?? tid(animTestid.graveyard(side));
  const pile = tid(animTestid.library(side));
  const cues: FxCue[] = [
    motifBurst(spec, from, "area", t.G, TUNING.fall, "dust"),
    { kind: "ghost", from, to: pile, delayMs: t.T, durationMs: t.L - t.T },
    motifBurst(spec, pile, "area", t.L, TUNING.sink, "sparkle"),
  ];
  return { cues, row: "keep" };
};

/** The gallery's death: the banner shuffles a Prime into the deck. */
export function bannerFoldFixture(_view: PlayerView, instanceId: string, player: PlayerId): GameEvent[] {
  return [
    { type: "hookResolving", sourceId: instanceId, defId: "classicplus-046", controller: player, hook: "death" },
    { type: "shuffledIn", player, instanceId: "fx-banner-prime", defId: "classicplus-046-1", position: 0 },
  ];
}
