//! Swap (SPEC §6.3's Swap row, R73): Pocket Chaos (#87) exchanges one thing between the two players
//! — the two heroes' health, the board contents lane by lane, or the libraries.
//!
//! - Health: the two values change places, armor stays with its hero. Not damage, a heal or "lose
//!   health" (R18): no pipeline, no armor step; the only event is `swapped` (§10.3).
//! - Board: contents change sides lane by lane, in both rows of §3.1. A swapped card never leaves the
//!   field, so R78's reset never runs and everything on it comes along (R14). `controller` changes,
//!   which is an entry (R171): every card that lands, a dormant Stack card included, takes this turn
//!   as its `summonedTurn` and a fresh exertion. `owner` stays (R12, R747); locks stay with their zones
//!   (R73, §3.2); a face-down trap stays face-down, its `faceUp` untouched (R33).
//! - Library: the piles change places whole, in order, and each card's owner becomes the player
//!   whose library holds it, R12's one exception (R73). Fatigue is player state (§2.4) and stays.

use serde::{Deserialize, Serialize};

use crate::combat::enter_new_side;
use crate::own_library::hide_from_owner;
use crate::script::{Effect, EffectContext};
use crate::state::{CardInstance, GameState, find_instance};
use crate::wire::{GameEvent, PlayerId, Row, Zone, opponent_of};
use crate::zones::{
    PlaceOnFieldOptions, RemoveFromFieldOptions, ZoneSlot, is_locked, is_reserved, place_on_field,
    remove_from_field, slots_of, zone_contents,
};

use super::choose::chosen_options;
use super::move_::bounce_card;

/// The three things #87 can swap; the values are the `swapped` event's `what` (§10.3).
pub use crate::wire::SwapWhat;

const SWAP_WHATS: &[SwapWhat] = &[SwapWhat::Health, SwapWhat::Board, SwapWhat::Library];

/// §3.1: the board is two rows, and a board swap moves both.
pub const SWAP_ROWS: &[Row] = &[Row::Units, Row::Backrow];

/// One zone's worth of cards and where they are headed. A Stack pile travels whole (§3.2).
struct SwapEntry {
    #[allow(dead_code)]
    from: ZoneSlot,
    to: ZoneSlot,
    cards: Vec<CardInstance>,
}

/// Everything in a zone, top card first. A unit zone may hold a Stack pile, and the dormant cards
/// under the top are in the zone too, so they swap with it (§3.2).
fn contents_of(state: &GameState, slot: &ZoneSlot) -> Vec<CardInstance> {
    // B5 E21, R446: a backrow zone's pile and a carrier's Unit travel whole too (`zones::zone_contents`).
    zone_contents(state, slot)
}

/// R73: "lane-preserving" — the same row and lane on the other side of the centre line.
fn mirror_of(slot: &ZoneSlot) -> ZoneSlot {
    ZoneSlot {
        player: opponent_of(slot.player),
        row: slot.row,
        lane: slot.lane,
    }
}

/// Whether a destination can take a swapped card. R73 says locks stay with their zones but not what
/// happens to a card whose destination is Locked, or reserved for a dying Reborn unit (R64); R88
/// settles it, following R14: the card bounces to its controller's hand, #87's override of R688.
fn can_accept(state: &GameState, slot: &ZoneSlot) -> bool {
    !is_locked(state, slot) && !is_reserved(state, slot)
}

/// Put a zone's cards down in their new zone. A pile is rebuilt from the bottom up so the card that
/// was on top is on top again, which keeps the same card acting for the zone (§3.2).
fn place_contents(state: &mut GameState, cards: &[CardInstance], to: &ZoneSlot) {
    for (placed, card) in cards.iter().rev().enumerate() {
        // Every swapped zone was emptied and the destination accepts cards, so a refusal here is a
        // broken invariant, not a game rule.
        if !place_on_field(
            state,
            &mut card.clone(),
            to,
            PlaceOnFieldOptions {
                stack: Some(placed > 0),
            },
        ) {
            panic!(
                "swap could not place {} in {} {} {}",
                card.id, to.player, to.row, to.lane
            );
        }
    }
}

/// The bounce of the decision above: the card goes to its controller's hand (R747, shared with
/// §6.3 Bounce). The hand cap applies, so a full hand burns it (§2.4, R4), and a unit token ceases
/// to exist on the way and never reaches a hand (R11).
fn bounce_home(ctx: &mut EffectContext<'_>, card: &CardInstance) {
    bounce_card(ctx, card);
}

/// R73 health: the two values change places; armor stays with its hero.
fn swap_health_now(ctx: &mut EffectContext<'_>) {
    let me = ctx.controller;
    let them = opponent_of(me);
    let keep = ctx.state.players[me].hero.health;
    ctx.state.players[me].hero.health = ctx.state.players[them].hero.health;
    ctx.state.players[them].hero.health = keep;
    ctx.events.push(GameEvent::Swapped {
        what: SwapWhat::Health,
    });
}

/// R73 board: every zone's contents change sides, lane by lane, in both rows. The whole board is
/// read before anything is placed, so no card lands on a zone whose occupant has not moved yet.
/// Events (§10.3): `swapped` once, then `controlChanged` per card that landed, the controller's
/// zones first, units by lane then backrow (R68's order).
fn swap_board_now(ctx: &mut EffectContext<'_>) {
    let sides: [PlayerId; 2] = [ctx.controller, opponent_of(ctx.controller)];

    let mut entries: Vec<SwapEntry> = Vec::new();
    for player in sides {
        for row in SWAP_ROWS {
            for from in slots_of(player, *row) {
                let cards = contents_of(ctx.state, &from);
                if cards.is_empty() {
                    continue;
                }
                entries.push(SwapEntry {
                    to: mirror_of(&from),
                    from,
                    cards,
                });
            }
        }
    }

    ctx.events.push(GameEvent::Swapped {
        what: SwapWhat::Board,
    });

    for entry in &entries {
        for card in &entry.cards {
            remove_from_field(
                &mut *ctx.state,
                card,
                RemoveFromFieldOptions {
                    with_pile: Some(true),
                },
            );
        }
    }

    for entry in &entries {
        if !can_accept(ctx.state, &entry.to) {
            for card in &entry.cards {
                bounce_home(ctx, card);
            }
            continue;
        }

        let before: Vec<PlayerId> = entry.cards.iter().map(|card| card.controller).collect();
        place_contents(&mut *ctx.state, &entry.cards, &entry.to);

        // Every destination is on the other side, so every card that landed changed controller (R73),
        // dormant Stack cards included (§3.2), and has entered its new side (R171).
        for (at, card) in entry.cards.iter().enumerate() {
            // The card as it stands in its new zone.
            let landed = find_instance(ctx.state, &card.id)
                .cloned()
                .unwrap_or_else(|| card.clone());
            let from = before.get(at).copied().unwrap_or(card.owner);
            enter_new_side(ctx, &landed, from);
            ctx.events.push(GameEvent::ControlChanged {
                instance_id: card.id.clone(),
                controller: entry.to.player,
                row: entry.to.row,
                lane: entry.to.lane,
                former_id: None,
                how: None,
            });
        }
    }
}

/// R12's one exception: a card in the library that was swapped now belongs to the player holding it,
/// so it feeds that player's draws and later reaches that player's graveyard or exile. Off the field
/// control follows ownership (R78), and the zone records the new side.
fn claim_library(cards: &mut [CardInstance], player: PlayerId) {
    for card in cards {
        card.owner = player;
        card.controller = player;
        card.zone = Zone::Library { player };
        // R312: its new owner was never shown it, and what its old owner was shown is not theirs.
        hide_from_owner(card);
    }
}

/// R73 library: the two piles change places whole, in order, and change owners with them. They are
/// exchanged directly: `move_to_zone` routes a card to its own owner's pile (R12), which R73
/// overrides here. Nothing leaves the library, so R11 never fires on a unit-token card in one (#75).
fn swap_library_now(ctx: &mut EffectContext<'_>) {
    let mine = ctx.controller;
    let theirs = opponent_of(mine);

    let was_mine = std::mem::take(&mut ctx.state.players[mine].library);
    let was_theirs = std::mem::take(&mut ctx.state.players[theirs].library);
    ctx.state.players[mine].library = was_theirs;
    ctx.state.players[theirs].library = was_mine;

    claim_library(&mut ctx.state.players[mine].library, mine);
    claim_library(&mut ctx.state.players[theirs].library, theirs);

    // §2.4: fatigue is a property of the player, not of the library, so `fatigueCount` is untouched.
    ctx.events.push(GameEvent::Swapped {
        what: SwapWhat::Library,
    });
}

/// §6.3 Choose one: the answered mode arrives in `ctx.targets` or `ctx.modes` (§10.6, R81).
fn chosen_what(ctx: &EffectContext<'_>) -> Option<SwapWhat> {
    for option in chosen_options(ctx) {
        if let Some(what) = SWAP_WHATS.iter().find(|candidate| candidate.as_str() == option) {
            return Some(*what);
        }
    }
    None
}

/// `swap`'s argument.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct SwapArgs {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub what: Option<SwapWhat>,
}

/// §6.3 Swap (#87 Pocket Chaos): exchange one thing with the opponent. With no `what` the effect
/// reads the Choose one answer, so the card script stays declarative data (CLAUDE.md rule 5); an
/// answer naming none of the three fizzles and the rest of the card still resolves.
pub fn swap(args: SwapArgs) -> Effect {
    Effect::new("swap", move |ctx| {
        let Some(what) = args.what.or_else(|| chosen_what(ctx)) else {
            return;
        };
        match what {
            SwapWhat::Health => swap_health_now(ctx),
            SwapWhat::Board => swap_board_now(ctx),
            SwapWhat::Library => swap_library_now(ctx),
        }
    })
}

/// R73: the two heroes' health values change places; armor stays with its hero.
pub fn swap_health() -> Effect {
    Effect::new("swapHealth", swap_health_now)
}

/// R73: zone contents change sides lane by lane in both rows; locks stay, control moves, owners don't.
pub fn swap_board() -> Effect {
    Effect::new("swapBoard", swap_board_now)
}

/// R73: the libraries change places whole, and each swapped card's owner changes with it (R12).
pub fn swap_library() -> Effect {
    Effect::new("swapLibrary", swap_library_now)
}
