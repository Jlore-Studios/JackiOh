//! Exiling out of a library: the halves of §6.3's Exile row that name a library card by position (at
//! random, or the bottom) rather than a `TargetSpec` (#34, #40, #42, #65); and a whole library sent to the
//! graveyard (R3951001).
//!
//! `library[0]` is the TOP (`draw::draw_one` takes it), so the BOTTOM is the last element; getting it
//! backwards is silent, as a test that only counts cards would not notice.

use serde::{Deserialize, Serialize};

use crate::effects::targets::{PlayerSpec, player_of};
use crate::script::Effect;
use crate::state::{CardInstance, find_instance};
use crate::wire::{PLAYER_IDS, Selection};
use crate::zones::{OffFieldZone, exile_to_pile, move_to_zone, report_graveyard_landing};

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

/// `discardLibrary`'s arguments: `player` defaults to "self".
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct DiscardLibraryArgs {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player: Option<PlayerSpec>,
}

/// `putOnLibraryBottom`'s arguments: none.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct PutOnLibraryBottomArgs {}

/// R4586005 (Infinite Assault #31 Unreliable Narrator): each picked card still in a library goes to
/// that library's bottom, in pick order; the rest keep their order. No rng, no event: nobody
/// reads a library (R177).
pub fn put_on_library_bottom(_args: PutOnLibraryBottomArgs) -> Effect {
    Effect::new("putOnLibraryBottom", move |ctx| {
        let ids: Vec<String> = ctx
            .targets
            .iter()
            .filter_map(|selection| match selection {
                Selection::Instance { instance_id } => Some(instance_id.clone()),
                _ => None,
            })
            .collect();
        for id in ids {
            for player in PLAYER_IDS {
                let Some(at) = ctx.state.players[player]
                    .library
                    .iter()
                    .position(|card| card.id == id)
                else {
                    continue;
                };
                let card = ctx.state.players[player].library.remove(at);
                ctx.state.players[player].library.push(card);
                break;
            }
        }
    })
}

/// R3951001: "discard your deck" (Classic #28 Second Wind's Radiant face): every card of a library goes to
/// its owner's graveyard, top first, each reported as it lands (`enteredGraveyard`, or where a "would go to
/// a graveyard" replacement sends it, B5 E5), a unit token ceasing to exist instead (R11). It is no draw,
/// so no fatigue (§2.4), and no discard from a hand: no `discarded`, no discard guard (R800), nothing that
/// answers a discard.
pub fn discard_library(args: DiscardLibraryArgs) -> Effect {
    Effect::new("discardLibrary", move |ctx| {
        let player = player_of(ctx, args.player.unwrap_or(PlayerSpec::SelfSide));
        let pile: Vec<CardInstance> = ctx.state.players[player].library.clone();
        for mut card in pile {
            let moved = move_to_zone(ctx.state, &mut card, OffFieldZone::Graveyard, Default::default());
            let landed = find_instance(ctx.state, &card.id).cloned().unwrap_or(card);
            report_graveyard_landing(ctx, &landed, moved);
        }
    })
}
