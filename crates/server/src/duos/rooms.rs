//! Duos co-op rooms (fullsend SPEC S6, B33, B34, B42): `POST /api/duos/rooms`, `GET` and `DELETE`
//! `/api/duos/rooms/:code`, `POST /api/duos/rooms/:code/join` and `GET /api/duos/live`, every one
//! `AuthLevel::Active`.
//!
//! A room is the 1v1 room's twin (`actor/rooms.rs`): a code of `ROOM_CODE_LENGTH` from the room-code
//! alphabet (R79, R104), minted by retrying a bounded number of times (R149), the host's deck or
//! random choice frozen when it is made (R253, through the queue's own `freeze_choice`), and a join
//! that claims it in one statement, so the loser of a race is told the room is full and never gets a
//! second match. The join starts the match: the host is a1 and the guest a2, the AI plays b1 and b2
//! (`duos::engine::match_options`), the row and the claim are written in one transaction, and the
//! actor starts on the begun state (`duos::registry`). Both people learn the match id: the guest from
//! the join, the host from the room (or `GET /api/duos/live`).
//!
//! A person in a 1v1 match, a Conquest series that is not over or a live co-op match can neither make
//! nor join a room, and a join is refused while the host is in one: the same `assert_free_to_play`
//! and `assert_host_free` (`api::decks`, R1065000) every 1v1 door asks. The join's claim cancels both
//! people's open tickets, as a series' start does. Co-op games are unranked: no rating moves and no
//! results row is written; the result lives on the match's own row (B40).
//!
//! A code is no key past a block (§9.4, §13.9): to an account the host has blocked, or that has
//! blocked the host, the room answers as an unknown code does, to read and to join. Once joined, a
//! room answers its host and its guest alone; anyone else reads it as unknown (a join still says it
//! is full, S6). A host's frozen deck is checked against the catalog it was frozen under: a join
//! under another catalog is `409 update_required`, as a 1v1 room's (§9.4: a stale catalog is
//! refused at every door into a match), and the match is pinned to the catalog it is dealt from
//! (R767000, `DuosMatchOptions::catalog_version`).
//!
//! S6's two join refusals keep the error body every route answers with, the code being S6's:
//! `409 { "error": { "code": "full" | "own_room", "message" } }`.

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::{Arc, LazyLock, Mutex};

use axum::body::Body;
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::Response;
use indexmap::IndexMap;
use serde_json::{Value, json};

use jackioh_engine::config::Difficulty;

use crate::actor::contracts::one_tx;
use crate::actor::registry::panic_text;
use crate::api::collection::caller_profile;
use crate::api::crypto::{is_well_formed_code, normalize_code, random_code};
use crate::api::decks::{
    FrozenChoice, ModeChoiceInput, assert_free_to_play, assert_host_free, freeze_choice, lean_of,
    read_mode_choice,
};
use crate::api::http::{
    ApiError, ApiErrorCode, ApiResult, Req, bad_request, json as json_response, lock, ok,
};
use crate::api::queue::{new_seed, new_uuid, seed_override_of};
use crate::app::{App, now_ms};
use crate::config::{DUOS_ROOM_TTL_MS, MAX_OPEN_ROOMS_PER_HOST, ROOM_CODE_LENGTH};
use crate::duos::engine::{self, Pinned};
use crate::duos::live_match_of;
use crate::duos::registry::registry;
use crate::duos::store::{DuosChoice, DuosMatch, DuosRoom};

/// R149's bound, as the 1v1 room's: a fresh code is tried this many times against the codes still
/// held before the create reports that none is free.
const CODE_ATTEMPTS: usize = 8;

// ---------------------------------------------------------------------------
// R143: the end-to-end mode's optional seed, held from the create to the join
// ---------------------------------------------------------------------------

/// One remembered seed and the expiry of the room it belongs to.
#[derive(Clone, Debug)]
struct HeldSeed {
    seed: String,
    expires_at: i64,
}

/// SPEC §11 R143, as `actor/rooms.rs` keeps it for a 1v1 room: a seeded end-to-end room's seed waits
/// here, by code, until its join (a room row carries no seed). `seed_override_of` refuses the field
/// outside end-to-end mode, so a production process keeps this map empty.
static E2E_SEED_BY_ROOM: LazyLock<Mutex<IndexMap<String, HeldSeed>>> =
    LazyLock::new(|| Mutex::new(IndexMap::new()));

fn remember_seed(code: &str, seed: &str, expires_at: i64, now: i64) {
    let mut held = lock(&E2E_SEED_BY_ROOM);
    held.retain(|_, entry| entry.expires_at > now);
    held.insert(
        code.to_string(),
        HeldSeed {
            seed: seed.to_string(),
            expires_at,
        },
    );
}

/// The room's seed, consumed; the host's wins over the joiner's.
fn take_seed(code: &str, joiner_seed: Option<String>) -> Option<String> {
    match lock(&E2E_SEED_BY_ROOM).shift_remove(code) {
        Some(entry) => Some(entry.seed),
        None => joiner_seed,
    }
}

fn forget_seed(code: &str) {
    lock(&E2E_SEED_BY_ROOM).shift_remove(code);
}

// ---------------------------------------------------------------------------
// Reading a request
// ---------------------------------------------------------------------------

/// S6: `difficulty`, one of §9.9's three tiers.
fn read_difficulty(body: &Value) -> Result<Difficulty, ApiError> {
    body.get("difficulty")
        .and_then(Value::as_str)
        .and_then(|text| text.parse::<Difficulty>().ok())
        .ok_or_else(|| bad_request(r#""difficulty" must be "easy", "medium" or "hard""#))
}

/// S6: exactly one of `deckId` (one of the caller's saved decks) and `random: true` (with its "More
/// cards from" pick, R924000's `leanSet` or R1372's `leanNewest`), read as the queue reads a Best-of-1
/// or an All Random choice.
fn read_choice(body: &Value) -> Result<ModeChoiceInput, ApiError> {
    let deck_id = body.get("deckId").filter(|value| !value.is_null());
    let random = match body.get("random") {
        None | Some(Value::Null) => false,
        Some(Value::Bool(random)) => *random,
        Some(_) => return Err(bad_request(r#""random" must be true or false"#)),
    };
    match (deck_id, random) {
        (Some(deck_id), false) => read_mode_choice(&json!({ "mode": "bo1", "deckId": deck_id })),
        (None, true) => Ok(ModeChoiceInput::Random { lean: lean_of(body)? }),
        _ => Err(bad_request(
            r#"send exactly one of "deckId" (one of your decks) and "random": true"#,
        )),
    }
}

/// R253: the deck is checked legal now, under the running catalog, and frozen with that catalog's
/// version; a random choice keeps its lean for the deal.
async fn freeze(app: &App, profile_id: &str, choice: &ModeChoiceInput) -> Result<DuosChoice, ApiError> {
    match freeze_choice(app, profile_id, choice).await? {
        FrozenChoice::Bo1 { deck } => Ok(DuosChoice::Deck {
            deck,
            catalog_version: app.catalog.version.clone(),
        }),
        FrozenChoice::Random { lean } => Ok(DuosChoice::Random { lean }),
        FrozenChoice::Bo3 { .. } => Err(ApiError::internal("a trio reached a co-op room")),
    }
}

/// S6's 409: the error body every route answers with, under S6's code.
fn refusal(status: u16, code: &str, message: &str) -> Response {
    json_response(status, json!({ "error": { "code": code, "message": message } }))
}

/// 204, no body.
fn no_content() -> Response {
    let mut response = Response::new(Body::empty());
    *response.status_mut() = StatusCode::NO_CONTENT;
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

/// The code a path names, normalised, or none when it could never be a room code.
fn code_of(req: &Req) -> Option<String> {
    let code = normalize_code(req.params.get("code").map(String::as_str).unwrap_or(""));
    is_well_formed_code(&code, ROOM_CODE_LENGTH).then_some(code)
}

fn not_open() -> ApiError {
    ApiError::new(ApiErrorCode::NotFound, "that co-op room is not open")
}

/// A room `caller` may read or join: joined, or unjoined and unexpired (B34), and its host and the
/// caller not blocked either way (§13.9). Missing, malformed, expired and blocked codes are one
/// answer, so a scan cannot tell "wrong" from "too late", nor a blocked caller that it is one.
async fn room_of(app: &App, req: &Req, caller: &str) -> Result<DuosRoom, ApiError> {
    let Some(code) = code_of(req) else {
        return Err(not_open());
    };
    let (room, blocked) = one_tx!(app.db, |t| {
        let room = t.duos_rooms_get(&code).await?;
        let blocked = match &room {
            Some(room) if room.host != caller => t.blocks_between(&room.host, caller).await?,
            _ => false,
        };
        (room, blocked)
    })?;
    match room {
        Some(room) if !blocked && (room.guest.is_some() || room.expires_at > now_ms()) => Ok(room),
        _ => Err(not_open()),
    }
}

// ---------------------------------------------------------------------------
// The routes
// ---------------------------------------------------------------------------

/// `POST /api/duos/rooms` (B33): a room the caller holds, its code `201 { code }`.
pub async fn create(app: &Arc<App>, req: Req) -> ApiResult {
    let profile = caller_profile(&req)?;
    let difficulty = read_difficulty(&req.body)?;
    let choice = read_choice(&req.body)?;
    // R143: read before any work, so a production caller gets its 400 without a room being made.
    let seed = seed_override_of(app, &req.body)?;
    assert_free_to_play(app, &profile.id).await?;
    let host_choice = freeze(app, &profile.id, &choice).await?;

    let now = now_ms();
    let expires_at = now + DUOS_ROOM_TTL_MS;

    // R890000, R1047002: one host holds at most `MAX_OPEN_ROOMS_PER_HOST` co-op rooms nobody has
    // joined, counted apart from their 1v1 rooms, so its rooms cannot amplify a seat race without
    // bound. Advisory: the claim in `join` is what stops a second match, not this count.
    let open = one_tx!(app.db, |t| t
        .duos_rooms_count_open_by_host(&profile.id, now)
        .await?)?;
    if open >= MAX_OPEN_ROOMS_PER_HOST {
        return Err(ApiError::new(
            ApiErrorCode::Conflict,
            "you have too many co-op rooms open; wait for someone to join or let one expire",
        ));
    }

    for _attempt in 0..CODE_ATTEMPTS {
        let code = random_code(ROOM_CODE_LENGTH);
        let room = DuosRoom {
            code: code.clone(),
            host: profile.id.clone(),
            guest: None,
            difficulty,
            host_choice: host_choice.clone(),
            guest_choice: None,
            match_id: None,
            created_at: now,
            expires_at,
        };
        let created = one_tx!(app.db, |t| t.duos_rooms_create(&room).await?)?;
        if !created {
            continue;
        }
        if let Some(seed) = seed.as_deref() {
            remember_seed(&code, seed, expires_at, now);
        }
        tracing::info!(event = "duos.room.created", code = %code, hostProfileId = %profile.id, difficulty = %difficulty);
        return Ok(json_response(201, json!({ "code": code })));
    }
    tracing::error!(event = "duos.room.code.exhausted", attempts = CODE_ATTEMPTS);
    Err(ApiError::new(
        ApiErrorCode::Unavailable,
        "could not allocate a room code; try again",
    ))
}

/// `GET /api/duos/rooms/:code`: the room as its lobby shows it, the match id once the guest joined.
pub async fn get_room(app: &Arc<App>, req: Req) -> ApiResult {
    let profile = caller_profile(&req)?;
    let room = room_of(app, &req, &profile.id).await?;
    // §13.9: a joined room is its two people's; to anyone else it is as gone as an unknown code.
    let member = room.host == profile.id || room.guest.as_deref() == Some(profile.id.as_str());
    if room.guest.is_some() && !member {
        return Err(not_open());
    }
    let ids: Vec<String> = std::iter::once(room.host.clone())
        .chain(room.guest.clone())
        .collect();
    let profiles = one_tx!(app.db, |t| t.profiles_get_many(&ids).await?)?;
    // R1436: a player is shown by profile id and username, as every other read shows one.
    let player = |id: &str| {
        profiles
            .iter()
            .find(|profile| profile.id == id)
            .map(|profile| json!({ "profileId": profile.id, "username": profile.username() }))
            .unwrap_or(Value::Null)
    };
    Ok(ok(json!({
        "code": room.code,
        "difficulty": room.difficulty,
        "host": player(&room.host),
        "guest": room.guest.as_deref().map(&player).unwrap_or(Value::Null),
        "matchId": room.match_id,
    })))
}

/// `POST /api/duos/rooms/:code/join` (B33): claims the room and starts the match, `200 { matchId }`.
/// A room already joined is `409 full`; the host's own room `409 own_room`.
pub async fn join(app: &Arc<App>, req: Req) -> ApiResult {
    let profile = caller_profile(&req)?;
    let choice = read_choice(&req.body)?;
    let joiner_seed = seed_override_of(app, &req.body)?;
    let room = room_of(app, &req, &profile.id).await?;
    if room.host == profile.id {
        return Ok(refusal(
            409,
            "own_room",
            "you made that room; wait for your teammate to join",
        ));
    }
    if room.guest.is_some() {
        return Ok(refusal(409, "full", "someone already joined that room"));
    }
    // §9.4: a stale catalog is refused at every door into a match. The host's deck was checked
    // against the catalog it was frozen under, which may not deal it now.
    if let DuosChoice::Deck { catalog_version, .. } = &room.host_choice
        && *catalog_version != app.catalog.version
    {
        return Err(ApiError::with_details(
            ApiErrorCode::UpdateRequired,
            "update required",
            json!({ "expected": app.catalog.version, "received": catalog_version }),
        ));
    }
    assert_free_to_play(app, &profile.id).await?;
    let guest_choice = freeze(app, &profile.id, &choice).await?;
    assert_host_free(app, &room.host).await?;

    // Everything the match is created with is settled, and the engine has begun it, before anything is
    // written: a setup the engine refuses leaves the room open and nothing behind. The match is
    // pinned to the running catalog (R767000).
    let catalog_version = app.catalog.version.clone();
    let pinned = Pinned::of(&catalog_version).map_err(|reason| {
        ApiError::internal(format!(
            "the running catalog {catalog_version} cannot be played: {reason}"
        ))
    })?;
    let match_id = new_uuid();
    let seed = take_seed(&room.code, joiner_seed).unwrap_or_else(new_seed);
    let options = catch_unwind(AssertUnwindSafe(|| {
        engine::match_options(
            pinned,
            &catalog_version,
            &seed,
            room.difficulty,
            &room.host_choice,
            &guest_choice,
        )
    }))
    .map_err(|payload| ApiError::internal(panic_text(payload)))?;
    let now = now_ms();
    let row = DuosMatch {
        id: match_id.clone(),
        seed,
        a1: room.host.clone(),
        a2: profile.id.clone(),
        difficulty: room.difficulty,
        options,
        created_at: now,
        ended_at: None,
        result: None,
    };
    let state = catch_unwind(AssertUnwindSafe(|| {
        engine::create_and_begin(pinned, &engine::create_options(&row))
    }))
    .map_err(|payload| ApiError::internal(panic_text(payload)))?
    .map_err(ApiError::internal)?;

    // B33's single claim and the match row, in one transaction: the loser of a race writes nothing.
    let claimed = one_tx!(app.db, |t| {
        let claimed = t
            .duos_rooms_claim(&room.code, &profile.id, &guest_choice, &match_id, now)
            .await?;
        if claimed.is_some() {
            t.duos_matches_create(&row).await?;
            // R1065000: a player in a co-op match is in no queue, as one in a series is: a ticket
            // left open would let the sweep pair them while the match lasts.
            for profile_id in [&row.a1, &row.a2] {
                if let Some(stale) = t.tickets_open_for_profile(profile_id).await? {
                    t.tickets_cancel(&stale.id, now).await?;
                }
            }
        }
        claimed
    })?;
    if claimed.is_none() {
        return Ok(refusal(409, "full", "someone already joined that room"));
    }
    registry(app).start(app, row, state, pinned);
    tracing::info!(
        event = "duos.room.joined",
        code = %room.code,
        matchId = %match_id,
        guestProfileId = %profile.id,
    );
    Ok(ok(json!({ "matchId": match_id })))
}

/// `DELETE /api/duos/rooms/:code` (B34): the host frees a room nobody has joined, `204`. Another
/// caller's room, or none, is 404; a room already joined is `409 full`.
pub async fn cancel(app: &Arc<App>, req: Req) -> ApiResult {
    let profile = caller_profile(&req)?;
    let Some(code) = code_of(&req) else {
        return Err(not_open());
    };
    let room = one_tx!(app.db, |t| t.duos_rooms_get(&code).await?)?;
    let Some(room) = room.filter(|room| room.host == profile.id) else {
        return Err(not_open());
    };
    if room.guest.is_some() {
        return Ok(refusal(409, "full", "someone already joined that room"));
    }
    let deleted = one_tx!(app.db, |t| t.duos_rooms_delete(&code, &profile.id).await?)?;
    if !deleted {
        // A guest's claim landed between the read and the delete.
        return Ok(refusal(409, "full", "someone already joined that room"));
    }
    forget_seed(&code);
    tracing::info!(event = "duos.room.cancelled", code = %code);
    Ok(no_content())
}

/// `GET /api/duos/live` (B42): the caller's live co-op match, so the shell can offer the way back.
pub async fn live(app: &Arc<App>, req: Req) -> ApiResult {
    let profile = caller_profile(&req)?;
    let match_id = live_match_of(app, &profile.id).await?;
    Ok(ok(json!({ "matchId": match_id })))
}
