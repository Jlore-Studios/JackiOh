// A finished game's record for the card statistics (SPEC §9.11, R376): which cards each seat started
// with, drew and played, and how the game ended, read off a replay of `(seed, decks, log)`.
//
// The state keeps no such history — `turnLog` is one turn's, and `applied` keeps only the last
// actions' events — so this folds the log exactly as `replay.fold` does and reads the raw events each
// step emits, which no view has redacted. It runs only where the whole game is already known: the
// server once a match has its result, and the AI's development runs.
//
// What "played" means needs one bookkeeping device. `cardPlayed` is emitted for a play from hand and
// for a cast alike (§2.4's cast on draw, R70), and only the first is the player's play. So each
// seat's hand is followed through the events: the hand the state holds as an action begins, plus
// every `addedToHand`, minus every card that leaves it. A `cardPlayed` naming a card in that hand is
// a play from hand; a cast names a card that never entered it. A play countered in its announce
// window has no `cardPlayed` (§10.5, R448), so it is none; a play step 3 replaced (R449) is followed
// by its `transformed` to the card it resolves as, the one its `cardPlayed` names.

import { PLAYER_IDS, type GameEvent, type GameSummary, type PlayerId, type SeatSummary } from "@jackioh/shared";
import { beginGame, reduce } from "./reduce";
import type { ReplayInput } from "./replay";
import { createGame, type GameState } from "./state";

/** Each seat's hand as instance id → catalog id, in hand order. */
type Hands = Record<PlayerId, Map<string, string>>;

type Reading = {
  hands: Hands;
  /** The seat of the first `turnStarted`, and both hands at that moment; null until a turn has begun. */
  first: PlayerId | null;
  opening: Record<PlayerId, string[]> | null;
  drawn: Record<PlayerId, string[]>;
  played: Record<PlayerId, string[]>;
};

/**
 * Each seat's hand as an action begins: the cards the state holds there, and those of `following`
 * (the hands the last action ended with) that a play has taken to the resolving zone and not yet
 * placed. A prompt can hold a play there past its action (§10.5: the choices of the card step 3 put
 * in its place, R449, or the announce window, C #4 Palantir), and its `cardPlayed` comes in a later one.
 */
function handsOf(state: GameState, following?: Hands): Hands {
  const hand = (player: PlayerId): Map<string, string> => {
    const cards = new Map(state.players[player].hand.map((card) => [card.id, card.defId]));
    for (const card of state.players[player].resolving) {
      if (following?.[player].has(card.id) === true) cards.set(card.id, card.defId);
    }
    return cards;
  };
  return { p1: hand("p1"), p2: hand("p2") };
}

function perSeat(): Record<PlayerId, string[]> {
  return { p1: [], p2: [] };
}

/** A card leaves whichever hand holds it. */
function leave(hands: Hands, instanceId: string): void {
  for (const player of PLAYER_IDS) hands[player].delete(instanceId);
}

/** One step's events, in the order they happened. */
function read(reading: Reading, events: readonly GameEvent[]): void {
  const { hands } = reading;
  events.forEach((event, at) => {
    switch (event.type) {
      case "turnStarted":
        // §2.1: the mulligans have resolved and The Coin is dealt; the first turn's draw comes next.
        if (reading.opening === null) {
          reading.first = event.player;
          reading.opening = { p1: [...hands.p1.values()], p2: [...hands.p2.values()] };
        }
        return;
      case "drawn": {
        // §2.4: a draw that reached the hand is followed at once by its `addedToHand`; a burned
        // card's `burned` or a cast's `cardPlayed` follows instead. The opening hand's draws are the
        // opening hand's, not draws of the game.
        const next = events[at + 1];
        if (reading.opening !== null && next?.type === "addedToHand" && next.instanceId === event.instanceId) {
          reading.drawn[event.player].push(event.defId);
        }
        return;
      }
      case "addedToHand":
        hands[event.player].set(event.instanceId, event.defId);
        return;
      case "cardPlayed": {
        // R227: a Trap set face-down took a fresh id; the hand knew it by the old one.
        const id = event.formerId ?? event.instanceId;
        const hand = hands[event.player];
        if (!hand.has(id)) return;
        reading.played[event.player].push(event.defId);
        hand.delete(id);
        return;
      }
      case "transformed":
        for (const player of PLAYER_IDS) {
          const hand = hands[player];
          if (!hand.has(event.instanceId)) continue;
          hand.delete(event.instanceId);
          hand.set(event.newInstanceId, event.toDefId);
        }
        return;
      case "fused":
        for (const id of event.instanceIds) leave(hands, id);
        return;
      case "shuffledIn":
      case "discarded":
      case "exiled":
      case "bounced":
      case "enteredGraveyard":
        leave(hands, event.instanceId);
        return;
      default:
        return;
    }
  });
}

function seatSummary(reading: Reading, decks: ReplayInput["decks"], player: PlayerId, seat: number): SeatSummary {
  return {
    deck: [...(decks[seat] ?? [])],
    opening: reading.opening?.[player] ?? [],
    drawn: reading.drawn[player],
    played: reading.played[player],
  };
}

/**
 * R376: what each seat's cards did in a finished game, and how it ended. The fold is `replay.fold`'s,
 * so an action the log holds and the engine refuses is skipped as it is there. Null when the log
 * leaves the game without a result: a record is only ever made of a finished game.
 */
export function summarizeGame(input: ReplayInput): GameSummary | null {
  const start = createGame({
    seed: input.seed,
    decks: input.decks,
    ...(input.catalog === undefined ? {} : { catalog: input.catalog }),
    ...(input.handicaps === undefined ? {} : { handicaps: input.handicaps }),
  });
  const reading: Reading = { hands: handsOf(start), first: null, opening: null, drawn: perSeat(), played: perSeat() };

  const begun = beginGame(start);
  read(reading, begun.events);
  let state = begun.state;

  for (const action of input.log) {
    // The state's own hands as the action begins, so nothing the events do not spell out (a card
    // whose control changed, say) carries from one action into the next, but a play still in flight.
    reading.hands = handsOf(state, reading.hands);
    const result = reduce(state, action);
    if (result.error !== undefined) continue;
    read(reading, result.events);
    state = result.state;
  }

  if (state.result === null) return null;
  return {
    // §2.1: Player 1 takes the first turn. A game that ended before any turn began names it too.
    first: reading.first ?? (PLAYER_IDS[0] as PlayerId),
    winner: state.result.winner,
    reason: state.result.reason,
    turns: state.turn,
    seats: {
      p1: seatSummary(reading, input.decks, "p1", 0),
      p2: seatSummary(reading, input.decks, "p2", 1),
    },
  };
}
