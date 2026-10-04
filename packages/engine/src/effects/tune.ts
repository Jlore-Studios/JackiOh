// Degrade and Upgrade (docs/classic-sets.md B3.4, R386), and Classic+ #41 KY's Constant's "change a
// number to 3": the verbs that write a card's `tuning` (`tuning.ts` holds the readers, `numbers.ts`
// the numbers on a card).
//
// One application is one change, drawn from the menu rows that can change the card now (B3.4 rule 3):
//
//   | Row     | Degrade                          | Upgrade                         | Can apply to
//   | cost    | `costMod` +1, never above (4)    | −1, never below (0)             | never an X-cost card (R65)
//   | stats   | −4 split, k to attack            | +4 split                        | a Unit or an Animated card
//   | keyword | remove one it has                | add one it lacks (R21's pool)   | add: a Unit
//   | x       | one numbered keyword or X worse  | one better                      | never below 1
//   | number  | one declared number a step worse | one step better                 | within its bounds
//
// R442: the draw is the row first, uniformly among the rows that can change the card, then the item
// within the row (which keyword, which number), uniformly, and for a stats row the split k in 0–4. A
// row or an item with one choice draws nothing (R129), and a card no row can change is left alone
// with no draw at all (B3.4 rule 1) — an Immutable card first of all (rule 2).
//
// Events: `degraded` / `upgraded` for each application that changed the card, public on a public
// card. On a card someone may not read — a hand card, a deck card, a face-down trap — the event says
// who could not read it (`hiddenFrom`, R177), and every application is cued, a card nothing changed
// with the change `none` (R440), so the count of cues over a hidden pile numbers the applications,
// never the changes. For the same reason a scope over a hidden pile reaches every card in it: its
// filters decide which cards change, and a card they leave out is cued `none` (`cardScope.ts`).
// `numberChanged` reports KY's Constant's set the same way.
//
// None of these opens a prompt but `discoverNumber`, which is one prompt and nothing after it in the
// same effect, so a pause never splits an application (R113): what a list does after a Degrade is the
// list's own business, parked by `prompts.applyResumable` like any other effect's.

import type { Keyword, KeywordKind, TuningChange } from "@jackioh/shared";
import { hasKeyword } from "@jackioh/shared";
import { activeBrittleCount } from "../brittleCount";
import {
  TUNE_ATTACK_FLOOR,
  TUNE_COST_CAP,
  TUNE_COST_FLOOR,
  TUNE_COST_STEP,
  TUNE_HARMFUL_KEYWORDS,
  TUNE_HEALTH_FLOOR,
  TUNE_STAT_TOTAL,
  TUNE_X_STEP,
} from "../config";
import { cardTypeOf } from "../faces";
import { cardKeywords, printedKeywordsOf, statsWithBuffs, unclampedAttack, unitView } from "../layers";
import { isXCost } from "../mana";
import {
  currentStats,
  numberKey,
  numberOn,
  numberRefId,
  numbersOn,
  numberedKeywordsOn,
  ownCost,
  parseNumberRef,
  type NumberRef,
} from "../numbers";
import { setParam, stepParam, steppableParams } from "../params";
import { openPrompt, resumeSelf } from "../prompts";
import type { Effect, EffectContext } from "../script";
import type { CardInstance, GameState } from "../state";
import { TUNED_FLOOR, X_KEY, addStep, tidyTuning, tunedCount, tuningOf, xOf } from "../tuning";
import { randomPoolKeywords } from "./buff";
import { cardsInCardScope, unreadableBy, type CardScope } from "./cardScope";
import { instanceOnItsStay, resolveTarget, selfOnItsStay, type TargetSpec } from "./targets";

/** Which way a change goes. */
export type TuneDirection = "degrade" | "upgrade";

/**
 * Which cards a Degrade or Upgrade reaches: one named card — a pick the play or a prompt carried
 * (`target`, R81: Classic+ #71 Book of Buff's card, which may be in its caster's hand), `{ of:
 * "self" }` (#69 Buff Billy), or an id a script captured (`instanceId`) — or every card of a `scope`
 * (#8 Withering Storm's Radiant: "every card in your opponent's deck"), or `random` different cards
 * of it (R60: #8's "4 random cards", #70 Chaos Machine's, T-AI-10 Fine-Tuning's). `times` is how many
 * applications each card takes ("5 times" is five draws, B3.4 rule 1), default 1.
 */
export type TuneArgs = {
  target?: TargetSpec;
  instanceId?: string;
  scope?: CardScope;
  random?: number;
  times?: number;
};

// ---------------------------------------------------------------------------
// The menu (B3.4 rule 3)
// ---------------------------------------------------------------------------

/** One applicable row: `apply` makes the change (drawing within the row as it must) and reports it. */
type MenuRow = { row: TuneRow; apply: (ctx: EffectContext) => TuningChange };

/** B3.4 rule 3's menu rows, in its order. */
export type TuneRow = "cost" | "stats" | "keyword" | "x" | "number";

/** The keywords the card has now — all five layers in the unit row (§10.4), layers 1 to 4 elsewhere. */
function keywordsNow(state: GameState, card: CardInstance): Keyword[] {
  const zone = card.zone;
  return zone.z === "field" && zone.row === "units" ? unitView(state, card).keywords : cardKeywords(state, card);
}

/** A uniform pick (R442), drawing nothing when there is one choice (R129). */
function pickOne<T>(ctx: EffectContext, items: readonly T[]): T {
  const only = items[0];
  if (items.length === 1 && only !== undefined) return only;
  const picked = ctx.rng.pick(items);
  if (picked === undefined) throw new Error("B3.4: a menu row was offered with nothing in it");
  return picked;
}

/** B3.4 rule 3, cost: `costMod` one step toward (4) or toward (0); never an X-cost card (R65). */
function costRow(state: GameState, card: CardInstance, direction: TuneDirection): MenuRow | null {
  const own = ownCost(state, card);
  if (own === null) return null;
  const delta =
    direction === "degrade"
      ? Math.min(TUNE_COST_STEP, TUNE_COST_CAP - own)
      : 0 - Math.min(TUNE_COST_STEP, own - TUNE_COST_FLOOR);
  if ((direction === "degrade" && delta <= 0) || (direction === "upgrade" && delta >= 0)) return null;
  return {
    row: "cost",
    apply: () => {
      card.costMod += delta;
      return { kind: "cost", delta };
    },
  };
}

/**
 * B3.4 rule 3, stats: a split of `TUNE_STAT_TOTAL` rolled as k to attack and the rest to health. A
 * Degrade's attack floors at 0 and its current health at 1, and what the floors refuse is lost; so a
 * Degrade can apply only while the card has attack above 0 or health above 1. On the field the change
 * moves max health, damage staying, so current health moves with it (B3.4 rule 6).
 */
function statsRow(state: GameState, card: CardInstance, direction: TuneDirection): MenuRow | null {
  const stats = currentStats(state, card);
  if (stats === null) return null;
  if (direction === "degrade" && stats.attack <= TUNE_ATTACK_FLOOR && stats.health <= TUNE_HEALTH_FLOOR) return null;
  return {
    row: "stats",
    apply: (at) => {
      const k = at.rng.int(TUNE_STAT_TOTAL + 1);
      const rest = TUNE_STAT_TOTAL - k;
      // `0 - n`, never `-n`: a share the floors refuse whole is 0, not −0, in the event and the record.
      const attack = direction === "upgrade" ? k : 0 - Math.min(k, Math.max(0, stats.attack - TUNE_ATTACK_FLOOR));
      const health =
        direction === "upgrade" ? rest : 0 - Math.min(rest, Math.max(0, stats.health - TUNE_HEALTH_FLOOR));
      const tuning = tuningOf(card);
      tuning.attack = (tuning.attack ?? 0) + attack;
      tuning.health = (tuning.health ?? 0) + health;
      tidyTuning(card);
      return { kind: "stats", attack, health };
    },
  };
}

const HARMFUL: readonly KeywordKind[] = TUNE_HARMFUL_KEYWORDS;

/**
 * B3.4 rule 3, keyword. A Degrade removes one keyword the card has of its own — printed (as tuning
 * leaves it) or granted, never one an aura or its position lends it, and never a harmful one (Can't
 * attack, Brittle): every entry of the kind goes, a printed one by `tuning.removeKeywords`, an added
 * one out of `tuning.addKeywords`, a granted one off the instance. An Upgrade adds one R21 keyword a
 * Unit lacks (`tuning.addKeywords`); a Vanilla unit's text is gone, and an added keyword would be
 * text, so nothing is added to one.
 */
function keywordRow(state: GameState, card: CardInstance, direction: TuneDirection): MenuRow | null {
  if (direction === "degrade") {
    const own = cardKeywords(state, card).filter((keyword) => !HARMFUL.includes(keyword.kind));
    const kinds = [...new Set(own.map((keyword) => keyword.kind))];
    if (kinds.length === 0) return null;
    return {
      row: "keyword",
      apply: (at) => {
        const kind = pickOne(at, kinds);
        const shown = own.find((keyword) => keyword.kind === kind);
        removeKind(at.state, card, kind);
        return { kind: "keyword", keyword: shown ?? ({ kind } as Keyword), added: false };
      },
    };
  }
  if (cardTypeOf(state, card) !== "Unit" || card.vanilla) return null;
  const held = keywordsNow(state, card);
  const candidates = randomPoolKeywords().filter((keyword) => !hasKeyword(held, keyword.kind));
  if (candidates.length === 0) return null;
  return {
    row: "keyword",
    apply: (at) => {
      const keyword = pickOne(at, candidates);
      const tuning = tuningOf(card);
      tuning.addKeywords = [...(tuning.addKeywords ?? []), keyword];
      // §10.4: a keyword the card gains anew is up, as a granted one is (`buff.grantTo`).
      if (keyword.kind === "Divine Shield") delete card.divineShieldSpent;
      if (keyword.kind === "Reborn") delete card.rebornSpent;
      return { kind: "keyword", keyword, added: true };
    },
  };
}

function removeKind(state: GameState, card: CardInstance, kind: KeywordKind): void {
  const tuning = tuningOf(card);
  if (printedKeywordsOf(state, card).some((keyword) => keyword.kind === kind)) {
    if (!(tuning.removeKeywords ?? []).includes(kind)) tuning.removeKeywords = [...(tuning.removeKeywords ?? []), kind];
  }
  tuning.addKeywords = (tuning.addKeywords ?? []).filter((keyword) => keyword.kind !== kind);
  card.grantedKeywords = card.grantedKeywords.filter((keyword) => keyword.kind !== kind);
  tidyTuning(card);
}

/** One number the X row may move: its key, its value now and after one step, and the write. */
type XItem = { key: string; before: number; after: number; write: () => void };

/** A copy of the card's tuning with one more X step on `key`, to read the value the step would give. */
function withXStep(card: CardInstance, key: string, delta: number): Pick<CardInstance, "tuning"> {
  return { tuning: { ...(card.tuning ?? {}), x: addStep(card.tuning?.x, key, delta) } };
}

/**
 * B3.4 rule 3, X: one numbered keyword or an X-cost card's X, one step (`TUNE_X_STEP`) worse or
 * better — more is better for all but Tribute, and nothing goes below 1 (`TUNED_FLOOR`). A Brittle
 * count in force moves itself (B3.3 rule 5); a printed Brittle that has not started moves the number
 * it will start at. An X-cost card with no chosen X (off the field) has its X still to come, so its
 * step always applies ("its X counts 1 less or more when it resolves"). On the field its X has
 * resolved, so it has no X to move (SPEC §8.7 row 69: Classic+ #69 Buff Billy's Cry Upgrades draw
 * from the stats and keyword rows only); elsewhere one applies while the X it counts can move.
 */
function xItems(state: GameState, card: CardInstance, direction: TuneDirection): XItem[] {
  const items: XItem[] = [];
  const better = direction === "upgrade" ? TUNE_X_STEP : -TUNE_X_STEP;
  const step = (key: string, delta: number) => (): void => {
    const tuning = tuningOf(card);
    tuning.x = addStep(tuning.x, key, delta);
    tidyTuning(card);
  };

  if (isXCost(state, card) && card.zone.z !== "field") {
    if (card.x === undefined) {
      items.push({ key: X_KEY, before: 0, after: better, write: step(X_KEY, better) });
    } else {
      const before = xOf(card);
      const after = tunedCount(withXStep(card, X_KEY, better), X_KEY, card.x);
      if (after !== before) items.push({ key: X_KEY, before, after, write: step(X_KEY, better) });
    }
  }

  for (const entry of numberedKeywordsOn(state, card)) {
    const delta = entry.better === "up" ? better : -better;
    if (entry.printed === null) {
      const brittle = card.brittle;
      if (brittle === undefined) continue;
      const after = Math.max(TUNED_FLOOR, entry.value + delta);
      if (after === entry.value) continue;
      items.push({
        key: entry.key,
        before: entry.value,
        after,
        write: () => {
          card.brittle = { ...brittle, count: after };
        },
      });
      continue;
    }
    const after = tunedCount(withXStep(card, entry.key, delta), entry.key, entry.printed);
    if (after === entry.value) continue;
    items.push({ key: entry.key, before: entry.value, after, write: step(entry.key, delta) });
  }
  return items;
}

function xRow(state: GameState, card: CardInstance, direction: TuneDirection): MenuRow | null {
  const items = xItems(state, card, direction);
  if (items.length === 0) return null;
  return {
    row: "x",
    apply: (at) => {
      const item = pickOne(at, items);
      item.write();
      return { kind: "x", key: item.key, delta: item.after - item.before };
    },
  };
}

/** B3.4 rule 3, number: one declared number one step worse or better (`params.steppableParams`). */
function numberRow(state: GameState, card: CardInstance, direction: TuneDirection): MenuRow | null {
  const items = steppableParams(state, card, direction);
  if (items.length === 0) return null;
  return {
    row: "number",
    apply: (at) => {
      const item = pickOne(at, items);
      stepParam(card, item.param.key, item.steps);
      return { kind: "number", key: item.param.key, delta: item.delta };
    },
  };
}

/** B3.4 rules 1–3: the rows that can change the card now, in the menu's order. */
function menuOf(state: GameState, card: CardInstance, direction: TuneDirection): MenuRow[] {
  // Rule 2: an Immutable card is never changed, and nothing is drawn for it.
  if (hasKeyword(keywordsNow(state, card), "Immutable")) return [];
  return [
    costRow(state, card, direction),
    statsRow(state, card, direction),
    keywordRow(state, card, direction),
    xRow(state, card, direction),
    numberRow(state, card, direction),
  ].filter((row): row is MenuRow => row !== null);
}

/**
 * B3.4 rule 3: the rows that can change the card now, in the menu's order — a pure read, for a
 * `conditionMet` or a `targetChecks` predicate (a Degrade with nothing to change) and for the tests.
 */
export function applicableChanges(state: GameState, card: CardInstance, direction: TuneDirection): TuneRow[] {
  return menuOf(state, card, direction).map((row) => row.row);
}

// ---------------------------------------------------------------------------
// One application, and the cards a verb reaches
// ---------------------------------------------------------------------------

/**
 * B3.4 rule 1: one Degrade or Upgrade of one card — a row drawn among those that can change it, then
 * the change. `matches` is false for a card of a hidden pile the scope's filters left out, which is
 * cued and never changed (R440). Reported as this file's header says.
 */
export function tuneOnce(ctx: EffectContext, card: CardInstance, direction: TuneDirection, matches = true): void {
  // R177: who could not read the card where it changed, judged before the change moves anything.
  const hiddenFrom = unreadableBy(ctx.state, card);
  const menu = matches ? menuOf(ctx.state, card, direction) : [];
  if (menu.length === 0 && hiddenFrom.length === 0) return;
  const change: TuningChange = menu.length === 0 ? { kind: "none" } : pickOne(ctx, menu).apply(ctx);
  ctx.events.push({
    type: direction === "upgrade" ? "upgraded" : "degraded",
    instanceId: card.id,
    defId: card.defId,
    change,
    ...(hiddenFrom.length === 0 ? {} : { hiddenFrom }),
  });
}

/** The one card a verb names, by id or by spec, in whatever zone it is (B3.4 rule 2). */
function namedCard(ctx: EffectContext, args: { target?: TargetSpec; instanceId?: string }): CardInstance | null {
  // R174: a card named by id is aimed at the stay it had when the run began (`instanceOnItsStay`).
  if (args.instanceId !== undefined) return instanceOnItsStay(ctx, args.instanceId);
  if (args.target === undefined) return null;
  const target = resolveTarget(ctx, args.target);
  if (target === null || target.kind !== "unit") return null;
  // A card that has ceased to exist (R11, R86) is in no pile to change.
  return target.instance.zone.z === "gone" ? null : target.instance;
}

/**
 * The cards a verb over `args` reaches, each with whether it may change: the one named card, or a
 * scope's cards — every card of the scope, or `random` different ones (R60), drawn uniformly over
 * the scope with every card of a hidden pile in it whatever the filters say (R440), so the odds and
 * the count of what is reached hang on the piles' sizes and the public cards alone. The picks are
 * applied in the scope's order (R242), never in the order they were drawn. A pick of at least as many
 * cards as there are takes them all and draws nothing (R129).
 */
export function reachedCards(
  ctx: EffectContext,
  args: { target?: TargetSpec; instanceId?: string; scope?: CardScope; random?: number },
): { card: CardInstance; matches: boolean }[] {
  if (args.scope === undefined) {
    const card = namedCard(ctx, args);
    return card === null ? [] : [{ card, matches: true }];
  }
  const pool = cardsInCardScope(ctx, args.scope, { wholeHiddenPiles: true });
  if (args.random === undefined) return pool;
  const count = Math.max(0, Math.trunc(args.random));
  if (count === 0) return [];
  if (pool.length <= count) return pool;
  const picked = new Set(ctx.rng.shuffle(pool.map((entry) => entry.card.id)).slice(0, count));
  return pool.filter((entry) => picked.has(entry.card.id));
}

function tuneEffect(kind: string, direction: TuneDirection, args: TuneArgs): Effect {
  return {
    kind,
    apply(ctx): void {
      const times = Math.max(0, Math.trunc(args.times ?? 1));
      if (times === 0) return;
      for (const { card, matches } of reachedCards(ctx, args)) {
        for (let i = 0; i < times; i += 1) tuneOnce(ctx, card, direction, matches);
      }
    },
  };
}

/**
 * B3.4, R386: Degrade — `times` applications to each card reached, each one change drawn from the
 * menu (this file's header). Classic+ #8 Withering Storm ("Degrade 4 random cards in your opponent's
 * deck": `{ scope: { side: "enemy", zones: ["library"] }, random: 4 }`), #72 Book of Nerf, #70 Chaos
 * Machine, #73's "Degrade every card on your opponent's field and in their hand three times".
 */
export function degrade(args: TuneArgs): Effect {
  return tuneEffect("degrade", "degrade", args);
}

/**
 * B3.4, R386: Upgrade — Degrade's mirror (B9 #7). Classic+ #69 Buff Billy ("Upgrade this X times":
 * `{ target: { of: "self" }, times: ctx.x }`), #71 Book of Buff, #70, #73's "every card in your hand
 * and deck twice", T-AI-10 Fine-Tuning.
 */
export function upgrade(args: TuneArgs): Effect {
  return tuneEffect("upgrade", "upgrade", args);
}

/**
 * R656, Core #98's Steady Shot: "Upgrade this permanently by +2 damage" — an Upgrade of the card
 * running the text whose change is named rather than drawn: `steps` steps of its declared number
 * `key` toward better (the number row, `params.steppableParams`), kept in `tuning` like every other
 * (R386), so nothing is drawn. An Immutable card is not changed (B3.4 rule 2), and a number at the
 * bound it would pass stops there. Reported by `upgraded` with the change made, hidden as any
 * Upgrade's is (R177); a change of nothing is not reported.
 */
export function upgradeOwnNumber(args: { key: string; steps?: number }): Effect {
  return {
    kind: "upgradeOwnNumber",
    apply(ctx): void {
      // R174: "this" is the card on the stay the run began with; one that has left since is not it.
      const card = selfOnItsStay(ctx);
      if (card === null || hasKeyword(keywordsNow(ctx.state, card), "Immutable")) return;
      const hiddenFrom = unreadableBy(ctx.state, card);
      let delta = 0;
      for (let step = 0; step < Math.max(0, Math.trunc(args.steps ?? 1)); step += 1) {
        const item = steppableParams(ctx.state, card, "upgrade").find((entry) => entry.param.key === args.key);
        if (item === undefined) break;
        stepParam(card, item.param.key, item.steps);
        delta += item.delta;
      }
      if (delta === 0) return;
      ctx.events.push({
        type: "upgraded",
        instanceId: card.id,
        defId: card.defId,
        change: { kind: "number", key: args.key, delta },
        ...(hiddenFrom.length === 0 ? {} : { hiddenFrom }),
      });
    },
  };
}

// ---------------------------------------------------------------------------
// KY's Constant: a number set outright (Classic+ #41)
// ---------------------------------------------------------------------------

/**
 * Classic+ #41: set one number on a card to `value`. It is `tuning` (B3.4 rule 4), so it stays with
 * the card: the cost by `costMod`, attack and health by the stats delta that makes them read `value`
 * now (current health on the field), a numbered keyword and a declared number by `tuning.set` (a
 * Brittle count in force is set itself). The steps recorded on that number before are spent.
 */
function writeNumber(ctx: EffectContext, card: CardInstance, ref: NumberRef, value: number): void {
  const state = ctx.state;
  if (ref.kind === "cost") {
    const own = ownCost(state, card);
    if (own !== null) card.costMod += value - own;
    return;
  }
  if (ref.kind === "attack" || ref.kind === "health") {
    const onField = card.zone.z === "field" && card.zone.row === "units";
    const now =
      ref.kind === "attack"
        ? onField
          ? unclampedAttack(state, card)
          : statsWithBuffs(state, card).attack
        : onField
          ? unitView(state, card).health
          : statsWithBuffs(state, card).maxHealth - card.damage;
    const tuning = tuningOf(card);
    tuning[ref.kind] = (tuning[ref.kind] ?? 0) + (value - now);
    tidyTuning(card);
    return;
  }
  if (ref.kind === "param") {
    setParam(card, ref.key, value);
    return;
  }
  if (ref.key === "Brittle" && activeBrittleCount(card) !== null && card.brittle !== undefined) {
    card.brittle = { ...card.brittle, count: Math.max(0, value) };
    return;
  }
  const tuning = tuningOf(card);
  tuning.set = { ...(tuning.set ?? {}), [ref.key]: value };
  const x = { ...(tuning.x ?? {}) };
  delete x[ref.key];
  tuning.x = x;
  tidyTuning(card);
}

/**
 * Classic+ #41 KY's Constant: change a number on a card to `value`. `which` names the number
 * (`numbers.NumberRef`, or its id as a prompt option carries it, `numberRefId`), or is `"random"`: a
 * uniform pick (R60) among the card's numbers that are not `value` already, which changes nothing
 * and draws nothing when there is none (R129). The card is named as a Degrade's is. A number the
 * card does not have, and an Immutable card (B3.4 rule 2), are left alone.
 *
 * Reported by `numberChanged` with the number it came to (a declared number's bounds may hold it off
 * `value`), hidden like a Degrade's event (R177, `hiddenFrom`) — the base face picks at random on a
 * card in its caster's hand, so which number moved is the card's to keep.
 */
export function setNumber(args: {
  target?: TargetSpec;
  instanceId?: string;
  which: NumberRef | string | "random";
  value: number;
}): Effect {
  return {
    kind: "setNumber",
    apply(ctx): void {
      const card = namedCard(ctx, args);
      if (card === null) return;
      const value = Math.trunc(args.value);
      const ref = refFor(ctx, card, args.which, value);
      if (ref === null) return;
      const hiddenFrom = unreadableBy(ctx.state, card);
      writeNumber(ctx, card, ref, value);
      ctx.events.push({
        type: "numberChanged",
        instanceId: card.id,
        defId: card.defId,
        key: numberKey(ref),
        value: numberOn(ctx.state, card, ref) ?? value,
        ...(hiddenFrom.length === 0 ? {} : { hiddenFrom }),
      });
    },
  };
}

/** The number `which` names on the card, or null when the card has no such number to change. */
function refFor(ctx: EffectContext, card: CardInstance, which: NumberRef | string, value: number): NumberRef | null {
  const numbers = numbersOn(ctx.state, card);
  if (which === "random") {
    const open = numbers.filter((entry) => entry.value !== value);
    if (open.length === 0) return null;
    return pickOne(ctx, open).ref;
  }
  const ref = typeof which === "string" ? parseNumberRef(which) : which;
  if (ref === null) return null;
  const id = numberRefId(ref);
  return numbers.some((entry) => entry.id === id) ? ref : null;
}

/** The key under which `discoverNumber` hands its card's id to the step its answer re-enters. */
export const NUMBER_CARD_KEY = "numberOf";

/**
 * Classic+ #41 KY's Constant's Radiant: "Discover a number on it" — up to `count` different numbers
 * on the card that are not `value` already, drawn uniformly (R60; all of them, with no draw, when
 * there are no more than `count`, R129), offered to the card's controller in a `discover` prompt whose
 * options are the numbers' ids (`numberRefId`) labelled with what each is now. The answer re-enters
 * the card's own step `step`, whose data carries the card's id under `NUMBER_CARD_KEY`; the step
 * reads the pick with `chosenTuningNumber` (not E18's `number` prompt reader: this answer names a
 * number on a card) and sets it with `setNumber`. The options name the caster's own card's numbers
 * and go to the caster alone (R81). No number to offer: nothing opens, and the list goes on.
 */
export function discoverNumber(args: {
  target?: TargetSpec;
  instanceId?: string;
  value: number;
  count: number;
  step: string;
  prompt?: string;
}): Effect {
  return {
    kind: "discoverNumber",
    apply(ctx): void {
      const card = namedCard(ctx, args);
      if (card === null) return;
      const open = numbersOn(ctx.state, card).filter((entry) => entry.value !== Math.trunc(args.value));
      const count = Math.max(0, Math.trunc(args.count));
      if (open.length === 0 || count === 0) return;
      const offered =
        open.length <= count
          ? open
          : (() => {
              const ids = new Set(ctx.rng.shuffle(open.map((entry) => entry.id)).slice(0, count));
              return open.filter((entry) => ids.has(entry.id));
            })();
      openPrompt(ctx, {
        player: ctx.controller,
        kind: "discover",
        prompt: args.prompt ?? "Choose a number",
        options: offered.map((entry) => ({
          key: `mode:${entry.id}`,
          label: `${entry.label} (${entry.value})`,
          selection: { pick: "mode", option: entry.id },
        })),
        resume: resumeSelf(ctx, args.step, { [NUMBER_CARD_KEY]: card.id }),
      });
    },
  };
}

/** The number a `discoverNumber` answer picked and the card it is on, for the step it re-enters. */
export function chosenTuningNumber(ctx: EffectContext): { instanceId: string; which: NumberRef } | null {
  const pick = ctx.targets[0];
  const instanceId = ctx.data[NUMBER_CARD_KEY];
  if (pick?.pick !== "mode" || typeof instanceId !== "string") return null;
  const which = parseNumberRef(pick.option);
  return which === null ? null : { instanceId, which };
}
