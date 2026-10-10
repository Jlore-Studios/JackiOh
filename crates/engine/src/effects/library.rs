//! Exiling out of a library: the two halves of §6.3's Exile row that name a library rather than a
//! target (#34 Collateral Damage, #40 Echoes of the Forgotten, #42 Eugenics, #65 Masochism Mask).
//!
//! `move_.rs`'s `exile` takes a `TargetSpec`, which is how a card names a permanent or a card a prompt
//! picked. A library card is neither: nobody chose it and nothing on the board points at it, so the
//! two verbs below name it positionally instead — at random, or at the bottom of the pile.
//!
//! WHICH END IS THE TOP. `library[0]` is the TOP: `draw.rs`'s `draw_one` takes it, and `zones.rs`'s
//! `move_to_zone(state, card, Library)` with no position puts a card back on top, while
//! `position: "bottom"` splices at `pile.len()`. So the BOTTOM card is `library[library.len() - 1]`.
//! Getting it backwards is silent: #40 eats the wrong end of the deck and no test that only counts
//! cards would notice.

use serde::{Deserialize, Serialize};

use crate::effects::targets::{PlayerSpec, player_of};
use crate::script::{Effect, EffectContext};
use crate::state::CardInstance;
use crate::wire::GameEvent;
use crate::zones::{MoveResult, OffFieldZone, move_to_zone};

/// One card to the exile pile, as `move_.rs`'s `exile` does it: the exile counter counts only cards
/// that get there, a unit-token library card ceases to exist instead and is not counted (R11, §3.2),
/// and the `exiled` event reports the card leaving either way for §10.10.
///
/// §6.3's Exile row gives no Death trigger, so nothing is dispatched here.
fn exile_card(ctx: &mut EffectContext<'_>, card: &CardInstance) {
    let mut card = card.clone();
    let moved = move_to_zone(ctx.state, &mut card, OffFieldZone::Exile, Default::default());
    if matches!(moved, MoveResult::Moved) {
        ctx.state.counters.exiled += 1;
    }
    ctx.events.push(GameEvent::Exiled {
        instance_id: card.id.clone(),
        def_id: card.def_id.clone(),
        owner: card.owner,
    });
}

/// `exileRandomFromLibrary`'s arguments: `count` defaults to 1, `player` to "self".
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct ExileRandomFromLibraryArgs {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub count: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player: Option<PlayerSpec>,
}

/// §6.3 Exile, `count` random cards out of a library: #34 Collateral Damage, #42 Eugenics.
///
/// The cards are DISTINCT (R60, and #42's "Fewer than 8 → exile all"): fewer than `count` in the
/// library exiles the whole library. `shuffle` copies the list first, so the picks depend only on
/// (seed, cursor) and the loop may empty the real library underneath it.
///
/// An empty library fizzles and the card still resolves (§6.3). No draw happens, so no fatigue.
pub fn exile_random_from_library(args: ExileRandomFromLibraryArgs) -> Effect {
    Effect::new("exileRandomFromLibrary", move |ctx| {
        let player = player_of(ctx, args.player.unwrap_or(PlayerSpec::SelfSide));
        let count = args.count.unwrap_or(1).max(0);
        if count == 0 || ctx.state.players[player].library.is_empty() {
            return;
        }

        let picks = ctx.sink.rng.shuffle(&ctx.sink.state.players[player].library);
        for card in picks.iter().take(count as usize) {
            exile_card(ctx, card);
        }
    })
}

/// `exileBottomOfLibrary`'s arguments: `player` defaults to "self", `count` to 1.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct ExileBottomOfLibraryArgs {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player: Option<PlayerSpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub count: Option<i32>,
}

/// §6.3 Exile, the BOTTOM card of a library: #40 Echoes of the Forgotten, #65 Masochism Mask. The
/// bottom is the last element (module header).
///
/// An empty library does nothing (#40's "no exile, no fatigue"): this verb never reaches `draw.rs`,
/// and §2.4's fatigue is the price of a DRAW, which this is not.
///
/// Each iteration re-reads the pile, so `count` 2 takes the bottom two and stops early on an empty one.
pub fn exile_bottom_of_library(args: ExileBottomOfLibraryArgs) -> Effect {
    Effect::new("exileBottomOfLibrary", move |ctx| {
        let player = player_of(ctx, args.player.unwrap_or(PlayerSpec::SelfSide));
        let count = args.count.unwrap_or(1).max(0);

        for _ in 0..count {
            let Some(card) = ctx.state.players[player].library.last().cloned() else {
                return;
            };
            exile_card(ctx, &card);
        }
    })
}
