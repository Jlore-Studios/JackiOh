//! Playing cards from the graveyard (docs/classic-sets.md B5 E11, R454): the permissions that allow
//! it, the plays `legal_actions` offers under them, the refusal §10.5 step 1 gives, and the Plague
//! Token payment Classic #74 Corpse Plantation adds.
//!
//! A permission is a card on the field saying so — Classic #28 Second Wind ("You may play cards from
//! your graveyard"; its Radiant face only those whose price, as it would be paid, is (1) or more),
//! Classic #74 Corpse Plantation (Units, paid partly or wholly with the Plague Counters on it), and
//! Classic #90 In Too Deep's reward L (an aura of the same permission while the card stands). Each is
//! `Script.graveyard_play`, a pure read asked of the cards acting on the player's own side.
//!
//! Everything else is a play's: the card leaves the graveyard at §10.5 step 4 as a hand card leaves
//! the hand (`play_steps::take_from_play_source`), its choices are checked and offered as a hand card's
//! are (R81, R90), R65's player prices reach it (`mana::play_cost`), it counts as played and its Cry
//! fires (R1: "played", wherever from). A permission only decides whether the play may be made and how
//! it may be paid.
//!
//! Port of `packages/engine/src/graveyardPlay.ts`. `GraveyardPlayPermission` lives in `script.rs` (a
//! `Script` hook returns it) and `PlagueSpend` in the wire (`wire::PlagueSpend`, the `play` action's
//! `plague`); both are used from there, never defined again.

use serde::{Deserialize, Serialize};

use crate::config::{MIN_PLAGUE_PAYMENT, PLAGUE_TOKEN_MANA};
use crate::script::EngineSink;
use crate::script::{GraveyardPlayPermission, HookArgs};
use crate::state::{CardInstance, EngineError, GameState};
use crate::wire::{CardType, CounterKind, GameEvent, PlayerId, Row, Zone};

/// The Plague Counters a play from the graveyard spends: the play action's `plague` (part 1 put the
/// type in the wire, where the action names it).
pub use crate::wire::PlagueSpend;

/// A permission and the card on the field granting it.
#[derive(Clone, Debug, PartialEq)]
pub struct GraveyardGrant {
    pub source: CardInstance,
    pub permission: GraveyardPlayPermission,
}

/// How one play pays its price besides mana: the Plague Counters a graveyard play spends (R454).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash, Default)]
#[serde(rename_all = "camelCase")]
pub struct PlayPayment {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plague: Option<PlagueSpend>,
}

/// E11: every permission the player has now — from each card acting on their side of the field (the
/// top of a pile, a backrow card) whose running face grants one, in lane order, units first. A Vanilla
/// card grants nothing (`script_of`, R115); a card dormant under a Stack pile is not on the field for
/// effects (§3.2, R13).
pub fn graveyard_grants_of(state: &GameState, player: PlayerId) -> Vec<GraveyardGrant> {
    let mut out: Vec<GraveyardGrant> = Vec::new();
    for row in [Row::Units, Row::Backrow] {
        for slot in crate::zones::slots_of(player, row) {
            let Some(source) = crate::zones::card_at(state, slot) else {
                continue;
            };
            if source.controller != player {
                continue;
            }
            let script = crate::scripts::script_of(state, source);
            let Some(hook) = script.graveyard_play.as_ref() else {
                continue;
            };
            for permission in hook(HookArgs {
                state,
                self_: source,
                radiant: source.radiant,
            }) {
                out.push(GraveyardGrant {
                    source: source.clone(),
                    permission,
                });
            }
        }
    }
    out
}

/// Whether a card lies in this player's own graveyard, where E11's permissions reach ("your graveyard").
pub fn in_own_graveyard(state: &GameState, player: PlayerId, card: &CardInstance) -> bool {
    // By id: a price is often read off a copy of the card with a probe's X or embiggen (`play_choices`).
    matches!(card.zone, Zone::Graveyard { player: owner } if owner == player)
        && state.players[player]
            .graveyard
            .iter()
            .any(|held| held.id == card.id)
}

/// The grants that admit this card at all: the right type (Units only, or every type).
fn admitting(state: &GameState, player: PlayerId, card: &CardInstance) -> Vec<GraveyardGrant> {
    let unit = crate::faces::card_type_of(state, card) == CardType::Unit;
    graveyard_grants_of(state, player)
        .into_iter()
        .filter(|grant| grant.permission.units != Some(true) || unit)
        .collect()
}

/// R454, R65: whether a play may take this card from its player's graveyard now — the card lies there
/// and a permission admits its type. Such a card is priced as a play wherever it is read (R65: the
/// player's prices reach a card where a play takes it from), whatever price a permission then asks.
pub fn playable_from_graveyard(state: &GameState, card: &CardInstance) -> bool {
    let player = card.zone.player();
    in_own_graveyard(state, player, card) && !admitting(state, player, card).is_empty()
}

/// The Plague Counters on a card now (§6.3 Plague Counter).
fn plague_on(card: &CardInstance) -> i32 {
    card.counters.plague.unwrap_or(0)
}

/// R454: every way a play of this graveyard card at this price may be paid under the permissions — `{}`
/// in mana alone when a permission without `plague` admits the price and the mana covers it; and, per
/// Plague permission that admits it, each number of tokens from MIN_PLAGUE_PAYMENT up to what the card
/// holds and the price allows, with the rest in mana. None when no permission admits the card, or the
/// card is not in its player's own graveyard. `play_choices::graveyard_play_actions_for` crosses these
/// with the play's choices.
pub fn graveyard_payments_for(
    state: &GameState,
    player: PlayerId,
    card: &CardInstance,
    price: i32,
) -> Vec<PlayPayment> {
    if !in_own_graveyard(state, player, card) {
        return Vec::new();
    }
    let grants = admitting(state, player, card);
    let mana = state.players[player].mana.current;
    let mut out: Vec<PlayPayment> = Vec::new();
    let priced: Vec<&GraveyardGrant> = grants
        .iter()
        .filter(|grant| price >= grant.permission.min_price.unwrap_or(0))
        .collect();
    if priced.iter().any(|grant| grant.permission.plague != Some(true)) && price <= mana {
        out.push(PlayPayment::default());
    }
    for grant in &priced {
        if grant.permission.plague != Some(true) {
            continue;
        }
        // Math.floor on non-negative integers is integer division (SURFACE §4.4.4); a negative price
        // admits no token, as the loop below would find.
        let most = plague_on(&grant.source).min(price.div_euclid(PLAGUE_TOKEN_MANA));
        let mut tokens = MIN_PLAGUE_PAYMENT;
        while tokens <= most {
            if price - tokens * PLAGUE_TOKEN_MANA <= mana {
                out.push(PlayPayment {
                    plague: Some(PlagueSpend {
                        from: grant.source.id.clone(),
                        tokens,
                    }),
                });
            }
            tokens += 1;
        }
    }
    out
}

/// E11, R454: why a play of this graveyard card at this price, paid as the action says, is refused —
/// the same permissions and payments `graveyard_play_actions_for` offers, asked from the other side
/// (§10.2), or `Ok` when a permission admits it. Mana is part of the answer, since a Plague payment
/// changes what the mana has to cover.
pub fn why_graveyard_play_refused(
    state: &GameState,
    player: PlayerId,
    card: &CardInstance,
    price: i32,
    plague: Option<&PlagueSpend>,
) -> Result<(), EngineError> {
    let grants = admitting(state, player, card);
    if grants.is_empty() {
        return Err(EngineError::new("you may not play that card from your graveyard"));
    }
    let priced: Vec<&GraveyardGrant> = grants
        .iter()
        .filter(|grant| price >= grant.permission.min_price.unwrap_or(0))
        .collect();
    if priced.is_empty() {
        // `grants` is not empty here, so the least is a number (SURFACE §4.4.4's Math.min guard).
        let least = grants
            .iter()
            .map(|grant| grant.permission.min_price.unwrap_or(0))
            .min()
            .unwrap_or(0);
        return Err(EngineError::new(format!(
            "a card played from your graveyard must cost ({least}) or more"
        )));
    }
    let mana = state.players[player].mana.current;

    let Some(plague) = plague else {
        if !priced.iter().any(|grant| grant.permission.plague != Some(true)) {
            return Err(EngineError::new(
                "that card may only be played from your graveyard by spending Plague Counters",
            ));
        }
        if price > mana {
            return Err(EngineError::new(format!(
                "that card costs {price}, more than your mana"
            )));
        }
        return Ok(());
    };

    let Some(grant) = priced
        .iter()
        .find(|entry| entry.permission.plague == Some(true) && entry.source.id == plague.from)
    else {
        return Err(EngineError::new("those Plague Counters cannot pay for that card"));
    };
    // TS also refused a token count that is not a whole number; an `i32` always is.
    if plague.tokens < MIN_PLAGUE_PAYMENT {
        return Err(EngineError::new(format!(
            "spend at least {MIN_PLAGUE_PAYMENT} Plague Counter"
        )));
    }
    if plague.tokens > plague_on(&grant.source) {
        return Err(EngineError::new("there are not that many Plague Counters there"));
    }
    if plague.tokens * PLAGUE_TOKEN_MANA > price {
        return Err(EngineError::new("that is more Plague Counters than the price"));
    }
    if price - plague.tokens * PLAGUE_TOKEN_MANA > mana {
        return Err(EngineError::new("the rest of the price is more than your mana"));
    }
    Ok(())
}

/// R454: the mana a play pays once its Plague Counters have paid their part.
pub fn mana_due(price: i32, plague: Option<&PlagueSpend>) -> i32 {
    (price - plague.map_or(0, |spend| spend.tokens) * PLAGUE_TOKEN_MANA).max(0)
}

/// R454, §10.5 step 2: take the Plague Counters a play spends off the card that holds them, reported as
/// any change of the count is (`counterChanged`, no `placed`: a removal, E19).
///
/// TS wrote through the live card it was handed; here the card is found in the state by id and its
/// counters changed there.
pub fn spend_plague_tokens(sink: &mut EngineSink, card: &CardInstance, tokens: i32) {
    let current =
        crate::state::find_instance(sink.state, &card.id).map_or_else(|| plague_on(card), plague_on);
    let next = (current - tokens.max(0)).max(0);
    if next == current {
        return;
    }
    if let Some(live) = crate::state::find_instance_mut(sink.state, &card.id) {
        live.counters.plague = if next == 0 { None } else { Some(next) };
    }
    sink.events.push(GameEvent::CounterChanged {
        instance_id: card.id.clone(),
        counter: CounterKind::Plague,
        value: next,
        placed: None,
    });
}
