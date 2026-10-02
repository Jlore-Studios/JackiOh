// The verbs over a last board (docs/classic-sets.md B5 E30, SPEC §8.7 C+ #29, R417, R564): Discover
// a card of the caster's frozen board, add the Discovered one, add random ones. The board is
// `subsystems/lastBoards`'; these only read it and make cards from it.
//
// Each card made is a NEW card the caster owns, on its entry's face, through `addToHand`'s one
// creation path (§2.4's hand cap burns it, R317; a unit-token card in hand is R11's). A fused entry
// is rebuilt from its id into this match the first time it is offered or made
// (`subsystems/fuse.rebuildFusedDef`, R179). An empty board opens no prompt and draws nothing (R129).

import { selfDefIds } from "../catalog";
import { openPrompt, resumeSelf } from "../prompts";
import type { Effect, EffectContext } from "../script";
import type { LastBoardEntry } from "../state";
import { rebuildFusedDef } from "../subsystems/fuse";
import { lastBoardCandidates } from "../subsystems/lastBoards";
import { addToHand } from "./addToHand";
import { chosenOptions } from "./choose";

/** §6.3 Discover: "choose 1 of 3". */
export const LAST_BOARD_DISCOVER_OPTIONS = 3;

/** C+ #29: "It costs (0)", "Each costs (0)" — a `costOverride` (R65). */
export const LAST_BOARD_CARD_COST = 0;

/** The caster's candidates, never the running card's own definitions (R387). */
function candidatesFor(ctx: EffectContext): LastBoardEntry[] {
  const own = ctx.self?.defId ?? ctx.defId;
  return lastBoardCandidates(ctx.state, ctx.controller, own === undefined ? [] : selfDefIds(own));
}

/** One entry as a new card in the caster's hand, on its face, with the cost asked for. */
function addEntry(ctx: EffectContext, entry: LastBoardEntry, costOverride: number | undefined): void {
  if (rebuildFusedDef(ctx.state, entry.defId, ctx.controller) === null) return;
  addToHand({
    defId: entry.defId,
    radiant: entry.radiant,
    ...(costOverride === undefined ? {} : { costOverride }),
  }).apply(ctx);
}

/**
 * C+ #29 base (R417): Discover among `count` different cards of the caster's last board (all of
 * them when it holds fewer), drawn without replacement and shown to the chooser only (§10.8, R81).
 * Each option is the card's definition id, on its entry's face; the answer re-enters `step`.
 */
export function discoverFromLastBoard(args: { step: string; count?: number; data?: Record<string, unknown> }): Effect {
  return {
    kind: "discoverFromLastBoard",
    apply(ctx): void {
      const pool = candidatesFor(ctx);
      if (pool.length === 0) return;
      const options = ctx.rng
        .shuffle(pool)
        .slice(0, args.count ?? LAST_BOARD_DISCOVER_OPTIONS)
        .flatMap((entry) => {
          const def = rebuildFusedDef(ctx.state, entry.defId, ctx.controller);
          return def === null
            ? []
            : [
                {
                  key: `mode:${entry.defId}`,
                  label: def.name,
                  selection: { pick: "mode" as const, option: entry.defId },
                  ...(entry.radiant ? { radiant: true as const } : {}),
                },
              ];
        });
      openPrompt(ctx, {
        player: ctx.controller,
        kind: "discover",
        prompt: "Discover a card from the board your last game ended with",
        options,
        resume: resumeSelf(ctx, args.step, args.data ?? {}),
      });
    },
  };
}

/** C+ #29 base's resume (R417): the Discovered card into the caster's hand, on its entry's face. */
export function addFromLastBoard(args: { costOverride?: number } = {}): Effect {
  return {
    kind: "addFromLastBoard",
    apply(ctx): void {
      const picked = chosenOptions(ctx)[0];
      const entry = candidatesFor(ctx).find((candidate) => candidate.defId === picked);
      if (entry !== undefined) addEntry(ctx, entry, args.costOverride);
    },
  };
}

/**
 * C+ #29 Radiant (R417): `count` different random cards of the caster's last board (all of them when
 * it holds fewer) straight into the caster's hand, drawn without replacement in one shuffle.
 */
export function addRandomFromLastBoard(args: { count: number; costOverride?: number }): Effect {
  return {
    kind: "addRandomFromLastBoard",
    apply(ctx): void {
      const pool = candidatesFor(ctx);
      if (pool.length === 0 || args.count < 1) return;
      for (const entry of ctx.rng.shuffle(pool).slice(0, args.count)) addEntry(ctx, entry, args.costOverride);
    },
  };
}
