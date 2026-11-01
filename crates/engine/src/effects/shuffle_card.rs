//! Shuffling an EXISTING card into its owner's library (§6.3, R80, R316): Classic #30 Recycle's
//! "Shuffle your graveyard into your deck". A card-specific verb of the Classic #1–#45 workstream,
//! beside `shuffle_into.rs`, whose verbs make fresh copies; this one moves a card that already exists,
//! keeping its id, its `costMod` and everything else R78 carries between zones.
//!
//! Each card goes in at a uniformly random position, drawn from the match rng, through the one
//! shuffle-in the engine has (`draw::shuffle_into_library`): the `shuffledIn` event with its position
//! blanked for both players (R97), the owner shown what went in (R311) and R43's arrival roll.
//!
//! R80, R316: a card that would go into a full library does not, and `shuffle_into_library` alone
//! decides it. It STAYS where it is when it is in its owner's graveyard already — no second move and
//! no second `enteredGraveyard`, since it never left — and `libraryOverflow { outcome: "graveyard" }`
//! reports the refusal as for any existing card. A card anywhere else is refused as any existing card
//! is: to its owner's graveyard, or ceasing to exist if it is a unit-token card (R11).
//!
//! R1867006: `player` moves the card into another player's library instead (Stinkyscape #12.3
//! Psychic Scream's "into your opponent's deck"). The card becomes the new owner's before the move,
//! owner and controller alike as `new_instance` makes them, so R78's reset and the arrival land on
//! their card (R12: a steal off the field makes a card the taker's).
//!
//! Port of `packages/engine/src/effects/shuffleCard.ts`.

use serde::{Deserialize, Serialize};

use crate::draw::shuffle_into_library;
use crate::script::Effect;
use crate::state::{find_instance, find_instance_mut};

use super::targets::{PlayerSpec, player_of};

/// `shuffleCardInto`'s arguments: the card, by id, and whose library it goes to.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct ShuffleCardIntoArgs {
    pub instance_id: String,
    /// R1867006: the library's owner. Absent is the card's own owner, as before.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player: Option<PlayerSpec>,
}

/// §6.3, R80, R316: shuffle the card `instance_id` names into its owner's library at a random position.
/// A card that no longer exists is skipped. A graveyard card a full library refuses stays in that
/// graveyard, reported by `libraryOverflow`; any other refused card is handled as `shuffle_into_library`
/// handles an existing card.
pub fn shuffle_card_into(args: ShuffleCardIntoArgs) -> Effect {
    Effect::new("shuffleCardInto", move |ctx| {
        // R1867006: the move into another player's deck. The card becomes the new owner's in the
        // state first — owner and controller alike as `new_instance` makes them — because the move
        // below refreshes its working copy from the live card, which would drop a change made only
        // on the clone.
        if let Some(spec) = args.player {
            let owner = player_of(ctx, spec);
            if let Some(live) = find_instance_mut(ctx.state, &args.instance_id) {
                live.owner = owner;
                live.controller = owner;
            }
        }
        let Some(mut card) = find_instance(ctx.state, &args.instance_id).cloned() else {
            return;
        };
        shuffle_into_library(ctx, &mut card, true, None);
    })
}
