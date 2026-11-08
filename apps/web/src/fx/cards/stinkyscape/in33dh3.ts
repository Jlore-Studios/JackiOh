// R3407000: IN33DH3 and the two tokens his line leaves (#3407), each a Legendary signature (stinkyscape-2.json5).
//
// - IN33DH3 (`stinkyscape-147`, entrance `in33dh3Drop`, recipe `dataPacket`, tile aura `aiwisp` in
//   Tech/hologram): a headset crackles at the top of his zone and a translucent teal AI companion (the
//   `spirit` figure, hologram-tinted) materialises first and points a thin beam down at the zone; he drops
//   in at the slam (dust at his feet, a ring, a light shake) and cracks his knuckles, two sparks. His Death
//   opens its hook entry: at the marker he dissolves in teal circuit motes where he lies (his card while
//   the view shows it, else his graveyard pile), and on the `shuffledIn` the Prime goes deckward as a teal
//   data packet, a circuit bolt beside a card back from there to the library pile, a ring as it lands; it
//   draws in place of the shuffle row's own card back. The gallery fixture is that Death.
// - IN33DH3 Prime (`stinkyscape-147-1`, entrance `primeCrown`, recipe `primeConduct`, tile aura
//   `crownpulse`): a black command coat sweeps through his zone in smoke, a neural crown lights on his
//   brow (a ring and a ring of motes at the top of the zone), the hologram companion kneels at his side
//   (the `spirit` figure, small, at the zone's corner) and he lands at the slam. His Cry: as the Wardrum
//   Blob it summons lands in his side's backrow, he raises a hand like a conductor (motes on his card),
//   rays light his backrow, a ring lands on the Blob's zone and a hologram beam hums from him to it; the
//   Blob's own entrance (`blobPlop`) draws beside it on that `summoned`. The gallery fixture is that summon.
// - Wardrum Blob (`stinkyscape-147-2`, entrance `blobPlop`, recipe `blobDrum`, zone aura `drumbeat`): a
//   teal drip falls into its zone, the blob plops down at the slam (a splat at its foot and a ring), wobbles
//   twice and settles on its pedestal (a sheen on the zone). Its Start of turn: the marker drums once on its
//   zone (a ring and a burst), and the helpful cast's announce drums a second time and runs a teal ripple
//   from it to the first target the announce names that the board shows (a Unit, a face-up card or a hero),
//   else to its controller's hero; the cast's own rows play. The gallery fixture is that turn's hook.
//
// What moved from the issue's concept: the companion lighting the Pareto-optimal plays (IN33DH3's Cry) and
// the Blob's aura doing the same are owner-only state with no event (R1844001: the hand marks them on its
// owner's view), so neither is drawn. IN33DH3's dissolve plays at his Death's marker, not on `destroyed`:
// that event comes before the hook opens (in combat, inside no play), so no recipe of his is asked then.
// The Blob's two drums fall on two entries (its marker's lasts 0 ms, so it cannot hold two beats apart).
//
// R202: no cue names a card. A deck or a graveyard is only ever its pile (`pileOf`), the Prime going in is a
// card back, and the `shuffledIn` is read for its player alone (its ids are hidden on the other seat). A
// card is drawn only where the view shows it (`locateInstance`, a board card, never a hand card). Every beat
// starts inside its entry and lands within FX_MAX_TAIL_MS of its end (R200).

import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";

import { HIDDEN_ID, animTestid, locateInstance } from "../../../game/animations.ts";
import { sideOf, testid } from "../../../game/contract.ts";
import { frac, raysCue, shakeCues, tid } from "../../build.ts";
import { figureCue, figureTint } from "../../figures.ts";
import { hitAnchor, motifBurst, motifRing, motifShot, type CueSpec } from "../../motifs/build.ts";
import type { FxAnchor, FxCue, FxPlay, FxPoint } from "../../types.ts";
import { ENTRANCE_TRAUMA, FOOT, heroOf, type CardFxPlan, type CardRecipe, type Entrance } from "../recipe.ts";
import { backTo, entranceSpec, entranceTimes, otherPlayer, pileOf, sigSpec } from "./signature.ts";

/** Every particle count the three throw at scale 1 (rule 9). */
const TUNING = {
  crackle: { count: 8, power: 0.6 },
  drop: { count: 20, power: 1 },
  knuckle: { count: 4, power: 0.5 },
  coat: { count: 18, power: 0.8 },
  crown: { count: 12, power: 0.6 },
  drip: { count: 6, power: 0.4 },
  splat: { count: 18, power: 0.9 },
  wobble: { count: 6, power: 0.4 },
  dissolve: { count: 16, power: 0.8 },
  packet: { count: 10, power: 0.7 },
  conduct: { count: 8, power: 0.5 },
  drum: { count: 10, power: 0.7 },
} as const;

/** Each beat's moments as fractions of its entry's D (an entrance's of its motion after its hold); figures linger `tailMs` past it. */
const TIMING = {
  in33dh3Drop: { crackleAt: 0, companionAt: 0.1, pointAt: 0.3, knuckleAt: 0.75, knuckleStep: 0.12, tailMs: 300 },
  primeCrown: { coatAt: 0, crownAt: 0.3, kneelAt: 0.4, tailMs: 300 },
  blobPlop: { dripAt: 0.1, wobbleAt: 0.75, wobbleStep: 0.12 },
  dataPacket: { leaveAt: 0.1, landAt: 0.65 },
  primeConduct: { raiseAt: 0, raysAt: 0.1, humAt: 0.2, humLand: 0.6 },
  blobDrum: { secondAt: 0.3, rippleLand: 0.75 },
} as const;

/** The palettes: the hologram teal of the AI, and the black of the Prime's command coat. */
const HOLOGRAM = "Tech/hologram";
const COAT = "Dark/shadow";

/** Where in a zone: its top (the headset), his brow, his hands, and the corner the companion kneels in. */
const TOP: FxPoint = { x: 0.5, y: 0 };
const BROW: FxPoint = { x: 0.5, y: 0.2 };
const HANDS: FxPoint = { x: 0.5, y: 0.6 };
const KNEEL: FxPoint = { x: 0.85, y: 0.75 };

/** The cards the fixtures name (events only, never a cue: R202). */
const IN33DH3 = "stinkyscape-147";
const PRIME = "stinkyscape-147-1";
const WARDRUM_BLOB = "stinkyscape-147-2";
/** The helpful cast the Blob's fixture announces: Hit Job, at the first enemy Unit the view shows. */
const HELPFUL_CAST = "core-016";
/** Where the fixture's Prime goes into the deck, and the backrow lane its Blob lands in when the view has no open zone. */
const FIXTURE_DECK_POSITION = 0;
const FIXTURE_BLOB_LANE = 2;
/** A cast's price: free (R70). */
const CAST_COST = 0;

/** The look's circuit motes: the data packet's, in the hologram's teal. */
function circuit(spec: CueSpec): CueSpec {
  return { ...spec, look: { ...spec.look, particle: "circuit" } };
}

/** A board card's testid (`card-…`) for `instanceId` when the view shows it there, else null (never a hand card: R202). */
function boardCard(view: PlayerView, instanceId: string): FxAnchor | null {
  if (instanceId === HIDDEN_ID) return null;
  const at = locateInstance(view, instanceId);
  return at !== null && at.startsWith("card-") ? tid(at) : null;
}

/**
 * R3407000: where the play's own card stands: its card while the view shows it, else the zone its own
 * `summoned` landed it in (in the planner's memory), else `fallback`.
 */
function standing(p: CardFxPlan, play: FxPlay, fallback: FxAnchor): FxAnchor {
  const card = boardCard(p.view, play.instanceId);
  if (card !== null) return card;
  const landed = [...p.env.memory.recent()].reverse().find((event) => event.type === "summoned" && event.instanceId === play.instanceId);
  return landed?.type === "summoned" ? tid(testid.zone(sideOf(p.view, landed.player), landed.row, landed.lane)) : fallback;
}

/* IN33DH3 */

export const in33dh3Drop: Entrance = (p) => {
  const spec = entranceSpec(p, HOLOGRAM);
  const { at, slam } = entranceTimes(p);
  const T = TIMING.in33dh3Drop;
  const zone = tid(p.tgt);
  const side = p.view === undefined || p.landing === undefined ? {} : { side: sideOf(p.view, p.landing.player) };
  const appear = at(T.companionAt);
  const point = at(T.pointAt);
  return [
    motifBurst(spec, tid(p.tgt, TOP), "point", at(T.crackleAt), TUNING.crackle, "spark"),
    figureCue("spirit", zone, figureTint(spec.look), appear, p.D - appear + T.tailMs, side),
    motifShot(spec, "beam", tid(p.tgt, BROW), tid(p.tgt, FOOT), point, Math.max(0, slam - point)),
    motifBurst(spec, tid(p.tgt, FOOT), "area", slam, TUNING.drop, "dust"),
    motifRing(spec, p.D, zone, slam),
    motifBurst(spec, tid(p.tgt, HANDS), "point", at(T.knuckleAt), TUNING.knuckle, "spark"),
    motifBurst(spec, tid(p.tgt, HANDS), "point", at(T.knuckleAt + T.knuckleStep), TUNING.knuckle, "spark"),
    ...shakeCues(p.intensity, ENTRANCE_TRAUMA.in33dh3Drop, slam),
  ];
};

export const dataPacket: CardRecipe = (event, p, play) => {
  if (play.hook !== "death") return null;
  const spec = circuit(sigSpec(p, play, HOLOGRAM));
  // Where he lies: his card while the view still shows it, else his graveyard pile.
  const from = boardCard(p.view, play.instanceId) ?? pileOf(p.view, play.player, "graveyard");
  if (event.type === "hookResolving" && event.sourceId === play.instanceId) {
    return { cues: [motifBurst(spec, from, "area", 0, TUNING.dissolve)], row: "keep" };
  }
  if (event.type === "shuffledIn") {
    // Its ids are hidden on the other seat: the deck is its player's pile (R202).
    const T = TIMING.dataPacket;
    const deck = pileOf(p.view, event.player, "library");
    const leave = frac(T.leaveAt, p.D);
    const land = frac(T.landAt, p.D);
    return {
      cues: [motifShot(spec, "bolt", from, deck, leave, land - leave), backTo(from, deck, leave, land - leave), motifRing(spec, p.D, deck, land), motifBurst(spec, deck, "point", land, TUNING.packet)],
      row: "replace",
    };
  }
  return null;
};

/* IN33DH3 Prime */

export const primeCrown: Entrance = (p) => {
  const spec = entranceSpec(p, HOLOGRAM);
  const coat = entranceSpec(p, COAT);
  const { at, slam } = entranceTimes(p);
  const T = TIMING.primeCrown;
  const zone = tid(p.tgt);
  const brow = tid(p.tgt, BROW);
  const side = p.view === undefined || p.landing === undefined ? {} : { side: sideOf(p.view, p.landing.player) };
  const kneel = at(T.kneelAt);
  return [
    motifBurst(coat, zone, "area", at(T.coatAt), TUNING.coat),
    motifRing(spec, p.D, brow, at(T.crownAt)),
    motifBurst(spec, brow, "ring", at(T.crownAt), TUNING.crown),
    figureCue("spirit", tid(p.tgt, KNEEL), figureTint(spec.look), kneel, p.D - kneel + T.tailMs, { ...side, size: "small" }),
    motifRing(spec, p.D, zone, slam),
    ...shakeCues(p.intensity, ENTRANCE_TRAUMA.primeCrown, slam),
  ];
};

export const primeConduct: CardRecipe = (event, p, play) => {
  // His Cry's Blob: a backrow summon of his controller's while his own play resolves.
  if (play.hook !== undefined || event.type !== "summoned" || event.row !== "backrow" || event.player !== play.player || event.instanceId === play.instanceId) return null;
  const spec = sigSpec(p, play, HOLOGRAM);
  const T = TIMING.primeConduct;
  const side = sideOf(p.view, event.player);
  const blob = tid(testid.zone(side, event.row, event.lane));
  const him = standing(p, play, heroOf(p.view, play));
  const hum = frac(T.humAt, p.D);
  return {
    cues: [
      motifBurst(spec, him, "point", frac(T.raiseAt, p.D), TUNING.conduct),
      raysCue(p.D, "radiant", tid(animTestid.backrow(side)), frac(T.raysAt, p.D)),
      motifRing(spec, p.D, blob, hum),
      motifShot(spec, "beam", him, blob, hum, frac(T.humLand, p.D) - hum),
    ],
    row: "keep",
  };
};

/* Wardrum Blob */

export const blobPlop: Entrance = (p) => {
  const spec = entranceSpec(p, HOLOGRAM);
  const { at, slam } = entranceTimes(p);
  const T = TIMING.blobPlop;
  const zone = tid(p.tgt);
  return [
    motifBurst(spec, tid(p.tgt, TOP), "point", at(T.dripAt), TUNING.drip),
    motifBurst(spec, tid(p.tgt, FOOT), "area", slam, TUNING.splat),
    motifRing(spec, p.D, zone, slam),
    { kind: "sheen", at: zone, delayMs: slam, durationMs: p.D - slam },
    motifBurst(spec, zone, "ring", at(T.wobbleAt), TUNING.wobble),
    motifBurst(spec, zone, "ring", at(T.wobbleAt + T.wobbleStep), TUNING.wobble),
    ...shakeCues(p.intensity, ENTRANCE_TRAUMA.blobPlop, slam),
  ];
};

export const blobDrum: CardRecipe = (event, p, play) => {
  if (play.hook !== "trigger") return null;
  const spec = sigSpec(p, play, HOLOGRAM);
  const zone = standing(p, play, tid(animTestid.backrow(sideOf(p.view, play.player))));
  if (event.type === "hookResolving" && event.sourceId === play.instanceId) return { cues: drum(spec, p.D, zone, 0), row: "keep" };
  if (event.type === "cardAnnounced" && event.player === play.player) {
    const T = TIMING.blobDrum;
    const first = event.targets[0];
    const shown = first === undefined || first === HIDDEN_ID ? null : hitAnchor(p.view, first);
    const to = shown !== null && shown.kind === "testid" && !shown.testid.startsWith("hand-card-") ? shown : heroOf(p.view, play);
    const beat = frac(T.secondAt, p.D);
    return { cues: [...drum(spec, p.D, zone, beat), motifShot(spec, "beam", zone, to, beat, frac(T.rippleLand, p.D) - beat)], row: "keep" };
  }
  return null;
};

/** One drumbeat on the Blob's zone: a ring and a burst. */
function drum(spec: CueSpec, D: number, zone: FxAnchor, delayMs: number): FxCue[] {
  return [motifRing(spec, D, zone, delayMs), motifBurst(spec, zone, "ring", delayMs, TUNING.drum)];
}

/* The gallery's fixtures */

const marker = (instanceId: string, player: PlayerId, defId: string, hook: "death" | "startOfTurn"): GameEvent => ({ type: "hookResolving", sourceId: instanceId, defId, controller: player, hook });

/** `player`'s side of `view`. */
function sideView(view: PlayerView, player: PlayerId): PlayerView["you"] {
  return player === view.viewer ? view.you : view.opponent;
}

/** R3407000: IN33DH3's Death: its marker, then an IN33DH3 Prime shuffled into its controller's deck (R1421). */
export function dataPacketFixture(_view: PlayerView, instanceId: string, player: PlayerId, defId = IN33DH3): GameEvent[] {
  return [marker(instanceId, player, defId, "death"), { type: "shuffledIn", player, instanceId: "fx-prime", defId: PRIME, position: FIXTURE_DECK_POSITION }];
}

/** R3407000: the Prime's Cry: a Wardrum Blob summoned into its controller's first open backrow zone (R64), else a lane of the showcase's own. */
export function primeConductFixture(view: PlayerView, _instanceId: string, player: PlayerId, _defId?: string): GameEvent[] {
  const open = sideView(view, player).backrow.findIndex((slot) => slot === null);
  return [{ type: "summoned", player, instanceId: "fx-wardrum-blob", defId: WARDRUM_BLOB, row: "backrow", lane: open < 0 ? FIXTURE_BLOB_LANE : open + 1 }];
}

/** R3407000: the Blob's Start of turn: its marker, then the helpful cast announced (R1844002), at the first enemy Unit the view shows. */
export function blobDrumFixture(view: PlayerView, instanceId: string, player: PlayerId, defId = WARDRUM_BLOB): GameEvent[] {
  const foe = sideView(view, otherPlayer(player)).units.find((unit) => unit !== null && unit.defId !== HIDDEN_ID);
  const targets = foe === undefined || foe === null ? [] : [foe.instanceId];
  return [
    marker(instanceId, player, defId, "startOfTurn"),
    { type: "cardAnnounced", player, instanceId: "fx-helpful", defId: HELPFUL_CAST, cardType: "Spell", costPaid: CAST_COST, targets },
  ];
}
