//! Friends and blocks (SPEC S7, B43–B46).
//!
//! - `GET /api/friends`: the caller's friends with their status (B47), the requests they received
//!   and sent, and the players they blocked.
//! - `POST /api/friends/requests` `{username}`: a request by the exact username as shown, `name#tag`
//!   (B43), looked up as a sign-in by username looks one up (R1443: the base's key and the tag).
//!   An unknown name and a pair either side blocked answer the same 404, so a request never tells
//!   whom a player blocked; a request to oneself, to a friend, a second request, or one past
//!   `MAX_FRIENDS` is 409 with its own code; past `FRIEND_REQUESTS_PER_HOUR` an hour, 429, counted
//!   before the name is looked up, so the limit also bounds how fast names can be tried. A request
//!   the other side already sent is accepted at once (B44).
//! - `POST /api/friends/requests/:id/accept` makes the friendship and deletes the request;
//!   `…/decline` (its recipient) and `DELETE /api/friends/requests/:id` (its sender) delete it;
//!   `DELETE /api/friends/:profileId` ends a friendship (B45), and with it every replay share
//!   between the two, either way, and a watch of either by the other (B50, B52): making friends
//!   again restores neither.
//! - `POST /api/blocks` `{profileId}` blocks a player: the friendship, every request and every
//!   replay share between the two go, either way, and so does every open challenge, its room freed,
//!   and a watch of either by the other; from then on requests, challenges, chat and shares are
//!   refused either way (B46). `DELETE /api/blocks/:profileId` lifts it and restores nothing.
//!
//! `GET /api/friends` is read every `PRESENCE_HEARTBEAT_MS` by each signed-in client, so it reads
//! everything in one transaction by a fixed number of set-based reads, whatever the number of
//! friends (`presence::standings`).

use std::sync::Arc;

use serde::Serialize;
use serde_json::json;

use crate::actor::rooms::release_room;
use crate::api::collection::caller_profile;
use crate::api::http::{
    ApiResult, Req, account_key, bad_request, log_info, now_ms, rate_limited, str, to_json,
};
use crate::api::queue::new_uuid;
use crate::app::App;
use crate::config::MAX_FRIENDS;
use crate::db::store::{ChallengeState, FriendRequestRow, Profile, StoreError, Tx};
use crate::social::presence::{Standing, Status, standings};
use crate::social::watch::revoke_between;
use crate::social::{PlayerRef, created, iso, no_content, not_found, param, profiles_by_id, refused};
use crate::username::sign_in_name;

/// S7's `Friend`: a friend as the list shows them.
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Friend {
    pub profile_id: String,
    pub username: String,
    pub status: Status,
    pub match_id: Option<String>,
    pub watchable: bool,
    /// When the friendship began, ISO-8601.
    pub since: String,
}

impl Friend {
    pub fn of(profile: &Profile, since: i64, standing: Option<&Standing>) -> Friend {
        let standing = standing.cloned().unwrap_or(Standing::OFFLINE);
        Friend {
            profile_id: profile.id.clone(),
            username: profile.username(),
            status: standing.status,
            match_id: standing.match_id,
            watchable: standing.watchable,
            since: iso(since),
        }
    }
}

/// S7's `FriendRequest`.
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FriendRequest {
    pub request_id: String,
    pub from: PlayerRef,
    pub to: PlayerRef,
    pub created_at: String,
}

/// B46, B53 and the rest of S7: whether the two are friends that neither has blocked, the one
/// relation chat, challenges and shares are open between. (A block ends the friendship, so the
/// second read only guards a row written around it.)
pub(crate) async fn are_friends(tx: &mut Tx<'_>, x: &str, y: &str) -> Result<bool, StoreError> {
    if x == y || tx.friends_get(x, y).await?.is_none() {
        return Ok(false);
    }
    Ok(!tx.blocks_between(x, y).await?)
}

/// The order the list is shown in: by name, then tag, then id, so it reads the same each time.
fn by_name(a: &Profile, b: &Profile) -> std::cmp::Ordering {
    (&a.username_key, a.username_tag, &a.id).cmp(&(&b.username_key, b.username_tag, &b.id))
}

/// `GET /api/friends` (B47, B55): `{ friends, incoming, outgoing, blocked }`.
pub async fn get_friends(app: &Arc<App>, req: Req) -> ApiResult {
    let me = caller_profile(&req)?.id;

    let mut tx = app.db.begin(Some(&me)).await?;
    let friendships = tx.friends_of(&me).await?;
    let requests = tx.friends_requests_of(&me).await?;
    let blocks = tx.blocks_of(&me).await?;
    let mut named: Vec<String> = friendships.iter().map(|row| row.other(&me).to_string()).collect();
    named.extend(
        requests
            .iter()
            .flat_map(|row| [row.from_profile.clone(), row.to_profile.clone()]),
    );
    named.extend(blocks.iter().map(|row| row.blocked.clone()));
    named.push(me.clone());
    let profiles = profiles_by_id(&mut tx, &named).await?;

    let mut friend_profiles: Vec<(Profile, i64)> = friendships
        .iter()
        .filter_map(|row| {
            profiles
                .get(row.other(&me))
                .map(|profile| (profile.clone(), row.since))
        })
        .collect();
    friend_profiles.sort_by(|a, b| by_name(&a.0, &b.0));
    let listed: Vec<Profile> = friend_profiles
        .iter()
        .map(|(profile, _)| profile.clone())
        .collect();
    let standing = standings(&mut tx, app, &me, &listed).await?;
    tx.commit().await?;
    let friends: Vec<Friend> = friend_profiles
        .iter()
        .map(|(profile, since)| Friend::of(profile, *since, standing.get(&profile.id)))
        .collect();

    let request_of = |row: &FriendRequestRow| -> Option<FriendRequest> {
        Some(FriendRequest {
            request_id: row.id.clone(),
            from: PlayerRef::of(profiles.get(&row.from_profile)?),
            to: PlayerRef::of(profiles.get(&row.to_profile)?),
            created_at: iso(row.created_at),
        })
    };
    let incoming: Vec<FriendRequest> = requests
        .iter()
        .filter(|row| row.to_profile == me)
        .filter_map(&request_of)
        .collect();
    let outgoing: Vec<FriendRequest> = requests
        .iter()
        .filter(|row| row.from_profile == me)
        .filter_map(&request_of)
        .collect();
    let blocked: Vec<PlayerRef> = blocks
        .iter()
        .filter_map(|row| profiles.get(&row.blocked).map(PlayerRef::of))
        .collect();

    Ok(crate::api::http::ok(json!({
        "friends": to_json(&friends)?,
        "incoming": to_json(&incoming)?,
        "outgoing": to_json(&outgoing)?,
        "blocked": to_json(&blocked)?,
    })))
}

/// `POST /api/friends/requests` `{username}` (B43, B44): `201 {requestId}`.
pub async fn post_request(app: &Arc<App>, req: Req) -> ApiResult {
    let me = caller_profile(&req)?.id;
    let typed = str(&req.body, "username")?;
    let now = now_ms();

    // B43: counted on every attempt, before the name is looked up.
    let key = account_key(&me);
    if !app.social.friend_requests.allow(&key, now) {
        return Err(rate_limited(
            "too many friend requests; try again later",
            app.social.friend_requests.retry_after_ms(&key, now),
        ));
    }

    let unknown = || not_found("no player has that username");
    let Some((name_key, tag)) = sign_in_name(&typed) else {
        return Err(unknown());
    };
    let mut tx = app.db.begin(Some(&me)).await?;
    let Some(target) = tx.profiles_get_by_username(&name_key, tag).await? else {
        return Err(unknown());
    };
    if target.id == me {
        return refused(409, "self", "That's you.");
    }
    // B43, B46: a block either way is an unknown name.
    if tx.blocks_between(&me, &target.id).await? {
        return Err(unknown());
    }
    if tx.friends_get(&me, &target.id).await?.is_some() {
        return refused(409, "already_friends", "You're already friends.");
    }
    if tx.friends_request_between(&me, &target.id).await?.is_some() {
        return refused(409, "already_requested", "You've already sent them a request.");
    }
    if tx.friends_count(&me).await? >= MAX_FRIENDS {
        return refused(409, "friend_cap", "Your friend list is full.");
    }

    // B44: they asked first, so this is their request accepted.
    if let Some(theirs) = tx.friends_request_between(&target.id, &me).await? {
        if tx.friends_count(&target.id).await? >= MAX_FRIENDS {
            return refused(409, "friend_cap", "Their friend list is full.");
        }
        tx.friends_make(&me, &target.id, now).await?;
        tx.commit().await?;
        log_info(
            "friends.added",
            json!({ "profileId": me, "friendProfileId": target.id, "mutual": true }),
        );
        return Ok(created(json!({ "requestId": theirs.id })));
    }

    let request = FriendRequestRow {
        id: new_uuid(),
        from_profile: me.clone(),
        to_profile: target.id.clone(),
        created_at: now,
    };
    if !tx.friends_request_create(&request).await? {
        return refused(409, "already_requested", "You've already sent them a request.");
    }
    tx.commit().await?;
    log_info(
        "friends.requested",
        json!({ "requestId": request.id, "fromProfileId": me, "toProfileId": target.id }),
    );
    Ok(created(json!({ "requestId": request.id })))
}

/// `POST /api/friends/requests/:id/accept` (B45): `200 {friend}`. Only the request's recipient may
/// accept it; anyone else, and an id no request holds, is 404. Past `MAX_FRIENDS` on either side,
/// 409 `friend_cap`, and the request stays.
pub async fn accept_request(app: &Arc<App>, req: Req) -> ApiResult {
    let me = caller_profile(&req)?.id;
    let request_id = param(&req, "id");
    let now = now_ms();

    let mut tx = app.db.begin(Some(&me)).await?;
    let request = match tx.friends_request_get(&request_id).await? {
        Some(request) if request.to_profile == me => request,
        _ => return Err(not_found("no such friend request")),
    };
    let sender_id = request.from_profile.clone();
    if tx.blocks_between(&me, &sender_id).await? {
        return Err(not_found("no such friend request"));
    }
    if tx.friends_count(&me).await? >= MAX_FRIENDS {
        return refused(409, "friend_cap", "Your friend list is full.");
    }
    if tx.friends_count(&sender_id).await? >= MAX_FRIENDS {
        return refused(409, "friend_cap", "Their friend list is full.");
    }
    let friendship = tx.friends_make(&me, &sender_id, now).await?;
    let Some(sender) = tx.profiles_get_by_id(&sender_id).await? else {
        return Err(not_found("no such friend request"));
    };
    let standing = standings(&mut tx, app, &me, std::slice::from_ref(&sender)).await?;
    tx.commit().await?;
    log_info(
        "friends.added",
        json!({ "profileId": me, "friendProfileId": sender.id, "mutual": false }),
    );
    let friend = Friend::of(&sender, friendship.since, standing.get(&sender.id));
    Ok(crate::api::http::ok(json!({ "friend": to_json(&friend)? })))
}

/// `POST /api/friends/requests/:id/decline` (B45): its recipient deletes it. 204, or 404.
pub async fn decline_request(app: &Arc<App>, req: Req) -> ApiResult {
    remove_request(app, req, |request, me| request.to_profile == me).await
}

/// `DELETE /api/friends/requests/:id` (B45): its sender cancels it. 204, or 404.
pub async fn cancel_request(app: &Arc<App>, req: Req) -> ApiResult {
    remove_request(app, req, |request, me| request.from_profile == me).await
}

/// Deletes the request `:id` when `mine` says the caller holds the side that may.
async fn remove_request(app: &App, req: Req, mine: fn(&FriendRequestRow, &str) -> bool) -> ApiResult {
    let me = caller_profile(&req)?.id;
    let request_id = param(&req, "id");
    let mut tx = app.db.begin(Some(&me)).await?;
    match tx.friends_request_get(&request_id).await? {
        Some(request) if mine(&request, &me) => {
            tx.friends_request_delete(&request.id).await?;
        }
        _ => return Err(not_found("no such friend request")),
    }
    tx.commit().await?;
    Ok(no_content())
}

/// `DELETE /api/friends/:profileId` (B45): ends the friendship, if there is one, and with it the
/// replay shares between the two (B52) and a watch of either by the other (B50). 204 either way.
pub async fn remove_friend(app: &Arc<App>, req: Req) -> ApiResult {
    let me = caller_profile(&req)?.id;
    let friend_id = param(&req, "profileId");
    let mut tx = app.db.begin(Some(&me)).await?;
    let removed = tx.friends_remove(&me, &friend_id).await?;
    let shares = tx.shares_remove_between(&me, &friend_id).await?;
    tx.commit().await?;
    // After the commit, so a hello judged from here on is refused and one judged before is revoked.
    let watchers = revoke_between(app, &me, &friend_id);
    if removed {
        log_info(
            "friends.removed",
            json!({
                "profileId": me,
                "friendProfileId": friend_id,
                "sharesRemoved": shares,
                "watchersRevoked": watchers,
            }),
        );
    }
    Ok(no_content())
}

/// `POST /api/blocks` `{profileId}` (B46): 204. Blocking oneself is 400; an id no profile holds,
/// 404. Blocking again changes nothing.
pub async fn post_block(app: &Arc<App>, req: Req) -> ApiResult {
    let me = caller_profile(&req)?.id;
    let target = str(&req.body, "profileId")?;
    if target == me {
        return Err(bad_request("you cannot block yourself"));
    }
    let now = now_ms();

    let mut tx = app.db.begin(Some(&me)).await?;
    if tx.profiles_find(&target).await?.is_none() {
        return Err(not_found("no such player"));
    }
    tx.blocks_add(&me, &target, now).await?;
    tx.friends_remove(&me, &target).await?;
    tx.friends_requests_delete_between(&me, &target).await?;
    // B46, B52: a share either way ends for good, so unblocking restores none.
    let shares = tx.shares_remove_between(&me, &target).await?;
    // B46, B49: an open challenge either way is cancelled and its room freed. One that lapsed
    // already freed itself: its room expired with it (and its code may be another room's now).
    let closed = tx
        .challenges_close_between(&me, &target, ChallengeState::Cancelled)
        .await?;
    for challenge in closed.iter().filter(|challenge| challenge.expires_at > now) {
        release_room(&mut tx, &challenge.room_code).await?;
    }
    tx.commit().await?;
    // B46, B50: and a watch either way ends at once.
    let watchers = revoke_between(app, &me, &target);
    log_info(
        "blocks.added",
        json!({
            "profileId": me,
            "blockedProfileId": target,
            "challengesClosed": closed.len(),
            "sharesRemoved": shares,
            "watchersRevoked": watchers,
        }),
    );
    Ok(no_content())
}

/// `DELETE /api/blocks/:profileId` (B46): lifts the caller's block, if there is one. 204 either
/// way. The friendship it ended stays ended.
pub async fn delete_block(app: &Arc<App>, req: Req) -> ApiResult {
    let me = caller_profile(&req)?.id;
    let target = param(&req, "profileId");
    let mut tx = app.db.begin(Some(&me)).await?;
    tx.blocks_remove(&me, &target).await?;
    tx.commit().await?;
    Ok(no_content())
}
