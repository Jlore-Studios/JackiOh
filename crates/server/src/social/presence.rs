//! Presence and status (SPEC S7, B47): `POST /api/presence` is the heartbeat a signed-in client
//! sends every `PRESENCE_HEARTBEAT_MS`, and `standings` is what a friend's row says of where they
//! are.
//!
//! A friend is `inMatch` while a live 1v1 match holds them (the in-match flag §9.5 sets and every
//! ending clears, which `GET /api/auth/me` reads as `currentMatchId`), while they are between the
//! games of a Conquest series that is not over (`currentSeriesId`, R259), or while a live Duos
//! co-op match holds them (`duos::live_match_of`, B42); else `online` while their last heartbeat is
//! at most `PRESENCE_TTL_MS` old; else `offline`. A ladder bot never beats, so it reads as a person
//! who is away, and in a match as a person in one (R1452).
//!
//! `watchable` (B50) is true only for a friend in a live 1v1 match that is not ranked, who allows
//! spectators (`profiles.allow_spectators`). The watcher cap (`SPECTATORS_PER_MATCH`) is the watch
//! socket's to enforce (`/ws/watch`, B50): the count lives with the watchers, so a full match still
//! reads as watchable here and its hello is refused.

use std::sync::Arc;

use indexmap::{IndexMap, IndexSet};
use serde::Serialize;

use crate::api::collection::caller_profile;
use crate::api::http::{ApiError, ApiResult, Req, now_ms};
use crate::app::App;
use crate::config::PRESENCE_TTL_MS;
use crate::db::store::{MatchStatus, Profile};
use crate::social::no_content;

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

/// `POST /api/presence`: the caller is here now (B47). 204.
pub async fn heartbeat(app: &Arc<App>, req: Req) -> ApiResult {
    let me = caller_profile(&req)?.id;
    let mut tx = app.db.begin(Some(&me)).await?;
    tx.presence_touch(&me, now_ms()).await?;
    tx.commit().await?;
    Ok(no_content())
}

/// B47, B50: each profile's standing, by profile id, read from the profiles as given (their
/// in-match flag), the series that are not over, their heartbeats, the live matches holding them
/// and, last and outside any transaction, the Duos co-op matches.
pub(crate) async fn standings(app: &App, profiles: &[Profile]) -> Result<IndexMap<String, Standing>, ApiError> {
    if profiles.is_empty() {
        return Ok(IndexMap::new());
    }
    let now = now_ms();
    let ids: Vec<String> = profiles.iter().map(|profile| profile.id.clone()).collect();

    let mut tx = app.db.begin(None).await?;
    let seen = tx.presence_last_seen(&ids).await?;
    let in_series: IndexSet<String> = tx
        .series_active()
        .await?
        .into_iter()
        .flat_map(|series| [series.sides.0.profile_id, series.sides.1.profile_id])
        .collect();
    // B50: whether each in-match profile's 1v1 match may be watched.
    let mut watchable: IndexMap<String, bool> = IndexMap::new();
    for profile in profiles {
        let Some(match_id) = profile.in_match_id.as_deref() else {
            continue;
        };
        let unranked_live = tx
            .matches_get(match_id)
            .await?
            .is_some_and(|row| row.status == MatchStatus::Live && row.ranked != Some(true));
        let allowed = unranked_live && tx.profiles_allows_spectators(&profile.id).await?;
        watchable.insert(profile.id.clone(), allowed);
    }
    tx.commit().await?;

    let mut standings = IndexMap::new();
    for profile in profiles {
        let standing = if let Some(match_id) = &profile.in_match_id {
            Standing {
                status: Status::InMatch,
                match_id: Some(match_id.clone()),
                watchable: watchable.get(&profile.id).copied().unwrap_or(false),
            }
        } else if in_series.contains(&profile.id) {
            Standing {
                status: Status::InMatch,
                match_id: None,
                watchable: false,
            }
        } else if let Some(duos_match) = crate::duos::live_match_of(app, &profile.id).await? {
            // Out of scope: watching a Duos game (SPEC, Out of scope).
            Standing {
                status: Status::InMatch,
                match_id: Some(duos_match),
                watchable: false,
            }
        } else if seen.get(&profile.id).is_some_and(|at| now - at <= PRESENCE_TTL_MS) {
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
