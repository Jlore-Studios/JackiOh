//! Card proficiency (SPEC §14.14, #3013): how often an account has played each card, and the level
//! those plays reached, each level unlocking one stamp the card may wear (R3013001).
//!
//! [[R3013000]]: the server counts a seat's plays from its own fold of the log, as it counts the
//! actions XP pays for ([[R1502000]]); the client reports nothing. A play is a card of the seat's own
//! deck, dealt, played from its hand: a Unit summoned, a Spell cast or a Trap set, its base and Radiant
//! faces alike (`SeatSummary::played`). A Created card (R2089000, `created_played`) and a token count
//! nothing. Only a game the server ran and pays for counts: ladder, unranked and Conquest. Practice,
//! played against the AI in the browser, and co-op, whose log the server does not fold, count nothing,
//! nor does a bot's side ([[R1454]]). [`add_plays`] runs inside `pay_game_rewards`, in the transaction
//! that records the game, so a failure rolls both back. Levels follow `PROFICIENCY_LEVEL_PLAYS` and
//! never fall.

use std::collections::BTreeMap;

use jackioh_engine::wire::{CardDefs, SeatSummary};

use super::rewards::GameMode;
use super::store::CardProficiency;
use crate::config::{PLAYER_SETTINGS_TEXT_MAX_LENGTH, PROFICIENCY_LEVEL_PLAYS, PROFICIENCY_LEVELS};
use crate::cosmetics::is_slug;
use crate::db::store::{StoreResult, Tx};

/// The top proficiency level, `PROFICIENCY_LEVELS`, as the store's `level` holds it.
pub fn top_level() -> i64 {
    i64::try_from(PROFICIENCY_LEVELS).unwrap_or(i64::MAX)
}

/// The plays that reach the top level, the curve's last threshold.
pub fn top_level_plays() -> i64 {
    PROFICIENCY_LEVEL_PLAYS.last().copied().unwrap_or(0)
}

/// [[R3013000]]: the level `plays` reach: how many of `PROFICIENCY_LEVEL_PLAYS` they meet.
pub fn level_for_plays(plays: i64) -> i64 {
    let met = PROFICIENCY_LEVEL_PLAYS
        .iter()
        .filter(|threshold| **threshold <= plays)
        .count();
    i64::try_from(met).unwrap_or(i64::MAX)
}

/// [[R3013000]]: whether a game of `mode` counts its plays: one the server ran and pays for.
pub fn counts_proficiency(mode: GameMode) -> bool {
    matches!(mode, GameMode::Ladder | GameMode::Unranked | GameMode::Conquest)
}

/// [[R3013000]]: the cards `seat` played from its hand that count toward its account's proficiency,
/// one id per play: its dealt plays (`played`, never `created_played`) of cards its own deck holds,
/// no token and no id a store row could not take (so a result never fails on one). Empty without a
/// fold of the game.
pub fn proficiency_plays(seat: Option<&SeatSummary>, defs: &CardDefs) -> Vec<String> {
    let Some(seat) = seat else {
        return Vec::new();
    };
    seat.played
        .iter()
        .filter(|id| {
            seat.deck.contains(*id)
                && defs.get(id.as_str()).is_none_or(|def| !def.is_token())
                && is_slug(id)
                && id.len() <= PLAYER_SETTINGS_TEXT_MAX_LENGTH
        })
        .cloned()
        .collect()
}

/// [[R3013000]]: adds one play per id of `plays` to the account's proficiency, in the caller's
/// transaction, each card's level raised to what its plays reach and never lowered.
pub async fn add_plays(tx: &mut Tx<'_>, profile_id: &str, plays: &[String], at: i64) -> StoreResult<()> {
    if plays.is_empty() {
        return Ok(());
    }
    let mut counts: BTreeMap<&str, i64> = BTreeMap::new();
    for id in plays {
        *counts.entry(id.as_str()).or_default() += 1;
    }
    let held = tx.economy_card_proficiency_get(profile_id).await?;
    let rows: Vec<CardProficiency> = counts
        .into_iter()
        .map(|(card_id, count)| {
            let before = held.iter().find(|row| row.card_id == card_id);
            let plays = before.map_or(0, |row| row.plays).saturating_add(count);
            CardProficiency {
                card_id: card_id.to_string(),
                plays,
                level: level_for_plays(plays).max(before.map_or(0, |row| row.level)),
                stamp: before.and_then(|row| row.stamp),
            }
        })
        .collect();
    tx.economy_card_proficiency_put(profile_id, &rows, at).await
}

#[cfg(test)]
mod tests {
    use super::*;

    use jackioh_engine::wire::CreatedUse;

    #[test]
    fn r3013000_the_curve_rises_from_a_quick_first_level() {
        let curve = PROFICIENCY_LEVEL_PLAYS;
        assert_eq!(curve.len(), PROFICIENCY_LEVELS);
        assert!(curve.first().is_some_and(|first| *first >= 1), "level 0 needs no play");
        assert!(
            curve.windows(2).all(|pair| pair[0] < pair[1]),
            "each level takes more plays than the one before"
        );
        assert_eq!(usize::try_from(top_level()).ok(), Some(PROFICIENCY_LEVELS));
        assert_eq!(Some(&top_level_plays()), curve.last());
    }

    #[test]
    fn r3013000_level_for_plays_steps_at_each_threshold() {
        assert_eq!(level_for_plays(0), 0);
        for (below, threshold) in PROFICIENCY_LEVEL_PLAYS.iter().enumerate() {
            let below = i64::try_from(below).expect("a few levels");
            assert_eq!(level_for_plays(threshold - 1), below, "one play short of {threshold}");
            assert_eq!(level_for_plays(*threshold), below + 1, "at {threshold}");
        }
        assert_eq!(level_for_plays(i64::MAX), top_level());
    }

    #[test]
    fn r3013000_only_dealt_cards_of_the_seats_deck_played_from_hand_count() {
        let defs = &*jackioh_cards::CATALOG;
        let token = defs
            .values()
            .find(|def| def.is_token())
            .map(|def| def.id.clone())
            .expect("the catalog ships a token");
        let x = "core-008".to_string();
        let y = "core-009".to_string();
        let seat = SeatSummary {
            deck: vec![x.clone(), x.clone(), token.clone()],
            // Y was never in the deck (a card taken from the other seat's hand), and a token in
            // `played` would be one no deck may hold; X's middle play was Radiant.
            played: vec![x.clone(), y, x.clone(), token, x.clone()],
            played_radiant: Some(vec![false, false, true, false, false]),
            created_played: Some(vec![CreatedUse {
                card: x.clone(),
                turn: 3,
                radiant: false,
            }]),
            ..SeatSummary::default()
        };
        assert_eq!(
            proficiency_plays(Some(&seat), defs),
            vec![x.clone(), x.clone(), x]
        );
        assert!(proficiency_plays(None, defs).is_empty());
    }

    #[test]
    fn r3013000_practice_and_coop_count_no_plays() {
        assert!(counts_proficiency(GameMode::Ladder));
        assert!(counts_proficiency(GameMode::Unranked));
        assert!(counts_proficiency(GameMode::Conquest));
        assert!(!counts_proficiency(GameMode::Practice));
        assert!(!counts_proficiency(GameMode::Coop));
    }
}
