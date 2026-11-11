// R3407000: Stinkyscape #148 Alteran Reach (`stinkyscape-148`, a Mythic Field Spell: entrance "alteranDive",
// recipe "starscapeJump", zone aura "starpath" in Tech/starfield, takeover "alteran") and the card its
// twentieth node adds, #148.1 Edge of the Universe (`stinkyscape-148-1`, a token Spell printed Mythic:
// recipe "edgeFlee", takeover "edge"), each a signature. takeover.ts draws each takeover with its card's own
// picture filling the screen (twenty island-stations stepping into the deep; the starfield's frayed edge).
//
// - Alteran Reach's entrance, after its takeover's hold: the camera rushes along the path, a sheen across the
//   board and three island-stations flaring in turn on the way down, a star streak diving from the board's
//   top edge into its zone; at the slam its star map opens there in violet (a ring, a ring of stars, a sheen
//   on the zone) and the board shakes.
// - Its Activate: each `starscapeTravelled` jumps a small ship one node along the zone's path (an exhaust
//   bolt from its left to its right), a souvenir sparkle at the stop; at the edge (`reward: "edge"`) the
//   path reaches a bright cliff, holy rays over the zone; the Edge of the Universe it adds flies out of the
//   zone as a card back to that player's hand pile (`addedToHand`), in place of the hand's own row.
// - Edge of the Universe's cast, as its takeover's hold ends: the blank white light at the edge (a holy ring
//   and a burst of light) on its caster's hero, the other side's hero greying in smoke. Its `fled`: the board
//   peels in from its edges (`walls`), a sheen on the caster's hero as they walk off into the white, and a
//   ring of smoke greys the other hero.
//
// Moved from the issue's concept: the path is the aura (`starpath`), the ship's jump one bolt across the zone
// (no figure draws a ship); the paper curl is the takeover's sweep (fx.css `fx-takeover-edge`) and the walls,
// and the opponent's side greying to a draw is smoke at their hero: the `gameOver` after it is the game's.
// The light is a ring and a burst, not rays: rays outlast any entry by FX_RAYS_TAIL_MS, past the Mythic
// Spell's ceiling from its cast's held entry. The fleeing follows the cast's entry and the fleeing's own
// moment outlasts the ceiling from the cast, so the gallery plays the cast alone and the test proves the
// `fled` beat from its own entry (R3332000).
//
// R202: no cue names a card. The card the edge adds is a back between the zone and a pile, its `addedToHand`
// read for its `player` alone ("hidden" on the other seat); a fleeing is public. Each beat starts inside its
// entry and ends within FX_MAX_TAIL_MS of it (R200).

import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";

import { locateInstance } from "../../../game/animations.ts";
import { sideOf, testid } from "../../../game/contract.ts";
import { FX_WALLS_REACH } from "../../constants.ts";
import { frac, raysCue, ringCue, shakeCues, tid } from "../../build.ts";
import { motifBurst, motifRing, motifShot } from "../../motifs/build.ts";
import type { FxCue, FxPlay, FxPoint } from "../../types.ts";
import { ENTRANCE_TRAUMA, FOOT, type CardRecipe, type Entrance } from "../recipe.ts";
import { backTo, entranceSpec, entranceTimes, otherPlayer, pileOf, sigSpec } from "./signature.ts";

const ALTERAN_REACH = "stinkyscape-148";

/** The palettes: the deep's starfield, the edge's white light, the greying smoke. */
const STARFIELD = "Tech/starfield";
const LIGHT = "Holy";
const GREY = "Dark/shadow";

/** Every particle count the two throw at scale 1 (rule 9). */
const TUNING = {
  station: { count: 6, power: 0.5 },
  map: { count: 22, power: 0.9 },
  sparkle: { count: 8, power: 0.5 },
  grey: { count: 14, power: 0.5 },
  light: { count: 20, power: 0.8 },
  /** The edge's light shake on its cast, within SIGNATURE_SPELL_MAX_TRAUMA. */
  edgeTrauma: 0.3,
} as const;

/** Each beat's moments as fractions of its entry's D (the entrance's of its motion after the hold). */
const TIMING = {
  alteranDive: { sweepAt: 0, stationAt: 0.05, stationStep: 0.1, diveAt: 0.2 },
  starscapeJump: { jumpAt: 0, jumpFor: 0.5, raysAt: 0.3, sendAt: 0, sendFor: 0.8 },
  edgeFlee: { greyAt: 0.1, wallsFor: 1, sheenAt: 0.2 },
} as const;

/** The island-stations the camera passes on its way down, on the board. */
const STATIONS: readonly FxPoint[] = [
  { x: 0.3, y: 0.15 },
  { x: 0.6, y: 0.3 },
  { x: 0.45, y: 0.45 },
];
/** Where the dive starts: the board's top edge. */
const SKY: FxPoint = { x: 0.5, y: 0 };
/** The ship's path across the zone, its two ends. */
const PATH_FROM: FxPoint = { x: 0.2, y: 0.5 };
const PATH_TO: FxPoint = { x: 0.8, y: 0.5 };

/** The testid of the zone or card `play`'s Field Spell stands in, else its controller's backrow. */
function standing(view: PlayerView, play: FxPlay): string {
  return locateInstance(view, play.instanceId) ?? `backrow-${sideOf(view, play.player)}`;
}

/* Alteran Reach */

export const alteranDive: Entrance = (p) => {
  const spec = entranceSpec(p, STARFIELD);
  const { at, slam } = entranceTimes(p);
  const T = TIMING.alteranDive;
  const zone = tid(p.tgt);
  const dive = at(T.diveAt);
  return [
    { kind: "sheen", at: tid(testid.board), delayMs: at(T.sweepAt), durationMs: Math.max(0, slam - at(T.sweepAt)) },
    ...STATIONS.map((station, k) => motifBurst(spec, tid(testid.board, station), "point", at(T.stationAt + k * T.stationStep), TUNING.station, "star")),
    motifShot(spec, "bolt", tid(testid.board, SKY), zone, dive, Math.max(0, slam - dive)),
    motifRing(spec, p.D, zone, slam),
    motifBurst(spec, zone, "ring", slam, TUNING.map, "star"),
    motifBurst(spec, tid(p.tgt, FOOT), "area", slam, TUNING.map),
    { kind: "sheen", at: zone, delayMs: slam, durationMs: p.D - slam },
    ...shakeCues(p.intensity, ENTRANCE_TRAUMA.alteranDive, slam),
  ];
};

export const starscapeJump: CardRecipe = (event, p, play) => {
  if (play.hook !== "activate") return null;
  const T = TIMING.starscapeJump;
  const zone = standing(p.view, play);
  if (event.type === "starscapeTravelled" && event.player === play.player) {
    const spec = sigSpec(p, play, STARFIELD);
    const jumpAt = frac(T.jumpAt, p.D);
    const landAt = jumpAt + frac(T.jumpFor, p.D);
    const cues: FxCue[] = [
      motifShot(spec, "bolt", tid(zone, PATH_FROM), tid(zone, PATH_TO), jumpAt, landAt - jumpAt),
      motifBurst(spec, tid(zone, PATH_TO), "point", landAt, TUNING.sparkle, "sparkle"),
    ];
    if (event.reward === "edge") cues.push(raysCue(p.D, "holy", tid(zone), frac(T.raysAt, p.D)));
    return { cues, row: "keep" };
  }
  if (event.type === "addedToHand") {
    // R202: the seat it goes to is public; the card is not read.
    const delayMs = frac(T.sendAt, p.D);
    return { cues: [backTo(tid(zone), pileOf(p.view, event.player, "hand"), delayMs, frac(T.sendFor, p.D))], row: "replace" };
  }
  return null;
};

/**
 * R3407000: one Activate of Alteran Reach as the gallery plays it: its marker and the travel to the edge
 * (the twentieth node); the Edge of the Universe it adds is proven by its test from that marker.
 */
export function starscapeJumpFixture(_view: PlayerView, instanceId: string, player: PlayerId, defId = ALTERAN_REACH): GameEvent[] {
  return [
    { type: "hookResolving", sourceId: instanceId, defId, controller: player, hook: "activate" },
    { type: "starscapeTravelled", player, node: 20, reward: "edge" },
  ];
}

/* Edge of the Universe */

export const edgeFlee: CardRecipe = (event, p, play) => {
  const T = TIMING.edgeFlee;
  const hero = tid(testid.hero(sideOf(p.view, play.player)));
  const other = tid(testid.hero(sideOf(p.view, otherPlayer(play.player))));
  const grey = sigSpec(p, play, GREY);
  if (event.type === "cardPlayed" && event.instanceId === play.instanceId) {
    // The white light as its takeover's hold ends.
    const hold = Math.min(p.D, p.entry.setPiece?.holdMs ?? 0);
    const light = sigSpec(p, play, LIGHT);
    return {
      cues: [
        motifRing(light, p.D, hero, hold),
        motifBurst(light, hero, "area", hold, TUNING.light),
        motifBurst(grey, other, "area", hold + frac(T.greyAt, p.D - hold), TUNING.grey, "smoke"),
        ...shakeCues(p.env.intensity, TUNING.edgeTrauma, hold),
      ],
      row: "replace",
    };
  }
  if (event.type === "fled" && event.player === play.player) {
    const sheenAt = frac(T.sheenAt, p.D);
    return {
      cues: [
        { kind: "walls", at: tid(testid.board), reach: FX_WALLS_REACH, delayMs: 0, durationMs: frac(T.wallsFor, p.D) },
        { kind: "sheen", at: hero, delayMs: sheenAt, durationMs: p.D - sheenAt },
        ringCue(p.D, "smoke", other, sheenAt),
      ],
      row: "replace",
    };
  }
  return null;
};

/**
 * R3407000: Edge of the Universe's play as the gallery plays it: its cast alone. Its caster's fleeing
 * (`fled`), which follows, is proven by its test from its own entry.
 */
export function edgeFleeFixture(_view: PlayerView, _instanceId: string, _player: PlayerId, _defId?: string): GameEvent[] {
  return [];
}
