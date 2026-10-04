// Zones, lanes, adjacency, the two rotation rings, locks and Stack piles (SPEC §3).
// These are the only places a card changes zone; effects (M3-T1) call them and emit the events.

import type { GameEvent, PlayerId, Row, Zone } from "@jackioh/shared";
import { PLAYER_IDS, opponentOf } from "@jackioh/shared";
import { BACKROW_ZONES, UNIT_ZONES } from "./config";
import { dropSpentBrittle, startBrittleOnField } from "./brittleCount";
import { defOf } from "./catalog";
import { cardTypeOf } from "./faces";
import { showToOwner } from "./ownLibrary";
import { flagsOf } from "./scripts";
import { renameInBoardHistory, type CardInstance, type GameState, type HomeZone, type Pile, type PlayerState } from "./state";
import { noteFieldExit, noteMoved, noteUncovered } from "./stays";

export type ZoneSlot = { player: PlayerId; row: Row; lane: number };

export function rowSize(row: Row): number {
  return row === "units" ? UNIT_ZONES : BACKROW_ZONES;
}

export function slotsOf(player: PlayerId, row: Row): ZoneSlot[] {
  // A loop, not `Array.from({ length })`: every unit read asks for slots through the layers, and
  // Array.from's generic path was a fifth of a long AI gate game's time (#188).
  const size = rowSize(row);
  const slots: ZoneSlot[] = [];
  for (let lane = 1; lane <= size; lane += 1) slots.push({ player, row, lane });
  return slots;
}

/** §3.1: lane N-1 and N+1 on the same side and row, never across sides. */
export function adjacent(ref: ZoneSlot): ZoneSlot[] {
  const size = rowSize(ref.row);
  return [ref.lane - 1, ref.lane + 1]
    .filter((lane) => lane >= 1 && lane <= size)
    .map((lane) => ({ player: ref.player, row: ref.row, lane }));
}

/**
 * R14: one ring per row. From the rotating player's seat it runs their lane 1 to 5, then the
 * opponent's lane 5 down to 1, and back. "Right" is one step forward along that order.
 */
export function ringOrder(row: Row, perspective: PlayerId): ZoneSlot[] {
  const size = rowSize(row);
  const mine = Array.from({ length: size }, (_, i) => ({ player: perspective, row, lane: i + 1 }));
  const theirs = Array.from({ length: size }, (_, i) => ({
    player: opponentOf(perspective),
    row,
    lane: size - i,
  }));
  return [...mine, ...theirs];
}

export function ringNeighbor(ref: ZoneSlot, direction: "left" | "right", perspective: PlayerId): ZoneSlot {
  const ring = ringOrder(ref.row, perspective);
  const at = ring.findIndex((slot) => slot.player === ref.player && slot.lane === ref.lane);
  if (at < 0) throw new Error(`zone not on the ${ref.row} ring: ${ref.player} lane ${ref.lane}`);
  const step = direction === "right" ? 1 : -1;
  const next = ring[(at + step + ring.length) % ring.length];
  if (next === undefined) throw new Error("ring index out of range");
  return next;
}

export function isLocked(state: GameState, ref: ZoneSlot): boolean {
  return state.players[ref.player].locks[ref.row][ref.lane - 1] === true;
}

export function lockZone(state: GameState, ref: ZoneSlot): void {
  state.players[ref.player].locks[ref.row][ref.lane - 1] = true;
}

/** B5 E20: a Locked zone accepts summons again. Its occupant, if any, is unaffected. */
export function unlockZone(state: GameState, ref: ZoneSlot): void {
  state.players[ref.player].locks[ref.row][ref.lane - 1] = false;
}

export function pileAt(state: GameState, ref: ZoneSlot): Pile | null {
  if (ref.row !== "units") throw new Error("piles exist in the unit row only");
  return state.players[ref.player].units[ref.lane - 1] ?? null;
}

/**
 * The card that acts in this zone: the top of a Stack pile, or the backrow card (§3.2). In a backrow
 * zone that is the top of its pile (B5 E21) — and a carrier stays that card beneath the Unit it
 * carries, which stands in the zone as a Unit and never as its backrow card (R446, `carriedAt`).
 */
export function cardAt(state: GameState, ref: ZoneSlot): CardInstance | null {
  if (ref.row === "units") return pileAt(state, ref)?.[0] ?? null;
  return state.players[ref.player].backrow[ref.lane - 1] ?? null;
}

export function isEmpty(state: GameState, ref: ZoneSlot): boolean {
  return cardAt(state, ref) === null && (ref.row === "units" || carriedAt(state, ref) === null);
}

// ---------------------------------------------------------------------------
// Backrow piles and carried Units (docs/classic-sets.md B5 E21, R446, R447)
// ---------------------------------------------------------------------------

/** B5 E21: the dormant cards beneath a backrow zone's top card, top first; empty when none. */
export function beneathAt(state: GameState, ref: ZoneSlot): readonly CardInstance[] {
  if (ref.row !== "backrow") return pileAt(state, ref)?.slice(1) ?? [];
  return state.players[ref.player].backrowPiles?.[ref.lane - 1] ?? [];
}

/** R446: the Unit a carrier in this backrow zone holds, or null. */
export function carriedAt(state: GameState, ref: ZoneSlot): CardInstance | null {
  if (ref.row !== "backrow") return null;
  return state.players[ref.player].carried?.[ref.lane - 1] ?? null;
}

/**
 * B5 E21, R446: a backrow card whose text lets a Unit be played on top of it (`staticFlags.carrier`,
 * or Classic+ #33 Ivory Tower's `fusesCarried`, R653). The flag is the card's text, so a Vanilla
 * carrier carries nothing more (§6.3, R115: `flagsOf` reads nothing off a Vanilla instance).
 */
export function isCarrier(card: CardInstance): boolean {
  const flags = flagsOf(card);
  return flags.carrier === true || flags.fusesCarried === true;
}

/**
 * R653: where a carrier that fuses its Unit (`fusesCarried`) notes the Unit stacked onto it, by id, for
 * the rest of its stay. Memory, so R78 clears it when the card leaves the field, and a Fuse that keeps
 * the carrier keeps it (R77: it is the engine's entry, not a text's).
 */
const STACKED_KEY = "__stacked";

/** R653: the id of the Unit stacked onto this `fusesCarried` carrier on this stay, or null if none yet. */
export function stackedOnto(card: CardInstance): string | null {
  const id = card.memory[STACKED_KEY];
  return typeof id === "string" ? id : null;
}

/** R446: whether this card is a Unit standing on a carrier in a backrow zone. */
export function isCarried(state: GameState, card: CardInstance): boolean {
  const zone = card.zone;
  if (zone.z !== "field" || zone.row !== "backrow") return false;
  return carriedAt(state, { player: zone.player, row: "backrow", lane: zone.lane })?.id === card.id;
}

/** R446: every Unit a carrier of this player's holds, in lane order. */
export function carriedUnitsOf(state: GameState, player: PlayerId): CardInstance[] {
  return (state.players[player].carried ?? []).flatMap((card) => (card === null ? [] : [card]));
}

/**
 * R446: the backrow zones of `player`'s side a Unit they play may name — each zone whose acting card
 * is a carrier holding no Unit yet, and which takes a card at all: not Locked, not held for a card's
 * return (R64, B3.1 rule 6). `playChoices` offers and checks exactly these (`legalZonesFor`,
 * `refuseZone`), so the list and the refusal cannot disagree.
 */
export function carrierZonesFor(state: GameState, player: PlayerId): ZoneSlot[] {
  return slotsOf(player, "backrow").filter((ref) => whyCannotCarry(state, ref) === null);
}

/** R446: why a Unit played now could not name this backrow zone, or null when it can. */
export function whyCannotCarry(state: GameState, ref: ZoneSlot): string | null {
  if (ref.row !== "backrow") return "only a backrow zone carries a Unit";
  const top = cardAt(state, ref);
  if (top === null || !isCarrier(top)) return "that zone holds no card a Unit may be played on top of";
  if (carriedAt(state, ref) !== null) return "that card already carries a Unit";
  if (flagsOf(top).fusesCarried === true && stackedOnto(top) !== null) return "that card has taken its one Unit";
  if (isLocked(state, ref)) return "that zone is Locked";
  if (isReserved(state, ref)) return "that zone is held for a card's return";
  return null;
}

/**
 * §3.2, B5 E21: a zone a Stack card may enter although it is occupied. Occupancy is exactly what
 * Stack lifts, so what is left is what occupancy never covered — a Locked zone and a zone held for a
 * card's return (R64, B3.1 rule 6) take no Stack card either — plus, in a backrow zone, a carrier's
 * Unit: a zone carrying one takes nothing more (R446).
 */
export function acceptsStackCard(state: GameState, ref: ZoneSlot): boolean {
  if (isLocked(state, ref) || isReserved(state, ref)) return false;
  return ref.row === "units" || carriedAt(state, ref) === null;
}

/**
 * Everything in a zone, top card first, so a move that lifts whole zones (#52's rotation, #87's board
 * swap) sets each down whole (§3.2): a unit zone's pile, or a backrow zone's carried Unit, its top
 * card and the dormant cards beneath (B5 E21, R446).
 */
export function zoneContents(state: GameState, ref: ZoneSlot): CardInstance[] {
  if (ref.row === "units") return [...(pileAt(state, ref) ?? [])];
  const carried = carriedAt(state, ref);
  const top = cardAt(state, ref);
  return [...(carried === null ? [] : [carried]), ...(top === null ? [] : [top]), ...beneathAt(state, ref)];
}

/** B5 E21: every dormant card beneath this player's backrow tops, lane by lane. */
export function dormantBackrowOf(state: GameState, player: PlayerId): CardInstance[] {
  return (state.players[player].backrowPiles ?? []).flat();
}

function setBeneath(side: PlayerState, lane: number, cards: CardInstance[]): void {
  const piles = side.backrowPiles ?? Array.from({ length: BACKROW_ZONES }, (): CardInstance[] => []);
  piles[lane - 1] = cards;
  if (piles.every((pile) => pile.length === 0)) delete side.backrowPiles;
  else side.backrowPiles = piles;
}

function setCarried(side: PlayerState, lane: number, card: CardInstance | null): void {
  const row = side.carried ?? Array.from({ length: BACKROW_ZONES }, (): CardInstance | null => null);
  row[lane - 1] = card;
  if (row.every((held) => held === null)) delete side.carried;
  else side.carried = row;
}

/**
 * Whether a card's own face is a Unit's (§5.2, B2.7) — the one kind of card that stands on a carrier
 * (R446). Its face, not where it stands: an animated card in a unit zone is a Unit there (R383) but
 * never a Unit face, so it never lands on a carrier.
 */
function isUnitFace(state: GameState, instance: CardInstance): boolean {
  const def = defOf(state, instance.defId);
  const face = instance.radiant ? def.radiant : def.base;
  return (face.type ?? def.type) === "Unit";
}

/** A zone that accepts a summon: empty and unlocked (§3.2). */
export function isOpen(state: GameState, ref: ZoneSlot): boolean {
  return isEmpty(state, ref) && !isLocked(state, ref) && !isReserved(state, ref);
}

/**
 * R64: a dying Reborn unit holds its zone until it comes back. B3.1 rule 6: so does an animated
 * "Animated on your turn" card its backrow zone, for its return at its controller's cleanup.
 */
export function isReserved(state: GameState, ref: ZoneSlot): boolean {
  if (state.reserved.some((r) => r.player === ref.player && r.row === ref.row && r.lane === ref.lane)) return true;
  return (state.homes ?? []).some(
    (home) => home.zone.player === ref.player && home.zone.row === ref.row && home.zone.lane === ref.lane,
  );
}

/** B3.1 rule 6: the home zone held for this animated card, if any. */
export function homeOf(state: GameState, instanceId: string): HomeZone | undefined {
  return (state.homes ?? []).find((home) => home.instanceId === instanceId);
}

/** B3.1 rule 6: hold a backrow zone for an animated card's return. One home per card. */
export function reserveHome(state: GameState, zone: ZoneSlot, instanceId: string): void {
  const homes = (state.homes ?? []).filter((home) => home.instanceId !== instanceId);
  homes.push({ instanceId, zone: { player: zone.player, row: zone.row, lane: zone.lane } });
  state.homes = homes;
}

/** B3.1 rule 6: the card's home is no longer held — it returned, or it left the field. */
export function releaseHome(state: GameState, instanceId: string): void {
  if (state.homes === undefined) return;
  const homes = state.homes.filter((home) => home.instanceId !== instanceId);
  if (homes.length === 0) delete state.homes;
  else state.homes = homes;
}

export function reserveZone(state: GameState, ref: ZoneSlot): void {
  if (!isReserved(state, ref)) state.reserved.push({ ...ref });
}

export function releaseZone(state: GameState, ref: ZoneSlot): void {
  state.reserved = state.reserved.filter(
    (r) => !(r.player === ref.player && r.row === ref.row && r.lane === ref.lane),
  );
}

export function openZones(state: GameState, player: PlayerId, row: Row): ZoneSlot[] {
  return slotsOf(player, row).filter((ref) => isOpen(state, ref));
}

/** R64: the leftmost open zone, or null when the row is full. */
export function firstFreeZone(state: GameState, player: PlayerId, row: Row): ZoneSlot | null {
  return openZones(state, player, row)[0] ?? null;
}

export function zoneOf(ref: ZoneSlot): Zone {
  return { z: "field", player: ref.player, row: ref.row, lane: ref.lane };
}

/** True when this def is a unit token, which ceases to exist off the field (R11). */
export function isUnitToken(state: GameState, instance: CardInstance): boolean {
  const def = defOf(state, instance.defId);
  return def.token && cardTypeOf(state, instance) === "Unit";
}

/**
 * Put a card on the field. A Stack card may enter an occupied unit zone and becomes the top of the
 * pile; the card beneath keeps its damage and stops acting (§3.2). Returns false when the zone
 * cannot take it, leaving the state untouched.
 */
export function placeOnField(
  state: GameState,
  instance: CardInstance,
  ref: ZoneSlot,
  options: { stack?: boolean } = {},
): boolean {
  if (isLocked(state, ref)) return false;
  if (isReserved(state, ref)) return false;
  const side = state.players[ref.player];

  if (ref.row === "units") {
    const existing = side.units[ref.lane - 1] ?? null;
    if (existing !== null && options.stack !== true) return false;
    side.units[ref.lane - 1] = existing === null ? [instance] : [instance, ...existing];
  } else if (isUnitFace(state, instance)) {
    // R446: a Unit enters a backrow zone only on top of a carrier, which stays beneath it and keeps
    // acting there. A move that sets a whole zone down (`zoneContents`: a rotation, a board swap)
    // puts the carrier down first and its Unit back on it, whatever the carrier's text says by then.
    const top = side.backrow[ref.lane - 1] ?? null;
    if (top === null || carriedAt(state, ref) !== null) return false;
    if (!isCarrier(top) && options.stack !== true) return false;
    setCarried(side, ref.lane, instance);
    // R653: the first Unit to stand on a carrier that fuses its Unit is the one it takes this stay.
    if (flagsOf(top).fusesCarried === true && stackedOnto(top) === null) top.memory[STACKED_KEY] = instance.id;
  } else {
    // B5 E21: a Stack card may top an occupied backrow zone as it may a unit zone; the card beneath
    // goes dormant (§3.2). A zone carrying a Unit takes nothing more (R446).
    const existing = side.backrow[ref.lane - 1] ?? null;
    if (existing !== null) {
      if (options.stack !== true || carriedAt(state, ref) !== null) return false;
      setBeneath(side, ref.lane, [existing, ...beneathAt(state, ref)]);
    }
    side.backrow[ref.lane - 1] = instance;
  }

  // R12/R662: a card's current owner follows the side that receives it on the field. Keeping the
  // two aligned here gives every later bounce, graveyard, exile and library move the normal zone
  // routing without an original-owner exception at each departure.
  instance.owner = ref.player;
  // R638: a move from one field zone to another (a steal, a swap, a rotation) is no arrival.
  const fromOffField = instance.zone.z !== "field";
  instance.controller = ref.player;
  instance.zone = zoneOf(ref);
  if (ref.row === "units" || isUnitFace(state, instance)) instance.position ??= "ATK";
  // B3.3 rule 1, R385, R638: a printed Brittle starts as its card enters the field, and a held count starts ticking.
  startBrittleOnField(state, instance, fromOffField);
  return true;
}

/**
 * R227: whether a card placed in this row lands face-down — a Trap or a Field Trap in a backrow
 * (§3.2, R33). A Field Spell lands face-up, and a Unit never reaches the backrow.
 */
export function landsFaceDown(state: GameState, instance: CardInstance, row: Row): boolean {
  if (row !== "backrow") return false;
  const type = cardTypeOf(state, instance);
  return type === "Trap" || type === "Field Trap";
}

/**
 * R227: a card going face-down takes a fresh instance id, so the one handle the action protocol has
 * for a face-down card — a play's target, a prompt option's answer (R177) — is an id no player has
 * seen before, and an id seen while the card was public never names it again. The id is the next
 * number, as a new card's is; whether a card goes face-down is public (§10.8 shows the zone
 * occupied), so the number it takes says nothing either. Called on a card that is in no pile, just
 * before it is placed. Returns the id the card had, which the `cardPlayed` or `summoned` that
 * places it carries as `formerId` for the views to follow (R97).
 */
export function freshFaceDownId(state: GameState, instance: CardInstance): string {
  const former = instance.id;
  instance.id = `c${state.nextId}`;
  state.nextId += 1;
  // R419: C+ #35's history names the card by the id it has now.
  renameInBoardHistory(state, former, instance.id);
  return former;
}

/**
 * §6.3 Replace on the field: the new card takes the old one's place — the same zone, and the same
 * place in a Stack pile — under the same controller. That is no summon, so §3.2's Lock ("the zone
 * accepts no summons … the current occupant is unaffected") and R64's reservation do not refuse it:
 * the zone was occupied before and is occupied after. Returns false, changing nothing, when the old
 * card is not on the field. The old card is left pointing at its zone for the caller to retire.
 */
export function replaceInZone(state: GameState, old: CardInstance, replacement: CardInstance): boolean {
  const zone = old.zone;
  if (zone.z !== "field") return false;
  const side = state.players[zone.player];
  if (zone.row === "units") {
    const pile = side.units[zone.lane - 1] ?? null;
    if (pile === null || !pile.some((card) => card.id === old.id)) return false;
    side.units[zone.lane - 1] = pile.map((card) => (card.id === old.id ? replacement : card));
  } else if (side.backrow[zone.lane - 1]?.id === old.id) {
    side.backrow[zone.lane - 1] = replacement;
  } else if (side.carried?.[zone.lane - 1]?.id === old.id) {
    // R446: the carried Unit's place on its carrier.
    setCarried(side, zone.lane, replacement);
  } else {
    // B5 E21: a dormant card's place in a backrow pile.
    const beneath = beneathAt(state, { player: zone.player, row: "backrow", lane: zone.lane });
    if (!beneath.some((card) => card.id === old.id)) return false;
    setBeneath(side, zone.lane, beneath.map((card) => (card.id === old.id ? replacement : card)));
  }
  replacement.controller = zone.player;
  replacement.zone = { ...zone };
  if (zone.row === "units" || isUnitFace(state, replacement)) replacement.position ??= "ATK";
  // B3.3 rule 1, R385: the new card has entered the field, so its printed Brittle starts.
  startBrittleOnField(state, replacement, true);
  return true;
}

/**
 * Take a card off the field; the card beneath a Stack resumes acting (§3.2), which is noted against
 * the card that left (`stays.noteUncovered`, R212): no event reports a resume. `withPile` is for a
 * move that lifts whole piles and sets each down whole elsewhere (#87's board swap, #52's rotation):
 * its cards come off one at a time, but nothing beneath any of them resumes, so no resume is noted.
 */
export function removeFromField(
  state: GameState,
  instance: CardInstance,
  options: { withPile?: boolean } = {},
): boolean {
  for (const player of PLAYER_IDS) {
    const side = state.players[player];
    for (let i = 0; i < side.units.length; i += 1) {
      const pile = side.units[i] ?? null;
      if (pile === null) continue;
      const at = pile.findIndex((card) => card.id === instance.id);
      if (at >= 0) {
        const rest = pile.filter((card) => card.id !== instance.id);
        side.units[i] = rest.length === 0 ? null : rest;
        noteUncovered(state, instance.id, at === 0 && options.withPile !== true ? rest[0]?.id : undefined);
        return true;
      }
    }
    for (let i = 0; i < side.backrow.length; i += 1) {
      const lane = i + 1;
      if (side.backrow[i]?.id === instance.id) {
        // B5 E21: the card beneath a backrow pile's top resumes, as in a unit pile (§3.2, R212).
        const beneath = [...beneathAt(state, { player, row: "backrow", lane })];
        const resumed = beneath.shift();
        side.backrow[i] = resumed ?? null;
        setBeneath(side, lane, beneath);
        noteUncovered(state, instance.id, options.withPile !== true ? resumed?.id : undefined);
        return true;
      }
      if (side.carried?.[i]?.id === instance.id) {
        setCarried(side, lane, null);
        noteUncovered(state, instance.id, undefined);
        return true;
      }
      const beneath = beneathAt(state, { player, row: "backrow", lane });
      if (beneath.some((card) => card.id === instance.id)) {
        setBeneath(side, lane, beneath.filter((card) => card.id !== instance.id));
        noteUncovered(state, instance.id, undefined);
        return true;
      }
    }
  }
  return false;
}

export type OffFieldZone = "hand" | "library" | "graveyard" | "exile";

/** The pile a card lands in; off the field it always belongs to its owner (R12). */
function pileFor(side: PlayerState, zone: OffFieldZone): CardInstance[] {
  if (zone === "hand") return side.hand;
  if (zone === "library") return side.library;
  if (zone === "graveyard") return side.graveyard;
  return side.exile;
}

export function removeFromAnyZone(state: GameState, instance: CardInstance): void {
  if (removeFromField(state, instance)) return;
  for (const player of PLAYER_IDS) {
    const side = state.players[player];
    for (const zone of ["hand", "library", "graveyard", "exile"] as const) {
      const pile = pileFor(side, zone);
      const at = pile.findIndex((card) => card.id === instance.id);
      if (at >= 0) {
        pile.splice(at, 1);
        // R212: a move of a card that left a pile's top ends that removal's Stack note.
        noteMoved(state, instance.id);
        // R155: §5.1's end-of-turn return belongs to the Spell its own play landed in the graveyard
        // (§10.5 step 7). A card that leaves the graveyard has spent that landing, so whatever puts
        // it back there this turn — a discard (#76), a burn — is no play of its, and it stays (R153).
        if (zone === "graveyard") delete instance.returnToHandAtEndOfTurn;
        return;
      }
    }
    // §10.5 step 4 parks a card here between its play and its destination. Without this a Spell
    // moved from `resolving` to the graveyard would be left in both piles, i.e. two live copies
    // of one instance — so the resolving pile is searched like any other.
    const resolvingAt = side.resolving.findIndex((card) => card.id === instance.id);
    if (resolvingAt >= 0) {
      side.resolving.splice(resolvingAt, 1);
      noteMoved(state, instance.id);
      return;
    }
  }
}

/**
 * R78: leaving the field resets an instance, while costMod, costOverride and radiant persist. R215
 * applies the same reset to a hand or library card that reaches a graveyard or exile, and to a card
 * leaving the resolving zone once its play is over.
 *
 * Patch v0.2.0 adds three more that persist in every zone (R385, R386, B5 E39): `tuning` (what
 * Degrade, Upgrade and KY's Constant changed), `brittle` (the Brittle count) and `enchantments` —
 * none of them is touched here. The one exception is a Brittle count that has crumbled its card, which
 * is spent and goes (R441, `brittleCount.dropSpentBrittle`).
 */
export function resetInstance(instance: CardInstance): void {
  dropSpentBrittle(instance);
  instance.damage = 0;
  instance.buffs = { attack: 0, health: 0 };
  instance.grantedKeywords = [];
  instance.vanilla = false;
  instance.counters = {};
  instance.memory = {};
  instance.exertion = { attacked: false, switched: false };
  instance.controller = instance.owner;
  delete instance.position;
  delete instance.summonedTurn;
  delete instance.statsOverride;
  delete instance.armorOverride;
  delete instance.tauntSuppressedTurn;
  delete instance.faceUp;
  delete instance.lastDamagedBy;
  delete instance.x;
  delete instance.embiggened;
  delete instance.divineShieldSpent;
  delete instance.markedDestroyed;
  delete instance.rebornSpent;
  // B5 E35: Berserk is a status of the unit on the field, lost as it leaves (R78).
  delete instance.berserk;
}

/**
 * R174: drop the delayed effects aimed at a card that is leaving the field (`DelayedEffect.watch`).
 * The card that may later stand in the same zone under the same id — bounced and replayed, or back
 * through Reborn — is a new arrival (R78, R83), and an effect aimed at the old one fizzles (R76).
 */
function forgetWatchers(state: GameState, instanceId: string): void {
  if (!state.delayed.some((effect) => effect.watch === instanceId)) return;
  state.delayed = state.delayed.filter((effect) => effect.watch !== instanceId);
}

/** The zones a queue entry names when the card answered from the field (`triggers.queueTrigger`). */
const FIELD_TRIGGER_ZONES: readonly unknown[] = ["field", "backrow"];

/**
 * R174: drop the triggers and turn hooks this card queued while it stood on the field. They belong
 * to that stay: a Reborn body or a replayed card under the same id is a reset instance that has
 * entered the field again (R78, R83), so an entry queued before it left — #91's Plague Token for the
 * hit that killed it, #37's start-of-turn hook queued before it died — never acts on what came
 * back. Without Reborn the entry already fizzled, because a card in a graveyard answers none of
 * its field triggers (R153); this makes the card that returns answer none of them either.
 */
function forgetQueuedTriggers(state: GameState, instanceId: string): void {
  const owned = (entry: GameState["triggerQueue"][number]): boolean =>
    entry.instanceId === instanceId && FIELD_TRIGGER_ZONES.includes(entry.resume.data.zone);
  if (!state.triggerQueue.some(owned)) return;
  state.triggerQueue = state.triggerQueue.filter((entry) => !owned(entry));
}

/**
 * R86: a card ceases to exist — replaced by a Transform (R35), fused away (R77), or a unit token
 * leaving the field (R11) — and is in no pile afterwards, which is the `{ z: "gone" }` zone. One
 * that ceases to exist ON the field has left it, as a destroyed or bounced card has (R174): the
 * departure is counted, so a trap later in the same dispatch meets a play the first Sheepish turned
 * into a Sheep as a card no longer in play (`traps.standingEvent`), and the delayed effects and
 * queued triggers aimed at that stay end with it, as `moveToZone` ends them for a card that lands.
 */
export function ceaseToExist(state: GameState, instance: CardInstance): void {
  const wasOnField = instance.zone.z === "field";
  removeFromAnyZone(state, instance);
  if (wasOnField) leftTheField(state, instance.id);
  instance.zone = { z: "gone", player: instance.owner };
}

/**
 * R174: what a card leaving the field ends — the stay every effect aimed at it was aimed at, the
 * delayed effects watching it, the triggers it queued there — and, B3.1 rule 6, the home zone an
 * animated card held for its return: it will not return from a graveyard, a hand or exile.
 */
function leftTheField(state: GameState, instanceId: string): void {
  noteFieldExit(state, instanceId);
  forgetWatchers(state, instanceId);
  forgetQueuedTriggers(state, instanceId);
  releaseHome(state, instanceId);
}

/**
 * §2.3: X and the embiggen price are chosen at play time and stored on the played instance, and
 * R65 has an X-cost card cost 0 and an embiggen card its base price everywhere outside play. So the
 * choice ends with the play: a Spell that leaves the resolving zone (to its graveyard, to exile, or
 * straight to a hand) drops it, as R78's reset drops it from a permanent leaving the field. Without
 * this #24 Efficiency Dividend returned to hand at the X it was last played for.
 */
function endPlayChoices(instance: CardInstance): void {
  delete instance.x;
  delete instance.embiggened;
}

/**
 * `replaced` (B5 E5, R460): the card was on its way to a graveyard and a replacement sent it
 * elsewhere — its exile pile, or the bottom of its library — so it is not in the graveyard, and
 * `reportGraveyardLanding` names where it went.
 */
export type MoveResult = "moved" | "vanished" | "replaced";

// ---- B5 E5: "would go to a graveyard" (damage and combat) ----

/** Where a replacement sends a card that would go to a graveyard (B5 E5; Classic #28, #50, #60). */
export type GraveyardRedirect = { to: "exile" } | { to: "library"; position: "bottom" };

/**
 * B5 E5: the replacement check for a move into a graveyard, registered at module scope by
 * `replacements.ts`, which reads the board's replacements (R460) — this module sits under it, so the
 * layering forbids the call. Asked once the card has left the zone it was in, so a card whose own
 * aura it was has taken the aura with it (Classic #50 Voidwalker's own card). Unregistered, nothing is replaced.
 */
export type GraveyardRedirectCheck = (state: GameState, instance: CardInstance) => GraveyardRedirect | null;

let graveyardRedirect: GraveyardRedirectCheck = () => null;

/** Registered by `replacements.ts` at module scope. Returns the check it replaced. */
export function registerGraveyardRedirect(check: GraveyardRedirectCheck): GraveyardRedirectCheck {
  const previous = graveyardRedirect;
  graveyardRedirect = check;
  return previous;
}

/**
 * B5 E5: the event a move toward a graveyard reports once the card has landed, wherever that was —
 * `enteredGraveyard` in the graveyard, `exiled` when a replacement exiled it, `shuffledIn` at the
 * bottom of its library — and nothing for a unit token that ceased to exist (R11). Every engine path
 * that sends a card to a graveyard reports its landing here rather than assuming the graveyard.
 */
export function reportGraveyardLanding(sink: { events: GameEvent[]; state: GameState }, instance: CardInstance, result: MoveResult): void {
  if (result === "vanished") return;
  const base = { instanceId: instance.id, defId: instance.defId, owner: instance.owner };
  const zone = instance.zone.z;
  if (zone === "graveyard") {
    sink.events.push({ type: "enteredGraveyard", ...base });
  } else if (zone === "exile") {
    sink.events.push({ type: "exiled", ...base });
  } else if (zone === "library") {
    const position = sink.state.players[instance.owner].library.findIndex((card) => card.id === instance.id);
    sink.events.push({ type: "shuffledIn", player: instance.owner, instanceId: instance.id, defId: instance.defId, position });
  }
}

/**
 * Move a card to one of its owner's off-field zones. Unit tokens cease to exist instead (R11),
 * and a unit-token card leaving hand or library other than by being drawn or played does too.
 *
 * R151's arrival hook is deliberately NOT here, although this is the single point every zone change
 * goes through: the roll a card makes as it arrives needs the match rng, and this function takes a
 * `GameState`, which holds only the seed and the cursor `reduce` stores between actions. Building an
 * rng from those mid-action would repeat draws the action's own rng has already taken and would have
 * its advanced cursor thrown away by `reduce`'s `next.rngCursor = sink.rng.cursor`. The hook lives
 * one layer up, on the sink-holding funnels every hand and library arrival passes through —
 * `draw.addToHand` and `draw.shuffleIntoLibrary` (`runArrivalHooks` in `draw.ts`).
 */
export function moveToZone(
  state: GameState,
  instance: CardInstance,
  zone: OffFieldZone,
  options: { position?: "top" | "bottom" | number; keepState?: boolean } = {},
): MoveResult {
  const from = instance.zone.z;
  const wasOnField = from === "field";
  const token = isUnitToken(state, instance);
  removeFromAnyZone(state, instance);
  // R174: leaving the field ends every delayed effect aimed at this card and every trigger it
  // queued there, whatever comes back, and ends the stay every effect aimed at it was aimed at.
  if (wasOnField) leftTheField(state, instance.id);
  if (from === "resolving") endPlayChoices(instance);

  // R11: a unit token ceases to exist when it leaves the field, and a unit-token card ceases to
  // exist when it would reach a graveyard or exile. One may live in a hand or library (#75) and
  // "ceases to exist if it leaves that zone other than by being drawn or played, burning
  // included" — a draw is this call moving it from the library to the hand, and a play never comes
  // through here (`playSteps` puts the card on the field or in `resolving` itself), so every other
  // move out of a hand or a library ends it: discarded, exiled, burned, shuffled back.
  // Staying put is not leaving, so a copy shuffled into the library it was made in lives (R34).
  const leftHandOrLibrary =
    (from === "hand" || from === "library") && zone !== from && !(from === "library" && zone === "hand");
  if (
    token &&
    (wasOnField || from === "resolving" || zone === "graveyard" || zone === "exile" || leftHandOrLibrary)
  ) {
    // R86: it ceased to exist, so it goes to "gone" rather than looking like an exiled card.
    instance.zone = { z: "gone", player: instance.owner };
    return "vanished";
  }

  // R215: a card that reaches a graveyard or an exile pile from a hand or a library is reset too, so
  // what comes back from there is the printed card (#89's hand buffs, #98's rolled power, R151) —
  // R78's reset, with `costMod`, `costOverride` and `radiant` kept in every zone as R78 keeps them.
  // So is a card that lands from the resolving zone (§10.5 step 7): its play is over, and a #95 an
  // earlier Call to Chaos cast (R87) carries no link of that chain (R28) back into a play of its own.
  const pileToPile = (from === "hand" || from === "library") && (zone === "graveyard" || zone === "exile");
  const landed = from === "resolving";
  if ((wasOnField || pileToPile || landed) && options.keepState !== true) resetInstance(instance);

  const side = state.players[instance.owner];
  // B5 E5, R460: a card that would go to a graveyard may be sent elsewhere instead — asked now that it
  // has left the zone it was in. It lands the way the graveyard would have had it land (the reset
  // above), and an exile counts like any other (R55). A unit token never gets here (R11).
  const redirect = zone === "graveyard" ? graveyardRedirect(state, instance) : null;
  if (redirect !== null) {
    if (redirect.to === "exile") {
      side.exile.push(instance);
      instance.zone = { z: "exile", player: instance.owner };
      state.counters.exiled += 1;
    } else {
      side.library.push(instance);
      instance.zone = { z: "library", player: instance.owner };
      // R311: it goes in openly, on its way to a pile both players read.
      showToOwner(instance);
    }
    return "replaced";
  }
  const pile = pileFor(side, zone);
  const at = options.position;
  if (zone === "library" && at !== undefined && at !== "top") {
    const index = at === "bottom" ? pile.length : Math.max(0, Math.min(pile.length, at));
    pile.splice(index, 0, instance);
  } else if (zone === "library") {
    pile.unshift(instance);
  } else {
    pile.push(instance);
  }
  instance.zone = { z: zone, player: instance.owner };
  return "moved";
}

/**
 * Every unit that acts for this player: each unit zone's top card in lane order (§3.2), then the
 * Units its carriers hold, in backrow lane order — a carried Unit is a Unit for every rule (R446).
 */
export function activeUnitsOf(state: GameState, player: PlayerId): CardInstance[] {
  // Read straight off the rows, not through `slotsOf` and `cardAt`: the auras ask for this on every
  // unit read (`layers.auraSources`), so it builds nothing it does not return (#188).
  const side = state.players[player];
  const units: CardInstance[] = [];
  for (let lane = 1; lane <= UNIT_ZONES; lane += 1) {
    const top = side.units[lane - 1]?.[0] ?? null;
    if (top !== null) units.push(top);
  }
  for (const card of side.carried ?? []) if (card !== null) units.push(card);
  return units;
}

export function dormantUnitsOf(state: GameState, player: PlayerId): CardInstance[] {
  return state.players[player].units.flatMap((pile) => (pile ?? []).slice(1));
}

/**
 * §3.2, R13: a card dormant under a Stack pile — in a unit zone and not the top of its pile. It is
 * "not on the field for effects": nothing targets it, and an effect aimed at it fizzles (R174).
 */
export function isBuried(state: GameState, instance: CardInstance): boolean {
  const zone = instance.zone;
  if (zone.z !== "field") return false;
  const ref: ZoneSlot = { player: zone.player, row: zone.row, lane: zone.lane };
  // B5 E21: a backrow pile's dormant cards are buried the same way; the top and a carrier's Unit act.
  if (zone.row === "backrow") return beneathAt(state, ref).some((card) => card.id === instance.id);
  return cardAt(state, ref)?.id !== instance.id;
}

/**
 * §3.2, R446: whether a card acts on the field — the top of a unit pile, the top of a backrow zone, or
 * a Unit a carrier holds. A dormant card under either kind of pile does not.
 */
export function actsOnField(state: GameState, instance: CardInstance): boolean {
  return instance.zone.z === "field" && !isBuried(state, instance);
}

/**
 * R64: "fill your board" takes every empty, unlocked unit zone, left to right. The caller makes
 * each card; this returns the zones to fill, in order.
 */
export function fillBoardZones(state: GameState, player: PlayerId): ZoneSlot[] {
  return openZones(state, player, "units");
}

export function slotOf(state: GameState, instance: CardInstance): ZoneSlot | null {
  const zone = instance.zone;
  if (zone.z !== "field") return null;
  return { player: zone.player, row: zone.row, lane: zone.lane };
}

// ---------------------------------------------------------------------------
// Moves inside the field: Animated (B3.1, R383), a carrier's Unit (R446), Flicker (B5 E22, R444)
// ---------------------------------------------------------------------------

/**
 * B3.1 rules 2 and 5, R446: move a card acting in one of its side's backrow zones — an Animated card,
 * or a Unit a carrier held — into an open unit zone of that side, without leaving the field: no R78
 * reset, no departure counted (R174), nothing that watches it or that it queued forgotten. A card
 * dormant beneath it in its backrow pile resumes (§3.2). False, changing nothing, when the card is
 * not acting in a backrow zone or `to` is not an open unit zone of its side.
 */
export function stepIntoUnitZone(state: GameState, card: CardInstance, to: ZoneSlot): boolean {
  const from = slotOf(state, card);
  if (from === null || from.row !== "backrow" || to.row !== "units") return false;
  if (from.player !== to.player || !actsOnField(state, card) || !isOpen(state, to)) return false;
  removeFromField(state, card, { withPile: true });
  return placeOnField(state, card, to);
}

/**
 * B3.1 rule 6: move a card acting in a unit zone into a backrow zone of its side, without leaving the
 * field. `to` must take it: not Locked, not held for another card, and empty — or, for a card that
 * has Stack, a zone a Stack card may top (B5 E21). The caller releases the card's own home first. False,
 * changing nothing, when it cannot go.
 */
export function stepIntoBackrow(state: GameState, card: CardInstance, to: ZoneSlot, options: { stack?: boolean } = {}): boolean {
  const from = slotOf(state, card);
  if (from === null || from.row !== "units" || to.row !== "backrow") return false;
  if (from.player !== to.player || !actsOnField(state, card)) return false;
  const takes = isEmpty(state, to) ? isOpen(state, to) : options.stack === true && acceptsStackCard(state, to);
  if (!takes) return false;
  removeFromField(state, card, { withPile: true });
  return placeOnField(state, card, to, { stack: options.stack === true });
}

/**
 * B5 E22: the card leaves the field and re-enters the same zone at once — the same place in its pile,
 * on the same side. Leaving is leaving (R174: every effect aimed at it and every trigger it queued
 * ends, and an animated card's home is released), and R78 resets it; re-entering is entering, so it is
 * summoning sick (R83) and in Attack Position. It never passes through another pile, so a unit token
 * comes back like any other card (R444, as R175 brings one back through Reborn). The caller emits
 * the events. False, changing nothing, for a card that is not acting on the field.
 */
export function flickerInPlace(state: GameState, card: CardInstance): boolean {
  const zone = card.zone;
  if (zone.z !== "field" || !actsOnField(state, card)) return false;
  leftTheField(state, card.id);
  resetInstance(card);
  card.controller = zone.player;
  card.summonedTurn = state.turn;
  if (zone.row === "units" || isUnitFace(state, card)) card.position = "ATK";
  return true;
}
