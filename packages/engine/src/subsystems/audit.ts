// C+ #44 Simplicity Audit and #45 Complexity Audit (SPEC §8.7 rows 44, 45; docs/classic-sets.md E36):
// the permanents whose card has fewer (or more) lines of code than the Audit's own. `loc` is card data
// the catalog generator writes (E36); a fused card's is its ingredients' sum (R77, `subsystems/fuse`).
// The cards keep their own wiring — the mode, the exile, the preview — because their own `loc` is
// what they compare against, and a card file that only named this module would be three lines long.

import type { PlayerId } from "@jackioh/shared";
import { opponentOf } from "@jackioh/shared";
import { defOf } from "../catalog";
import type { CardInstance, GameState } from "../state";
import { activeUnitsOf, cardAt, slotsOf } from "../zones";

/** E36: a definition's lines of code. A card with no script file yet has none, and reads 0. */
export function linesOfCode(state: GameState, defId: string): number {
  return defOf(state, defId).loc ?? 0;
}

/**
 * The permanents an Audit run by `controller` exiles now: the cards acting on the field — tops of
 * piles (a Unit a carrier holds too) and backrow cards, face-down ones included, never a card dormant
 * under a Stack (§3.2) — on both sides, `active`'s first (R68; the caller says who is active, since a
 * preview never reads `state.active`, R280), or the opponent's only; each one whose card's `loc` is
 * strictly below `loc` (`more` false) or above it (`more` true). Equal stays.
 */
export function auditTargets(
  state: GameState,
  args: { controller: PlayerId; active: PlayerId; loc: number; more: boolean; enemyOnly: boolean },
): CardInstance[] {
  const enemy = opponentOf(args.controller);
  const sides = args.enemyOnly ? [enemy] : [args.active, opponentOf(args.active)];
  return sides
    .flatMap((player) => [
      ...activeUnitsOf(state, player),
      ...slotsOf(player, "backrow").flatMap((ref) => cardAt(state, ref) ?? []),
    ])
    .filter((card) => {
      const loc = linesOfCode(state, card.defId);
      return args.more ? loc > args.loc : loc < args.loc;
    });
}
