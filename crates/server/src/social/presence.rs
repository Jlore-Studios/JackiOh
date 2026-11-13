//! Presence and status (SPEC S7, B47): every signed-in request of an active account that passes auth,
//! the rate limit and the gate is a heartbeat, which the router's touch (`touch_on_request`) records at
//! most once a `PRESENCE_TOUCH_EVERY_MS` (R3210000); `POST /api/presence` is the beat a client sends
//! once `PRESENCE_HEARTBEAT_MS` passes with no other signed-in request, and `standings` is what a
//! friend's row says of where they are. Nothing is recorded while the account owes agreement to the
//! terms in force (R970002).
//!
//! A friend is `inMatch` while a live 1v1 match holds them (the in-match flag §9.5 sets and every ending
//! clears, which `GET /api/auth/me` reads as `currentMatchId`), while they are between the games of a
//! Conquest series that is not over (`currentSeriesId`, R259), or while a live Duos co-op match holds them
//! (B42, read as `duos::live_match_of` reads it); else `online` while their last heartbeat is at most
//! `PRESENCE_TTL_MS` old; else `offline`. A ladder bot never beats, so it reads as a person who is away,
//! and in a match as a person in one (R1452).
//!
//! R3732001: a `Standing` also says the turn of the 1v1 match a friend is in, while the viewer may
//! watch it, and when an offline friend was last seen, if presence kept a heartbeat.
//!
//! `watchable` (B50): the viewer may watch the friend's live 1v1 match, ranked or unranked, or their live
//! co-op game when the viewer is not their teammate in it and stands blocked with neither of its people
//! (R2881000, `duos` then true), with fewer than
//! `SPECTATORS_PER_MATCH` watchers; no setting of the friend's refuses it (R2615000), and the watch socket
//! judges it again at its hello (`social::watch`). Every caller hands `standings` the viewer's friends only,
//! so a co-op game's `watchable` reads no friendship of its own. `standings` reads every friend's at once in
//! the caller's transaction, by a fixed number of set-based reads, so `GET /api/friends` costs no per-friend
//! query.

use std::sync::{Arc, Mutex};

use indexmap::IndexMap;
use serde::Serialize;
use serde_json::json;

use crate::api::collection::caller_profile;
use crate::api::consent::terms_owed;
use crate::api::http::{ApiResult, Req, log_warn, now_ms};
use crate::app::App;
use crate::config::{PRESENCE_TOUCH_EVERY_MS, PRESENCE_TTL_MS, SPECTATORS_PER_MATCH};
use crate::db::store::{Profile, ProfileStatus, StoreError, Tx};
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
/// between a series' games, or when not in one), whether a friend may watch it (B50), whether it
/// is a co-op game (R2881000), the turn of a live 1v1 game a friend may watch and, for an offline
/// friend, the last heartbeat (R3732001).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Standing {
    pub status: Status,
    pub match_id: Option<String>,
    pub watchable: bool,
    pub duos: bool,
    /// R3732001: the player-turn counter of the live 1v1 match, only while the viewer may watch it
    /// (`watchable`), as a watcher is shown it; never a co-op game's or a series' gap's.
    pub turn: Option<i32>,
    /// R3732001: epoch ms of an offline friend's last heartbeat, when presence has kept one; none
    /// while they are online or in a match, and for a friend who never beat.
    pub last_seen: Option<i64>,
}

impl Standing {
    pub const OFFLINE: Standing = Standing {
        status: Status::Offline,
        match_id: None,
        watchable: false,
        duos: false,
        turn: None,
        last_seen: None,
    };
}

/// R3210000: when the router's touch last wrote each account's presence row, in epoch ms, so a busy
/// client costs one write a `PRESENCE_TOUCH_EVERY_MS`. One per `App`, in memory, as R897000's queue
/// presence is: a restart or a second server only writes sooner.
#[derive(Default)]
pub struct PresenceTouches {
    touched: Mutex<IndexMap<String, i64>>,
}

impl PresenceTouches {
    /// Whether a touch of `profile_id` at `now` writes: none has in the last `PRESENCE_TOUCH_EVERY_MS`.
    /// A claim marks it; the marks older than that period go, so the map holds only the accounts
    /// heard from within one.
    pub fn claim(&self, profile_id: &str, now: i64) -> bool {
        let mut touched = self
            .touched
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if touched
            .get(profile_id)
            .is_some_and(|at| now - *at < PRESENCE_TOUCH_EVERY_MS)
        {
            return false;
        }
        touched.retain(|_, at| now - *at < PRESENCE_TOUCH_EVERY_MS);
        touched.insert(profile_id.to_string(), now);
        true
    }

    /// The next touch of `profile_id` writes: its write failed, or it has just agreed to the terms.
    pub fn forget(&self, profile_id: &str) {
        self.touched
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .shift_remove(profile_id);
    }
}

/// R3210000: the router's touch, for a request that passed auth, the rate limit and the gate. An
/// active account is here now (B47), written at most once a `PRESENCE_TOUCH_EVERY_MS`; a pending or
/// banned one never is, on a route open to it or not. A write that fails is logged and refuses
/// nothing, and the next request tries again.
pub async fn touch_on_request(app: &App, profile: &Profile) {
    if !matches!(profile.status, ProfileStatus::Active) {
        return;
    }
    let now = now_ms();
    if !app.social.presence.claim(&profile.id, now) {
        return;
    }
    if let Err(error) = record_presence(app, &profile.id, now).await {
        app.social.presence.forget(&profile.id);
        log_warn(
            "presence.touch_failed",
            json!({ "profileId": profile.id, "message": error.to_string() }),
        );
    }
}

/// The caller is here at `now` (B47), unless it owes agreement to the terms in force, when nothing is
/// recorded until it agrees (R970002).
async fn record_presence(app: &App, me: &str, now: i64) -> Result<(), StoreError> {
    let mut tx = app.db.begin(Some(me)).await?;
    if !terms_owed(&tx.consent_get(me).await?) {
        tx.presence_touch(me, now).await?;
    }
    tx.commit().await
}

/// `POST /api/presence`: the caller is here now (B47), which the router's touch (`touch_on_request`,
/// R3210000) has recorded, unless the account owes agreement to the terms in force (R970002). 204
/// either way.
pub async fn heartbeat(_app: &Arc<App>, req: Req) -> ApiResult {
    caller_profile(&req)?;
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
            let may_watch =
                watchable.contains(&profile.id) && watcher_count(app, match_id) < SPECTATORS_PER_MATCH;
            Standing {
                status: Status::InMatch,
                match_id: Some(match_id.clone()),
                watchable: may_watch,
                duos: false,
                // R3732001: the live actor's own counter, which a watcher sees too.
                turn: if may_watch {
                    app.matches.get(match_id).map(|actor| actor.snapshot().turn)
                } else {
                    None
                },
                last_seen: None,
            }
        } else if in_series.contains(&profile.id) {
            Standing {
                status: Status::InMatch,
                match_id: None,
                watchable: false,
                duos: false,
                turn: None,
                last_seen: None,
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
                turn: None,
                last_seen: None,
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
                turn: None,
                last_seen: None,
            }
        } else {
            Standing {
                last_seen: seen.get(&profile.id).copied(),
                ..Standing::OFFLINE
            }
        };
        standings.insert(profile.id.clone(), standing);
    }
    Ok(standings)
}
