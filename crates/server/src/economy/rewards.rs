//! The one reward hook (SPEC §14.8, #1302): a finished game's summary in, ledger rows out.
//!
//! Every payout goes through [`pay_game_rewards`], which takes a finished game's summary (the
//! account, the result, the actions taken and what the game did), pays it, and moves the account's
//! set tracks (§14.7) and its quests (§14.9, `quests::move_quests`, [[R1304001]]). Practice
//! earning calls into this same hook: it owns no transaction of its own, so each caller pays in
//! the transaction that records its game, and a failure rolls the game back with it. A bot's side
//! is paid nothing and moves no quest: a ladder bot earns nothing ([[R1454]]).
//!
//! - Where the server ran the match, it counts the actions from its own log and pays in the
//!   transaction that records the result; the client reports nothing. `api::results`'s write pays
//!   every 1v1 match it records (ladder, unranked and Conquest games), and `duos::finish_match`
//!   pays a co-op game, counting each person's own moves ([[R1502000]]). An action counts when
//!   [`is_player_action`] says the player took it ([[R1302004]]): every caller counts with it.
//! - The summary names its [`GameMode`]. Jlore is [`game_jlore`]: on the ladder `JLORE_LADDER_WIN`
//!   for a win and `JLORE_LADDER_LOSS` otherwise, in practice `JLORE_PRACTICE_WIN` and
//!   `JLORE_PRACTICE_LOSS`, and for an unranked game, a game of a Conquest series and a co-op game
//!   `JLORE_UNRANKED_*`, `JLORE_CONQUEST_*` and `JLORE_COOP_*` the same way ([[R1502000]]).
//! - A day's games pay an account at most `JLORE_GAMES_DAILY_CAP` Jlore, every mode's together,
//!   counted over the UTC day the game ended in ([[R1483000]]). The game that crosses
//!   the cap is paid the part that fits and the ones after it no Jlore, with no Jlore row; XP and
//!   quests are paid in full. Quest and track Jlore neither count nor stop. [`GameRewards`] says
//!   what was paid and whether the cap held some back.
//! - XP is [`game_xp`]: `XP_PER_ACTION` for each counted action, `XP_WIN_BONUS` more for a win,
//!   never above `XP_GAME_CAP`, so a loop of empty actions cannot farm it; a practice game earns
//!   that times `XP_PRACTICE_SCALE`, rounded down, and a co-op game that times `XP_COOP_SCALE`
//!   ([[R1502000]]). It goes to every
//!   active track, each gaining all of it; the newest set's track is active from the start, and
//!   an older set's once it unlocks, one at a time, once the newest reaches
//!   `SET_TRACK_UNLOCK_LEVEL` ([[R1302003]]). A level pays its [`reward_for_level`](super::track_rewards::reward_for_level)
//!   once, as ledger rows, when its track reaches it; a track's last level is
//!   `SET_TRACK_LEVELS`, and XP past it lands but reaches no level ([`track_level`], [[R1302005]]).
//!   [`pay_xp`] is that payout, and a completed quest's XP goes through it too.
//!
//! - A new account starts with its tracks' first rewards (§14.12, [[R1310000]]):
//!   [`pay_starting_rewards`] pays the newest set's track its first level when the account's
//!   redemption makes it active, the only track active from the start; an older set's pays its
//!   first level when it unlocks, as above.
//!
//! Nothing here checks `ECONOMY_LIVE` (§14.12): a caller pays only while it is on, so a build
//! behind the switch changes nothing a player owns. `api::results` is that caller for a 1v1 match,
//! `duos::finish_match` for a co-op game, and `api::codes`'s redemption for the starting rewards.
//!
//! Every number is read through its [`crate::config`] constant (CLAUDE.md rule 9).

use jackioh_engine::wire::{SHIPPED_SETS, SetName, newest_shipped_set};
use jackioh_engine::{ActionBody, PlayerId};

use super::math::level_for_xp;
use super::quests::{GameTally, move_quests};
use super::store::{EconomyReason, InventoryGrant, InventoryItem, JloreGrant, JloreReason, TrackGrant};
use super::track_rewards::reward_for_level;
use crate::config::{
    JLORE_CONQUEST_LOSS, JLORE_CONQUEST_WIN, JLORE_COOP_LOSS, JLORE_COOP_WIN, JLORE_GAMES_DAILY_CAP,
    JLORE_LADDER_LOSS, JLORE_LADDER_WIN, JLORE_PRACTICE_LOSS, JLORE_PRACTICE_WIN, JLORE_UNRANKED_LOSS,
    JLORE_UNRANKED_WIN, SET_TRACK_FIRST_LEVEL, SET_TRACK_LEVELS, SET_TRACK_UNLOCK_LEVEL, XP_COOP_SCALE,
    XP_GAME_CAP, XP_PER_ACTION, XP_PRACTICE_SCALE, XP_WIN_BONUS,
};
use crate::db::store::{StoreResult, Tx};

/// Milliseconds in a UTC day, the span `JLORE_GAMES_DAILY_CAP` counts over ([[R1483000]]).
const MS_PER_UTC_DAY: i64 = 86_400_000;

/// Which kind of game finished, so the hook pays its rates (SPEC §14.8).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GameMode {
    /// A ranked match that is not a game of a Conquest series.
    Ladder,
    /// A game against the AI in the browser, its log checked by the server (§9.9).
    Practice,
    /// A 1v1 match the queue did not pair that is no game of a Conquest series: a room challenge
    /// and its rematches (R604, [[R1502000]]).
    Unranked,
    /// A game of a Conquest series, ranked or not, paid as its own result ([[R1502000]]).
    Conquest,
    /// A Duos co-op game, each person paid for their own seat ([[R1502000]]).
    Coop,
}

/// A finished game's summary, as the reward hook takes it (SPEC §14.8).
pub struct GameSummary {
    /// The kind of game, which sets the rates.
    pub mode: GameMode,
    /// Whose account is paid.
    pub profile_id: String,
    /// Which seat they played.
    pub player: PlayerId,
    /// `Some(true)` for a win, `Some(false)` for a loss, `None` for a draw ([[R1302000]]).
    pub won: Option<bool>,
    /// How many actions the player took: the log's rows of their seat that
    /// [`is_player_action`] counts.
    pub actions: i64,
    /// The match, for the ledger rows' `reference`.
    pub match_id: String,
    /// When the game ended, for the ledger rows' `at`.
    pub at: i64,
    /// A bot's side: never paid ([[R1454]]).
    pub bot: bool,
    /// What the seat did, as the quests count it (§14.9).
    pub tally: GameTally,
}

/// What [`pay_game_rewards`] paid one game ([[R1483000]]): all zero for a bot's side.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GameRewards {
    /// The Jlore written: the game's rate, or what was left under the daily cap.
    pub jlore: i64,
    pub xp: i64,
    /// Whether the daily cap held part or all of the game's Jlore back.
    pub jlore_capped: bool,
}

/// One XP payout onto every active set track (§14.7): a game's or a completed quest's.
pub struct XpPayout<'a> {
    pub profile_id: &'a str,
    pub xp: i64,
    /// The track rows' reason; the level rewards it reaches are `Track`'s.
    pub reason: EconomyReason,
    /// What every row it writes names: the match, or the quest.
    pub reference: &'a str,
    /// A dev tool's payout (§14.10).
    pub dev: bool,
    pub at: i64,
}

/// Whether a logged action is one the player took, so earns `XP_PER_ACTION` (SPEC §14.8,
/// [[R1302004]]): every move of the game, an emote a card hears (MD-D29) among them. The server's
/// own rows are not ([[R79]]: a clock that ran out, a disconnect that expired, the hard ceiling),
/// nor [[R345]]'s automatic-end setting, nor a practice sandbox's tool ([[R923000]]). Every kind is
/// named, so a new one is counted or not by a decision, never by default.
pub fn is_player_action(body: &ActionBody) -> bool {
    match body {
        ActionBody::Mulligan { .. }
        | ActionBody::Play { .. }
        | ActionBody::Attack { .. }
        | ActionBody::SwitchPosition { .. }
        | ActionBody::Activate { .. }
        | ActionBody::ActivatePower { .. }
        | ActionBody::Answer { .. }
        | ActionBody::OfferDraw
        | ActionBody::AnswerDraw { .. }
        | ActionBody::Emote { .. }
        | ActionBody::Concede
        | ActionBody::EndTurn => true,
        ActionBody::SetAutoEndTurn { .. }
        | ActionBody::Sandbox { .. }
        | ActionBody::Timeout
        | ActionBody::DisconnectExpired { .. }
        | ActionBody::CeilingReached => false,
    }
}

/// The level a set track with `total_xp` XP stands on (SPEC §14.7, [[R1302005]]): the level
/// curve's, never past the track's last level, `SET_TRACK_LEVELS`.
pub fn track_level(total_xp: i64) -> i64 {
    i64::from(level_for_xp(total_xp).min(SET_TRACK_LEVELS))
}

/// A game's XP (SPEC §14.8): `XP_PER_ACTION` for each action taken, `XP_WIN_BONUS` more for a
/// win, the total never above `XP_GAME_CAP`; a practice game earns that times `XP_PRACTICE_SCALE`,
/// and a co-op game that times `XP_COOP_SCALE`, each rounded down ([[R1502000]]); an unranked game
/// and a game of a Conquest series earn what the ladder does.
pub fn game_xp(mode: GameMode, actions: i64, won: bool) -> i64 {
    let bonus = if won { i64::from(XP_WIN_BONUS) } else { 0 };
    let ladder = i64::from(XP_PER_ACTION)
        .saturating_mul(actions)
        .saturating_add(bonus)
        .min(i64::from(XP_GAME_CAP));
    match mode {
        GameMode::Ladder | GameMode::Unranked | GameMode::Conquest => ladder,
        GameMode::Practice => (ladder as f64 * XP_PRACTICE_SCALE).floor() as i64,
        GameMode::Coop => (ladder as f64 * XP_COOP_SCALE).floor() as i64,
    }
}

/// A game's Jlore (SPEC §14.8): the mode's win rate for a win, its loss rate for a loss and for
/// a draw ([[R1302000]]).
pub fn game_jlore(mode: GameMode, won: Option<bool>) -> i32 {
    match (mode, won == Some(true)) {
        (GameMode::Ladder, true) => JLORE_LADDER_WIN,
        (GameMode::Ladder, false) => JLORE_LADDER_LOSS,
        (GameMode::Practice, true) => JLORE_PRACTICE_WIN,
        (GameMode::Practice, false) => JLORE_PRACTICE_LOSS,
        (GameMode::Unranked, true) => JLORE_UNRANKED_WIN,
        (GameMode::Unranked, false) => JLORE_UNRANKED_LOSS,
        (GameMode::Conquest, true) => JLORE_CONQUEST_WIN,
        (GameMode::Conquest, false) => JLORE_CONQUEST_LOSS,
        (GameMode::Coop, true) => JLORE_COOP_WIN,
        (GameMode::Coop, false) => JLORE_COOP_LOSS,
    }
}

/// The UTC day `at` (milliseconds) falls in, as `[start, end)` ([[R1483000]]).
pub fn utc_day_bounds(at: i64) -> (i64, i64) {
    let start = at.div_euclid(MS_PER_UTC_DAY) * MS_PER_UTC_DAY;
    (start, start.saturating_add(MS_PER_UTC_DAY))
}

/// Pays one finished game's rewards in the caller's transaction: its Jlore, held to the account's
/// daily cap ([[R1483000]]), its XP on every active set track ([`pay_xp`]), and then moves its
/// quests, which may pay too ([[R1304001]]). A bot's side writes nothing.
pub async fn pay_game_rewards(tx: &mut Tx<'_>, summary: &GameSummary) -> StoreResult<GameRewards> {
    if summary.bot {
        return Ok(GameRewards::default());
    }
    // [[R1301003]]: one account's payouts run one after the other, as its openings do: the
    // account's economy lock, taken before any track or quest is read. A completed quest paying
    // through `pay_xp` in this same transaction is unaffected: the advisory lock is re-entrant.
    tx.economy_lock_account(&summary.profile_id).await?;
    let rate = i64::from(game_jlore(summary.mode, summary.won));
    let (from, until) = utc_day_bounds(summary.at);
    let paid_today = tx
        .economy_game_jlore_totals(&summary.profile_id, from, until, &summary.match_id)
        .await?
        .day;
    let jlore = rate.min((JLORE_GAMES_DAILY_CAP - paid_today).max(0));
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
    let xp = game_xp(summary.mode, summary.actions, summary.won == Some(true));
    pay_xp(
        tx,
        &XpPayout {
            profile_id: &summary.profile_id,
            xp,
            reason: EconomyReason::Track,
            reference: &summary.match_id,
            dev: false,
            at: summary.at,
        },
    )
    .await?;
    move_quests(tx, summary).await?;
    Ok(GameRewards {
        jlore,
        xp,
        jlore_capped: jlore < rate,
    })
}

/// What the starting rewards' rows name as their `reference` ([`pay_starting_rewards`]).
pub const STARTING_REWARDS_REFERENCE: &str = "signup";

/// A new account's tracks' first rewards (§14.12, [[R1310000]]), in the caller's transaction: the
/// newest set's track, active from the start, is marked paid through its first level
/// (`SET_TRACK_FIRST_LEVEL`) and that level's [`reward_for_level`] is paid, as a payout reaching
/// it would. A track already paid through its first level writes nothing, so it pays once.
pub async fn pay_starting_rewards(tx: &mut Tx<'_>, profile_id: &str, at: i64) -> StoreResult<()> {
    // [[R1301003]]: the account's economy lock, as every payout takes.
    tx.economy_lock_account(profile_id).await?;
    let newest = newest_shipped_set();
    let claimed = tx
        .economy_tracks_get(profile_id)
        .await?
        .into_iter()
        .find(|track| track.set_id == newest.as_str())
        .map_or(0, |track| track.rewards_claimed);
    if claimed >= SET_TRACK_FIRST_LEVEL {
        return Ok(());
    }
    tx.economy_tracks_grant(&TrackGrant {
        profile_id: profile_id.to_string(),
        set_id: newest.as_str().to_string(),
        xp: 0,
        levels: 0,
        rewards: SET_TRACK_FIRST_LEVEL - claimed,
        reason: EconomyReason::Track,
        reference: Some(STARTING_REWARDS_REFERENCE.to_string()),
        dev: false,
        at,
    })
    .await?;
    let payout = XpPayout {
        profile_id,
        xp: 0,
        reason: EconomyReason::Track,
        reference: STARTING_REWARDS_REFERENCE,
        dev: false,
        at,
    };
    pay_level_rewards(tx, &payout, &newest, claimed, SET_TRACK_FIRST_LEVEL).await
}

/// Pays `payout.xp` XP onto every active set track in the caller's transaction, each newly reached
/// level's rewards, and at most one newly unlocked older track ([[R1302003]]). No XP writes no
/// track row.
pub async fn pay_xp(tx: &mut Tx<'_>, payout: &XpPayout<'_>) -> StoreResult<()> {
    let xp = payout.xp;
    if xp <= 0 {
        return Ok(());
    }
    let tracks = tx.economy_tracks_get(payout.profile_id).await?;
    let held = |set: SetName| {
        tracks
            .iter()
            .find(|track| track.set_id == set.as_str())
            .map(|track| (track.xp, track.level, track.rewards_claimed))
            .unwrap_or((0, SET_TRACK_FIRST_LEVEL, 0))
    };
    let newest = newest_shipped_set();
    // Every active track gains all of the payout's XP: the newest set's, active from the start,
    // and every older shipped set that already has a row.
    let mut active: Vec<SetName> = vec![newest];
    active.extend(
        SHIPPED_SETS
            .iter()
            .filter(|set| **set != newest && tracks.iter().any(|track| track.set_id == set.as_str())),
    );
    for set in &active {
        let (old_xp, old_level, old_claimed) = held(*set);
        let new_level = track_level(old_xp + xp);
        tx.economy_tracks_grant(&TrackGrant {
            profile_id: payout.profile_id.to_string(),
            set_id: set.as_str().to_string(),
            xp,
            levels: new_level - old_level,
            rewards: new_level - old_claimed,
            reason: payout.reason,
            reference: Some(payout.reference.to_string()),
            dev: payout.dev,
            at: payout.at,
        })
        .await?;
        pay_level_rewards(tx, payout, set, old_claimed, new_level).await?;
    }
    // One older track unlocks at a time, nearest-older first, once the newest track reaches
    // `SET_TRACK_UNLOCK_LEVEL`; it counts as active from this payout, carrying this payout's XP.
    // The gate reads the newest track after this payout's XP landed, so the payout that reaches
    // the unlock level is the one that unlocks.
    let newest_level = track_level(held(newest).0 + xp);
    if newest_level >= i64::from(SET_TRACK_UNLOCK_LEVEL)
        && let Some(set) = SHIPPED_SETS
            .iter()
            .rev()
            .skip_while(|set| **set != newest)
            .skip(1)
            .find(|set| !tracks.iter().any(|track| track.set_id == set.as_str()))
    {
        let new_level = track_level(xp);
        tx.economy_tracks_grant(&TrackGrant {
            profile_id: payout.profile_id.to_string(),
            set_id: set.as_str().to_string(),
            xp,
            levels: new_level - SET_TRACK_FIRST_LEVEL,
            rewards: new_level,
            reason: payout.reason,
            reference: Some(payout.reference.to_string()),
            dev: payout.dev,
            at: payout.at,
        })
        .await?;
        pay_level_rewards(tx, payout, set, 0, new_level).await?;
    }
    Ok(())
}

/// Pays the levels `(claimed, level]` reached — each once, as ledger rows — of one track.
async fn pay_level_rewards(
    tx: &mut Tx<'_>,
    payout: &XpPayout<'_>,
    set: &SetName,
    claimed: i64,
    level: i64,
) -> StoreResult<()> {
    for reached in claimed + 1..=level {
        let reward = reward_for_level(reached);
        if reward.jlore != 0 {
            tx.economy_jlore_grant(&JloreGrant {
                profile_id: payout.profile_id.to_string(),
                delta: reward.jlore,
                reason: JloreReason::Track,
                reference: Some(payout.reference.to_string()),
                dev: payout.dev,
                at: payout.at,
            })
            .await?;
        }
        if reward.set_packs != 0 {
            tx.economy_inventory_grant(&InventoryGrant {
                profile_id: payout.profile_id.to_string(),
                item: InventoryItem::SetPack,
                key: set.as_str().to_string(),
                delta: reward.set_packs,
                reason: EconomyReason::Track,
                reference: Some(payout.reference.to_string()),
                dev: payout.dev,
                at: payout.at,
            })
            .await?;
        }
        if reward.card_tokens != 0 {
            tx.economy_inventory_grant(&InventoryGrant {
                profile_id: payout.profile_id.to_string(),
                item: InventoryItem::CardToken,
                key: String::new(),
                delta: reward.card_tokens,
                reason: EconomyReason::Track,
                reference: Some(payout.reference.to_string()),
                dev: payout.dev,
                at: payout.at,
            })
            .await?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{XP_GAME_CAP, XP_LADDER_TARGET_MEAN, XP_PER_ACTION, XP_WIN_BONUS};
    use crate::economy::math::total_xp_for_level;
    use jackioh_engine::{Action, GameResult, Winner};
    use serde_json::json;

    /// R1333001's measured games: 40 of this build's AI playing itself, one JSON line each
    /// (`cargo jackioh selfplay --games 40 --seed xp-1333`).
    const MEASURED_GAMES: &str = include_str!("../../tests/fixtures/xp-selfplay.jsonl");
    /// How far a measured mean XP may land from its target, as a share of the target (R1333001).
    const XP_MEAN_TOLERANCE: f64 = 0.05;
    /// The seats of R1333001's games: two to each of the 40.
    const MEASURED_SEATS: usize = 80;

    /// A measured game, as far as its XP reads it.
    #[derive(serde::Deserialize)]
    struct MeasuredGame {
        log: Vec<Action>,
        result: Option<GameResult>,
    }

    fn body(action: serde_json::Value) -> ActionBody {
        serde_json::from_value(action).expect("an ActionBody")
    }

    #[test]
    fn r1302004_only_moves_the_player_took_count() {
        for taken in [
            json!({ "type": "mulligan", "keep": [] }),
            json!({ "type": "play", "instanceId": "c1" }),
            json!({ "type": "attack", "attackerId": "c1", "targetId": "c2" }),
            json!({ "type": "switchPosition", "instanceId": "c1" }),
            json!({ "type": "activate", "instanceId": "c1" }),
            json!({ "type": "activatePower", "instanceId": "c1" }),
            json!({ "type": "answer", "choiceId": "q1", "selection": [] }),
            json!({ "type": "offerDraw" }),
            json!({ "type": "answerDraw", "accept": false }),
            json!({ "type": "emote", "emote": "greetings" }),
            json!({ "type": "concede" }),
            json!({ "type": "endTurn" }),
        ] {
            assert!(is_player_action(&body(taken.clone())), "{taken} is a move");
        }
        for not_taken in [
            json!({ "type": "setAutoEndTurn", "enabled": false }),
            json!({ "type": "timeout" }),
            json!({ "type": "disconnectExpired", "player": "p1" }),
            json!({ "type": "ceilingReached" }),
        ] {
            assert!(
                !is_player_action(&body(not_taken.clone())),
                "{not_taken} is no move"
            );
        }
    }

    #[test]
    fn r1302005_a_track_stops_at_its_last_level() {
        let last = i64::from(SET_TRACK_LEVELS);
        let at_last = total_xp_for_level(SET_TRACK_LEVELS);
        assert_eq!(track_level(0), SET_TRACK_FIRST_LEVEL);
        assert_eq!(track_level(at_last - 1), last - 1);
        assert_eq!(track_level(at_last), last);
        assert_eq!(track_level(total_xp_for_level(SET_TRACK_LEVELS + 1)), last);
        assert_eq!(track_level(i64::MAX), last);
    }

    #[test]
    fn xp_counts_actions_and_stops_at_the_cap() {
        assert_eq!(
            game_xp(GameMode::Ladder, 10, false),
            10 * i64::from(XP_PER_ACTION)
        );
        assert_eq!(
            game_xp(GameMode::Ladder, 10, true),
            10 * i64::from(XP_PER_ACTION) + i64::from(XP_WIN_BONUS)
        );
        assert_eq!(
            game_xp(GameMode::Ladder, i64::from(XP_GAME_CAP), true),
            i64::from(XP_GAME_CAP)
        );
        assert_eq!(
            game_xp(GameMode::Ladder, i64::MAX / 2, true),
            i64::from(XP_GAME_CAP)
        );
    }

    #[test]
    fn r1302007_practice_xp_is_the_ladders_times_the_scale_rounded_down() {
        assert_eq!(game_jlore(GameMode::Practice, Some(true)), JLORE_PRACTICE_WIN);
        assert_eq!(game_jlore(GameMode::Practice, Some(false)), JLORE_PRACTICE_LOSS);
        assert_eq!(game_jlore(GameMode::Practice, None), JLORE_PRACTICE_LOSS);
        for (actions, won) in [(0, true), (3, false), (7, true), (i64::MAX, true)] {
            let ladder = game_xp(GameMode::Ladder, actions, won);
            let practice = game_xp(GameMode::Practice, actions, won);
            assert_eq!(practice, (ladder as f64 * XP_PRACTICE_SCALE).floor() as i64);
            assert!(practice <= ladder);
        }
    }

    #[test]
    fn r1502000_each_mode_pays_its_own_rates() {
        for (mode, win, loss) in [
            (GameMode::Unranked, JLORE_UNRANKED_WIN, JLORE_UNRANKED_LOSS),
            (GameMode::Conquest, JLORE_CONQUEST_WIN, JLORE_CONQUEST_LOSS),
            (GameMode::Coop, JLORE_COOP_WIN, JLORE_COOP_LOSS),
        ] {
            assert_eq!(game_jlore(mode, Some(true)), win, "{mode:?} pays its win rate");
            assert_eq!(game_jlore(mode, Some(false)), loss, "{mode:?} pays its loss rate");
            assert_eq!(game_jlore(mode, None), loss, "{mode:?} pays a draw its loss rate");
        }
        for (actions, won) in [(0, true), (3, false), (7, true), (i64::MAX, true)] {
            let ladder = game_xp(GameMode::Ladder, actions, won);
            assert_eq!(game_xp(GameMode::Unranked, actions, won), ladder);
            assert_eq!(game_xp(GameMode::Conquest, actions, won), ladder);
            assert_eq!(
                game_xp(GameMode::Coop, actions, won),
                (ladder as f64 * XP_COOP_SCALE).floor() as i64
            );
        }
    }

    #[test]
    fn jlore_pays_the_win_rate_only_for_a_win() {
        assert_eq!(game_jlore(GameMode::Ladder, Some(true)), JLORE_LADDER_WIN);
        assert_eq!(game_jlore(GameMode::Ladder, Some(false)), JLORE_LADDER_LOSS);
        assert_eq!(game_jlore(GameMode::Ladder, None), JLORE_LADDER_LOSS);
    }

    #[test]
    fn the_win_bonus_and_the_cap_shape_the_payout() {
        // The shape #1333's table states, as behaviour: a win pays more than a loss even with no
        // actions, and the cap binds.
        assert_eq!(game_xp(GameMode::Ladder, 0, true), i64::from(XP_WIN_BONUS));
        assert_eq!(game_xp(GameMode::Ladder, 0, false), 0);
        assert!(game_xp(GameMode::Ladder, 0, true) > game_xp(GameMode::Ladder, 0, false));
        assert_eq!(game_xp(GameMode::Ladder, i64::MAX, false), i64::from(XP_GAME_CAP));
        assert_eq!(game_xp(GameMode::Ladder, i64::MAX, true), i64::from(XP_GAME_CAP));
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

    #[test]
    fn game_summary_carries_the_seat_the_caller_counted() {
        // The hook's input shape, as the parallel practice and quest callers construct it: whose
        // account, which seat's actions were counted, the result, and what to stamp the rows with.
        let summary = GameSummary {
            mode: GameMode::Ladder,
            profile_id: "profile-a".to_string(),
            player: PlayerId::P1,
            won: Some(true),
            actions: 10,
            match_id: "match-1".to_string(),
            at: 42,
            bot: false,
            tally: GameTally::default(),
        };
        assert_eq!(summary.player, PlayerId::P1);
        assert_eq!(summary.won, Some(true));
        assert_eq!(game_jlore(summary.mode, summary.won), JLORE_LADDER_WIN);
        assert_eq!(
            game_xp(summary.mode, summary.actions, summary.won == Some(true)),
            10 * i64::from(XP_PER_ACTION) + i64::from(XP_WIN_BONUS)
        );
    }
}
