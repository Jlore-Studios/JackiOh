//! Jlarna's credit line (R1223–R1225, Meditative #89): while a card with the `credit_line`
//! static flag acts on its controller's field, any mana they spend may go past the mana they have.
//! What they owe at once never passes the limit; everything borrowed during one turn is one debt,
//! split into instalments as evenly as possible, larger first; one instalment falls due at each of
//! the borrower's next refreshes, taken off what the refresh gives; an instalment that finds too
//! little mana takes what there is and the rest is forgiven (R1224). The debt outlives the lender,
//! and two or more lenders share one line. `spendable_mana` is the one affordability function every
//! refusal and listing reads (R1223).

use crate::state::{GameState, PlayerState};
use crate::wire::{PlayerId, Row};

/// What a credit line offers: the most its controller may owe at once, and how many instalments
/// one turn's debt is split into.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CreditTerms {
    pub limit: i32,
    pub instalments: i32,
}

/// R1223: the credit line acting on this player's field, if any. The limit is the largest declared
/// `credit` among the acting lenders; the instalments the most among them (never the first in slot
/// order). Read through `declared_or` so Degrade and Upgrade move the numbers.
pub fn credit_terms(state: &GameState, player: PlayerId) -> Option<CreditTerms> {
    let mut limit: Option<i32> = None;
    let mut instalments: Option<i32> = None;
    for row in [Row::Units, Row::Backrow] {
        for slot in crate::zones::slots_of(player, row) {
            let Some(card) = crate::zones::card_at(state, slot) else {
                continue;
            };
            if !crate::subsystems::activate::is_acting_on_field(state, card) {
                continue;
            }
            let flags = crate::scripts::flags_of(state, card);
            let Some(line) = flags.credit_line else {
                continue;
            };
            let line = crate::params::declared_or(state, card, "credit", line);
            limit = Some(limit.unwrap_or(line).max(line));
            let parts = crate::params::declared_or(
                state,
                card,
                "instalments",
                flags.credit_instalments.unwrap_or(1),
            )
            .max(1);
            instalments = Some(instalments.unwrap_or(parts).max(parts));
        }
    }
    match (limit, instalments) {
        (Some(limit), Some(instalments)) => Some(CreditTerms { limit, instalments }),
        _ => None,
    }
}

/// R1223: everything this player owes on coming refreshes — the sum of the schedule.
pub fn owed_mana_of(state: &GameState, player: PlayerId) -> i32 {
    state.players[player]
        .owed_instalments
        .as_ref()
        .map(|owed| owed.iter().sum())
        .unwrap_or(0)
}

/// R1223: how much more this player may borrow — the limit less what they owe, floored at 0, and 0
/// with no line acting.
pub fn credit_available(state: &GameState, player: PlayerId) -> i32 {
    match credit_terms(state, player) {
        Some(terms) => (terms.limit - owed_mana_of(state, player)).max(0),
        None => 0,
    }
}

/// R1223: the one affordability function — current mana plus what the credit line still offers. Every
/// refusal and listing reads this, so `legal_actions` and the reducer agree.
pub fn spendable_mana(state: &GameState, player: PlayerId) -> i32 {
    state.players[player].mana.current + credit_available(state, player)
}

/// R1223: one turn's debt split into `parts` instalments as evenly as possible, larger first: 4 over
/// 4 is 1, 1, 1, 1; 3 is 1, 1, 1, 0; 2 is 1, 1, 0, 0; 6 is 2, 2, 1, 1.
pub fn split_debt(total: i32, parts: i32) -> Vec<i32> {
    if parts <= 0 {
        return Vec::new();
    }
    let total = total.max(0);
    let base = total.div_euclid(parts);
    let extra = total.rem_euclid(parts);
    (0..parts)
        .map(|index| if index < extra { base + 1 } else { base })
        .collect()
}

/// Drop the schedule's trailing zeros; empty reads as `None`, so a game without credit hashes and
/// serializes as before.
fn trim(schedule: Vec<i32>) -> Option<Vec<i32>> {
    let mut schedule = schedule;
    while schedule.last() == Some(&0) {
        schedule.pop();
    }
    if schedule.is_empty() {
        None
    } else {
        Some(schedule)
    }
}

/// R1223: pay `amount` mana, borrowing the shortfall past current mana up to the credit limit.
/// Returns how much was borrowed. Without a line acting this is `spend_mana` (which floors at 0).
/// A turn's debt stays one debt: the old split is subtracted off the schedule and the new one added.
pub fn pay_mana(state: &mut GameState, player: PlayerId, amount: i32) -> i32 {
    let current = state.players[player].mana.current;
    if amount <= current {
        crate::mana::spend_mana(&mut state.players[player], amount);
        return 0;
    }
    let Some(terms) = credit_terms(state, player) else {
        crate::mana::spend_mana(&mut state.players[player], amount);
        return 0;
    };
    let borrow = (amount - current).min(credit_available(state, player)).max(0);
    state.players[player].mana.current = 0;
    if borrow == 0 {
        return 0;
    }
    let old_borrowed = state.players[player].turn_log.mana_borrowed.unwrap_or(0);
    let old_parts = state.players[player].turn_log.borrowed_parts.unwrap_or(0);
    let old_split = split_debt(old_borrowed, old_parts);
    let new_split = split_debt(old_borrowed + borrow, terms.instalments);
    let schedule = state.players[player].owed_instalments.clone().unwrap_or_default();
    let width = schedule.len().max(new_split.len());
    let mut next = Vec::with_capacity(width);
    for index in 0..width {
        let held = schedule.get(index).copied().unwrap_or(0);
        let old = old_split.get(index).copied().unwrap_or(0);
        let new = new_split.get(index).copied().unwrap_or(0);
        next.push(held - old + new);
    }
    state.players[player].owed_instalments = trim(next);
    state.players[player].turn_log.mana_borrowed = Some(old_borrowed + borrow);
    state.players[player].turn_log.borrowed_parts = Some(terms.instalments);
    borrow
}

/// R1224: take the next instalment off a refresh — what the refresh gives, less the instalment, never
/// below 0. An instalment that finds less mana takes what there is and the rest is forgiven, never
/// carried to a later turn. The instalment actually taken is the turn log's `mana_locked`.
pub fn take_instalment(side: &mut PlayerState) {
    let Some(mut schedule) = side.owed_instalments.clone() else {
        return;
    };
    if schedule.is_empty() {
        side.owed_instalments = None;
        return;
    }
    let due = schedule.remove(0).max(0);
    let taken = due.min(side.mana.current).max(0);
    side.mana.current -= taken;
    side.turn_log.mana_locked = if taken > 0 { Some(taken) } else { None };
    side.owed_instalments = trim(schedule);
}

/// R1225: whether a face carrying the end-of-turn Tribute for an unused line acts on this player's
/// field now (Jlarna's base face).
pub fn lapsing_face_acts(state: &GameState, player: PlayerId) -> bool {
    !credit_lapsing(state, player).is_empty()
}

/// R1225: the ids of the acting `credit_lapses` cards whose line went unused this turn — the Jlarnas
/// the end-of-turn check will Tribute. Empty when the player borrowed this turn.
pub fn credit_lapsing(state: &GameState, player: PlayerId) -> Vec<String> {
    if state.players[player].turn_log.mana_borrowed.unwrap_or(0) > 0 {
        return Vec::new();
    }
    let mut out = Vec::new();
    for row in [Row::Units, Row::Backrow] {
        for slot in crate::zones::slots_of(player, row) {
            let Some(card) = crate::zones::card_at(state, slot) else {
                continue;
            };
            if !crate::subsystems::activate::is_acting_on_field(state, card) {
                continue;
            }
            if crate::scripts::flags_of(state, card).credit_lapses == Some(true) {
                out.push(card.id.clone());
            }
        }
    }
    out
}
