//! The one reward hook (SPEC §14.8, #1302): a finished game's summary in, ledger rows out. The store's half.
//!
//! What a game pays, and every rule that decides it, is `jackioh_shared::economy::rewards` since #3104: the
//! game's Jlore at its mode's rate held to the daily cap (R1483000), its XP on every active set track
//! (§14.7), the levels it reaches and the quests it moves (§14.9, R1304001) are one decision,
//! `pay_game(records, summary, seed) -> outcome`, over what this module reads from the store, and a new
//! account's starting rewards (R1310000) another. This module reads those records under the account's
//! economy lock (R1301003), calls the decision and writes its grants in the caller's transaction, and decides
//! nothing. It owns no transaction: each caller pays in the transaction that records its game, so a failure
//! rolls the game back with it.
//!
//! - Where the server ran the match, it counts the actions from its own log; the client reports
//!   nothing. `api::results`'s write pays every 1v1 match it records (ladder, unranked and Conquest),
//!   and `duos::finish_match` a co-op game, counting each person's own moves ([[R1502000]]).
//! - A ladder, unranked or Conquest game also counts the cards each seat played toward its proficiency
//!   (§14.14, `proficiency::add_plays`, [[R3013000]]).
//!
//! Nothing here checks `ECONOMY_LIVE` (§14.12): a caller pays only while it is on (the two above, and
//! `api::codes`'s redemption for the starting rewards).

pub use jackioh_shared::economy::rewards::{
    GameEconomy, GameMode, GamePayout, GameRecords, GameSummary, LevelReached, QuestPaid,
    STARTING_REWARDS_REFERENCE, TrackGain, TrackPoint, game_jlore, game_xp, is_player_action, track_level,
    utc_day_bounds,
};

use jackioh_shared::economy::rewards::{pay_game, starting_rewards};

use super::grants::{Writing, store_refusal, write_grants};
use super::proficiency::{add_plays, counts_proficiency};
use super::quests::fresh_seed;
use crate::db::store::{StoreResult, Tx};

/// Pays one finished game's rewards in the caller's transaction: its Jlore, held to the account's
/// daily cap ([[R1483000]]), its XP on every active set track, and then moves its quests, which may pay
/// too ([[R1304001]]). A bot's side writes nothing. Answers what it paid, each track's gain read
/// off the tracks before and after the whole payout, so a completed quest's XP is in it.
pub async fn pay_game_rewards(tx: &mut Tx<'_>, summary: &GameSummary) -> StoreResult<GameEconomy> {
    if summary.bot {
        return Ok(GameEconomy::default());
    }
    // [[R1301003]]: one account's payouts run one after the other, as its openings do: the
    // account's economy lock, taken before any track or quest is read.
    tx.economy_lock_account(&summary.profile_id).await?;
    let (from, until) = utc_day_bounds(summary.at);
    let records = GameRecords {
        paid_today: tx
            .economy_game_jlore_totals(&summary.profile_id, from, until, &summary.match_id)
            .await?
            .day,
        tracks: tx.economy_tracks_get(&summary.profile_id).await?,
        quests: tx.economy_quests_get(&summary.profile_id).await?,
    };
    let payout = pay_game(&records, summary, &fresh_seed());
    let writing = Writing {
        profile_id: &summary.profile_id,
        reference: &None,
        dev: false,
        at: summary.at,
    };
    write_grants(tx, &writing, &payout.grants, store_refusal).await?;
    // [[R3013000]]: the game's plays, in this transaction, so they go with the game's record.
    if counts_proficiency(summary.mode) {
        add_plays(tx, &summary.profile_id, &summary.proficiency, summary.at).await?;
    }
    Ok(payout.economy)
}

/// A new account's tracks' first rewards (§14.12, [[R1310000]]), in the caller's transaction: the
/// newest set's track, active from the start, is marked paid through its first level and that level's
/// reward is paid, as a payout reaching it would. A track already paid through its first level writes
/// nothing, so it pays once.
pub async fn pay_starting_rewards(tx: &mut Tx<'_>, profile_id: &str, at: i64) -> StoreResult<()> {
    // [[R1301003]]: the account's economy lock, as every payout takes.
    tx.economy_lock_account(profile_id).await?;
    let grants = starting_rewards(&tx.economy_tracks_get(profile_id).await?);
    let writing = Writing {
        profile_id,
        reference: &None,
        dev: false,
        at,
    };
    write_grants(tx, &writing, &grants, store_refusal).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{XP_LADDER_TARGET_MEAN, XP_PRACTICE_SCALE};
    use jackioh_engine::{Action, GameResult, PlayerId, Winner};

    /// R1333001's measured games: 40 of this build's AI playing itself, one JSON line each
    /// (`cargo jackioh selfplay --games 40 --seed xp-1333`).
    const MEASURED_GAMES: &str = include_str!("../../tests/fixtures/xp-selfplay.jsonl");
    /// How far a measured mean XP may land from its target, as a share of the target (R1333001).
    const XP_MEAN_TOLERANCE: f64 = 0.05;
    /// The seats of R1333001's games: two to each of the 40.
    const MEASURED_SEATS: usize = 80;

    #[derive(serde::Deserialize)]
    struct MeasuredGame {
        log: Vec<Action>,
        result: Option<GameResult>,
    }

    #[test]
    fn r1333001_measured_games_average_the_ladder_target_and_practice_half_of_it() {
        // Each seat's actions as the reward hook counts them, and whether it won.
        let mut seats: Vec<(i64, bool)> = Vec::new();
        for line in MEASURED_GAMES.lines() {
            let game: MeasuredGame = serde_json::from_str(line).expect("a measured game");
            let result = game.result.expect("every measured game finished");
            for player in [PlayerId::P1, PlayerId::P2] {
                let actions = game
                    .log
                    .iter()
                    .filter(|action| action.player_id == player && is_player_action(&action.body))
                    .count();
                seats.push((
                    i64::try_from(actions).expect("a game's actions fit"),
                    result.winner == Winner::from(player),
                ));
            }
        }
        assert_eq!(seats.len(), MEASURED_SEATS);
        let target = f64::from(XP_LADDER_TARGET_MEAN);
        for (mode, target) in [
            (GameMode::Ladder, target),
            (GameMode::Practice, target * XP_PRACTICE_SCALE),
        ] {
            let total: i64 = seats
                .iter()
                .map(|(actions, won)| game_xp(mode, *actions, *won))
                .sum();
            let mean = total as f64 / seats.len() as f64;
            assert!(
                (mean - target).abs() <= target * XP_MEAN_TOLERANCE,
                "{mode:?} games average {mean} XP, not about {target}"
            );
        }
    }
}
