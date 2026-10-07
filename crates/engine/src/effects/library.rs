//! Exiling out of a library: the two halves of §6.3's Exile row that name a library rather than a
//! target (#34 Collateral Damage, #40 Echoes of the Forgotten, #42 Eugenics, #65 Masochism Mask).
//!
//! `move_.rs`'s `exile` takes a `TargetSpec`, which is how a card names a permanent or a card a prompt
//! picked. A library card is neither: nobody chose it and nothing on the board points at it, so the
//! two verbs below name it positionally instead — at random, or at the bottom of the pile.
//!
//! WHICH END IS THE TOP. `state.players[p].library[0]` is the TOP: `draw.rs`'s `draw_one` takes
//! `side.library[0]` and splices it out, and `zones.rs`'s `move_to_zone(state, card, Library)` with
//! no position puts a card back on top, while `position: "bottom"` splices at `pile.len()`. So the
//! BOTTOM card of a library is its LAST element, `library[library.len() - 1]`. This comment exists
//! because getting it backwards is silent: the game plays on, #40 eats the wrong end of the deck, and
//! no test that only counts cards would notice.
//!
//! Port of `packages/engine/src/effects/library.ts`.

use serde::{Deserialize, Serialize};

use crate::effects::targets::{PlayerSpec, player_of};
use crate::script::{Effect, EffectContext};
use crate::state::CardInstance;
use crate::wire::GameEvent;
use crate::zones::{MoveResult, OffFieldZone, move_to_zone};

/// One card to the exile pile, exactly as `move_.rs`'s `exile` does it: the game exile counter counts
/// only the cards that actually get there, a unit-token library card ceases to exist instead and is
/// not counted (R11 — §3.2 names Infinite Reserves and the Unstable Clone Machine / Recycling
/// Initiative / Combo-Index copies as the cards that can be in a library at all), and the `exiled`
/// event reports the card leaving either way so §10.10 can animate it.
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

/// §6.3 Exile, `count` random cards out of a library: #34 Collateral Damage's "a random card from the
/// opponent's library", #42 Eugenics' "Exile 8 random cards from your library".
///
/// The cards are DISTINCT. R60 rules that "a random pick of N existing cards picks N different cards,
/// or all of them if fewer exist", and #42's engine cell says "Fewer than 8 → exile all" — so fewer
/// than `count` in the library exiles the whole library rather than drawing the short end twice.
/// `ctx.rng.shuffle(library)` then the first `count` is the deterministic form of both: `shuffle`
/// copies the list before its Fisher-Yates walk, so the picks depend on nothing but (seed, cursor) and
/// the loop may empty the real library underneath it.
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

/// §6.3 Exile, the BOTTOM card of a library: #40 Echoes of the Forgotten's "then exile the bottom
/// card of your library" and #65 Masochism Mask's "exile the bottom card of your library". The bottom
/// is the last element — see the module header for how that was verified against `draw_one`.
///
/// #40's engine cell: "empty library → no exile, no fatigue". So an empty library does nothing at
/// all: this verb never reaches `draw.rs`, so §2.4's fatigue clock is untouched and `fatigue_count`
/// does not move. Fatigue is the price of a DRAW from an empty library, and this is not a draw.
///
/// `count` is here for a card that takes more than one; each iteration re-reads the pile, so exiling
/// the bottom twice takes the bottom two cards bottom-upward and stops early on an empty library.
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
