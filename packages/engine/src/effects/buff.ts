// Permanent stat buffs and keyword grants. A buff is layer 4 of §10.4, under the auras of layer 5,
// so it is stored on the instance and survives the aura that came and went; §10.4 computes the
// totals, so nothing here ever writes a stat. R78 drops both when the card leaves the field.
//
// B5 E38 (patch v0.2.0): a buff or a granted keyword may land on a card in a hand or a deck as well —
// a named card anywhere, or `buffCards` / `grantKeywordCards` over a card scope — and rides it onto
// the field: a draw, a play and a Recruit move a card without R78's reset, so what it gained in its
// hand or deck is still on it there (R243 shows it to the hand's owner). It goes when the card leaves
// the field (R78), or reaches a graveyard or an exile pile from a hand or a deck (R215).

import type { Keyword } from "@jackioh/shared";
import { hasKeyword } from "@jackioh/shared";
import { RANDOM_KEYWORD_POOL } from "../config";
import { unitView } from "../layers";
import type { Effect, EffectContext } from "../script";
import type { CardInstance } from "../state";
import { activeUnitsOf } from "../zones";
import { cardsInCardScope, type CardScope } from "./cardScope";
import { playerOf, type PlayerSpec, resolveTarget, type TargetSpec } from "./targets";

/** A stat change in attack, max health, or both (#4 Gary, #43 Friend of Felinors, #63). */
export type BuffAmount = { attack?: number; health?: number };

/** R21's pool is written as text in config; "Armor 1" is its one numbered entry (§6.1). */
const POOL_KEYWORDS: Record<(typeof RANDOM_KEYWORD_POOL)[number], Keyword> = {
  Taunt: { kind: "Taunt" },
  "Armor 1": { kind: "Armor", n: 1 },
  Rush: { kind: "Rush" },
  Charge: { kind: "Charge" },
  "First Strike": { kind: "First Strike" },
  Poisonous: { kind: "Poisonous" },
  Lifesteal: { kind: "Lifesteal" },
  Reborn: { kind: "Reborn" },
  "Divine Shield": { kind: "Divine Shield" },
  Trample: { kind: "Trample" },
  Cleave: { kind: "Cleave" },
  Pierce: { kind: "Pierce" },
};

/** R21's pool as keywords, in the pool's order: what a random keyword is drawn from (B3.4's Upgrade too). */
export function randomPoolKeywords(): Keyword[] {
  return RANDOM_KEYWORD_POOL.map((entry) => POOL_KEYWORDS[entry]);
}

/**
 * A layer-4 buff on one card. `report` false leaves out the `buffed` event: R440's silent change to a
 * card of a hidden pile a scope reached (`buffCards`).
 */
function applyBuff(ctx: EffectContext, unit: CardInstance, amount: BuffAmount, report = true): void {
  const attack = Math.trunc(amount.attack ?? 0);
  const health = Math.trunc(amount.health ?? 0);
  if (attack === 0 && health === 0) return;
  unit.buffs.attack += attack;
  unit.buffs.health += health;
  // The event carries the change this buff made; a unit's totals are read through `unitView`.
  if (report) ctx.events.push({ type: "buffed", instanceId: unit.id, attack, health });
}

/** "+X/+Y" on one unit: a permanent layer-4 buff (§10.4). */
export function buff(args: { target: TargetSpec } & BuffAmount): Effect {
  return {
    kind: "buff",
    apply(ctx): void {
      const target = resolveTarget(ctx, args.target);
      if (target === null || target.kind !== "unit") return;
      applyBuff(ctx, target.instance, args);
    },
  };
}

/** "+X/+Y to every unit you control" (#43), in lane order, one `buffed` event each. */
export function buffAllUnits(args: { side?: PlayerSpec | "both" } & BuffAmount): Effect {
  return {
    kind: "buffAllUnits",
    apply(ctx): void {
      const side = args.side ?? "self";
      const players =
        side === "both" ? [playerOf(ctx, "self"), playerOf(ctx, "enemy")] : [playerOf(ctx, side)];
      for (const player of players) {
        for (const unit of activeUnitsOf(ctx.state, player)) applyBuff(ctx, unit, args);
      }
    },
  };
}

/**
 * Add a keyword to `grantedKeywords` (§10.4). Keywords are a set, so a kind the unit was already
 * granted is not stored twice, while Armor and Lucky carry a number and sum across sources (§6.1).
 * A spent Divine Shield and a used Reborn come back when the keyword is granted again (§10.4).
 */
function grantTo(ctx: EffectContext, unit: CardInstance, keyword: Keyword, report = true): void {
  if (keyword.kind === "Divine Shield") delete unit.divineShieldSpent;
  if (keyword.kind === "Reborn") delete unit.rebornSpent;

  const stacks = keyword.kind === "Armor" || keyword.kind === "Lucky";
  const already = unit.grantedKeywords.some((k) => k.kind === keyword.kind);
  if (stacks || !already) unit.grantedKeywords.push(keyword);

  if (report) ctx.events.push({ type: "keywordGranted", instanceId: unit.id, keyword });
}

export function grantKeyword(args: { target: TargetSpec; keyword: Keyword }): Effect {
  return {
    kind: "grantKeyword",
    apply(ctx): void {
      const target = resolveTarget(ctx, args.target);
      if (target === null || target.kind !== "unit") return;
      grantTo(ctx, target.instance, args.keyword);
    },
  };
}

/**
 * Every pool keyword this unit does not have yet, read through the layers (§10.4, R21). A unit
 * that already has Armor from any source is not offered "Armor 1", as R21 counts by keyword.
 */
function poolCandidates(ctx: EffectContext, unit: CardInstance): Keyword[] {
  const held = unitView(ctx.state, unit).keywords;
  return randomPoolKeywords().filter((keyword) => !hasKeyword(held, keyword.kind));
}

/**
 * R21: #63 Plastic Surgery's and #80 Zao Gao's random keywords. Each draw comes from `ctx.rng`,
 * never picks a keyword the unit already has, and never repeats inside one grant; a unit that
 * already holds the whole pool gets nothing.
 */
export function grantRandomKeywords(args: { target: TargetSpec; count?: number }): Effect {
  return {
    kind: "grantRandomKeywords",
    apply(ctx): void {
      const target = resolveTarget(ctx, args.target);
      if (target === null || target.kind !== "unit") return;
      const unit = target.instance;
      const count = Math.max(0, Math.trunc(args.count ?? 1));

      for (let i = 0; i < count; i += 1) {
        // Recomputed each draw, so the keyword just granted is out of the pool for the next one.
        const candidates = poolCandidates(ctx, unit);
        if (candidates.length === 0) return;
        const keyword = ctx.rng.pick(candidates);
        if (keyword === undefined) return;
        grantTo(ctx, unit, keyword);
      }
    },
  };
}

/**
 * B5 E38: every card a card scope reaches, on the field and in hands and decks, gets "+X/+Y" (Classic+
 * #40 Appropriations' Military and Healthcare: "your Units on the field, in your hand and in your deck
 * get +2X Attack", `{ scope: { zones: ["field", "hand", "library"], types: ["Unit"] }, attack }`). A
 * card on the field acts at once; one in a hand or a deck carries it onto the field. R440: a card of a
 * pile someone may not read (a hand, a deck, a face-down trap) changes silently — a `buffed` there
 * would count the cards the scope's filters let through — and its owner reads its stats off its view
 * (R243); a public card's buff is reported as `buff`'s is.
 */
export function buffCards(args: { scope: CardScope } & BuffAmount): Effect {
  return {
    kind: "buffCards",
    apply(ctx): void {
      for (const { card, readers } of cardsInCardScope(ctx, args.scope)) {
        applyBuff(ctx, card, args, readers === "everyone");
      }
    },
  };
}

/**
 * B5 E38: every card a card scope reaches gains `keyword` (Military's Rush, Healthcare's Armor X,
 * Classic+ #77 Anti-Softlock's Stack and Pierce on every card on the field and in hands and decks),
 * carried onto the field as `buffCards`' stats are and reported the same way (R440).
 */
export function grantKeywordCards(args: { scope: CardScope; keyword: Keyword }): Effect {
  return {
    kind: "grantKeywordCards",
    apply(ctx): void {
      for (const { card, readers } of cardsInCardScope(ctx, args.scope)) {
        grantTo(ctx, card, args.keyword, readers === "everyone");
      }
    },
  };
}
