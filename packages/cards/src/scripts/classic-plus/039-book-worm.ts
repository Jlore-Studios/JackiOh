// C+ #39 Book Worm (SPEC §8.7 row 39): (1) Unit, Common, 1/4 → 2/8.
//   Base:    "Death: Add N random Books to your hand. N starts at 1. Start of turn: N increases by {growth}."
//   Radiant: the same, the Books Radiant.
// N lives on the instance (memory, reset as it leaves the field, R78, so a bounced worm starts at 1),
// the Death reads it last-known (R89), and the pool is the non-token Books of every set (R380, R60).

import type { EffectContext, Script } from "@jackioh/engine";
import { param, recalled } from "@jackioh/engine";
import { addRandomFromCatalog, remember } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classicplus-039");

const BOOKS = "books";

/** N: 1 as it arrives, raised at each start of its controller's turn (a fused part reads its own). */
function booksOf(ctx: Pick<EffectContext, "self" | "data">): number {
  const n = recalled(ctx, BOOKS);
  return typeof n === "number" ? n : 1;
}

function bookWorm(radiant: boolean): Script {
  return {
    startOfTurn: (ctx) => [remember({ key: BOOKS, value: booksOf(ctx) + param(ctx, "growth") })],
    death: (ctx) => [addRandomFromCatalog({ query: { tags: ["Book"] }, count: booksOf(ctx), radiant })],
    preview: (ctx) => [{ label: "N", value: booksOf({ self: ctx.self, data: {} }) }],
  };
}

export const base: Script = bookWorm(false);

export const radiant: Script = bookWorm(true);
