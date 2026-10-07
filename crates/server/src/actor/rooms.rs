//! The room-code challenge (SPEC §9.5, R79, BUILD M6-T4: "room-code challenge (`createRoom` →
//! 6-char code → `joinRoom`)"), in the queue's three modes (R264).
//!
//! §9.5: "Direct challenge by room code (6 characters from the invite-code alphabet) ships before
//! the ranked queue and is the primary mode while the player base is small." The queue's rules
//! apply here too, because they are the same assertions §9.5 makes about entering a match:
//!
//!  - the account is active (the route's `AuthLevel::Active`, §9.4), not already in a match and not
//!    in a series that is not over (R264);
//!  - the deck or trio is validated at match time by the shared validator, not trusted from save
//!    time (§9.4, R253), through the same `freeze_choice` the queue uses;
//!  - the chosen deck or trio is frozen into the room the moment it is created, so editing it
//!    afterwards cannot change the game (§9.4, §9.5, §9.8);
//!  - `rooms_claim` is the atomic single-claim, so the loser of a join race gets a 409 and never a
//!    second match.
//!
//! R264: a room is created in a mode, and a joiner plays that mode or is refused with it named. A
//! Best-of-1 join starts the match on the two frozen decks; an All Random join deals both decks
//! (R258) and starts the match; a Conquest join makes the series (R259), whose first game starts
//! once both players have picked a deck.
//!
//! Port of `apps/server/src/match/rooms.ts` (SURFACE §4.1). The two handlers keep TS's names,
//! `create` (`POST /api/rooms`) and `join` (`POST /api/rooms/:code/join`), with SURFACE §11.2's
//! shape; `app.rs`'s `ROUTES` lists them where `createRoomRoutes()` put them in `allRoutes()`.

use std::cell::RefCell;
use std::sync::{Arc, LazyLock, Mutex};

use indexmap::IndexMap;
use serde_json::{json, Value};

use jackioh_engine::{pick_portrait_from_seed, PlayerId};

use crate::actor::engine::deal_random_deck;
use crate::api::crypto::{is_well_formed_code, normalize_code, random_code};
use crate::api::decks::{assert_not_in_series, freeze_choice, read_mode_choice, FrozenChoice, ModeChoiceInput};
use crate::api::http::{json as json_response, ApiError, ApiErrorCode, ApiResult, Req};
use crate::api::queue::{seed_override_of, new_uuid, new_seed};
use crate::api::series::start_series;
use crate::api::series_rules::{NewSeriesInput, NewSeriesSide};
// `now_ms`: the server's one clock in epoch milliseconds (TS `deps.timers.now()`; SURFACE §11.3:
// `Timers` → `tokio::time`), read through tokio's clock so a test that pauses and advances it moves
// every module's "now" together.
use crate::app::{now_ms, App};
use crate::config::{ROOM_CODE_LENGTH, ROOM_CODE_TTL_SECONDS};
use crate::db::store::{MatchSeat, QueueMode, Room, SeriesRow, StartMatchInput};

/// SPEC §11 R149: a room code "is minted by retrying a bounded number of times against the codes
/// still in use and then reporting that no code is available, rather than retrying without limit".
/// R149 requires the bound and leaves the number to config, the way R79 leaves its clocks; eight is
/// far more than a 30-bit space (R104) ever needs against the codes R110 has not yet released.
const CODE_ATTEMPTS: usize = 8;

// ---------------------------------------------------------------------------------------------
// R143 — the end-to-end mode's optional seed, for the room half
// ---------------------------------------------------------------------------------------------

/// One remembered seed and the expiry of the room it belongs to.
#[derive(Clone, Debug)]
struct HeldSeed {
    seed: String,
    expires_at: i64,
}

/// SPEC §11 R143: "In end-to-end mode the room and queue endpoints accept an optional seed and use
/// it verbatim so a networked spec can be seeded, and outside that mode the field is rejected."
///
/// A room's match is not created until someone joins, so the seed the *host* supplied to
/// `POST /api/rooms` has to be held between the two calls. `room code -> seed`, exactly as
/// `api/queue.rs` holds `ticket id -> seed`, and for the same reasons written out there: `Room`
/// (db/store.rs) carries no seed, and adding one would put a test-mode field into the shape
/// `src/db/**` persists. `seed_override_of` — the same function the queue uses — has already refused
/// the field outside end-to-end mode by the time anything is written here, so a production process
/// keeps this map permanently empty.
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

/// A room that is never joined has no claim to clear it, unlike a ticket, which is always paired or
/// cancelled. So every remembered seed carries its room's expiry and the stale ones are dropped as
/// new rooms are made: a long-running end-to-end server cannot accumulate them.
fn remember_seed(code: &str, seed: &str, expires_at: i64, now: i64) {
    let mut held = seeds();
    held.retain(|_, entry| entry.expires_at > now);
    held.insert(code.to_string(), HeldSeed { seed: seed.to_string(), expires_at });
}

/// The seed for this room, consumed. The host's seed wins over a seed the joiner sent: the room was
/// created first, which is the same "whoever asked first" tie-break `queue.rs` uses between two
/// seeded tickets.
fn take_seed_for_room(code: &str, joiner_seed: Option<String>) -> Option<String> {
    let held = seeds().shift_remove(code);
    match held {
        Some(entry) => Some(entry.seed),
        None => joiner_seed,
    }
}

// ---------------------------------------------------------------------------------------------
// R149 — where a room code comes from
// ---------------------------------------------------------------------------------------------

thread_local! {
    /// Not in SPEC, and no R-row: the test seam for R149's bound (TS's `deps.ids.code`, which
    /// `rooms.test.ts` scripted to collide). Absent, as everywhere outside a test, codes are random.
    static SCRIPTED_CODES: RefCell<Option<ScriptedCodes>> = const { RefCell::new(None) };
}

/// The codes a test scripted, and how many have been minted from them.
struct ScriptedCodes {
    codes: Vec<String>,
    minted: usize,
}

/// Not in SPEC, and no R-row: scripts the room codes `create` mints on the calling thread, in order,
/// the last one repeating once the list runs out (TS `roomIds(codes)`), until the guard drops. A
/// `#[tokio::test]` runs the handlers on its own thread, so the script reaches its requests and no
/// other test's.
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
///
/// TS's `MODE_REFUSAL: Record<QueueMode, string>`, as a total match.
fn mode_refusal(mode: QueueMode) -> &'static str {
    match mode {
        QueueMode::Bo1 => "This room plays Best of 1: pick one of your decks.",
        QueueMode::Bo3 => "This room plays Conquest: pick one of your trios.",
        QueueMode::Random => "This room plays All Random: join it without a deck.",
    }
}

/// The mode a request's choice asks for (TS read `choice.mode`).
fn choice_mode(choice: &ModeChoiceInput) -> QueueMode {
    match choice {
        ModeChoiceInput::Bo1 { .. } => QueueMode::Bo1,
        ModeChoiceInput::Bo3 { .. } => QueueMode::Bo3,
        ModeChoiceInput::Random => QueueMode::Random,
    }
}

/// The mode a frozen choice was made in (TS read `frozen.mode`).
fn frozen_mode(frozen: &FrozenChoice) -> QueueMode {
    match frozen {
        FrozenChoice::Bo1 { .. } => QueueMode::Bo1,
        FrozenChoice::Bo3 { .. } => QueueMode::Bo3,
        FrozenChoice::Random => QueueMode::Random,
    }
}

/// What a TS `throw new Error(…)` in a handler became: the router logged `handler.threw` and
/// answered 500 `internal` "something went wrong" (api/http.ts). A wiring fault, never a player's.
fn wiring_fault(message: String) -> ApiError {
    tracing::warn!(event = "handler.threw", message = %message);
    ApiError::new(ApiErrorCode::Internal, "something went wrong")
}

/// §9.5: "Enqueue asserts the account is active and not in a match."
fn assert_not_in_match(req: &Req) -> Result<(), ApiError> {
    let in_match = req.caller.as_ref().and_then(|caller| caller.profile.in_match_id.as_ref());
    if in_match.is_some() {
        return Err(ApiError::new(ApiErrorCode::AlreadyInMatch, "finish your current match first"));
    }
    Ok(())
}

fn profile_of(req: &Req) -> Result<String, ApiError> {
    match req.caller.as_ref() {
        Some(caller) => Ok(caller.profile.id.clone()),
        // Unreachable on an `AuthLevel::Active` route; the router resolves the caller first.
        None => Err(ApiError::new(ApiErrorCode::Unauthorized, "sign in first")),
    }
}

/// A room the caller may still join. Missing, malformed and expired codes are one answer, so a
/// scan of the 30-bit code space cannot tell "wrong" from "too late" (the same reasoning as §9.4's
/// identical invite-code error).
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
        return Err(ApiError::new(ApiErrorCode::Conflict, "someone already joined that room"));
    }
    // §9.4: a stale catalog is rejected at every door into a match.
    if room.catalog_version != app.catalog.version {
        return Err(ApiError::new(ApiErrorCode::UpdateRequired, "update required"));
    }
    Ok(room)
}

/// §9.5's "not in a match" holds for the host at the moment the match is made, not only when the
/// room was opened — the same reason `try_pair` re-reads it at pairing. A room can wait for its
/// guest up to `ROOM_CODE_TTL_SECONDS`, and the host can be matched from the queue (or paired into
/// a series, R264) meanwhile. The room is left unclaimed, so the join can be tried again once the
/// host is free.
async fn assert_host_free(app: &App, host_profile_id: &str) -> Result<(), ApiError> {
    let mut tx = app.db.begin(None).await?;
    let host = tx.profiles_get_by_id(host_profile_id).await?;
    let series = tx.series_active_for(host_profile_id).await?;
    tx.commit().await?;
    let host_in_match = host.as_ref().and_then(|profile| profile.in_match_id.as_ref()).is_some();
    if host_in_match || series.is_some() {
        return Err(ApiError::new(ApiErrorCode::Conflict, "The host of that room is playing another game right now."));
    }
    Ok(())
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
        // R642: the portraits are dealt the same way, uniformly and seat by seat.
        return Ok((
            MatchSeat {
                profile_id: room.host_profile_id.clone(),
                player: PlayerId::P1,
                deck: deal_random_deck(&format!("{seed}:p1-deck")),
                portrait: Some(pick_portrait_from_seed(&format!("{seed}:portrait:p1")).as_str().to_string()),
            },
            MatchSeat {
                profile_id: joiner_id.to_string(),
                player: PlayerId::P2,
                deck: deal_random_deck(&format!("{seed}:p2-deck")),
                portrait: Some(pick_portrait_from_seed(&format!("{seed}:portrait:p2")).as_str().to_string()),
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
    let profile_id = profile_of(&req)?;
    assert_not_in_match(&req)?;
    // R257, R264: the same choice the queue takes.
    let choice = read_mode_choice(&req.body)?;
    // R143: an optional `seed`, accepted only by an end-to-end test server and rejected — never
    // ignored — anywhere else. Read before any work is done, so a production caller that sends one
    // gets the 400 without a room being made.
    let seed = seed_override_of(app, &req.body)?;
    assert_not_in_series(app, &profile_id).await?;

    // R253: checked now and frozen into the room — a deck, a trio, or nothing for All Random.
    let frozen = freeze_choice(app, &profile_id, &choice).await?;
    let mode = frozen_mode(&frozen);

    let now = now_ms();
    let expires_at = now + ROOM_CODE_TTL_SECONDS * 1000;

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
        // R143, after the create won: a seed remembered for a room that does not exist would never be
        // consumed and never dropped (`queue.rs` waits for its insert for the same reason).
        if let Some(seed) = seed.as_deref() {
            remember_seed(&code, seed, expires_at, now);
        }
        tracing::info!(event = "room.created", code = %code, hostProfileId = %profile_id, mode = mode.as_str());
        return Ok(json_response(200, json!({ "code": code, "expiresAt": expires_at, "mode": mode })));
    }

    tracing::error!(event = "room.code.exhausted", attempts = CODE_ATTEMPTS);
    Err(ApiError::new(ApiErrorCode::Unavailable, "could not allocate a room code; try again"))
}

/// POST /api/rooms/:code/join — claim the room and start its game (§9.5, R264): the match for
/// Best of 1 and All Random, the series for Conquest.
///
/// Takes `&Arc<App>`, not SURFACE §11.2's `&App`: it starts a match, and `Registry::start` takes
/// `&Arc<App>` (the same choice `api::queue::enqueue` makes; part 31 settles `h!`).
pub async fn join(app: &Arc<App>, req: Req) -> ApiResult {
    let profile_id = profile_of(&req)?;
    assert_not_in_match(&req)?;
    let choice = read_mode_choice(&req.body)?;
    // R143 again: both room endpoints accept the field in end-to-end mode and both refuse it
    // outside one. A spec that seeds the join rather than the create still gets its seed.
    let joiner_seed = seed_override_of(app, &req.body)?;
    assert_not_in_series(app, &profile_id).await?;

    let typed = req.params.get("code").cloned().unwrap_or_default();
    let room = joinable_room(app, &typed).await?;
    if room.host_profile_id == profile_id {
        return Err(ApiError::new(ApiErrorCode::Conflict, "you created that room; wait for someone to join"));
    }
    // R264: the room's mode, or a refusal naming it — before the joiner's deck is looked at, so the
    // answer is the useful one.
    if choice_mode(&choice) != room.mode {
        return Err(ApiError::with_details(ApiErrorCode::Conflict, mode_refusal(room.mode), json!({ "mode": room.mode })));
    }

    let frozen = freeze_choice(app, &profile_id, &choice).await?;
    assert_host_free(app, &room.host_profile_id).await?;

    let match_id = new_uuid();
    let now = now_ms();

    // The atomic single-claim: whoever wins this statement is the guest, and there is no second
    // winner (§9.5, mirroring `tickets_claim_pair`).
    let mut tx = app.db.begin(None).await?;
    let claimed = tx.rooms_claim(&room.code, &profile_id, &match_id, now).await?;
    tx.commit().await?;
    let Some(claimed) = claimed else {
        return Err(ApiError::new(ApiErrorCode::Conflict, "someone already joined that room"));
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
            return Err(wiring_fault(format!("Conquest room {} holds no trio", claimed.code)));
        };
        let input = NewSeriesInput {
            series_id: new_uuid(),
            first_match_id: match_id.clone(),
            sides: (
                NewSeriesSide { profile_id: claimed.host_profile_id.clone(), trio: host_trio },
                NewSeriesSide { profile_id: profile_id.clone(), trio: trio.clone() },
            ),
            seed_base: seed.clone(),
            catalog_version: app.catalog.version.clone(),
            // R604: a room's series is unranked.
            ranked: false,
        };
        // One transaction, so the series row and `start_series`'s stale-ticket cancels land
        // together or not at all: a 'picking' row that half-landed would hold both players out
        // of the queue and the room until the pick deadline ran it out (R333).
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
                // The claim has already committed, so the room's `open` row is still there, claimed
                // and pointing at no series — and nothing reaps `open` rows. `matches_discard_open`
                // releases it, which frees the room code at once (`matches_room_code_open_key` covers
                // only rows that exist).
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
    app.matches
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
        .await?;

    // §9.5: the in-match flag both ends of the lifecycle read ("not in a match" above, and
    // "clears both players' in-match state" when the result lands).
    // Two store calls, as TS made them (each its own transaction).
    for seat_profile in [&claimed.host_profile_id, &profile_id] {
        let mut tx = app.db.begin(None).await?;
        tx.profiles_set_in_match(seat_profile, Some(&match_id)).await?;
        tx.commit().await?;
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
