// R3400001: Meditative+ #24 Bob, Wisconsin's Strongest Soldier (`meditativeplus-024`, entrance
// "snowplowHeave", bespoke "bobSoldier", a snow flurry on his tile), a Legendary Unit's signature. He drops
// into his zone in a burst of snow, heaves a snowplow's outline overhead and slams it down at the slam,
// frost cracking out across his lane into the enemy's zone. His Cry then plays the mode it ran: frost
// sweeps every enemy Unit the board shows at the first freeze; a big mitten plucks the stolen Unit and its
// card back flies to his hand; the other player's Coins fly from his hero; a Discovered card flies in, a
// sheen on the hand pile for its owner alone (its Radiance is theirs to read); the drawn card's copies fan
// in. It keys on his play and what that play has seen (`seen`), never on a card's definition, and draws
// card backs alone (R202).

import type { GameEvent, PlayerView } from "@jackioh/shared";

import { animTestid, locateInstance } from "../../../game/animations.ts";
import { sideOf, testid, type Side } from "../../../game/contract.ts";
import { FROZEN_MARK } from "../../../stats/config.ts";
import { frac, ringCue, shakeCues, tid, tunedBurst } from "../../build.ts";
import { FX_SLAM_AT } from "../../constants.ts";
import { figureCue, figureTint } from "../../figures.ts";
import { FLAVOR_LOOKS, paletteLook } from "../../flavors.ts";
import { otherSide } from "../../motifs/build.ts";
import type { FxCue, FxGhostCue } from "../../types.ts";
import { ENTRANCE_TRAUMA, FOOT, heroOf, type CardRecipe, type Entrance } from "../recipe.ts";

/** The look his snow wears: Physical's wind sub-look. */
const BOB_PALETTE = "Physical/wind";

/** Every particle count his signature throws, at intensity "normal" (rule 9). */
const TUNING = {
  drop: { count: 22, power: 1 },
  frost: { count: 18, power: 1.1 },
  sweep: { count: 12, power: 0.9 },
} as const;

/** The plow lasts its entry and this much more; the copies fan in this share of the entry apart, at most `fanMax` in. */
const TIMING = { plowTailMs: 300, fanStep: 0.15, fanMax: 0.5 } as const;

/** The unit zones of `side` that show a Unit, in lane order, by testid (a face-down one is shown as its back). */
function unitZones(view: PlayerView, side: Side): string[] {
  return view[side].units.flatMap((unit, k) => (unit === null ? [] : [testid.zone(side, "units", k + 1)]));
}

export const snowplowHeave: Entrance = ({ D, tgt, intensity: i, view, landing }) => {
  const look = paletteLook(BOB_PALETTE) ?? FLAVOR_LOOKS.Physical;
  const at = tid(tgt);
  const slam = frac(FX_SLAM_AT, D);
  const cues: FxCue[] = [
    tunedBurst(i, "frost", tid(tgt, FOOT), "area", 0, TUNING.drop),
    figureCue("plow", at, figureTint(look), 0, D + TIMING.plowTailMs, view === undefined || landing === undefined ? {} : { side: sideOf(view, landing.player) }),
    ringCue(D, "frost", at, slam),
  ];
  // The frost cracks out across his lane into the enemy's zone.
  if (view !== undefined && landing !== undefined) {
    cues.push(tunedBurst(i, "frost", tid(testid.zone(otherSide(sideOf(view, landing.player)), "units", landing.lane)), "ring", slam, TUNING.frost));
  }
  cues.push(...shakeCues(i, ENTRANCE_TRAUMA.snowplowHeave, slam));
  return cues;
};

function frozen(event: GameEvent): boolean {
  return event.type === "marked" && event.mark === FROZEN_MARK && event.added;
}

export const bobSoldier: CardRecipe = (event, p, play) => {
  const look = paletteLook(BOB_PALETTE) ?? FLAVOR_LOOKS.Physical;
  const caster = sideOf(p.view, play.player);
  const hand = (side: Side) => tid(animTestid.hand(side));
  const ghost = (from: FxGhostCue["from"], to: FxGhostCue["to"], delayMs = 0): FxGhostCue => ({ kind: "ghost", from, to, delayMs, durationMs: p.D - delayMs });
  const seen = play.seen;
  if (frozen(event)) {
    // The first freeze of the play sweeps frost over every enemy Unit; the rest of its marks draw nothing more.
    const own = p.entry.events.filter(frozen).length;
    const first = (seen.marked ?? 0) - own === 0 && p.entry.events.find(frozen) === event;
    if (!first) return { cues: [], row: "replace" };
    return { cues: unitZones(p.view, otherSide(caster)).map((zone) => tunedBurst(p.env.intensity, "frost", tid(zone), "area", 0, TUNING.sweep)), row: "replace" };
  }
  if (event.type === "stolen" && event.zone === "field" && event.to === play.player) {
    const unit = locateInstance(p.view, event.instanceId);
    if (unit === null) return { cues: [ghost(tid(testid.hero(otherSide(caster))), hand(caster))], row: "replace" };
    return { cues: [figureCue("mitten", tid(unit), figureTint(look), 0, p.D, { side: caster }), ghost(tid(unit), hand(caster))], row: "replace" };
  }
  if (event.type !== "addedToHand") return null;
  // The other player's Coins, from his hero.
  if (event.player !== play.player) return { cues: [ghost(heroOf(p.view, play), hand(sideOf(p.view, event.player)))], row: "replace" };
  const nth = seen.addedToHand ?? 1;
  // The stolen Unit's own card, which its mitten already flew.
  if ((seen.stolen ?? 0) > 0 && nth === 1) return { cues: [], row: "replace" };
  const from = heroOf(p.view, play);
  if ((seen.drawn ?? 0) > 0) {
    const delayMs = Math.min(frac((nth - 1) * TIMING.fanStep, p.D), frac(TIMING.fanMax, p.D));
    return { cues: [ghost(tid(animTestid.library(caster)), hand(caster), delayMs)], row: "replace" };
  }
  if ((seen.promptAnswered ?? 0) > 0) {
    // Its Radiance is the owner's to read: the sheen plays on their seat alone.
    const sheen: FxCue[] = p.view.viewer === play.player ? [{ kind: "sheen", at: hand(caster), delayMs: 0, durationMs: p.D }] : [];
    return { cues: [ghost(from, hand(caster)), ...sheen], row: "replace" };
  }
  return { cues: [ghost(from, hand(caster))], row: "replace" };
};
