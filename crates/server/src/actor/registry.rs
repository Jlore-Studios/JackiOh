//! The actor registry: TS's `MatchDirectory` (`src/api/ports.ts`) plus the two things a socket
//! needs. Port of `apps/server/src/match/registry.ts`; its API is SURFACE §11.2's.
//!
//! It is the only place that knows how a match comes into existence and how a crashed one comes
//! back:
//!
//!  - `start` writes the `matches` row and creates the actor (§9.2: one actor per match).
//!  - `actor_for` rebuilds an actor that is not in memory by folding the log — SPEC §9.5: "A crashed
//!    actor rebuilds its state by folding `(seed, log)`", which is only sound because §9.3 makes
//!    `reduce` pure and the log append-only.
//!  - `stop` drops the in-memory actor and leaves the log alone, which is exactly what a crash
//!    looks like from the outside (and how `recovery.rs` kills one mid-game).
//!
//! `api/queue.rs` and `rooms.rs` only ever call `start`, `has`, `stop` and `presence_of`, so neither
//! reaches into the actor. Finished actors stay in the map until the reaper or a restart
//! (SURFACE §11.3): a rematch's presence reads their sockets.

use std::panic::AssertUnwindSafe;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use indexmap::IndexMap;
use tokio::sync::OnceCell;

use jackioh_engine::{PLAYER_IDS, PerPlayer, PlayerId, portrait_or_default};

use crate::actor::contracts::one_tx;
use crate::actor::ws_server::Socket;
use crate::actor::engine;
use crate::actor::match_actor::{MatchActor, MatchActorInput, create_match_actor, last_boards_of};
use crate::api::http::{ApiError, ApiErrorCode};
use crate::app::{App, now_ms};
use crate::config::{GLITCH_BOARDS_SAMPLED, MATCH_CEILING_MINUTES};
use crate::db::store::{LastBoardKind, MatchClocks, MatchRow, MatchStatus, QueueMode, StartMatchInput};

/// Why `attach` could not hand the socket to a seat. `ws_server.rs` closes with 4404 for an
/// `Api` error whose code is `not_found`, 4403 for any other `Api` error, and 1011 for `Internal`
/// (TS: an `ApiError` by its status, anything else as an internal fault).
#[derive(Debug)]
pub enum AttachError {
    Api(ApiError),
    Internal(String),
}

impl std::fmt::Display for AttachError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AttachError::Api(error) => f.write_str(&error.message),
            AttachError::Internal(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for AttachError {}

/// A rebuild's failure, kept so every caller awaiting the same rebuild hears the same answer.
#[derive(Clone, Debug)]
enum RebuildError {
    NotFound,
    Internal(String),
}

impl From<RebuildError> for AttachError {
    fn from(error: RebuildError) -> AttachError {
        match error {
            RebuildError::NotFound => AttachError::Api(api_error(ApiErrorCode::NotFound, "no such match")),
            RebuildError::Internal(message) => AttachError::Internal(message),
        }
    }
}

type Rebuild = Arc<OnceCell<Result<MatchActor, RebuildError>>>;

/// Live actors by match id; finished actors stay until the reaper or a restart (SURFACE §11.2).
pub struct Registry {
    actors: Mutex<IndexMap<String, MatchActor>>,
    /// In-flight rebuilds, so two sockets arriving together fold the log once, not twice.
    rebuilding: Mutex<IndexMap<String, Rebuild>>,
}

impl Default for Registry {
    fn default() -> Registry {
        Registry::new()
    }
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

fn api_error(code: ApiErrorCode, message: &str) -> ApiError {
    ApiError {
        code,
        message: message.to_string(),
        details: None,
        retry_after_ms: None,
    }
}

fn internal(error: impl std::fmt::Display) -> ApiError {
    api_error(ApiErrorCode::Internal, &error.to_string())
}

/// What a panic carried, as TS's thrown message.
fn panic_text(payload: Box<dyn std::any::Any + Send>) -> String {
    if let Some(text) = payload.downcast_ref::<&str>() {
        (*text).to_string()
    } else if let Some(text) = payload.downcast_ref::<String>() {
        text.clone()
    } else {
        "the engine panicked".to_string()
    }
}

/// R79: the hard wall-clock ceiling is measured from the moment the match row is written.
fn initial_clocks(now: i64, ceiling_minutes: i64) -> MatchClocks {
    MatchClocks {
        turn_deadline: None,
        prompt_deadline: None,
        grace_deadline: PerPlayer::new(None, None),
        ceiling_at: now + ceiling_minutes * 60_000,
    }
}

/// R258, R433: an All Random match deals both seats' decks, so `create_game` and every rebuild's
/// `fold` are told both seats were dealt, and neither player's own library lists a card they were
/// not shown going in. The match row does not record its mode, so it is read off what made the
/// match (`matches_mode_of`, as `api/game_records.rs` files a record): the same answer at the start
/// and at every rebuild, so a rebuilt actor folds the same game. Every other mode's decks were built.
fn dealt_for(mode: Option<QueueMode>) -> Option<Vec<PlayerId>> {
    if matches!(mode, Some(QueueMode::Random)) { Some(PLAYER_IDS.to_vec()) } else { None }
}

impl Registry {
    pub fn new() -> Registry {
        Registry {
            actors: Mutex::new(IndexMap::new()),
            rebuilding: Mutex::new(IndexMap::new()),
        }
    }

    /// R679: a voided match is gone, so its actor leaves the map without `stop` (which waits on the
    /// actor's own queue, and the void runs inside it). A series game started again under the same id
    /// then finds no actor in the way.
    pub(crate) fn forget(&self, match_id: &str) {
        lock(&self.actors).shift_remove(match_id);
        tracing::info!(event = "match.forgotten", matchId = %match_id);
    }

    fn on_voided(app: &Arc<App>, match_id: &str) -> Box<dyn Fn() + Send + Sync> {
        let app = Arc::clone(app);
        let match_id = match_id.to_string();
        Box::new(move || app.matches.forget(&match_id))
    }

    /// Writes the `matches` row and creates the actor (§9.2). Called by queue pairing, a room's
    /// join, a series' game and a rematch.
    pub async fn start(&self, app: &Arc<App>, input: StartMatchInput) -> Result<(), ApiError> {
        let (first, second) = &input.seats;
        let now = now_ms();
        let profile_ids = vec![first.profile_id.clone(), second.profile_id.clone()];
        let (boards, sampled, derived_mode) = one_tx!(app.db, |t| {
            // R417: each seat's last server-match board, frozen on the match as it starts.
            let boards = (
                t.last_boards_get(&first.profile_id, LastBoardKind::Server).await?.unwrap_or_default(),
                t.last_boards_get(&second.profile_id, LastBoardKind::Server).await?.unwrap_or_default(),
            );
            // R678: two other players' last server boards — never either seat's own — sampled once,
            // here, and frozen on the row beside the decks, so every rebuild folds the same Glitch. A
            // seat with no board to sample gets the empty one; with none at all the field is left off.
            let sampled = t.last_boards_sample_others(&profile_ids, GLITCH_BOARDS_SAMPLED as i64).await?;
            // R433: the mode is read off what made the match — its tickets, its room or its series,
            // which the reserved `open` skeleton already links — so it reads the same before the row
            // is written as `rebuild` reads it after, and both fold the same setup. R672: a rematch
            // states its mode instead, since nothing made it but the finished match.
            let derived_mode = match input.mode {
                Some(mode) => Some(mode),
                None => t.matches_mode_of(&input.match_id).await?,
            };
            (boards, sampled, derived_mode)
        })
        .map_err(internal)?;

        let glitch_boards = (sampled.first().cloned().unwrap_or_default(), sampled.get(1).cloned().unwrap_or_default());
        let match_row = MatchRow {
            id: input.match_id.clone(),
            seed: input.seed.clone(),
            players: (first.profile_id.clone(), second.profile_id.clone()),
            decks: (first.deck.clone(), second.deck.clone()),
            catalog_version: input.catalog_version.clone(),
            ranked: Some(input.ranked),
            // R672: only a rematch states its mode and stakes — which no ticket, room or series made —
            // so only it lands on the row; every other start derives the mode as before.
            mode: input.mode,
            stake: input.stake,
            status: MatchStatus::Live,
            created_at: now,
            finished_at: None,
            clocks: initial_clocks(now, MATCH_CEILING_MINUTES as i64),
            last_boards: if boards.0.len() + boards.1.len() > 0 { Some(boards) } else { None },
            glitch_boards: if sampled.is_empty() { None } else { Some(glitch_boards) },
            // R642: the portraits the seats were dealt, frozen on the row so a rebuilt actor (and a
            // reconnected client) reads the same pair.
            portraits: Some((
                portrait_or_default(first.portrait.as_deref()),
                portrait_or_default(second.portrait.as_deref()),
            )),
        };
        let dealt = dealt_for(derived_mode);

        // The opening draw is part of the engine, not of the log: `fold` replays `create_game` and
        // `begin_game` from `(seed, decks)` before it applies a single action (§9.3). It runs before
        // the row is written so a game the engine cannot begin leaves nothing behind: written first,
        // the row went `live` with no actor and no log, and nothing but the ceiling reaper could ever
        // end it — while an `open` skeleton a caller reserved stays `open`, which
        // `matches_discard_open` (R263) is still able to release. (TS's `createGame` threw on a deck
        // it refused; the engine panics, and the panic is that error.)
        let (last_boards, glitch) = last_boards_of(&match_row);
        let args = engine::create_game_args(&match_row.seed, &match_row.decks, last_boards, glitch, dealt);
        let state = std::panic::catch_unwind(AssertUnwindSafe(|| engine::begin_game(&engine::create_game(&args)).state))
            .map_err(|payload| internal(panic_text(payload)))?;

        one_tx!(app.db, |t| t.matches_create(&match_row).await?).map_err(internal)?;
        let match_id = match_row.id.clone();
        let players = match_row.players.clone();
        let actor = create_match_actor(
            Arc::clone(app),
            MatchActorInput {
                match_row,
                state: Some(state),
                log: Vec::new(),
                on_voided: Some(Registry::on_voided(app, &match_id)),
            },
        );
        lock(&self.actors).insert(match_id.clone(), actor);
        tracing::info!(event = "match.started", matchId = %match_id, players = ?players);
        Ok(())
    }

    async fn rebuild(&self, app: &Arc<App>, match_id: &str) -> Result<MatchActor, RebuildError> {
        let store = |error: crate::db::store::StoreError| RebuildError::Internal(error.to_string());
        let (match_row, log, mode) = one_tx!(app.db, |t| {
            let match_row = t.matches_get(match_id).await?;
            let log = t.matches_actions(match_id).await?;
            let mode = t.matches_mode_of(match_id).await?;
            (match_row, log, mode)
        })
        .map_err(store)?;
        let Some(match_row) = match_row else { return Err(RebuildError::NotFound) };

        let dealt = dealt_for(mode);
        let (last_boards, glitch_boards) = last_boards_of(&match_row);
        let args = engine::fold_args(
            &match_row.seed,
            &match_row.decks,
            log.iter().map(|row| row.action.clone()).collect(),
            last_boards,
            glitch_boards,
            dealt,
        );
        let folded = std::panic::catch_unwind(AssertUnwindSafe(|| engine::fold(&args)))
            .map_err(|payload| RebuildError::Internal(panic_text(payload)))?;
        if !folded.errors.is_empty() {
            // An action the engine once accepted and now refuses is a determinism break: the log no
            // longer reconstructs the match (§9.3). Rebuild anyway — a live match is better than a dead
            // one — but say so loudly.
            tracing::error!(event = "match.fold.errors", matchId = %match_id, errors = ?folded.errors);
        }

        let actions = log.len();
        let actor = create_match_actor(
            Arc::clone(app),
            MatchActorInput {
                match_row,
                state: Some(folded.state),
                log,
                on_voided: Some(Registry::on_voided(app, match_id)),
            },
        );
        lock(&self.actors).insert(match_id.to_string(), actor.clone());
        tracing::info!(event = "match.rebuilt", matchId = %match_id, actions = actions);
        Ok(actor)
    }

    /// The live actor, folding `(seed, log)` when it is not in memory (§9.5).
    pub async fn actor_for(&self, app: &Arc<App>, match_id: &str) -> Result<MatchActor, AttachError> {
        if let Some(existing) = lock(&self.actors).get(match_id).cloned() {
            return Ok(existing);
        }

        let (cell, mine) = {
            let mut rebuilding = lock(&self.rebuilding);
            match rebuilding.get(match_id) {
                Some(in_flight) => (Arc::clone(in_flight), false),
                None => {
                    let cell: Rebuild = Arc::new(OnceCell::new());
                    rebuilding.insert(match_id.to_string(), Arc::clone(&cell));
                    (cell, true)
                }
            }
        };
        let outcome = cell.get_or_init(|| self.rebuild(app, match_id)).await.clone();
        if mine {
            let mut rebuilding = lock(&self.rebuilding);
            if rebuilding.get(match_id).is_some_and(|current| Arc::ptr_eq(current, &cell)) {
                rebuilding.shift_remove(match_id);
            }
        }
        outcome.map_err(AttachError::from)
    }

    pub fn has(&self, match_id: &str) -> bool {
        lock(&self.actors).contains_key(match_id)
    }

    /// R672: the seats' open sockets, or `None` once the actor is gone (a restart, the reaper).
    /// Never rebuilds: presence is about who is here now, not who could be folded back.
    pub fn presence_of(&self, match_id: &str) -> Option<PerPlayer<bool>> {
        let actor = lock(&self.actors).get(match_id).cloned();
        actor.map(|actor| actor.presence())
    }

    /// Drop the in-memory actor; the log stays. Used by the reaper and by tests.
    pub async fn stop(&self, match_id: &str) {
        let actor = lock(&self.actors).shift_remove(match_id);
        let Some(actor) = actor else { return };
        actor.stop().await;
        tracing::info!(event = "match.stopped", matchId = %match_id);
    }

    /// Hands a socket to the seat this profile holds, rebuilding the actor first if it is not in
    /// memory. The socket itself is never told anything but its own `view_for` (§10.8). (TS resolved
    /// with the seat for the caller's log; SURFACE §11.2 answers `()`, and the actor logs the seat.)
    pub async fn attach(&self, app: &Arc<App>, match_id: &str, profile_id: &str, socket: Socket) -> Result<(), AttachError> {
        let actor = self.actor_for(app, match_id).await?;
        // Identical to a missing match on purpose: a socket learns nothing about matches it is not a
        // player in (§9.1).
        let Some(seat) = actor.seat_of(profile_id) else {
            return Err(AttachError::Api(api_error(ApiErrorCode::NotFound, "no such match")));
        };
        actor.attach(seat, socket);
        Ok(())
    }

    /// The match ids currently held in memory.
    pub fn live(&self) -> Vec<String> {
        lock(&self.actors).keys().cloned().collect()
    }
}
