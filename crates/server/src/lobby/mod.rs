//! Private lobbies (SPEC §9.5, §13.9; R3730000 to R3730006, R3731000 to R3731003; issues #3730,
//! #3731): a friend game's pre-game stage, 1v1 in a room mode, co-op against the AI team, or a 2v2
//! between two teams of two people, every route `AuthLevel::Active`.
//!
//! - `POST /api/lobbies` makes one (`201 {code}`), `GET /api/lobbies` lists the caller's.
//! - `GET /api/lobbies/:code` is the lobby as the caller sees it, or a classic room still joinable
//!   under that code (R3730006).
//! - `POST /api/lobbies/:code/{enter,leave,remove,settings,pick,ready,invite,start}` change it, and
//!   a 2v2's `team` and `move` (R3731000) move its members between its teams.
//!
//! A lobby is a row of `public.lobbies` (migrations 0090, 0091): its host, its guest seat (and a 2v2's
//! other team's two, `guest2` and `guest3`), the host's settings, each member's frozen pick and Ready.
//! A member holds no seat of the ledger until the Start (R1523000, R3730000), which reserves them all
//! and starts the game as a room's join does: a 1v1 Start opens and claims a room under the lobby's
//! code (`actor::rooms::start_lobby_room`), a co-op Start starts the co-op match with the host as a1,
//! and a 2v2's a Duos match whose team A the seed's coin picks (R3731001)
//! (`duos::rooms::start_coop_match`). Every write is a compare-and-set on the revision it read
//! (`lobbies_update`), runs the TTL from now (R3730004) and tells every member, old and new, over
//! `/ws/social` (`nudge_lobby`).
//!
//! Blocks (§9.4, R1689000's reading) make a lobby read as unknown, and a guest a block now separates
//! from the host (in a 2v2, from any member seated before them, R3731000) is dropped from the seat as
//! the lobby is read. A refusal of a member's own doing is `409 conflict` with `details.reason`
//! naming it (`full`, `own_lobby`, `not_host`, `started`, `not_ready`, `no_pick`, `already_in`,
//! `changed`, `busy`, and a 2v2's `team_full` and `host_stays`).

pub mod store;

use std::sync::Arc;

use serde_json::{Value, json};

use crate::actor::contracts::one_tx;
use crate::actor::rooms::start_lobby_room;
use crate::api::collection::caller_profile;
use crate::api::crypto::{is_well_formed_code, normalize_code, random_code};
use crate::api::decks::{
    FrozenChoice, ModeChoiceInput, already_in_match, assert_free_to_play, freeze_choice, in_game,
    read_mode_choice,
};
use crate::api::http::{
    ApiError, ApiErrorCode, ApiResult, Req, account_key, bad_request, json as json_response, ok,
    rate_limited, str,
};
use crate::api::queue::{new_seed, new_uuid, seat_coin_p1_first, seed_override_of};
use crate::app::{App, now_ms};
use crate::config::{DUOS_ROOM_TTL_MS, MAX_OPEN_ROOMS_PER_HOST, ROOM_CODE_LENGTH, ROOM_CODE_TTL_SECONDS};
use crate::db::store::QueueMode;
use crate::duos::rooms::{
    CoopStart, CoopStartOutcome, CoopTeamB, duos_choice_of, read_choice, read_difficulty, start_coop_match,
};
use crate::seats::Taken;
use crate::social::friends::are_friends;
use crate::social::live::{nudge_lobby, send_lobby_invite};
use crate::social::presence::{Standing, standings};
use crate::social::{PlayerRef, no_content, not_found};
use jackioh_engine::config::Difficulty;
use store::{Lobby, LobbyMember, LobbyPick, LobbySettings};

/// R149's bound, as a room's: a fresh code is tried this many times before the create gives up.
const CODE_ATTEMPTS: usize = 8;

// Reading a request and a lobby

/// The code a path names, normalized, or none when it could never be a code.
fn code_of(req: &Req) -> Option<String> {
    let code = normalize_code(req.params.get("code").map(String::as_str).unwrap_or(""));
    is_well_formed_code(&code, ROOM_CODE_LENGTH).then_some(code)
}

/// Missing, expired, blocked and (to anyone not in it) started or full are one answer (§9.4).
fn not_open() -> ApiError {
    ApiError::new(ApiErrorCode::NotFound, "that lobby is not open")
}

/// A refusal of the caller's own doing: `409 conflict`, `details.reason` naming it.
fn conflict(reason: &str, message: &str) -> ApiError {
    ApiError::with_details(ApiErrorCode::Conflict, message, json!({ "reason": reason }))
}

fn started() -> ApiError {
    conflict("started", "This lobby's game has started.")
}

fn not_host() -> ApiError {
    conflict("not_host", "Only the host can do that.")
}

/// R3730004: how long a lobby stays open after its last change: a 1v1 room's or a co-op room's TTL
/// (a 2v2's too, R3731000).
fn ttl_of(settings: &LobbySettings) -> i64 {
    match settings {
        LobbySettings::Duel { .. } => ROOM_CODE_TTL_SECONDS * 1000,
        LobbySettings::Coop { .. } | LobbySettings::TwoVTwo => DUOS_ROOM_TTL_MS,
    }
}

// A lobby's seats

/// One seat of a lobby: the host's, the guest's, or (R3731000) one of a 2v2's other team's two.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Place {
    Host,
    Guest,
    Other(usize),
}

const TWO_SEATS: [Place; 2] = [Place::Host, Place::Guest];
const FOUR_SEATS: [Place; 4] = [Place::Host, Place::Guest, Place::Other(0), Place::Other(1)];

fn two_v_two(lobby: &Lobby) -> bool {
    lobby.settings == LobbySettings::TwoVTwo
}

/// The lobby's seats in order, the host's team first.
fn places(lobby: &Lobby) -> &'static [Place] {
    if two_v_two(lobby) { &FOUR_SEATS } else { &TWO_SEATS }
}

/// Who sits at `place`.
fn profile_at(lobby: &Lobby, place: Place) -> Option<&str> {
    match place {
        Place::Host => Some(lobby.host.as_str()),
        Place::Guest => lobby.guest.as_deref(),
        Place::Other(index) => lobby.others[index].profile.as_deref(),
    }
}

/// Where `id` sits, if they do.
fn place_of(lobby: &Lobby, id: &str) -> Option<Place> {
    places(lobby)
        .iter()
        .copied()
        .find(|place| profile_at(lobby, *place) == Some(id))
}

fn pick_at(lobby: &Lobby, place: Place) -> Option<&LobbyPick> {
    match place {
        Place::Host => lobby.host_pick.as_ref(),
        Place::Guest => lobby.guest_pick.as_ref(),
        Place::Other(index) => lobby.others[index].pick.as_ref(),
    }
}

fn ready_at(lobby: &Lobby, place: Place) -> bool {
    match place {
        Place::Host => lobby.host_ready,
        Place::Guest => lobby.guest_ready,
        Place::Other(index) => lobby.others[index].ready,
    }
}

/// The seat at `place` as one member's: who, their pick and their Ready.
fn member_at(lobby: &Lobby, place: Place) -> LobbyMember {
    LobbyMember {
        profile: profile_at(lobby, place).map(str::to_string),
        pick: pick_at(lobby, place).cloned(),
        ready: ready_at(lobby, place),
    }
}

/// Seats `member` at `place`; the host's seat keeps its host, whose pick and Ready alone change.
fn set_at(lobby: &mut Lobby, place: Place, member: LobbyMember) {
    match place {
        Place::Host => {
            lobby.host_pick = member.pick;
            lobby.host_ready = member.ready;
        }
        Place::Guest => {
            lobby.guest = member.profile;
            lobby.guest_pick = member.pick;
            lobby.guest_ready = member.ready;
        }
        Place::Other(index) => lobby.others[index] = member,
    }
}

/// The word a seat is shown by (`role`, `you`), and a Move names (R3731000).
fn role(place: Place) -> &'static str {
    match place {
        Place::Host => "host",
        Place::Guest => "guest",
        Place::Other(0) => "guest2",
        Place::Other(_) => "guest3",
    }
}

/// R3731000: a 2v2's teams, the host's (0) and the other (1).
fn team(place: Place) -> usize {
    match place {
        Place::Host | Place::Guest => 0,
        Place::Other(_) => 1,
    }
}

/// R3731000: the first empty seat, which an entrant takes.
fn empty_place(lobby: &Lobby) -> Option<Place> {
    places(lobby)
        .iter()
        .copied()
        .find(|place| profile_at(lobby, *place).is_none())
}

/// R3731000: every member's Ready off, as a change of teams takes it.
fn unready_all(lobby: &mut Lobby) {
    lobby.host_ready = false;
    lobby.guest_ready = false;
    for member in &mut lobby.others {
        member.ready = false;
    }
}

/// The lobby's members, host first.
fn members(lobby: &Lobby) -> Vec<&str> {
    places(lobby)
        .iter()
        .filter_map(|place| profile_at(lobby, *place))
        .collect()
}

/// Whether `id` hosts or sits in the lobby.
fn is_member(lobby: &Lobby, id: &str) -> bool {
    place_of(lobby, id).is_some()
}

/// The settings a create's body asks for: `{format: "duel", mode}`, `{format: "coop", difficulty}`
/// or `{format: "2v2"}` (R3731000).
fn read_settings(body: &Value) -> Result<LobbySettings, ApiError> {
    match body.get("format").and_then(Value::as_str) {
        Some("duel") => Ok(LobbySettings::Duel {
            mode: read_mode(body)?,
        }),
        Some("coop") => Ok(LobbySettings::Coop {
            difficulty: read_difficulty(body)?,
        }),
        Some(store::FORMAT_TWO_V_TWO) => Ok(LobbySettings::TwoVTwo),
        _ => Err(bad_request(r#""format" must be "duel", "coop" or "2v2""#)),
    }
}

/// A 1v1 lobby's `mode`, one of the room modes (§9.5, R264).
fn read_mode(body: &Value) -> Result<QueueMode, ApiError> {
    match body.get("mode").and_then(Value::as_str) {
        Some("bo1") => Ok(QueueMode::Bo1),
        Some("bo3") => Ok(QueueMode::Bo3),
        Some("random") => Ok(QueueMode::Random),
        _ => Err(bad_request(r#""mode" must be "bo1", "bo3" or "random""#)),
    }
}

/// The mode a 1v1 pick asks for.
fn choice_mode(choice: &ModeChoiceInput) -> QueueMode {
    match choice {
        ModeChoiceInput::Bo1 { .. } => QueueMode::Bo1,
        ModeChoiceInput::Bo3 { .. } => QueueMode::Bo3,
        ModeChoiceInput::Random { .. } => QueueMode::Random,
    }
}

/// R264: a pick in another mode than the lobby's is refused with the lobby's named in `details.mode`.
fn mode_refusal(mode: QueueMode) -> ApiError {
    let message = match mode {
        QueueMode::Bo1 => "This lobby plays Best of 1: pick one of your decks.",
        QueueMode::Bo3 => "This lobby plays Conquest: pick one of your trios.",
        QueueMode::Random => "This lobby plays All Random: take a random deck.",
    };
    ApiError::with_details(
        ApiErrorCode::Conflict,
        message,
        json!({ "mode": mode, "reason": "mode" }),
    )
}

/// The lobby under `code` as `caller` may reach it: missing or expired is 404, and so is one a block
/// separates the caller from its host (in a 2v2, from any member, R3731000) unless the caller is in
/// it. A guest a block now separates from the host is dropped from the seat first (R3730000); in a
/// 2v2, each member a block separates from one seated before them, in seat order (R3731000). One
/// dropped is then refused as anyone blocked is.
async fn load(app: &App, code: &str, caller: &str) -> Result<Lobby, ApiError> {
    let (lobby, dropped, caller_blocked) = one_tx!(app.db, |t| {
        let lobby = t.lobbies_get(code).await?;
        let mut dropped: Vec<Place> = Vec::new();
        let mut caller_blocked = false;
        if let Some(lobby) = &lobby {
            let mut kept: Vec<&str> = vec![lobby.host.as_str()];
            for place in places(lobby).iter().skip(1) {
                let Some(member) = profile_at(lobby, *place) else {
                    continue;
                };
                let mut blocked = false;
                for earlier in &kept {
                    if t.blocks_between(earlier, member).await? {
                        blocked = true;
                        break;
                    }
                }
                if blocked {
                    dropped.push(*place);
                } else {
                    kept.push(member);
                }
            }
            if !is_member(lobby, caller) {
                let guarded = if two_v_two(lobby) { kept.len() } else { 1 };
                for member in &kept[..guarded] {
                    if t.blocks_between(member, caller).await? {
                        caller_blocked = true;
                        break;
                    }
                }
            }
        }
        (lobby, dropped, caller_blocked)
    })?;
    let Some(mut lobby) = lobby.filter(|lobby| lobby.expires_at > now_ms()) else {
        return Err(not_open());
    };
    if !dropped.is_empty() && lobby.started_at.is_none() {
        let gone: Vec<String> = dropped
            .iter()
            .filter_map(|place| profile_at(&lobby, *place).map(str::to_string))
            .collect();
        let mut next = lobby.clone();
        for place in &dropped {
            set_at(&mut next, *place, LobbyMember::default());
        }
        let also: Vec<&str> = gone.iter().map(String::as_str).collect();
        lobby = write(app, &lobby, next, &also).await?;
        tracing::info!(event = "lobby.guest_dropped", code = %code, reason = "blocked", dropped = gone.len());
        if gone.iter().any(|id| id == caller) {
            return Err(not_open());
        }
    }
    if caller_blocked {
        return Err(not_open());
    }
    Ok(lobby)
}

/// `load`, for a member only: anyone else meets the 404 an unknown code does.
async fn load_member(app: &App, req: &Req, caller: &str) -> Result<Lobby, ApiError> {
    let Some(code) = code_of(req) else {
        return Err(not_open());
    };
    let lobby = load(app, &code, caller).await?;
    if !is_member(&lobby, caller) {
        return Err(not_open());
    }
    Ok(lobby)
}

/// `load_member`, for a change to a lobby whose game has not started.
async fn load_open(app: &App, req: &Req, caller: &str) -> Result<Lobby, ApiError> {
    let lobby = load_member(app, req, caller).await?;
    if lobby.started_at.is_some() {
        return Err(started());
    }
    Ok(lobby)
}

/// Writes `next` over `read` (R3730000): one revision on, open the format's TTL from now (R3730004),
/// and only if nothing else wrote since `read` (else `409 changed`). Then every member of either, and
/// `also`, is told.
async fn write(app: &App, read: &Lobby, next: Lobby, also: &[&str]) -> Result<Lobby, ApiError> {
    let next = Lobby {
        rev: read.rev + 1,
        expires_at: now_ms() + ttl_of(&next.settings),
        ..next
    };
    let written = one_tx!(app.db, |t| t.lobbies_update(&next, read.rev).await?)?;
    if !written {
        return Err(conflict("changed", "The lobby changed; try again."));
    }
    tell(app, &[read, &next], also);
    Ok(next)
}

/// R3730000: every member of `lobbies`, and `also`, is told the lobby changed.
fn tell(app: &App, lobbies: &[&Lobby], also: &[&str]) {
    let mut ids: Vec<&str> = lobbies.iter().flat_map(|lobby| members(lobby)).collect();
    ids.extend_from_slice(also);
    ids.sort_unstable();
    ids.dedup();
    if let Some(lobby) = lobbies.first() {
        nudge_lobby(app, &ids, &lobby.code);
    }
}

// What a reader is shown

/// A seat's deck state (R3730000, R3730001): nothing picked yet, picked, or picked and ready.
fn deck_state(pick: Option<&LobbyPick>, ready: bool) -> &'static str {
    match (pick, ready) {
        (None, _) => "choosing",
        (Some(_), false) => "chosen",
        (Some(_), true) => "ready",
    }
}

/// The name a frozen pick shows, a deck's or a trio's; none for a random deck.
fn pick_name(pick: &LobbyPick) -> Option<&str> {
    match &pick.frozen {
        FrozenChoice::Bo1 { deck } => Some(deck.name.as_str()),
        FrozenChoice::Bo3 { trio } => Some(trio.name.as_str()),
        FrozenChoice::Random { .. } => None,
    }
}

/// The lobby as `caller` reads it (R3730000). Rule 7: no card leaves, and a 1v1 opponent's deck is
/// shown only as its state, never by name, as the queue never names it; a 2v2's other team's decks
/// alike, a teammate's by name (R3731000).
async fn view(app: &App, lobby: &Lobby, caller: &str) -> Result<Value, ApiError> {
    let ids: Vec<String> = members(lobby).into_iter().map(str::to_string).collect();
    let (profiles, standing) = one_tx!(app.db, |t| {
        let profiles = t.profiles_get_many(&ids).await?;
        let standing = standings(&mut t, app, caller, &profiles).await?;
        (profiles, standing)
    })?;
    let yours = place_of(lobby, caller);
    // Whose decks the caller is shown by name: their own; in co-op everyone's; in a 2v2 their team's.
    let shown_at = |place: Place| match lobby.settings {
        LobbySettings::Duel { .. } => yours == Some(place),
        LobbySettings::Coop { .. } => true,
        LobbySettings::TwoVTwo => yours.is_some_and(|yours| team(yours) == team(place)),
    };
    let player = |id: &str| {
        profiles
            .iter()
            .find(|profile| profile.id == id)
            .map(|profile| {
                let status = standing.get(id).unwrap_or(&Standing::OFFLINE).status;
                json!({ "profileId": profile.id, "username": profile.username(), "status": status })
            })
            .unwrap_or(Value::Null)
    };
    let seat = |place: Place| {
        let pick = pick_at(lobby, place);
        let shown = pick.filter(|_| shown_at(place));
        json!({
            "role": role(place),
            "player": profile_at(lobby, place).map(&player).unwrap_or(Value::Null),
            "state": deck_state(pick, ready_at(lobby, place)),
            "deck": shown.and_then(pick_name),
            "random": shown.is_some_and(|pick| matches!(pick.frozen, FrozenChoice::Random { .. })),
        })
    };
    let seats: Vec<Value> = places(lobby).iter().map(|place| seat(*place)).collect();
    let your_pick = yours.and_then(|place| pick_at(lobby, place));
    Ok(json!({
        "kind": "lobby",
        "code": lobby.code,
        "settings": lobby.settings,
        "expiresAt": lobby.expires_at,
        "you": yours.map(role),
        "yourPick": your_pick.map(|pick| &pick.choice),
        "seats": seats,
        "startedAt": lobby.started_at,
        "matchId": lobby.match_id,
        "seriesId": lobby.series_id,
    }))
}

/// R3730006: with no lobby under `code`, a classic room still open under it, as the lobby's screen
/// offers it: a 1v1 room the caller may join (a challenge's is its friend's alone, R1689000) or an
/// unjoined co-op room. Anything else, a block between the caller and its host among it, is 404.
async fn classic_room(app: &App, code: &str, caller: &str) -> Result<Value, ApiError> {
    let now = now_ms();
    let (room, coop) = one_tx!(app.db, |t| {
        let room = t.rooms_get(code).await?;
        let coop = t.duos_rooms_get(code).await?;
        (room, coop)
    })?;
    if let Some(room) = room.filter(|room| room.guest_profile_id.is_none() && room.expires_at > now) {
        crate::social::challenges::assert_may_join(app, code, caller)
            .await
            .map_err(|_| not_open())?;
        if let Some(host) = host_ref(app, &room.host_profile_id, caller).await? {
            return Ok(json!({ "kind": "room", "code": room.code, "mode": room.mode, "host": host }));
        }
    }
    if let Some(room) = coop.filter(|room| room.guest.is_none() && room.expires_at > now)
        && let Some(host) = host_ref(app, &room.host, caller).await?
    {
        return Ok(json!({
            "kind": "coopRoom",
            "code": room.code,
            "difficulty": room.difficulty,
            "host": host,
        }));
    }
    Err(not_open())
}

/// A room host as `caller` is shown them, or none when a block stands between them.
async fn host_ref(app: &App, host: &str, caller: &str) -> Result<Option<PlayerRef>, ApiError> {
    Ok(one_tx!(app.db, |t| {
        if host != caller && t.blocks_between(host, caller).await? {
            None
        } else {
            t.profiles_get_by_id(host).await?.as_ref().map(PlayerRef::of)
        }
    })?)
}

// The routes

/// `POST /api/lobbies` (R3730000): `{format: "duel", mode}` or `{format: "coop", difficulty}`, a lobby
/// the caller hosts, `201 {code}`. R890000, R3730004: at most `MAX_OPEN_ROOMS_PER_HOST` unstarted
/// lobbies of each format, counted apart from rooms.
pub async fn create(app: &Arc<App>, req: Req) -> ApiResult {
    let profile = caller_profile(&req)?;
    let settings = read_settings(&req.body)?;
    assert_free_to_play(app, &profile.id).await?;
    let now = now_ms();
    let open = one_tx!(app.db, |t| {
        t.lobbies_reap(now).await?;
        t.lobbies_count_open_by_host(&profile.id, settings.format(), now)
            .await?
    })?;
    if open >= MAX_OPEN_ROOMS_PER_HOST {
        return Err(ApiError::new(
            ApiErrorCode::Conflict,
            "you have too many lobbies open; close one or let one expire",
        ));
    }
    for _attempt in 0..CODE_ATTEMPTS {
        let code = random_code(ROOM_CODE_LENGTH);
        let lobby = Lobby {
            code: code.clone(),
            host: profile.id.clone(),
            guest: None,
            settings,
            host_pick: None,
            guest_pick: None,
            host_ready: false,
            guest_ready: false,
            others: Default::default(),
            rev: 0,
            catalog_version: app.catalog.version.clone(),
            created_at: now,
            expires_at: now + ttl_of(&settings),
            match_id: None,
            series_id: None,
            started_at: None,
        };
        // R3730006: a code a room holds stays the room's, so an old link never meets a lobby.
        let created = one_tx!(app.db, |t| {
            t.rooms_get(&code).await?.is_none()
                && t.duos_rooms_get(&code).await?.is_none()
                && t.lobbies_create(&lobby).await?
        })?;
        if !created {
            continue;
        }
        tracing::info!(event = "lobby.created", code = %code, hostProfileId = %profile.id, format = settings.format());
        return Ok(json_response(201, json!({ "code": code })));
    }
    tracing::error!(event = "lobby.code.exhausted", attempts = CODE_ATTEMPTS);
    Err(ApiError::new(
        ApiErrorCode::Unavailable,
        "could not allocate a lobby code; try again",
    ))
}

/// `GET /api/lobbies` (R3730004): the caller's lobbies whose game has not started, newest first, so a
/// reload finds its lobby again (R1075000).
pub async fn mine(app: &Arc<App>, req: Req) -> ApiResult {
    let profile = caller_profile(&req)?;
    let held = one_tx!(app.db, |t| t.lobbies_of_member(&profile.id, now_ms()).await?)?;
    let lobbies: Vec<Value> = held
        .iter()
        .filter(|lobby| lobby.started_at.is_none())
        .filter_map(|lobby| {
            let place = place_of(lobby, &profile.id)?;
            Some(json!({
                "code": lobby.code,
                "settings": lobby.settings,
                "you": role(place),
                "expiresAt": lobby.expires_at,
            }))
        })
        .collect();
    Ok(ok(json!({ "lobbies": lobbies })))
}

/// `GET /api/lobbies/:code` (R3730000, R3730006): the lobby as the caller sees it, a member or a
/// visitor holding its code (none once it is full or started), else a classic room under the code.
pub async fn get_lobby(app: &Arc<App>, req: Req) -> ApiResult {
    let profile = caller_profile(&req)?;
    let Some(code) = code_of(&req) else {
        return Err(not_open());
    };
    match load(app, &code, &profile.id).await {
        Ok(lobby) => {
            if !is_member(&lobby, &profile.id) && (empty_place(&lobby).is_none() || lobby.started_at.is_some()) {
                return Err(not_open());
            }
            Ok(ok(view(app, &lobby, &profile.id).await?))
        }
        Err(error) if error.code == ApiErrorCode::NotFound => {
            let exists = one_tx!(app.db, |t| t.lobbies_get(&code).await?.is_some())?;
            if exists {
                // A lobby the caller may not reach is never a room's code either.
                return Err(error);
            }
            Ok(ok(classic_room(app, &code, &profile.id).await?))
        }
        Err(error) => Err(error),
    }
}

/// `POST /api/lobbies/:code/enter` (R3730000): the caller takes the empty seat (in a 2v2 the first
/// empty one, R3731000). Their own lobby is `own_lobby`, a lobby with no empty seat `full`; a blocked
/// caller meets the 404 an unknown code does.
pub async fn enter(app: &Arc<App>, req: Req) -> ApiResult {
    let profile = caller_profile(&req)?;
    let Some(code) = code_of(&req) else {
        return Err(not_open());
    };
    let lobby = load(app, &code, &profile.id).await?;
    if lobby.host == profile.id {
        if lobby.started_at.is_some() {
            return Err(not_open());
        }
        return Err(conflict("own_lobby", "This is your lobby."));
    }
    if is_member(&lobby, &profile.id) {
        return Ok(ok(json!({ "code": lobby.code })));
    }
    if lobby.started_at.is_some() {
        return Err(not_open());
    }
    let Some(place) = empty_place(&lobby) else {
        return Err(conflict("full", "This lobby is full."));
    };
    assert_free_to_play(app, &profile.id).await?;
    // §9.4: a stale catalog is refused at every door into a match.
    if lobby.catalog_version != app.catalog.version {
        return Err(ApiError::new(ApiErrorCode::UpdateRequired, "update required"));
    }
    let mut next = lobby.clone();
    set_at(
        &mut next,
        place,
        LobbyMember {
            profile: Some(profile.id.clone()),
            ..LobbyMember::default()
        },
    );
    write(app, &lobby, next, &[]).await?;
    tracing::info!(event = "lobby.entered", code = %code, guestProfileId = %profile.id, seat = role(place));
    Ok(ok(json!({ "code": lobby.code })))
}

/// `POST /api/lobbies/:code/leave` (R3730004): a guest frees their seat; the host leaving closes the
/// lobby (R110, R1075000), `204`. A started lobby is left to its TTL: it is what takes each member
/// to the board, so a Leave during the countdown would strand the others (`409 started`).
pub async fn leave(app: &Arc<App>, req: Req) -> ApiResult {
    let profile = caller_profile(&req)?;
    let lobby = load_open(app, &req, &profile.id).await?;
    if lobby.host == profile.id {
        let deleted = one_tx!(app.db, |t| t.lobbies_delete(&lobby.code).await?)?;
        if deleted {
            tell(app, &[&lobby], &[]);
            tracing::info!(event = "lobby.closed", code = %lobby.code, reason = "host_left");
        }
        return Ok(no_content());
    }
    let Some(place) = place_of(&lobby, &profile.id) else {
        return Err(not_open());
    };
    let mut next = lobby.clone();
    set_at(&mut next, place, LobbyMember::default());
    write(app, &lobby, next, &[]).await?;
    Ok(no_content())
}

/// `POST /api/lobbies/:code/remove` `{profileId}` (R3730004): the host frees a guest's seat, `204`.
/// The removed guest may enter again.
pub async fn remove(app: &Arc<App>, req: Req) -> ApiResult {
    let profile = caller_profile(&req)?;
    let removed = str(&req.body, "profileId")?;
    let lobby = load_open(app, &req, &profile.id).await?;
    if lobby.host != profile.id {
        return Err(not_host());
    }
    let Some(place) = place_of(&lobby, &removed).filter(|place| *place != Place::Host) else {
        return Err(not_found("that player is not in this lobby"));
    };
    let mut next = lobby.clone();
    set_at(&mut next, place, LobbyMember::default());
    write(app, &lobby, next, &[]).await?;
    tracing::info!(event = "lobby.removed", code = %lobby.code);
    Ok(no_content())
}

/// `POST /api/lobbies/:code/settings` (R3730003): the host's `{mode}` (1v1) or `{difficulty}`
/// (co-op), `204`. A change takes every Ready off; a new mode takes the picks off too, as no pick
/// of one mode plays in another. The format never changes, and a 2v2 has nothing to set (R3731000).
pub async fn settings(app: &Arc<App>, req: Req) -> ApiResult {
    let profile = caller_profile(&req)?;
    let lobby = load_open(app, &req, &profile.id).await?;
    if lobby.host != profile.id {
        return Err(not_host());
    }
    let next = match lobby.settings {
        LobbySettings::Duel { mode } => {
            let asked = read_mode(&req.body)?;
            if asked == mode {
                return Ok(no_content());
            }
            Lobby {
                settings: LobbySettings::Duel { mode: asked },
                host_pick: None,
                guest_pick: None,
                host_ready: false,
                guest_ready: false,
                ..lobby.clone()
            }
        }
        LobbySettings::Coop { difficulty } => {
            let asked = read_difficulty(&req.body)?;
            if asked == difficulty {
                return Ok(no_content());
            }
            Lobby {
                settings: LobbySettings::Coop { difficulty: asked },
                host_ready: false,
                guest_ready: false,
                ..lobby.clone()
            }
        }
        LobbySettings::TwoVTwo => return Err(bad_request("a 2v2 lobby has no settings")),
    };
    write(app, &lobby, next, &[]).await?;
    Ok(no_content())
}

/// `POST /api/lobbies/:code/pick` (R3730001): the caller's deck, trio or random deck, checked legal
/// and frozen now (R253): a 1v1 lobby takes the queue's body in its own mode (R264), a co-op or 2v2
/// lobby a co-op room's (`deckId` or `random: true`). A new pick takes the caller's Ready off. `204`.
pub async fn pick(app: &Arc<App>, req: Req) -> ApiResult {
    let profile = caller_profile(&req)?;
    let lobby = load_open(app, &req, &profile.id).await?;
    let choice = match lobby.settings {
        LobbySettings::Duel { mode } => {
            let choice = read_mode_choice(&req.body)?;
            if choice_mode(&choice) != mode {
                return Err(mode_refusal(mode));
            }
            choice
        }
        LobbySettings::Coop { .. } | LobbySettings::TwoVTwo => read_choice(&req.body)?,
    };
    let frozen = freeze_choice(app, &profile.id, &choice).await?;
    let Some(place) = place_of(&lobby, &profile.id) else {
        return Err(not_open());
    };
    let mut next = lobby.clone();
    set_at(
        &mut next,
        place,
        LobbyMember {
            profile: Some(profile.id.clone()),
            pick: Some(LobbyPick { choice, frozen }),
            ready: false,
        },
    );
    write(app, &lobby, next, &[]).await?;
    Ok(no_content())
}

/// `POST /api/lobbies/:code/ready` `{ready}` (R3730001): the caller's Ready, on only over a pick
/// (`no_pick`). `204`.
pub async fn ready(app: &Arc<App>, req: Req) -> ApiResult {
    let profile = caller_profile(&req)?;
    let wanted = match req.body.get("ready") {
        Some(Value::Bool(ready)) => *ready,
        _ => return Err(bad_request(r#""ready" must be true or false"#)),
    };
    let lobby = load_open(app, &req, &profile.id).await?;
    let Some(place) = place_of(&lobby, &profile.id) else {
        return Err(not_open());
    };
    if wanted && pick_at(&lobby, place).is_none() {
        return Err(conflict("no_pick", "Pick a deck first."));
    }
    let mut next = lobby.clone();
    let member = member_at(&lobby, place);
    set_at(
        &mut next,
        place,
        LobbyMember {
            ready: wanted,
            ..member
        },
    );
    write(app, &lobby, next, &[]).await?;
    Ok(no_content())
}

/// `POST /api/lobbies/:code/team` (R3731000): a 2v2's member, the host aside, moves to the other team's
/// first empty seat, keeping their pick; every Ready goes off. `204`. Outside a 2v2 it is `400`; the
/// host's team is the host's (`host_stays`), and a full other team is `team_full`.
pub async fn team_switch(app: &Arc<App>, req: Req) -> ApiResult {
    let profile = caller_profile(&req)?;
    let lobby = load_open(app, &req, &profile.id).await?;
    if !two_v_two(&lobby) {
        return Err(bad_request("only a 2v2 lobby has teams"));
    }
    let Some(from) = place_of(&lobby, &profile.id) else {
        return Err(not_open());
    };
    if from == Place::Host {
        return Err(conflict("host_stays", "The host stays on their own team."));
    }
    let Some(to) = places(&lobby)
        .iter()
        .copied()
        .find(|place| team(*place) != team(from) && profile_at(&lobby, *place).is_none())
    else {
        return Err(conflict("team_full", "The other team is full."));
    };
    let mut next = lobby.clone();
    set_at(&mut next, to, member_at(&lobby, from));
    set_at(&mut next, from, LobbyMember::default());
    unready_all(&mut next);
    write(app, &lobby, next, &[]).await?;
    tracing::info!(event = "lobby.team_switched", code = %lobby.code, seat = role(to));
    Ok(no_content())
}

/// `POST /api/lobbies/:code/move` `{profileId, seat}` (R3731000): the host seats a member, the host
/// aside, at `seat` (`guest`, `guest2` or `guest3`), swapping them with whoever sits there; every
/// Ready goes off. `204`. Outside a 2v2 or for another seat it is `400`; a profile not in the lobby
/// `404`.
pub async fn move_member(app: &Arc<App>, req: Req) -> ApiResult {
    let profile = caller_profile(&req)?;
    let moved = str(&req.body, "profileId")?;
    let seat = str(&req.body, "seat")?;
    let lobby = load_open(app, &req, &profile.id).await?;
    if lobby.host != profile.id {
        return Err(not_host());
    }
    if !two_v_two(&lobby) {
        return Err(bad_request("only a 2v2 lobby has teams"));
    }
    let Some(to) = FOUR_SEATS
        .into_iter()
        .find(|place| *place != Place::Host && role(*place) == seat)
    else {
        return Err(bad_request(r#""seat" must be "guest", "guest2" or "guest3""#));
    };
    let Some(from) = place_of(&lobby, &moved).filter(|place| *place != Place::Host) else {
        return Err(not_found("that player is not in this lobby"));
    };
    if from == to {
        return Ok(no_content());
    }
    let mut next = lobby.clone();
    set_at(&mut next, to, member_at(&lobby, from));
    set_at(&mut next, from, member_at(&lobby, to));
    unready_all(&mut next);
    write(app, &lobby, next, &[]).await?;
    tracing::info!(event = "lobby.member_moved", code = %lobby.code, seat = role(to));
    Ok(no_content())
}

/// `POST /api/lobbies/:code/invite` `{profileId}` (R3730005): a member invites a friend neither
/// blocked (in a 2v2, with no member blocked, R3731000) into an empty seat, a live toast over
/// `/ws/social`, counted against the challenges' `CHALLENGES_PER_HOUR`. A non-friend is 404; one in
/// the lobby `already_in`, a lobby with no empty seat `full`. `204`.
pub async fn invite(app: &Arc<App>, req: Req) -> ApiResult {
    let profile = caller_profile(&req)?;
    let friend_id = str(&req.body, "profileId")?;
    let now = now_ms();
    let key = account_key(&profile.id);
    if !app.social.challenges.allow(&key, now) {
        return Err(rate_limited(
            "too many invites; try again later",
            app.social.challenges.retry_after_ms(&key, now),
        ));
    }
    let lobby = load_open(app, &req, &profile.id).await?;
    // Who a block with the friend refuses the invite for: the host, or in a 2v2 any member; the
    // inviter's own friendship already says theirs.
    let guarded: Vec<&str> = if two_v_two(&lobby) {
        members(&lobby)
    } else {
        vec![lobby.host.as_str()]
    };
    let friends = one_tx!(app.db, |t| {
        let mut unblocked = are_friends(&mut t, &profile.id, &friend_id).await?;
        for member in &guarded {
            if !unblocked || *member == profile.id {
                continue;
            }
            unblocked = !t.blocks_between(member, &friend_id).await?;
        }
        unblocked
    })?;
    if !friends {
        return Err(not_found("that player is not your friend"));
    }
    if is_member(&lobby, &friend_id) {
        return Err(conflict("already_in", "They're already in this lobby."));
    }
    if empty_place(&lobby).is_none() {
        return Err(conflict("full", "This lobby is full."));
    }
    let from = serde_json::to_value(PlayerRef::of(&profile))
        .map_err(|error| ApiError::internal(error.to_string()))?;
    send_lobby_invite(app, &friend_id, &lobby.code, lobby.settings.format(), &from);
    tracing::info!(event = "lobby.invited", code = %lobby.code);
    Ok(no_content())
}

/// R3731001: a 2v2's teams as the Duos match seats them, team A then team B, each in seat order (its
/// first seat a1 or b1, in duel 0): the seed's coin (`seat_coin_p1_first`, R1989000) puts the host's
/// team first when it lands true. Only the seed goes in, so the same seed always seats the same way.
pub(crate) fn lineup<'a>(
    seed: &str,
    host_team: [&'a str; 2],
    other_team: [&'a str; 2],
) -> ([&'a str; 2], [&'a str; 2]) {
    if seat_coin_p1_first(seed) {
        (host_team, other_team)
    } else {
        (other_team, host_team)
    }
}

/// `POST /api/lobbies/:code/start` (R3730002): the host starts the game once every seat is filled
/// and every member ready (`not_ready`), `200 {matchId, seriesId}`. The Start is written first,
/// compare-and-set, so one Start wins and nothing changes the lobby under it; then every seat is
/// reserved and the game starts as a room's does (unranked, R604), the catalog, every player's
/// games and the ledger checked again. A refusal takes the Start back off. A 2v2's starts the Duos
/// match its seed's coin lines up (R3731001), which takes an end-to-end server's `seed` (R143).
pub async fn start(app: &Arc<App>, req: Req) -> ApiResult {
    let profile = caller_profile(&req)?;
    let lobby = load_open(app, &req, &profile.id).await?;
    if lobby.host != profile.id {
        return Err(not_host());
    }
    let seated: Option<Vec<(String, LobbyPick)>> = places(&lobby)
        .iter()
        .map(|place| {
            let id = profile_at(&lobby, *place)?;
            let pick = pick_at(&lobby, *place)?;
            ready_at(&lobby, *place).then(|| (id.to_string(), pick.clone()))
        })
        .collect();
    let Some(seated) = seated else {
        return Err(conflict("not_ready", "Every seat must be filled and ready."));
    };
    let seed = if two_v_two(&lobby) {
        Some(seed_override_of(app, &req.body)?.unwrap_or_else(new_seed))
    } else {
        None
    };
    // §9.4: a stale catalog is refused at every door into a match.
    if lobby.catalog_version != app.catalog.version {
        return Err(ApiError::new(ApiErrorCode::UpdateRequired, "update required"));
    }
    assert_free_to_play(app, &profile.id).await?;

    let starting = write(
        app,
        &lobby,
        Lobby {
            started_at: Some(now_ms()),
            ..lobby.clone()
        },
        &[],
    )
    .await?;
    let host_id = profile.id.clone();
    let others_word = if two_v_two(&lobby) {
        "A player is playing another game right now."
    } else {
        "Your opponent is playing another game right now."
    };
    let refusal = |taken: &Taken| {
        if taken.profile == host_id {
            in_game(&taken.seat)
        } else {
            conflict("busy", others_word)
        }
    };
    let began: Result<(Option<String>, Option<String>), ApiError> = match lobby.settings {
        LobbySettings::Duel { .. } => start_lobby_room(
            app,
            &lobby.code,
            &seated[0].0,
            &seated[0].1.frozen,
            &seated[1].0,
            &seated[1].1.frozen,
            refusal,
        )
        .await
        .map(|started| (started.match_id, started.series_id)),
        LobbySettings::Coop { .. } | LobbySettings::TwoVTwo => {
            async {
                let choices = seated
                    .iter()
                    .map(|(_, pick)| duos_choice_of(pick.frozen.clone(), &lobby.catalog_version))
                    .collect::<Result<Vec<_>, ApiError>>()?;
                let match_id = new_uuid();
                // Who sits where: the host a1 against the AI team, or a 2v2's teams as its coin
                // lines them up, each team's first seat in duel 0 (R3731001).
                let at = |id: &str| seated.iter().position(|(held, _)| held == id).unwrap_or(0);
                let ids: Vec<&str> = seated.iter().map(|(id, _)| id.as_str()).collect();
                let (team_a, team_b) = match (&lobby.settings, &seed) {
                    (LobbySettings::TwoVTwo, Some(seed)) => {
                        let (a, b) = lineup(seed, [ids[0], ids[1]], [ids[2], ids[3]]);
                        (a, Some(b))
                    }
                    _ => ([ids[0], ids[1]], None),
                };
                let difficulty = match lobby.settings {
                    LobbySettings::Coop { difficulty } => difficulty,
                    // R3731002: a 2v2 has no AI team; its row stores the lowest tier, never read.
                    _ => Difficulty::Easy,
                };
                let outcome = start_coop_match(
                    app,
                    CoopStart {
                        match_id: match_id.clone(),
                        a1: team_a[0],
                        a2: team_a[1],
                        host_choice: &choices[at(team_a[0])],
                        guest_choice: &choices[at(team_a[1])],
                        difficulty,
                        seed: seed.clone(),
                        claim_room: None,
                        team_b: team_b.map(|b| CoopTeamB {
                            b1: b[0],
                            b2: b[1],
                            b1_choice: &choices[at(b[0])],
                            b2_choice: &choices[at(b[1])],
                        }),
                    },
                    |taken| refusal(&taken),
                )
                .await?;
                match outcome {
                    CoopStartOutcome::Started => Ok((Some(match_id), None)),
                    // No room is claimed, so neither is ever answered; read as a seat taken.
                    CoopStartOutcome::Blocked | CoopStartOutcome::Unavailable => Err(already_in_match()),
                }
            }
            .await
        }
    };
    let (match_id, series_id) = match began {
        Ok(began) => began,
        Err(error) => {
            let back = Lobby {
                started_at: None,
                ..starting.clone()
            };
            if let Err(revert) = write(app, &starting, back, &[]).await {
                tracing::error!(event = "lobby.start_revert_failed", code = %lobby.code, message = %revert.message);
            }
            return Err(error);
        }
    };
    let begun = Lobby {
        match_id: match_id.clone(),
        series_id: series_id.clone(),
        ..starting.clone()
    };
    if let Err(error) = write(app, &starting, begun, &[]).await {
        // The game has started; a guest's next read of the lobby misses only where it is.
        tracing::error!(event = "lobby.start_record_failed", code = %lobby.code, message = %error.message);
    }
    tracing::info!(
        event = "lobby.started",
        code = %lobby.code,
        matchId = ?match_id,
        seriesId = ?series_id,
    );
    Ok(ok(json!({ "matchId": match_id, "seriesId": series_id })))
}
