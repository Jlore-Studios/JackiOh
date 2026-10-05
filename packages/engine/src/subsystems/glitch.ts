// The Glitch Easter egg (issue #170; SPEC §7, R673–R679).
//
//   - `countSystemPlay` (R673): a play of a "… in the System" card (`SYSTEM_CARD_DEF_IDS`), by either
//     player, adds one to `state.systemPlays`, which `catalog.pickGenerated` reads: every card
//     generated into a hand or a deck after that is Glitch with odds n/10000.
//   - `glitch` (R676): Glitch's own text. One draw of the match rng picks one of `GLITCH_OUTCOMES`,
//     and a public `glitched` event names it:
//       reset  — the match starts again from `createGame`'s decks, shuffled and dealt by the match
//                rng, mulligans and all (`resetMatch`, run by `reduce` once the action has settled,
//                so nothing of the old game is still resolving when it goes);
//       swap   — each account now plays the other seat (R677). The engine's game is unchanged; the
//                hosts read `state.seatSwaps` (`seatSwapped`), and the server credits results by it;
//       boards — both fields become the boards of two other players' games, a frozen setup input
//                like C+ #29's last boards (`state.glitchBoards`, R678); hands, decks and life stay;
//       void   — the game ends with no winner and reason `voided`, and the server keeps no trace of
//                it but a log line (R679).
//
// All of it is plain data on the state, so `(seed, decks, …, log)` folds to the same game (§9.3).

import type { PlayerId } from "@jackioh/shared";
import { PLAYER_IDS } from "@jackioh/shared";
import { defOf, selfDefIds } from "../catalog";
import { GLITCH_OUTCOMES, SETUP_TURN, SYSTEM_CARD_DEF_IDS } from "../config";
import { endGame } from "../gameOver";
import type { EngineSink } from "../resolve";
import type { Effect } from "../script";
import { beginSetup } from "../setup";
import { createGameForReset, newInstance, type CardInstance, type GameState } from "../state";
import { ceaseToExist, firstFreeZone, placeOnField, slotsOf, unlockZone, zoneContents } from "../zones";
import { rebuildFusedDef } from "./fuse";

/** R673: count one play of `card` if it is a "… in the System" card (a fused one counts once). */
export function countSystemPlay(state: GameState, card: CardInstance): void {
  if (!selfDefIds(card.defId).some((id) => SYSTEM_CARD_DEF_IDS.includes(id))) return;
  state.systemPlays = (state.systemPlays ?? 0) + 1;
}

/** R677: whether the accounts now hold each other's seat — an odd number of swaps. */
export function seatsSwapped(state: Pick<GameState, "seatSwaps">): boolean {
  return (state.seatSwaps ?? 0) % 2 === 1;
}

/** R677: the seat the account that began the match in `seat` plays now. */
export function seatPlayedBy(state: Pick<GameState, "seatSwaps">, seat: PlayerId): PlayerId {
  if (!seatsSwapped(state)) return seat;
  return seat === "p1" ? "p2" : "p1";
}

/** R676: Glitch's text — one of its four outcomes, drawn by the match rng. */
export function glitch(): Effect {
  return {
    kind: "glitch",
    apply(ctx): void {
      const outcome = GLITCH_OUTCOMES[ctx.rng.int(GLITCH_OUTCOMES.length)] ?? "reset";
      ctx.events.push({ type: "glitched", player: ctx.controller, outcome });
      if (outcome === "reset") ctx.state.resetOwed = true;
      else if (outcome === "swap") ctx.state.seatSwaps = (ctx.state.seatSwaps ?? 0) + 1;
      else if (outcome === "boards") placeGlitchBoards(ctx.state);
      else endGame({ state: ctx.state, events: ctx.events, rng: ctx.rng }, "draw", "voided");
    },
  };
}

/**
 * R678: every card on both fields ceases to exist (no Death, no graveyard, R11's way out), the Locks
 * go, and each side takes its frozen other game's board in order: its Units into the unit zones and
 * the rest into the backrow, left to right, until a row is full. Each card is a new one its side owns,
 * on its entry's face, as having entered this turn (R171). Nothing is summoned or played, so nothing
 * triggers. A side with no board frozen is left empty.
 */
function placeGlitchBoards(state: GameState): void {
  for (const player of PLAYER_IDS) {
    for (const row of ["units", "backrow"] as const) {
      for (const ref of slotsOf(player, row)) {
        for (const card of zoneContents(state, ref)) ceaseToExist(state, card);
        unlockZone(state, ref);
      }
    }
  }
  for (const player of PLAYER_IDS) {
    for (const entry of state.glitchBoards?.[player] ?? []) {
      if (rebuildFusedDef(state, entry.defId, player) === null) continue;
      // Created only once a zone is free, so a full row creates nothing.
      const card = newInstance(state, entry.defId, player, { z: "resolving", player });
      card.radiant = entry.radiant;
      const type = defOf(state, entry.defId).type;
      if (type === "Spell") continue;
      const ref = firstFreeZone(state, player, type === "Unit" ? "units" : "backrow");
      if (ref === null || !placeOnField(state, card, ref)) continue;
      card.summonedTurn = state.turn;
      if (type === "Field Spell") card.faceUp = true;
    }
  }
}

/**
 * R676: the reset a Glitch owed, once its action has settled. The state becomes a new game made from
 * the decks the match began with — its seats' handicaps, last boards and Glitch boards, the seat
 * swaps and the nonce log kept — with fresh ids, numbered from where the old game stopped by a stream
 * of this reset's own (R223), and setup runs again on the match rng. A state that keeps no record of
 * its opening (one the AI redacted) does nothing.
 */
export function resetMatch(sink: EngineSink): void {
  const old = sink.state;
  delete old.resetOwed;
  if (old.result !== null || old.opening === undefined) return;
  const resets = (old.resets ?? 0) + 1;
  const handicaps: Partial<Record<PlayerId, NonNullable<GameState["players"]["p1"]["handicap"]>>> = {};
  for (const player of PLAYER_IDS) {
    const handicap = old.players[player].handicap;
    if (handicap !== undefined) handicaps[player] = handicap;
  }
  const fresh = createGameForReset(
    {
      seed: old.seed,
      decks: old.opening.decks,
      handicaps,
      ...(old.opening.dealt === undefined ? {} : { dealt: old.opening.dealt }),
    },
    { nextId: old.nextId, resets },
  );
  fresh.nextSeq = old.nextSeq;
  fresh.applied = old.applied;
  fresh.resets = resets;
  if (old.lastBoards !== undefined) fresh.lastBoards = old.lastBoards;
  if (old.glitchBoards !== undefined) fresh.glitchBoards = old.glitchBoards;
  if (old.seatSwaps !== undefined) fresh.seatSwaps = old.seatSwaps;
  fresh.turn = SETUP_TURN;
  for (const key of Object.keys(old)) delete (old as Record<string, unknown>)[key];
  Object.assign(old, fresh);
  beginSetup(sink);
}
