//! Exiling at random out of a hand (§6.3 Exile, R60): Classic #15 Nose Hunter's Radiant face, a
//! card-specific verb of the Classic #1–#45 workstream. Exile is public (§3.2), so the `exiled` event
//! names the card once it is there.

use serde::{Deserialize, Serialize};

use crate::effects::targets::{PlayerSpec, player_of};
use crate::script::Effect;
use crate::state::CardInstance;
use crate::zones::exile_to_pile;

/// `exileRandomFromHand`'s argument.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct ExileRandomFromHandArgs {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub count: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player: Option<PlayerSpec>,
}

/// §6.3 Exile, `count` distinct random cards out of a player's hand (R60), from the match rng. An
/// empty hand fizzles and draws nothing.
pub fn exile_random_from_hand(args: ExileRandomFromHandArgs) -> Effect {
    Effect::new("exileRandomFromHand", move |ctx| {
        let player = player_of(ctx, args.player.unwrap_or(PlayerSpec::SelfSide));
        let hand: Vec<CardInstance> = ctx.sink.state.players[player].hand.clone();
        let count = args.count.unwrap_or(1).max(0) as usize;
        if count == 0 || hand.is_empty() {
            return;
        }

        let picked: Vec<CardInstance> = ctx.sink.rng.shuffle(&hand).into_iter().take(count).collect();
        for card in picked {
            exile_to_pile(ctx, card);
        }
    })
}
