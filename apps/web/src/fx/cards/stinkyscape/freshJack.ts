// R3407000: Stinkyscape #109 xxFreshJack (`stinkyscape-109`, a Mythic Unit): entrance "freshJackVault",
// bespoke "freshJackComet", tile aura "pixelcrawl" in Tech/glitch, takeover "freshJack" (takeover.ts draws
// the screen glitching into his own art, the cracked warp gate over a spaceport, teal and magenta).
//
// - Entrance (after the takeover's hold): a spray of teal and magenta pixels where the art filled the
//   screen, he vaults out of it as a glitch bolt onto his zone, and at the slam he skids in a trail of pixel
//   bursts along its foot, a glitch ring and a sheen on the zone and a shake.
// - Cry: the first card his Cry announces (the BUSTED card his base face casts, or the one his Radiant face
//   Discovered) tears the gate open again on his card (his hero when the board does not show him): a glitch
//   ring and burst there, and an overcharged comet (a fire projectile) hurled to the board's centre, which
//   bursts there in fire and pixels, so the BUSTED card's own entry plays out of it. The announce's own row
//   still plays.
//
// Moved from the issue's concept: the tile's pixel shimmer is the aura (`pixelcrawl`, its table entry), not a
// cue here; a tear figure is a paper prop (fx.css), so the gate is a ring and a burst. The comet keys on the
// announce while his play is the innermost one: in a live game his Cry's cast arrives as one cast entry with
// the cast card's own `cardPlayed` (game/animations.ts), whose play then is the innermost, so the beat draws
// where the announce stands alone (the gallery's fixture) until cardFx.ts asks the casting play too.
//
// R202: no cue names a card. The announced card's id and definition are never read: the comet flies from his
// own card or hero to the centre. The Radiant face's Discover options are the owner's alone: `promptOpened`
// is dropped on the other seat and draws nothing here, on either seat. Each beat starts inside its entry and
// ends within FX_MAX_TAIL_MS of it (R200).

import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";

import { locateInstance } from "../../../game/animations.ts";
import { sideOf, testid } from "../../../game/contract.ts";
import { frac, shakeCues, tid } from "../../build.ts";
import { FX_CENTER, FX_SLAM_AT } from "../../constants.ts";
import { motifBurst, motifRing, motifShot } from "../../motifs/build.ts";
import type { FxAnchor, FxCue, FxPlay, FxPoint } from "../../types.ts";
import { ENTRANCE_TRAUMA, FOOT, projectile, type CardRecipe, type Entrance } from "../recipe.ts";
import { entranceSpec, entranceTimes, sigSpec } from "./signature.ts";

/** His palette: Tech's glitch, white, magenta and teal. */
const GLITCH = "Tech/glitch";

/** Every particle count his signature throws at scale 1 (rule 9). */
const TUNING = {
  spray: { count: 16, power: 1.1 },
  skid: { count: 8, power: 0.7 },
  gate: { count: 14, power: 1 },
  impact: { count: 20, power: 1.2 },
} as const;

/** Each beat's moments as fractions of its entry's D (the entrance's of its motion after the hold). */
const TIMING = {
  freshJackVault: { sprayAt: 0, vaultAt: 0.05, skidStep: 0.06 },
  freshJackComet: { gateAt: 0, launchAt: 0.1, landAt: 0.6 },
} as const;

/** The skid's pixel bursts along the foot of his zone, in order. */
const SKID: readonly FxPoint[] = [
  { x: 0.2, y: FOOT.y },
  { x: FOOT.x, y: FOOT.y },
  { x: 0.8, y: FOOT.y },
];

/** The board's centre, where the comet lands and the cast card's entry plays out of it. */
const CENTRE: FxAnchor = tid(testid.board, FX_CENTER);

/** His card where the board shows it (never a hand card), else his controller's hero. */
function origin(view: PlayerView, play: FxPlay): FxAnchor {
  const at = locateInstance(view, play.instanceId);
  return tid(at !== null && at === testid.card(play.instanceId) ? at : testid.hero(sideOf(view, play.player)));
}

export const freshJackVault: Entrance = (p) => {
  const spec = entranceSpec(p, GLITCH);
  const { at, slam } = entranceTimes(p);
  const T = TIMING.freshJackVault;
  const zone = tid(p.tgt);
  const vault = at(T.vaultAt);
  return [
    motifBurst(spec, CENTRE, "ring", at(T.sprayAt), TUNING.spray),
    motifShot(spec, "bolt", CENTRE, zone, vault, Math.max(0, slam - vault)),
    ...SKID.map((point, k) => motifBurst(spec, tid(p.tgt, point), "point", at(FX_SLAM_AT + k * T.skidStep), TUNING.skid)),
    motifRing(spec, p.D, zone, slam),
    { kind: "sheen", at: zone, delayMs: slam, durationMs: p.D - slam },
    ...shakeCues(p.intensity, ENTRANCE_TRAUMA.freshJackVault, slam),
  ];
};

/** Whether `event` is the first card his play announces (his own announce comes before his play opens). */
function firstCast(event: GameEvent, play: FxPlay): boolean {
  return event.type === "cardAnnounced" && event.instanceId !== play.instanceId && play.hook === undefined && (play.seen.cardAnnounced ?? 0) === 1;
}

export const freshJackComet: CardRecipe = (event, p, play) => {
  if (!firstCast(event, play)) return null;
  const spec = sigSpec(p, play, GLITCH);
  const T = TIMING.freshJackComet;
  const from = origin(p.view, play);
  const launch = frac(T.launchAt, p.D);
  const land = frac(T.landAt, p.D);
  const gate = frac(T.gateAt, p.D);
  const cues: FxCue[] = [
    motifRing(spec, p.D, from, gate),
    motifBurst(spec, from, "ring", gate, TUNING.gate),
    projectile(spec.scale, "fire", from, CENTRE, launch, land - launch),
    motifBurst(spec, CENTRE, "area", land, TUNING.impact, "fire"),
    motifBurst(spec, CENTRE, "ring", land, TUNING.impact),
  ];
  return { cues, row: "keep" };
};

/**
 * R3407000: his Cry's cast, as the gallery plays it: the card it casts announced (K-Pop Fanatic, a BUSTED
 * card, stands in for any), free. Alone, as the gallery has it, the announce is its own entry.
 */
export function freshJackCometFixture(_view: PlayerView, _instanceId: string, player: PlayerId, _defId?: string): GameEvent[] {
  return [{ type: "cardAnnounced", player, instanceId: "fx-busted", defId: "core-050", cardType: "Unit", costPaid: 0, targets: [] }];
}
