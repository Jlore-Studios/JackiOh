// The targeting point (docs/classic-sets.md B5 E5 "a friendly unit is targeted", E9's target redirect;
// R450): what happens the moment a player targets a card, before anything resolves at it.
//
// Two things answer a targeting, in this order:
//   1. A targeting cost (Classic #89 Paul Allen's Ghost: "to target this with anything but an attack,
//      a player must also discard N cards"). The discards are random at pay time (R654); §10.5 step
//      2 pays them (`payTargetingDiscards`), and a prompt answer naming such a card pays them before
//      it goes on. It binds both players. With fewer cards than the cost in hand the card is not a
//      legal target at all (`targeting.canPayToTarget`), so it is never offered.
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
// prompts (R122).

import type { CardType, PlayerId, Selection } from "@jackioh/shared";
import { discardFromHand } from "./effects/move";
import { summon } from "./effects/summon";
import { registerTargetingHooks, type OpenPromptArgs } from "./prompts";
import { makeContext, type EngineSink } from "./resolve";
import { findInstance, type CardInstance, type GameState, type PendingChoice, type PromptOption } from "./state";
import { cardTypeOf } from "./faces";
import { interceptorFor, mayTarget, targetingDiscardsFor } from "./targeting";

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

/**
 * R450, R654, §6.3 Discard: pay a targeting cost of `count` discards — random cards from the
 * player's hand outside `keep`, drawn through the match rng. Fewer cards than the cost ends it; a
 * cost nobody can pay is never listed or offered (`canPayToTarget`, `whyTargetingDiscardsUnpayable`),
 * so the keep is what the refusal kept: the card a play is taking out of that hand, and any hand
 * card the same play or activation picks. A prompt answer keeps nothing, as its refusal does.
 */
export function payTargetingDiscards(
  sink: EngineSink,
  player: PlayerId,
  count: number,
  keep: readonly string[] = [],
): void {
  for (let i = 0; i < Math.max(0, Math.trunc(count)); i += 1) {
    const hand = sink.state.players[player].hand.filter((card) => !keep.includes(card.id));
    if (hand.length === 0) return;
    const card = sink.rng.pick(hand);
    if (card === undefined) return;
    discardFromHand(sink, card);
  }
}

// ---------------------------------------------------------------------------
// A prompt's answer (R450): the cost, then the interception
// ---------------------------------------------------------------------------

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
 * R450, R654: the targeting point of a `target` prompt's answer, which the caller has validated and
 * closed. A costly answer pays its random discards first, then the interception answers.
 */
export function targetAnswer(sink: EngineSink, pending: PendingChoice, picks: readonly Selection[]): Selection[] {
  if (pending.kind !== "target") return [...picks];
  const chooser = pending.playerId;
  const cost = targetingDiscardsFor(sink.state, picks);
  if (cost > 0) payTargetingDiscards(sink, chooser, cost);
  return interceptTargeting(sink, {
    chooser,
    picks,
    what: "target",
    source: sourceOf(sink.state, pending.resume.instanceId),
  });
}
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
