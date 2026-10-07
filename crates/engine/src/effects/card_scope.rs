//! How a verb names cards anywhere a player keeps them — on the field, in a hand, in a deck — which
//! patch v0.2.0's instance-data verbs need (docs/classic-sets.md B3.3, B3.4, B5 E38, E39): Brittle
//! given to a hand, "Upgrade every card in your hand and deck twice", "your Units on the field, in your
//! hand and in your deck get +2X Attack and Rush", "Degrade 4 random cards in your opponent's deck".
//!
//! `targets::BoardScope` names cards on the field only, so this is the wider vocabulary, and each verb
//! that takes it walks `cards_in_card_scope`. A card dormant under a Stack is not on the field (§3.2,
//! R13) and is never in a scope.
//!
//! The walk is in R242's order: the cards everyone reads first (a unit, a face-up backrow card), then
//! the cards only their owner reads (a hand, a face-down trap: §9.1, R33), then the cards nobody reads
//! (a deck, §3) — each group side by side (the active player's first, R68), and within a side in the
//! zones' own order (units by lane, backrow by lane, hand order, deck top down). A verb that reports
//! its changes one card at a time reports them in this order, so a hidden card's place among the
//! events says only which group it was in (R242), never which zone.
//!
//! Port of `packages/engine/src/effects/cardScope.ts`.

use serde::{Deserialize, Serialize};

use crate::catalog::def_of;
use crate::faces::card_type_of;
use crate::preview::is_face_down;
use crate::script::EffectContext;
use crate::state::{CardInstance, GameState};
use crate::wire::{CardType, PlayerId, Row, Tag, Zone, opponent_of};
use crate::zones::{card_at, slots_of};

/// The piles a card scope reaches: the field (both rows unless `rows` narrows it), a hand, a deck.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
pub enum CardZone {
    Field,
    Hand,
    Library,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct CardScope {
    /// Sides, relative to `ctx.controller`. Default "self".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub side: Option<super::targets::ScopeSide>,
    /// Which piles. A hand and a deck are their owner's; the field is its controller's.
    pub zones: Vec<CardZone>,
    /// Field rows. Default both.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rows: Option<Vec<Row>>,
    /// Card types to keep, by the type the card has now (B2.7, `faces::card_type_of`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub types: Option<Vec<CardType>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<Tag>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub not_tags: Option<Vec<Tag>>,
    /// Leave out the card running the script.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exclude_self: Option<bool>,
}

/// R242: who may read a card where it sits.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
pub enum Readers {
    Everyone,
    Owner,
    Nobody,
}

/// R242's group order, public first.
const READER_ORDER: &[Readers] = &[Readers::Everyone, Readers::Owner, Readers::Nobody];

/// One card of a scope: where it is, who reads it there, and whether it passes the scope's filters.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ScopedCard {
    pub card: CardInstance,
    pub zone: CardZone,
    pub readers: Readers,
    pub matches: bool,
}

/// `cards_in_card_scope`'s options (TS `{ wholeHiddenPiles?: boolean }`).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct CardScopeOptions {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub whole_hidden_piles: Option<bool>,
}

/// R97, R33, §9.1: who may read a card where it sits now.
pub fn readers_of(state: &GameState, card: &CardInstance) -> Readers {
    match card.zone {
        Zone::Library { .. } => Readers::Nobody,
        Zone::Hand { .. } => Readers::Owner,
        Zone::Field {
            row: Row::Backrow, ..
        } if is_face_down(state, card) => Readers::Owner,
        _ => Readers::Everyone,
    }
}

/// R177: the players who may not read a card where it sits — both for a deck card, the other player for
/// a hand card or a face-down trap — which an event about a change made there records (`hiddenFrom`),
/// so a view keeps it unread for good.
pub fn unreadable_by(state: &GameState, card: &CardInstance) -> Vec<PlayerId> {
    let readers = readers_of(state, card);
    if readers == Readers::Nobody {
        return vec![PlayerId::P1, PlayerId::P2];
    }
    if readers == Readers::Everyone {
        return Vec::new();
    }
    let reader = match card.zone {
        Zone::Hand { player } => player,
        _ => card.controller,
    };
    vec![opponent_of(reader)]
}

/// Whether a card passes a scope's filters (type, tags, self). Where it is is the walk's business.
pub fn matches_card_scope(ctx: &EffectContext<'_>, card: &CardInstance, scope: &CardScope) -> bool {
    if scope.exclude_self == Some(true)
        && let Some(this) = &ctx.self_
        && card.id == this.id
    {
        return false;
    }
    if let Some(types) = &scope.types
        && !types.contains(&card_type_of(ctx.state, card))
    {
        return false;
    }
    let tags = &def_of(Some(&*ctx.state), &card.def_id).tags;
    if let Some(wanted) = &scope.tags
        && !wanted.iter().any(|tag| tags.contains(tag))
    {
        return false;
    }
    if let Some(unwanted) = &scope.not_tags
        && unwanted.iter().any(|tag| tags.contains(tag))
    {
        return false;
    }
    true
}

fn field_cards(state: &GameState, player: PlayerId, rows: &[Row]) -> Vec<CardInstance> {
    rows.iter()
        .flat_map(|row| {
            slots_of(player, *row)
                .into_iter()
                .filter_map(|slot| card_at(state, &slot).cloned())
                .collect::<Vec<CardInstance>>()
        })
        .collect()
}

/// Every card the scope reaches, in R242's order (this file's header). `wholeHiddenPiles` keeps every
/// card the scope reaches that someone may not read — a hand's, a deck's, a face-down trap — the ones
/// its filters reject marked `matches: false`: a verb that reports each card of a hidden pile it
/// changes must cue the whole pile, or the count of its cues would tell the other player how many
/// cards there passed the filter (R440). Otherwise the walk keeps the matching cards only.
pub fn cards_in_card_scope(
    ctx: &EffectContext<'_>,
    scope: &CardScope,
    options: Option<&CardScopeOptions>,
) -> Vec<ScopedCard> {
    let state: &GameState = &*ctx.state;
    let rows: Vec<Row> = scope
        .rows
        .clone()
        .unwrap_or_else(|| vec![Row::Units, Row::Backrow]);
    let whole_hidden_piles = options.and_then(|options| options.whole_hidden_piles) == Some(true);
    let mut out: Vec<ScopedCard> = Vec::new();
    for player in super::targets::sides_of(ctx, Some(scope.side.unwrap_or(super::targets::ScopeSide::SelfSide))) {
        let side = &state.players[player];
        let piles: [(CardZone, Vec<CardInstance>); 3] = [
            (
                CardZone::Field,
                if scope.zones.contains(&CardZone::Field) {
                    field_cards(state, player, &rows)
                } else {
                    Vec::new()
                },
            ),
            (
                CardZone::Hand,
                if scope.zones.contains(&CardZone::Hand) {
                    side.hand.clone()
                } else {
                    Vec::new()
                },
            ),
            (
                CardZone::Library,
                if scope.zones.contains(&CardZone::Library) {
                    side.library.clone()
                } else {
                    Vec::new()
                },
            ),
        ];
        for (zone, cards) in piles {
            for card in cards {
                let matches = matches_card_scope(ctx, &card, scope);
                let readers = readers_of(state, &card);
                if !matches && !(whole_hidden_piles && readers != Readers::Everyone) {
                    continue;
                }
                out.push(ScopedCard {
                    card,
                    zone,
                    readers,
                    matches,
                });
            }
        }
    }
    READER_ORDER
        .iter()
        .flat_map(|readers| out.iter().filter(|entry| entry.readers == *readers).cloned())
        .collect()
}
