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
//   - `placePlagueTokens`: "Place N Plague Tokens" with no card named — N placements, each on a
//     permanent the placer chooses in a prompt of their own over every permanent on the field, either
//     side, face-down included, repeats allowed (Classic #61, #70, #76, #90 reward D);
//   - `consumePlague`: take tokens off (Classic #78 Mutate Spell).
//
// PROMPTS (R113, R122). Each placement of `placePlagueTokens` is its own `target` prompt. The first
// is opened by the effect, so the list it stands in parks its rest on `state.work` as any asking
// effect's does; the rest are opened by the answer to the one before (`answerPlacement`, registered
// with `prompts.registerPromptAnswerer` for this module's hook), each carrying only how many
// placements are left and how many tokens each puts, so a paused chain is plain data that survives
// JSON and replays exactly. The last answer drains what the first prompt interrupted. The state check
// waits for the whole effect (R59), so a unit a placement shrank to 0 health (an aura reading its
// tokens, #42) dies after the last placement, not between two.

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
import { beginWorkCascade, drainWork, paused } from "../work";
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

/** What a placement prompt carries to its answer: this placement's tokens and how many follow it. */
type PlacementData = { amount: number; left: number };

function placementData(data: Record<string, unknown>): PlacementData | null {
  const { amount, left } = data;
  if (typeof amount !== "number" || typeof left !== "number") return null;
  return { amount, left };
}

/** The label a placement option shows its chooser; `viewFor` hides a card the chooser may not read (R177). */
function labelOf(ctx: Pick<EffectContext, "state">, card: CardInstance): string {
  return defOf(ctx.state, card.defId).name;
}

/**
 * Open the next placement's prompt for `resume`'s placer over every permanent on the field (R68's
 * order, the placer's side first), or return false when there is none — the placements left fizzle
 * then, and draw nothing (R129).
 */
function askPlacement(sink: EngineSink, player: PendingChoice["playerId"], resume: Resume): boolean {
  const cards = permanentsOnField(sink.state, player);
  if (cards.length === 0) return false;
  const data = placementData(resume.data);
  const tokens = data?.amount ?? 1;
  // B5 E12, R452: under a random cast of the placer's every placement is random too, with no prompt —
  // this hook answers its own prompts (`registerPromptAnswerer`), so `openPrompt` would ask instead.
  const mode = castModeForPrompt(sink.state, player, resume.instanceId);
  if (mode?.random === true) {
    for (let left = data?.left ?? 0; left >= 0 && !paused(sink); left -= 1) {
      const now = permanentsOnField(sink.state, player);
      const pool = mode.targetEnemies ? preferEnemies(sink.state, player, now, (card) => ({ pick: "instance", instanceId: card.id }), 1) : now;
      const card = pool[sink.rng.int(pool.length)];
      if (card === undefined) break;
      placePlagueOn(sink, card, tokens);
    }
    return false;
  }
  const asked = openPrompt(sink, {
    player,
    kind: "target",
    prompt: tokens === 1 ? "Place a Plague Token on a permanent" : `Place ${tokens} Plague Tokens on a permanent`,
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
 * "Place N Plague Tokens" (B5 E19, R471): `count` placements of `amount` (default 1), each on a
 * permanent the running card's controller chooses — either side, face-down cards included, the same
 * card as often as they like — in a prompt of its own. The first prompt opens as this effect applies;
 * each answer places, then opens the next; with no permanent on the field the placements left do
 * nothing. The other player sees only that a prompt is open (§10.6, R81).
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
        data: { amount: Math.max(1, Math.trunc(args.amount ?? 1)), left: count - 1 },
      });
      askPlacement(ctx, ctx.controller, resume);
    },
  };
}

/**
 * R122: the answer to a placement prompt — validated as any prompt's, closed, the placement made on
 * the pick (a card that is no longer a permanent on the field takes nothing), then the next
 * placement's prompt; once none is left, or none can be asked, what the first prompt interrupted
 * resumes (R113).
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
  if (card !== undefined) placePlagueOn(sink, card, data.amount);

  if (data.left > 0 && !paused(sink)) {
    const next: Resume = { ...resume, data: { ...resume.data, left: data.left - 1 } };
    if (askPlacement(sink, pending.playerId, next)) return null;
  }
  drainWork(sink);
  return null;
}

registerPromptAnswerer(PLAGUE_PLACEMENT_HOOK, answerPlacement);
