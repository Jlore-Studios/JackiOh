// The targeting point (docs/classic-sets.md B5 E5 "a friendly unit is targeted", E9's target redirect;
// R450): what happens the moment a player targets a card, before anything resolves at it.
//
// Two things answer a targeting, in this order:
//   1. A targeting cost (Classic #89 Paul Allen's Ghost: "to target this with anything but an attack,
//      a player must also discard N cards"). A declared target carries its discards in the action, as
//      a Tribute carries its paying set (R101), and §10.5 step 2 pays them (`payTargetingDiscards`);
//      a prompt answer naming such a card asks its chooser for the cards next, as a `hand` prompt of
//      their own, and only then goes on. It binds both players. With fewer cards than the cost in
//      hand the card is not a legal target at all (`targeting.canPayToTarget`), so it is never
//      offered.
//   2. An interception (Classic #33 Joro: "While this is in your hand: when your opponent targets one
//      of your Units with a Spell, summon this and make it the new target"). The first card in the
//      targeted unit's controller's hand whose `replacements` declare `{ on: "targeted", where:
//      "hand" }` and answer this source (`by: "spell"` needs a Spell, R651) is
//      summoned (R64's leftmost open unit zone; with none, nothing happens), with no Cry and
//      summoning sick, and the pick moves to it — `redirected` "target". One interceptor answers one targeting: a play naming several of
//      that player's units redirects the first. A declared pick moves only when the interceptor is
//      itself a legal pick of that declaration (Hearthstone's Spellbender); a cost already paid for
//      the first pick stays paid.
// "Targeting" is choosing: declared `target` picks of a play, a cast and an activation, and every
// answer to a `target` prompt. Random picks, "all" effects, Tributes, hand picks and zone picks target
// nothing (R450). The attack half (§4.2 step 2, `combat.ts`) calls `interceptTargeting` with
// `what: "attack"`.
//
// A prompt answer reaches this through `prompts.registerTargetingHooks` (the card continuations'
// prompts) and through the play pipeline's own answerer (`playSteps.answerPlayPrompt`), which owns its
// prompts (R122). The cost's `hand` prompt is an engine sequence of its own (`TARGET_COST_HOOK`): its
// continuation carries the interrupted answer as plain JSON, so a paused targeting survives a round
// trip and a replay (§9.3, R113).

import type { CardType, PlayerId, Selection } from "@jackioh/shared";
import { defOf } from "./catalog";
import { discardFromHand } from "./effects/move";
import { summon } from "./effects/summon";
import {
  closePrompt,
  continueAnswer,
  openPrompt,
  registerPromptAnswerer,
  registerTargetingHooks,
  whyAnswerRefused,
  type AnswerInput,
  type OpenPromptArgs,
} from "./prompts";
import { makeContext, type EngineSink } from "./resolve";
import { findInstance, type CardInstance, type GameState, type PendingChoice, type PromptOption, type Resume } from "./state";
import { cardTypeOf } from "./faces";
import { interceptorFor, mayTarget, targetingDiscardsFor, targetingDiscardsOf } from "./targeting";

/** What a redirect moved: a chosen target here, an attack at §4.2 step 2 (`combat.ts`). */
export type RedirectKind = "target" | "attack";

export type InterceptArgs = {
  /** The player who targeted. */
  chooser: PlayerId;
  /** The picks, in the order they were made. */
  picks: readonly Selection[];
  /** Which picks are targetings (a declared `target` pick); every pick when absent. */
  targeting?: (index: number) => boolean;
  /** Whether the interceptor may stand as pick `index` (a declaration's filter); always when absent. */
  accepts?: (interceptor: CardInstance, index: number) => boolean;
  what?: RedirectKind;
  /** The targeting card's type; an attack carries none, so a `by: "spell"` card never answers it (R651). */
  source?: CardType;
};

/**
 * Classic #33 Joro, R450: summon the card that answers this targeting and move the first pick it
 * answers to it. Returns the picks, with at most one changed. The summon is §6.3's (`effects/summon`):
 * the leftmost open unit zone of its controller, no Cry, summoning sick, `summoned`; then `redirected`.
 */
export function interceptTargeting(sink: EngineSink, args: InterceptArgs): Selection[] {
  const picks = [...args.picks];
  for (let index = 0; index < picks.length; index += 1) {
    if (args.targeting !== undefined && !args.targeting(index)) continue;
    const pick = picks[index];
    if (pick?.pick !== "instance") continue;
    const targeted = findInstance(sink.state, pick.instanceId);
    if (targeted === undefined) continue;
    const interceptor = interceptorFor(sink.state, args.chooser, targeted, args.source);
    if (interceptor === null) continue;
    if (args.accepts !== undefined && !args.accepts(interceptor, index)) continue;

    const owner = interceptor.controller;
    const ctx = makeContext(sink, interceptor, { controller: owner });
    summon({ instance: { of: "instance", instanceId: interceptor.id } }).apply(ctx);
    if (interceptor.zone.z !== "field") return picks;
    picks[index] = { pick: "instance", instanceId: interceptor.id };
    sink.events.push({
      type: "redirected",
      what: args.what ?? "target",
      fromId: targeted.id,
      toId: interceptor.id,
      byInstanceId: interceptor.id,
    });
    return picks;
  }
  return picks;
}

/** R450, §6.3 Discard: pay a targeting cost — each card from the player's hand, as a discard. */
export function payTargetingDiscards(sink: EngineSink, player: PlayerId, discards: readonly string[]): void {
  for (const id of discards) {
    const card = findInstance(sink.state, id);
    if (card === undefined || card.zone.z !== "hand" || card.zone.player !== player) continue;
    discardFromHand(sink, card);
  }
}

// ---------------------------------------------------------------------------
// A prompt's answer (R450): the cost's own question, then the interception
// ---------------------------------------------------------------------------

/** `resume.hook` of the cost's `hand` prompt: an engine sequence, never a card script's key. */
export const TARGET_COST_HOOK = "@targetCost";

/**
 * The answer a cost prompt interrupted, as plain JSON: the prompt's continuation and chooser (its
 * options are spent — the picks are already in the order it offered them, R221), and the picks.
 */
type OwedTargeting = {
  prompt: Pick<PendingChoice, "id" | "playerId" | "kind" | "resume">;
  picks: Selection[];
};

/**
 * How an answer that a cost interrupted goes on once the cost is paid, by the hook of the prompt it
 * answered: the play pipeline registers its own (`playSteps`), and every other prompt re-enters its
 * card's step (`prompts.continueAnswer`).
 */
export type TargetedContinuation = (sink: EngineSink, prompt: OwedTargeting["prompt"], picks: Selection[]) => void;

const continuations = new Map<string, TargetedContinuation>();

/** Registered at module scope by the sequence that owns the prompt, like `registerPromptAnswerer`. */
export function registerTargetedContinuation(hook: string, fn: TargetedContinuation | undefined): TargetedContinuation | undefined {
  const previous = continuations.get(hook);
  if (fn === undefined) continuations.delete(hook);
  else continuations.set(hook, fn);
  return previous;
}

function continueTargeted(sink: EngineSink, prompt: OwedTargeting["prompt"], picks: Selection[]): void {
  const owner = continuations.get(prompt.resume.hook);
  if (owner !== undefined) {
    owner(sink, prompt, picks);
    return;
  }
  continueAnswer(sink, prompt, picks);
}

function namesOf(state: GameState, picks: readonly Selection[]): string {
  const names = picks.flatMap((pick) => {
    if (pick.pick !== "instance") return [];
    const card = findInstance(state, pick.instanceId);
    return card === undefined || targetingDiscardsOf(state, card) === 0 ? [] : [defOf(state, card.defId).name];
  });
  return names.join(" and ");
}

/**
 * R450: why a `target` answer cannot stand — its picks cost more cards than its chooser holds (two
 * costly picks of one prompt, each payable alone). Null for any other prompt.
 */
export function whyTargetAnswerRefused(state: GameState, pending: PendingChoice, picks: readonly Selection[]): string | null {
  if (pending.kind !== "target") return null;
  const cost = targetingDiscardsFor(state, picks);
  const hand = state.players[pending.playerId].hand.length;
  return cost > hand ? `targeting those costs ${cost} discards, and you hold ${hand} cards` : null;
}

/**
 * R651: the targeting card's type behind a `target` prompt — the card whose prompt it is
 * (`resume.instanceId`), read as the pick is answered. Undefined when the prompt names no card.
 */
function sourceOf(state: GameState, instanceId: string | undefined): CardType | undefined {
  if (instanceId === undefined) return undefined;
  const source = findInstance(state, instanceId);
  return source === undefined ? undefined : cardTypeOf(state, source);
}

/**
 * R450: the targeting point of a `target` prompt's answer, which the caller has validated and closed.
 * Returns the picks to go on with — the interceptor's in place of the pick it answered — or null when
 * the picks cost discards: the cost's `hand` prompt is open, and its answer finishes this one.
 */
export function targetAnswer(sink: EngineSink, pending: PendingChoice, picks: readonly Selection[]): Selection[] | null {
  if (pending.kind !== "target") return [...picks];
  const chooser = pending.playerId;
  const cost = targetingDiscardsFor(sink.state, picks);
  if (cost > 0) {
    const hand = sink.state.players[chooser].hand;
    const owed: OwedTargeting = {
      prompt: { id: pending.id, playerId: chooser, kind: pending.kind, resume: pending.resume },
      picks: [...picks],
    };
    const resume: Resume = {
      defId: "",
      hook: TARGET_COST_HOOK,
      step: "discard",
      radiant: false,
      data: { owed: JSON.parse(JSON.stringify(owed)) as OwedTargeting },
    };
    const opened = openPrompt(sink, {
      player: chooser,
      kind: "hand",
      prompt: `Discard ${cost} to target ${namesOf(sink.state, picks)}`,
      options: hand.map((card) => ({
        key: `instance:${card.id}`,
        label: defOf(sink.state, card.defId).name,
        selection: { pick: "instance", instanceId: card.id },
      })),
      min: cost,
      max: cost,
      resume,
    });
    if (opened !== null) return null;
  }
  return interceptTargeting(sink, {
    chooser,
    picks,
    what: "target",
    source: sourceOf(sink.state, pending.resume.instanceId),
  });
}

function owedTargetingOf(resume: Resume): OwedTargeting | null {
  const raw: unknown = resume.data.owed;
  if (raw === null || typeof raw !== "object") return null;
  const owed = raw as Partial<OwedTargeting>;
  if (owed.prompt === undefined || !Array.isArray(owed.picks)) return null;
  return owed as OwedTargeting;
}

/** The cost prompt's answer: the cards are discarded, then the interrupted answer goes on (R450). */
function answerTargetCost(sink: EngineSink, answer: AnswerInput): string | null {
  const pending = sink.state.pending;
  if (pending === null) return "no prompt is open";
  const refused = whyAnswerRefused(pending, answer);
  if (refused !== null) return refused;
  const owed = owedTargetingOf(pending.resume);
  closePrompt(sink);
  if (owed === null) return null;
  payTargetingDiscards(sink, pending.playerId, answer.selection.flatMap((pick) => (pick.pick === "instance" ? [pick.instanceId] : [])));
  const picks = interceptTargeting(sink, {
    chooser: pending.playerId,
    picks: owed.picks,
    what: "target",
    source: sourceOf(sink.state, owed.prompt.resume.instanceId),
  });
  continueTargeted(sink, owed.prompt, picks);
  return null;
}

registerPromptAnswerer(TARGET_COST_HOOK, answerTargetCost);

/**
 * R450, E35: the options a `target` prompt may offer its chooser — none they could not pay to target,
 * and none Immune to Spells when a Spell asks. Every other prompt keeps its options.
 */
function targetableOptions(state: GameState, args: OpenPromptArgs): readonly PromptOption[] {
  if (args.kind !== "target") return args.options;
  const sourceId = args.resume.instanceId;
  const source = sourceId === undefined ? null : (findInstance(state, sourceId) ?? null);
  const spellSource = source !== null && cardTypeOf(state, source) === "Spell" ? source : null;
  return args.options.filter((option) => {
    if (option.selection.pick !== "instance") return true;
    const card = findInstance(state, option.selection.instanceId);
    if (card === undefined || card.zone.z !== "field") return true;
    return mayTarget(state, args.player, spellSource, card);
  });
}

registerTargetingHooks({
  options: targetableOptions,
  refuse: whyTargetAnswerRefused,
  answer: targetAnswer,
});
