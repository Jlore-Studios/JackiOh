// R3407000: Stinkyscape #152 Sanctum (`stinkyscape-152`, entrance `sanctumRise`, recipe `sanctumEcho`, zone
// aura `pages` in Holy), a Legendary Field Spell's signature (#3407, stinkyscape-2.json5), in silver light
// (Holy's motes in the steel sub-look's colors).
//
// - Its entrance: a rose-window porthole of pale silver light opens in its zone (a ring, holy rays and a
//   ring of motes like the window's tracery); at the slam the hollowed-asteroid library rises through it
//   (crumbs of rock from the zone's foot, a light shake) and its shelves unfold (a sheen on the zone); its
//   reflecting pool ripples twice (two rings at the zone's foot).
// - Its recipe, on its own landing `summoned` beside the entrance: Flashback lights for every card in its
//   owner's graveyard and deck (a silver ring on each pile), and a silver echo rises from each pile to its
//   zone (two bolts), landing in a burst of motes as the library settles. Its Radiant Cry's `manaChanged`
//   rings its owner's mana pile in silver, over the mana row. The gallery fixture is that Cry's mana.
//
// What moved from the issue's concept: each later Flashback cast lifting its card from the graveyard is that
// card's own play (Sanctum's recipe is not asked there), so the shared Flashback beat draws it, not this one.
//
// R202: a graveyard, a deck and the mana are only ever their piles (`pileOf`); no cue names a card, and
// nothing reads a card in a pile. Every beat starts inside its entry and lands within FX_MAX_TAIL_MS of its
// end (R200).

import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";

import { leadMs } from "../../../game/animations.ts";
import { sideOf, testid } from "../../../game/contract.ts";
import { frac, raysCue, shakeCues, tid } from "../../build.ts";
import { motifBurst, motifRing, motifShot, type CueSpec } from "../../motifs/build.ts";
import type { FxCue } from "../../types.ts";
import { ENTRANCE_TRAUMA, FOOT, type CardRecipe, type Entrance } from "../recipe.ts";
import { entranceSpec, entranceTimes, lookOf, pileOf, sigSpec } from "./signature.ts";

/** Every particle count the Sanctum throws at scale 1 (rule 9). */
const TUNING = {
  window: { count: 16, power: 0.6 },
  rise: { count: 18, power: 0.9 },
  echo: { count: 12, power: 0.6 },
} as const;

/** Each beat's moments as fractions of its entry's D (the entrance's and the echo's of the motion after its hold). */
const TIMING = {
  sanctumRise: { portholeAt: 0, windowAt: 0.2, rippleAt: 0.75, rippleStep: 0.12 },
  sanctumEcho: { lightAt: 0, echoAt: 0.5, echoLand: 0.9, manaAt: 0.1 },
} as const;

/** The palettes: Holy's motes, in the steel sub-look's silver. */
const HOLY = "Holy";
const SILVER = "Physical/steel";

/** `spec` in silver: Holy's particle in the steel colors. */
function silver(spec: CueSpec): CueSpec {
  return { ...spec, look: { ...lookOf(HOLY), colors: lookOf(SILVER).colors } };
}

export const sanctumRise: Entrance = (p) => {
  const spec = silver(entranceSpec(p, HOLY));
  const { at, slam } = entranceTimes(p);
  const T = TIMING.sanctumRise;
  const zone = tid(p.tgt);
  const pool = tid(p.tgt, FOOT);
  const open = at(T.portholeAt);
  return [
    motifRing(spec, p.D, zone, open),
    raysCue(p.D, "holy", zone, open),
    motifBurst(spec, zone, "ring", at(T.windowAt), TUNING.window),
    motifBurst(spec, pool, "area", slam, TUNING.rise, "crumb"),
    { kind: "sheen", at: zone, delayMs: slam, durationMs: p.D - slam },
    motifRing(spec, p.D, pool, at(T.rippleAt)),
    motifRing(spec, p.D, pool, at(T.rippleAt + T.rippleStep)),
    ...shakeCues(p.intensity, ENTRANCE_TRAUMA.sanctumRise, slam),
  ];
};

export const sanctumEcho: CardRecipe = (event, p, play) => {
  if (play.hook !== undefined) return null;
  const T = TIMING.sanctumEcho;
  if (event.type === "summoned" && event.instanceId === play.instanceId) {
    const spec = silver(sigSpec(p, play, HOLY));
    const zone = testid.zone(sideOf(p.view, event.player), event.row, event.lane);
    // After any hold its entry opens with, as the entrance it lands with times its own.
    const { at } = entranceTimes({ D: p.D, tgt: zone, intensity: p.env.intensity, wait: leadMs(p.entry) });
    const leave = at(T.echoAt);
    const land = at(T.echoLand);
    const piles = [pileOf(p.view, event.player, "graveyard"), pileOf(p.view, event.player, "library")];
    const cues: FxCue[] = piles.flatMap((pile) => [motifRing(spec, p.D, pile, at(T.lightAt)), motifShot(spec, "bolt", pile, tid(zone), leave, land - leave)]);
    cues.push(motifBurst(spec, tid(zone), "point", land, TUNING.echo));
    return { cues, row: "keep" };
  }
  if (event.type === "manaChanged" && event.player === play.player) {
    return { cues: [motifRing(silver(sigSpec(p, play, HOLY)), p.D, pileOf(p.view, event.player, "mana"), frac(T.manaAt, p.D))], row: "keep" };
  }
  return null;
};

/**
 * R3407000: the Radiant Sanctum's Cry: its controller gains mana (§6.3). The beat reads no number, so the
 * event reports the mana the view shows.
 */
export function sanctumEchoFixture(view: PlayerView, _instanceId: string, player: PlayerId, _defId?: string): GameEvent[] {
  const { mana } = player === view.viewer ? view.you : view.opponent;
  return [{ type: "manaChanged", player, current: mana.current, max: mana.max }];
}
