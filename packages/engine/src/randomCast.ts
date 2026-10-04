// Random casts and casts that target enemies (docs/classic-sets.md B5 E12, E39, R452).
//
// A cast (R70) is §10.5's pipeline entered at step 3 (`playSteps.castThroughPipeline`). Two things
// can change how its choices are made:
//
//  - a random cast (Classic+ #47 Jogg's Box, #38.1 Solarius-Prime) makes every choice its caster
//    would make at random: its declared targets and modes, its Echo repeats' fresh picks, and every
//    prompt opened for its caster while it resolves — Discover picks included — answered at once from
//    the match rng, so its caster is never asked. Its X is the caster's current mana, at least 1, as
//    every cast's is. A cast made while it resolves is random too, and the whole chain is capped
//    (RANDOM_CAST_CHAIN_CAP). The other player's prompts are theirs and are asked as usual.
//  - a cast that targets enemies when it can (Solarius-Prime's "They target enemies when they can",
//    the `targetEnemies` enchantment Classic+ #40 Appropriations gives its Books, E39) narrows each
//    target pick to the enemies among its options when there is one: its declared targets, its Echo
//    repeats' and the prompts its own text opens for its caster.
//
// While such a cast's steps run, its mode sits on `state.castsResolving` (`withCastMode`), which is
// what `prompts.openPrompt` reads to answer or narrow a prompt. The stack is transient: a step that
// pauses returns through `withCastMode`, which takes the mode off again, so a state at rest — a paused
// one included — never carries it, and the owed step puts it back when it is driven again (the run
// record keeps `random` and `targetEnemies`, which are JSON).

import type { PlayerId, Selection } from "@jackioh/shared";
import { opponentOf } from "@jackioh/shared";
import { RANDOM_CAST_CHAIN_CAP } from "./config";
import type { Rng } from "./rng";
import { findInstance, type CastMode, type GameState } from "./state";

/** The casts whose modes are in force now, outermost first. */
export function castModesOf(state: GameState): readonly CastMode[] {
  return state.castsResolving ?? [];
}

/**
 * Run `body` with a cast's mode in force, and take it off again whatever `body` does — a pause, the
 * end of the game — so the stack is empty whenever the state is at rest. `mode` null runs `body` alone.
 */
export function withCastMode<T>(state: GameState, mode: CastMode | null, body: () => T): T {
  if (mode === null) return body();
  state.castsResolving = [...castModesOf(state), mode];
  try {
    return body();
  } finally {
    // By value, not by reference: an AI playout inside the cast (R44) adopts a cloned state.
    const stack = [...castModesOf(state)];
    const at = stack.map((entry) => entry.instanceId).lastIndexOf(mode.instanceId);
    if (at >= 0) stack.splice(at, 1);
    if (stack.length === 0) delete state.castsResolving;
    else state.castsResolving = stack;
  }
}

/** R452: the innermost random cast of this player's being driven now, or null. */
export function randomCastOf(state: GameState, player: PlayerId): CastMode | null {
  const stack = castModesOf(state);
  for (let at = stack.length - 1; at >= 0; at -= 1) {
    const entry = stack[at];
    if (entry !== undefined && entry.player === player && entry.random) return entry;
  }
  return null;
}

/** R452: the outermost random cast of this player's — the one whose chain every inner cast counts in. */
function rootRandomCast(state: GameState, player: PlayerId): CastMode | null {
  return castModesOf(state).find((entry) => entry.player === player && entry.random) ?? null;
}

/**
 * R452: whether one more cast may begin under this player's random cast chain — always, when none is
 * running; otherwise while the chain has made fewer than RANDOM_CAST_CHAIN_CAP casts. A cast verb asks
 * this before it makes or moves the card, and a cast it refuses resolves into nothing (R28's shape).
 */
export function mayCastNow(state: GameState, player: PlayerId): boolean {
  const root = rootRandomCast(state, player);
  return root === null || root.casts < RANDOM_CAST_CHAIN_CAP;
}

/** R452: count one more cast in this player's random cast chain, when one is running. */
export function countChainCast(state: GameState, player: PlayerId): void {
  const root = rootRandomCast(state, player);
  if (root !== null) root.casts += 1;
}

/**
 * R452: how a prompt opened for `player` is to be answered while casts are being driven: at random
 * when a random cast of theirs is (with that cast's enemy preference), narrowed to enemies when the
 * prompt is the own text of a cast of theirs that targets enemies (`resumeInstanceId`), else null.
 */
export function castModeForPrompt(
  state: GameState,
  player: PlayerId,
  resumeInstanceId: string | undefined,
): { random: boolean; targetEnemies: boolean } | null {
  const random = randomCastOf(state, player);
  if (random !== null) return { random: true, targetEnemies: random.targetEnemies };
  const own = castModesOf(state).find(
    (entry) => entry.player === player && entry.targetEnemies && entry.instanceId === resumeInstanceId,
  );
  return own === undefined ? null : { random: false, targetEnemies: true };
}

// ---------------------------------------------------------------------------
// Enemies (R452)
// ---------------------------------------------------------------------------

/** A pick that names a card or a hero, which "target enemies" can narrow; a mode or a zone it cannot. */
export function isTargetPick(selection: Selection): boolean {
  return selection.pick === "instance" || selection.pick === "hero";
}

/**
 * R452: whether a pick names one of `chooser`'s enemies — the other player's hero, a card they
 * control on the field, or a card in one of their piles.
 */
export function isEnemyPick(state: GameState, chooser: PlayerId, selection: Selection): boolean {
  const enemy = opponentOf(chooser);
  if (selection.pick === "hero") return selection.player === enemy;
  if (selection.pick !== "instance") return false;
  const card = findInstance(state, selection.instanceId);
  if (card === undefined) return false;
  return card.zone.z === "field" ? card.controller === enemy : card.zone.player === enemy;
}

/**
 * R452: "target enemies when possible" — the options narrowed to the enemies among the target picks
 * (a mode or a zone option is kept as it is), when there is an enemy to pick and enough of them for
 * the `required` picks; otherwise every option, since a friendly target is then the legal one.
 */
export function preferEnemies<T>(
  state: GameState,
  chooser: PlayerId,
  options: readonly T[],
  selectionOf: (option: T) => Selection,
  required: number,
): T[] {
  if (!options.some((option) => isEnemyPick(state, chooser, selectionOf(option)))) return [...options];
  const narrowed = options.filter((option) => {
    const selection = selectionOf(option);
    return !isTargetPick(selection) || isEnemyPick(state, chooser, selection);
  });
  return narrowed.length >= required ? narrowed : [...options];
}

/**
 * R651: whether a pick names one of `chooser`'s friendly targets — the chooser's own hero,
 * a card they control on the field, or a card in one of their piles.
 */
export function isFriendlyPick(state: GameState, chooser: PlayerId, selection: Selection): boolean {
  if (selection.pick === "hero") return selection.player === chooser;
  if (selection.pick !== "instance") return false;
  const card = findInstance(state, selection.instanceId);
  if (card === undefined) return false;
  return card.zone.z === "field" ? card.controller === chooser : card.zone.player === chooser;
}

/**
 * R651: "target allies when beneficial" — the mirror of `preferEnemies`. Under a random cast that
 * targets enemies, a declaration with `aim: "help"` narrows to friendly targets among the target picks
 * when there is a friendly target to pick and enough of them for `required`; otherwise every option.
 */
export function preferFriends<T>(
  state: GameState,
  chooser: PlayerId,
  options: readonly T[],
  selectionOf: (option: T) => Selection,
  required: number,
): T[] {
  if (!options.some((option) => isFriendlyPick(state, chooser, selectionOf(option)))) return [...options];
  const narrowed = options.filter((option) => {
    const selection = selectionOf(option);
    return !isTargetPick(selection) || isFriendlyPick(state, chooser, selection);
  });
  return narrowed.length >= required ? narrowed : [...options];
}

/**
 * R452: a uniformly random set of between `low` and `high` of `items` (the size first, then the set),
 * kept in the order they were offered — a declaration's picks are a set, taken in offered order (R221).
 */
export function randomPicks<T>(rng: Rng, items: readonly T[], low: number, high: number): T[] {
  const top = Math.max(0, Math.min(high, items.length));
  const bottom = Math.max(0, Math.min(low, top));
  const size = bottom + rng.int(top - bottom + 1);
  const chosen = new Set(rng.shuffle(items.map((_, index) => index)).slice(0, size));
  return items.filter((_, index) => chosen.has(index));
}
