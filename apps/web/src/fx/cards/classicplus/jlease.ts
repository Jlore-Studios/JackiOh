// R3399000: Classic+ J-lease J-Jungle EX-plorer (`classicplus-075`, entrance "vineSwing", bespoke "packLob"), its
// signature (#3399): her entrance and her Cry.
// Entrance: she swings in on a jungle vine, a leaf bolt from the board's top edge to her zone; she lands in a crouch
// (dust, ring, shake), tips her hat (a sparkle) and leaves flutter down, two leaf bursts from above the zone.
// Cry (`shuffledIn`): she lobs a bulging pack from her zone (a leaf puff) in an arc, a ghost, onto the deck pile,
// where it sinks in with a sparkle and a ring.
// R202: nothing here names a card; the pack is a card back.

import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";

import { animTestid } from "../../../game/animations.ts";
import { sideOf } from "../../../game/contract.ts";
import { frac, shakeCues, tid } from "../../build.ts";
import { FX_SLAM_AT } from "../../constants.ts";
import { FLAVOR_LOOKS, paletteLook } from "../../flavors.ts";
import { hitAnchor, motifBurst, motifRing, motifShot, timesOf, type CueSpec } from "../../motifs/build.ts";
import type { FxAnchor, FxCue } from "../../types.ts";
import { ENTRANCE_TRAUMA, FOOT, heroOf, type CardRecipe, type Entrance } from "../recipe.ts";

/** Every particle count the jungle throws, at intensity "normal" (rule 9). */
const TUNING = {
  land: { count: 20, power: 1 },
  hat: { count: 8, power: 0.7 },
  leaves: { count: 10, power: 0.5 },
  lob: { count: 8, power: 0.8 },
  sink: { count: 14, power: 0.8 },
} as const;

/** Entrance beats, as fractions of D. */
const SWING_AT = 0.05;
const HAT_AT = 0.72;
const LEAVES_AT = [0.74, 0.88] as const;
/** The vine hangs from the board's top edge. */
const VINE: FxAnchor = { kind: "viewport", at: { x: 0.5, y: 0 } };
/** The leaves flutter down from above the zone. */
const CANOPY = { x: 0.5, y: 0 } as const;

const look = paletteLook("Nature") ?? FLAVOR_LOOKS.Nature;

export const vineSwing: Entrance = ({ D, tgt, intensity: i }) => {
  const spec: CueSpec = { look, scale: i, row: "keep" };
  const at = tid(tgt);
  const slam = frac(FX_SLAM_AT, D);
  const swing = frac(SWING_AT, D);
  return [
    motifShot(spec, "bolt", VINE, at, swing, slam - swing),
    motifRing(spec, D, at, slam),
    motifBurst(spec, tid(tgt, FOOT), "area", slam, TUNING.land, "dust"),
    motifBurst(spec, at, "point", frac(HAT_AT, D), TUNING.hat, "sparkle"),
    ...LEAVES_AT.map((f) => motifBurst(spec, tid(tgt, CANOPY), "area", frac(f, D), TUNING.leaves, "leaf")),
    ...shakeCues(i, ENTRANCE_TRAUMA.vineSwing, slam),
  ];
};

/** #75: her pack arcs onto the deck pile. */
export const packLob: CardRecipe = (event, p, play) => {
  if (play.hook !== undefined || event.type !== "shuffledIn") return null;
  const t = timesOf(p.D);
  const spec: CueSpec = { look, scale: p.env.intensity, row: "keep" };
  const from = hitAnchor(p.view, play.instanceId) ?? heroOf(p.view, play);
  const pile = tid(animTestid.library(sideOf(p.view, event.player)));
  const cues: FxCue[] = [
    motifBurst(spec, from, "point", t.G, TUNING.lob, "leaf"),
    { kind: "ghost", from, to: pile, delayMs: t.T, durationMs: t.L - t.T },
    motifRing(spec, p.D, pile, t.L),
    motifBurst(spec, pile, "area", t.L, TUNING.sink, "sparkle"),
  ];
  return { cues, row: "keep" };
};

/** The gallery's Cry: a pack shuffles into the deck. */
export function packLobFixture(_view: PlayerView, _instanceId: string, player: PlayerId): GameEvent[] {
  return [{ type: "shuffledIn", player, instanceId: "fx-explorer-pack", defId: "classicplus-075-1", position: 0 }];
}
