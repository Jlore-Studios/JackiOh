//! The one reward hook (SPEC §14.8, #1302): a finished game's summary in, ledger rows out.
//!
//! Every payout goes through [`pay_game_rewards`], which takes a finished game's summary (the
//! account, the result and the actions taken), pays it, and moves the account's set tracks
//! (§14.7). Practice earning and quests call into this same hook: it owns no transaction of its
//! own, so each caller pays in the transaction that records its game, and a failure rolls the
//! game back with it. It is never called for bots: a ladder bot earns nothing ([[R1454]]).
//!
//! - On the ladder the server ran the match, so the caller counts the actions from its own log
//!   and pays in the transaction that records the result; the client reports nothing. The caller
//!   is `api::results`'s write, gated on a ranked non-series match there.
//! - Jlore is [`game_jlore`]: `JLORE_LADDER_WIN` for a win, `JLORE_LADDER_LOSS` otherwise.
//! - XP is [`game_xp`]: `XP_PER_ACTION` for each counted action, `XP_WIN_BONUS` more for a win,
//!   never above `XP_GAME_CAP`, so a loop of empty actions cannot farm it. It goes to every
//!   active track, each gaining all of it; the newest set's track is active from the start, and
//!   an older set's once it unlocks, one at a time, once the newest reaches
//!   `SET_TRACK_UNLOCK_LEVEL` ([[R1302003]]). A level pays its [`reward_for_level`](super::track_rewards::reward_for_level)
//!   once, as ledger rows, when its track reaches it.
//!
//! Every number is read through its [`crate::config`] constant (CLAUDE.md rule 9).

use jackioh_engine::PlayerId;
use jackioh_engine::wire::{SHIPPED_SETS, SetName, newest_shipped_set};

use super::math::level_for_xp;
use super::store::{EconomyReason, InventoryGrant, InventoryItem, JloreGrant, JloreReason, TrackGrant};
use super::track_rewards::reward_for_level;
use crate::config::{
    JLORE_LADDER_LOSS, JLORE_LADDER_WIN, SET_TRACK_FIRST_LEVEL, SET_TRACK_UNLOCK_LEVEL, XP_GAME_CAP,
    XP_PER_ACTION, XP_WIN_BONUS,
};
use crate::db::store::{StoreResult, Tx};

/// A finished game's summary, as the reward hook takes it (SPEC §14.8).
pub struct GameSummary {
    /// Whose account is paid.
    pub profile_id: String,
    /// Which seat they played.
    pub player: PlayerId,
    /// `Some(true)` for a win, `Some(false)` for a loss, `None` for a draw ([[R1302000]]).
    pub won: Option<bool>,
    /// How many of the match log's actions carry their seat.
    pub actions: i64,
    /// The match, for the ledger rows' `reference`.
    pub match_id: String,
    /// When the game ended, for the ledger rows' `at`.
    pub at: i64,
    /// A bot's side: never paid ([[R1454]]).
    pub bot: bool,
}

/// A game's XP (SPEC §14.8): `XP_PER_ACTION` for each action taken, `XP_WIN_BONUS` more for a
/// win, the total never above `XP_GAME_CAP`.
pub fn game_xp(actions: i64, won: bool) -> i64 {
    let bonus = if won { i64::from(XP_WIN_BONUS) } else { 0 };
    i64::from(XP_PER_ACTION)
        .saturating_mul(actions)
        .saturating_add(bonus)
        .min(i64::from(XP_GAME_CAP))
}

/// A ladder game's Jlore (SPEC §14.8): `JLORE_LADDER_WIN` for a win, `JLORE_LADDER_LOSS` for a
/// loss and for a draw ([[R1302000]]).
pub fn game_jlore(won: Option<bool>) -> i32 {
    if won == Some(true) {
        JLORE_LADDER_WIN
    } else {
        JLORE_LADDER_LOSS
    }
}

/// Pays one finished game's rewards in the caller's transaction: its Jlore, its XP on every
/// active set track, each newly reached level's rewards, and at most one newly unlocked older
/// track ([[R1302003]]). A bot's side, and a game with no XP, writes no track row.
pub async fn pay_game_rewards(tx: &mut Tx<'_>, summary: &GameSummary) -> StoreResult<()> {
    if summary.bot {
        return Ok(());
    }
    let jlore = i64::from(game_jlore(summary.won));
    if jlore != 0 {
        tx.economy_jlore_grant(&JloreGrant {
            profile_id: summary.profile_id.clone(),
            delta: jlore,
            reason: JloreReason::Game,
            reference: Some(summary.match_id.clone()),
            dev: false,
            at: summary.at,
        })
        .await?;
    }
    let xp = game_xp(summary.actions, summary.won == Some(true));
    if xp <= 0 {
        return Ok(());
    }
    let tracks = tx.economy_tracks_get(&summary.profile_id).await?;
    let held = |set: SetName| {
        tracks
            .iter()
            .find(|track| track.set_id == set.as_str())
            .map(|track| (track.xp, track.level, track.rewards_claimed))
            .unwrap_or((0, SET_TRACK_FIRST_LEVEL, 0))
    };
    let newest = newest_shipped_set();
    // Every active track gains all of the game's XP: the newest set's, active from the start,
    // and every older shipped set that already has a row.
    let mut active: Vec<SetName> = vec![newest];
    active.extend(
        SHIPPED_SETS
            .iter()
            .filter(|set| **set != newest && tracks.iter().any(|track| track.set_id == set.as_str())),
    );
    for set in &active {
        let (old_xp, old_level, old_claimed) = held(*set);
        let new_level = i64::from(level_for_xp(old_xp + xp));
        tx.economy_tracks_grant(&TrackGrant {
            profile_id: summary.profile_id.clone(),
            set_id: set.as_str().to_string(),
            xp,
            levels: new_level - old_level,
            rewards: new_level - old_claimed,
            reason: EconomyReason::Track,
            reference: Some(summary.match_id.clone()),
            dev: false,
            at: summary.at,
        })
        .await?;
        pay_level_rewards(tx, summary, set, old_claimed, new_level).await?;
    }
    // One older track unlocks at a time, nearest-older first, once the newest track reaches
    // `SET_TRACK_UNLOCK_LEVEL`; it counts as active from this payout, carrying this game's XP.
    // The gate reads the newest track after this game's XP landed, so the payout that reaches
    // the unlock level is the one that unlocks.
    let newest_level = i64::from(level_for_xp(held(newest).0 + xp));
    if newest_level >= i64::from(SET_TRACK_UNLOCK_LEVEL)
        && let Some(set) = SHIPPED_SETS
            .iter()
            .rev()
            .skip_while(|set| **set != newest)
            .skip(1)
            .find(|set| !tracks.iter().any(|track| track.set_id == set.as_str()))
    {
        let new_level = i64::from(level_for_xp(xp));
        tx.economy_tracks_grant(&TrackGrant {
            profile_id: summary.profile_id.clone(),
            set_id: set.as_str().to_string(),
            xp,
            levels: new_level - SET_TRACK_FIRST_LEVEL,
            rewards: new_level,
            reason: EconomyReason::Track,
            reference: Some(summary.match_id.clone()),
            dev: false,
            at: summary.at,
        })
        .await?;
        pay_level_rewards(tx, summary, set, 0, new_level).await?;
    }
    Ok(())
}

/// Pays the levels `(claimed, level]` reached — each once, as ledger rows — of one track.
async fn pay_level_rewards(
    tx: &mut Tx<'_>,
    summary: &GameSummary,
    set: &SetName,
    claimed: i64,
    level: i64,
) -> StoreResult<()> {
    for reached in claimed + 1..=level {
        let reward = reward_for_level(reached);
        if reward.jlore != 0 {
            tx.economy_jlore_grant(&JloreGrant {
                profile_id: summary.profile_id.clone(),
                delta: reward.jlore,
                reason: JloreReason::Track,
                reference: Some(summary.match_id.clone()),
                dev: false,
                at: summary.at,
            })
            .await?;
        }
        if reward.set_packs != 0 {
            tx.economy_inventory_grant(&InventoryGrant {
                profile_id: summary.profile_id.clone(),
                item: InventoryItem::SetPack,
                key: set.as_str().to_string(),
                delta: reward.set_packs,
                reason: EconomyReason::Track,
                reference: Some(summary.match_id.clone()),
                dev: false,
                at: summary.at,
            })
            .await?;
        }
        if reward.card_tokens != 0 {
            tx.economy_inventory_grant(&InventoryGrant {
                profile_id: summary.profile_id.clone(),
                item: InventoryItem::CardToken,
                key: String::new(),
                delta: reward.card_tokens,
                reason: EconomyReason::Track,
                reference: Some(summary.match_id.clone()),
                dev: false,
                at: summary.at,
            })
            .await?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{XP_GAME_CAP, XP_PER_ACTION, XP_WIN_BONUS};

    #[test]
    fn xp_counts_actions_and_stops_at_the_cap() {
        assert_eq!(game_xp(10, false), 10 * i64::from(XP_PER_ACTION));
        assert_eq!(
            game_xp(10, true),
            10 * i64::from(XP_PER_ACTION) + i64::from(XP_WIN_BONUS)
        );
        assert_eq!(game_xp(i64::from(XP_GAME_CAP), true), i64::from(XP_GAME_CAP));
        assert_eq!(game_xp(i64::MAX / 2, true), i64::from(XP_GAME_CAP));
    }

    #[test]
    fn jlore_pays_the_win_rate_only_for_a_win() {
        assert_eq!(game_jlore(Some(true)), JLORE_LADDER_WIN);
        assert_eq!(game_jlore(Some(false)), JLORE_LADDER_LOSS);
        assert_eq!(game_jlore(None), JLORE_LADDER_LOSS);
    }

    #[test]
    fn the_win_bonus_and_the_cap_shape_the_payout() {
        // #1333 sets every value at the end; this only pins the shape the description states, as
        // behaviour: a win pays more than a loss even with no actions, and the cap binds.
        assert_eq!(game_xp(0, true), i64::from(XP_WIN_BONUS));
        assert_eq!(game_xp(0, false), 0);
        assert!(game_xp(0, true) > game_xp(0, false));
        assert_eq!(game_xp(i64::MAX, false), i64::from(XP_GAME_CAP));
        assert_eq!(game_xp(i64::MAX, true), i64::from(XP_GAME_CAP));
    }

    #[test]
    fn game_summary_carries_the_seat_the_caller_counted() {
        // The hook's input shape, as the parallel practice and quest callers construct it: whose
        // account, which seat's actions were counted, the result, and what to stamp the rows with.
        let summary = GameSummary {
            profile_id: "profile-a".to_string(),
            player: PlayerId::P1,
            won: Some(true),
            actions: 10,
            match_id: "match-1".to_string(),
            at: 42,
            bot: false,
        };
        assert_eq!(summary.player, PlayerId::P1);
        assert_eq!(summary.won, Some(true));
        assert_eq!(game_jlore(summary.won), JLORE_LADDER_WIN);
        assert_eq!(
            game_xp(summary.actions, summary.won == Some(true)),
            10 * i64::from(XP_PER_ACTION) + i64::from(XP_WIN_BONUS)
        );
    }
}
