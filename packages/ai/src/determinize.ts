// R185: one concrete world consistent with what the seat knows. `redact` has already turned every
// hidden card into a placeholder; this fills each one with a real non-token definition of any set
// (R380) drawn from what the opponent has not shown, shuffles the seat's own library, and gives the world a seed
// of its own, so no simulation can foresee a real draw or a real coin flip.
//
// docs/polish/3-ai.md's six steps, in order. `rng` is the AI's own stream and every draw below comes
// from it in a fixed order, so the same public state and the same rng give the same world.

import type { PlayerId } from "@jackioh/shared";
import { PLAYER_IDS, opponentOf } from "@jackioh/shared";
import {
  activeUnitsOf,
  cloneState,
  query,
  scriptsFor,
  unitView,
  type CardInstance,
  type GameState,
  type Rng,
} from "@jackioh/engine";
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
 * with replacement from the whole pool once that is empty.
 */
function sampleDef(
  pool: readonly string[],
  seen: ReadonlySet<string>,
  sampled: Set<string>,
  rng: Rng,
): string {
  const open = pool.filter((id) => !seen.has(id) && !sampled.has(id));
  if (open.length > 0) {
    const pick = open[rng.int(open.length)] as string;
    sampled.add(pick);
    return pick;
  }
  return pool[rng.int(pool.length)] ?? HIDDEN_DEF_ID;
}

function isPlaceholder(card: CardInstance): boolean {
  return card.defId === HIDDEN_DEF_ID;
}

/** Every unit's Attack, Health and keywords as the board shows them, one string per unit. */
function shownUnits(state: GameState): string {
  return PLAYER_IDS.flatMap((player) =>
    activeUnitsOf(state, player).map((unit) => {
      const view = unitView(state, unit);
      return `${unit.id}:${view.attack}/${view.maxHealth}/${JSON.stringify(view.keywords)}`;
    }),
  ).join("|");
}

/** Whether either face of a def projects an aura, which a face-down Trap does from the moment it is set (R403). */
function hasAura(defId: string): boolean {
  const { base, radiant } = scriptsFor(defId);
  return base.aura !== undefined || radiant.aura !== undefined;
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

  // Step 3: face-down backrow placeholders, in lane order, from the Trap and Field Trap pool. R602: a
  // face-down Trap's aura is live (R403) and the units it changes are on the board for the seat to read,
  // so a candidate whose aura would change any unit's shown stats is not in the pool for that card.
  const trapPool = query({ type: ["Trap", "Field Trap"] }).map((def) => def.id);
  const auraTraps = trapPool.filter(hasAura);
  const shown = auraTraps.length > 0 ? shownUnits(next) : "";
  const trapFor = (card: CardInstance): string => {
    const agrees = (defId: string): boolean => {
      card.defId = defId;
      const same = shownUnits(next) === shown;
      card.defId = HIDDEN_DEF_ID;
      return same;
    };
    const open = auraTraps.length === 0 ? trapPool : trapPool.filter((id) => !auraTraps.includes(id) || agrees(id));
    return sampleDef(open.length > 0 ? open : trapPool, seen, sampled, rng);
  };
  for (const side of [opp, seat] as const) {
    // B5 E21: then the face-down cards dormant under each backrow pile, lane by lane.
    const backrow = [...next.players[side].backrow, ...(next.players[side].backrowPiles ?? []).flat()];
    for (const card of backrow) {
      if (card !== null && isPlaceholder(card)) card.defId = trapFor(card);
    }
    // R448: a card being set face-down waits in the resolving zone as a placeholder; it is a trap too.
    for (const card of next.players[side].resolving) {
      if (isPlaceholder(card)) card.defId = trapFor(card);
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
