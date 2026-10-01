// Shuffle random catalog cards into a library (§6.3 Shuffle into, §5.1; Classic+ #40 Appropriations'
// Education, E39): `count` independent picks from the pool (repeats allowed, R60; never the running
// card, R387), each made Radiant and enchanted as asked before it goes in at a uniformly random
// position, so the enchantment rides it from the start (`enchantments.ts`). R80's cap turns the rest
// away, as `shuffleInto` does; an empty pool does nothing and draws nothing (R129).

import type { Enchantment } from "@jackioh/shared";
import { excludingDefId, query, type CatalogQueryArgs } from "../catalog";
import { shuffleIntoLibrary } from "../draw";
import { addEnchantment } from "../enchantments";
import type { Effect } from "../script";
import { newInstance } from "../state";
import { playerOf, type PlayerSpec } from "./targets";

export function shuffleRandomFromCatalog(args: {
  query: CatalogQueryArgs;
  count: number;
  player?: PlayerSpec;
  radiant?: boolean;
  enchantments?: readonly Enchantment[];
}): Effect {
  return {
    kind: "shuffleRandomFromCatalog",
    apply(ctx): void {
      const pool = query(excludingDefId(args.query, ctx.self?.defId ?? ctx.defId));
      const player = playerOf(ctx, args.player ?? "self");
      for (let at = 0; at < args.count && pool.length > 0; at += 1) {
        const def = ctx.rng.pick(pool);
        if (def === undefined) return;
        const card = newInstance(ctx.state, def.id, player, { z: "library", player });
        if (args.radiant === true) card.radiant = true;
        for (const enchantment of args.enchantments ?? []) addEnchantment(card, enchantment);
        shuffleIntoLibrary(ctx, card, false);
      }
    },
  };
}
