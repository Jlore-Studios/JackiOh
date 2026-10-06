// Board snapshots (docs/classic-sets.md B5 E29; SPEC §2.2, §10.1, §8.7 C+ #35; R419, R562, R563, R566):
// the history C+ #35 Rollback returns the field to.
//
//   - `recordBoardSnapshot` is the first step of every turn's start, before the mana refresh (R62): both
//     sides' zones as whole instances (dormant cards, backrow piles and carried Units included), their
//     Locks and the homes held then (R383), the last BOARD_HISTORY_DEPTH kept in `state.boardHistory`.
//     Plain data (§9.3); `viewFor` never names it (§10.8). A card that takes a fresh id is renamed in it
//     (`state.renameInBoardHistory`, R227), so it always names a card by the id it has now.
//   - `restoreBoard` is R419's three steps, each on every restored side before the next: 1. a card there
//     the restored part of the snapshot does not hold goes to its controller's hand (a Bounce: R78, §2.4, R11, R747);
//     2. each card it holds goes back to its zone and place, moved from wherever it is or recreated, with
//     a fresh id when it goes face-down from anywhere but a face-down zone (R227); 3. the Locks become
//     the snapshot's. R562 picks the snapshot, R563 the held zones, R566 what a card keeps of the present.
//
// Not plays, summons or deaths: `rolledBack`, then `bounced`/`burned` (step 1), `controlChanged` for a
// card that entered the side (step 2, as a board swap's do, R73) and `locked`/`unlocked` (step 3). A card
// that only moved along its own side entered nothing (R171): `rolledBack` covers it, as `rotated` does.

import type { GameEvent, PlayerId, Row } from "@jackioh/shared";
import { PLAYER_IDS } from "@jackioh/shared";
import { enterNewSide } from "../combat";
import { BOARD_HISTORY_DEPTH } from "../config";
import { bounceCard } from "../effects/move";
import { isFaceDown } from "../preview";
import type { EngineSink } from "../resolve";
import type { Effect } from "../script";
import { findInstance, sideSnapshotInstances, type BoardSnapshot, type CardInstance, type GameState, type SideSnapshot } from "../state";
import { freshFaceDownId, placeOnField, releaseHome, removeFromAnyZone, removeFromField, slotsOf, zoneContents, type ZoneSlot } from "../zones";

const ROWS: readonly Row[] = ["units", "backrow"];

const copyOf = <T>(value: T): T => JSON.parse(JSON.stringify(value)) as T;

/** R419, §2.2: the field as the turn that has just begun finds it; the last BOARD_HISTORY_DEPTH are kept, oldest first. */
export function recordBoardSnapshot(state: GameState): void {
  const side = (player: PlayerId): SideSnapshot => {
    const { units, backrow, backrowPiles, carried, locks } = state.players[player];
    const homes = (state.homes ?? []).filter((home) => home.zone.player === player);
    return copyOf({ units, backrow, locks, backrowPiles, carried, ...(homes.length === 0 ? {} : { homes }) });
  };
  const snapshot: BoardSnapshot = { turn: state.turn, sides: { p1: side("p1"), p2: side("p2") } };
  state.boardHistory = [...(state.boardHistory ?? []), snapshot].slice(-BOARD_HISTORY_DEPTH);
}

/**
 * R419, R562: the snapshot of the start of player-turn (this turn − N), or the oldest the history holds
 * when it holds none that old — at most the game's first turn's. Null with no history (before turn 1).
 */
export function snapshotFor(state: GameState, turnsAgo: number): BoardSnapshot | null {
  return (state.boardHistory ?? []).find((snapshot) => snapshot.turn >= state.turn - turnsAgo) ?? null;
}

/** One zone as a snapshot holds it, in `zones.zoneContents`'s order: a pile top first; a carried Unit, top, dormant cards. */
function snapshotZone(side: SideSnapshot, ref: ZoneSlot): CardInstance[] {
  const i = ref.lane - 1;
  if (ref.row === "units") return [...(side.units[i] ?? [])];
  return [side.carried?.[i] ?? null, side.backrow[i] ?? null, ...(side.backrowPiles?.[i] ?? [])].filter(
    (card): card is CardInstance => card !== null,
  );
}

const zonesOf = (player: PlayerId): ZoneSlot[] => ROWS.flatMap((row) => slotsOf(player, row));

/** A card the snapshot holds (`card`, a copy, becomes the card on the field) and where it stood just before. */
type Placement = { card: CardInstance; ref: ZoneSlot; live: CardInstance | undefined; from: PlayerId | null; wasFaceDown: boolean };

/**
 * R566: a card that stood on the same side of the field just before, under the same id, entered nothing
 * (R171) and keeps its sickness and exertion; any other — a fresh id included — entered the field on
 * this turn (R83), crossing sides with what it installed (`combat.enterNewSide`). Returns whether it entered.
 */
function stampTurnState(sink: EngineSink, { card, ref, live, from }: Placement, renamed: boolean): boolean {
  delete card.summonedTurn;
  delete card.tauntSuppressedTurn;
  if (live !== undefined && from === ref.player && !renamed) {
    const { summonedTurn, exertion, tauntSuppressedTurn } = live;
    Object.assign(card, { exertion }, summonedTurn === undefined ? {} : { summonedTurn }, tauntSuppressedTurn === undefined ? {} : { tauntSuppressedTurn });
    return false;
  }
  enterNewSide(sink, card, from ?? ref.player);
  return true;
}

/**
 * R419: return `only`'s sides of the field to the snapshot `turnsAgo` names (R562). Returns how many
 * turns it went back, or null with no history, when nothing happens.
 */
export function restoreBoard(sink: EngineSink, by: PlayerId, turnsAgo: number, only: readonly PlayerId[]): number | null {
  const state = sink.state;
  const found = snapshotFor(state, turnsAgo);
  const sides = PLAYER_IDS.filter((player) => only.includes(player));
  if (found === null || sides.length === 0) return null;
  // A copy: the fresh ids handed out below rename the stored history's cards (R227), not these.
  const snapshot = copyOf(found);
  const back = state.turn - snapshot.turn;
  sink.events.push({ type: "rolledBack", player: by, turnsAgo: back, sides });

  // Step 1.
  const held = new Set(sides.flatMap((player) => sideSnapshotInstances(snapshot.sides[player]).map((card) => card.id)));
  for (const player of sides) {
    for (const card of zonesOf(player).flatMap((ref) => zoneContents(state, ref))) {
      if (!held.has(card.id)) bounceCard(sink, card);
    }
  }

  // Step 2: read where every held card stands, lift them all out, then rebuild each zone bottom first.
  const placements: Placement[] = [];
  for (const player of sides) {
    for (const ref of zonesOf(player)) {
      for (const card of snapshotZone(snapshot.sides[player], ref)) {
        const live = findInstance(state, card.id);
        // R566: a card mid-play stays its play's (§10.5); its place in the snapshot is left empty.
        if (live?.zone.z === "resolving") continue;
        const at = live?.zone.z === "field" ? live.zone : null;
        placements.push({ card, ref, live, from: at?.player ?? null, wasFaceDown: at?.row === "backrow" && isFaceDown(state, live as CardInstance) });
      }
    }
  }
  for (const { live, from } of placements) {
    if (live === undefined) continue;
    if (from === null) {
      removeFromAnyZone(state, live);
      continue;
    }
    // A restored side's piles are rebuilt whole, so nothing there resumes; on the other side the card
    // beneath does (§3.2). A move along the field is no departure (R174); a moved card's own home goes (R563).
    removeFromField(state, live, { withPile: sides.includes(from) });
    releaseHome(state, live.id);
  }
  // R563: what is held now on a restored side is let go; the cards go back whatever is Locked now.
  state.reserved = state.reserved.filter((zone) => !sides.includes(zone.player));
  state.homes = (state.homes ?? []).filter((home) => !sides.includes(home.zone.player));
  const locksBefore = copyOf(sides.map((player) => state.players[player].locks));
  for (const player of sides) {
    const { units, backrow } = state.players[player].locks;
    state.players[player].locks = { units: units.map(() => false), backrow: backrow.map(() => false) };
  }
  for (const player of sides) {
    for (const ref of zonesOf(player)) {
      const zone = placements.filter((p) => p.ref.player === ref.player && p.ref.row === ref.row && p.ref.lane === ref.lane);
      const entered: GameEvent[] = [];
      zone.reverse().forEach((placement, at) => {
        const { card } = placement;
        const brittle = card.brittle;
        // R227: going face-down from anywhere but a face-down zone, it takes a fresh id as it goes.
        const formerId = ref.row === "backrow" && isFaceDown(state, card) && !placement.wasFaceDown ? freshFaceDownId(state, card) : undefined;
        // Every restored zone is empty and open now; only a carried Unit whose carrier is mid-play
        // (left out above) can find no place, and it goes to its controller's hand as step 1's cards do.
        if (!placeOnField(state, card, ref, { stack: at > 0 })) {
          bounceCard(sink, card);
          return;
        }
        // The snapshot's Brittle count exactly: placement starts a printed one on a card with none (R385).
        if (brittle === undefined) delete card.brittle;
        if (!stampTurnState(sink, placement, formerId !== undefined)) return;
        entered.unshift({ type: "controlChanged", instanceId: card.id, controller: ref.player, row: ref.row, lane: ref.lane, ...(formerId === undefined ? {} : { formerId }) });
      });
      sink.events.push(...entered);
    }
  }
  // R563: the snapshot's own holds come back for the cards standing there again. Read `state.homes`
  // afresh: the hand bounce above releases homes, which deletes the field when none is left.
  const regained = sides.flatMap((player) => snapshot.sides[player].homes ?? []).filter((home) => findInstance(state, home.instanceId)?.zone.z === "field");
  const homes = [...(state.homes ?? []), ...regained];
  if (homes.length === 0) delete state.homes;
  else state.homes = homes;

  // Step 3.
  sides.forEach((player, at) => {
    const after = snapshot.sides[player].locks;
    state.players[player].locks = after;
    for (const ref of zonesOf(player)) {
      const now = after[ref.row][ref.lane - 1] === true;
      if (now !== (locksBefore[at]?.[ref.row][ref.lane - 1] === true)) {
        sink.events.push({ type: now ? "locked" : "unlocked", player, row: ref.row, lane: ref.lane });
      }
    }
  });
  return back;
}

/**
 * The verb C+ #35 casts (R419): return the board to the snapshot `turnsAgo` names, on the side of the
 * card's controller ("self"), the other ("enemy") or both.
 */
export function rollBack(args: { turnsAgo: number; sides: "self" | "enemy" | "both" }): Effect {
  return {
    kind: "rollBack",
    apply(ctx): void {
      const sides = PLAYER_IDS.filter((player) => args.sides === "both" || (player === ctx.controller) === (args.sides === "self"));
      restoreBoard(ctx, ctx.controller, args.turnsAgo, sides);
    },
  };
}
