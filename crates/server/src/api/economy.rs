//! The wallet and the set tracks, read-only (SPEC §14.1, §14.7, #1302).
//!
//! An account reads its own Jlore balance and its own tracks — level, XP, the XP to the next
//! level, the next level's rewards and how many levels have paid — and nobody else's (§14). Like
//! `GET /api/collection`, these are reads alone: the ledger moves only through the server-side
//! payouts (`economy::rewards`), never through a client path.

use std::sync::Arc;

use jackioh_engine::wire::{SHIPPED_SETS, newest_shipped_set};
use serde_json::json;

use crate::api::collection::caller_profile;
use crate::api::http::{ApiResult, Req, json};
use crate::app::App;
use crate::config::{SET_TRACK_FIRST_LEVEL, SET_TRACK_LEVELS};
use crate::economy::math::xp_to_next_level;
use crate::economy::track_rewards::reward_for_level;

/// `GET /api/wallet` (`AuthLevel::Active`): the caller's Jlore balance.
pub async fn get_wallet(app: &Arc<App>, req: Req) -> ApiResult {
    let profile = caller_profile(&req)?;
    let mut tx = app.db.begin(Some(&profile.id)).await?;
    let jlore = tx.economy_jlore_get(&profile.id).await?;
    tx.commit().await?;
    Ok(json(200, json!({ "jlore": jlore })))
}

/// `GET /api/tracks` (`AuthLevel::Active`): one entry per shipped set in catalog order. A locked
/// older set reports the first level with no XP, its first rewards next, and `unlocked: false`.
/// A track on its last level, `SET_TRACK_LEVELS`, has no next level: its `xpToNext` and
/// `nextRewards` are `null` ([[R1302005]]).
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
            json!({
                "setId": set.as_str(),
                "level": level,
                "xp": xp,
                "xpToNext": (level < last).then(|| xp_to_next_level(xp)),
                "nextRewards": (claimed < last).then(|| reward_for_level(claimed + 1)),
                "rewardsClaimed": claimed,
                "unlocked": unlocked,
            })
        })
        .collect();
    Ok(json(200, json!({ "tracks": entries })))
}
