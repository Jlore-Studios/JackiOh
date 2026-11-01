//! Shuffling an EXISTING card into its owner's library (§6.3, R80, R316): Classic #30 Recycle's
//! "Shuffle your graveyard into your deck" (of Classic #1–#45). Unlike `shuffle_into.rs`'s fresh copies,
//! the card keeps its id, its `costMod` and all else R78 carries between zones. It goes through the one
//! shuffle-in (`draw::shuffle_into_library`): its position blanked for both players (R97), the owner
//! shown what went in (R311) and R43's arrival roll.
//!
//! R80, R316: `shuffle_into_library` alone decides a full library's refusal. A card in its owner's
//! graveyard that it refuses STAYS there (it never left: no second `enteredGraveyard`), reported by
//! `libraryOverflow`; any other refused card goes to its owner's graveyard, or ceases to exist if a
//! unit-token card (R11).
//!
//! R1867006 (Stinkyscape #12.3 Psychic Scream's "into your opponent's deck"): `player` names another
//! library; the card becomes that player's first, so R78's reset and the arrival land on their card (R12).

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
/// A card that no longer exists is skipped.
pub fn shuffle_card_into(args: ShuffleCardIntoArgs) -> Effect {
    Effect::new("shuffleCardInto", move |ctx| {
        // R1867006: the new owner (and controller, as `new_instance` makes them) goes on the live card,
        // since the move refreshes its working copy from it and would drop a change made on a clone.
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
