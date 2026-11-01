//! The room-code challenge (SPEC §9.5, R79) in the queue's modes (R264), under the queue's rules: an
//! active account (§9.4) in no match, unfinished series or live co-op match (R1065000), its deck
//! validated (§9.4, R253) and frozen into the room (§9.5, §9.8). `rooms_claim` is the atomic
//! single-claim: a join race's loser gets a 409, never a second match. A joiner plays the room's mode
//! or is refused with it named; All Random deals both decks (R258), Conquest makes the series (R259).
//! R1075000: `mine` and `cancel`. Surface contract: docs/v0.3.0/SURFACE.md §4.1.

use std::cell::RefCell;
use std::sync::{Arc, LazyLock, Mutex};

use indexmap::IndexMap;
use serde_json::{Value, json};

use jackioh_engine::{PlayerId, pick_portrait_from_seed};

use crate::actor::engine::{deal_random_deck, lean_of};
use crate::api::crypto::{is_well_formed_code, normalize_code, random_code};
use crate::api::decks::{
    FrozenChoice, ModeChoiceInput, already_in_match, assert_free_to_play, freeze_choice, host_busy,
    join_refusal, read_mode_choice,
};
use crate::api::http::{ApiError, ApiErrorCode, ApiResult, Req, caller_of, json as json_response};
use crate::api::queue::{new_seed, new_uuid, seat_coin_p1_first, seat_override_of, seed_override_of};
use crate::api::series::start_series;
use crate::api::series_rules::{NewSeriesInput, NewSeriesSide};
// The server's one clock (SURFACE §11.3), via tokio's, so a paused test moves every "now" together.
use crate::app::{App, now_ms};
use crate::config::{MAX_OPEN_ROOMS_PER_HOST, ROOM_CODE_LENGTH, ROOM_CODE_TTL_SECONDS};
use crate::db::store::{MatchSeat, QueueMode, Room, SeriesRow, StartMatchInput};
use crate::seats::Seat;

/// SPEC §11 R149: bounded retries against the codes in use; eight is far more than a 30-bit space
/// (R104) needs against the codes R110 has not released. Config's, like R79's clocks.
const CODE_ATTEMPTS: usize = 8;

/// Held seed, requested seat (R1989000), and room expiry.
#[derive(Clone, Debug)]
struct HeldSeed {
    seed: String,
    seat: Option<PlayerId>,
    expires_at: i64,
}

/// SPEC §11 R143: the e2e seed, held from create to join: a test-mode field must not enter what
/// `src/db/**` persists. `seed_override_of` refuses it outside e2e, so production keeps this empty.
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

/// A room never joined has no claim to clear its seed, so stale seeds drop as rooms are made.
fn remember_seed(code: &str, seed: &str, seat: Option<PlayerId>, expires_at: i64, now: i64) {
    let mut held = seeds();
    held.retain(|_, entry| entry.expires_at > now);
    held.insert(
        code.to_string(),
        HeldSeed {
            seed: seed.to_string(),
            seat,
            expires_at,
        },
    );
}

/// Consumes the room's seed and pins; the host's seed wins, but a seat-only pin carries none (R143),
/// so the joiner's is honored then (R1989000).
fn take_seed_for_room(
    code: &str,
    joiner_seed: Option<String>,
    joiner_seat: Option<PlayerId>,
) -> (Option<String>, Option<PlayerId>, Option<PlayerId>) {
    let held = seeds().shift_remove(code);
    match held {
        Some(entry) if !entry.seed.is_empty() => (Some(entry.seed), entry.seat, joiner_seat),
        Some(entry) => (joiner_seed, entry.seat, joiner_seat),
        None => (joiner_seed, None, joiner_seat),
    }
}

thread_local! {
    /// The test seam for R149's bound; absent, as outside a test, codes are random.
    static SCRIPTED_CODES: RefCell<Option<ScriptedCodes>> = const { RefCell::new(None) };
}

/// The codes a test scripted, and how many have been minted from them.
struct ScriptedCodes {
    codes: Vec<String>,
    minted: usize,
}

/// Scripts room codes for the calling thread until the guard drops, so only that test sees them.
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

/// R264: refusal naming the room's mode; the client reads it from `details.mode`.
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

/// A room the caller may join; missing, malformed and expired are one answer, so a scan cannot tell
/// "wrong" from "too late" (§9.4).
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

/// The two seats of a room's match: R1989000 seats host and joiner by seed coin.
fn room_seats(
    room: &Room,
    joiner_id: &str,
    joiner: &FrozenChoice,
    seed: &str,
    host_is_p1: bool,
) -> Result<(MatchSeat, MatchSeat), ApiError> {
    let (host_player, joiner_player) = if host_is_p1 {
        (PlayerId::P1, PlayerId::P2)
    } else {
        (PlayerId::P2, PlayerId::P1)
    };
    if room.mode == QueueMode::Random {
        // R258, R642: decks and portraits dealt by seat; R1372, R924000: each leans on its player's set.
        let joiner_lean = match joiner {
            FrozenChoice::Random { lean } => *lean,
            _ => None,
        };
        return Ok((
            MatchSeat {
                profile_id: room.host_profile_id.clone(),
                player: host_player,
                deck: deal_random_deck(
                    &format!("{seed}:{}-deck", host_player.as_str()),
                    lean_of(room.host_lean),
                ),
                portrait: Some(
                    pick_portrait_from_seed(&format!("{seed}:portrait:{}", host_player.as_str()))
                        .as_str()
                        .to_string(),
                ),
                // R1834002: an All Random seat plays no deck of its own: the account's slots.
                deck_slots: None,
            },
            MatchSeat {
                profile_id: joiner_id.to_string(),
                player: joiner_player,
                deck: deal_random_deck(
                    &format!("{seed}:{}-deck", joiner_player.as_str()),
                    lean_of(joiner_lean),
                ),
                portrait: Some(
                    pick_portrait_from_seed(&format!("{seed}:portrait:{}", joiner_player.as_str()))
                        .as_str()
                        .to_string(),
                ),
                deck_slots: None,
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
            player: host_player,
            deck: room.host_deck.clone(),
            portrait: room.host_portrait.clone().flatten(),
            // R1834002: each deck's slots, frozen with it: the host's at the room's making.
            deck_slots: room.host_deck_slots.clone(),
        },
        MatchSeat {
            profile_id: joiner_id.to_string(),
            player: joiner_player,
            deck: deck.cards.clone(),
            portrait: deck.portrait.clone().flatten(),
            deck_slots: deck.slots.clone(),
        },
    ))
}

/// POST /api/rooms — create a room in a mode and return its code (§9.5, R264); a challenge (B48,
/// #937, #938, #941) opens its room through `open_room` too.
pub async fn create(app: &Arc<App>, req: Req) -> ApiResult {
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

/// Opens a room for `ttl_ms` unjoined; a challenge's lapses with it (B48, B49).
pub async fn open_room(app: &App, req: &Req, ttl_ms: i64) -> Result<OpenedRoom, ApiError> {
    // ---- end Social ----
    let profile_id = caller_of(req)?.profile.id.clone();
    // R257, R264: the same choice the queue takes.
    let choice = read_mode_choice(&req.body)?;
    // R143, R1989000: e2e seed and seat, read first so a refused one makes no room.
    let seed = seed_override_of(app, &req.body)?;
    let seat = seat_override_of(app, &req.body)?;
    assert_free_to_play(app, &profile_id).await?;

    // R253: checked now and frozen into the room — a deck, a trio, or nothing for All Random.
    let frozen = freeze_choice(app, &profile_id, &choice).await?;
    let mode = frozen_mode(&frozen);

    let now = now_ms();
    let expires_at = now + ttl_ms;

    // R890000: bounded open rooms per host; advisory, as `join`'s seats (R1523000) stop a double seat.
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
            // R1834002: and its slots with it.
            host_deck_slots: match &frozen {
                FrozenChoice::Bo1 { deck } => deck.slots.clone(),
                _ => None,
            },
            host_trio: match &frozen {
                FrozenChoice::Bo3 { trio } => Some(trio.as_ref().clone()),
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
        // R143, R1989000: held only once the create won, else it would never be dropped.
        if let Some(seed) = seed.as_deref() {
            remember_seed(&code, seed, seat, expires_at, now);
        } else if let Some(seat) = seat {
            remember_seed(&code, "", Some(seat), expires_at, now);
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

/// POST /api/rooms/:code/join — claim room and start game (§9.5, R264); `&Arc<App>`, not SURFACE
/// §11.2's `&App`, since `Registry::start` takes one.
pub async fn join(app: &Arc<App>, req: Req) -> ApiResult {
    let profile_id = caller_of(&req)?.profile.id.clone();
    let choice = read_mode_choice(&req.body)?;
    // R143, R1989000: joiner seed and seat overrides in e2e mode.
    let joiner_seed = seed_override_of(app, &req.body)?;
    let joiner_seat = seat_override_of(app, &req.body)?;
    assert_free_to_play(app, &profile_id).await?;

    let typed = req.params.get("code").cloned().unwrap_or_default();
    let room = joinable_room(app, &typed).await?;
    if room.host_profile_id == profile_id {
        return Err(ApiError::new(
            ApiErrorCode::Conflict,
            "you created that room; wait for someone to join",
        ));
    }
    // B49: a challenge's room is the challenged friend's to join.
    crate::social::challenges::assert_may_join(app, &room.code, &profile_id).await?;
    // R264: mode checked before the deck, so the refusal is the useful one.
    if choice_mode(&choice) != room.mode {
        return Err(ApiError::with_details(
            ApiErrorCode::Conflict,
            mode_refusal(room.mode),
            json!({ "mode": room.mode }),
        ));
    }

    let frozen = freeze_choice(app, &profile_id, &choice).await?;

    let match_id = new_uuid();
    let series_id = new_uuid();
    let now = now_ms();

    // R1523000: both seats or neither, before anything is written; a rival that reserved first wins.
    let seat = if matches!(frozen, FrozenChoice::Bo3 { .. }) {
        Seat::Series {
            id: series_id.clone(),
            game: None,
        }
    } else {
        Seat::Match(match_id.clone())
    };
    let lease = app
        .seats
        .reserve(&[&profile_id, &room.host_profile_id], seat)
        .map_err(|taken| join_refusal(&profile_id, &taken))?;

    // The atomic single-claim: whoever wins this statement is the guest, and there is no second
    // winner (§9.5, mirroring `tickets_claim_pair`). Fenced first (R1523000): once a newer server's
    // epoch has overtaken this one, nothing is claimed and the join answers 503.
    let mut tx = app.db.begin(None).await?;
    tx.seats_fence(app.seats.epoch()).await?;
    let claimed = tx.rooms_claim(&room.code, &profile_id, &match_id, now).await?;
    // B49: accept challenge in same transaction.
    if claimed.is_some() {
        tx.challenges_accept_room(&room.code, &profile_id, now).await?;
    }
    tx.commit().await?;
    let Some(claimed) = claimed else {
        return Err(ApiError::new(
            ApiErrorCode::Conflict,
            "someone already joined that room",
        ));
    };
    // R143: minted unless an e2e room asked for one. R1989000: a honored pin skips the seed's coin.
    let (seed_held, host_pin, guest_pin) = take_seed_for_room(&claimed.code, joiner_seed, joiner_seat);
    let seed = seed_held.filter(|held| !held.is_empty()).unwrap_or_else(new_seed);
    let host_is_p1 = match (host_pin, guest_pin) {
        (Some(seat), _) => seat == PlayerId::P1,
        (None, Some(seat)) => seat == PlayerId::P2,
        (None, None) => seat_coin_p1_first(&seed),
    };

    if let FrozenChoice::Bo3 { trio } = &frozen {
        // R259, R263: the series opens on its pick phase. R1989000, R335: the coin winner is index 0.
        let Some(host_trio) = claimed.host_trio.clone() else {
            return Err(wiring_fault(format!(
                "Conquest room {} holds no trio",
                claimed.code
            )));
        };
        let (first, second) = if host_is_p1 {
            (
                NewSeriesSide {
                    profile_id: claimed.host_profile_id.clone(),
                    trio: host_trio,
                },
                NewSeriesSide {
                    profile_id: profile_id.clone(),
                    trio: trio.as_ref().clone(),
                },
            )
        } else {
            (
                NewSeriesSide {
                    profile_id: profile_id.clone(),
                    trio: trio.as_ref().clone(),
                },
                NewSeriesSide {
                    profile_id: claimed.host_profile_id.clone(),
                    trio: host_trio,
                },
            )
        };
        let input = NewSeriesInput {
            series_id,
            first_match_id: match_id.clone(),
            sides: (first, second),
            seed_base: seed.clone(),
            catalog_version: app.catalog.version.clone(),
            // R604: a room's series is unranked.
            ranked: false,
        };
        // One transaction, so a half-landed 'picking' row cannot hold both out until R333's deadline.
        let started: Result<SeriesRow, ApiError> = async {
            let mut tx = app.db.begin(None).await?;
            tx.seats_fence(app.seats.epoch()).await?;
            let series = start_series(app, input, &mut tx).await?;
            tx.commit().await?;
            Ok(series)
        }
        .await;
        let series = match started {
            Ok(series) => {
                lease.commit();
                series
            }
            Err(error) => {
                // Discard open match row on series start failure.
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
                // R1989000: the joiner's actual seat, which the coin may have given the host's away.
                "seat": if host_is_p1 { "p2" } else { "p1" },
                "mode": claimed.mode,
            }),
        ));
    }

    let (host_seat, joiner_seat) = room_seats(&claimed, &profile_id, &frozen, &seed, host_is_p1)?;
    // The row is seat-ordered — index 0 is p1 — whatever the coin gave the host.
    let seats = if host_is_p1 {
        (host_seat, joiner_seat)
    } else {
        (joiner_seat, host_seat)
    };

    // §9.5, R890000: in-match flags set compare-and-set before the start, a second guard behind
    // R1523000's seats, so a refusal is drift.
    {
        let mut tx = app.db.begin(None).await?;
        let fenced = tx.seats_fence(app.seats.epoch()).await;
        let host_seated = fenced.is_ok()
            && tx
                .profiles_set_in_match(&claimed.host_profile_id, Some(&match_id))
                .await?;
        let guest_seated = host_seated && tx.profiles_set_in_match(&profile_id, Some(&match_id)).await?;
        if host_seated && guest_seated {
            tx.commit().await?;
        } else {
            // Discard claim and free room code; `tx` drops first, as it holds the fake store's lock.
            drop(tx);
            let mut cleanup = app.db.begin(None).await?;
            cleanup.matches_discard_open(&match_id).await?;
            cleanup.commit().await?;
            fenced?;
            tracing::error!(event = "seats.drift", source = "compare-and-set", matchId = %match_id);
            if host_seated {
                return Err(already_in_match());
            }
            return Err(host_busy());
        }
    }
    // R1523000: commit seat lease once match row holds both seats.
    lease.commit();
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
        // Revert in-match flags (R890000, R1450) and release seats on start failure; a conditional
        // clear never wipes a rival flag.
        let cleanup: Result<(), ApiError> = async {
            let mut tx = app.db.begin(None).await?;
            tx.profiles_clear_in_match(&claimed.host_profile_id, &match_id)
                .await?;
            tx.profiles_clear_in_match(&profile_id, &match_id).await?;
            tx.commit().await?;
            let seat = Seat::Match(match_id.clone());
            for seated in [&claimed.host_profile_id, &profile_id] {
                app.seats.release(seated, &seat);
            }
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
            // R1989000: the joiner's actual seat, which the coin may have given the host's away.
            "seat": if host_is_p1 { "p2" } else { "p1" },
            "mode": claimed.mode,
        }),
    ))
}

/// DELETE /api/rooms/:code — the host frees an unjoined room via `release_room` (§9.5, R1075000);
/// one an open challenge holds is 409, as its challenge's cancel frees it (B49).
pub async fn cancel(app: &Arc<App>, req: Req) -> ApiResult {
    let profile_id = caller_of(&req)?.profile.id.clone();
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
    let canceled = release_room(&mut tx, &code).await?;
    tx.commit().await?;
    if canceled {
        tracing::info!(event = "room.cancelled", code = %code, hostProfileId = %profile_id);
    }
    Ok(json_response(200, json!({ "cancelled": canceled })))
}

/// GET /api/rooms — list caller's active unjoined rooms (§9.5, R1075000).
pub async fn mine(app: &Arc<App>, req: Req) -> ApiResult {
    let profile_id = caller_of(&req)?.profile.id.clone();
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

/// B49: frees a declined or canceled challenge's unjoined room and its e2e seed (R110, R143).
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
