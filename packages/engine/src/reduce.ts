// reduce(state, action, rng) and legalActions (SPEC §9.3, §10.2, §10.5). The reducer is pure: it
// clones the state, applies the action, and returns the new state with the events it produced.
// Illegal actions come back as an error with the state untouched.
//
// This file is the action layer and nothing else. Every rule it needs belongs to a module that owns
// it, and the two directions of each rule — what `legalActions` offers and what an action is
// refused for — come from the same place, so a client's greyed-out button and the reducer's error
// can never disagree (§9.3):
//
//   play             → `playSteps.runPlaySteps` (§10.5's eight steps), listed by
//                      `playChoices.playActionsFor` (R81, R90) for a hand card and by
//                      `playChoices.graveyardPlayActionsFor` for a graveyard card a permission
//                      lets its player play (E11, R454). Step 1's validation is
//                      `playSteps.validatePlay`, which asks `playChoices.whyChoicesRefused` for
//                      the zone, X, embiggen, Tribute, target and mode refusals (R90) before it
//                      reads the cost, so the refusal a client's greyed-out button comes from and
//                      the refusal this reducer returns are the same call. Nothing of that rule is
//                      restated here; a second copy is what would let the two disagree.
//   attack           → `combat.declareAttack`, listed by `combat.attackTargets` (§4.2). That call
//                      is §4.2 steps 1 to 5 whole, step 4's trap window included, so an `attack`
//                      can come back with a prompt open and the combat still owed on `state.work`
//                      (R113): the answer action finishes it, exactly as it finishes a Cry.
//   switchPosition   → `combat.switchPosition` (§4.1, R20, R49)
//   activate         → `subsystems/activate.activateAbility`, listed by `activateActionsFor` (B3.2,
//                      R384); on a Heroic Power, `subsystems/heroPower.activatePower` (R43)
//   activatePower    → the alias of `activate` every old log carries: the same routing (R384)
//   answer           → `prompts.answerPrompt`, which hands a prompt the play pipeline opened itself
//                      to that pipeline's answerer (R122)
//   mulligan         → `setup.answerMulligan`, refused by `setup.whyMulliganRefused` (§2.1, R265)
//   draws, concede, endTurn, the turn cap → `turn.ts` (§2.2, §2.5, R36)
//   setAutoEndTurn   → the sender's own `autoEndTurn`, read by `endDueTurns` below (R82, R345)
//
// After the action the resolution loop of §10.3 runs (`triggers.settle`): it dispatches the events
// the action emitted, drains whatever a prompt left owed in `state.work`, runs the state check and
// pops the trigger queue until nothing is left or a prompt stops it. Then a turn an effect has cut
// short ends (B5 E10, R456), and a turn with nothing left to do ends by itself (R82).

import type { Action, ActionBody, ActionType, GameEvent, PlayerId } from "@jackioh/shared";
import { NON_ACTIVE_ACTION_TYPES, PROMPT_OPEN_ACTION_TYPES, opponentOf } from "@jackioh/shared";
import { attackTargets, declareAttack, hasExertion, switchPosition, type AttackTarget } from "./combat";
import { NONCE_HISTORY, TIMEOUT_ANSWER_CAP, TURN_CAP_PLAYER_TURNS } from "./config";
import { endGame } from "./gameOver";
import { runPlaySteps } from "./playSteps";
import { graveyardPlayActionsFor, playActionsFor } from "./playChoices";
import { answerPrompt, promptAnswers } from "./prompts";
import { createRng, type Rng } from "./rng";
import type { EngineSink } from "./resolve";
import { answerMulligan, beginSetup, mulliganOwed, mulliganPromptFor, whyMulliganRefused } from "./setup";
import { flagsOf } from "./scripts";
import { removeModifier, turnEndsOf } from "./modifiers";
import { cloneState, findInstance, type CardInstance, type GameState } from "./state";
import { activateAbility, activateActionsFor } from "./subsystems/activate";
import { playOutTurn } from "./subsystems/aiPolicy";
import { syncFusedScripts } from "./subsystems/fuse";
import { activatePower, powerOf, whyCannotActivate } from "./subsystems/heroPower";
import { resetMatch } from "./subsystems/glitch";
import { settle } from "./triggers";
import { answerDraw, canOfferDraw, concede, endTurn, hasStandingDrawOffer, offerDraw } from "./turn";
import { activeUnitsOf, cardAt, carriedUnitsOf, slotsOf } from "./zones";

export type ReduceResult = { state: GameState; events: GameEvent[]; error?: string };

const MAX_MULLIGAN_SUBSETS = 256;

/**
 * R265: what may be sent while the mulligans are open — a seat's own mulligan, and the actions that
 * end a game or answer for a seat whose clock ran out (R79, R268), exactly as while a prompt is open.
 */
const MULLIGAN_OPEN_ACTION_TYPES: readonly ActionType[] = [
  "mulligan",
  "concede",
  "setAutoEndTurn",
  "timeout",
  "disconnectExpired",
  "ceilingReached",
];

/**
 * The seat the game waits on first: the holder of the open prompt, else the first seat in seat order
 * that still owes its mulligan (R265), else the active player. A harness that plays both seats asks
 * this; a live table asks each seat's own `legalActions`, since both may owe a mulligan at once.
 */
export function seatToAct(state: GameState): PlayerId {
  return state.pending?.playerId ?? mulliganOwed(state)[0] ?? state.active;
}

/** §4.2: a target names an enemy unit by instance id, or an enemy hero as `hero-<player>`. */
export function attackTargetId(target: AttackTarget): string {
  return target.kind === "unit" ? target.instance.id : `hero-${target.player}`;
}

/**
 * The attacker an `attack` action names: any card in this player's unit piles, a dormant one
 * included, so that §3.2's "not on the field" is `combat`'s refusal to give rather than a lookup
 * failure here (R13).
 */
function attackerOf(state: GameState, player: PlayerId, instanceId: string): CardInstance | undefined {
  // R446: a Unit a carrier holds is one of the player's units too, which `combat` refuses to attack.
  return [...state.players[player].units.flatMap((pile) => pile ?? []), ...carriedUnitsOf(state, player)].find(
    (card) => card.id === instanceId,
  );
}

/**
 * The target an `attack` action names, looked up among the enemy's active units and the enemy hero
 * and nothing else, so a friendly target never reaches the validator (§4.2 step 2).
 */
function attackTargetOf(state: GameState, player: PlayerId, targetId: string): AttackTarget | null {
  const enemy = opponentOf(player);
  if (targetId === `hero-${enemy}`) return { kind: "hero", player: enemy };
  const unit = activeUnitsOf(state, enemy).find((card) => card.id === targetId);
  return unit === undefined ? null : { kind: "unit", instance: unit };
}

function attack(sink: EngineSink, player: PlayerId, action: Extract<ActionBody, { type: "attack" }>): string | null {
  const attacker = attackerOf(sink.state, player, action.attackerId);
  if (attacker === undefined) return `no unit ${action.attackerId} you control`;

  const target = attackTargetOf(sink.state, player, action.targetId);
  if (target === null) return `no target ${action.targetId}`;

  return declareAttack(sink, attacker, target).error ?? null;
}

/** §4.1: the player's own switch, which spends the unit's exertion (R20 is the effect's version). */
function switchAction(sink: EngineSink, player: PlayerId, instanceId: string): string | null {
  const unit = attackerOf(sink.state, player, instanceId);
  if (unit === undefined) return `no unit ${instanceId} you control`;
  return switchPosition(sink, unit).error ?? null;
}

type ActivationAction = Extract<ActionBody, { type: "activate" | "activatePower" }>;

/**
 * B3.2 rule 10, R384, R43: `activate` and its alias `activatePower`, one routing for both. A Heroic
 * Power's power is its own activation in v0.2.0 (R43), so an action on a card with a power that names
 * no ability of the card's own goes to `heroPower.activatePower` — every old log's `activatePower`
 * replays exactly — and everything else to `activate.activateAbility`, which answers for any card's
 * "Activate:" abilities.
 */
function activateCard(sink: EngineSink, player: PlayerId, action: ActivationAction): string | null {
  const card = findInstance(sink.state, action.instanceId);
  const named = action.type === "activate" ? action.ability : undefined;
  const modes = action.type === "activate" ? action.modes : undefined;
  const tributes = action.type === "activate" ? action.tributes : undefined;
  const discards = action.type === "activate" ? action.discards : undefined;
  if (card !== undefined && powerOf(card) !== null && named === undefined) {
    if ((modes?.length ?? 0) > 0) return "that power takes no mode choices";
    if ((tributes?.length ?? 0) > 0) return "that power needs no Tribute";
    if ((discards?.length ?? 0) > 0) return "that power takes no discards";
    return activatePower(sink, player, {
      instanceId: action.instanceId,
      ...(action.targets === undefined ? {} : { targets: action.targets }),
    });
  }
  return activateAbility(sink, player, {
    type: "activate",
    instanceId: action.instanceId,
    ...(named === undefined ? {} : { ability: named }),
    ...(action.targets === undefined ? {} : { targets: action.targets }),
    ...(modes === undefined ? {} : { modes }),
    ...(tributes === undefined ? {} : { tributes }),
    ...(discards === undefined ? {} : { discards }),
  });
}

/**
 * R43, R384: what `legalActions` offers for one card acting on the field — a Heroic Power's power as
 * the `activatePower` it has always been listed as, and every usable "Activate:" ability with its
 * choices (`activate.activateActionsFor`).
 */
function activationActions(state: GameState, player: PlayerId, card: CardInstance): ActionBody[] {
  const out: ActionBody[] = [];
  if (powerOf(card) !== null && whyCannotActivate(state, player, card.id) === null) {
    out.push({ type: "activatePower", instanceId: card.id });
  }
  out.push(...activateActionsFor(state, player, card));
  return out;
}

/** §4.1 and R49: whether this unit's own switch is on offer at all. */
function canSwitch(state: GameState, unit: CardInstance): boolean {
  if (!hasExertion(state, unit, "switch")) return false;
  // §4.1: Spikey Pillow can never be in Defense Position, so a unit in Attack has nowhere to go.
  return (unit.position ?? "ATK") === "DEF" || flagsOf(unit).neverDefense !== true;
}

function applyAction(sink: EngineSink, action: Action): string | null {
  const state = sink.state;

  switch (action.type) {
    case "mulligan": {
      // R265: each seat answers its own mulligan, in either order; the answer is sealed (R266).
      const refused = whyMulliganRefused(state, action.playerId, action.keep);
      if (refused !== null) return refused;
      answerMulligan(sink, action.playerId, action.keep);
      return null;
    }
    case "play":
      return runPlaySteps(sink, action.playerId, action);
    case "switchPosition":
      return switchAction(sink, action.playerId, action.instanceId);
    case "attack":
      return attack(sink, action.playerId, action);
    case "activate":
    case "activatePower":
      // R43, R384: a power or an ability lives on the instance, and the module that owns it owns
      // every part of using it — the costs, the choices, and that turn's use.
      return activateCard(sink, action.playerId, action);
    case "answer":
      // §10.6: a card's continuation is re-entered through its script; a prompt an engine sequence
      // opened for itself (an Echo repeat's fresh pick, §10.5 step 6) goes to that sequence's
      // answerer, which `answerPrompt` looks up (R122).
      return answerPrompt(sink, action);
    case "offerDraw": {
      if (!canOfferDraw(state, action.playerId)) return "you cannot offer a draw right now";
      offerDraw(sink, action.playerId);
      return null;
    }
    case "answerDraw": {
      if (!hasStandingDrawOffer(state, action.playerId)) return "there is no draw offer to answer";
      answerDraw(sink, action.playerId, action.accept);
      return null;
    }
    case "concede":
      concede(sink, action.playerId);
      return null;
    case "endTurn":
      endTurn(sink);
      return null;
    case "setAutoEndTurn":
      // R345: a preference, not a move. It emits nothing, and `maybeAutoEndTurn` reads it after
      // this reduction as after every other, so turning it back on with nothing left to do ends the
      // turn at once.
      if (action.enabled) delete state.players[action.playerId].autoEndTurn;
      else state.players[action.playerId].autoEndTurn = false;
      return null;
    case "timeout":
      return timeout(sink, action);
    case "disconnectExpired": {
      endGame(sink, opponentOf(action.player), "disconnect");
      return null;
    }
    case "ceilingReached": {
      // R79: past the hard wall-clock ceiling the match is a draw.
      endGame(sink, "draw", "match-ceiling");
      return null;
    }
    default:
      return "unknown action";
  }
}

/**
 * R79 and §2.5: `timeout` "answers only the prompts of the player whose clock ran out, using the AI
 * policy, and ends the turn only when that is the active player". The AI policy answers, so a
 * timeout is as deterministic as any action.
 *
 * - The non-active player's clock is a prompt clock: on expiry it answers that one prompt of theirs,
 *   and with none of theirs open it does nothing — it never touches the active player's prompt or
 *   turn.
 * - The active player's clock is the turn clock: every prompt of theirs that is open, or that an
 *   answer opens in turn (a chain like KY's Private Tutor's), is answered, and then the turn ends.
 *   A prompt the other player holds stops it there, since that one has a clock of its own, and it
 *   stops once the turn has passed, so nothing on the next turn is answered for anybody.
 */
function timeout(sink: EngineSink, action: Extract<Action, { type: "timeout" }>): string | null {
  const who = action.playerId;
  const turn = sink.state.turn;
  const turnClock = who === sink.state.active;

  // R268: while the mulligans are open (R265) the clock that ran out is the mulligan's, and it
  // answers only this seat's own mulligan, by keeping the whole hand: Hearthstone confirms the hand
  // as it stands when its mulligan timer runs out, and nothing is marked to return until the player
  // marks it. It draws nothing from the rng, and it never answers the other seat's.
  if (sink.state.pending === null && sink.state.mulligan !== undefined) {
    const prompt = mulliganPromptFor(sink.state, who);
    if (prompt !== null) answerMulligan(sink, who, prompt.options.map((option) => option.key));
    return null;
  }

  for (let step = 0; step < TIMEOUT_ANSWER_CAP; step += 1) {
    const state = sink.state;
    if (state.result !== null || state.turn !== turn) return null;

    const pending = state.pending;
    if (pending !== null) {
      if (pending.playerId !== who) return null;
      // R79: the AI policy answers, and it never concedes (R84), so the draw is over the prompt's
      // own answers — the concede R211 also offers is not one of them.
      const answers = legalActions(state, who).filter((body) => body.type !== "concede");
      const pick = answers[sink.rng.int(answers.length)];
      if (pick === undefined) return null;
      const error = applyAction(sink, { ...pick, playerId: who, nonce: action.nonce } as Action);
      if (error !== null) return error;
      if (!turnClock) return null;
      // The answer's own resolution loop, so a prompt it leads to is open before the next look.
      settle(sink);
      continue;
    }

    if (!turnClock || state.active !== who || state.phase !== "main") return null;
    endTurn(sink);
    settle(sink);
  }
  return null;
}

/**
 * R44, §8 #96: "an AI plays the rest of their turn with random legal actions", and while it does,
 * that player is locked out — their client does not act while `aiTurn` is set (R152). A question of
 * theirs can still open outside the AI's own playout: a Death hook of their unit that the other
 * player's trap destroys inside the other player's answer, or the Cry of a card the AI played once
 * the other player has answered the trap that asked about it. That question is the AI's to answer,
 * as every prompt of that turn is (§10.7), and the answer goes on to finish the turn the AI owes
 * (`aiPolicy.AI_TURN_WORK`). Left open, the turn stalled until the turn clock, which R79 then ended.
 */
function answerForLockedOut(sink: EngineSink): void {
  for (let guard = 0; guard <= TURN_CAP_PLAYER_TURNS; guard += 1) {
    const state = sink.state;
    const pending = state.pending;
    if (state.result !== null || pending === null) return;
    if (!state.players[pending.playerId].aiTurn) return;
    if (playOutTurn(sink, pending.playerId).actions.length === 0) return;
  }
}

/**
 * B5 E10, R456: the main-phase actions "one more action, then your turn ends" counts — a play, an
 * attack, a position switch, an activation. An answer is part of the action that asked; ending the
 * turn uses the rest up by ending it.
 */
const TURN_ACTION_TYPES: readonly ActionType[] = ["play", "attack", "switchPosition", "activate", "activatePower"];

/**
 * B5 E10, R456: the rider an action counts against — the acting player's own "your turn ends" rider
 * with actions still left, as it stood before the action. A rider the action itself puts in place
 * (Classic+ #26 Radiant drawn by it) is counted from the next action on.
 */
function turnActionCounted(state: GameState, action: Action): string | null {
  if (!TURN_ACTION_TYPES.includes(action.type)) return null;
  if (action.playerId !== state.active) return null;
  const rider = turnEndsOf(state, action.playerId);
  return rider === null || rider.actionsLeft <= 0 ? null : rider.id;
}

/** R456: spend one of the actions a rider leaves, once the action it counted was accepted. */
function countTurnAction(state: GameState, player: PlayerId, riderId: string): void {
  const rider = turnEndsOf(state, player);
  if (rider !== null && rider.id === riderId && rider.actionsLeft > 0) rider.actionsLeft -= 1;
}

/**
 * B5 E10, R456: whether the active player's turn is due to end because an effect cut it short: its
 * rider has no actions left, and what was resolving has resolved — no prompt open, the turn in its
 * main phase (a rider set during the start of a turn, a cast on draw's, waits for it).
 */
function turnCutDue(state: GameState): ReturnType<typeof turnEndsOf> {
  if (state.result !== null || state.pending !== null || state.phase !== "main") return null;
  const rider = turnEndsOf(state, state.active);
  return rider !== null && rider.actionsLeft <= 0 ? rider : null;
}

/**
 * R82, R456: end every turn that is due to end once the action has resolved — one an effect cut
 * short (`turnCutShort`, then every end-of-turn step, as if End turn were pressed), and one with
 * nothing but ending it left to do, unless its player turned that off (R345). Either may start a
 * turn that is due to end in its turn (a Tommy Tempo drawn at its start), hence the loop.
 */
function endDueTurns(sink: EngineSink): void {
  for (let guard = 0; guard <= TURN_CAP_PLAYER_TURNS; guard += 1) {
    const state = sink.state;
    const cut = turnCutDue(state);
    if (cut !== null) {
      const player = state.active;
      removeModifier(sink, player, cut.id);
      sink.events.push({ type: "turnCutShort", player, byInstanceId: cut.byInstanceId });
      endTurn(sink);
      settle(sink);
      continue;
    }
    if (!autoEndDue(state)) return;
    sink.events.push({ type: "turnAutoEnded", player: state.active, turn: state.turn });
    endTurn(sink);
    settle(sink);
  }
}

/**
 * §2.5, R82: when nothing but ending the turn is left, the turn ends by itself — unless the active
 * player has turned that off for themselves (R345), when the turn waits for their End turn. Only
 * whether one other action exists matters, so the walk stops at the first (`eachLegalAction`): this
 * runs after every action, the AI's simulated ones included, and listing every play to the end was
 * nearly a third of a long gate game (#188).
 */
function autoEndDue(state: GameState): boolean {
  if (state.result !== null || state.pending !== null || state.phase !== "main") return false;
  const player = state.active;
  if (state.players[player].autoEndTurn === false) return false;
  for (const action of eachLegalAction(state, player)) {
    if (action.type !== "endTurn" && action.type !== "concede" && action.type !== "offerDraw") return false;
  }
  return true;
}

function rememberNonce(state: GameState, nonce: string, events: GameEvent[]): void {
  state.applied.push({ nonce, events });
  if (state.applied.length > NONCE_HISTORY) {
    state.applied.splice(0, state.applied.length - NONCE_HISTORY);
  }
}

export function reduce(state: GameState, action: Action, rng?: Rng): ReduceResult {
  syncFusedScripts(state);
  const previous = state.applied.find((entry) => entry.nonce === action.nonce);
  if (previous !== undefined) return { state, events: previous.events };

  if (state.result !== null) return { state, events: [], error: "the game is over" };

  // A prompt blocks every action but its own answer and the ones that end a game (§9.3, R79).
  if (state.pending !== null) {
    const allowed = PROMPT_OPEN_ACTION_TYPES.includes(
      action.type as (typeof PROMPT_OPEN_ACTION_TYPES)[number],
    );
    if (!allowed) return { state, events: [], error: "a prompt is open: answer it first" };
    const isAnswer = action.type === "answer" || action.type === "mulligan";
    if (isAnswer && state.pending.playerId !== action.playerId) {
      return { state, events: [], error: "that prompt belongs to the other player" };
    }
  }

  // R265: while the mulligans are open both seats owe one, so neither is "not on turn" — setup is no
  // player's turn (§2.1) — and nothing but a mulligan and the actions that end a game moves.
  const mulliganOpen = state.pending === null && state.mulligan !== undefined;
  if (mulliganOpen && !MULLIGAN_OPEN_ACTION_TYPES.includes(action.type)) {
    return { state, events: [], error: "the mulligan is open: answer it first" };
  }

  const nonActive = action.playerId !== state.active;
  if (
    nonActive &&
    !mulliganOpen &&
    !NON_ACTIVE_ACTION_TYPES.includes(action.type as (typeof NON_ACTIVE_ACTION_TYPES)[number]) &&
    state.pending?.playerId !== action.playerId
  ) {
    return { state, events: [], error: "it is not your turn" };
  }

  const next = cloneState(state);
  const events: GameEvent[] = [];
  const sink: EngineSink = { state: next, events, rng: rng ?? createRng(next.seed, next.rngCursor) };

  const counted = turnActionCounted(next, action);
  const error = applyAction(sink, action);
  if (error !== null) return { state, events: [], error };
  if (counted !== null) countTurnAction(next, action.playerId, counted);

  // §10.3: the resolution loop finishes the action — the events it emitted, the work a prompt left
  // owed, the state check and the trigger queue — and stops where a prompt is waiting.
  settle(sink);
  answerForLockedOut(sink);
  endDueTurns(sink);
  // R661: a Glitch's reset goes once the action that drew it has settled.
  if (next.resetOwed === true) resetMatch(sink);

  next.rngCursor = sink.rng.cursor;
  rememberNonce(next, action.nonce, events);
  return { state: next, events };
}

/** Start the game: shuffle, deal and open both mulligans (§2.1, R265). */
export function beginGame(state: GameState, rng?: Rng): ReduceResult {
  const next = cloneState(state);
  const events: GameEvent[] = [];
  const sink: EngineSink = { state: next, events, rng: rng ?? createRng(next.seed, next.rngCursor) };
  beginSetup(sink);
  next.rngCursor = sink.rng.cursor;
  return { state: next, events };
}

function mulliganSubsets(ids: string[]): string[][] {
  const total = 2 ** ids.length;
  if (total > MAX_MULLIGAN_SUBSETS) return [ids, []];
  const out: string[][] = [];
  for (let mask = 0; mask < total; mask += 1) {
    out.push(ids.filter((_, i) => (mask & (1 << i)) !== 0));
  }
  return out;
}

/**
 * Every action that would not error, for the client's greying-out and for the AI policy (§10.2).
 *
 * Each kind comes from the module that refuses it, never from a second copy of the rule here: the
 * plays from `playChoices` (R81's five choice kinds crossed and bounded, R90), the attacks from
 * `combat.attackTargets`, the powers from `heroPower.whyCannotActivate` and, while a prompt is
 * open, that prompt's own answers from `prompts.promptAnswers`.
 */
export function legalActions(state: GameState, player: PlayerId): ActionBody[] {
  return [...eachLegalAction(state, player)];
}

/**
 * `legalActions` one action at a time, in the same order: the one place the list is made, so a
 * caller that only asks whether some action exists (`autoEndDue`) stops computing at the first.
 */
function* eachLegalAction(state: GameState, player: PlayerId): Generator<ActionBody> {
  syncFusedScripts(state);
  if (state.result !== null) return;

  const pending = state.pending;
  if (pending !== null) {
    // R211: a prompt blocks everything but its own answer and the actions that end a game, and
    // `reduce` accepts a concede from either seat while it is open (§2.5, BUILD M1-T3) — so both
    // seats are offered it, as they are at every other moment of a live game. §10.7's policy never
    // takes it (R84), so what the policy draws from is still that prompt's answers alone.
    const concede: ActionBody = { type: "concede" };
    if (pending.playerId !== player) {
      yield concede;
      return;
    }
    yield* promptAnswers(pending);
    yield concede;
    return;
  }

  if (state.mulligan !== undefined) {
    // R265: both mulligans are open at once. A seat that still owes one is offered its answers and
    // concede (R211); a seat that has answered waits for the other, with concede alone.
    const concede: ActionBody = { type: "concede" };
    const mulligan = mulliganPromptFor(state, player);
    if (mulligan === null) {
      yield concede;
      return;
    }
    for (const keep of mulliganSubsets(mulligan.options.map((option) => option.key))) yield { type: "mulligan", keep };
    yield concede;
    return;
  }

  if (hasStandingDrawOffer(state, player)) {
    yield { type: "answerDraw", accept: true };
    yield { type: "answerDraw", accept: false };
  }

  if (state.active !== player || state.phase !== "main") {
    yield { type: "concede" };
    return;
  }

  const side = state.players[player];
  for (const card of side.hand) yield* playActionsFor(state, player, card);
  // B5 E11, R454: a card in the player's graveyard, while a permission on their field lets them play
  // it (`graveyardPlay.ts`) — the same `play` action, naming a graveyard card.
  for (const card of side.graveyard) yield* graveyardPlayActionsFor(state, player, card);

  for (const unit of activeUnitsOf(state, player)) {
    for (const target of attackTargets(state, unit)) {
      yield { type: "attack", attackerId: unit.id, targetId: attackTargetId(target) };
    }
    if (canSwitch(state, unit)) yield { type: "switchPosition", instanceId: unit.id };
  }

  // R43, R384: a power or an ability is the instance's, so every card the player has acting on the
  // field is asked — the top of each unit pile, then the backrow, lane by lane.
  for (const row of ["units", "backrow"] as const) {
    for (const ref of slotsOf(player, row)) {
      const card = cardAt(state, ref);
      if (card !== null) yield* activationActions(state, player, card);
    }
  }

  if (canOfferDraw(state, player)) yield { type: "offerDraw" };

  yield { type: "endTurn" };
  yield { type: "concede" };
}

/** Exported for the AI policy and the client: the instance an `attack` action would move (§10.2). */
export function unitForAction(state: GameState, instanceId: string): CardInstance | undefined {
  return findInstance(state, instanceId);
}
