//! Exiling out of a library: the halves of §6.3's Exile row that name a library card by position (at
//! random, or the bottom) rather than a `TargetSpec` (#34, #40, #42, #65).
//!
//! `library[0]` is the TOP (`draw::draw_one` takes it), so the BOTTOM is the last element; getting it
//! backwards is silent, as a test that only counts cards would not notice.

use serde::{Deserialize, Serialize};

use crate::effects::targets::{PlayerSpec, player_of};
use crate::script::Effect;
use crate::zones::exile_to_pile;

/// `exileRandomFromLibrary`'s arguments: `count` defaults to 1, `player` to "self".
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct ExileRandomFromLibraryArgs {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub count: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player: Option<PlayerSpec>,
}

/// §6.3 Exile, `count` distinct random cards out of a library (R60): fewer exiles the whole library.
/// `shuffle` copies the list, so the loop may empty the real library underneath it; no fatigue.
pub fn exile_random_from_library(args: ExileRandomFromLibraryArgs) -> Effect {
    Effect::new("exileRandomFromLibrary", move |ctx| {
        let player = player_of(ctx, args.player.unwrap_or(PlayerSpec::SelfSide));
        let count = args.count.unwrap_or(1).max(0);
        if count == 0 || ctx.state.players[player].library.is_empty() {
            return;
        }

        let picks = ctx.sink.rng.shuffle(&ctx.sink.state.players[player].library);
        for card in picks.iter().take(count as usize) {
            exile_to_pile(ctx, card.clone());
        }
    })
}

/// `exileBottomOfLibrary`'s arguments: `player` defaults to "self", `count` to 1.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct ExileBottomOfLibraryArgs {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player: Option<PlayerSpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub count: Option<i32>,
}

/// §6.3 Exile, the bottom `count` cards of a library, stopping early on an empty one: no draw, so no
/// fatigue (§2.4).
pub fn exile_bottom_of_library(args: ExileBottomOfLibraryArgs) -> Effect {
    Effect::new("exileBottomOfLibrary", move |ctx| {
        let player = player_of(ctx, args.player.unwrap_or(PlayerSpec::SelfSide));
        let count = args.count.unwrap_or(1).max(0);

        for _ in 0..count {
            let Some(card) = ctx.state.players[player].library.last().cloned() else {
                return;
            };
            exile_to_pile(ctx, card);
        }
    })
}
