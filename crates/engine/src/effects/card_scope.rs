//! How a verb names cards on the field, in a hand or in a deck (docs/classic-sets.md B3.3, B3.4, B5
//! E38, E39); `targets::BoardScope` names the field only. A card dormant under a Stack (§3.2, R13) is
//! in no scope. The walk is in R242's order: what everyone reads, then what only its owner reads (§9.1,
//! R33), then what nobody reads (§3), each side by side (the active player's first, R68), so a hidden
//! card's place among a verb's events says only its group.

use serde::{Deserialize, Serialize};

use crate::faces::card_type_of;
use crate::preview::is_face_down;
use crate::script::EffectContext;
use crate::state::{CardInstance, GameState};
use crate::wire::{CardType, PlayerId, Row, Tag, Zone, opponent_of};
use crate::zones::{active_units_of, card_at, is_standing_unit, slots_of};

/// The piles a card scope reaches: the field (both rows unless `rows` narrows it), a hand, a deck.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum CardZone {
    Field,
    Hand,
    Library,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
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
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
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
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct ScopedCard {
    pub card: CardInstance,
    pub zone: CardZone,
    pub readers: Readers,
    pub matches: bool,
}

/// `cards_in_card_scope`'s options (TS `{ wholeHiddenPiles?: boolean }`).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct CardScopeOptions {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub whole_hidden_piles: Option<bool>,
    /// MD-B1, R940: the walk's aim. A harmful walk (`Some(Harm)`) passes an immune card by when
    /// the scope names a tribal tag; any other aim reaches it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub aim: Option<crate::wire::TargetAim>,
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

/// R177: the players who may not read a card where it sits, which an event about a change there
/// records (`hiddenFrom`), so a view keeps it unread for good.
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

/// Whether a card passes a scope's filters (type, tags, self), wherever it is.
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
    // MD-B15, R923: a granted tag is part of the card for a scope too.
    let tags = crate::query::tags_of(ctx.state, card);
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
            // R446, R1081000, R1824000: the unit row is the side's Units, as `zones::acting_on` lists them.
            if *row == Row::Units {
                return active_units_of(state, player).into_iter().cloned().collect();
            }
            slots_of(player, *row)
                .into_iter()
                .filter_map(|slot| card_at(state, slot))
                // R1081000: a Unit standing in a backrow zone is no backrow card (§6.1 Animated).
                .filter(|card| !is_standing_unit(state, card))
                // R1839000: a Dormant card is in no card scope (`restrictions::unaffected_by`).
                .filter(|card| !crate::slumber::is_slumbering(card))
                .cloned()
                .collect::<Vec<CardInstance>>()
        })
        .collect()
}

/// Every card the scope reaches, in R242's order. `wholeHiddenPiles` also keeps the hidden cards its
/// filters reject (`matches: false`): a verb cueing each card of a hidden pile it changes must cue the
/// whole pile, or the count would say how many passed the filter (R440).
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
    for player in super::targets::sides_of(
        ctx,
        Some(scope.side.unwrap_or(super::targets::ScopeSide::SelfSide)),
    ) {
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
                let mut matches = matches_card_scope(ctx, &card, scope);
                // MD-B1, R940: a harmful card-scope walk passes an immune card by.
                if matches
                    && options.and_then(|options| options.aim) == Some(crate::wire::TargetAim::Harm)
                    && crate::restrictions::tribal_hate_cannot_reach(
                        state,
                        &card,
                        scope.tags.as_deref(),
                        scope.not_tags.as_deref(),
                    )
                {
                    matches = false;
                }
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
