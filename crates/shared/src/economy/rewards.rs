//! What a finished game pays (SPEC §14.7 to §14.9; #1302, #1304, #1483, #3104), pure: a game's Jlore and XP at
//! its mode's rates, the daily Jlore cap's partial pay ([[R1483000]]), the set tracks it moves and the levels
//! they reach ([[R1302005]]), the quests it advances and completes ([[R1304001]]), and a new account's
//! starting rewards ([[R1310000]]). [`pay_game`] is the one decision over the records the store read (what the
//! account was paid today, its tracks, its quests) answering the grants to write, in order, and what the
//! game paid for a client to animate (#1657); [`pay_xp`], [`starting_rewards`] and [`complete_quest`] are
//! the smaller ones a dev tool or a new account makes. The server writes them under the account's lock
//! (R1301003) and decides nothing (`crates/shared/README.md`, the host contract).
//!
//! - A day's games pay an account at most `JLORE_GAMES_DAILY_CAP` Jlore, every mode's together, over the
//!   UTC day the game ended in ([[R1483000]]): the game that crosses it is paid the part that fits, the
//!   ones after it no Jlore (and no Jlore row); XP and quests are paid in full. Quest and track Jlore
//!   neither count nor stop.
//! - A bot's side pays nothing ([[R1454]]).
//! - Every number is a [`crate::config`] constant (CLAUDE.md rule 9).

use jackioh_engine::wire::{SHIPPED_SETS, SetName, newest_shipped_set};
use jackioh_engine::{ActionBody, PlayerId};
use serde::{Deserialize, Serialize};

use crate::config::{
    JLORE_CONQUEST_LOSS, JLORE_CONQUEST_WIN, JLORE_COOP_LOSS, JLORE_COOP_WIN, JLORE_GAMES_DAILY_CAP,
    JLORE_LADDER_LOSS, JLORE_LADDER_WIN, JLORE_PRACTICE_LOSS, JLORE_PRACTICE_WIN, JLORE_UNRANKED_LOSS,
    JLORE_UNRANKED_WIN, MS_PER_DAY, SET_TRACK_FIRST_LEVEL, SET_TRACK_LEVELS, SET_TRACK_UNLOCK_LEVEL,
    XP_COOP_SCALE, XP_GAME_CAP, XP_PER_ACTION, XP_PRACTICE_SCALE, XP_WIN_BONUS,
};
use crate::economy::fleet::{FleetPlay, FleetTrack, counts_fleet, pay_fleet};
use crate::economy::ledger::{Cause, InventoryItem, LedgerGrant, QuestBook, QuestChange, SetTrack};
use crate::economy::math::{level_for_xp, level_span_xp, xp_to_next_level};
use crate::economy::quests::{GameTally, QuestDef, progress_of, quest_def, quest_xp, top_up_book};
use crate::economy::track_rewards::{LevelReward, reward_for_level};

/// Which kind of game finished, so the hook pays its rates (SPEC §14.8).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "../../../apps/web/src/wire/generated/",
        rename = "EconomyGameMode"
    )
)]
#[serde(rename_all = "camelCase")]
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
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "../../../apps/web/src/wire/generated/",
        rename = "EconomyGameSummary"
    )
)]
#[serde(rename_all = "camelCase")]
pub struct GameSummary {
    pub mode: GameMode,
    /// Whose account is paid.
    pub profile_id: String,
    pub player: PlayerId,
    /// `Some(true)` for a win, `Some(false)` for a loss, `None` for a draw ([[R1302000]]).
    pub won: Option<bool>,
    /// How many actions the player took: the log's rows of their seat that
    /// [`is_player_action`] counts.
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub actions: i64,
    /// The match, for the ledger rows' `reference`.
    pub match_id: String,
    /// When the game ended, for the ledger rows' `at`.
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub at: i64,
    /// A bot's side: never paid ([[R1454]]).
    pub bot: bool,
    /// What the seat did, as the quests count it (§14.9).
    pub tally: GameTally,
    /// [[R3013000]]: the cards the seat played that count toward its proficiency, one id per play
    /// (`proficiency::proficiency_plays`); counted only in a mode that counts them.
    pub proficiency: Vec<String>,
    /// [[R3335001]]: what the seat did that the Fleet track pays for (`fleet::fleet_play`), all zero in a mode
    /// that pays none ([`crate::economy::fleet::counts_fleet`]) and in a summary written before #3335.
    #[serde(default)]
    pub fleet: FleetPlay,
}

/// Where one set track stands (§14.7), as `GET /api/tracks` spells it (`api::economy::get_tracks`):
/// its level and XP, the XP its level spans and the XP to the next level, so a client draws the bar
/// without knowing the curve. A track on its last level, `SET_TRACK_LEVELS`, has neither
/// ([[R1302005]]).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct TrackPoint {
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub level: i64,
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub xp: i64,
    #[cfg_attr(feature = "ts", ts(type = "number | null"))]
    pub level_xp: Option<i64>,
    #[cfg_attr(feature = "ts", ts(type = "number | null"))]
    pub xp_to_next: Option<i64>,
}

impl TrackPoint {
    pub fn at(level: i64, xp: i64) -> TrackPoint {
        let next = level < i64::from(SET_TRACK_LEVELS);
        TrackPoint {
            level,
            xp,
            level_xp: next.then(|| level_span_xp(xp)),
            xp_to_next: next.then(|| xp_to_next_level(xp)),
        }
    }
}

/// A level a payout reached and what it paid ([`reward_for_level`]).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct LevelReached {
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub level: i64,
    pub rewards: LevelReward,
}

/// How far one game's payout moved one set track, the XP of every quest it completed included:
/// where the track stood before and after, and each level it paid, in order. A track the payout
/// unlocked stood on its first level with no XP before, as `GET /api/tracks` reads a locked one.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct TrackGain {
    pub set_id: String,
    pub before: TrackPoint,
    pub after: TrackPoint,
    pub levels: Vec<LevelReached>,
}

/// A quest a game completed (§14.9), named as `GET /api/quests` names it.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct QuestPaid {
    pub quest_id: String,
    pub title: String,
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub jlore: i64,
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub xp: i64,
}

impl QuestPaid {
    fn of(def: &QuestDef) -> QuestPaid {
        QuestPaid {
            quest_id: def.id.clone(),
            title: def.title.clone(),
            jlore: i64::from(def.jlore),
            xp: quest_xp(def),
        }
    }
}

/// What [`pay_game_rewards`] paid one game (#1657): its own Jlore and XP, whether the daily cap
/// held some of that Jlore back ([[R1483000]]), each set track it moved and each quest it
/// completed. All zero and empty for a bot's side.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct GameEconomy {
    /// The Jlore written: the game's rate, or what was left under the daily cap.
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub jlore: i64,
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub xp: i64,
    /// Whether the daily cap held part or all of the game's Jlore back.
    pub jlore_capped: bool,
    /// One per track the payout moved, in catalog order, as `GET /api/tracks` lists them.
    pub tracks: Vec<TrackGain>,
    pub quests: Vec<QuestPaid>,
}

/// Whether a logged action is one the player took, so earns `XP_PER_ACTION` (SPEC §14.8,
/// [[R1302004]]): every move of the game, an emote a card hears (MD-D29) among them. The server's
/// own rows are not ([[R79]]), nor [[R345]]'s automatic-end setting, nor a practice sandbox's tool
/// ([[R923000]]). Every kind is named, so a new one is counted or not by a decision, never by default.
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
        | ActionBody::EndTurn
        // R1838000: a Trade is a move the player took; R1853003: so is a buy or a sale at the Marketplace.
        | ActionBody::Trade { .. }
        | ActionBody::MarketBuy { .. }
        | ActionBody::MarketSell { .. } => true,
        ActionBody::SetAutoEndTurn { .. }
        | ActionBody::Sandbox { .. }
        | ActionBody::Timeout
        | ActionBody::DisconnectExpired { .. }
        | ActionBody::CeilingReached => false,
    }
}

/// The level a set track with `total_xp` XP stands on (SPEC §14.7, [[R1302005]]): the level
/// curve's, never past the track's last level, `SET_TRACK_LEVELS`; XP past it lands but reaches no
/// level.
pub fn track_level(total_xp: i64) -> i64 {
    i64::from(level_for_xp(total_xp).min(SET_TRACK_LEVELS))
}

/// A game's XP (SPEC §14.8): `XP_PER_ACTION` for each action taken, `XP_WIN_BONUS` more for a
/// win, the total never above `XP_GAME_CAP`, so a loop of empty actions cannot farm it; a practice
/// game earns that times `XP_PRACTICE_SCALE`, and a co-op game that times `XP_COOP_SCALE`, each
/// rounded down ([[R1502000]]).
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
    let start = at.div_euclid(MS_PER_DAY) * MS_PER_DAY;
    (start, start.saturating_add(MS_PER_DAY))
}

/// A [`TrackGain`] for each shipped set's track that differs from `before` to `after`, in catalog
/// order, each read as [`pay_xp`] reads it ([`held_track`]). The levels a gain lists are those
/// paid in between, `(claimed, claimed']`.
fn track_gains(before: &[SetTrack], after: &[SetTrack]) -> Vec<TrackGain> {
    SHIPPED_SETS
        .iter()
        .filter_map(|set| {
            let (old_xp, old_level, old_claimed) = held_track(before, *set);
            let (xp, level, claimed) = held_track(after, *set);
            ((level, xp, claimed) != (old_level, old_xp, old_claimed)).then(|| TrackGain {
                set_id: set.as_str().to_string(),
                before: TrackPoint::at(old_level, old_xp),
                after: TrackPoint::at(level, xp),
                levels: (old_claimed + 1..=claimed)
                    .map(|level| LevelReached {
                        level,
                        rewards: reward_for_level(level),
                    })
                    .collect(),
            })
        })
        .collect()
}

/// `set`'s track in `tracks` as `(xp, level, levels paid)`: its row's, or, with none, its first
/// level with no XP and no level paid (§14.7).
fn held_track(tracks: &[SetTrack], set: SetName) -> (i64, i64, i64) {
    tracks
        .iter()
        .find(|track| track.set_id == set.as_str())
        .map(|track| (track.xp, track.level, track.rewards_claimed))
        .unwrap_or((0, SET_TRACK_FIRST_LEVEL, 0))
}

/// What the account's store holds that a finished game's pay reads: the Jlore its games already paid it in the
/// UTC day the game ended in, not counting this match's own (R1483000), its set tracks and its quests.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct GameRecords {
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub paid_today: i64,
    pub tracks: Vec<SetTrack>,
    pub quests: QuestBook,
    /// [[R3335001]]: the account's Fleet track, with the XP its games paid it today.
    #[serde(default)]
    pub fleet: FleetTrack,
}

/// A finished game paid: what it paid, for a client to animate, and every write to make.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct GamePayout {
    pub economy: GameEconomy,
    /// The writes, in order: the game's Jlore, its XP on every active track and the levels it reached, its Fleet
    /// XP and the levels' rewards (R3335001), the quests that arrived, then each quest's progress and completion with what a completion pays.
    pub grants: Vec<LedgerGrant>,
}

/// The tracks after a grant moves one: the row of `set_id`, written as the store writes it (a track not yet
/// written starts at its first level with no XP and nothing paid).
fn move_track(tracks: &mut Vec<SetTrack>, set_id: &str, xp: i64, levels: i64, rewards: i64) {
    match tracks.iter_mut().find(|track| track.set_id == set_id) {
        Some(track) => {
            track.xp = track.xp.saturating_add(xp);
            track.level = track.level.saturating_add(levels);
            track.rewards_claimed = track.rewards_claimed.saturating_add(rewards);
        }
        None => tracks.push(SetTrack {
            set_id: set_id.to_string(),
            xp,
            level: SET_TRACK_FIRST_LEVEL + levels,
            rewards_claimed: rewards,
        }),
    }
}

/// Pays the levels `(claimed, level]` reached of one track, each once: its Jlore, its packs of the track's own
/// set and its card tokens, every row naming `reference` (the match, or the quest, or the signup).
fn level_rewards(grants: &mut Vec<LedgerGrant>, set: SetName, claimed: i64, level: i64, reference: &str) {
    for reached in claimed + 1..=level {
        let reward = reward_for_level(reached);
        if reward.jlore != 0 {
            grants.push(LedgerGrant::Jlore {
                delta: reward.jlore,
                cause: Cause::Track,
                reference: Some(reference.to_string()),
            });
        }
        if reward.set_packs != 0 {
            grants.push(LedgerGrant::Inventory {
                item: InventoryItem::SetPack,
                key: set.as_str().to_string(),
                delta: reward.set_packs,
                cause: Cause::Track,
                reference: Some(reference.to_string()),
            });
        }
        if reward.card_tokens != 0 {
            grants.push(LedgerGrant::Inventory {
                item: InventoryItem::CardToken,
                key: String::new(),
                delta: reward.card_tokens,
                cause: Cause::Track,
                reference: Some(reference.to_string()),
            });
        }
    }
}

/// One XP payout onto every active set track (§14.7): a game's or a completed quest's. Every active track gains
/// all of `xp`, the newest set's, active from the start, and every older shipped set that already has a row, each
/// reaching the levels its XP now stands on and claiming their rewards. One older track unlocks at a time,
/// nearest-older first, once the newest track reaches `SET_TRACK_UNLOCK_LEVEL`; it counts as active from this
/// payout, carrying this payout's XP, and the payout that reaches the unlock level is the one that unlocks
/// ([[R1302003]]). `cause` is the track rows' reason (`Track` for a game's XP, `Quest` for a quest's); the level
/// rewards are always `Track`'s. No XP writes no track row. `tracks` moves as the store's rows will.
fn pay_xp_onto(
    tracks: &mut Vec<SetTrack>,
    grants: &mut Vec<LedgerGrant>,
    xp: i64,
    cause: Cause,
    reference: &str,
) {
    if xp <= 0 {
        return;
    }
    // The tracks as this payout found them: every gate below reads them, not what the payout has moved.
    let found = tracks.clone();
    let held = |set: SetName| held_track(&found, set);
    let newest = newest_shipped_set();
    let mut active: Vec<SetName> = vec![newest];
    active.extend(
        SHIPPED_SETS
            .iter()
            .filter(|set| **set != newest && found.iter().any(|track| track.set_id == set.as_str())),
    );
    for set in &active {
        let (old_xp, old_level, old_claimed) = held(*set);
        let new_level = track_level(old_xp + xp);
        grants.push(LedgerGrant::Track {
            set_id: set.as_str().to_string(),
            xp,
            levels: new_level - old_level,
            rewards: new_level - old_claimed,
            cause,
            reference: Some(reference.to_string()),
        });
        move_track(
            tracks,
            set.as_str(),
            xp,
            new_level - old_level,
            new_level - old_claimed,
        );
        level_rewards(grants, *set, old_claimed, new_level, reference);
    }
    let newest_level = track_level(held(newest).0 + xp);
    if newest_level >= i64::from(SET_TRACK_UNLOCK_LEVEL)
        && let Some(set) = SHIPPED_SETS
            .iter()
            .rev()
            .skip_while(|set| **set != newest)
            .skip(1)
            .find(|set| !found.iter().any(|track| track.set_id == set.as_str()))
    {
        let new_level = track_level(xp);
        grants.push(LedgerGrant::Track {
            set_id: set.as_str().to_string(),
            xp,
            levels: new_level - SET_TRACK_FIRST_LEVEL,
            rewards: new_level,
            cause,
            reference: Some(reference.to_string()),
        });
        move_track(
            tracks,
            set.as_str(),
            xp,
            new_level - SET_TRACK_FIRST_LEVEL,
            new_level,
        );
        level_rewards(grants, *set, 0, new_level, reference);
    }
}

/// A dev tool's or other XP payout onto the account's `tracks` (§14.7, [[R1302003]]): the writes
/// ([`pay_xp_onto`]'s), each naming `reference`.
pub fn pay_xp(tracks: &[SetTrack], xp: i64, cause: Cause, reference: &str) -> Vec<LedgerGrant> {
    let mut grants = Vec::new();
    pay_xp_onto(&mut tracks.to_vec(), &mut grants, xp, cause, reference);
    grants
}

/// What the starting rewards' rows name as their `reference` ([`starting_rewards`]).
pub const STARTING_REWARDS_REFERENCE: &str = "signup";

/// A new account's tracks' first rewards (§14.12, [[R1310000]]): the newest set's track, active from the start,
/// is marked paid through its first level (`SET_TRACK_FIRST_LEVEL`) and that level's [`reward_for_level`] is
/// paid, as a payout reaching it would. A track already paid through its first level writes nothing, so it pays
/// once.
pub fn starting_rewards(tracks: &[SetTrack]) -> Vec<LedgerGrant> {
    let newest = newest_shipped_set();
    let claimed = tracks
        .iter()
        .find(|track| track.set_id == newest.as_str())
        .map_or(0, |track| track.rewards_claimed);
    if claimed >= SET_TRACK_FIRST_LEVEL {
        return Vec::new();
    }
    let mut grants = vec![LedgerGrant::Track {
        set_id: newest.as_str().to_string(),
        xp: 0,
        levels: 0,
        rewards: SET_TRACK_FIRST_LEVEL - claimed,
        cause: Cause::Track,
        reference: Some(STARTING_REWARDS_REFERENCE.to_string()),
    }];
    level_rewards(
        &mut grants,
        newest,
        claimed,
        SET_TRACK_FIRST_LEVEL,
        STARTING_REWARDS_REFERENCE,
    );
    grants
}

/// Completes the quest `def` and pays it onto `tracks`: its row (`cause`: a game's, or a dev tool's), its Jlore,
/// and its XP onto every active track (§14.7), each row naming the quest.
fn complete_onto(
    tracks: &mut Vec<SetTrack>,
    grants: &mut Vec<LedgerGrant>,
    def: &QuestDef,
    cause: Cause,
    reference: Option<&str>,
    at: i64,
) {
    grants.push(LedgerGrant::Quest {
        quest_id: def.id.clone(),
        change: QuestChange::Completed,
        progress: 0,
        cause,
        reference: reference.map(str::to_string),
        at,
    });
    if def.jlore != 0 {
        grants.push(LedgerGrant::Jlore {
            delta: i64::from(def.jlore),
            cause: Cause::Quest,
            reference: Some(def.id.clone()),
        });
    }
    pay_xp_onto(tracks, grants, quest_xp(def), Cause::Quest, &def.id);
}

/// A dev tool's completion of the held quest `quest_id` (§14.10): the writes a completion makes, or `None` when
/// `quests` does not hold it (or the pool no longer has it): a quest pays once.
pub fn complete_quest(
    quests: &QuestBook,
    tracks: &[SetTrack],
    quest_id: &str,
    cause: Cause,
    reference: Option<&str>,
    at: i64,
) -> Option<Vec<LedgerGrant>> {
    quests.held.iter().find(|held| held.quest_id == quest_id)?;
    let def = quest_def(quest_id)?;
    let mut grants = Vec::new();
    complete_onto(&mut tracks.to_vec(), &mut grants, def, cause, reference, at);
    Some(grants)
}

/// Pays one finished game's rewards ([[R1302006]]): its Jlore, held to the account's daily cap
/// ([[R1483000]]), its XP on every active set track ([`pay_xp_onto`]), and then moves its quests, which may pay
/// too ([[R1304001]]): the account is topped up at the game's end first (from `seed`), so a quest that fell due
/// as it ended counts it too; each held quest then moves by what the game did, never past its target, and
/// completes and pays on reaching it. A bot's side pays nothing. Answers what it paid, each track's gain read
/// off the tracks before and after the whole payout, so a completed quest's XP is in it, and the writes in
/// order.
pub fn pay_game(records: &GameRecords, summary: &GameSummary, seed: &str) -> GamePayout {
    if summary.bot {
        return GamePayout::default();
    }
    let mut tracks = records.tracks.clone();
    let mut grants = Vec::new();
    let rate = i64::from(game_jlore(summary.mode, summary.won));
    let jlore = rate.min((JLORE_GAMES_DAILY_CAP - records.paid_today).max(0));
    if jlore != 0 {
        grants.push(LedgerGrant::Jlore {
            delta: jlore,
            cause: Cause::Game,
            reference: Some(summary.match_id.clone()),
        });
    }
    let xp = game_xp(summary.mode, summary.actions, summary.won == Some(true));
    pay_xp_onto(&mut tracks, &mut grants, xp, Cause::Track, &summary.match_id);
    // R3335001: a recorded 1v1 game's Fleet XP and the levels it reaches, apart from the set tracks.
    if counts_fleet(summary.mode) {
        grants.extend(pay_fleet(
            &records.fleet,
            &summary.fleet,
            summary.won,
            &summary.match_id,
        ));
    }

    let topped = top_up_book(&records.quests, seed, summary.at);
    grants.extend(topped.grants);
    let mut completed: Vec<&'static QuestDef> = Vec::new();
    for held in &topped.book.held {
        let Some(def) = quest_def(&held.quest_id) else {
            continue;
        };
        let step =
            progress_of(&def.goal, summary.won, &summary.tally).min((def.target - held.progress).max(0));
        if step > 0 {
            grants.push(LedgerGrant::Quest {
                quest_id: def.id.clone(),
                change: QuestChange::Progressed,
                progress: step,
                cause: Cause::Game,
                reference: Some(summary.match_id.clone()),
                at: summary.at,
            });
        }
        if held.progress.saturating_add(step) >= def.target {
            complete_onto(
                &mut tracks,
                &mut grants,
                def,
                Cause::Game,
                Some(&summary.match_id),
                summary.at,
            );
            completed.push(def);
        }
    }
    GamePayout {
        economy: GameEconomy {
            jlore,
            xp,
            jlore_capped: jlore < rate,
            tracks: track_gains(&records.tracks, &tracks),
            quests: completed.into_iter().map(QuestPaid::of).collect(),
        },
        grants,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{QUEST_MAX_HELD, XP_GAME_CAP, XP_PER_ACTION, XP_WIN_BONUS};
    use crate::economy::ledger::HeldQuest;
    use crate::economy::math::total_xp_for_level;
    use crate::economy::quests::{QuestGoal, pool, quest_interval_ms};
    use serde_json::json;

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
        // The shape #1333's table states, as behavior: a win pays more than a loss even with no
        // actions, and the cap binds.
        assert_eq!(game_xp(GameMode::Ladder, 0, true), i64::from(XP_WIN_BONUS));
        assert_eq!(game_xp(GameMode::Ladder, 0, false), 0);
        assert!(game_xp(GameMode::Ladder, 0, true) > game_xp(GameMode::Ladder, 0, false));
        assert_eq!(game_xp(GameMode::Ladder, i64::MAX, false), i64::from(XP_GAME_CAP));
        assert_eq!(game_xp(GameMode::Ladder, i64::MAX, true), i64::from(XP_GAME_CAP));
    }

    #[test]
    fn game_summary_carries_the_seat_the_caller_counted() {
        // The hook's input shape, as the practice and quest callers construct it.
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
            proficiency: Vec::new(),
            fleet: FleetPlay::default(),
        };
        assert_eq!(summary.player, PlayerId::P1);
        assert_eq!(summary.won, Some(true));
        assert_eq!(game_jlore(summary.mode, summary.won), JLORE_LADDER_WIN);
        assert_eq!(
            game_xp(summary.mode, summary.actions, summary.won == Some(true)),
            10 * i64::from(XP_PER_ACTION) + i64::from(XP_WIN_BONUS)
        );
    }

    const AT: i64 = 1_700_000_000_000;

    fn summary(mode: GameMode, won: Option<bool>, actions: i64) -> GameSummary {
        GameSummary {
            mode,
            profile_id: "profile-a".to_string(),
            player: PlayerId::P1,
            won,
            actions,
            match_id: "match-1".to_string(),
            at: AT,
            bot: false,
            tally: GameTally::default(),
            proficiency: Vec::new(),
            fleet: FleetPlay::default(),
        }
    }

    /// Records with every quest slot spoken for, so no top-up arrival draws.
    fn quiet() -> QuestBook {
        QuestBook {
            held: Vec::new(),
            next_due_at: Some(AT + 1000 * quest_interval_ms()),
        }
    }

    fn records(paid_today: i64, tracks: Vec<SetTrack>, quests: QuestBook) -> GameRecords {
        GameRecords {
            paid_today,
            tracks,
            quests,
            fleet: FleetTrack::default(),
        }
    }

    fn jlore_of(grants: &[LedgerGrant], cause: Cause) -> Vec<i64> {
        grants
            .iter()
            .filter_map(|grant| match grant {
                LedgerGrant::Jlore { delta, cause: at, .. } if *at == cause => Some(*delta),
                _ => None,
            })
            .collect()
    }

    /// R1483000: the game that crosses the day's cap is paid the part that fits, the ones after it no Jlore
    /// and no Jlore row, and its XP is paid in full.
    #[test]
    fn r1483000_the_game_that_crosses_the_cap_is_paid_the_part_that_fits() {
        let win = summary(GameMode::Ladder, Some(true), 10);
        let xp = game_xp(GameMode::Ladder, 10, true);
        let under = pay_game(&records(0, Vec::new(), quiet()), &win, "s");
        assert_eq!(
            jlore_of(&under.grants, Cause::Game),
            [i64::from(JLORE_LADDER_WIN)]
        );
        assert!(!under.economy.jlore_capped);
        let crossing = pay_game(
            &records(JLORE_GAMES_DAILY_CAP - 5, Vec::new(), quiet()),
            &win,
            "s",
        );
        assert_eq!(jlore_of(&crossing.grants, Cause::Game), [5]);
        assert_eq!((crossing.economy.jlore, crossing.economy.jlore_capped), (5, true));
        assert_eq!(crossing.economy.xp, xp);
        let over = pay_game(&records(JLORE_GAMES_DAILY_CAP, Vec::new(), quiet()), &win, "s");
        assert!(jlore_of(&over.grants, Cause::Game).is_empty(), "no Jlore row");
        assert_eq!(
            (over.economy.jlore, over.economy.jlore_capped, over.economy.xp),
            (0, true, xp)
        );
        let past = pay_game(
            &records(JLORE_GAMES_DAILY_CAP + 50, Vec::new(), quiet()),
            &win,
            "s",
        );
        assert!(jlore_of(&past.grants, Cause::Game).is_empty());
        // Track Jlore neither counts nor stops: the first level's purse is paid under a spent cap.
        assert_eq!(jlore_of(&over.grants, Cause::Track).first(), Some(&100));
    }

    /// R1302002, R1302005: a game's XP lands on the newest track, and each level it reaches pays its reward
    /// once, in order, naming the match.
    #[test]
    fn r1302006_a_game_pays_xp_on_the_newest_track_and_the_levels_it_reaches() {
        let paid = pay_game(
            &records(0, Vec::new(), quiet()),
            &summary(GameMode::Ladder, Some(true), 10),
            "s",
        );
        let newest = newest_shipped_set();
        let xp = game_xp(GameMode::Ladder, 10, true);
        let level = track_level(xp);
        assert!(level >= 2, "the fixture reaches a level");
        let track = paid
            .grants
            .iter()
            .find_map(|grant| match grant {
                LedgerGrant::Track {
                    set_id,
                    xp: gained,
                    levels,
                    rewards,
                    cause,
                    reference,
                } => Some((
                    set_id.as_str(),
                    *gained,
                    *levels,
                    *rewards,
                    *cause,
                    reference.as_deref(),
                )),
                _ => None,
            })
            .expect("a track grant");
        assert_eq!(
            track,
            (
                newest.as_str(),
                xp,
                level - SET_TRACK_FIRST_LEVEL,
                level,
                Cause::Track,
                Some("match-1")
            )
        );
        // The rewards of levels 1 to `level`, in order, each naming the match.
        let mut expected = Vec::new();
        for reached in 1..=level {
            let reward = reward_for_level(reached);
            if reward.jlore != 0 {
                expected.push(("jlore", reward.jlore));
            }
            if reward.set_packs != 0 {
                expected.push(("pack", reward.set_packs));
            }
            if reward.card_tokens != 0 {
                expected.push(("token", reward.card_tokens));
            }
        }
        let rewards: Vec<(&str, i64)> = paid
            .grants
            .iter()
            .filter_map(|grant| match grant {
                LedgerGrant::Jlore {
                    delta,
                    cause: Cause::Track,
                    reference,
                } => {
                    assert_eq!(reference.as_deref(), Some("match-1"));
                    Some(("jlore", *delta))
                }
                LedgerGrant::Inventory {
                    item,
                    delta,
                    cause: Cause::Track,
                    ..
                } => Some((
                    if *item == InventoryItem::SetPack {
                        "pack"
                    } else {
                        "token"
                    },
                    *delta,
                )),
                _ => None,
            })
            .collect();
        assert_eq!(rewards, expected);
        // The answer reads the track off before and after.
        assert_eq!(paid.economy.tracks.len(), 1);
        let gain = &paid.economy.tracks[0];
        assert_eq!(
            (gain.before.level, gain.after.level, gain.after.xp),
            (1, level, xp)
        );
        assert_eq!(gain.levels.len() as i64, level);
    }

    /// R1302003: the payout that takes the newest track to `SET_TRACK_UNLOCK_LEVEL` unlocks the nearest older
    /// track without a row, carrying the payout's XP, and one at a time.
    #[test]
    fn r1302003_the_payout_that_reaches_the_unlock_level_unlocks_one_older_track() {
        let newest = newest_shipped_set();
        let older = SHIPPED_SETS
            .iter()
            .rev()
            .skip_while(|set| **set != newest)
            .nth(1)
            .copied()
            .expect("an older shipped set");
        let at_nine = total_xp_for_level(SET_TRACK_UNLOCK_LEVEL - 1);
        let tracks = vec![SetTrack {
            set_id: newest.as_str().to_string(),
            xp: at_nine,
            level: i64::from(SET_TRACK_UNLOCK_LEVEL) - 1,
            rewards_claimed: i64::from(SET_TRACK_UNLOCK_LEVEL) - 1,
        }];
        let xp = total_xp_for_level(SET_TRACK_UNLOCK_LEVEL) - at_nine;
        let grants = pay_xp(&tracks, xp, Cause::Track, "m");
        let unlocked: Vec<(&str, i64, i64, i64)> = grants
            .iter()
            .filter_map(|grant| match grant {
                LedgerGrant::Track {
                    set_id,
                    xp: gained,
                    levels,
                    rewards,
                    ..
                } => Some((set_id.as_str(), *gained, *levels, *rewards)),
                _ => None,
            })
            .collect();
        let older_level = track_level(xp);
        assert_eq!(unlocked.len(), 2, "the newest track and one older");
        assert_eq!(unlocked[0].0, newest.as_str());
        assert_eq!(
            unlocked[1],
            (
                older.as_str(),
                xp,
                older_level - SET_TRACK_FIRST_LEVEL,
                older_level
            )
        );
        // With the older track already written, it is an active track like any other, and the next older one
        // unlocks in its place.
        let mut both = tracks.clone();
        both.push(SetTrack {
            set_id: older.as_str().to_string(),
            xp: 0,
            level: SET_TRACK_FIRST_LEVEL,
            rewards_claimed: 0,
        });
        let again = pay_xp(&both, xp, Cause::Track, "m");
        let sets: Vec<&str> = again
            .iter()
            .filter_map(|grant| match grant {
                LedgerGrant::Track { set_id, .. } => Some(set_id.as_str()),
                _ => None,
            })
            .collect();
        let next_older = SHIPPED_SETS
            .iter()
            .rev()
            .skip_while(|set| **set != newest)
            .nth(2)
            .copied()
            .expect("a second older shipped set");
        assert_eq!(sets, [newest.as_str(), older.as_str(), next_older.as_str()]);
        // No XP writes no track row.
        assert!(pay_xp(&tracks, 0, Cause::Track, "m").is_empty());
    }

    /// R1304001: a held quest moves by what the game did, never past its target, and completes and pays on
    /// reaching it: its row, its Jlore and its XP onto the tracks.
    #[test]
    fn r1304001_a_game_moves_a_held_quest_and_a_completion_pays() {
        let def = pool()
            .iter()
            .find(|quest| quest.goal == QuestGoal::PlayGames)
            .expect("a play-games quest");
        let held = |progress: i64| QuestBook {
            held: vec![HeldQuest {
                quest_id: def.id.clone(),
                progress,
                arrived_at: AT - 1,
            }],
            next_due_at: Some(AT + 1000 * quest_interval_ms()),
        };
        // One step short of the target: it progresses by the game's one and, reaching it, completes.
        let paid = pay_game(
            &records(0, Vec::new(), held(def.target - 1)),
            &summary(GameMode::Ladder, Some(false), 0),
            "s",
        );
        let quest_rows: Vec<(QuestChange, i64, Cause)> = paid
            .grants
            .iter()
            .filter_map(|grant| match grant {
                LedgerGrant::Quest {
                    change,
                    progress,
                    cause,
                    quest_id,
                    ..
                } => {
                    assert_eq!(*quest_id, def.id);
                    Some((*change, *progress, *cause))
                }
                _ => None,
            })
            .collect();
        assert_eq!(
            quest_rows,
            [
                (QuestChange::Progressed, 1, Cause::Game),
                (QuestChange::Completed, 0, Cause::Game)
            ]
        );
        assert_eq!(jlore_of(&paid.grants, Cause::Quest), [i64::from(def.jlore)]);
        assert_eq!(paid.economy.quests.len(), 1);
        assert_eq!(paid.economy.quests[0].quest_id, def.id);
        // The quest's XP lands on the track after the game's own.
        let xps: Vec<(i64, Cause)> = paid
            .grants
            .iter()
            .filter_map(|grant| match grant {
                LedgerGrant::Track { xp, cause, .. } => Some((*xp, *cause)),
                _ => None,
            })
            .collect();
        assert_eq!(xps.last(), Some(&(quest_xp(def), Cause::Quest)));
        // Short of the target by more than the game moves: progress only.
        if def.target > 1 {
            let slower = pay_game(
                &records(0, Vec::new(), held(0)),
                &summary(GameMode::Ladder, Some(false), 0),
                "s",
            );
            assert!(slower.economy.quests.is_empty());
            assert!(slower.grants.iter().all(|grant| !matches!(
                grant,
                LedgerGrant::Quest {
                    change: QuestChange::Completed,
                    ..
                }
            )));
        }
    }

    /// R1304001: the account is topped up at the game's end first, so a quest that fell due counts the game.
    #[test]
    fn r1304001_a_quest_due_as_the_game_ended_counts_it() {
        let due = QuestBook {
            held: Vec::new(),
            next_due_at: Some(AT - 1),
        };
        let paid = pay_game(
            &records(0, Vec::new(), due),
            &summary(GameMode::Ladder, Some(true), 3),
            "seed-x",
        );
        let arrivals = paid
            .grants
            .iter()
            .filter(|grant| {
                matches!(
                    grant,
                    LedgerGrant::Quest {
                        change: QuestChange::Arrived,
                        ..
                    }
                )
            })
            .count();
        assert!((1..=QUEST_MAX_HELD).contains(&arrivals));
        assert!(
            paid.grants
                .iter()
                .any(|grant| matches!(grant, LedgerGrant::QuestDue { .. }))
        );
        // A bot's side pays nothing, and reads nothing.
        let mut bot = summary(GameMode::Ladder, Some(true), 3);
        bot.bot = true;
        assert_eq!(
            pay_game(&records(0, Vec::new(), quiet()), &bot, "s"),
            GamePayout::default()
        );
    }

    /// R1310000: a new account's newest track is marked paid through its first level and that level's reward is
    /// paid; a track already paid through it writes nothing.
    #[test]
    fn r1310000_the_starting_rewards_pay_the_first_level_once() {
        let grants = starting_rewards(&[]);
        let newest = newest_shipped_set();
        assert_eq!(
            grants[0],
            LedgerGrant::Track {
                set_id: newest.as_str().to_string(),
                xp: 0,
                levels: 0,
                rewards: SET_TRACK_FIRST_LEVEL,
                cause: Cause::Track,
                reference: Some(STARTING_REWARDS_REFERENCE.to_string()),
            }
        );
        assert_eq!(
            grants[1],
            LedgerGrant::Jlore {
                delta: reward_for_level(SET_TRACK_FIRST_LEVEL).jlore,
                cause: Cause::Track,
                reference: Some(STARTING_REWARDS_REFERENCE.to_string()),
            }
        );
        let paid = vec![SetTrack {
            set_id: newest.as_str().to_string(),
            xp: 0,
            level: SET_TRACK_FIRST_LEVEL,
            rewards_claimed: SET_TRACK_FIRST_LEVEL,
        }];
        assert!(starting_rewards(&paid).is_empty());
    }

    /// A dev completion pays a held quest once: not held, nothing.
    #[test]
    fn a_completion_pays_a_held_quest_and_only_that() {
        let def = &pool()[0];
        let book = QuestBook {
            held: vec![HeldQuest {
                quest_id: def.id.clone(),
                progress: 0,
                arrived_at: AT,
            }],
            next_due_at: None,
        };
        let grants = complete_quest(&book, &[], &def.id, Cause::Dev, None, AT).expect("a completion");
        assert!(matches!(
            grants[0],
            LedgerGrant::Quest {
                change: QuestChange::Completed,
                cause: Cause::Dev,
                reference: None,
                ..
            }
        ));
        assert_eq!(jlore_of(&grants, Cause::Quest), [i64::from(def.jlore)]);
        assert_eq!(complete_quest(&quiet(), &[], &def.id, Cause::Dev, None, AT), None);
        assert_eq!(complete_quest(&book, &[], "no-such", Cause::Dev, None, AT), None);
    }
}
