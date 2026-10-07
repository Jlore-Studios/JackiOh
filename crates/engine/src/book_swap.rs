//! Classic #55 Book of Wildfire's swap (SPEC §8.6 row 55, R671): "Becomes a different Book at the end
//! of your turn." One hand trigger, printed by Wildfire's own script and granted to every Book it
//! becomes by the `swapsBook` enchantment it leaves on them (B5 E39), so the swap goes on turn after
//! turn while the card stays in its owner's hand. The registry reads the grant (`triggers::holder_of`):
//! a hand card that carries the enchantment answers this trigger as if its text printed it.
//!
//! Port of `packages/engine/src/bookSwap.ts`. TS's `BOOK_SWAP_TRIGGER` constant holds closures, which a
//! Rust `const` cannot, so it is `book_swap_trigger()`, which builds the same trigger each call.

use serde_json::json;

use crate::effects::transform::{book_swap_source_of, swap_book};
use crate::prelude::json_as;
use crate::script::{Effect, EffectContext, TriggerDef};
use crate::state::CardInstance;
use crate::wire::{GameEvent, GameEventType, ZoneName};

pub const BOOK_SWAP_TRIGGER_ID: &str = "book-swap";

/// At the end of its controller's turn (that turn's `turnEnded`, which hand triggers answer once the
/// end-of-turn triggers have run), the card in hand becomes a different Book. A swapped Book names the
/// card that started the swap from its enchantment; Wildfire itself is that card.
///
/// (TS `BOOK_SWAP_TRIGGER`.)
pub fn book_swap_trigger() -> TriggerDef {
    TriggerDef::new(BOOK_SWAP_TRIGGER_ID, &[GameEventType::TurnEnded], run_book_swap)
}

fn run_book_swap(ctx: &mut EffectContext<'_>, event: &GameEvent) -> Vec<Effect> {
    let Some(self_) = ctx.live_self().cloned() else {
        return vec![];
    };
    if self_.zone.z() != ZoneName::Hand {
        return vec![];
    }
    match event {
        GameEvent::TurnEnded { player, .. } if *player == ctx.controller => {}
        _ => return vec![],
    }
    let from = book_swap_source_of(&self_)
        .map(|source| source.to_string())
        .unwrap_or_else(|| self_.def_id.clone());
    vec![swap_book(json_as(
        json!({ "instanceId": self_.id, "from": from }),
    ))]
}

/// R671: the hand triggers a card's enchantments grant it — the swap, once, unless its text prints it.
pub fn granted_hand_triggers(card: &CardInstance, printed: &[TriggerDef]) -> Vec<TriggerDef> {
    if book_swap_source_of(card).is_none() {
        return printed.to_vec();
    }
    if printed.iter().any(|def| def.id == BOOK_SWAP_TRIGGER_ID) {
        return printed.to_vec();
    }
    let mut out = printed.to_vec();
    out.push(book_swap_trigger());
    out
}
