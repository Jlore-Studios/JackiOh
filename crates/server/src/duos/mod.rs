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
//!   decks a match is dealt. `store`: the three tables of migration 0037.
//!
//! The upgrade is `actor::ws_server::handle_duos` and the routes are `app::ROUTES`' co-op block.
//! Nothing here touches a 1v1 match: a co-op match sets no profile's in-match flag, writes no
//! results row and moves no rating (S6: co-op games are unranked).

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

use crate::actor::contracts::one_tx;
use crate::app::{App, now_ms};
use crate::config::{DUOS_ROOM_TTL_MS, MATCH_ACTION_RETENTION_DAYS, MATCH_REAPER_INTERVAL_SECONDS};
use crate::db::store::StoreError;

pub use registry::{DuosRegistry, registry};

/// Unit conversion, not configuration.
const MS_PER_DAY: i64 = 86_400_000;

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

/// One sweep: B34's expired rooms go (a joined room one TTL later than an unjoined one, so its host's
/// poll still reads the match), finished actors no socket holds leave memory, and a live match with no
/// actor (a restart) is folded back, so its people find it running and an abandoned one is played out
/// by the AI (B37) and gets its result.
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
