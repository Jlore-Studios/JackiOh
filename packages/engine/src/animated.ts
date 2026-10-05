// Animated (docs/classic-sets.md B3.1, R383): a Field Spell, Trap or Field Trap that steps into a unit
// zone as a Unit, and the "Animated on your turn" cards that go back to their backrow zone at their
// controller's cleanup. `turn.ts` runs `animateAtTurnStart` as a stage of the start of a turn (after the
// Brittle tick, before the delayed effects) and `returnAtCleanup` at cleanup, after every end-of-turn
// step (§2.2, R62).
//
// What this module owns, rule by rule:
//   1. printing — `animatedKindOf` reads the keyword off the card's layers (§10.4), so a Vanilla card
//      has lost it where it stands (R115) and a granted one counts;
//   2. to animate — `animateCard`: the same lane's unit zone when it is open, else the leftmost open,
//      unlocked, unreserved one (R64's placement), in Attack Position unless the text says otherwise;
//      with none open the card stays where it is;
//   3. while animated it is a Unit for every rule — it stands in its controller's `units` pile, so
//      every unit walk finds it, and `faces.cardTypeOf` answers "Unit" there — and it keeps its text:
//      an animated Field Trap still fires as a trap (`traps.trapsInOrder`), which is why the trap
//      machinery reads `faceTypeOf` here and never `cardTypeOf`;
//   4. when — the trap's own list ends in the `animate` verb (`effects/animate.ts`), a Field Spell as it
//      enters (`animateOnEntry`), an "on your turn" card at its controller's start of turn and as it
//      enters on their turn, and back at their cleanup (`returnAtCleanup`);
//   5. moving is not leaving the field (`zones.stepIntoUnitZone`, `zones.stepIntoBackrow`: no R78
//      reset, no departure, R174), but entering the unit zone is entering it on that turn: summoning
//      sick, a fresh exertion (R83, R171);
//   6. the home zone — held while an "on your turn" card is animated (`zones.reserveHome`), with the
//      rule's three exceptions: a Lock since stops the return, a new controller has no home on the
//      old side (the card goes to its new controller's leftmost open backrow zone, or stays), and a
//      card dormant under a Stack does not return;
//   7. in the backrow it is not a Unit (it is not in a unit pile);
//   8. a face-down Animated Trap is hidden like any trap until it fires: nothing here animates a
//      face-down card except the trap's own firing, which has turned it face-up first (`traps.fireTrap`).
//
// R445: animating is not a summon — the card was on the field already (§6.3 Summon puts a card onto
// the field from anywhere else) — so it emits `animated`, never `summoned`, and nothing that answers
// a summon (Classic #5 Tesla) answers it.

import type { CardType, GameEvent, KeywordKind, PlayerId } from "@jackioh/shared";
import { hasKeyword } from "@jackioh/shared";
import { defOf } from "./catalog";
import { runningFace } from "./faces";
import { unitView } from "./layers";
import { isFaceDown } from "./preview";
import { isTurnOf, type CardInstance, type GameState, type Position } from "./state";
import {
  actsOnField,
  cardAt,
  firstFreeZone,
  homeOf,
  isCarried,
  isOpen,
  releaseHome,
  reserveHome,
  slotOf,
  slotsOf,
  stepIntoBackrow,
  stepIntoUnitZone,
  type ZoneSlot,
} from "./zones";

/** What the moves here need: the state and the event list — an `EngineSink`, or an effect's context. */
export type FieldSink = { state: GameState; events: GameEvent[] };

/** B3.1 rule 1: the two printings. */
export type AnimatedKind = "Animated" | "Animated on your turn";

const ON_YOUR_TURN: KeywordKind = "Animated on your turn";

/**
 * B2.7: the type the card's running face is printed as, wherever it stands. An animated card is a Unit
 * while it stands in a unit zone (`faces.cardTypeOf`, R383), but what its text *is* — a trap that
 * fires, a Field Trap that stays — is its face's, and the trap machinery reads this.
 */
export function faceTypeOf(state: GameState, card: Pick<CardInstance, "defId" | "radiant">): CardType {
  return runningFace(state, card).type ?? defOf(state, card.defId).type;
}

/**
 * B3.1 rule 1: the Animated the card has now, read off its keywords as the layers compute them (§10.4:
 * printed unless Vanilla, granted, from an aura), or null. "Animated on your turn" is the narrower
 * printing and wins when a card somehow has both.
 */
export function animatedKindOf(state: GameState, card: CardInstance): AnimatedKind | null {
  const keywords = unitView(state, card).keywords;
  if (hasKeyword(keywords, ON_YOUR_TURN)) return "Animated on your turn";
  if (hasKeyword(keywords, "Animated")) return "Animated";
  return null;
}

/** B3.1 rule 3, R383: a card that is not a Unit by its face, standing in a unit zone as one. */
export function isAnimated(state: GameState, card: CardInstance): boolean {
  const zone = card.zone;
  return zone.z === "field" && zone.row === "units" && faceTypeOf(state, card) !== "Unit";
}

/** B3.1 rule 2: where an animating card goes — its lane's unit zone when open, else R64's leftmost. */
function unitZoneFor(state: GameState, player: PlayerId, lane: number): ZoneSlot | null {
  const same: ZoneSlot = { player, row: "units", lane };
  return isOpen(state, same) ? same : firstFreeZone(state, player, "units");
}

/**
 * B3.1 rules 2, 4 and 5 (R383): animate a card acting in its controller's backrow — move it into a unit
 * zone as a Unit, face-up, summoning sick, with a fresh exertion, in `position` (Attack unless the text
 * says otherwise), without leaving the field. An "Animated on your turn" card's backrow zone is held for
 * its return (rule 6). Emits `animated`, never `summoned` (R445).
 *
 * True when the card now stands in a unit zone — including a card that already did, which "does not
 * move or change position" (rule 4). False, changing nothing, when it is not acting in a backrow zone
 * (dormant under a pile, a carried Unit, off the field) or no unit zone of its side is open (rule 2:
 * it stays where it is).
 */
export function animateCard(sink: FieldSink, card: CardInstance, options: { position?: Position } = {}): boolean {
  const state = sink.state;
  if (isAnimated(state, card) && actsOnField(state, card)) return true;
  const from = slotOf(state, card);
  if (from === null || from.row !== "backrow" || !actsOnField(state, card) || isCarried(state, card)) return false;
  const to = unitZoneFor(state, from.player, from.lane);
  if (to === null || !stepIntoUnitZone(state, card, to)) return false;

  card.position = options.position ?? "ATK";
  card.summonedTurn = state.turn;
  card.exertion = { attacked: false, switched: false };
  card.faceUp = true;
  if (animatedKindOf(state, card) === ON_YOUR_TURN) reserveHome(state, from, card.id);
  sink.events.push({
    type: "animated",
    player: from.player,
    instanceId: card.id,
    defId: card.defId,
    backrowLane: from.lane,
    unitLane: to.lane,
  });
  return true;
}

/**
 * B3.1 rule 6: an animated "Animated on your turn" card goes back to a backrow zone of its side — its
 * home when that is still held for it on this side, Locked since or not (R688: the return is a move,
 * and only plays refuse a Locked zone); its new controller's
 * leftmost open backrow zone when it has changed sides and so has no home here; nowhere, staying a
 * Unit, when neither takes it. A card dormant under a Stack does not return (it is not acting). Its
 * position goes with the unit zone. Emits `deanimated`. True when it went back.
 */
export function returnHome(sink: FieldSink, card: CardInstance): boolean {
  const state = sink.state;
  const from = slotOf(state, card);
  if (from === null || from.row !== "units" || !actsOnField(state, card) || !isAnimated(state, card)) return false;

  const home = homeOf(state, card.id);
  const atHome = home !== undefined && home.zone.player === from.player;
  let to: ZoneSlot | null;
  if (home !== undefined && atHome) {
    to = { ...home.zone };
  } else {
    // A home on the other side belongs to the side the card left: that zone is free again.
    if (home !== undefined) releaseHome(state, card.id);
    to = firstFreeZone(state, from.player, "backrow");
    if (to === null) return false;
  }

  // Its own home is held for it alone; a card dormant there since it left (the pile it animated off,
  // B5 E21) is the one thing the zone can hold, and it goes back on top of that pile.
  releaseHome(state, card.id);
  if (!stepIntoBackrow(state, card, to, { stack: atHome })) {
    if (home !== undefined && atHome) reserveHome(state, home.zone, card.id);
    return false;
  }
  delete card.position;
  sink.events.push({
    type: "deanimated",
    player: from.player,
    instanceId: card.id,
    defId: card.defId,
    unitLane: from.lane,
    backrowLane: to.lane,
  });
  return true;
}

/**
 * B3.1 rule 4: a card that entered a backrow zone just now animates as it enters when it is an
 * Animated Field Spell, or an "Animated on your turn" card entering on its controller's turn. A
 * face-down Trap never does (rule 8): an Animated Trap or Field Trap animates as its firing's last step
 * instead. Called by the paths that put a card onto the field — §10.5 step 4 (`playSteps.placeCard`,
 * plays and casts) and every summon (`effects/summon.summonOnto`) — right after the event that placed it.
 */
export function animateOnEntry(sink: FieldSink, card: CardInstance): boolean {
  const state = sink.state;
  const at = slotOf(state, card);
  if (at === null || at.row !== "backrow" || !actsOnField(state, card) || isCarried(state, card)) return false;
  if (isFaceDown(state, card)) return false;
  const kind = animatedKindOf(state, card);
  if (kind === "Animated on your turn") return isTurnOf(state, card.controller) && animateCard(sink, card);
  if (kind === "Animated" && faceTypeOf(state, card) === "Field Spell") return animateCard(sink, card);
  return false;
}

/**
 * B3.1 rule 4: `player`'s "Animated on your turn" cards step into their unit zones, backrow lane 1 to
 * 5 (R68's order), each as far as an open unit zone lets it. Only the top of a backrow pile acts (B5
 * E21), and a face-down card stays hidden (rule 8).
 */
export function animateAtTurnStart(sink: FieldSink, player: PlayerId): void {
  for (const ref of slotsOf(player, "backrow")) {
    const card = cardAt(sink.state, ref);
    if (card === null || isFaceDown(sink.state, card)) continue;
    if (animatedKindOf(sink.state, card) !== ON_YOUR_TURN) continue;
    animateCard(sink, card);
  }
}

/**
 * B3.1 rules 4 and 6: `player`'s animated "on your turn" cards go back to their home zones, unit lane
 * 1 to 5, after every end-of-turn step so their own end-of-turn text ran while they were Units. A card
 * that has lost the keyword since (a Vanilla) stays a Unit for good, and the zone it held is let go.
 */
export function returnAtCleanup(sink: FieldSink, player: PlayerId): void {
  for (const ref of slotsOf(player, "units")) {
    const card = cardAt(sink.state, ref);
    if (card === null || !isAnimated(sink.state, card)) continue;
    if (animatedKindOf(sink.state, card) !== ON_YOUR_TURN) {
      releaseHome(sink.state, card.id);
      continue;
    }
    returnHome(sink, card);
  }
}
