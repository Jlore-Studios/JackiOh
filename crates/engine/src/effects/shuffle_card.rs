//! Shuffling an EXISTING card into its owner's library (§6.3, R80, R316): Classic #30 Recycle's
//! "Shuffle your graveyard into your deck". A card-specific verb of the Classic #1–#45 workstream,
//! beside `shuffle_into.rs`, whose verbs make fresh copies; this one moves a card that already exists,
//! keeping its id, its `costMod` and everything else R78 carries between zones.
//!
//! Each card goes in at a uniformly random position, drawn from the match rng, through the one
//! shuffle-in the engine has (`draw::shuffle_into_library`): the `shuffledIn` event with its position
//! blanked for both players (R97), the owner shown what went in (R311) and R43's arrival roll.
//!
//! R80, R316: a card that would go into a full library does not. It STAYS where it is when it is in
//! its owner's graveyard already — no second move and no second `enteredGraveyard`, since it never
//! left — and `libraryOverflow { outcome: "graveyard" }` reports the refusal as for any existing card.
//! A card anywhere else is refused the way `shuffle_into_library` refuses an existing card: to its
//! owner's graveyard, or ceasing to exist if it is a unit-token card (R11).
//!
//! Port of `packages/engine/src/effects/shuffleCard.ts`.

use serde::{Deserialize, Serialize};

use crate::config::LIBRARY_CAP;
use crate::draw::shuffle_into_library;
use crate::script::Effect;
use crate::state::find_instance;
use crate::wire::{GameEvent, LibraryOverflowOutcome, Zone};

/// `shuffleCardInto`'s arguments: the card, by id.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ShuffleCardIntoArgs {
    pub instance_id: String,
}

/// §6.3, R80, R316: shuffle the card `instance_id` names into its owner's library at a random position.
/// A card that no longer exists is skipped. A graveyard card a full library refuses stays in that
/// graveyard, reported by `libraryOverflow`; any other refused card is handled as `shuffle_into_library`
/// handles an existing card.
pub fn shuffle_card_into(args: ShuffleCardIntoArgs) -> Effect {
    Effect::new("shuffleCardInto", move |ctx| {
        let Some(mut card) = find_instance(ctx.state, &args.instance_id).cloned() else {
            return;
        };
        let full = ctx.state.players[card.owner].library.len() as i32 >= LIBRARY_CAP;
        let in_own_graveyard = matches!(card.zone, Zone::Graveyard { player } if player == card.owner);
        if full && in_own_graveyard {
            ctx.events.push(GameEvent::LibraryOverflow {
                player: card.owner,
                instance_id: card.id.clone(),
                def_id: card.def_id.clone(),
                outcome: LibraryOverflowOutcome::Graveyard,
                radiant: if card.radiant { Some(true) } else { None },
                copy_of: None,
            });
            return;
        }
        shuffle_into_library(ctx, &mut card, true, None);
    })
}
