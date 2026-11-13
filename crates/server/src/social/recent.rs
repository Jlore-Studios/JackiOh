//! Recently played (R3732000, issue #3732): `GET /api/friends/recent`, the people the caller lately
//! played an unranked game with and has no friendship or block with, each one with an Add friend
//! on the Friends page.
//!
//! A game the queue paired is ranked (R604) and a room, lobby, challenge or daily game is not, so
//! the ranked flag alone keeps every queue opponent out. That is what R1452 asks: a ladder bot must
//! be indistinguishable from a person, and a list of the queue's opponents with an Add friend on each
//! would hand the bots over. Nothing here reads a name, a rating or a profile's kind to decide; a
//! bot's profile that somehow sat in an unranked game would be left out as well, not shown.

use std::collections::BTreeSet;
use std::sync::Arc;

use serde::Serialize;
use serde_json::json;

use crate::api::collection::caller_profile;
use crate::api::http::{ApiResult, Req, to_json};
use crate::app::App;
use crate::config::{RECENT_OPPONENTS_MAX, RECENT_OPPONENTS_SCAN};
use crate::db::store::ProfileKind;
use crate::social::{iso, profiles_by_id};

/// One opponent of `GET /api/friends/recent`: who, and when the game with them ended.
#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RecentOpponent {
    pub profile_id: String,
    pub username: String,
    /// When the newest unranked game with them ended, ISO-8601.
    pub played_at: String,
}

/// `GET /api/friends/recent`: `{ recent: [{ profileId, username, playedAt }] }`, newest first, at
/// most `RECENT_OPPONENTS_MAX`, read from the caller's newest `RECENT_OPPONENTS_SCAN` games. Left
/// out: ranked games (R3732000), the caller, a friend, and anyone a block stands between.
pub async fn get_recent(app: &Arc<App>, req: Req) -> ApiResult {
    let me = caller_profile(&req)?.id;

    let mut tx = app.db.begin(Some(&me)).await?;
    let games = tx.replays_list_with_modes(&me, RECENT_OPPONENTS_SCAN, 0).await?;
    let friends: BTreeSet<String> = tx
        .friends_of(&me)
        .await?
        .iter()
        .map(|row| row.other(&me).to_string())
        .collect();

    // Newest first, one entry for each opponent; the block read runs for a person once, and the
    // walk stops at the cap.
    let mut seen: BTreeSet<String> = BTreeSet::new();
    let mut found: Vec<(String, i64)> = Vec::new();
    for entry in games.iter().filter(|entry| entry.replay.row.ranked != Some(true)) {
        let row = &entry.replay.row;
        let opponent = if row.players.0 == me {
            &row.players.1
        } else {
            &row.players.0
        };
        if opponent.is_empty()
            || *opponent == me
            || friends.contains(opponent)
            || !seen.insert(opponent.clone())
        {
            continue;
        }
        if tx.blocks_between(&me, opponent).await? {
            continue;
        }
        found.push((opponent.clone(), entry.replay.result.ended_at));
        if found.len() >= RECENT_OPPONENTS_MAX {
            break;
        }
    }
    let ids: Vec<String> = found.iter().map(|(id, _)| id.clone()).collect();
    let profiles = profiles_by_id(&mut tx, &ids).await?;
    tx.commit().await?;

    let recent: Vec<RecentOpponent> = found
        .iter()
        .filter_map(|(id, ended_at)| {
            let profile = profiles
                .get(id)
                .filter(|profile| profile.kind != ProfileKind::Bot)?;
            Some(RecentOpponent {
                profile_id: profile.id.clone(),
                username: profile.username(),
                played_at: iso(*ended_at),
            })
        })
        .collect();
    Ok(crate::api::http::ok(json!({ "recent": to_json(&recent)? })))
}
