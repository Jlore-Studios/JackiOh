// B5 E34, the perfect-hand scorer (SPEC §8.7 C+ #27 Zephrys Zealotism, §10.7, R29, R387, R416).
//
// "Replace your hand with the perfect hand of Classic and Classic+ cards." The scoring is the Zephyrs
// scorer's own (`./scorer`, R29, untouched): `scoreDef` and its dry run on one copy of the state as
// the card resolves, which draws from a seed of its own, never the match rng, so the same state gives
// the same hand. This adds only the pool (every non-token Classic and Classic+ card but the asking
// card, R387), the face (Radiant on the Radiant face), the order (score, then card id, since an index
// repeats across sets) and the hand.

import type { PlayerId } from "@jackioh/shared";
import { excludingDefId, query } from "../catalog";
import { addToHand } from "../effects/addToHand";
import type { Effect } from "../script";
import type { GameState } from "../state";
import { moveToZone, reportGraveyardLanding } from "../zones";
import { dryRunBase, scoreDef, type Scored } from "./scorer";

/** R416: every non-token Classic and Classic+ card but `selfDefId`'s (R387), best first, ties by id. */
export function rankPerfectHand(
  state: GameState,
  viewer: PlayerId,
  options: { radiant?: boolean; selfDefId?: string } = {},
): Scored[] {
  const base = dryRunBase(state, viewer);
  return query(excludingDefId({ set: ["Classic", "Classic+"] }, options.selfDefId))
    .map((def) => scoreDef(state, viewer, def, { radiant: options.radiant === true }, base))
    .sort((a, b) => b.score - a.score || (a.def.id < b.def.id ? -1 : 1));
}

/**
 * R416: each other card in the controller's hand goes to their graveyard — a replace, not a discard
 * (#76's reading; a unit-token card ceases to exist, R11) — and the top N of the ranking, made on the
 * state before anything moves, arrive in rank order at their printed cost. N is the hand's size, so
 * an empty hand ranks nothing and gets nothing.
 */
export function replaceHandWithPerfect(args: { radiant?: boolean } = {}): Effect {
  return {
    kind: "replaceHandWithPerfect",
    apply(ctx): void {
      const hand = [...ctx.state.players[ctx.controller].hand];
      if (hand.length === 0) return;
      const radiant = args.radiant === true;
      const selfDefId = ctx.self?.defId ?? ctx.defId;
      const picks = rankPerfectHand(ctx.state, ctx.controller, { radiant, ...(selfDefId === undefined ? {} : { selfDefId }) });
      for (const card of hand) reportGraveyardLanding(ctx, card, moveToZone(ctx.state, card, "graveyard"));
      for (const { def } of picks.slice(0, hand.length)) addToHand({ defId: def.id, radiant }).apply(ctx);
    },
  };
}
