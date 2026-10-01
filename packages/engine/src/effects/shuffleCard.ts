// Shuffling an EXISTING card into its owner's library (§6.3, R80, R316): Classic #30 Recycle's
// "Shuffle your graveyard into your deck". A card-specific verb of the Classic #1–#45 workstream,
// beside `shuffleInto.ts`, whose verbs make fresh copies; this one moves a card that already exists,
// keeping its id, its `costMod` and everything else R78 carries between zones.
//
// Each card goes in at a uniformly random position, drawn from the match rng, through the one
// shuffle-in the engine has (`draw.shuffleIntoLibrary`): the `shuffledIn` event with its position
// blanked for both players (R97), the owner shown what went in (R311) and R43's arrival roll.
//
// R80, R316: a card that would go into a full library does not. It STAYS where it is when it is in
// its owner's graveyard already — no second move and no second `enteredGraveyard`, since it never
// left — and `libraryOverflow { outcome: "graveyard" }` reports the refusal as for any existing card.
// A card anywhere else is refused the way `shuffleIntoLibrary` refuses an existing card: to its
// owner's graveyard, or ceasing to exist if it is a unit-token card (R11).

import { LIBRARY_CAP } from "../config";
import { shuffleIntoLibrary } from "../draw";
import type { Effect } from "../script";
import { findInstance } from "../state";

/**
 * §6.3, R80, R316: shuffle the card `instanceId` names into its owner's library at a random position.
 * A card that no longer exists is skipped. A graveyard card a full library refuses stays in that
 * graveyard, reported by `libraryOverflow`; any other refused card is handled as `shuffleIntoLibrary`
 * handles an existing card.
 */
export function shuffleCardInto(args: { instanceId: string }): Effect {
  return {
    kind: "shuffleCardInto",
    apply(ctx): void {
      const card = findInstance(ctx.state, args.instanceId);
      if (card === undefined) return;
      const full = ctx.state.players[card.owner].library.length >= LIBRARY_CAP;
      if (full && card.zone.z === "graveyard" && card.zone.player === card.owner) {
        ctx.events.push({
          type: "libraryOverflow",
          player: card.owner,
          instanceId: card.id,
          defId: card.defId,
          outcome: "graveyard",
          ...(card.radiant ? { radiant: true as const } : {}),
        });
        return;
      }
      shuffleIntoLibrary(ctx, card, true);
    },
  };
}
