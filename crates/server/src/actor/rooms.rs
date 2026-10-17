//! The room-code challenge (SPEC §9.5, R79, BUILD M6-T4), in the queue's three modes (R264).
//!
//! §9.5: direct challenge by a 6-character room code from the invite-code alphabet. The queue's
//! rules apply too, being the same assertions §9.5 makes about entering a match:
//!
//!  - the account is active (`AuthLevel::Active`, §9.4), not in a match, a series that is not over
//!    (R264) or a live co-op match (R1065000);
//!  - the deck or trio is validated at match time by the shared validator (§9.4, R253), through the
//!    queue's `freeze_choice`, and frozen into the room at creation (§9.4, §9.5, §9.8);
//!  - `rooms_claim` is the atomic single-claim: the loser of a join race gets a 409, never a second
//!    match.
//!
//! R264: a room is created in a mode, and a joiner plays that mode or is refused with it named. A
//! Best-of-1 join starts the match on the two frozen decks; an All Random join deals both decks
//! (R258); a Conquest join makes the series (R259), whose first game starts once both players have
//! picked a deck. Handlers: `create` (`POST /api/rooms`), `join` (`POST /api/rooms/:code/join`),
//! and, R1075000, `mine` (`GET /api/rooms`) and `cancel` (`DELETE /api/rooms/:code`).
//! Surface contract: docs/v0.3.0/SURFACE.md §4.1.

use std::cell::RefCell;
use std::sync::{Arc, LazyLock, Mutex};

use indexmap::IndexMap;
use serde_json::{Value, json};

use jackioh_engine::{PlayerId, pick_portrait_from_seed};

use crate::actor::engine::{deal_random_deck, lean_of};
use crate::api::crypto::{is_well_formed_code, normalize_code, random_code};
use crate::api::decks::{
    FrozenChoice, ModeChoiceInput, assert_free_to_play, assert_host_free, freeze_choice, read_mode_choice,
};
use crate::api::http::{ApiError, ApiErrorCode, ApiResult, Req, json as json_response};
use crate::api::queue::{new_seed, new_uuid, seed_override_of};
use crate::api::series::start_series;
use crate::api::series_rules::{NewSeriesInput, NewSeriesSide};
// `now_ms`: the server's one clock (SURFACE §11.3), read through tokio's clock so a paused test
// moves every module's "now" together.
use crate::app::{App, now_ms};
use crate::config::{MAX_OPEN_ROOMS_PER_HOST, ROOM_CODE_LENGTH, ROOM_CODE_TTL_SECONDS};
use crate::db::store::{MatchSeat, QueueMode, Room, SeriesRow, StartMatchInput};

/// SPEC §11 R149: a room code "is minted by retrying a bounded number of times against the codes
/// still in use and then reporting that no code is available"; the bound is config's, like R79's
/// clocks. Eight is far more than a 30-bit space (R104) needs against the codes R110 has not released.
const CODE_ATTEMPTS: usize = 8;

// R143 — the end-to-end mode's optional seed, for the room half

/// One remembered seed and the expiry of the room it belongs to.
#[derive(Clone, Debug)]
struct HeldSeed {
    seed: String,
    expires_at: i64,
}

/// SPEC §11 R143: "In end-to-end mode the room and queue endpoints accept an optional seed and use
/// it verbatim so a networked spec can be seeded, and outside that mode the field is rejected."
///
/// A room's match is not created until someone joins, so the host's seed is held between the two
/// calls, `room code -> seed` as `api/queue.rs` holds `ticket id -> seed`: `Room` carries no seed,
/// and a test-mode field must not enter what `src/db/**` persists. `seed_override_of` has already
/// refused the field outside end-to-end mode, so a production process keeps this map empty.
static E2E_SEED_BY_ROOM: LazyLock<Mutex<IndexMap<String, HeldSeed>>> =
    LazyLock::new(|| Mutex::new(IndexMap::new()));

/// The map above, recovered from a poisoned lock (a panic elsewhere never loses the seeds).
fn seeds() -> std::sync::MutexGuard<'static, IndexMap<String, HeldSeed>> {
    match E2E_SEED_BY_ROOM.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

/// Exported for the R143 tests; nothing in `src/` calls it.
pub fn e2e_room_seed_count() -> usize {
    seeds().len()
}

/// A room never joined has no claim to clear it, so each remembered seed carries its room's expiry
/// and stale ones are dropped as new rooms are made.
fn remember_seed(code: &str, seed: &str, expires_at: i64, now: i64) {
    let mut held = seeds();
    held.retain(|_, entry| entry.expires_at > now);
    held.insert(
        code.to_string(),
        HeldSeed {
            seed: seed.to_string(),
            expires_at,
        },
    );
}

/// The seed for this room, consumed. The host's seed wins over the joiner's: the room came first.
fn take_seed_for_room(code: &str, joiner_seed: Option<String>) -> Option<String> {
    let held = seeds().shift_remove(code);
    match held {
        Some(entry) => Some(entry.seed),
        None => joiner_seed,
    }
}

// R149 — where a room code comes from

thread_local! {
    /// The test seam for R149's bound; absent, as outside a test, codes are random.
    static SCRIPTED_CODES: RefCell<Option<ScriptedCodes>> = const { RefCell::new(None) };
}

/// The codes a test scripted, and how many have been minted from them.
struct ScriptedCodes {
    codes: Vec<String>,
    minted: usize,
}

/// Scripts the room codes `create` mints on the calling thread, in order, the last repeating, until
/// the guard drops. A `#[tokio::test]` runs the handlers on its own thread, so only its requests see it.
pub fn script_room_codes(codes: &[&str]) -> ScriptedRoomCodes {
    let codes = codes.iter().map(|code| (*code).to_string()).collect();
    SCRIPTED_CODES.with(|scripted| *scripted.borrow_mut() = Some(ScriptedCodes { codes, minted: 0 }));
    ScriptedRoomCodes { _private: () }
}

/// `script_room_codes`' guard: dropped, the thread mints random codes again.
pub struct ScriptedRoomCodes {
    _private: (),
}

impl Drop for ScriptedRoomCodes {
    fn drop(&mut self) {
        SCRIPTED_CODES.with(|scripted| *scripted.borrow_mut() = None);
    }
}

/// R79, §9.5: a room code, 6 characters from the invite-code alphabet — or the next scripted one.
fn mint_code() -> String {
    SCRIPTED_CODES.with(|scripted| {
        let mut scripted = scripted.borrow_mut();
        match scripted.as_mut() {
            Some(script) if !script.codes.is_empty() => {
                let code = script.codes[script.minted.min(script.codes.len() - 1)].clone();
                script.minted += 1;
                code
            }
            _ => random_code(ROOM_CODE_LENGTH),
        }
    })
}

/// R264: a join in another mode than the room's is refused with the room's mode named, so the lobby
/// can ask for the right deck or trio (`details.mode` carries it for the client).
fn mode_refusal(mode: QueueMode) -> &'static str {
    match mode {
        QueueMode::Bo1 => "This room plays Best of 1: pick one of your decks.",
        QueueMode::Bo3 => "This room plays Conquest: pick one of your trios.",
        QueueMode::Random => "This room plays All Random: join it without a deck.",
    }
}

/// The mode a request's choice asks for.
fn choice_mode(choice: &ModeChoiceInput) -> QueueMode {
    match choice {
        ModeChoiceInput::Bo1 { .. } => QueueMode::Bo1,
        ModeChoiceInput::Bo3 { .. } => QueueMode::Bo3,
        ModeChoiceInput::Random { .. } => QueueMode::Random,
    }
}

/// The mode a frozen choice was made in.
fn frozen_mode(frozen: &FrozenChoice) -> QueueMode {
    match frozen {
        FrozenChoice::Bo1 { .. } => QueueMode::Bo1,
        FrozenChoice::Bo3 { .. } => QueueMode::Bo3,
        FrozenChoice::Random { .. } => QueueMode::Random,
    }
}

/// A wiring fault, never a player's: logged as `handler.threw`, answered 500 `internal`.
fn wiring_fault(message: String) -> ApiError {
    tracing::warn!(event = "handler.threw", message = %message);
    ApiError::new(ApiErrorCode::Internal, "something went wrong")
}

fn profile_of(req: &Req) -> Result<String, ApiError> {
    match req.caller.as_ref() {
        Some(caller) => Ok(caller.profile.id.clone()),
        // Unreachable on an `AuthLevel::Active` route; the router resolves the caller first.
        None => Err(ApiError::new(ApiErrorCode::Unauthorized, "sign in first")),
    }
}

/// A room the caller may still join. Missing, malformed and expired codes are one answer, so a
/// scan of the 30-bit code space cannot tell "wrong" from "too late" (§9.4's invite-code error).
async fn joinable_room(app: &App, typed: &str) -> Result<Room, ApiError> {
    let code = normalize_code(typed);
    let miss = || ApiError::new(ApiErrorCode::NotFound, "that room code is not open");
    if !is_well_formed_code(&code, ROOM_CODE_LENGTH) {
        return Err(miss());
    }

    let mut tx = app.db.begin(None).await?;
    let room = tx.rooms_get(&code).await?;
    tx.commit().await?;
    let Some(room) = room else {
        return Err(miss());
    };
    if room.expires_at <= now_ms() {
        return Err(miss());
    }
    if room.guest_profile_id.is_some() {
        return Err(ApiError::new(
            ApiErrorCode::Conflict,
            "someone already joined that room",
        ));
    }
    // §9.4: a stale catalog is rejected at every door into a match.
    if room.catalog_version != app.catalog.version {
        return Err(ApiError::new(ApiErrorCode::UpdateRequired, "update required"));
    }
    Ok(room)
}

/// The two seats of a room's match: the host is p1 (§9.5), the joiner p2.
fn room_seats(
    room: &Room,
    joiner_id: &str,
    joiner: &FrozenChoice,
    seed: &str,
) -> Result<(MatchSeat, MatchSeat), ApiError> {
    if room.mode == QueueMode::Random {
        // R258: dealt from the match seed and the seat, and frozen into the match row like any deck.
        // R642: the portraits are dealt the same way, uniformly and seat by seat. R1372, R924000:
        // each seat's deck leans on the set its own player picked, if any, the host when the room was
        // made and the joiner now.
        let joiner_lean = match joiner {
            FrozenChoice::Random { lean } => *lean,
            _ => None,
        };
        return Ok((
            MatchSeat {
                profile_id: room.host_profile_id.clone(),
                player: PlayerId::P1,
                deck: deal_random_deck(&format!("{seed}:p1-deck"), lean_of(room.host_lean)),
                portrait: Some(
                    pick_portrait_from_seed(&format!("{seed}:portrait:p1"))
                        .as_str()
                        .to_string(),
                ),
            },
            MatchSeat {
                profile_id: joiner_id.to_string(),
                player: PlayerId::P2,
                deck: deal_random_deck(&format!("{seed}:p2-deck"), lean_of(joiner_lean)),
                portrait: Some(
                    pick_portrait_from_seed(&format!("{seed}:portrait:p2"))
                        .as_str()
                        .to_string(),
                ),
            },
        ));
    }
    let FrozenChoice::Bo1 { deck } = joiner else {
        return Err(wiring_fault(format!(
            "a {} choice reached a Best-of-1 room",
            frozen_mode(joiner).as_str()
        )));
    };
    Ok((
        MatchSeat {
            profile_id: room.host_profile_id.clone(),
            player: PlayerId::P1,
            deck: room.host_deck.clone(),
            portrait: room.host_portrait.clone().flatten(),
        },
        MatchSeat {
            profile_id: joiner_id.to_string(),
            player: PlayerId::P2,
            deck: deck.cards.clone(),
            portrait: deck.portrait.clone().flatten(),
        },
    ))
}

/// POST /api/rooms — create a room in a mode and return its code (§9.5, R264).
pub async fn create(app: &Arc<App>, req: Req) -> ApiResult {
    // ---- Social: friends, presence, challenges, chat (#937, #938, #941) ----
    // The work is `open_room`'s, which a challenge (SPEC S7, B48) opens its room through too.
    let room = open_room(app, &req, ROOM_CODE_TTL_SECONDS * 1000).await?;
    Ok(json_response(
        200,
        json!({ "code": room.code, "expiresAt": room.expires_at, "mode": room.mode }),
    ))
}

/// A room `open_room` opened: its code, when it stops being joinable, and its mode.
pub struct OpenedRoom {
    pub code: String,
    pub expires_at: i64,
    pub mode: QueueMode,
}

/// `POST /api/rooms`' work on the request's caller and body, the room open for `ttl_ms` unjoined:
/// `ROOM_CODE_TTL_SECONDS` for a room, `CHALLENGE_TTL_MS` for a challenge's (SPEC S7, B48, B49),
/// which so lapses with its challenge. Every refusal is the route's own.
pub async fn open_room(app: &App, req: &Req, ttl_ms: i64) -> Result<OpenedRoom, ApiError> {
    // ---- end Social ----
    let profile_id = profile_of(req)?;
    // R257, R264: the same choice the queue takes.
    let choice = read_mode_choice(&req.body)?;
    // R143: an optional `seed`, accepted only by an end-to-end server and rejected elsewhere,
    // read before any work so a production caller gets the 400 without a room being made.
    let seed = seed_override_of(app, &req.body)?;
    assert_free_to_play(app, &profile_id).await?;

    // R253: checked now and frozen into the room — a deck, a trio, or nothing for All Random.
    let frozen = freeze_choice(app, &profile_id, &choice).await?;
    let mode = frozen_mode(&frozen);

    let now = now_ms();
    let expires_at = now + ttl_ms;

    // R890000: one host holds at most `MAX_OPEN_ROOMS_PER_HOST` unclaimed unexpired rooms, so its
    // rooms cannot amplify a seat race without bound. Advisory: the seat compare-and-set in `join`
    // is what stops a double seat, not this count.
    {
        let mut tx = app.db.begin(None).await?;
        let open = tx.rooms_count_open_by_host(&profile_id, now).await?;
        tx.commit().await?;
        if open >= MAX_OPEN_ROOMS_PER_HOST {
            return Err(ApiError::new(
                ApiErrorCode::Conflict,
                "you have too many rooms open; wait for someone to join or let one expire",
            ));
        }
    }

    for _attempt in 0..CODE_ATTEMPTS {
        // R79, §9.5: 6 characters from the invite-code alphabet.
        let code = mint_code();
        let room = Room {
            code: code.clone(),
            host_profile_id: profile_id.clone(),
            mode,
            // §9.4, §9.5: the deck or the trio is frozen here, exactly as it is into a queue ticket.
            host_deck: match &frozen {
                FrozenChoice::Bo1 { deck } => deck.cards.clone(),
                _ => Vec::new(),
            },
            host_portrait: match &frozen {
                FrozenChoice::Bo1 { deck } => Some(deck.portrait.clone().flatten()),
                _ => Some(None),
            },
            host_trio: match &frozen {
                FrozenChoice::Bo3 { trio } => Some(trio.clone()),
                _ => None,
            },
            // R1372, R924000: an All Random host's lean waits with the room for the deal at the join.
            host_lean: match &frozen {
                FrozenChoice::Random { lean } => *lean,
                _ => None,
            },
            catalog_version: app.catalog.version.clone(),
            created_at: now,
            expires_at,
            guest_profile_id: None,
            match_id: None,
        };
        let mut tx = app.db.begin(None).await?;
        let created = tx.rooms_create(&room).await?;
        tx.commit().await?;
        if !created {
            continue;
        }
        // R143, after the create won: a seed for a room that does not exist would never be dropped.
        if let Some(seed) = seed.as_deref() {
            remember_seed(&code, seed, expires_at, now);
        }
        tracing::info!(event = "room.created", code = %code, hostProfileId = %profile_id, mode = mode.as_str());
        return Ok(OpenedRoom {
            code,
            expires_at,
            mode,
        });
    }

    tracing::error!(event = "room.code.exhausted", attempts = CODE_ATTEMPTS);
    Err(ApiError::new(
        ApiErrorCode::Unavailable,
        "could not allocate a room code; try again",
    ))
}

/// POST /api/rooms/:code/join — claim the room and start its game (§9.5, R264): the match for
/// Best of 1 and All Random, the series for Conquest.
///
/// Takes `&Arc<App>` because `Registry::start` does (as `api::queue::enqueue`); SURFACE §11.2's
/// `&App` does not fit.
pub async fn join(app: &Arc<App>, req: Req) -> ApiResult {
    let profile_id = profile_of(&req)?;
    let choice = read_mode_choice(&req.body)?;
    // R143 again: both endpoints accept the field in end-to-end mode and refuse it outside one.
    let joiner_seed = seed_override_of(app, &req.body)?;
    assert_free_to_play(app, &profile_id).await?;

    let typed = req.params.get("code").cloned().unwrap_or_default();
    let room = joinable_room(app, &typed).await?;
    if room.host_profile_id == profile_id {
        return Err(ApiError::new(
            ApiErrorCode::Conflict,
            "you created that room; wait for someone to join",
        ));
    }
    // ---- Social: friends, presence, challenges, chat (#937, #938, #941) ----
    // SPEC S7, B49: a room a challenge opened is the challenged friend's to join.
    crate::social::challenges::assert_may_join(app, &room.code, &profile_id).await?;
    // ---- end Social ----
    // R264: the room's mode, or a refusal naming it — before the joiner's deck is looked at, so the
    // answer is the useful one.
    if choice_mode(&choice) != room.mode {
        return Err(ApiError::with_details(
            ApiErrorCode::Conflict,
            mode_refusal(room.mode),
            json!({ "mode": room.mode }),
        ));
    }

    let frozen = freeze_choice(app, &profile_id, &choice).await?;
    assert_host_free(app, &room.host_profile_id).await?;

    let match_id = new_uuid();
    let now = now_ms();

    // The atomic single-claim: whoever wins this statement is the guest, and there is no second
    // winner (§9.5, mirroring `tickets_claim_pair`).
    let mut tx = app.db.begin(None).await?;
    let claimed = tx.rooms_claim(&room.code, &profile_id, &match_id, now).await?;
    // ---- Social: friends, presence, challenges, chat (#937, #938, #941) ----
    // SPEC S7, B49: accepting a challenge is joining its room, so the claim accepts it, in the
    // claim's own transaction.
    if claimed.is_some() {
        tx.challenges_accept_room(&room.code, &profile_id, now).await?;
    }
    // ---- end Social ----
    tx.commit().await?;
    let Some(claimed) = claimed else {
        return Err(ApiError::new(
            ApiErrorCode::Conflict,
            "someone already joined that room",
        ));
    };
    // R143: the server mints the seed, unless an end-to-end room asked for one.
    let seed = match take_seed_for_room(&claimed.code, joiner_seed) {
        Some(seed) => seed,
        None => new_seed(),
    };

    if let FrozenChoice::Bo3 { trio } = &frozen {
        // R259, R263: the series, with the host as series p1 and the id the claim reserved as game
        // 1's. Nobody is in a match yet: the series opens on its pick phase.
        let Some(host_trio) = claimed.host_trio.clone() else {
            return Err(wiring_fault(format!(
                "Conquest room {} holds no trio",
                claimed.code
            )));
        };
        let input = NewSeriesInput {
            series_id: new_uuid(),
            first_match_id: match_id.clone(),
            sides: (
                NewSeriesSide {
                    profile_id: claimed.host_profile_id.clone(),
                    trio: host_trio,
                },
                NewSeriesSide {
                    profile_id: profile_id.clone(),
                    trio: trio.clone(),
                },
            ),
            seed_base: seed.clone(),
            catalog_version: app.catalog.version.clone(),
            // R604: a room's series is unranked.
            ranked: false,
        };
        // One transaction, so a half-landed 'picking' row cannot hold both players out of the
        // queue and the room until the pick deadline (R333).
        let started: Result<SeriesRow, ApiError> = async {
            let mut tx = app.db.begin(None).await?;
            let series = start_series(app, input, &mut tx).await?;
            tx.commit().await?;
            Ok(series)
        }
        .await;
        let series = match started {
            Ok(series) => series,
            Err(error) => {
                // The claim has committed, so the room's `open` row would stay claimed and
                // pointing at no series; `matches_discard_open` releases it and frees the room code.
                let cleanup: Result<(), ApiError> = async {
                    let mut tx = app.db.begin(None).await?;
                    tx.matches_discard_open(&match_id).await?;
                    tx.commit().await?;
                    Ok(())
                }
                .await;
                if let Err(cleanup_error) = cleanup {
                    tracing::error!(
                        event = "room.series_cleanup_failed",
                        matchId = %match_id,
                        message = %cleanup_error.message,
                    );
                }
                return Err(error);
            }
        };
        tracing::info!(
            event = "room.joined",
            code = %claimed.code,
            mode = claimed.mode.as_str(),
            seriesId = %series.id,
            firstMatchId = %match_id,
            guestProfileId = %profile_id,
        );
        return Ok(json_response(
            200,
            json!({
                "matchId": Value::Null,
                "seriesId": series.id,
                "code": claimed.code,
                "seat": "p2",
                "mode": claimed.mode,
            }),
        ));
    }

    let seats = room_seats(&claimed, &profile_id, &frozen, &seed)?;

    // §9.5: the in-match flag both ends of the lifecycle read ("not in a match" above, and
    // "clears both players' in-match state" when the result lands). Both seats are claimed
    // compare-and-set (R890000) before the start, while the claim's renamed `open` row satisfies
    // the `current_match_id` foreign key, as the queue path's comment states: a queue sweep that
    // seated a seat between `assert_host_free` and now loses one seat here instead of seating
    // them twice. Dropping `tx` on a loss rolls both stores back, and `matches_discard_open`
    // deletes the claim and frees the room code, so the room stays joinable.
    {
        let mut tx = app.db.begin(None).await?;
        let host_seated = tx
            .profiles_set_in_match(&claimed.host_profile_id, Some(&match_id))
            .await?;
        let guest_seated = host_seated && tx.profiles_set_in_match(&profile_id, Some(&match_id)).await?;
        if host_seated && guest_seated {
            tx.commit().await?;
        } else {
            // Before the commit, so nothing was written — but the claim above did commit, so
            // delete it and free the room code: the room stays joinable. Dropped first: the
            // fake store's lock is held by this transaction, and the cleanup begins its own.
            drop(tx);
            let mut cleanup = app.db.begin(None).await?;
            cleanup.matches_discard_open(&match_id).await?;
            cleanup.commit().await?;
            if host_seated {
                return Err(ApiError::new(
                    ApiErrorCode::AlreadyInMatch,
                    "finish your current match first",
                ));
            }
            return Err(ApiError::new(
                ApiErrorCode::Conflict,
                "The host of that room is playing another game right now.",
            ));
        }
    }
    if let Err(error) = app
        .matches
        .start(
            app,
            StartMatchInput {
                match_id: match_id.clone(),
                seed: seed.clone(),
                catalog_version: app.catalog.version.clone(),
                // R604: a room challenge is unranked: it moves neither rating nor rank.
                ranked: false,
                seats,
                mode: None,
                stake: None,
            },
        )
        .await
    {
        // The seats committed, so a failed start would lock both players out until the reaper's
        // open-row sweep (R1450). Undo it now: conditional clears (R890000) that never wipe a
        // rival flag, then the claim's row.
        let cleanup: Result<(), ApiError> = async {
            let mut tx = app.db.begin(None).await?;
            tx.profiles_clear_in_match(&claimed.host_profile_id, &match_id)
                .await?;
            tx.profiles_clear_in_match(&profile_id, &match_id).await?;
            tx.commit().await?;
            let mut cleanup = app.db.begin(None).await?;
            cleanup.matches_discard_open(&match_id).await?;
            cleanup.commit().await?;
            Ok(())
        }
        .await;
        if let Err(cleanup_error) = cleanup {
            tracing::error!(
                event = "room.start_cleanup_failed",
                matchId = %match_id,
                message = %cleanup_error.message,
            );
        }
        return Err(error);
    }

    tracing::info!(
        event = "room.joined",
        code = %claimed.code,
        mode = claimed.mode.as_str(),
        matchId = %match_id,
        guestProfileId = %profile_id,
    );
    Ok(json_response(
        200,
        json!({
            "matchId": match_id,
            "seriesId": Value::Null,
            "code": claimed.code,
            "seat": "p2",
            "mode": claimed.mode,
        }),
    ))
}

/// DELETE /api/rooms/:code — the host frees a room nobody has joined (§9.5, R1075000) through
/// `release_room`, as a challenge's close frees its room: the code is free at once (R110), a join
/// meets the miss an unknown code is, the end-to-end seed goes with it (R143). Idempotent like
/// `DELETE /api/queue`: `cancelled: false` when the code holds no unjoined room, and a joined room
/// is never touched. Another host's room is the join's 404; a room an open challenge holds is 409,
/// since its challenge's cancel frees it (SPEC S7, B49).
pub async fn cancel(app: &Arc<App>, req: Req) -> ApiResult {
    let profile_id = profile_of(&req)?;
    let code = normalize_code(&req.params.get("code").cloned().unwrap_or_default());
    let mut tx = app.db.begin(None).await?;
    let Some(room) = tx.rooms_get(&code).await? else {
        tx.commit().await?;
        return Ok(json_response(200, json!({ "cancelled": false })));
    };
    if room.host_profile_id != profile_id {
        return Err(ApiError::new(
            ApiErrorCode::NotFound,
            "that room code is not open",
        ));
    }
    if tx.challenges_open_for_room(&code, now_ms()).await?.is_some() {
        return Err(ApiError::new(
            ApiErrorCode::Conflict,
            "that room is a challenge to a friend; cancel the challenge instead",
        ));
    }
    // `rooms_release` deletes only an unjoined row: a claim that landed after the read keeps its room.
    let cancelled = release_room(&mut tx, &code).await?;
    tx.commit().await?;
    if cancelled {
        tracing::info!(event = "room.cancelled", code = %code, hostProfileId = %profile_id);
    }
    Ok(json_response(200, json!({ "cancelled": cancelled })))
}

/// GET /api/rooms — the caller's own rooms nobody has joined and not expired, newest first, as
/// `POST /api/rooms` answered them (§9.5, R1075000); a room an open challenge holds is the
/// challenge's and is not listed.
pub async fn mine(app: &Arc<App>, req: Req) -> ApiResult {
    let profile_id = profile_of(&req)?;
    let now = now_ms();
    let mut tx = app.db.begin(None).await?;
    let open = tx.rooms_open_by_host(&profile_id, now).await?;
    let challenged = tx.challenges_open_of(&profile_id, now).await?;
    tx.commit().await?;
    let rooms: Vec<Value> = open
        .iter()
        .filter(|room| {
            !challenged
                .iter()
                .any(|challenge| challenge.room_code == room.code)
        })
        .map(|room| json!({ "code": room.code, "expiresAt": room.expires_at, "mode": room.mode }))
        .collect();
    Ok(json_response(200, json!({ "rooms": rooms })))
}

// ---- Social: friends, presence, challenges, chat (#937, #938, #941) ----

/// SPEC S7, B49: frees a challenge's room once the challenge is declined or cancelled, in the
/// caller's transaction: an unjoined room goes (its code is free again, R110, and a join meets the
/// miss an unknown code is), with the end-to-end seed its host held for it (R143). A room someone
/// joined is never touched. False when there was no unjoined room under the code.
pub async fn release_room(
    tx: &mut crate::db::store::Tx<'_>,
    code: &str,
) -> Result<bool, crate::db::store::StoreError> {
    let released = tx.rooms_release(code).await?;
    if released {
        seeds().shift_remove(code);
    }
    Ok(released)
}
// ---- end Social ----
