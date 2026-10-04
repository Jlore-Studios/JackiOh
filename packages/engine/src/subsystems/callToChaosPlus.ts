// C+ #73 Call to Chaos (Classic+ Edition) (SPEC §8.7 row 73, R28, R87, R380, R382, R387, R423, R436): its
// table of ten effects, which Core #95's subsystem (`callToChaos.ts`) rolls, announces and resolves —
// `callToChaos({ radiant, table: CHAOS_PLUS_EFFECTS })` — so one rule serves both editions: the base face
// rolls one entry, the Radiant three different ones resolved in the list's order (R423), the recursion
// counts casts of either edition against `CALL_TO_CHAOS_CHAIN_CAP` (R28) and at the cap resolves into
// nothing (R87), and both players are told what was rolled (`chaosRolled`, R436).
//
// Every entry is built when it resolves, not when the Cry returns (a part of the list, `lazyPart`), as
// Core's are: a recursion rolled before it resolves its whole chain first and changes the hand, the deck
// and the board each later entry reads (R87), and a pause inside one parks the rest of it (R113).

import type { CatalogQuery } from "@jackioh/shared";
import { defByIndex, pickGenerated, query } from "../catalog";
import {
  CHAOS_PLUS_BOOKS,
  CHAOS_PLUS_CLASSIC_CARDS,
  CHAOS_PLUS_COST,
  CHAOS_PLUS_DEGRADES,
  CHAOS_PLUS_FRUITS,
  CHAOS_PLUS_UPGRADES,
} from "../config";
import { addRandomFromCatalog } from "../effects/addToHand";
import { destroyAll } from "../effects/destroy";
import { fuseRandomInto } from "../effects/fuse";
import { summon } from "../effects/summon";
import { transform } from "../effects/transform";
import { degrade, upgrade } from "../effects/tune";
import { lazyPart } from "../resolve";
import type { Effect, EffectContext } from "../script";
import { CHAOS_TAG, castRandomCallToChaos, type ChaosEffectDef } from "./callToChaos";

/** §7: the Classic Golem (C+ #73.1), by its index in its set (B2.2). */
const GOLEM_SET = "Classic+";
const GOLEM_INDEX = "73.1";

/** One entry, built as it resolves (`lazyPart`), kept by name across a pause like Core's (R87). */
function entry(name: string, build: (ctx: EffectContext) => Effect[]): Effect {
  return lazyPart(`callToChaosPlus:${name}`, (ctx) => ({ effects: build(ctx) }));
}

/** Entries 1, 2 and 4: N independent picks of a pool (R60), each a fresh hand card that costs (0); a full hand burns it (§2.4). */
function addFree(name: string, pool: CatalogQuery, count: number): () => Effect {
  return () => entry(name, () => [addRandomFromCatalog({ query: pool, count, costOverride: CHAOS_PLUS_COST })]);
}

/**
 * Entry 9: "replace your deck with random Call to Chaos cards, which cost (0)" — each card of the deck as
 * the entry resolves is Replaced (§6.3, R35) one for one, where it lies, by a random card of the "Call
 * to Chaos" pool (both editions, this one included: the text names its pool, R387, R28), each pick its
 * own (R60), the old card ceasing to exist and the new one a card its owner was never shown (R311), with
 * a `costOverride` of (0) it carries in every zone (R78). The deck keeps its size; an empty one draws
 * nothing (R129).
 */
export function replaceDeckWithCallToChaos(): Effect {
  return entry("replace", (ctx) => {
    const deck = ctx.state.players[ctx.controller].library;
    const pool = query({ tags: [CHAOS_TAG] });
    return [...deck].map((old): Effect => ({
      kind: "callToChaosPlus:replaceOne",
      apply(inner): void {
        const library = inner.state.players[inner.controller].library;
        const at = library.findIndex((card) => card.id === old.id);
        const def = at < 0 ? undefined : pickGenerated(inner.rng, pool, inner.state); // R658: into a deck
        if (def === undefined) return;
        transform({ instanceId: old.id, defId: def.id }).apply(inner);
        const replacement = library[at];
        if (replacement !== undefined && replacement.id !== old.id) replacement.costOverride = CHAOS_PLUS_COST;
      },
    }));
  });
}

/**
 * §8.7 row 73's ten entries, in the order the card prints them. Each `label` is the clause as the card
 * prints it (R432's cost words, R373's "deck"), which `chaosRolled` names to both players (R436).
 */
export const CHAOS_PLUS_EFFECTS: readonly ChaosEffectDef[] = [
  {
    // 1. The Fruit pool holds the five Grapes too (R382).
    name: "fruits",
    label: "Add 5 random Fruits to your hand, which cost (0)",
    build: addFree("fruits", { tags: ["Fruit"] }, CHAOS_PLUS_FRUITS),
  },
  {
    // 2. Non-token Books of every set (R380).
    name: "books",
    label: "Add 3 random Books to your hand, which cost (0)",
    build: addFree("books", { tags: ["Book"] }, CHAOS_PLUS_BOOKS),
  },
  {
    // 3. Every enemy permanent on the field — the top of each pile, face-down cards included — is marked
    // together (R59); Indestructible ones stay (R46).
    name: "destroy",
    label: "Destroy all enemy permanents",
    build: () => entry("destroy", () => [destroyAll({ side: "enemy", rows: ["units", "backrow"] })]),
  },
  {
    // 4. Non-token cards of the Classic set only (the text names it, R380).
    name: "classic",
    label: "Add 3 random Classic cards to your hand, which cost (0)",
    build: addFree("classic", { set: "Classic" }, CHAOS_PLUS_CLASSIC_CARDS),
  },
  {
    // 5. Two separate Upgrades of each card in your hand and your deck (R386), the deck's unseen by its
    // owner until the card leaves it (R311).
    name: "upgrade",
    label: "Upgrade every card in your hand and deck twice",
    build: () =>
      entry("upgrade", () => [upgrade({ scope: { side: "self", zones: ["hand", "library"] }, times: CHAOS_PLUS_UPGRADES })]),
  },
  {
    // 6. E23: a random non-token card of every set but this one (R387) fused into each deck card, which
    // is the kept instance, keeps its type and keeps its cost (R470); Immutable ones are skipped (R23).
    name: "fuse",
    label: "Fuse a random card into each card in your deck, each keeping its cost",
    build: () => entry("fuse", () => [fuseRandomInto({ into: { pile: "library" } })]),
  },
  {
    // 7. Three separate Degrades of each card on the opponent's field and in their hand, hidden in
    // their hand (R177, R242).
    name: "degrade",
    label: "Degrade every card on your opponent's field and in their hand three times",
    build: () =>
      entry("degrade", () => [degrade({ scope: { side: "enemy", zones: ["field", "hand"] }, times: CHAOS_PLUS_DEGRADES })]),
  },
  {
    // 8. The Classic Golem token, placed per R64.
    name: "golem",
    label: "Summon a Classic Golem",
    build: () =>
      entry("golem", () => {
        const defId = defByIndex(GOLEM_SET, GOLEM_INDEX)?.id;
        return defId === undefined ? [] : [summon({ defId })];
      }),
  },
  { name: "replace", label: "Replace your deck with random Call to Chaos cards, which cost (0)", build: replaceDeckWithCallToChaos },
  // 10. Core's recursion, the chain counting casts of either edition (R28, R87).
  { name: "recast", label: "Cast a random Call to Chaos", build: castRandomCallToChaos },
];
