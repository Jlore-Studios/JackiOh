//! Presence and status (SPEC S7, B47): `POST /api/presence` is the heartbeat a signed-in client
//! sends every `PRESENCE_HEARTBEAT_MS`, and `standings` is what a friend's row says of where they
//! are. A heartbeat from an account that owes agreement to the terms in force records nothing
//! (R970002): the Privacy Policy that names it is what the account has not agreed to yet.
//!
//! A friend is `inMatch` while a live 1v1 match holds them (the in-match flag §9.5 sets and every
//! ending clears, which `GET /api/auth/me` reads as `currentMatchId`), while they are between the
//! games of a Conquest series that is not over (`currentSeriesId`, R259), or while a live Duos
//! co-op match holds them (B42, read as `duos::live_match_of` reads it); else `online` while their
//! last heartbeat is at most `PRESENCE_TTL_MS` old; else `offline`. A ladder bot never beats, so it
//! reads as a person who is away, and in a match as a person in one (R1452).
//!
//! `watchable` (B50): the viewer may watch the friend's live 1v1 match, ranked or unranked, whose
//! friend allows spectators (`profiles.allow_spectators`) and has fewer than `SPECTATORS_PER_MATCH`
//! watchers. The watch socket judges it again at its hello (`social::watch`).
//!
//! `standings` reads every friend's at once, in the caller's transaction, by a fixed number of
//! set-based reads (heartbeats, series, co-op matches, the watch checks), whatever the number of
//! friends, so `GET /api/friends`, which a client reads every `PRESENCE_HEARTBEAT_MS`, costs one
//! transaction and no per-friend query.

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
/// between a series' games, or when not in one) and whether a friend may watch it (B50).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Standing {
    pub status: Status,
    pub match_id: Option<String>,
    pub watchable: bool,
}

impl Standing {
    /// A friend with no heartbeat within the TTL and in no match.
    pub const OFFLINE: Standing = Standing {
        status: Status::Offline,
        match_id: None,
        watchable: false,
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
/// profiles as given (their in-match flag) and four set-based reads: their heartbeats, those a
/// series that is not over holds, the live Duos co-op match holding each, and those whose live 1v1
/// match `viewer_id` may watch; each watched match's room is the actor's count, in memory.
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

    let seen = tx.presence_last_seen(&ids).await?;
    let in_series = tx.presence_in_series(&idle).await?;
    let in_duos = tx.presence_in_duos(&idle).await?;
    let watchable = tx.presence_watchable(viewer_id, &in_match).await?;

    let mut standings = IndexMap::new();
    for profile in profiles {
        let standing = if let Some(match_id) = &profile.in_match_id {
            Standing {
                status: Status::InMatch,
                match_id: Some(match_id.clone()),
                watchable: watchable.contains(&profile.id)
                    && watcher_count(app, match_id) < SPECTATORS_PER_MATCH,
            }
        } else if in_series.contains(&profile.id) {
            Standing {
                status: Status::InMatch,
                match_id: None,
                watchable: false,
            }
        } else if let Some(duos_match) = in_duos.get(&profile.id) {
            // Out of scope: watching a Duos game (SPEC, Out of scope).
            Standing {
                status: Status::InMatch,
                match_id: Some(duos_match.clone()),
                watchable: false,
            }
        } else if seen
            .get(&profile.id)
            .is_some_and(|at| now - at <= PRESENCE_TTL_MS)
        {
            Standing {
                status: Status::Online,
                match_id: None,
                watchable: false,
            }
        } else {
            Standing::OFFLINE
        };
        standings.insert(profile.id.clone(), standing);
    }
    Ok(standings)
}
