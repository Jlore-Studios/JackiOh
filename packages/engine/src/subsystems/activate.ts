// Activate (docs/classic-sets.md B3.2, R384): a card's "Activate:", "Activate N:" and "Activate ♾️:"
// abilities, used by the `activate` action while the card acts on its controller's side of the field.
//
// The rules, in B3.2's order, and where each one lives here:
//   1. "Activate" is once per turn, "Activate N" N times, "Activate ♾️" any number of times, bounded
//      by `ACTIVATE_UNLIMITED_CAP` (rule 7). Degrade and Upgrade move N by the tuning key "Activate"
//      (`tuning.tunedCount`, rule 9), never ♾️. `usesAllowed`.
//   2. Who and when: the card's controller, in their own main phase, with no prompt open and the game
//      not over, while the card acts on the field — the top of its pile, or a face-up backrow card
//      (`isActingOnField`). Summoning sickness and exertion do not apply: activating is not attacking.
//   3. Uses are counted per card per turn on the instance (`memory.activations = { turn, count }`,
//      SPEC §6.2), which R78's reset clears when the card leaves the field, so a card bounced and
//      played again, or a copy, starts fresh. A card with several abilities (a fusion's) counts every
//      use of any of them, and each ability allows as many as its own number says.
//   4. A cost is paid as the ability is activated — mana, a random discard, a Tribute of the
//      controller's units (the card itself allowed) or the card itself — and an ability whose cost
//      cannot be paid cannot be activated (`whyCannotActivateAbility`).
//   5. The targets and modes the ability declares travel in the action, as a play's do (R81), checked
//      by `playChoices` against the ability's declarations (R90); the discards a declared target
//      costs (B5 E5, R450, Classic #89) are random at pay time (R641) and travel nowhere. Choices
//      made during resolution are ordinary prompts, which the card's `resume` table answers.
//   6. Not a play: nothing that counts plays sees it (no turn log, no `counters.played`, no
//      `cardPlayed`). What the effect plays or casts counts as usual (R70).
//   8. `legalActions` lists `activate` exactly as it lists a Heroic Power's `activatePower`: the
//      refusal below is also the list (§10.2's pattern, R43), so a greyed-out control and a refused
//      action give one reason.
//  10. Heroic Power (Core #98) keeps its own power (R43) in v0.2.0; `reduce.ts` routes `activate` and
//      `activatePower` on it to `heroPower.ts`, so `activatePower` is an alias and every old log
//      replays. Patch v0.2.1 moves the powers onto this module as a card patch: an ability with a
//      mana price and a declared target, and `ActivationDecl.has` for the one power a copy rolled.
//
// The sequence is resumable like every other that can ask (§9.3, R113): paying a Tribute runs the
// tributed units' Death hooks, which can ask, and then the effect is owed on `state.work`
// (`ACTIVATION_WORK`) as plain data; the effect's own list runs through `prompts.runHookResumable`
// under the hook `activation:<id>`, so a tail its prompt parks comes back to the same ability
// (`work.scriptStepFor`).

import type { ActionBody, ActivationView, PlayerId, Selection } from "@jackioh/shared";
import { ACTIVATE_UNLIMITED_CAP } from "../config";
import { discardRandom } from "../effects/move";
import { unitView } from "../layers";
import { manaEvent, spendMana } from "../mana";
import {
  inDeclaredOrder,
  playChoiceCombinations,
  whyDeclaredChoicesRefused,
  targetingDiscardsRequired,
  type DeclaredChoices,
} from "../playChoices";
import { isFaceDown } from "../preview";
import { runHookResumable } from "../prompts";
import { makeContext, type EngineSink } from "../resolve";
import { activationDecls, activationHook, type ActivationDecl, type EffectContext } from "../script";
import { scriptOf } from "../scripts";
import { sacrificeTogether, stateCheck } from "../stateCheck";
import { findInstance, type CardInstance, type GameState, type Resume, type WorkItem } from "../state";
import { exitMark } from "../stays";
import { whyTargetingDiscardsUnpayable } from "../targeting";
import { payTargetingDiscards } from "../targetingPoint";
import { tunedCount } from "../tuning";
import { paused, pushWork, registerWorkHandler } from "../work";
import { actsOnField, activeUnitsOf, slotOf } from "../zones";

/** The `activate` member of the action union, without the `playerId` and `nonce` the caller adds. */
export type ActivateAction = Extract<ActionBody, { type: "activate" }>;

/** B3.2 rule 9: the tuning key Degrade and Upgrade move an "Activate N" by (`tuning.tunedCount`). */
export const ACTIVATE_TUNING_KEY = "Activate";

// ---------------------------------------------------------------------------
// The card and its abilities
// ---------------------------------------------------------------------------

/**
 * B3.2 rule 2: the card acts on the field — it is the top of its pile (a card dormant under a Stack
 * pile is not on the field for effects, R13, and the same holds under a backrow pile), and a backrow
 * card is face-up: a face-down card has no text anyone can use.
 */
export function isActingOnField(state: GameState, card: CardInstance): boolean {
  // §3.2, R13, R446, R447: the top of a pile of either row, or a Unit a carrier holds — never a card
  // dormant beneath (`zones.actsOnField`) — and a backrow card only while it is face-up.
  const at = slotOf(state, card);
  if (at === null || !actsOnField(state, card)) return false;
  return !(at.row === "backrow" && isFaceDown(state, card));
}

/**
 * The abilities the card has now: its running face's (`scriptOf`, so a Vanilla card has none, §6.3),
 * ids made unique for a fused card (`script.activationDecls`, R102), less any it does not have on this
 * instance (`ActivationDecl.has`).
 */
export function abilitiesOf(state: GameState, card: CardInstance): ActivationDecl[] {
  return activationDecls(scriptOf(card)).filter(
    (decl) => decl.has === undefined || decl.has({ state, self: card, radiant: card.radiant }),
  );
}

/** The ability an action names, or the refusal: none named picks the card's only one. */
function findAbility(state: GameState, card: CardInstance, ability: string | undefined): ActivationDecl | string {
  const abilities = abilitiesOf(state, card);
  if (abilities.length === 0) return "that card has no Activate ability";
  if (ability === undefined) {
    const [only] = abilities;
    if (abilities.length > 1 || only === undefined) return "that card has several abilities: name the one to activate";
    return only;
  }
  return abilities.find((decl) => decl.id === ability) ?? `that card has no ability "${ability}"`;
}

function declaredOf(decl: ActivationDecl): DeclaredChoices {
  return { targets: decl.targets ?? [], modes: decl.modes ?? [] };
}

/** B5 E5, R450: the discards the ability's declared targets cost (Classic #89). */
function discardsOwed(state: GameState, player: PlayerId, card: CardInstance, decl: ActivationDecl, targets: readonly Selection[], modes: readonly string[]): number {
  return targetingDiscardsRequired(state, player, card, targets, modes, decl.targets ?? []);
}

/** R450: the hand cards the activation's own picks use, which cannot pay its targeting cost. */
function handPicks(targets: readonly Selection[]): string[] {
  return targets.flatMap((selection) => (selection.pick === "instance" ? [selection.instanceId] : []));
}

// ---------------------------------------------------------------------------
// Uses (B3.2 rules 1, 3, 7, 9)
// ---------------------------------------------------------------------------

/** R384, SPEC §6.2: where an instance counts its uses — `{ turn, count }` for the turn it names. */
export const ACTIVATIONS_MEMORY_KEY = "activations";

type UsesRecord = { turn: number; count: number };

function usesRecord(card: CardInstance): UsesRecord | null {
  const raw: unknown = card.memory[ACTIVATIONS_MEMORY_KEY];
  if (raw === null || typeof raw !== "object") return null;
  const record = raw as Partial<UsesRecord>;
  if (typeof record.turn !== "number" || typeof record.count !== "number") return null;
  return { turn: record.turn, count: record.count };
}

/** How many times the card's abilities have been used this turn. */
export function usesThisTurn(state: GameState, card: CardInstance): number {
  const record = usesRecord(card);
  return record === null || record.turn !== state.turn ? 0 : record.count;
}

/**
 * B3.2 rules 1, 7, 9: how many uses the ability allows each turn. "Activate N" is N moved by Degrade
 * and Upgrade (never below 1, R386's floor); "Activate ♾️" is `ACTIVATE_UNLIMITED_CAP`, which no
 * tuning moves.
 */
export function usesAllowed(card: CardInstance, decl: ActivationDecl): number {
  if (decl.uses === "unlimited") return ACTIVATE_UNLIMITED_CAP;
  return tunedCount(card, ACTIVATE_TUNING_KEY, Math.max(1, Math.trunc(decl.uses)));
}

function markUse(state: GameState, card: CardInstance): void {
  card.memory[ACTIVATIONS_MEMORY_KEY] = { turn: state.turn, count: usesThisTurn(state, card) + 1 };
}

// ---------------------------------------------------------------------------
// The refusal, which is also the list (B3.2 rules 2, 4, 8)
// ---------------------------------------------------------------------------

/** B3.2 rule 4: the units a Tribute cost may take — the controller's acting units, the card too. */
function tributeUnitsFor(state: GameState, player: PlayerId, card: CardInstance, decl: ActivationDecl): CardInstance[] {
  const units = activeUnitsOf(state, player);
  // "Tribute this" pays with the card itself, so it is not also one of the units a Tribute counts.
  // R642: a cost that excludes itself (Classic #21) never lists the card either.
  if (decl.cost?.tributeSelf === true || decl.cost?.tributeExcludesSelf === true) {
    return units.filter((unit) => unit.id !== card.id);
  }
  return units;
}

function plural(count: number, one: string): string {
  return count === 1 ? `a ${one}` : `${count} ${one}s`;
}

/** The turn, the uses, the text's own condition and the costs: why the ability cannot be used now. */
function whyAbilityUnusable(state: GameState, player: PlayerId, card: CardInstance, decl: ActivationDecl): string | null {
  if (state.result !== null) return "the game is over";
  if (state.pending !== null) return "answer the open prompt first";
  if (state.active !== player) return "it is not your turn";
  if (state.phase !== "main") return "an ability is activated in the main phase";

  const allowed = usesAllowed(card, decl);
  if (usesThisTurn(state, card) >= allowed) {
    return allowed === 1 ? "that ability has already been used this turn" : `that ability has been used ${allowed} times this turn`;
  }
  if (
    decl.canActivate !== undefined &&
    decl.canActivate({ state, self: card, controller: player, radiant: card.radiant, zone: "field", yourTurn: true }) !== true
  ) {
    return "that ability can't be activated now";
  }

  const side = state.players[player];
  const mana = Math.max(0, decl.cost?.mana ?? 0);
  if (mana > side.mana.current) return `that ability costs ${mana}, more than your mana`;
  const discards = Math.max(0, decl.cost?.discardRandom ?? 0);
  if (discards > side.hand.length) return `that ability needs ${plural(discards, "card")} in your hand to discard`;
  const tributes = Math.max(0, decl.cost?.tribute ?? 0);
  if (tributes > tributeUnitsFor(state, player, card, decl).length) {
    return `that ability needs ${plural(tributes, "Unit")} to Tribute`;
  }
  return null;
}

/**
 * R384: why `player` cannot activate that ability of that card right now, or null when they can.
 * `ability` names one of several (B3.2 rule 5); none named is the card's only one. `legalActions`
 * lists an `activate` for exactly the abilities this answers null for, so the two agree (§10.2).
 */
export function whyCannotActivateAbility(
  state: GameState,
  player: PlayerId,
  instanceId: string,
  ability?: string,
): string | null {
  const card = findInstance(state, instanceId);
  if (card === undefined) return `no card ${instanceId}`;
  if (card.controller !== player) return "that card is not yours";
  if (card.zone.z !== "field") return "that card is not on the field";
  if (!isActingOnField(state, card)) {
    return isFaceDown(state, card) ? "a face-down card has no ability to use" : "that card is under a pile and does not act";
  }
  const decl = findAbility(state, card, ability);
  if (typeof decl === "string") return decl;
  return whyAbilityUnusable(state, player, card, decl);
}

/** B3.2 rule 4: the units an action tributes, checked: exactly the number the cost names, each once. */
function refuseTributes(
  state: GameState,
  player: PlayerId,
  card: CardInstance,
  decl: ActivationDecl,
  picked: readonly string[],
): string | null {
  const need = Math.max(0, decl.cost?.tribute ?? 0);
  if (need === 0) return picked.length === 0 ? null : "that ability needs no Tribute";
  const legal = new Set(tributeUnitsFor(state, player, card, decl).map((unit) => unit.id));
  const seen = new Set<string>();
  for (const id of picked) {
    if (!legal.has(id)) return `${id} cannot be tributed for that ability`;
    if (seen.has(id)) return "that ability cannot tribute the same Unit twice";
    seen.add(id);
  }
  return picked.length === need ? null : `that ability tributes ${plural(need, "Unit")}`;
}

/**
 * §9.3 "reduce refuses illegal actions itself", for everything an `activate` carries: the card and
 * the ability (`whyCannotActivateAbility`), the targets and modes it declares (R81, R90, through the
 * same `playChoices` rules a play is read by), and the units a Tribute cost takes.
 */
export function whyActivateRefused(state: GameState, player: PlayerId, action: ActivateAction): string | null {
  const why = whyCannotActivateAbility(state, player, action.instanceId, action.ability);
  if (why !== null) return why;
  const card = findInstance(state, action.instanceId) as CardInstance;
  const decl = findAbility(state, card, action.ability) as ActivationDecl;
  const targets = action.targets ?? [];
  const modes = action.modes ?? [];
  return (
    whyDeclaredChoicesRefused(state, player, card, declaredOf(decl), targets, modes) ??
    refuseTributes(state, player, card, decl, action.tributes ?? []) ??
    // B5 E5, R450, R641: a declared target that costs discards needs that many other cards held —
    // the discards are random at pay time, so the action carries none.
    whyTargetingDiscardsUnpayable(state, player, discardsOwed(state, player, card, decl, targets, modes), handPicks(targets))
  );
}

/** Every `size`-unit set of `units`, in board order (§6.3's Tribute: the price, listed whole, R90). */
function unitSets(units: readonly CardInstance[], size: number): string[][] {
  if (size <= 0) return [[]];
  const out: string[][] = [];
  const chosen: string[] = [];
  const walk = (from: number): void => {
    if (chosen.length === size) {
      out.push([...chosen]);
      return;
    }
    for (let at = from; at < units.length; at += 1) {
      const unit = units[at];
      if (unit === undefined) continue;
      chosen.push(unit.id);
      walk(at + 1);
      chosen.pop();
    }
  };
  walk(0);
  return out;
}

/**
 * B3.2 rule 8, R384: every `activate` action this card's abilities offer `player` now — each usable
 * ability crossed with the Tribute sets its cost may take and the target and mode combinations it
 * declares, bounded as a play's are (`playChoices.playChoiceCombinations`, R90).
 */
export function activateActionsFor(state: GameState, player: PlayerId, card: CardInstance): ActivateAction[] {
  const out: ActivateAction[] = [];
  for (const decl of abilitiesOf(state, card)) {
    if (whyCannotActivateAbility(state, player, card.id, decl.id) !== null) continue;
    const tributeSets = unitSets(tributeUnitsFor(state, player, card, decl), Math.max(0, decl.cost?.tribute ?? 0));
    const choices = playChoiceCombinations(state, player, card, declaredOf(decl));
    for (const tributes of tributeSets) {
      for (const choice of choices) {
        // B5 E5, R450, R641: the targets' discard cost is random at pay time, so it lists no
        // paying sets — one action, offered only when the cost can be paid at all.
        const owed = discardsOwed(state, player, card, decl, choice.targets ?? [], choice.modes ?? []);
        if (whyTargetingDiscardsUnpayable(state, player, owed, handPicks(choice.targets ?? [])) !== null) {
          continue;
        }
        out.push({
          type: "activate",
          instanceId: card.id,
          ability: decl.id,
          ...(tributes.length === 0 ? {} : { tributes }),
          ...choice,
        });
      }
    }
  }
  return out;
}

// ---------------------------------------------------------------------------
// Activating (B3.2 rules 3, 4, 6)
// ---------------------------------------------------------------------------

/**
 * R384, R78: a unit a Tribute cost took, as it stood before it died — Classic #21 Turtinator deals
 * damage equal to its Attack "as it stood" (last-known information, as a Death hook reads, R89).
 */
export type TributedUnit = {
  instanceId: string;
  defId: string;
  radiant: boolean;
  attack: number;
  health: number;
  maxHealth: number;
};

/** Where an ability's effect list finds what its activation paid (`activationPaid`). */
export const ACTIVATION_DATA_KEY = "__activation";

/** What an ability's effect list reads of the activation that runs it. */
export type ActivationPaid = { ability: string; tributed: TributedUnit[] };

/**
 * R384: what the activation running this effect paid — the ability's id and the units its Tribute
 * cost took, as they stood (`TributedUnit`). Carried in the run's data, so a step a prompt re-enters
 * reads it too. Empty outside an activation.
 */
export function activationPaid(ctx: Pick<EffectContext, "data">): ActivationPaid {
  const raw: unknown = ctx.data[ACTIVATION_DATA_KEY];
  if (raw === null || typeof raw !== "object") return { ability: "", tributed: [] };
  const paid = raw as Partial<ActivationPaid>;
  return {
    ability: typeof paid.ability === "string" ? paid.ability : "",
    tributed: Array.isArray(paid.tributed) ? paid.tributed : [],
  };
}

/** One activation between its costs and its effect: all JSON, so a pause can owe it (R113). */
type ActivationRun = {
  player: PlayerId;
  instanceId: string;
  defId: string;
  radiant: boolean;
  ability: string;
  targets: Selection[];
  modes: string[];
  tributed: TributedUnit[];
  /** R174: the field's departures when the choices were checked, so a target the costs took is gone. */
  exitsFrom: number;
};

/** R113: the `resume.hook` of an activation whose cost paused before its effect ran. */
export const ACTIVATION_WORK = "@activate";

const RUN_DATA_KEY = "run";

function snapshotOf(state: GameState, unit: CardInstance): TributedUnit {
  const view = unitView(state, unit);
  return {
    instanceId: unit.id,
    defId: unit.defId,
    radiant: unit.radiant,
    attack: view.attack,
    health: view.health,
    maxHealth: view.maxHealth,
  };
}

/**
 * B3.2 rule 4: the costs, in the order the ability names them — mana, then the discards its declared
 * targets cost (B5 E5, R450), then a random discard, then the Tribute (the tributed units and "Tribute
 * this" die together as one payment, §6.3, R101). A Tribute is a Sacrifice, so it is a death in full:
 * Death hooks, Reborn, the destroyed counter (§4.5).
 */
function payCosts(
  sink: EngineSink,
  run: ActivationRun,
  card: CardInstance,
  decl: ActivationDecl,
  tributes: readonly string[],
): void {
  const state = sink.state;
  const side = state.players[run.player];

  const mana = Math.max(0, decl.cost?.mana ?? 0);
  if (mana > 0) {
    spendMana(side, mana);
    sink.events.push(manaEvent(run.player, side));
  }

  // B5 E5, R450, R641: a targeting cost is part of the price, paid with it (Classic #89) — random
  // cards from the hand, drawn at pay time. Never a hand card the activation picks.
  const owed = discardsOwed(sink.state, run.player, card, decl, run.targets, run.modes);
  if (owed > 0) payTargetingDiscards(sink, run.player, owed, handPicks(run.targets));

  const random = Math.max(0, decl.cost?.discardRandom ?? 0);
  if (random > 0) discardRandom({ count: random }).apply(makeContext(sink, card, { controller: run.player }));

  const units = tributes.flatMap((id) => {
    const unit = findInstance(state, id);
    return unit === undefined || unit.zone.z !== "field" ? [] : [unit];
  });
  run.tributed = units.map((unit) => snapshotOf(state, unit));
  const paying = decl.cost?.tributeSelf === true && card.zone.z === "field" ? [...units, card] : units;
  if (paying.length > 0) sacrificeTogether(sink, paying);
}

/**
 * The ability's effect, run as the card's own list under `activation:<id>` so a prompt inside it
 * pauses the rest (R113) and its answer comes back to the same ability. The card is its `self`
 * wherever it is now (a card that tributed itself resolves from its graveyard, as a Death hook does),
 * with the face it was activated with. R59: the state check follows the whole ability; one whose list
 * is still asking is not whole yet, and the loop checks once it is.
 */
function runEffect(sink: EngineSink, run: ActivationRun): void {
  const paid: ActivationPaid = { ability: run.ability, tributed: run.tributed };
  runHookResumable(
    sink,
    { id: run.instanceId, defId: run.defId, controller: run.player, radiant: run.radiant },
    activationHook(run.ability),
    {
      controller: run.player,
      targets: run.targets,
      modes: run.modes,
      data: { [ACTIVATION_DATA_KEY]: paid },
      exitsFrom: run.exitsFrom,
    },
  );
  if (!paused(sink)) stateCheck(sink);
}

/** R113, R117: owe the effect of an activation whose cost paused, at the moment it paused. */
function oweEffect(sink: EngineSink, run: ActivationRun): void {
  const resume: Resume = {
    defId: run.defId,
    hook: ACTIVATION_WORK,
    step: run.ability,
    radiant: run.radiant,
    instanceId: run.instanceId,
    data: { [RUN_DATA_KEY]: run },
  };
  pushWork(sink, resume, run.player);
}

/** The run an owed item carries, read back defensively: it came through JSON (§10.1). */
function runFrom(data: Record<string, unknown>): ActivationRun | null {
  const raw: unknown = data[RUN_DATA_KEY];
  if (raw === null || typeof raw !== "object") return null;
  const run = raw as Partial<ActivationRun>;
  if (
    (run.player !== "p1" && run.player !== "p2") ||
    typeof run.instanceId !== "string" ||
    typeof run.defId !== "string" ||
    typeof run.ability !== "string" ||
    typeof run.exitsFrom !== "number"
  ) {
    return null;
  }
  return {
    player: run.player,
    instanceId: run.instanceId,
    defId: run.defId,
    radiant: run.radiant === true,
    ability: run.ability,
    targets: Array.isArray(run.targets) ? run.targets : [],
    modes: Array.isArray(run.modes) ? run.modes.filter((mode): mode is string => typeof mode === "string") : [],
    tributed: Array.isArray(run.tributed) ? run.tributed : [],
    exitsFrom: run.exitsFrom,
  };
}

function runOwedActivation(sink: EngineSink, item: WorkItem): void {
  const run = runFrom(item.resume.data);
  if (run !== null) runEffect(sink, run);
}

registerWorkHandler(ACTIVATION_WORK, runOwedActivation);

/**
 * The `activate` action (B3.2, R384): validate, count the use, announce it (`activated`), pay the
 * costs, run the effect. The use is counted before anything can pause, so an ability that asks has
 * spent its use and the answer cannot buy another (as R43's power does). Not a play (rule 6): no turn
 * log, no play counter, no `cardPlayed`.
 */
export function activateAbility(sink: EngineSink, player: PlayerId, action: ActivateAction): string | null {
  const state = sink.state;
  const refused = whyActivateRefused(state, player, action);
  if (refused !== null) return refused;
  const card = findInstance(state, action.instanceId) as CardInstance;
  const decl = findAbility(state, card, action.ability) as ActivationDecl;

  const modes = [...(action.modes ?? [])];
  const run: ActivationRun = {
    player,
    instanceId: card.id,
    defId: card.defId,
    radiant: card.radiant,
    ability: decl.id,
    // R221, R90: each declaration's picks are a set, taken in the order it offers them.
    targets: inDeclaredOrder(state, player, card, action.targets ?? [], modes, decl.targets ?? []),
    modes,
    tributed: [],
    exitsFrom: exitMark(state),
  };

  markUse(state, card);
  sink.events.push({ type: "activated", player, instanceId: card.id, defId: card.defId, ability: decl.id });
  payCosts(sink, run, card, decl, action.tributes ?? []);
  if (paused(sink)) {
    if (state.result === null) oweEffect(sink, run);
    return null;
  }
  runEffect(sink, run);
  return null;
}

// ---------------------------------------------------------------------------
// The view (§10.8)
// ---------------------------------------------------------------------------

/**
 * R384: the card's abilities as its controller's client needs them — on the controller's own view
 * of a card acting on the field, and nowhere else (the other player reads the card's text; whether it
 * could be used is its controller's business). `usable` is exactly whether `legalActions` lists it.
 */
export function activationViewsFor(state: GameState, viewer: PlayerId, card: CardInstance): ActivationView[] | null {
  if (card.controller !== viewer || !isActingOnField(state, card)) return null;
  const abilities = abilitiesOf(state, card);
  if (abilities.length === 0) return null;
  return abilities.map((decl) => {
    const reason = whyCannotActivateAbility(state, viewer, card.id, decl.id);
    const usesLeft = decl.uses === "unlimited" ? null : Math.max(0, usesAllowed(card, decl) - usesThisTurn(state, card));
    return {
      ability: decl.id,
      label: decl.label,
      usesLeft,
      usable: reason === null,
      ...(reason === null ? {} : { reason }),
    };
  });
}
