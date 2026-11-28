//! Swap (SPEC §6.3's Swap row, R73): Pocket Chaos (#87) exchanges one thing between the two players
//! — the two heroes' health, the board contents lane by lane, or the libraries.
//!
//! - Health: not damage, a heal or "lose health" (R18): no pipeline, no armor step; the only event is
//!   `swapped` (§10.3).
//! - Board: a swapped card never leaves the field, so R78's reset never runs and everything on it
//!   comes along (R14). The new `controller` is an entry (R171): this turn's `summonedTurn` and a fresh
//!   exertion, a dormant Stack card's too. `owner` stays (R12). Locks stay with their zones (R73,
//!   §3.2). A face-down trap stays face-down, readable by its new controller only: `view_for` keys that
//!   on `controller`, so `faceUp` is untouched (R33).

use serde::{Deserialize, Serialize};

use crate::combat::enter_new_side;
use crate::own_library::hide_from_owner;
use crate::script::{Effect, EffectContext};
use crate::state::{CardInstance, GameState};
use crate::wire::{GameEvent, PlayerId, Row, Zone, opponent_of};
use crate::zones::{
    PlaceOnFieldOptions, RemoveFromFieldOptions, ZoneSlot, carried_at, is_empty, is_locked, is_reserved,
    place_on_field, place_standing_on_field, remove_from_field, slots_of, zone_contents,
};

use super::choose::chosen_options;
use super::move_::bounce_card;

/// The three things #87 can swap; the values are the `swapped` event's `what` (§10.3).
pub use crate::wire::SwapWhat;

const SWAP_WHATS: &[SwapWhat] = &[SwapWhat::Health, SwapWhat::Board, SwapWhat::Library];

/// §3.1: the board is two rows, and a board swap moves both.
pub const SWAP_ROWS: &[Row] = &[Row::Units, Row::Backrow];

/// One zone's worth of cards and where they are headed.
struct SwapEntry {
    to: ZoneSlot,
    cards: Vec<CardInstance>,
    /// R446: the Unit a carrier holds in this backrow zone, if any, which goes back on its carrier;
    /// every other card lands where it lay (R1081000).
    carried: Option<String>,
}

/// Everything in a zone, top card first: a Stack pile's dormant cards swap with it (§3.2).
fn contents_of(state: &GameState, slot: &ZoneSlot) -> Vec<CardInstance> {
    // B5 E21, R446: a backrow zone's pile and a carrier's Unit travel whole too (`zones::zone_contents`).
    zone_contents(state, slot)
}

/// R73: "lane-preserving" — the same row and lane on the other side of the center line.
fn mirror_of(slot: &ZoneSlot) -> ZoneSlot {
    ZoneSlot {
        player: opponent_of(slot.player),
        row: slot.row,
        lane: slot.lane,
    }
}

/// R88 (after R14): a card whose destination is Locked, or reserved for a dying Reborn unit (R64),
/// bounces to its controller's hand instead, #87's override of R688.
fn can_accept(state: &GameState, slot: &ZoneSlot) -> bool {
    // R3554002: a void destination bounces the card instead of panicking in `place_contents`.
    !is_locked(state, slot) && !is_reserved(state, slot) && !crate::geometry::is_void(state, *slot)
}

/// A pile is rebuilt from the bottom up, so the same card is on top, acting for the zone (§3.2).
fn place_contents(state: &mut GameState, cards: &[CardInstance], carried: Option<&str>, to: &ZoneSlot) {
    for (placed, card) in cards.iter().rev().enumerate() {
        // Every swapped zone was emptied, so a refusal here is a broken invariant, not a game rule.
        let options = PlaceOnFieldOptions {
            stack: Some(placed > 0),
        };
        let landed = if carried == Some(card.id.as_str()) {
            place_on_field(state, &mut card.clone(), to, options)
        } else {
            place_standing_on_field(state, &mut card.clone(), to, options)
        };
        if !landed {
            panic!(
                "swap could not place {} in {} {} {}",
                card.id, to.player, to.row, to.lane
            );
        }
    }
}

/// R88's bounce, to the controller's hand as §6.3 Bounce's (R747): a full hand burns it (§2.4, R4), and
/// a unit token ceases to exist on the way (R11).
fn bounce_home(ctx: &mut EffectContext<'_>, card: &CardInstance) {
    bounce_card(ctx, card);
}

/// R73 health: the two values change places; armor stays with its hero. #936 B18: in a Duos duel
/// the values are the heroes' own HP, and while either hero is spent the swap does nothing at all.
fn swap_health_now(ctx: &mut EffectContext<'_>) {
    let me = ctx.controller;
    let them = opponent_of(me);
    if !crate::duos_life::heroes_swap_health(&mut *ctx.state, me, them) {
        return;
    }
    ctx.events.push(GameEvent::Swapped {
        what: SwapWhat::Health,
    });
}

/// R73 board: the whole board is read before anything is placed, so no card lands on a zone whose
/// occupant has not moved yet. Events (§10.3): `swapped`, then `controlChanged` per card that landed,
/// the controller's zones first, units by lane then backrow (R68's order).
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
                let carried = carried_at(ctx.state, from).map(|unit| unit.id.clone());
                entries.push(SwapEntry {
                    to: mirror_of(&from),
                    cards,
                    carried,
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
        // R3546003: a GIGA swaps whole or bounces: every spanned slot must take it.
        if entry.cards.first().is_some_and(crate::giga::is_giga)
            && !crate::giga::fits(entry.to, |slot| {
                can_accept(ctx.state, &slot) && is_empty(ctx.state, slot)
            })
        {
            for card in &entry.cards {
                bounce_home(ctx, card);
            }
            continue;
        }

        let before: Vec<PlayerId> = entry.cards.iter().map(|card| card.controller).collect();
        place_contents(&mut *ctx.state, &entry.cards, entry.carried.as_deref(), &entry.to);

        // Every card that landed changed controller (R73), dormant ones too, and entered its new side.
        for (at, card) in entry.cards.iter().enumerate() {
            // The card as it stands in its new zone.
            let landed = crate::zones::live_or(ctx.state, card);
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

/// R12's one exception: a swapped library card belongs to the player holding it, and off the field
/// control follows ownership (R78).
fn claim_library(kept: bool, cards: &mut [CardInstance], player: PlayerId) {
    for card in cards {
        crate::origin::note_origin(kept, card);
        card.owner = player;
        card.controller = player;
        card.zone = Zone::Library { player };
        // R312: its new owner was never shown it, and what its old owner was shown is not theirs.
        hide_from_owner(card);
    }
}

/// R73 library: the piles are exchanged directly, as `move_to_zone` routes a card to its own owner's
/// pile (R12). Nothing leaves the library, so R11 never fires on a unit-token card in one (#75).
fn swap_library_now(ctx: &mut EffectContext<'_>) {
    let mine = ctx.controller;
    let theirs = opponent_of(mine);

    let was_mine = std::mem::take(&mut ctx.state.players[mine].library);
    let was_theirs = std::mem::take(&mut ctx.state.players[theirs].library);
    ctx.state.players[mine].library = was_theirs;
    ctx.state.players[theirs].library = was_mine;

    // R4592002: the origin each card started the game in, before the claim changes its owner.
    let kept = crate::recursion_log::kept(ctx.state);
    claim_library(kept, &mut ctx.state.players[mine].library, mine);
    claim_library(kept, &mut ctx.state.players[theirs].library, theirs);

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
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct SwapArgs {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub what: Option<SwapWhat>,
}

/// §6.3 Swap (#87 Pocket Chaos). With no `what` it reads the Choose one answer, so the card script
/// stays data (CLAUDE.md rule 5); an answer naming none of the three fizzles, and the rest resolves.
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

/// R73: the libraries change places whole, and each swapped card's owner changes with it (R12).
pub fn swap_library() -> Effect {
    Effect::new("swapLibrary", swap_library_now)
}

/// `swapHands`'s arguments.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct SwapHandsArgs {
    /// R4586008: the discount the cards the controller takes now carry (`costMod`, below 0 a
    /// discount), on top of what they hold. Absent, the taken cards keep their price.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost_mod: Option<i32>,
    /// R5238013: at the end of the swapping player's turn (this one, or their next when the swap came on
    /// the other player's turn), every swapped card still in the hand the swap put it in goes back to the
    /// hand it came from, owned and controlled there again, and a card the swapper took gives back its
    /// `costMod`. A card that left that hand since (played, discarded, moved) stays where it is.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub until_end_of_turn: Option<bool>,
}

/// R5238013: the `Resume.hook` an `untilEndOfTurn` hand swap's return waits under, which the engine
/// runs itself (`effects::delay::run_engine_delayed`).
pub const DELAYED_SWAP_BACK_HOOK: &str = "@delayedSwapBack";
/// R5238013: the return's `data` key: each swapped card as `[instanceId, seat]`, the seat ("p1"/"p2")
/// whose hand it was in before the swap, in hand order, the swapper's hand first.
pub const SWAP_BACK_FROM_KEY: &str = "from";
/// R5238013: the return's `data` key for the swap's `costMod`, given back by the cards the swapper took.
pub const SWAP_BACK_COST_KEY: &str = "costMod";

/// R5238013: schedule the return of `was_mine` and `was_theirs` at the end of `mine`'s turn: one engine
/// delayed entry watching each card's stay in the hand the swap put it in (`DelayedEffect.handWatch`,
/// R1140), so a card that leaves that hand drops out of it, and the entry with the last. No mark.
fn schedule_swap_back(
    ctx: &mut EffectContext<'_>,
    mine: PlayerId,
    was_mine: &[CardInstance],
    was_theirs: &[CardInstance],
    cost_mod: Option<i32>,
) {
    let theirs = opponent_of(mine);
    let mut from: Vec<serde_json::Value> = Vec::new();
    let mut ids: Vec<String> = Vec::new();
    for (seat, cards) in [(mine, was_mine), (theirs, was_theirs)] {
        for card in cards {
            from.push(serde_json::Value::Array(vec![
                serde_json::Value::String(card.id.clone()),
                serde_json::Value::String(seat.as_str().to_string()),
            ]));
            ids.push(card.id.clone());
        }
    }
    if ids.is_empty() {
        return;
    }
    let mut data: indexmap::IndexMap<String, serde_json::Value> = indexmap::IndexMap::new();
    data.insert(SWAP_BACK_FROM_KEY.to_string(), serde_json::Value::Array(from));
    if let Some(discount) = cost_mod {
        data.insert(SWAP_BACK_COST_KEY.to_string(), serde_json::Value::from(discount));
    }
    let def_id = ctx
        .def_id
        .clone()
        .or_else(|| ctx.self_.as_ref().map(|card| card.def_id.clone()))
        .unwrap_or_default();
    let resume = crate::state::Resume {
        def_id,
        hook: DELAYED_SWAP_BACK_HOOK.to_string(),
        step: "back".to_string(),
        radiant: ctx.radiant,
        instance_id: None,
        data,
    };
    let at = crate::state::DelayedAt {
        phase: crate::wire::Phase::End,
        player: mine,
    };
    let entry = crate::modifiers::schedule_delayed(ctx, mine, at, resume, None, None);
    if let Some(stored) = ctx.state.delayed.iter_mut().find(|due| due.id == entry.id) {
        stored.hand_watch = Some(ids);
    }
}

/// R5238013: the swap's return, as its entry comes due: each card it still watches goes back from the
/// hand the swap put it in to the hand it came from (`ownership::change_owner`, then the move, its stay
/// in the old hand ended), at that hand's right end in the order it left, with no hand cap (as the swap
/// has none); a card the swapper took gives back the swap's `costMod`.
pub(crate) fn swap_back(ctx: &mut EffectContext<'_>, effect: &crate::state::DelayedEffect) {
    let watched: Vec<String> = effect.hand_watch.clone().unwrap_or_default();
    let swapper = effect.owner;
    let discount = effect
        .resume
        .data
        .get(SWAP_BACK_COST_KEY)
        .and_then(serde_json::Value::as_i64)
        .map(|discount| i32::try_from(discount).unwrap_or(0));
    let from: Vec<(String, PlayerId)> = effect
        .resume
        .data
        .get(SWAP_BACK_FROM_KEY)
        .and_then(serde_json::Value::as_array)
        .map(|pairs| {
            pairs
                .iter()
                .filter_map(|pair| {
                    let id = pair.get(0)?.as_str()?;
                    let seat = pair.get(1)?.as_str()?;
                    let seat = crate::wire::PLAYER_IDS
                        .into_iter()
                        .find(|player| player.as_str() == seat)?;
                    Some((id.to_string(), seat))
                })
                .collect()
        })
        .unwrap_or_default();
    for (id, origin) in from {
        if !watched.contains(&id) {
            continue;
        }
        let Some(card) = crate::state::find_instance(ctx.state, &id).cloned() else {
            continue;
        };
        let Zone::Hand { player: holder } = card.zone else {
            continue;
        };
        if holder == origin {
            continue;
        }
        crate::ownership::change_owner(&mut ctx.sink, &card, origin);
        let mut card = card;
        crate::zones::remove_from_any_zone(ctx.state, &mut card);
        card.zone = Zone::Hand { player: origin };
        if origin != swapper
            && let Some(discount) = discount
        {
            card.cost_mod -= discount;
        }
        ctx.state.players[origin].hand.push(card);
    }
}

/// R4586008 (Infinite Assault #48 Ego Death): both hands change holders at once, each card owned
/// and controlled by its new holder (`ownership::change_owner`, R466's readers judged in place),
/// then each taken card of the controller's new hand carries `costMod` more. No hand cap burns a
/// card (§2.4's cap is a draw's) and no `entersHand` hook runs: the hands move, nothing is drawn.
pub fn swap_hands(args: SwapHandsArgs) -> Effect {
    Effect::new("swapHands", move |ctx| {
        let mine = ctx.controller;
        let theirs = opponent_of(mine);
        let was_mine: Vec<CardInstance> = ctx.state.players[mine].hand.clone();
        let was_theirs: Vec<CardInstance> = ctx.state.players[theirs].hand.clone();
        for card in was_mine.iter().chain(was_theirs.iter()) {
            let thief = if card.owner == mine { theirs } else { mine };
            crate::ownership::change_owner(&mut ctx.sink, card, thief);
        }
        let mut mine_hand = std::mem::take(&mut ctx.state.players[mine].hand);
        let mut theirs_hand = std::mem::take(&mut ctx.state.players[theirs].hand);
        std::mem::swap(&mut mine_hand, &mut theirs_hand);
        ctx.state.players[mine].hand = mine_hand;
        ctx.state.players[theirs].hand = theirs_hand;
        for player in [mine, theirs] {
            for card in ctx.state.players[player].hand.clone() {
                if let Some(live) = crate::state::find_instance_mut(ctx.sink.state, &card.id) {
                    live.zone = Zone::Hand { player };
                }
            }
        }
        if let Some(discount) = args.cost_mod {
            for card in ctx.state.players[mine].hand.clone() {
                if let Some(live) = crate::state::find_instance_mut(ctx.sink.state, &card.id) {
                    live.cost_mod += discount;
                }
            }
        }
        // R5238013: the return at the end of the swapper's turn.
        if args.until_end_of_turn == Some(true) {
            schedule_swap_back(ctx, mine, &was_mine, &was_theirs, args.cost_mod);
        }
    })
}
