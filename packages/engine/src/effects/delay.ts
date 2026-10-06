// Delayed effects (SPEC §2.2's turn loop, §10.1, §10.6, R62, R68, R76).
//
// "At the start of your next turn …" and "End of turn: …" are not triggers. §2.2 gives them their
// own two points in the turn — after the mana refresh and before the start-of-turn triggers, and
// after the end-of-turn trap window and before cleanup — and R62 fixes that order. `state.delayed`
// is where one waits, `modifiers.scheduleDelayed` puts it there, `modifiers.dueDelayed` picks the
// ones due in R68's creation order and `turn.runDelayed` re-enters them. All of that already
// exists; this file is only the missing verb in front of it, because a card script may not write
// state itself (CLAUDE.md rule 5) and `effects/index.ts` is the whole card-script vocabulary.
//
// WHAT IS STORED IS A `Resume`, NEVER A CLOSURE (§9.3, §10.6). `prompts.resumeSelf` builds it out
// of the running context — this script's def id, the face that is running (§5.2) and the instance
// when it still exists — plus the data this step captures, so the continuation survives a JSON
// round-trip and a replay re-enters the same step with the same data. `instanceId` is deliberately
// optional: R76 has #50 K-Pop Fanatic's steal fire "even if K-Pop Fanatic died", so the continuation
// must be able to outlive its card, which is why a card carries what it needs in `data` rather
// than reaching back through `ctx.self`.

import type { CardMark, PlayerId } from "@jackioh/shared";
import { markDelayed, syncMarks } from "../marks";
import { addStartOfTurnEffect, scheduleDelayed } from "../modifiers";
import { SELF_KEY, resumeSelf } from "../prompts";
import { makeContext, type EngineSink } from "../resolve";
import { RUN_MARKS_KEY } from "../work";
import type { Effect, EffectContext } from "../script";
import { isTurnOf, type DelayedEffect, type Resume } from "../state";
import { destroy, destroyAll } from "./destroy";
import { discardHand } from "./move";
import {
  cardsInScope,
  instanceOf,
  playerOf,
  standsSinceScriptBegan,
  type BoardScope,
  type PlayerSpec,
  type TargetSpec,
} from "./targets";

/**
 * The `Script` key a delayed continuation lands on unless the card names another. `script.ts`
 * documents `delayed` as "a delayed effect this card scheduled, resolved at its R62 point", and it
 * is a `Hook` — a function — which is the only shape `turn.runDelayed` can re-enter today (it goes
 * through `resolve.runHook`, which calls `script[hook]`). A card whose continuation is an entry in
 * its `resume` step table passes `hook: prompts.RESUME_HOOK`; see the gap noted in
 * `test/effects-delay.test.ts`, which `turn.ts` must fix before that spelling works.
 */
export const DELAYED_HOOK = "delayed";

/**
 * When a delayed effect comes due, in the vocabulary a card file writes: §2.2's two points. The
 * player is relative to the controller like every other `PlayerSpec`, or `"turn"`: the player whose
 * turn is running as the effect is made — R350's "at the end of this turn", whoever's turn that is
 * (#90.1 CN-Virus, cast on a draw that may come on either player's turn).
 */
export type DelayAt = { phase: DelayedEffect["at"]["phase"]; player: PlayerSpec | typeof THIS_TURN };

/** R350: `DelayAt.player` for the turn that is running, whoever's it is. */
export const THIS_TURN = "turn";

/** The player a `DelayAt` waits for. During setup, which is no player's turn, "turn" is p1's first (§2.1). */
function delayPlayer(ctx: EffectContext, at: DelayAt): PlayerId {
  return at.player === THIS_TURN ? ctx.state.active : playerOf(ctx, at.player);
}

/**
 * §6.2's "at the start of your next turn" and "end of turn" as one verb (#39 Recycling Initiative,
 * #50 K-Pop Fanatic, #78 /fullsend). `at.player` says whose turn boundary it waits for, relative to
 * the controller like every other `PlayerSpec`; `owner` is the controller, which is both who the
 * continuation runs as and, through R68's creation `seq`, where it sits among several due at once.
 *
 * Scheduling never fizzles: a delay can be armed by a card that is about to exile or sacrifice
 * itself in the same effect list, which is exactly what #39 does, and the entry stays whatever
 * happens to the card afterwards (R76, R86).
 */
export function delay(args: {
  at: DelayAt;
  /** The step the continuation re-enters (§10.6: "script id + step + captured data"). */
  step: string;
  /** The `Script` key that step lives under; `DELAYED_HOOK` unless the card says otherwise. */
  hook?: string;
  /** What the continuation carries across the boundary — the only place it may keep anything. */
  data?: Record<string, unknown>;
  /**
   * R174: the card on the field this effect is aimed at, if any. The effect is forgotten the moment
   * that card leaves the field, so it never lands on a card that left and came back (#50).
   */
  watch?: string;
  /**
   * B5 E27, R458: "at the start / end of your *next* turn" rather than the next such boundary — the
   * boundary of the turn it is made on passes it by (`DelayedEffect.notBefore`), so an end-of-turn
   * clause made on the controller's own turn waits for the end of their next one (Classic #37
   * Radiant). R241 does not drop one made on the other player's turn: that player's next turn exists.
   */
  next?: boolean;
  /**
   * R437: the mark the watched card carries in both views while this effect waits (#50's pending
   * steal, `{ mark: "steal", color: "purple" }`). Ignored without `watch`.
   */
  mark?: CardMark;
}): Effect {
  return {
    kind: "delay",
    apply(ctx): void {
      // R174, R76: an effect aimed at a card on the field is aimed at that stay. A target an earlier
      // effect of the same list has already taken off the field — a fused card's other part bounced
      // it (#52) or sacrificed it (#22) — has no stay left to watch, so the delayed effect fizzles
      // now rather than waiting for whatever later stands under the same id (R78, R83).
      if (args.watch !== undefined && !standsSinceScriptBegan(ctx, args.watch)) return;
      // R241: "End of turn" is the controller's turn end (§6.2), and an end-of-turn clause is the
      // turn's it was made on (#39, #78). One made on the other player's turn — a cast on draw there
      // (R70) — has no end of its controller's turn to wait for, and waiting for the next one would
      // run it at the end of a turn the card was never played on, as R155 keeps a return Spell cast
      // then in the graveyard: it is not scheduled at all.
      if (args.next !== true && endsOtherPlayersTurn(ctx, args.at)) return;
      // `resumeSelf` is the one builder for the def id, the face and the instance id, so a delay
      // and a prompt store the same shape; only the hook differs, and only when a card says so.
      const resume = delayedResume(ctx, args.step, args.data ?? {}, args.hook ?? DELAYED_HOOK);
      const entry = scheduleDelayed(
        ctx,
        ctx.controller,
        { phase: args.at.phase, player: delayPlayer(ctx, args.at) },
        resume,
        args.watch,
        args.next === true ? nextTurnMark(ctx) : undefined,
      );
      // R437: the card it waits for carries the mark until it resolves, fizzles or is forgotten.
      if (args.mark !== undefined) markDelayed(ctx, entry, args.mark);
    },
  };
}

/**
 * The continuation a delayed effect stores: this script's step (`prompts.resumeSelf`) as a run of its
 * own. R127: it re-enters as whatever is left of its card then, so a Death hook's snapshot (R89) is
 * not carried past the hook that read it; nor does it carry the run it was made in
 * (`work.RUN_MARKS_KEY`): it resolves at its own R62 point, and a card it watches is watched through
 * `watch` (R174).
 */
function delayedResume(ctx: EffectContext, step: string, data: Record<string, unknown>, hook: string): Resume {
  const built = resumeSelf(ctx, step, data);
  const { [SELF_KEY]: _snapshot, [RUN_MARKS_KEY]: _run, ...kept } = built.data;
  return { ...built, data: kept, hook };
}

/** R458: the first turn a "next turn" clause may run on — any turn after the one it is made on. */
function nextTurnMark(ctx: EffectContext): number {
  return ctx.state.turn + 1;
}

/**
 * R241: an end-of-turn clause of the controller's own, made on a turn that is not the controller's —
 * the other player's, or setup's, which is no player's turn though `active` names p1 there (§2.1).
 */
function endsOtherPlayersTurn(ctx: EffectContext, at: DelayAt): boolean {
  // R350: "this turn" is the turn running, whoever's it is, so there is no other player's turn to miss.
  if (at.player === THIS_TURN) return false;
  return at.phase === "end" && playerOf(ctx, at.player) === ctx.controller && !isTurnOf(ctx.state, ctx.controller);
}

// ---------------------------------------------------------------------------
// ---- v0.2.0 verbs: activate and turn (B5 E27 delayed kinds, E28 rest of the game) ----
// ---------------------------------------------------------------------------

/**
 * B5 E27: the delayed effects the engine itself resolves, by the `Resume.hook` they are stored under —
 * a verb, not a card's step, so any card can say "destroyed at the start of your next turn" without a
 * `delayed` hook of its own. `turn.ts`'s delayed stage runs these (`runEngineDelayed`) and re-enters a
 * card's step for every other entry (R126).
 */
export const DELAYED_DESTROY_HOOK = "@delayedDestroy";
export const DELAYED_DISCARD_HAND_HOOK = "@delayedDiscardHand";

/**
 * A card's def id for the record an engine delayed effect keeps: the text that made it, if any — the
 * running `ctx.defId` first, as `prompts.resumeSelf` names it (B5 E14, R546).
 */
function makerOf(ctx: EffectContext): { defId: string; radiant: boolean } {
  return { defId: ctx.defId ?? ctx.self?.defId ?? "", radiant: ctx.radiant };
}

/**
 * B5 E27, R458, R174 (Classic #20 The Power to Punish): the unit `target` names is destroyed at the
 * start of its controller's next turn — a destroy (§6.3), so Indestructible ignores it (R46) and the
 * state check after the delayed effect collects it (R59). Aimed at that unit's stay on the field: it
 * fizzles the moment the unit leaves the field, whatever comes back under its id (`watch`, R76, R83).
 * With no unit on the field to aim at, nothing is scheduled.
 *
 * `scope` instead is "all enemy Units are destroyed at the start of your next turn" (the Radiant face):
 * the Units the scope names *then*, read as the delayed effect resolves, not a list fixed now — sides
 * relative to the controller who made it. R748: its `mark` goes on every Unit the scope names while it
 * waits, those that arrive meanwhile too (`refreshScopeMarks`).
 */
export function destroyAtNextTurnStart(
  args: { target: TargetSpec; mark?: CardMark } | { scope: BoardScope; mark?: CardMark },
): Effect {
  return {
    kind: "destroyAtNextTurnStart",
    apply(ctx): void {
      const maker = makerOf(ctx);
      const at = { phase: "start" as const, player: ctx.controller };
      if ("scope" in args) {
        const data = { scope: { ...args.scope }, ...(args.mark === undefined ? {} : { mark: { ...args.mark } }) };
        const resume: Resume = { ...maker, hook: DELAYED_DESTROY_HOOK, step: "scope", data };
        scheduleDelayed(ctx, ctx.controller, at, resume);
        return;
      }
      const unit = instanceOf(ctx, args.target);
      if (unit === null || unit.zone.z !== "field") return;
      const resume: Resume = { ...maker, hook: DELAYED_DESTROY_HOOK, step: "unit", data: { instanceId: unit.id } };
      const entry = scheduleDelayed(ctx, ctx.controller, at, resume, unit.id);
      // R437: the watched unit carries the mark while the destroy waits (Classic #20's red aura).
      if (args.mark !== undefined) markDelayed(ctx, entry, args.mark);
    },
  };
}

/**
 * B5 E27, R458 (Classic #37 Last Hurrah): the controller discards their whole hand at the end of
 * `this` turn — whoever's turn is running, as R350 reads "this turn" — or at the end of their `next`
 * turn (Radiant), which the end of the turn it is made on passes by. A discard (§6.3), so "whenever
 * you discard" sees each card.
 */
export function discardHandAtTurnEnd(args: { turn: "this" | "next" }): Effect {
  return {
    kind: "discardHandAtTurnEnd",
    apply(ctx): void {
      const resume: Resume = { ...makerOf(ctx), hook: DELAYED_DISCARD_HAND_HOOK, step: args.turn, data: {} };
      if (args.turn === "this") {
        scheduleDelayed(ctx, ctx.controller, { phase: "end", player: ctx.state.active }, resume);
        return;
      }
      scheduleDelayed(ctx, ctx.controller, { phase: "end", player: ctx.controller }, resume, undefined, nextTurnMark(ctx));
    },
  };
}

function scopeIn(data: Record<string, unknown>): BoardScope | null {
  const raw: unknown = data.scope;
  return raw !== null && typeof raw === "object" && !Array.isArray(raw) ? (raw as BoardScope) : null;
}

/** R748: the mark a delayed destroy of a scope puts on the Units it names, if it was made with one. */
function markIn(data: Record<string, unknown>): CardMark | null {
  const raw: unknown = data.mark;
  if (raw === null || typeof raw !== "object" || Array.isArray(raw)) return null;
  const { mark, color } = raw as Record<string, unknown>;
  return typeof mark === "string" && typeof color === "string" ? { mark, color } : null;
}

/**
 * R748, R437: every waiting delayed destroy of a scope made with a mark marks the Units its scope names
 * now — the read `runEngineDelayed` makes as it resolves — so a Unit that arrives while it waits is
 * marked and one that leaves is not. Called where the resolution loop collects events
 * (`triggers.collectEvents`), before `marks.sweepMarks` drops the marks of an effect that is gone.
 */
export function refreshScopeMarks(sink: EngineSink): void {
  for (const entry of sink.state.delayed) {
    if (entry.resume.hook !== DELAYED_DESTROY_HOOK) continue;
    const scope = scopeIn(entry.resume.data);
    const mark = markIn(entry.resume.data);
    if (scope === null || mark === null) continue;
    const ctx = makeContext(sink, null, { controller: entry.owner });
    syncMarks(sink, entry.id, mark, cardsInScope(ctx, scope).map((card) => card.id));
  }
}

/**
 * Run one of the engine's own delayed kinds, as its owner. Returns false for an entry that is a card's
 * step, which the caller re-enters through the card's script instead (R126).
 */
export function runEngineDelayed(sink: EngineSink, effect: DelayedEffect): boolean {
  const ctx = makeContext(sink, null, { controller: effect.owner });
  const data = effect.resume.data;
  switch (effect.resume.hook) {
    case DELAYED_DESTROY_HOOK: {
      const scope = scopeIn(data);
      if (scope !== null) destroyAll(scope).apply(ctx);
      else if (typeof data.instanceId === "string") destroy({ target: { of: "instance", instanceId: data.instanceId } }).apply(ctx);
      return true;
    }
    case DELAYED_DISCARD_HAND_HOOK:
      discardHand({ player: "self" }).apply(ctx);
      return true;
    default:
      return false;
  }
}

/**
 * B5 E28, R458 (Classic+ #52): "For the rest of the game: at the start of your turn, …". The card's
 * `step` (under `hook`, `DELAYED_HOOK` unless the card names another) is re-entered at the start of
 * each of `player`'s turns from the next one on, in R62's delayed stage with the delayed effects, in
 * creation order; several stack, each running once per turn. It is the player's effect now, not the
 * card's: it resolves with no `self` whatever became of the card (R127), under the face it was made
 * with, carrying `data`. `label` is its badge (R169), in the card's own words.
 */
export function forRestOfGame(args: {
  step: string;
  label: string;
  hook?: string;
  data?: Record<string, unknown>;
  player?: PlayerSpec;
}): Effect {
  return {
    kind: "forRestOfGame",
    apply(ctx): void {
      const { instanceId: _card, ...resume } = delayedResume(ctx, args.step, args.data ?? {}, args.hook ?? DELAYED_HOOK);
      addStartOfTurnEffect(ctx, playerOf(ctx, args.player ?? "self"), resume, args.label);
    },
  };
}
