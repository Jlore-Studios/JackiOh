// Call to Chaos (SPEC §8 #95, R28, R87, R423, R436): the ten effects, the roll that picks them, and the
// capped recursion the tenth one drives.
//
// The card is a Spell whose base text is "one random effect" and whose Radiant text, since patch
// v0.2.0, is "three different random effects, resolved in the order listed" (R423, the shape Classic+
// #73 has: one rule serves both editions). The recursion is one entry of the list like any other: the
// Radiant face rolls it only when it falls among its three, and it resolves where the list puts it,
// last. Every effect here is built from the effects library, so #95's own file is a one-line hook that
// returns `[callToChaos()]` and stays a list of effects, like every other card (CLAUDE.md rule 5).
//
// Three things need care. First, each effect reads the board when it *resolves*, not when the hook
// builds it: a nested cast can draw cards, summon units and change costs in between, so "your hand
// becomes Radiant" and "draw your whole library" must see the hand and library as they are at that
// moment (R58's "the library size when the effect starts"). Every effect is therefore one lazy
// wrapper that builds its sub-effects inside `apply`. Second, the chain length is game state, not a
// module variable: it lives on the cast instance's `memory` (§10.1), so a paused, serialized game
// resumes with the same cap left and two independent Calls in one turn never share a counter. Third,
// what was rolled is told to both players before any of it resolves (`chaosRolled`, R436), once: a
// roll a pause interrupted is rebuilt from the part's memo and is not announced a second time.

import type { CatalogQuery, Tag } from "@jackioh/shared";
import { defByIndex, pickGenerated, query } from "../catalog";
import { CALL_TO_CHAOS_CHAIN_CAP, CALL_TO_CHAOS_RADIANT_EFFECTS } from "../config";
import {
  addRandomFromCatalog,
  draw,
  gainMana,
  heal,
  setCostMod,
  setRadiant,
  summon,
  summonRandom,
} from "../effects";
import { castCard, lazyPart, type EngineSink } from "../resolve";
import type { Rng } from "../rng";
import type { Effect, EffectContext } from "../script";
import { newInstance, type CardInstance } from "../state";

/** §8 #95: "summon 3 random 3-cost Units". */
export const CHAOS_UNIT_COUNT = 3;
export const CHAOS_UNIT_COST = 3;
/** §8 #95: "heal your hero 30" — a hero heal has no cap (§6.3 Heal). */
export const CHAOS_HEAL = 30;
/** §8 #95: "draw your whole library and gain 4 mana". */
export const CHAOS_MANA = 4;
/** §8 #95: "add 3 random cards to hand costing 0". */
export const CHAOS_ADDED_CARDS = 3;
/**
 * §8 #95 and §7: "summon five Radiant Rush Tokens".
 *
 * They used to be a bespoke 5/5 through `statsOverride`, which made #95 the only card in the set
 * that invented a Rush Token size. It now summons the token's own RADIANT face (6/6, Rush), so
 * the stats live in one place — the catalog — and a later change to the radiant token moves this
 * card with it instead of leaving it behind.
 */
export const CHAOS_RUSH_TOKENS = 5;
/** §8 #95: "every card in your hand and library costs 2 less". */
export const CHAOS_COST_DISCOUNT = 2;
/** §8 #95: "summon 5 random Field Spells or Traps … into your backrow". */
export const CHAOS_BACKROW_CARDS = 5;

/** §5, §8 #95: the tag the Call to Chaos family carries, and so the recursion's pool (§10.7). */
export const CHAOS_TAG: Tag = "Call to Chaos";
/** §7: the tokens #95 summons, by catalog index. */
const RUSH_TOKEN_INDEX = "T-rush";
const CHAOS_GOLEM_INDEX = "95.1";
/** B2.2: the set whose indices those are, since an index is unique only within its set. */
const TOKEN_SET = "Core";

/** §5.1: the backrow half of the catalog — "Field Spells or Traps (Field Traps included)" (§8). */
const CHAOS_BACKROW_QUERY: CatalogQuery = { type: ["Field Spell", "Trap", "Field Trap"] };

/**
 * R28: how many casts of the chain this instance is. The played #95 has no entry and so is 0; the
 * card it casts is 1, the card that one casts is 2, and the chain stops once a cast would be the
 * 21st. It lives in `memory` because §10.1 puts everything a card must remember on the instance,
 * which keeps the counter serializable and per-chain.
 */
export const CHAOS_CHAIN_KEY = "chaosChain";

export function chaosChainOf(instance: CardInstance | null): number {
  const value = instance?.memory[CHAOS_CHAIN_KEY];
  return typeof value === "number" && Number.isFinite(value) && value > 0 ? Math.trunc(value) : 0;
}

/** R28: the hard stop. At the cap the recursion effect resolves and does nothing at all. */
export function chaosChainCapReached(depth: number): boolean {
  return depth >= CALL_TO_CHAOS_CHAIN_CAP;
}

function sinkOf(ctx: EffectContext): EngineSink {
  return { state: ctx.state, events: ctx.events, rng: ctx.rng };
}

/** §7: a token summon needs the token's def id, which the catalog holds under its index in Core. */
function tokenDefId(index: string): string | null {
  return defByIndex(TOKEN_SET, index)?.id ?? null;
}

/**
 * One of the ten effects, built when it resolves rather than when the hook returns it, so every
 * state read happens after the effects before it have landed. It is a part of the list that holds it
 * (`resolve.lazyPart`), so an effect inside it that asks — a draw whose cast asks, in "draw your
 * whole library and gain 4 mana" — pauses the rest of it until the answer (R113): the mana waits
 * for the draw, as the partner waits for the recursion (R87).
 */
function chaosEffect(name: ChaosEffectName, build: (ctx: EffectContext) => Effect[]): Effect {
  return lazyPart(`callToChaos:${name}`, (ctx) => ({ effects: build(ctx) }));
}

/**
 * Point an effect from the library at one card by id. The library names a target through the
 * selections a play carried (R81), so a pick a script made itself travels the same way instead of
 * opening a second targeting path — and `setCostMod` stays the only place a cost change is written.
 */
function onInstance(effect: Effect, instanceId: string): Effect {
  return {
    kind: effect.kind,
    apply(ctx): void {
      effect.apply({ ...ctx, targets: [{ pick: "instance", instanceId }] });
    },
  };
}

// ---------------------------------------------------------------------------
// The ten effects, in the order §8 #95 lists them.
// ---------------------------------------------------------------------------

/**
 * 1. "Summon 3 random 3-cost Units": three independent picks (R60), placed per R64. Each pick is its
 * own `summonRandom`, which draws only when its unit has a zone to go to (R129), so a full row takes
 * no draw for a summon that cannot land.
 */
export function summonRandomThreeCostUnits(): Effect {
  return chaosEffect("units", () =>
    Array.from({ length: CHAOS_UNIT_COUNT }, () =>
      summonRandom({ query: { type: "Unit", cost: CHAOS_UNIT_COST } }),
    ),
  );
}

/** 2. "Heal your hero 30": §6.3 gives a hero heal no cap, so this may pass 30 health. */
export function healHeroThirty(): Effect {
  return chaosEffect("heal", () => [heal({ target: { of: "selfHero" }, amount: CHAOS_HEAL })]);
}

/**
 * 3. "Draw your whole library and gain 4 mana": R58 fixes the count at the library size when the
 * effect starts, so a cast-on-draw card drawn along the way cannot lengthen the draw, and an empty
 * library draws nothing at all rather than taking a fatigue hit (§2.4, R3).
 */
export function drawLibraryAndGainMana(): Effect {
  return chaosEffect("draw", (ctx) => [
    draw({ count: ctx.state.players[ctx.controller].library.length }),
    gainMana({ amount: CHAOS_MANA }),
  ]);
}

/**
 * 4. "Add 3 random cards to hand costing 0": three independent picks (R60) from the whole catalog,
 * which §5.1 keeps free of tokens and of the generating card — "never include the generating card's
 * own definition, unless the card names the pool itself", and only the recursion names its pool —
 * so no Call to Chaos is added. The 0 is a `costOverride` the card takes on reaching the hand (R65);
 * a full hand burns what it cannot take (§2.4, R4), without the price.
 */
export function addRandomZeroCostCards(): Effect {
  return chaosEffect("add", () => [
    addRandomFromCatalog({ count: CHAOS_ADDED_CARDS, costOverride: 0 }),
  ]);
}

/**
 * 5. "Your hand becomes Radiant": every card in hand right now (§5.2). A card that is already
 * Radiant is untouched, since the flag is never unset (§6.3 Make Radiant).
 */
export function makeHandRadiant(): Effect {
  return chaosEffect("radiant", (ctx) =>
    ctx.state.players[ctx.controller].hand.map((card) => setRadiant({ instanceId: card.id })),
  );
}

/**
 * 6. "Summon five Radiant Rush Tokens": the §7 Rush Token's own Radiant face (6/6), five separate
 * summons, so a board with fewer free zones simply takes fewer (R64) instead of failing as a whole.
 */
export function summonRushTokens(): Effect {
  return chaosEffect("tokens", () => {
    const defId = tokenDefId(RUSH_TOKEN_INDEX);
    if (defId === null) return [];
    return Array.from({ length: CHAOS_RUSH_TOKENS }, () => summon({ defId, radiant: true }));
  });
}

/**
 * 7. "Every card in your hand and library costs 2 less": the cards that are there when the effect
 * resolves, each getting a permanent `costMod` that travels with it between zones (R78). It changes
 * those cards, not the player, so a card drawn afterwards still pays full price.
 */
export function discountHandAndLibrary(): Effect {
  return chaosEffect("discount", (ctx) => {
    const side = ctx.state.players[ctx.controller];
    const discount = setCostMod({ target: { of: "chosen" }, amount: -CHAOS_COST_DISCOUNT });
    return [...side.hand, ...side.library].map((card) => onInstance(discount, card.id));
  });
}

/** 8. "Summon a Chaos Golem": the 10/10 token of §7 (index 95.1), placed per R64. */
export function summonChaosGolem(): Effect {
  return chaosEffect("golem", () => {
    const defId = tokenDefId(CHAOS_GOLEM_INDEX);
    return defId === null ? [] : [summon({ defId })];
  });
}

/**
 * 9. "Summon 5 random Field Spells or Traps (Field Traps included, traps face-down) into your
 * backrow": five independent picks (R60). `summonRandom` sends every one of those types to the
 * backrow, leaves a Trap or Field Trap face-down while a Field Spell is public (§3.2, R33), and draws
 * only for a summon that has a zone to go to (R129).
 */
export function summonRandomBackrow(): Effect {
  return chaosEffect("backrow", () =>
    Array.from({ length: CHAOS_BACKROW_CARDS }, () => summonRandom({ query: CHAOS_BACKROW_QUERY })),
  );
}

/**
 * 10. "Cast a random Call to Chaos": a Cast per R70 — free, counted as a play, running the card's
 * own script. The pool is every card tagged Call to Chaos in every set (R380: a pool that names no set
 * reaches every set), so it holds both editions, and the card cast is the *base* form even when a
 * Radiant Call cast it (R28); the new card is Radiant only if something later makes it so.
 *
 * R28 caps the chain at CALL_TO_CHAOS_CHAIN_CAP casts of either edition. The cap is a hard stop: at
 * the cap this effect resolves into nothing, and no re-roll replaces it (R87, R423).
 *
 * The cast card is a real generated card, like the ones "add 3 random cards to hand" makes (R60), so
 * §10.5 step 7 sends it to the caster's graveyard when it has resolved (R87), which is what feeds
 * Gravedigger and Reminisce down a long chain.
 */
export function castRandomCallToChaos(): Effect {
  return {
    kind: "callToChaos:recast",
    apply(ctx): void {
      const depth = chaosChainOf(ctx.self);
      if (chaosChainCapReached(depth)) return;

      const def = pickGenerated(ctx.rng, query({ tags: [CHAOS_TAG] }), ctx.state);
      if (def === undefined) return;

      const card = newInstance(ctx.state, def.id, ctx.controller, {
        z: "resolving",
        player: ctx.controller,
      });
      card.memory[CHAOS_CHAIN_KEY] = depth + 1;
      castCard(sinkOf(ctx), card);
    },
  };
}

// ---------------------------------------------------------------------------
// The roll (§8 #95, R28, R423).
// ---------------------------------------------------------------------------

/** Core #95's ten entries by name. Another edition's list (Classic+ #73) names its own. */
export type ChaosEffectName =
  | "units"
  | "heal"
  | "draw"
  | "add"
  | "radiant"
  | "tokens"
  | "discount"
  | "golem"
  | "backrow"
  | "recast";

export type ChaosEffectDef = {
  /** Unique within its list: what a roll a pause interrupted is kept by (the part's memo). */
  name: string;
  /** The clause as the card prints it: what `chaosRolled` names to both players (R436). */
  label: string;
  build: () => Effect;
};

/** R28: the effect that drives the chain, "cast a random Call to Chaos" (R423: rolled like any other). */
export const CHAOS_RECURSION: ChaosEffectName = "recast";

/**
 * The ten effects of §8 #95, in the order the card lists them. Each `label` is the clause as the
 * card prints it (R432's cost words, R373's "deck"), which is what `chaosRolled` names to both
 * players (R436).
 */
export const CHAOS_EFFECTS: readonly ChaosEffectDef[] = [
  { name: "units", label: "Summon 3 random (3) Cost Units", build: summonRandomThreeCostUnits },
  { name: "heal", label: "Heal your hero 30", build: healHeroThirty },
  { name: "draw", label: "Draw your whole deck and gain 4 mana", build: drawLibraryAndGainMana },
  { name: "add", label: "Add 3 random cards to your hand, which cost (0)", build: addRandomZeroCostCards },
  { name: "radiant", label: "Make your hand Radiant", build: makeHandRadiant },
  { name: "tokens", label: "Summon 5 Radiant Rush Tokens", build: summonRushTokens },
  { name: "discount", label: "Cards in your hand and deck cost (2) less", build: discountHandAndLibrary },
  { name: "golem", label: "Summon a Chaos Golem", build: summonChaosGolem },
  {
    name: "backrow",
    label: "Summon 5 random Field Spells or Traps into your backrow, Traps face-down",
    build: summonRandomBackrow,
  },
  { name: "recast", label: "Cast a random Call to Chaos", build: castRandomCallToChaos },
];

export function chaosEffectByName(name: string): ChaosEffectDef | null {
  return CHAOS_EFFECTS.find((effect) => effect.name === name) ?? null;
}

/**
 * R28, R423: the base form rolls one entry of its list; the Radiant form rolls
 * `CALL_TO_CHAOS_RADIANT_EFFECTS` *different* entries — drawn one at a time without replacement, so no
 * effect comes up twice — and resolves them in the order the list writes them, whatever order they
 * were drawn in. The recursion is an entry like any other: it is rolled only when it falls among the
 * three, and resolves where the list puts it (R87's "the recursion where it falls"). `table` is the
 * edition's list (Core #95's by default; Classic+ #73 brings its own), so one roll serves both.
 */
export function rollChaosEffects(
  rng: Rng,
  radiant: boolean,
  table: readonly ChaosEffectDef[] = CHAOS_EFFECTS,
): ChaosEffectDef[] {
  const count = Math.min(radiant ? CALL_TO_CHAOS_RADIANT_EFFECTS : 1, table.length);
  const left = [...table];
  const drawn: ChaosEffectDef[] = [];
  for (let i = 0; i < count; i += 1) {
    const at = rng.int(left.length);
    const [one] = left.splice(at, 1);
    if (one !== undefined) drawn.push(one);
  }
  return table.filter((effect) => drawn.includes(effect));
}

/**
 * R436: tell both players what was rolled, before any of it resolves — the rolled clauses by their
 * printed labels, in the order they resolve. The card itself follows R97 (`viewFor.redactEvent`): a
 * #95 read by nobody is the sentinel, while what it rolled is public, as the resolution is.
 */
function announceRoll(rolled: readonly ChaosEffectDef[]): Effect {
  return {
    kind: "callToChaos:announce",
    apply(ctx): void {
      const defId = ctx.self?.defId ?? ctx.defId ?? "";
      ctx.events.push({
        type: "chaosRolled",
        player: ctx.controller,
        instanceId: ctx.self?.id ?? "",
        defId,
        effects: rolled.map((effect) => effect.label),
      });
    },
  };
}

/**
 * The whole card, as one effect: #95's script is `cry: () => [callToChaos()]` for both forms.
 *
 * The roll happens when the effect resolves, so the rng cursor moves with the resolution and a
 * replay that stops on a prompt in between still lines up (§10.7). `radiant` defaults to the
 * instance's own flag, which is what `makeContext` put in the context (§5.2). `table` is the
 * edition's list (R423: Classic+ #73 rolls its own through the same rule).
 */
export function callToChaos(args: { radiant?: boolean; table?: readonly ChaosEffectDef[] } = {}): Effect {
  const table = args.table ?? CHAOS_EFFECTS;
  return lazyPart("callToChaos", (ctx, memo) => {
    // R87, R423: the rolled effects resolve in list order, the recursion's whole chain where it falls,
    // and a cast in that chain can ask — so the roll is a part of the Cry's list, and a pause inside
    // it waits with the rest of it owed. What was rolled is the part's memo: resuming builds the same
    // effects again, and rolls nothing a second time (§10.7). The announcement heads the part, so a
    // resumed part, which goes on after what it had already run, never announces it again (R436).
    const kept = rolledNames(memo);
    const rolled =
      kept === null
        ? rollChaosEffects(ctx.rng, args.radiant ?? ctx.radiant, table)
        : kept.flatMap((name) => {
            const chosen = table.find((effect) => effect.name === name);
            return chosen === undefined ? [] : [chosen];
          });
    return {
      effects: [announceRoll(rolled), ...rolled.map((chosen) => chosen.build())],
      memo: rolled.map((chosen) => chosen.name),
    };
  });
}

/** A roll kept across a pause (`EffectPart.memo`), read back defensively: it came through JSON. */
function rolledNames(memo: unknown): string[] | null {
  if (!Array.isArray(memo)) return null;
  return memo.filter((name): name is string => typeof name === "string");
}
