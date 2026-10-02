// R185: one concrete world consistent with what the seat knows. `redact` has already turned every
// hidden card into a placeholder; this fills each one with a real non-token definition of any set
// (R380) drawn from what the opponent has not shown, shuffles the seat's own library, and gives the world a seed
// of its own, so no simulation can foresee a real draw or a real coin flip.
//
// docs/polish/3-ai.md's six steps, in order. `rng` is the AI's own stream and every draw below comes
// from it in a fixed order, so the same public state and the same rng give the same world.

import type { PlayerId } from "@jackioh/shared";
import { PLAYER_IDS, opponentOf } from "@jackioh/shared";
import { activeUnitsOf, cloneState, query, unitView, type CardInstance, type GameState, type Rng } from "@jackioh/engine";
import { AI_DETERMINIZE } from "./config";
import { HIDDEN_DEF_ID } from "./observe";

function allCards(state: GameState): CardInstance[] {
  const out: CardInstance[] = [];
  for (const player of PLAYER_IDS) {
    const side = state.players[player];
    out.push(...side.hand, ...side.library, ...side.graveyard, ...side.exile, ...side.resolving);
    for (const pile of side.units) if (pile !== null) out.push(...pile);
    for (const card of side.backrow) if (card !== null) out.push(card);
    // B5 E21, R446: a backrow pile's dormant cards and a carrier's Unit are on the board too.
    for (const pile of side.backrowPiles ?? []) out.push(...pile);
    for (const card of side.carried ?? []) if (card !== null) out.push(card);
  }
  return out;
}

/**
 * One def id, uniformly, from `pool` minus `seen` minus what this determinization already sampled;
 * with replacement from the whole pool once that is empty. `fits` narrows the pool to the
 * definitions the seat's observations allow (R602), both times.
 */
function sampleDef(
  pool: readonly string[],
  seen: ReadonlySet<string>,
  sampled: Set<string>,
  rng: Rng,
  fits: (id: string) => boolean = () => true,
): string {
  const open = pool.filter((id) => !seen.has(id) && !sampled.has(id) && fits(id));
  if (open.length > 0) {
    const pick = open[rng.int(open.length)] as string;
    sampled.add(pick);
    return pick;
  }
  const any = pool.filter(fits);
  return any[rng.int(any.length)] ?? HIDDEN_DEF_ID;
}

/** R602: every acting unit's Attack and Health as the board shows them, in board order. */
function shownStats(state: GameState): string {
  return PLAYER_IDS.flatMap((player) =>
    activeUnitsOf(state, player).map((unit) => {
      const view = unitView(state, unit);
      return `${unit.id}:${String(view.attack)}/${String(view.health)}/${String(view.maxHealth)}`;
    }),
  ).join(",");
}

function isPlaceholder(card: CardInstance): boolean {
  return card.defId === HIDDEN_DEF_ID;
}

/** R185: one concrete world consistent with `publicState` (the output of redact). Pure given rng. */
export function determinize(publicState: GameState, seat: PlayerId, rng: Rng): GameState {
  const opp = opponentOf(seat);
  const next = cloneState(publicState);

  // Step 1: a stream the match never uses.
  next.seed = `ai:${rng.int(2 ** 31)}`;
  next.rngCursor = 0;

  // Step 2: what the opponent has shown, in any zone.
  const seen = new Set<string>();
  for (const card of allCards(next)) {
    if (card.owner === opp && !isPlaceholder(card)) seen.add(card.defId);
  }
  const sampled = new Set<string>();

  // Step 3: face-down backrow placeholders, in lane order, from the Trap and Field Trap pool.
  const trapPool = query({ type: ["Trap", "Field Trap"] }).map((def) => def.id);
  // R602, R403: a face-down trap's aura is live, and `redact` already left every unit the stats the
  // board shows, so a placeholder on the board may only become a trap that changes none of them —
  // a world where it shrinks the seat's units is not one the seat could be in.
  const board = shownStats(next);
  const keepsBoard =
    (card: CardInstance) =>
    (id: string): boolean => {
      card.defId = id;
      const same = shownStats(next) === board;
      card.defId = HIDDEN_DEF_ID;
      return same;
    };
  for (const side of [opp, seat] as const) {
    // B5 E21: then the face-down cards dormant under each backrow pile, lane by lane.
    const backrow = [...next.players[side].backrow, ...(next.players[side].backrowPiles ?? []).flat()];
    for (const card of backrow) {
      if (card !== null && isPlaceholder(card)) card.defId = sampleDef(trapPool, seen, sampled, rng, keepsBoard(card));
    }
    // R448: a card being set face-down waits in the resolving zone as a placeholder; it is a trap too.
    for (const card of next.players[side].resolving) {
      if (isPlaceholder(card)) card.defId = sampleDef(trapPool, seen, sampled, rng);
    }
  }

  // Step 4: the opponent's hand, then its library, in (sorted) order, from the non-token pool of
  // every set.
  const pool = query({ excludeDefId: [...AI_DETERMINIZE.excludeDefIds] }).map((def) => def.id);
  for (const card of next.players[opp].hand) {
    if (isPlaceholder(card)) card.defId = sampleDef(pool, seen, sampled, rng);
  }
  for (const card of next.players[opp].library) {
    if (isPlaceholder(card)) card.defId = sampleDef(pool, seen, sampled, rng);
  }

  // Step 5: the seat's own library — the opponent's cards in it sampled as in step 4, then shuffled.
  for (const card of next.players[seat].library) {
    if (isPlaceholder(card)) card.defId = sampleDef(pool, seen, sampled, rng);
  }
  next.players[seat].library = rng.shuffle(next.players[seat].library);

  // Step 6: every sampled card kept its instance id, owner, controller and zone above.
  return next;
}
