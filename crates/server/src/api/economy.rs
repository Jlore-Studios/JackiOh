//! Read-only wallet and set tracks (SPEC §14, §14.1, §14.7, #1302).
//! Accounts read their own Jlore and track progress; ledgers update only through server-side
//! payouts (`economy::rewards`) and dev tools (R1300000).

use std::sync::Arc;

use jackioh_engine::wire::{SHIPPED_SETS, newest_shipped_set};
use serde_json::json;

use crate::api::collection::caller_profile;
use crate::api::http::{ApiResult, Req, json};
use crate::app::App;
use crate::config::{SET_TRACK_FIRST_LEVEL, SET_TRACK_LEVELS};
use crate::economy::rewards::TrackPoint;
use crate::economy::track_rewards::reward_for_level;

/// `GET /api/wallet` (`AuthLevel::Active`): the caller's Jlore balance.
pub async fn get_wallet(app: &Arc<App>, req: Req) -> ApiResult {
    let profile = caller_profile(&req)?;
    let mut tx = app.db.begin(Some(&profile.id)).await?;
    let jlore = tx.economy_jlore_get(&profile.id).await?;
    tx.commit().await?;
    Ok(json(200, json!({ "jlore": jlore })))
}

/// `GET /api/tracks` (`AuthLevel::Active`): progress for each shipped set.
/// Level and XP form a [`TrackPoint`]. Max-level tracks (`SET_TRACK_LEVELS`) return `null`
/// for `xpToNext`, `levelXp`, and `nextRewards` (R1302005).
pub async fn get_tracks(app: &Arc<App>, req: Req) -> ApiResult {
    let profile = caller_profile(&req)?;
    let mut tx = app.db.begin(Some(&profile.id)).await?;
    let tracks = tx.economy_tracks_get(&profile.id).await?;
    tx.commit().await?;
    let newest = newest_shipped_set();
    let entries: Vec<serde_json::Value> = SHIPPED_SETS
        .iter()
        .map(|set| {
            let held = tracks.iter().find(|track| track.set_id == set.as_str());
            let unlocked = *set == newest || held.is_some();
            let (level, xp, claimed) = held
                .map(|track| (track.level, track.xp, track.rewards_claimed))
                .unwrap_or((SET_TRACK_FIRST_LEVEL, 0, 0));
            let last = i64::from(SET_TRACK_LEVELS);
            let point = TrackPoint::at(level, xp);
            json!({
                "setId": set.as_str(),
                "level": point.level,
                "xp": point.xp,
                "xpToNext": point.xp_to_next,
                "levelXp": point.level_xp,
                "nextRewards": (claimed < last).then(|| reward_for_level(claimed + 1)),
                "rewardsClaimed": claimed,
                "unlocked": unlocked,
            })
        })
        .collect();
    Ok(json(200, json!({ "tracks": entries })))
}
