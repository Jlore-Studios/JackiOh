//! Presence and status (SPEC S7, B47): `POST /api/presence` is the heartbeat a signed-in client sends every
//! `PRESENCE_HEARTBEAT_MS`, and `standings` is what a friend's row says of where they are. A heartbeat
//! from an account that owes agreement to the terms in force records nothing (R970002).
//!
//! A friend is `inMatch` while a live 1v1 match holds them (the in-match flag §9.5 sets and every ending
//! clears, which `GET /api/auth/me` reads as `currentMatchId`), while they are between the games of a
//! Conquest series that is not over (`currentSeriesId`, R259), or while a live Duos co-op match holds them
//! (B42, read as `duos::live_match_of` reads it); else `online` while their last heartbeat is at most
//! `PRESENCE_TTL_MS` old; else `offline`. A ladder bot never beats, so it reads as a person who is away,
//! and in a match as a person in one (R1452).
//!
//! `watchable` (B50): the viewer may watch the friend's live 1v1 match, ranked or unranked, or their live
//! co-op game when the viewer is not their teammate in it and stands blocked with neither of its people
//! (R2881000, `duos` then true), with fewer than
//! `SPECTATORS_PER_MATCH` watchers; no setting of the friend's refuses it (R2615000), and the watch socket
//! judges it again at its hello (`social::watch`). Every caller hands `standings` the viewer's friends only,
//! so a co-op game's `watchable` reads no friendship of its own. `standings` reads every friend's at once in
//! the caller's transaction, by a fixed number of set-based reads, so `GET /api/friends` costs no per-friend
//! query.

use std::sync::Arc;

use indexmap::IndexMap;
use serde::Serialize;

use crate::api::collection::caller_profile;
use crate::api::consent::terms_owed;
use crate::api::http::{ApiResult, Req, now_ms};
use crate::app::App;
use crate::config::{PRESENCE_TTL_MS, SPECTATORS_PER_MATCH};
use crate::db::store::{Profile, StoreError, Tx};
use crate::social::no_content;
use crate::social::watch::watcher_count;

/// S7's `Friend.status`.
#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Status {
    Offline,
    Online,
    InMatch,
}

/// Where a friend is (B47): their status, the match holding them (a live 1v1 or Duos match; none
/// between a series' games, or when not in one), whether a friend may watch it (B50) and whether it
/// is a co-op game (R2881000).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Standing {
    pub status: Status,
    pub match_id: Option<String>,
    pub watchable: bool,
    pub duos: bool,
}

impl Standing {
    pub const OFFLINE: Standing = Standing {
        status: Status::Offline,
        match_id: None,
        watchable: false,
        duos: false,
    };
}

/// `POST /api/presence`: the caller is here now (B47), unless it owes agreement to the terms in
/// force, when nothing is recorded until it agrees (R970002). 204 either way.
pub async fn heartbeat(app: &Arc<App>, req: Req) -> ApiResult {
    let me = caller_profile(&req)?.id;
    let mut tx = app.db.begin(Some(&me)).await?;
    if !terms_owed(&tx.consent_get(&me).await?) {
        tx.presence_touch(&me, now_ms()).await?;
    }
    tx.commit().await?;
    Ok(no_content())
}

/// B47, B50: each profile's standing as `viewer_id` sees it, by profile id, read in `tx` from the
/// profiles as given and five set-based reads: heartbeats, series, Duos, watchable matches and the
/// co-op games seating someone the viewer stands blocked with (R2881000).
pub(crate) async fn standings(
    tx: &mut Tx<'_>,
    app: &App,
    viewer_id: &str,
    profiles: &[Profile],
) -> Result<IndexMap<String, Standing>, StoreError> {
    if profiles.is_empty() {
        return Ok(IndexMap::new());
    }
    let now = now_ms();
    let ids: Vec<String> = profiles.iter().map(|profile| profile.id.clone()).collect();
    let in_match: Vec<String> = profiles
        .iter()
        .filter(|profile| profile.in_match_id.is_some())
        .map(|profile| profile.id.clone())
        .collect();
    let idle: Vec<String> = profiles
        .iter()
        .filter(|profile| profile.in_match_id.is_none())
        .map(|profile| profile.id.clone())
        .collect();

    // R2881000: the viewer's own co-op game rides on the same read, so a teammate is never told
    // their own game is watchable.
    let mut duos_ids = idle.clone();
    duos_ids.push(viewer_id.to_string());

    let seen = tx.presence_last_seen(&ids).await?;
    let in_series = tx.presence_in_series(&idle).await?;
    let in_duos = tx.presence_in_duos(&duos_ids).await?;
    let watchable = tx.presence_watchable(viewer_id, &in_match).await?;
    // R2881000: a watcher is shown the friend's teammate's duel too (§13.6), so a block with the
    // teammate refuses the watch as one with the friend does.
    let coop_games: Vec<String> = idle.iter().filter_map(|id| in_duos.get(id).cloned()).collect();
    let duos_blocked = tx.presence_duos_blocked(viewer_id, &coop_games).await?;

    let mut standings = IndexMap::new();
    for profile in profiles {
        let standing = if let Some(match_id) = &profile.in_match_id {
            Standing {
                status: Status::InMatch,
                match_id: Some(match_id.clone()),
                watchable: watchable.contains(&profile.id)
                    && watcher_count(app, match_id) < SPECTATORS_PER_MATCH,
                duos: false,
            }
        } else if in_series.contains(&profile.id) {
            Standing {
                status: Status::InMatch,
                match_id: None,
                watchable: false,
                duos: false,
            }
        } else if let Some(duos_match) = in_duos.get(&profile.id) {
            // R2881000: a friend's co-op game is watched as a 1v1 match is, but by their teammate or
            // by someone standing blocked with that teammate.
            Standing {
                status: Status::InMatch,
                match_id: Some(duos_match.clone()),
                watchable: in_duos.get(viewer_id) != Some(duos_match)
                    && !duos_blocked.contains(duos_match)
                    && watcher_count(app, duos_match) < SPECTATORS_PER_MATCH,
                duos: true,
            }
        } else if seen
            .get(&profile.id)
            .is_some_and(|at| now - at <= PRESENCE_TTL_MS)
        {
            Standing {
                status: Status::Online,
                match_id: None,
                watchable: false,
                duos: false,
            }
        } else {
            Standing::OFFLINE
        };
        standings.insert(profile.id.clone(), standing);
    }
    Ok(standings)
}
