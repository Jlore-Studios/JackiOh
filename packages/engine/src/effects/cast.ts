// Cast, from anywhere (SPEC §6.3's Cast row, docs/classic-sets.md B5 E12; R70, R452, R453), and the
// price rules a card installs on a player (B5 E15, E39; R455).
//
// A cast is §10.5's pipeline entered at step 3 (`resolve.castCard`): free, counted as a play (R70),
// announced like one (E1), its Cry or spell script fired, its choices its caster's — or, for a random
// cast, the rng's (R452). These verbs only choose the card and how it is cast:
//
//  - `cast` — a card that exists, in a hand, a library, a graveyard or an exile pile (Classic #56 Spell
//    Tyrant's Spells, cast out of the graveyard and exiled after, R453);
//  - `castEach` — every card of a set read once as the list reaches it (#56 Radiant's "every Spell in
//    your graveyard, oldest first"), so a pause inside one cast resumes over the same set (R113);
//  - `castNew` — a new card of a named definition (Classic #47 casts #34 Ancient Acquisition; #7
//    InfiniScepter's copy of the Spell it remembers; Classic+ #37 Wardrum's copies), which the caster
//    owns and which lands in the caster's graveyard after it resolves (R87);
//  - `castRandom` — random cards from a pool, every choice random (Classic+ #47 Jogg's Box, #38.1
//    Solarius-Prime), never the casting card's own definition (B4.1, R387).
//
// A cast verb that would begin a cast past RANDOM_CAST_CHAIN_CAP inside a random cast's chain resolves
// into nothing (R452), as R28's Call to Chaos cap does.

import type { Enchantment } from "@jackioh/shared";
import { excludingDefId, query, type CatalogQueryArgs } from "../catalog";
import type { CostRule } from "../costRules";
import { addModifier } from "../modifiers";
import { mayCastNow } from "../randomCast";
import { castCard, lazyPart, type CastOptions, type EngineSink } from "../resolve";
import type { Effect, EffectContext } from "../script";
import { newInstance, type CardInstance, type ModifierExpiry } from "../state";
import { forEachCard } from "./each";
import { instanceOf, playerOf, type PlayerSpec, type TargetSpec } from "./targets";

/** How a cast verb casts (`resolve.CastOptions`, without the choices R70 leaves to the caster). */
export type CastHow = {
  /** R452: every choice at random. */
  random?: boolean;
  /** R452: target picks narrow to enemies when one is legal ("They target enemies when they can"). */
  targetEnemies?: boolean;
  /** R453: a resolved Spell goes to exile instead of its graveyard ("Cast them, then exile them"). */
  afterward?: "exile";
};

/** The piles a card that exists may be cast from: never the field, where it already is. */
const CASTABLE_ZONES: readonly string[] = ["hand", "library", "graveyard", "exile"];

function sinkOf(ctx: EffectContext): EngineSink {
  return { state: ctx.state, events: ctx.events, rng: ctx.rng };
}

function optionsOf(how: CastHow): CastOptions {
  return {
    ...(how.random === true ? { random: true } : {}),
    ...(how.targetEnemies === true ? { targetEnemies: true } : {}),
    ...(how.afterward === undefined ? {} : { afterward: how.afterward }),
  };
}

/**
 * E12, R453: cast a card that exists (default: the first chosen one). It is cast by the running card's
 * controller, who makes its choices (R70), from whatever pile it lies in; its owner does not change,
 * so it lands in its owner's piles afterwards. A card on the field, or one that has ceased to exist,
 * is not cast.
 */
export function cast(args: { target?: TargetSpec } & CastHow = {}): Effect {
  return {
    kind: "cast",
    apply(ctx): void {
      const card = instanceOf(ctx, args.target ?? { of: "chosen" });
      if (card === null || !CASTABLE_ZONES.includes(card.zone.z)) return;
      if (!mayCastNow(ctx.state, ctx.controller)) return;
      card.controller = ctx.controller;
      castCard(sinkOf(ctx), card, optionsOf(args));
    },
  };
}

/**
 * E12, R453: `cast` each card of a set read once, as the list reaches it (`forEachCard`, R113), one
 * after another, each with its caster's choices — Classic #56 Radiant's "Cast every Spell in your
 * graveyard, oldest first": a Spell a cast puts in the graveyard is not one of them. A card that has
 * left its pile by its turn (an earlier cast exiled it) is passed over.
 */
export function castEach(args: { cards: (ctx: EffectContext) => readonly (CardInstance | string)[] } & CastHow): Effect {
  return forEachCard({
    cards: args.cards,
    each: (instanceId) => cast({ ...args, target: { of: "instance", instanceId } }),
  });
}

/** What `castNew` casts: a definition, and the face to cast it with. */
export type CastDef = { defId: string; radiant?: boolean };

/**
 * E12, R453: cast a new card of a named definition — the running card's controller makes it, owns it
 * and casts it; it lands in their graveyard after it resolves (R87). `def` may be read off the context
 * as the effect applies (#7's remembered Spell); null casts nothing. `radiant` is the face when `def`
 * does not say (Classic+ #37's copy "by definition and face").
 */
export function castNew(
  args: { def: string | CastDef | ((ctx: EffectContext) => string | CastDef | null); radiant?: boolean } & CastHow,
): Effect {
  return {
    kind: "castNew",
    apply(ctx): void {
      const named = typeof args.def === "function" ? args.def(ctx) : args.def;
      if (named === null) return;
      const def: CastDef = typeof named === "string" ? { defId: named } : named;
      if (!mayCastNow(ctx.state, ctx.controller)) return;
      const card = newInstance(ctx.state, def.defId, ctx.controller, { z: "resolving", player: ctx.controller });
      card.radiant = def.radiant ?? args.radiant ?? false;
      castCard(sinkOf(ctx), card, optionsOf(args));
    },
  };
}

/** One random cast of `castRandom`: its own card, drawn as it applies. */
function castOneRandom(
  asked: CatalogQueryArgs,
  radiant: boolean,
  how: CastHow,
): Effect {
  return {
    kind: "castRandom:one",
    apply(ctx): void {
      if (!mayCastNow(ctx.state, ctx.controller)) return;
      // B4.1, R387: never the casting card's own definition, named by its id.
      const pool = query(excludingDefId(asked, ctx.self?.defId ?? ctx.defId));
      const def = ctx.rng.pick(pool);
      if (def === undefined) return;
      const card = newInstance(ctx.state, def.id, ctx.controller, { z: "resolving", player: ctx.controller });
      card.radiant = radiant;
      castCard(sinkOf(ctx), card, { ...optionsOf(how), random: true });
    },
  };
}

/**
 * E12, R452: cast `count` random cards from a pool (§5.1's `query`: no tokens unless it asks, never the
 * casting card's own definition), one after another, each a random cast — every choice random, its X
 * the caster's current mana (R453) — made and owned by the caster, landing in their graveyard (R87).
 * Classic+ #47 Jogg's Box: `{ query: { type: "Spell" }, count: 10 }`; #38.1 Solarius-Prime: five, with
 * `targetEnemies` and, on its Radiant face, `radiant`. The casts are parts of the list read once as it
 * reaches them, so one the other player's prompt pauses is followed by the rest when it is answered.
 */
export function castRandom(
  args: {
    query: CatalogQueryArgs | ((ctx: EffectContext) => CatalogQueryArgs);
    count?: number | ((ctx: EffectContext) => number);
    radiant?: boolean;
  } & Omit<CastHow, "random">,
): Effect {
  return lazyPart("castRandom", (ctx, memo) => {
    const count =
      typeof memo === "number"
        ? memo
        : Math.max(0, Math.trunc(typeof args.count === "function" ? args.count(ctx) : (args.count ?? 1)));
    const asked = typeof args.query === "function" ? args.query(ctx) : args.query;
    return {
      effects: Array.from({ length: count }, () => castOneRandom(asked, args.radiant === true, args)),
      memo: count,
    };
  });
}

// ---------------------------------------------------------------------------
// Price rules on a player (E15, E39, R455)
// ---------------------------------------------------------------------------

/**
 * How long a price rule lasts: until the play it prices is made ("your next …"), this turn, through the
 * named player's next turn (R48: "during their next turn", AI Alignment Tax), or for the rest of the
 * game.
 */
export type CostRuleSpan = "used" | "thisTurn" | "theirNextTurn" | "never";

function expiryOf(ctx: EffectContext, span: CostRuleSpan, player: ReturnType<typeof playerOf>): ModifierExpiry {
  switch (span) {
    case "used":
      return { until: "used" };
    case "thisTurn":
      return { until: "thisTurn", turn: ctx.state.turn };
    case "theirNextTurn":
      return { until: "nextTurnOf", player, fromTurn: ctx.state.turn };
    case "never":
      return { until: "never" };
  }
}

/**
 * E15, R455: a price rule on a player's cards (`costRules.ts`) — Classic #2's "Your next Trap or Field
 * Spell costs (2) less" (`{ types: ["Trap", "Field Trap", "Field Spell"], amount: -2 }` until used, or
 * `setTo: 0` on its Radiant face), AI Alignment Tax's "Your opponent's cards cost (1) more during
 * their next turn" (`player: "enemy"`, `{ amount: 1 }`, "theirNextTurn"). A rule until used is spent by
 * the play whose price it changed (never by a cast, R70).
 */
export function addCostRule(args: { player?: PlayerSpec; rule: CostRule; lasts: CostRuleSpan }): Effect {
  return {
    kind: "addCostRule",
    apply(ctx): void {
      const player = playerOf(ctx, args.player ?? "self");
      addModifier(ctx, player, { kind: "costRule", rule: { ...args.rule }, expiry: expiryOf(ctx, args.lasts, player) });
    },
  };
}

/**
 * E39, R455 (Classic+ #14 Forever&): "The next Spell you play gains …" — the enchantment is given to
 * that Spell as it is played (§10.5 step 4), and the rider is spent then. It waits across turns.
 */
export function enchantNextSpell(args: { player?: PlayerSpec; enchantment: Enchantment }): Effect {
  return {
    kind: "enchantNextSpell",
    apply(ctx): void {
      const player = playerOf(ctx, args.player ?? "self");
      addModifier(ctx, player, { kind: "enchantNextSpell", enchantment: { ...args.enchantment }, expiry: { until: "used" } });
    },
  };
}
