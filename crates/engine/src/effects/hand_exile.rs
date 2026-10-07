//! Exiling at random out of a hand (§6.3 Exile, R60): Classic #15 Nose Hunter's Radiant face, "Exile …
//! and a random card from their hand". A card-specific verb of the Classic #1–#45 workstream, kept in
//! its own file beside `library.ts`'s two library exiles, which name a library card positionally for
//! the same reason this names a hand card by lot: nobody chose it and nothing on the board points at it.
//!
//! The hand is hidden from the other seat until a card leaves it; exile is public (§3.2), so the
//! `exiled` event names the card once it is there, as every other exile does.
//!
//! Port of `packages/engine/src/effects/handExile.ts`.

use serde::{Deserialize, Serialize};

use crate::effects::targets::{PlayerSpec, player_of};
use crate::script::{Effect, EffectContext};
use crate::state::CardInstance;
use crate::wire::GameEvent;
use crate::zones::{MoveResult, OffFieldZone, move_to_zone};

/// One card to the exile pile, exactly as `library.ts` and `move.ts` exile one: the game exile counter
/// counts only a card that gets there (R55); a unit-token card ceases to exist instead (R11); the
/// `exiled` event reports the card leaving either way.
fn exile_card(ctx: &mut EffectContext<'_>, card: &CardInstance) {
    let moved = move_to_zone(ctx.sink.state, card, OffFieldZone::Exile, Default::default());
    if matches!(moved, MoveResult::Moved) {
        ctx.sink.state.counters.exiled += 1;
    }
    ctx.sink.events.push(GameEvent::Exiled {
        instance_id: card.id.clone(),
        def_id: card.def_id.clone(),
        owner: card.owner,
    });
}

/// `exileRandomFromHand`'s argument.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct ExileRandomFromHandArgs {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub count: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player: Option<PlayerSpec>,
}

/// §6.3 Exile, `count` random cards out of a player's hand (default one, the opponent's when
/// `player` is "enemy"). The picks are distinct (R60: a random pick of N picks N different cards, or
/// all of them if fewer exist), drawn from the match rng as `exileRandomFromLibrary` draws them, so
/// they depend on nothing but (seed, cursor). An empty hand fizzles: nothing moves and the rng is not
/// drawn from.
pub fn exile_random_from_hand(args: ExileRandomFromHandArgs) -> Effect {
    Effect::new("exileRandomFromHand", move |ctx| {
        let player = player_of(ctx, args.player.unwrap_or(PlayerSpec::SelfSide));
        let hand: Vec<CardInstance> = ctx.sink.state.players[player].hand.clone();
        let count = args.count.unwrap_or(1).max(0) as usize;
        if count == 0 || hand.is_empty() {
            return;
        }

        let picked: Vec<CardInstance> = ctx.sink.rng.shuffle(&hand).into_iter().take(count).collect();
        for card in &picked {
            exile_card(ctx, card);
        }
    })
}
