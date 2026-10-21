//! Duos co-op (#936, #946; fullsend SPEC S6, B33–B42): two people against an AI team, as two 1v1
//! duels side by side (the engine's `DuosState` wrapper), served by its own rooms, its own socket
//! (`/ws/duos`) and one actor per match.
//!
//! - `rooms`: the HTTP routes: make, read, join and free a room, and the caller's live co-op match.
//! - `replay`: a finished match's steps as the caller's seat saw them (B41).
//! - `ws`: the socket's authentication and hello, before the actor takes it (B35).
//! - `actor`: the match itself: the state, the log, the clocks, the AI's seats and the pings
//!   (B35–B40). `registry`: the live actors, and a crashed one folded back (B40).
//! - `protocol`: every `/ws/duos` frame. `engine`: the one path to the engine's Duos API and the
//!   decks a match is dealt. `store`: the three tables of migration 0039.
//!
//! The upgrade is `actor::ws_server::handle_duos` and the routes are `app::ROUTES`' co-op block.
//! Nothing here touches a 1v1 match: a co-op match sets no profile's in-match flag, writes no
//! results row and moves no rating (S6: co-op games are unranked). Its result pays both people
//! through the reward hook (`finish_match`, [[R1502000]]).

pub mod actor;
pub mod engine;
pub mod protocol;
pub mod registry;
pub mod replay;
pub mod rooms;
pub mod store;
pub mod ws;

use std::sync::Arc;
use std::time::Duration;

use jackioh_engine::wire::{
    DUOS_REASON_CEILING, DUOS_REASON_VOIDED, DuosActionBody, DuosOutcome, DuosResult, DuosSeat, Team,
};
use jackioh_engine::{ActionBody, PlayerId};

use crate::actor::contracts::one_tx;
use crate::actor::protocol::SERVER_NONCE_PREFIX;
use crate::app::{App, now_ms};
use crate::config::{
    DUOS_ROOM_TTL_MS, JLORE_GAMES_DAILY_CAP, MATCH_ACTION_RETENTION_DAYS, MATCH_CEILING_MS,
    MATCH_REAPER_INTERVAL_SECONDS,
};
use crate::db::store::{StoreError, StoreResult, Tx};
use crate::duos::store::{DuosActionRow, DuosMatch, action_of};
use crate::economy::quests::GameTally;
use crate::economy::rewards::{
    GameMode, GameSummary, game_jlore, is_player_action, pay_game_rewards, utc_day_bounds,
};

pub use registry::{DuosRegistry, registry};

/// Unit conversion, not configuration.
const MS_PER_DAY: i64 = 86_400_000;

/// When `row`'s match reaches the hard ceiling (§9.5, R79). R1047000: derived from the row and never
/// stored, so a rebuilt actor keeps the match's own deadline.
pub fn ceiling_at(row: &DuosMatch) -> i64 {
    row.created_at + MATCH_CEILING_MS
}

/// The live co-op match holding `profile_id`, if any (B42, B47).
pub async fn live_match_of(app: &App, profile_id: &str) -> Result<Option<String>, StoreError> {
    let mut tx = app.db.begin(Some(profile_id)).await?;
    let live = tx.duos_matches_live_for(profile_id).await?;
    tx.commit().await?;
    Ok(live)
}

/// The retention purge's co-op half (`api::retention::run_purge`): a co-op match's log goes
/// `MATCH_ACTION_RETENTION_DAYS` after the match ended, as a 1v1 match's does; the match row, its
/// result with it, stays, and its replay then answers 410 (`duos::replay`). The replays opened from
/// the logs it took go with them. Answers how many log rows went.
pub async fn purge_expired_logs(app: &Arc<App>) -> Result<u64, StoreError> {
    let before = now_ms() - MATCH_ACTION_RETENTION_DAYS * MS_PER_DAY;
    let purged = one_tx!(app.db, |t| t.duos_actions_purge(before).await?)?;
    if purged > 0 {
        registry(app).replays.clear();
    }
    Ok(purged)
}

/// R1047000's `ceiling` draw, as R1047001's sweep and R1253000 write it without the engine.
pub fn ceiling_draw() -> DuosResult {
    DuosResult {
        outcome: DuosOutcome::Draw,
        winner: None,
        reason: DUOS_REASON_CEILING.to_string(),
    }
}

/// B40, §14.8, [[R1502000]]: writes `result` and `ended_at` once (`duos_matches_finish`); the write
/// that lands pays both people in the same transaction while `ECONOMY_LIVE` is on ([[R1302006]]), so
/// a payout failure rolls the result back. A game a void ended pays nothing, as a voided 1v1 match
/// does not ([[R679]]). False, paying nothing, when the match already had its result.
pub async fn finish_match(
    t: &mut Tx<'_>,
    app: &App,
    row: &DuosMatch,
    result: &DuosResult,
    at: i64,
) -> StoreResult<bool> {
    let json = serde_json::to_value(result)?;
    if !t.duos_matches_finish(&row.id, &json, at).await? {
        return Ok(false);
    }
    if app.economy_live && result.reason != DUOS_REASON_VOIDED {
        pay_coop(t, row, result, at).await?;
    }
    Ok(true)
}

/// R1483000: whether the daily Jlore cap held back each person's pay for the finished co-op
/// match `match_id`, by `HUMAN_SEATS`. Read off the ledger after the write, so a fresh result, a
/// repeat of it and an actor rebuilt on the finished match all answer alike: a seat's pay was held
/// back when its `game` row for the match is below `GameMode::Coop`'s rate for its result and its
/// UTC day has reached `JLORE_GAMES_DAILY_CAP`. A voided game pays nothing (`finish_match` skips
/// it). A bot's side is never paid (`pay_coop` pays people only), so there is no bot row to skip.
pub async fn duos_game_jlore_caps(app: &App, match_id: &str) -> StoreResult<[bool; 2]> {
    let none = [false, false];
    if !app.economy_live {
        return Ok(none);
    }
    let mut tx = app.db.begin(None).await?;
    let row = tx.duos_matches_get(match_id).await?;
    let Some(row) = row else {
        return Ok(none);
    };
    let (Some(ended_at), Some(stored)) = (row.ended_at, row.result.clone()) else {
        return Ok(none);
    };
    let Ok(result) = serde_json::from_value::<DuosResult>(stored) else {
        return Ok(none);
    };
    if result.reason == DUOS_REASON_VOIDED {
        return Ok(none);
    }
    let won = result.winner.map(|team| team == Team::A);
    let (from, until) = utc_day_bounds(ended_at);
    let mut capped = [false, false];
    for (held_back, profile_id) in capped.iter_mut().zip([&row.a1, &row.a2]) {
        let totals = tx
            .economy_game_jlore_totals(profile_id, from, until, match_id)
            .await?;
        *held_back = totals.reference < i64::from(game_jlore(GameMode::Coop, won))
            && totals.day >= JLORE_GAMES_DAILY_CAP;
    }
    tx.commit().await?;
    Ok(capped)
}

/// [[R1502000]]: each person paid for their own seat at `GameMode::Coop`'s rates, a win when the
/// humans' team won. A co-op log is not folded into what a seat played, so its quests count the
/// result and the seat's attacks alone.
async fn pay_coop(t: &mut Tx<'_>, row: &DuosMatch, result: &DuosResult, at: i64) -> StoreResult<()> {
    // [[R1301003]]: both economy locks in profile-id order, as a 1v1 result takes its two.
    let mut ordered = [row.a1.clone(), row.a2.clone()];
    ordered.sort();
    for profile_id in &ordered {
        t.economy_lock_account(profile_id).await?;
    }
    let log = t.duos_actions(&row.id).await?;
    let won = result.winner.map(|team| team == Team::A);
    for (seat, profile_id) in [(DuosSeat::A1, &row.a1), (DuosSeat::A2, &row.a2)] {
        let (actions, attacks) = seat_moves(&log, seat);
        pay_game_rewards(
            t,
            &GameSummary {
                mode: GameMode::Coop,
                profile_id: profile_id.clone(),
                // §13.1: each person is Player 1 of their own duel.
                player: PlayerId::P1,
                won,
                actions,
                match_id: row.id.clone(),
                at,
                bot: false,
                tally: GameTally {
                    attacks,
                    ..GameTally::default()
                },
            },
        )
        .await?;
    }
    Ok(())
}

/// [[R1502000]], [[R1302004]]: the moves `seat`'s person sent in `log`, and its attacks. A row under
/// the server's nonce (R270: the AI's moves for the seat, the ceiling) is no one's; a duel action
/// counts as `is_player_action` says, a pass and the team's concede count. A row that does not read
/// as an action counts as none, so an unreadable log never stops a result.
fn seat_moves(log: &[DuosActionRow], seat: DuosSeat) -> (i64, i64) {
    let (mut moves, mut attacks) = (0, 0);
    for action in log.iter().filter_map(|row| action_of(&row.action).ok()) {
        if action.seat != seat || action.nonce.starts_with(SERVER_NONCE_PREFIX) {
            continue;
        }
        match &action.body {
            DuosActionBody::Duel { body } => {
                if is_player_action(body) {
                    moves += 1;
                }
                if matches!(body, ActionBody::Attack { .. }) {
                    attacks += 1;
                }
            }
            DuosActionBody::Pass { .. } | DuosActionBody::Concede => moves += 1,
        }
    }
    (moves, attacks)
}

/// R1047001: a live match no actor can be folded back for, once it is past its ceiling (R1047000), ends
/// as the ceiling's draw with the result the actor would have written, which frees both people as the
/// 1v1 reaper frees a stuck match's (§9.5). `duos_matches_finish` writes once and the first writer
/// stands, so an actor racing this sweep to the same result leaves one. Before the ceiling nothing
/// is written and the next sweep folds the match back again.
async fn end_abandoned(app: &Arc<App>, match_id: &str, now: i64) {
    let ended = one_tx!(app.db, |t| {
        match t.duos_matches_get(match_id).await? {
            Some(row) if row.ended_at.is_none() && now >= ceiling_at(&row) => {
                finish_match(&mut t, app, &row, &ceiling_draw(), now).await?
            }
            _ => false,
        }
    });
    match ended {
        Ok(true) => tracing::warn!(event = "duos.abandoned.ended", matchId = %match_id),
        Ok(false) => {}
        Err(error) => {
            tracing::error!(event = "duos.abandoned.failed", matchId = %match_id, message = %error);
        }
    }
}

/// One sweep: B34's expired rooms go (a joined room one TTL later than an unjoined one, so its host's
/// poll still reads the match), finished actors no socket holds leave memory, and a live match with no
/// actor (a restart) is folded back, so its people find it running and an abandoned one is played out
/// by the AI (B37) and gets its result. One that cannot be folded back is ended once past its ceiling
/// (R1047000, R1047001).
pub async fn sweep(app: &Arc<App>) -> Result<(), StoreError> {
    let now = now_ms();
    let reaped = one_tx!(app.db, |t| t.duos_rooms_reap(now, now - DUOS_ROOM_TTL_MS).await?)?;
    if reaped > 0 {
        tracing::info!(event = "duos.rooms.reaped", count = reaped);
    }
    let held = registry(app);
    held.reap_finished().await;
    let live = one_tx!(app.db, |t| t.duos_matches_live().await?)?;
    for match_id in live {
        if held.has(&match_id) {
            continue;
        }
        if let Err(error) = held.actor_for(app, &match_id).await {
            tracing::warn!(event = "duos.resume_failed", matchId = %match_id, message = %error);
            end_abandoned(app, &match_id, now).await;
        }
    }
    Ok(())
}

/// The co-op sweeper, at the 1v1 reaper's pace (`MATCH_REAPER_INTERVAL_SECONDS`), started by
/// `app::serve` through `loops::spawn_supervised`. A failed sweep is logged and tried again on the
/// next tick.
pub async fn run_sweeper(app: Arc<App>) {
    loop {
        tokio::time::sleep(Duration::from_secs(
            u64::try_from(MATCH_REAPER_INTERVAL_SECONDS).unwrap_or(0),
        ))
        .await;
        if let Err(error) = sweep(&app).await {
            tracing::warn!(event = "duos.sweep_failed", message = %error);
        }
    }
}
