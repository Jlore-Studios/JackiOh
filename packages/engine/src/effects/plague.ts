// Plague Tokens as verbs (SPEC §6.3 Plague Token; docs/classic-sets.md B5 E19; R471).
//
// The counter, the multiplier and the `counterChanged` report are `../plague`'s; this file names
// where a placement goes and asks the questions:
//   - `placePlague`: "Place N Plague Tokens on X" — ONE placement of N on the card named (Classic #39
//     Outbreak's target, #59 Plague Doctor's "on this", Classic+ #3's end of turn, #87's "enters with
//     X");
//   - `placePlagueEach`: one placement on each permanent a board scope names (Classic #63 Crop
//     Dusting's "on each permanent", face-down ones included);
//   - `placePlagueRandom`: one placement on each of N different random cards of a scope (Classic #42
//     Transmutable Toxins, R60);
//   - `placePlagueTokens`: "Place N Plague Tokens" with no card named — N placements, all on the one
//     permanent the placer chooses in a single prompt of their own over every permanent on the field,
//     either side, face-down included (R689; Classic #61, #70, #76, #90 reward D);
//   - `consumePlague`: take tokens off (Classic #78 Mutate Spell).
//
// PROMPTS (R113, R122, R689). `placePlagueTokens` opens one `target` prompt naming the single
// permanent all of its placements go on. The prompt is opened by the effect, so the list it stands
// in parks its rest on `state.work` as any asking effect's does; the answer places every placement
// on the pick and drains what the prompt interrupted (`answerPlacement`, registered with
// `prompts.registerPromptAnswerer` for this module's hook). The resume carries only how many tokens
// each placement puts and how many placements land, so a paused chain is plain data that survives
// JSON and replays exactly. The state check waits for the whole effect (R59), so a unit the
// placements shrank to 0 health (an aura reading its tokens, #42) dies after the last placement,
// not between two.

import type { Selection } from "@jackioh/shared";
import { defOf } from "../catalog";
import { permanentsOnField, placePlagueOn, removePlague } from "../plague";
import { castModeForPrompt, preferEnemies } from "../randomCast";
import {
  closePrompt,
  inOfferedOrder,
  openPrompt,
  registerPromptAnswerer,
  resumeAt,
  resumeOf,
  whyAnswerRefused,
  type AnswerInput,
} from "../prompts";
import type { EngineSink } from "../resolve";
import type { Effect, EffectContext } from "../script";
import { findInstance, type CardInstance, type PendingChoice, type Resume } from "../state";
import { beginWorkCascade, drainWork } from "../work";
import { isBuried } from "../zones";
import { cardsInScope, instanceOf, type BoardScope, type TargetSpec } from "./targets";

/** A card the verbs below may put tokens on: a permanent on the field, not dormant (R13). */
function onField(ctx: Pick<EffectContext, "state">, card: CardInstance | null): card is CardInstance {
  return card !== null && card.zone.z === "field" && !isBuried(ctx.state, card);
}

/**
 * "Place N Plague Tokens on X": one placement of `amount` on the card `target` names (default the
 * running card), multiplied by that card's multiplier (R471). Nothing happens for a card that is not
 * a permanent on the field, or for an amount below 1.
 */
export function placePlague(args: { target?: TargetSpec; amount: number }): Effect {
  return {
    kind: "placePlague",
    apply(ctx): void {
      const card = instanceOf(ctx, args.target ?? { of: "self" });
      if (!onField(ctx, card)) return;
      placePlagueOn(ctx, card, args.amount);
    },
  };
}

/**
 * One placement of `amount` on each card a board scope names, in R68's order (`cardsInScope`). Crop
 * Dusting's "each permanent" is `{ side: "any", rows: ["units", "backrow"] }`, which reaches
 * face-down cards, as every backrow card is in a scope's backrow row.
 */
export function placePlagueEach(args: { scope: BoardScope; amount: number }): Effect {
  return {
    kind: "placePlagueEach",
    apply(ctx): void {
      for (const card of cardsInScope(ctx, args.scope)) placePlagueOn(ctx, card, args.amount);
    },
  };
}

/**
 * One placement of `amount` on each of `count` different random cards of a scope (Classic #42's "a
 * Plague Token on each of 2 random Units"): R60's random pick of N existing cards picks N different
 * ones, or all of them if fewer exist. The cards are drawn with `ctx.rng` and placed in R68's order.
 * An empty scope draws nothing (R129).
 */
export function placePlagueRandom(args: { count: number; amount: number; scope?: BoardScope }): Effect {
  return {
    kind: "placePlagueRandom",
    apply(ctx): void {
      const pool = cardsInScope(ctx, args.scope ?? { side: "any", rows: ["units"] });
      const count = Math.trunc(args.count);
      if (pool.length === 0 || count <= 0) return;
      const picked = new Set(ctx.rng.shuffle(pool).slice(0, count).map((card) => card.id));
      for (const card of pool) {
        if (picked.has(card.id)) placePlagueOn(ctx, card, args.amount);
      }
    },
  };
}

/**
 * Take up to `amount` Plague Tokens (default 1) off the card `target` names: "remove a Plague Token
 * from a permanent" (Classic #78). The count floors at 0; a card with none changes nothing.
 */
export function consumePlague(args: { target?: TargetSpec; amount?: number }): Effect {
  return {
    kind: "consumePlague",
    apply(ctx): void {
      const card = instanceOf(ctx, args.target ?? { of: "self" });
      if (card === null) return;
      removePlague(ctx, card, args.amount ?? 1);
    },
  };
}

// ---------------------------------------------------------------------------
// "Place N Plague Tokens": N placements, each a prompt (R471, R113, R122).
// ---------------------------------------------------------------------------

/** The hook every placement prompt names, answered by `answerPlacement` below (R122). */
export const PLAGUE_PLACEMENT_HOOK = "plague:placement";

/** What the one placement prompt carries to its answer: each placement's tokens and how many land. */
type PlacementData = { amount: number; count: number };

function placementData(data: Record<string, unknown>): PlacementData | null {
  const { amount, count } = data;
  if (typeof amount !== "number" || typeof count !== "number") return null;
  return { amount, count };
}

/** The label a placement option shows its chooser; `viewFor` hides a card the chooser may not read (R177). */
function labelOf(ctx: Pick<EffectContext, "state">, card: CardInstance): string {
  return defOf(ctx.state, card.defId).name;
}

/**
 * Open the one placement prompt for `resume`'s placer over every permanent on the field (R68's
 * order, the placer's side first), or return false when there is none — the placements fizzle
 * then, and draw nothing (R129).
 */
function askPlacement(sink: EngineSink, player: PendingChoice["playerId"], resume: Resume): boolean {
  const cards = permanentsOnField(sink.state, player);
  if (cards.length === 0) return false;
  const data = placementData(resume.data);
  const tokens = data?.amount ?? 1;
  const count = Math.max(1, Math.trunc(data?.count ?? 1));
  const total = tokens * count;
  // B5 E12, R452: a random cast's caster is never asked, so under one every placement goes on one
  // random permanent (R60; an enemy one when the cast targets enemies) and nothing pauses —
  // this hook answers its own prompts (`registerPromptAnswerer`), so `openPrompt` would ask instead.
  const mode = castModeForPrompt(sink.state, player, resume.instanceId);
  if (mode?.random === true) {
    const now = permanentsOnField(sink.state, player);
    const pool = mode.targetEnemies ? preferEnemies(sink.state, player, now, (card) => ({ pick: "instance", instanceId: card.id }), 1) : now;
    if (pool.length === 0) return false;
    const card = pool[sink.rng.int(pool.length)];
    if (card === undefined) return false;
    for (let at = 0; at < count; at += 1) placePlagueOn(sink, card, tokens);
    return false;
  }
  const asked = openPrompt(sink, {
    player,
    kind: "target",
    prompt: total === 1 ? "Place a Plague Token on a permanent" : `Place ${total} Plague Tokens on a permanent`,
    options: cards.map((card) => ({
      key: `instance:${card.id}`,
      label: labelOf(sink, card),
      selection: { pick: "instance", instanceId: card.id } satisfies Selection,
    })),
    resume,
  });
  return asked !== null;
}

/**
 * "Place N Plague Tokens" (B5 E19, R471, R689): `count` placements of `amount` (default 1), all on
 * the one permanent the running card's controller chooses — either side, face-down cards included —
 * in a single prompt. The prompt opens as this effect applies; its answer places every placement on
 * the pick, then what the prompt interrupted resumes. With no permanent on the field the placements
 * do nothing. The other player sees only that a prompt is open (§10.6, R81).
 */
export function placePlagueTokens(args: { count: number; amount?: number }): Effect {
  return {
    kind: "placePlagueTokens",
    apply(ctx): void {
      const count = Math.trunc(args.count);
      if (count <= 0) return;
      const resume = resumeAt({
        defId: ctx.self?.defId ?? ctx.defId ?? "",
        hook: PLAGUE_PLACEMENT_HOOK,
        step: "place",
        radiant: ctx.radiant,
        ...(ctx.self === null ? {} : { instanceId: ctx.self.id }),
        data: { amount: Math.max(1, Math.trunc(args.amount ?? 1)), count },
      });
      askPlacement(ctx, ctx.controller, resume);
    },
  };
}

/**
 * R122, R689: the answer to the one placement prompt — validated as any prompt's, closed, every
 * placement made on the pick (a card that is no longer a permanent on the field takes nothing),
 * then what the prompt interrupted resumes (R113).
 */
function answerPlacement(sink: EngineSink, answer: AnswerInput): string | null {
  const pending = sink.state.pending;
  if (pending === null) return "no prompt is open";
  const refused = whyAnswerRefused(pending, answer);
  if (refused !== null) return refused;
  const resume = resumeOf(pending);
  const data = placementData(resume.data);
  if (data === null) return "that prompt carries no placement";
  const picked = inOfferedOrder(pending, answer.selection)[0];

  closePrompt(sink);
  beginWorkCascade(sink);
  const card = picked?.pick === "instance" ? findInstance(sink.state, picked.instanceId) : undefined;
  if (card !== undefined) {
    const times = Math.max(1, Math.trunc(data.count));
    for (let at = 0; at < times; at += 1) placePlagueOn(sink, card, data.amount);
  }

  drainWork(sink);
  return null;
}

registerPromptAnswerer(PLAGUE_PLACEMENT_HOOK, answerPlacement);
