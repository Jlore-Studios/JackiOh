//! The actor registry (SURFACE §11.2).
//!
//! Manages match lifecycles and rebuilds:
//!  - `start` writes `matches` row and creates the actor (§9.2: one actor per match).
//!  - `actor_for` rebuilds missing actors by folding `(seed, log)` (SPEC §9.5), sound because §9.3
//!    makes `reduce` pure and the log append-only.
//!  - `stop` drops the in-memory actor, leaving the log alone: what a crash looks like outside.
//!
//! `api/queue.rs` and `rooms.rs` call only `start`, `has`, `stop` and `presence_of`.
//!
//! Finished actors remain in memory while rematch presence reads sockets, until `reap_finished`
//! drops them after the rematch window with no open sockets, or a restart drains them (SURFACE §11.3).

use std::panic::AssertUnwindSafe;
use std::sync::{Arc, Mutex};

use indexmap::IndexMap;
use tokio::sync::OnceCell;

use jackioh_engine::{PLAYER_IDS, PerPlayer, PlayerId, portrait_or_default};

use crate::actor::checkpoint;
use crate::actor::contracts::one_tx;
use crate::actor::engine;
use crate::actor::match_actor::{
    MatchActor, MatchActorInput, bot_moved_turns, bot_moved_turns_from, create_match_actor, last_boards_of,
};
use crate::actor::telemetry;
use crate::actor::ws_server::Socket;
use crate::api::http::{ApiError, ApiErrorCode, lock};
use crate::app::{App, now_ms};
use crate::bug_reports::{Finding, FoundIn, refused_rows};
use crate::config::{GLITCH_BOARDS_SAMPLED, MATCH_CEILING_MINUTES, REMATCH_OFFER_TTL_MS};
use crate::db::store::{
    BotRow, LastBoardKind, MatchClocks, MatchRow, MatchStatus, ProfileKind, QueueMode, StartMatchInput,
    StoreError, Tx,
};
use crate::loadout::SeatPicks;

/// Why `attach` could not hand the socket to a seat. `ws_server.rs` closes with 4404 for an
/// `Api` error whose code is `not_found`, 4403 for any other `Api` error, and 1011 for `Internal`.
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
            RebuildError::NotFound => {
                AttachError::Api(ApiError::new(ApiErrorCode::NotFound, "no such match"))
            }
            RebuildError::Internal(message) => AttachError::Internal(message),
        }
    }
}

type Rebuild = Arc<OnceCell<Result<MatchActor, RebuildError>>>;

/// Live actors by match id; a finished one stays until `reap_finished` or a restart (SURFACE §11.2).
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

/// String message from a panic payload.
pub(crate) fn panic_text(payload: Box<dyn std::any::Any + Send>) -> String {
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

/// R258, R433: an All Random match deals both decks, so `create_game` and every `fold` are told both
/// were dealt. The row has no mode, so it is read off what made the match (`matches_mode_of`), the
/// same at start and rebuild, as `timing-backfill` folds too (R1442).
pub(crate) fn dealt_for(mode: Option<QueueMode>) -> Option<Vec<PlayerId>> {
    if matches!(mode, Some(QueueMode::Random)) {
        Some(PLAYER_IDS.to_vec())
    } else {
        None
    }
}

/// R388, R767000: catalog live match is played under. One this build cannot fold falls back to this
/// build's, logged: without an actor its players would be held until the ceiling.
pub(crate) fn pinned_for(match_row: &MatchRow) -> engine::Pinned {
    engine::Pinned::of(&match_row.catalog_version).unwrap_or_else(|reason| {
        tracing::error!(
            event = "match.catalog.unpinned",
            matchId = %match_row.id,
            catalogVersion = %match_row.catalog_version,
            reason = %reason,
        );
        engine::Pinned::default()
    })
}

/// R1452: the ladder bot at each home seat of a match between `players` (seat order), or none for a
/// person. The bots' rows are read only when a seat's profile is a bot's, inside the caller's
/// transaction, so a match `start` makes and the same match `rebuild` folds read the same seats.
async fn bots_of(
    t: &mut Tx<'_>,
    players: &(String, String),
) -> Result<PerPlayer<Option<BotRow>>, StoreError> {
    let profiles = t
        .profiles_get_many(&[players.0.clone(), players.1.clone()])
        .await?;
    let is_bot = |id: &str| {
        profiles
            .iter()
            .any(|profile| profile.id == id && profile.kind == ProfileKind::Bot)
    };
    if !is_bot(&players.0) && !is_bot(&players.1) {
        return Ok(PerPlayer::new(None, None));
    }
    let bots = t.bots_list().await?;
    let bot = |id: &str| {
        is_bot(id)
            .then(|| bots.iter().find(|bot| bot.profile_id == id).cloned())
            .flatten()
    };
    Ok(PerPlayer::new(bot(&players.0), bot(&players.1)))
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
        let players = (first.profile_id.clone(), second.profile_id.clone());
        let (boards, sampled, derived_mode, bots, picks) = one_tx!(app.db, |t| {
            // R417: each seat's last server-match board, frozen on the match as it starts.
            let boards = (
                t.last_boards_get(&first.profile_id, LastBoardKind::Server)
                    .await?
                    .unwrap_or_default(),
                t.last_boards_get(&second.profile_id, LastBoardKind::Server)
                    .await?
                    .unwrap_or_default(),
            );
            // R678: two other players' last boards, never a seat's own, sampled once and frozen on
            // the row so every rebuild folds the same Glitch.
            let sampled = t
                .last_boards_sample_others(&profile_ids, GLITCH_BOARDS_SAMPLED as i64)
                .await?;
            // R433: mode read from tickets/room/series, the same before the row is written as
            // `rebuild` reads it after. R672: a rematch states its mode.
            let derived_mode = match input.mode {
                Some(mode) => Some(mode),
                None => t.matches_mode_of(&input.match_id).await?,
            };
            // R1452: the ladder bot at either seat, which the actor plays.
            let bots = bots_of(&mut t, &players).await?;
            // R1834002: what each seat's loadout freezes from, as the match starts; R1834005: a
            // bot's from its own row, through the same path.
            let picks = (
                SeatPicks::read(&mut t, &first.profile_id, bots.p1.as_ref()).await?,
                SeatPicks::read(&mut t, &second.profile_id, bots.p2.as_ref()).await?,
            );
            (boards, sampled, derived_mode, bots, picks)
        })?;

        let glitch_boards = (
            sampled.first().cloned().unwrap_or_default(),
            sampled.get(1).cloned().unwrap_or_default(),
        );
        let mut match_row = MatchRow {
            id: input.match_id.clone(),
            seed: input.seed.clone(),
            players: (first.profile_id.clone(), second.profile_id.clone()),
            decks: (first.deck.clone(), second.deck.clone()),
            catalog_version: input.catalog_version.clone(),
            ranked: Some(input.ranked),
            // R672, R1555000: rematch and bot-vs-bot matches state mode/stakes explicitly.
            mode: input.mode,
            stake: input.stake,
            status: MatchStatus::Live,
            created_at: now,
            finished_at: None,
            clocks: initial_clocks(now, MATCH_CEILING_MINUTES),
            last_boards: if boards.0.len() + boards.1.len() > 0 {
                Some(boards)
            } else {
                None
            },
            glitch_boards: if sampled.is_empty() {
                None
            } else {
                Some(glitch_boards)
            },
            // R642: the portraits the seats were dealt, frozen on the row so a rebuilt actor (and a
            // reconnected client) reads the same pair.
            portraits: Some((
                portrait_or_default(first.portrait.as_deref()),
                portrait_or_default(second.portrait.as_deref()),
            )),
            // R1834002: frozen below, once the opening shows which cards each deck dealt.
            loadouts: None,
            // R768: written with the result, never at the start.
            final_hash: None,
        };
        let dealt = dealt_for(derived_mode);
        // R388, R767000: the match is pinned to the catalog its row froze. A version this build
        // neither is nor carries is refused before anything is written, as a co-op join refuses
        // (`duos::rooms`).
        let pinned = engine::Pinned::of(&match_row.catalog_version).map_err(|reason| {
            ApiError::internal(format!(
                "catalog {} cannot be played: {reason}",
                match_row.catalog_version
            ))
        })?;

        // The opening draw is the engine's, not the log's (§9.3). It runs before the row is written,
        // so a game the engine cannot begin leaves no `live` row nothing but the ceiling could end;
        // an `open` skeleton stays for `matches_discard_open` (R263). The engine's panic is the error.
        let (last_boards, glitch) = last_boards_of(&match_row);
        let args = engine::create_game_args(&match_row.seed, &match_row.decks, last_boards, glitch, dealt);
        let state = std::panic::catch_unwind(AssertUnwindSafe(|| {
            engine::begin_game(pinned, &engine::create_game(pinned, &args)).state
        }))
        .map_err(|payload| ApiError::internal(panic_text(payload)))?;
        // R1834002: each seat's loadout, frozen on the row beside its portrait, over the cards its
        // deck dealt (an All Random seat's dealt deck among them).
        let dealt_cards = engine::dealt_cards(&state);
        let deck_cards = |seat: PlayerId| -> Vec<String> {
            dealt_cards
                .values()
                .filter(|(owner, _)| *owner == seat)
                .map(|(_, def)| def.clone())
                .collect()
        };
        match_row.loadouts = Some((
            picks
                .0
                .freeze(first.deck_slots.as_ref(), &deck_cards(PlayerId::P1)),
            picks
                .1
                .freeze(second.deck_slots.as_ref(), &deck_cards(PlayerId::P2)),
        ));

        one_tx!(app.db, |t| t.matches_create(&match_row).await?)?;
        let match_id = match_row.id.clone();
        let players = match_row.players.clone();
        let actor = create_match_actor(
            Arc::clone(app),
            MatchActorInput {
                match_row,
                state,
                log: Vec::new(),
                on_voided: Some(Registry::on_voided(app, &match_id)),
                telemetry: None,
                bots,
                bot_moved_turn: PerPlayer::new(None, None),
                // R641000: a series game draws its bot no rematch plan.
                series_game: derived_mode == Some(QueueMode::Bo3),
                pinned,
                dealt_cards,
            },
        );
        lock(&self.actors).insert(match_id.clone(), actor);
        tracing::info!(event = "match.started", matchId = %match_id, players = ?players);
        Ok(())
    }

    async fn rebuild(&self, app: &Arc<App>, match_id: &str) -> Result<MatchActor, RebuildError> {
        let store = |error: crate::db::store::StoreError| RebuildError::Internal(error.to_string());
        let (match_row, log, mode, bots, saved) = one_tx!(app.db, |t| {
            let match_row = t.matches_get(match_id).await?;
            let log = t.matches_actions(match_id).await?;
            let mode = t.matches_mode_of(match_id).await?;
            // R1452: a restart resumes a bot's seat as it resumes the match.
            let bots = match &match_row {
                Some(row) => bots_of(&mut t, &row.players).await?,
                None => PerPlayer::new(None, None),
            };
            // #1169: the checkpoint the last restart drain stored, read with the log it follows.
            let saved = match &match_row {
                Some(_) => t.matches_checkpoint(match_id).await?,
                None => None,
            };
            (match_row, log, mode, bots, saved)
        })
        .map_err(store)?;
        let Some(match_row) = match_row else {
            return Err(RebuildError::NotFound);
        };
        if match_row.status == MatchStatus::Finished && log.is_empty() {
            // R1285000: a finished match whose log the retention purge has taken (`api/retention.rs`):
            // nothing left to show, so nothing is folded or filed, as the co-op rebuild answers.
            return Err(RebuildError::NotFound);
        }
        let pinned = pinned_for(&match_row);

        let dealt = dealt_for(mode);
        let (last_boards, glitch_boards) = last_boards_of(&match_row);
        let args = engine::fold_args(
            &match_row.seed,
            &match_row.decks,
            log.iter().map(|row| row.action.clone()).collect(),
            last_boards,
            glitch_boards,
            dealt.clone(),
        );
        // R1834003: the cards each deck dealt, read off the opening, when a loadout names a card of
        // its own (a match from before 0061 names none, and sends no card's picks).
        let opening = match_row
            .loadouts
            .as_ref()
            .is_some_and(|(first, second)| !first.cards.is_empty() || !second.cards.is_empty())
            .then(|| {
                let (last_boards, glitch_boards) = last_boards_of(&match_row);
                engine::fold_args(
                    &match_row.seed,
                    &match_row.decks,
                    Vec::new(),
                    last_boards,
                    glitch_boards,
                    dealt.clone(),
                )
            });
        let live = match_row.status == MatchStatus::Live;
        let has_bot = bots.p1.is_some() || bots.p2.is_some();
        let saved_seq = saved.as_ref().map(|saved| saved.seq);
        let (row, rows) = (match_row.clone(), log.clone());
        // #1035: the three folds in one job off the runtime, as Duos's rebuild folds: a long log is a
        // long fold. A panic in the first is the rebuild's internal error.
        let (folded, replayed, bot_turns, from, dealt_cards) =
            engine::off_runtime(app.engine_thread, move || {
                let dealt_cards = opening
                    .map(|opening| engine::dealt_cards(&engine::fold(pinned, &opening).state))
                    .unwrap_or_default();
                // #1169: verified checkpoint folds only tail rows; otherwise folds full log.
                let verified =
                    saved.map(|saved| checkpoint::verify(&saved, &row, &rows).map(|body| (saved.seq, body)));
                if let Some(Ok((seq, body))) = verified {
                    let tail = &rows[usize::try_from(seq).unwrap_or(rows.len()).min(rows.len())..];
                    let folded = checkpoint::fold_tail(pinned, body.state.clone(), tail);
                    let replayed = live.then(|| {
                        std::panic::catch_unwind(AssertUnwindSafe(|| {
                            telemetry::replay_from(pinned, body.telemetry, body.state.clone(), tail).0
                        }))
                        .map_err(panic_text)
                    });
                    let bot_turns = (has_bot && live).then(|| {
                        std::panic::catch_unwind(AssertUnwindSafe(|| {
                            bot_moved_turns_from(pinned, body.state, tail, body.bot_moved_turn)
                        }))
                        .map_err(panic_text)
                    });
                    return (folded, replayed, bot_turns, Ok(seq), dealt_cards);
                }
                let folded = engine::fold(pinned, &args);
                // R1442: the play telemetry the log holds, so a rebuilt actor's think times carry on. A
                // match already finished has none left to write, and is not replayed twice.
                let replayed = live.then(|| {
                    std::panic::catch_unwind(AssertUnwindSafe(|| {
                        telemetry::replay(pinned, &row, &rows, dealt.clone()).0
                    }))
                    .map_err(panic_text)
                });
                // R639000: where each bot's turn stands, so a restart mid-turn does not begin it again.
                let bot_turns = (has_bot && live).then(|| {
                    std::panic::catch_unwind(AssertUnwindSafe(|| bot_moved_turns(pinned, &row, &rows, dealt)))
                        .map_err(panic_text)
                });
                (
                    folded,
                    replayed,
                    bot_turns,
                    Err(verified.and_then(Result::err)),
                    dealt_cards,
                )
            })
            .await
            .map_err(RebuildError::Internal)?;
        // #1169: only the reason, never the body, which holds both hands and both libraries.
        if let Err(Some(reason)) = from {
            tracing::warn!(
                event = "match.checkpoint.refused",
                matchId = %match_id,
                seq = saved_seq.unwrap_or(0),
                reason = reason.as_str(),
            );
        }
        // R1442: a replay that panics costs the telemetry alone: the actor records afresh from the
        // match's creation.
        let telemetry = match replayed {
            Some(Ok(recorder)) => Some(recorder),
            Some(Err(message)) => {
                tracing::warn!(event = "telemetry.replay_failed", matchId = %match_id, message = %message);
                None
            }
            None => None,
        };
        // R639000: a fold that panics costs that alone: the bot reads its next move as its turn's first.
        let bot_moved_turn = match bot_turns {
            Some(Ok(moved)) => moved,
            Some(Err(message)) => {
                tracing::warn!(event = "bot.turns.replay_failed", matchId = %match_id, message = %message);
                PerPlayer::new(None, None)
            }
            None => PerPlayer::new(None, None),
        };
        if !folded.errors.is_empty() {
            // Determinism break: engine refused already accepted action (§9.3). Rebuild anyway, a
            // live match beats a dead one, but file it for the night bot.
            tracing::error!(event = "match.fold.errors", matchId = %match_id, errors = ?folded.errors);
            app.bug_reports.file(
                app.env.deployed_commit.as_deref(),
                Finding {
                    match_id: match_id.to_string(),
                    found_in: FoundIn::Rebuild,
                    catalog_version: match_row.catalog_version.clone(),
                    actions: log.len(),
                    refused: refused_rows(&log, &folded.errors),
                },
            );
        }

        let actions = log.len();
        let actor = create_match_actor(
            Arc::clone(app),
            MatchActorInput {
                match_row,
                state: folded.state,
                log,
                on_voided: Some(Registry::on_voided(app, match_id)),
                telemetry,
                bots,
                bot_moved_turn,
                series_game: mode == Some(QueueMode::Bo3),
                pinned,
                dealt_cards,
            },
        );
        lock(&self.actors).insert(match_id.to_string(), actor.clone());
        let from_seq = from.unwrap_or(0);
        tracing::info!(event = "match.rebuilt", matchId = %match_id, actions = actions, fromSeq = from_seq);
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
            if rebuilding
                .get(match_id)
                .is_some_and(|current| Arc::ptr_eq(current, &cell))
            {
                rebuilding.shift_remove(match_id);
            }
        }
        outcome.map_err(AttachError::from)
    }

    pub fn has(&self, match_id: &str) -> bool {
        lock(&self.actors).contains_key(match_id)
    }

    /// R672: the seats' open sockets, or `None` once actor is gone. Never rebuilds.
    pub fn presence_of(&self, match_id: &str) -> Option<PerPlayer<bool>> {
        let actor = lock(&self.actors).get(match_id).cloned();
        actor.map(|actor| actor.presence())
    }

    /// R639000: actor in memory, never rebuilt; rematch offer notifies ladder bot.
    pub fn get(&self, match_id: &str) -> Option<MatchActor> {
        lock(&self.actors).get(match_id).cloned()
    }

    /// Drop the in-memory actor; the log stays. Used by the reaper and by tests.
    pub async fn stop(&self, match_id: &str) {
        let actor = lock(&self.actors).shift_remove(match_id);
        let Some(actor) = actor else { return };
        actor.stop().await;
        tracing::info!(event = "match.stopped", matchId = %match_id);
    }

    /// #861 drain: stops every actor as `stop` does, 1001 to open sockets, its queue drained so a
    /// result being written lands. `halt` sets `stopped` first, so `on_socket_gone` stores no grace.
    /// Next process rebuilds from log (§9.5) or checkpoint (#1169); unreturned seats start grace (R744).
    pub async fn stop_all(&self) -> usize {
        let actors: Vec<(String, MatchActor)> = lock(&self.actors).drain(..).collect();
        let count = actors.len();
        let mut stopping = tokio::task::JoinSet::new();
        for (match_id, actor) in actors {
            stopping.spawn(async move {
                // #1169: stored before 1001 frames for immediate reconnect.
                actor.checkpoint().await;
                actor.stop().await;
                tracing::info!(event = "match.stopped", matchId = %match_id);
            });
        }
        while stopping.join_next().await.is_some() {}
        count
    }

    /// Hands a socket to the seat this profile holds, rebuilding if not in memory (SURFACE §11.2).
    /// Socket is only told its own `view_for` (§10.8).
    pub async fn attach(
        &self,
        app: &Arc<App>,
        match_id: &str,
        profile_id: &str,
        socket: Socket,
    ) -> Result<(), AttachError> {
        let actor = self.actor_for(app, match_id).await?;
        // Identical to a missing match on purpose: a socket learns nothing about matches it is not a
        // player in (§9.1).
        let Some(seat) = actor.seat_of(profile_id) else {
            return Err(AttachError::Api(ApiError::new(
                ApiErrorCode::NotFound,
                "no such match",
            )));
        };
        actor.attach(seat, socket);
        Ok(())
    }

    /// R672: drops finished actors past rematch window (`REMATCH_OFFER_TTL_MS`) with no open sockets.
    /// Reaper calls every tick (`api::results::run_reaper`). Rebuilding handles returning sockets (§9.5).
    /// An open socket keeps its actor; the check and the removal share the map's lock.
    pub async fn reap_finished(&self, now: i64) -> Vec<String> {
        let reaped: Vec<(String, MatchActor)> = {
            let mut actors = lock(&self.actors);
            let due: Vec<String> = actors
                .iter()
                .filter(|(_, actor)| {
                    let presence = actor.presence();
                    actor
                        .finished_at()
                        .is_some_and(|finished_at| now - finished_at > REMATCH_OFFER_TTL_MS)
                        && PLAYER_IDS.into_iter().all(|home| !presence[home])
                })
                .map(|(match_id, _)| match_id.clone())
                .collect();
            due.into_iter()
                .filter_map(|match_id| actors.shift_remove(&match_id).map(|actor| (match_id, actor)))
                .collect()
        };
        let mut ids = Vec::with_capacity(reaped.len());
        for (match_id, actor) in reaped {
            actor.stop().await;
            tracing::info!(event = "match.evicted", matchId = %match_id);
            ids.push(match_id);
        }
        ids
    }

    /// The match ids currently held in memory.
    pub fn live(&self) -> Vec<String> {
        lock(&self.actors).keys().cloned().collect()
    }
}

// Social: watching and shared replays (#939, #940)

impl Registry {
    /// B50: hands a watcher's socket to friend's seat, rebuilding if not in memory; `social::watch`
    /// has judged it may watch. A seatless friend reads as a missing match, a full match a conflict.
    pub async fn attach_watcher(
        &self,
        app: &Arc<App>,
        match_id: &str,
        friend_id: &str,
        watcher_id: &str,
        socket: Socket,
    ) -> Result<(), AttachError> {
        let actor = self.actor_for(app, match_id).await?;
        let Some(home) = actor.seat_of(friend_id) else {
            return Err(AttachError::Api(ApiError::new(
                ApiErrorCode::NotFound,
                "no such match",
            )));
        };
        actor
            .attach_watcher(home, watcher_id, socket)
            .map_err(|refused| AttachError::Api(ApiError::new(ApiErrorCode::Conflict, refused.message())))
    }

    /// B51, S7's `watchable`: how many sockets watch the match now; none for one not in memory.
    pub fn watcher_count(&self, match_id: &str) -> usize {
        self.get(match_id).map_or(0, |actor| actor.watcher_count())
    }

    /// B50, B46: drops revoked watchers in memory; returns count evicted.
    pub fn revoke_watchers(&self, revoked: impl Fn(&str, &str) -> bool) -> usize {
        let actors: Vec<MatchActor> = lock(&self.actors).values().cloned().collect();
        actors.iter().map(|actor| actor.revoke_watchers(&revoked)).sum()
    }

    /// R1351000: mutes emotes between blocked users across live matches; returns count muted. A
    /// match not in memory reads the block as its rebuilt actor arms.
    pub fn mute_emotes_between(&self, x: &str, y: &str) -> usize {
        let actors: Vec<MatchActor> = lock(&self.actors).values().cloned().collect();
        actors
            .iter()
            .filter(|actor| actor.mute_emotes_between(x, y))
            .count()
    }
}
