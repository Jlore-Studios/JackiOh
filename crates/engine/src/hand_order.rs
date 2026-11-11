//! Hand neighbours (Stinkyscape+ E8, R3558001, R3558002): a hand card's aura on the cards beside
//! it, and the pairs that meet.
//!
//! A `handAura` entry names which neighbour takes it: the card to the lender's left takes a `Right`
//! lend, the card to the right a `Left` one. A lent card is Radiant while the lend holds, and its
//! price moves by the lend's cost (`mana::price_of` reads the delta). The state check syncs the
//! lends after every settled board (`sync_hand_auras`): a card that leaves the hand keeps its
//! Radiance, and one made Radiant on its own (`effects::radiant::make_radiant`) keeps it too — its
//! lend is forgotten. Nothing here draws from the rng, so lends never move a game's hash.
//!
//! A meeting pair (`HandMeet`) waits in `state.hand_meets` until its two cards sit side by side in
//! the hand ("when they meet", R3558002), however they got there — added that way, or passed by
//! every card between them leaving. Then both become Radiant at once, at the meet's price when it
//! names one. The owner's hand view marks a card still waiting (`MEET_MARK`); the opponent's view
//! never sees it (R97).

use indexmap::IndexMap;

use crate::query::HandSide;
use crate::resolve::{HookOptions, make_context};
use crate::script::{EngineSink, HookArgs};
use crate::state::{CardInstance, GameState, active_first, find_instance, find_instance_mut};
use crate::wire::{GameEvent, PlayerId};

/// R3558002: the mark a card of a waiting pair wears in its owner's hand view, drawn in the
/// client's gold for a promised Radiance.
pub const MEET_MARK: &str = "meet";
/// R3558002: that mark's color.
pub const MEET_MARK_COLOR: &str = "gold";

/// R3558001: what `card`'s price moves by for sitting beside a lender — the `cost` of every
/// `handAura` entry aimed at it: its left neighbour's entries with `to` right, its right
/// neighbour's with `to` left. 0 for a card in no hand, or beside no lender.
pub fn hand_aura_cost_delta(state: &GameState, card: &CardInstance) -> i32 {
    let Some((player, at)) = crate::query::hand_index(state, &card.id) else {
        return 0;
    };
    let hand = &state.players[player].hand;
    let mut delta = 0;
    for (side, neighbour) in [
        (HandSide::Right, at.checked_sub(1).and_then(|left| hand.get(left))),
        (HandSide::Left, hand.get(at + 1)),
    ] {
        let Some(neighbour) = neighbour else {
            continue;
        };
        let script = crate::scripts::script_of(state, neighbour);
        let Some(lends) = script.hand_aura.as_ref() else {
            continue;
        };
        for lend in lends(HookArgs {
            state,
            self_: neighbour,
            radiant: neighbour.radiant,
        }) {
            if lend.to == side {
                delta += lend.cost;
            }
        }
    }
    delta
}

/// R3558001: every card lent Radiance now — each lent neighbour's id to its side and its lender —
/// in walk order (the active player's side first, each hand left to right), the first lender kept
/// when two lend one card. A cost-only lend (`radiant: false`) moves no Radiance and leaves no
/// record; `mana::price_of` reads it straight from the lender (`hand_aura_cost_delta`).
fn wanted_lends(state: &GameState) -> IndexMap<String, (PlayerId, String)> {
    let mut wanted: IndexMap<String, (PlayerId, String)> = IndexMap::new();
    for player in active_first(state) {
        let hand = &state.players[player].hand;
        for (at, lender) in hand.iter().enumerate() {
            let script = crate::scripts::script_of(state, lender);
            let Some(lends) = script.hand_aura.as_ref() else {
                continue;
            };
            let entries = lends(HookArgs {
                state,
                self_: lender,
                radiant: lender.radiant,
            });
            for lend in entries {
                if !lend.radiant {
                    continue;
                }
                let neighbour = match lend.to {
                    HandSide::Right => hand.get(at + 1),
                    HandSide::Left => at.checked_sub(1).and_then(|left| hand.get(left)),
                };
                if let Some(neighbour) = neighbour {
                    wanted
                        .entry(neighbour.id.clone())
                        .or_insert((player, lender.id.clone()));
                }
            }
        }
    }
    wanted
}

/// R3558001: bring the recorded lends up to what the hands hold now, at the settled point. A lend
/// whose card is no longer wanted goes; a card still in its side's hand loses the Radiance it only
/// held as a lend, while one that left keeps it. A wanted card stays lent — re-Radianted when a
/// De-Radiant cleared it — and a card wanted but neither lent nor Radiant becomes Radiant as a
/// lend. Absent lends stay absent: `None` comes back when nothing is lent.
pub fn sync_hand_auras(sink: &mut EngineSink<'_>) {
    let wanted = wanted_lends(sink.state);
    let mut lends: Vec<crate::state::RadianceLend> = sink.state.radiance_lends.clone().unwrap_or_default();
    lends.retain(|lend| {
        if wanted.contains_key(&lend.card) {
            return true;
        }
        // No longer wanted: a card still in its side's hand loses the lent Radiance.
        let live = find_instance(sink.state, &lend.card).cloned();
        let owed_back = live.as_ref().is_some_and(|live| {
            live.radiant
                && crate::query::hand_index(sink.state, &live.id)
                    .is_some_and(|(player, _)| player == lend.player)
        });
        if !owed_back {
            return false;
        }
        if let Some(live) = live
            && let Some(card) = find_instance_mut(sink.state, &lend.card)
        {
            card.radiant = false;
            sink.events.push(GameEvent::Deradianted {
                instance_id: lend.card.clone(),
                def_id: live.def_id.clone(),
                zone: live.zone.clone(),
            });
        }
        false
    });
    for (card_id, (player, lender)) in &wanted {
        if let Some(kept) = lends.iter_mut().find(|lend| lend.card == *card_id) {
            kept.player = *player;
            kept.lender = lender.clone();
            if let Some(live) = find_instance(sink.state, card_id).cloned()
                && !live.radiant
                && let Some(card) = find_instance_mut(sink.state, card_id)
            {
                card.radiant = true;
                sink.events.push(GameEvent::RadiantSet {
                    instance_id: card_id.clone(),
                    def_id: live.def_id.clone(),
                    zone: live.zone.clone(),
                });
            }
            continue;
        }
        if let Some(live) = find_instance(sink.state, card_id).cloned()
            && !live.radiant
            && let Some(card) = find_instance_mut(sink.state, card_id)
        {
            card.radiant = true;
            sink.events.push(GameEvent::RadiantSet {
                instance_id: card_id.clone(),
                def_id: live.def_id.clone(),
                zone: live.zone.clone(),
            });
            lends.push(crate::state::RadianceLend {
                player: *player,
                lender: lender.clone(),
                card: card_id.clone(),
            });
        }
    }
    sink.state.radiance_lends = if lends.is_empty() { None } else { Some(lends) };
}

/// R3558001: drop `card_id`'s lend, if it has one. A card made Radiant on its own keeps the face;
/// only the lend goes, so the sync never takes it back.
pub fn forget_lend(state: &mut GameState, card_id: &str) {
    if let Some(lends) = state.radiance_lends.as_mut() {
        lends.retain(|lend| lend.card != card_id);
        if lends.is_empty() {
            state.radiance_lends = None;
        }
    }
}

/// R3558002: wait for `meet`'s two cards to sit side by side in its player's hand.
pub fn add_meet(state: &mut GameState, meet: crate::state::HandMeet) {
    state.hand_meets.get_or_insert_with(Vec::new).push(meet);
}

/// R3558002: the other card of a waiting pair holding `id` — while both cards sit in the pair's
/// player's hand — or `None` when no pair waits on it.
pub fn meets_partner(state: &GameState, id: &str) -> Option<String> {
    for meet in state.hand_meets.iter().flatten() {
        let other = if meet.left == id {
            &meet.right
        } else if meet.right == id {
            &meet.left
        } else {
            continue;
        };
        let both_in_hand = [&meet.left, &meet.right].into_iter().all(|held| {
            crate::query::hand_index(state, held).is_some_and(|(player, _)| player == meet.player)
        });
        if both_in_hand {
            return Some(other.clone());
        }
    }
    None
}

/// R3558002: run every waiting pair in the order it was made. A pair with a card no longer in its
/// player's hand is dropped. A pair sitting side by side meets at once: the meet goes, and both
/// cards become Radiant, the leftmost first, at the meet's price when it names one.
pub fn meet_pairs(sink: &mut EngineSink<'_>) {
    let meets: Vec<crate::state::HandMeet> = sink.state.hand_meets.clone().unwrap_or_default();
    for meet in meets {
        let held: Vec<CardInstance> = [&meet.left, &meet.right]
            .into_iter()
            .filter_map(|id| find_instance(sink.state, id).cloned())
            .filter(|card| {
                crate::query::hand_index(sink.state, &card.id)
                    .is_some_and(|(player, _)| player == meet.player)
            })
            .collect();
        if held.len() != 2 {
            drop_meet(sink, &meet);
            continue;
        }
        let mut ordered = held;
        ordered.sort_by_key(|card| crate::query::hand_index(sink.state, &card.id).map(|(_, at)| at));
        let [left, right] = &ordered[..] else {
            continue;
        };
        let adjacent = crate::query::hand_index(sink.state, &left.id)
            .zip(crate::query::hand_index(sink.state, &right.id))
            .is_some_and(|((_, a), (_, b))| a.abs_diff(b) == 1);
        if !adjacent {
            continue;
        }
        drop_meet(sink, &meet);
        for card in [left, right] {
            let mut ctx = make_context(
                sink,
                Some(card),
                HookOptions {
                    controller: Some(card.controller),
                    ..Default::default()
                },
            );
            crate::effects::radiant::make_radiant(&mut ctx, card);
            if let Some(cost) = meet.cost {
                if let Some(live) = find_instance_mut(ctx.state, &card.id) {
                    live.cost_override = Some(cost.max(0));
                }
                let price = find_instance(ctx.state, &card.id)
                    .map(|live| crate::mana::effective_cost(ctx.state, live, Default::default()));
                if let Some(price) = price {
                    ctx.events.push(GameEvent::CostChanged {
                        instance_id: card.id.clone(),
                        cost: price,
                        hidden_from: None,
                    });
                }
            }
        }
    }
}

/// Drop one waiting pair, wherever its cards are now.
fn drop_meet(sink: &mut EngineSink<'_>, meet: &crate::state::HandMeet) {
    if let Some(meets) = sink.state.hand_meets.as_mut() {
        meets.retain(|kept| kept != meet);
        if meets.is_empty() {
            sink.state.hand_meets = None;
        }
    }
}
