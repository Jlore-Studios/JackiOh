//! The live co-op actors (fullsend SPEC S6, B40), as `actor/registry.rs` holds the 1v1 ones: `start`
//! creates an actor for a match a join just wrote, `actor_for` rebuilds one that is not in memory by
//! folding `(seed, options, log)` (`duos_fold`), and `stop` drops one and leaves its log alone, which
//! is what a crash looks like from outside.
//!
//! One registry per running `App`, reached through `registry(app)`. It is not a field of `App`, so
//! nothing that builds an `App` changes: it lives in a map keyed by the app's address, beside a weak
//! handle that tells a live app from one dropped and its address reused, as `ws_server.rs` keeps its
//! per-app socket counts. An actor holds its `Arc<App>`, so an app is never dropped while its
//! registry holds an actor.

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::{Arc, LazyLock, Mutex, Weak};

use indexmap::IndexMap;
use tokio::sync::OnceCell;

use jackioh_engine::duos::{DuosReplayInput, DuosState};
use jackioh_engine::wire::{DuosAction, DuosSeat};

use crate::actor::contracts::one_tx;
use crate::actor::registry::panic_text;
use crate::actor::ws_server::Socket;
use crate::api::http::lock;
use crate::app::App;
use crate::duos::actor::{DuosActor, DuosActorInput, create_duos_actor};
use crate::duos::engine;
use crate::duos::store::{DuosActionRow, DuosMatch, action_of};

/// Why a socket could not be handed to a co-op seat.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DuosAttachError {
    /// No such match, or the profile holds no seat in it: one answer, so a socket learns nothing
    /// about a match it is not a player in (§9.1).
    NotFound,
    /// The store failed, or the log no longer folds.
    Internal(String),
}

impl std::fmt::Display for DuosAttachError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DuosAttachError::NotFound => f.write_str("no such match"),
            DuosAttachError::Internal(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for DuosAttachError {}

type Rebuild = Arc<OnceCell<Result<DuosActor, DuosAttachError>>>;

/// Live co-op actors by match id.
#[derive(Default)]
pub struct DuosRegistry {
    actors: Mutex<IndexMap<String, DuosActor>>,
    /// In-flight rebuilds, so two sockets arriving together fold the log once.
    rebuilding: Mutex<IndexMap<String, Rebuild>>,
}

/// Every running app's registry, beside a weak handle on its app.
static REGISTRIES: LazyLock<Mutex<Vec<(Weak<App>, Arc<DuosRegistry>)>>> = LazyLock::new(|| Mutex::new(Vec::new()));

/// The co-op registry of `app`, made on first use. A registry whose app is gone is dropped here.
pub fn registry(app: &Arc<App>) -> Arc<DuosRegistry> {
    let mut held = lock(&REGISTRIES);
    held.retain(|(owner, _)| owner.strong_count() > 0);
    if let Some((_, found)) = held
        .iter()
        .find(|(owner, _)| std::ptr::eq(owner.as_ptr(), Arc::as_ptr(app)))
    {
        return Arc::clone(found);
    }
    let made = Arc::new(DuosRegistry::default());
    held.push((Arc::downgrade(app), Arc::clone(&made)));
    made
}

/// A log row as the engine folds it.
fn logged_actions(log: &[DuosActionRow]) -> Result<Vec<DuosAction>, DuosAttachError> {
    log.iter()
        .map(|row| {
            action_of(&row.action).map_err(|error| {
                DuosAttachError::Internal(format!("log row {} does not read: {error}", row.seq))
            })
        })
        .collect()
}

impl DuosRegistry {
    /// Creates the actor of a match whose row a join just wrote (B33), on the state it began with.
    pub fn start(&self, app: &Arc<App>, row: DuosMatch, state: DuosState) -> DuosActor {
        let match_id = row.id.clone();
        let actor = create_duos_actor(
            Arc::clone(app),
            DuosActorInput {
                row,
                state,
                log: Vec::new(),
            },
        );
        lock(&self.actors).insert(match_id.clone(), actor.clone());
        tracing::info!(event = "duos.started", matchId = %match_id);
        actor
    }

    /// B40: a crashed actor rebuilt by folding the match's log.
    async fn rebuild(&self, app: &Arc<App>, match_id: &str) -> Result<DuosActor, DuosAttachError> {
        let store = |error: crate::db::store::StoreError| DuosAttachError::Internal(error.to_string());
        let (row, log) = one_tx!(app.db, |t| {
            let row = t.duos_matches_get(match_id).await?;
            let log = match &row {
                Some(_) => t.duos_actions(match_id).await?,
                None => Vec::new(),
            };
            (row, log)
        })
        .map_err(store)?;
        let Some(row) = row else {
            return Err(DuosAttachError::NotFound);
        };
        let input = DuosReplayInput {
            options: engine::create_options(&row),
            log: logged_actions(&log)?,
        };
        let folded = catch_unwind(AssertUnwindSafe(|| engine::fold(&input)))
            .map_err(|payload| DuosAttachError::Internal(panic_text(payload)))?;
        if let Some(at) = folded.refused_at {
            // An action the engine once applied and now refuses is a determinism break: the log no
            // longer folds the match. Rebuild anyway, on the state up to it, and say so loudly.
            tracing::error!(event = "duos.fold.errors", matchId = %match_id, refusedAt = at);
        }
        let actions = log.len();
        let actor = create_duos_actor(
            Arc::clone(app),
            DuosActorInput {
                row,
                state: folded.state,
                log,
            },
        );
        lock(&self.actors).insert(match_id.to_string(), actor.clone());
        tracing::info!(event = "duos.rebuilt", matchId = %match_id, actions = actions);
        Ok(actor)
    }

    /// The live actor, folded back from the store when it is not in memory.
    pub async fn actor_for(&self, app: &Arc<App>, match_id: &str) -> Result<DuosActor, DuosAttachError> {
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
        outcome
    }

    /// B35: hands a socket to the seat `profile_id` holds, rebuilding the actor first when needed.
    /// The seat's view and legal actions follow on the actor's queue.
    pub async fn attach(
        &self,
        app: &Arc<App>,
        match_id: &str,
        profile_id: &str,
        socket: Socket,
    ) -> Result<DuosSeat, DuosAttachError> {
        let actor = self.actor_for(app, match_id).await?;
        let Some(seat) = actor.seat_of(profile_id) else {
            return Err(DuosAttachError::NotFound);
        };
        actor.attach(seat, socket);
        Ok(seat)
    }

    /// The actor in memory, never rebuilt.
    pub fn get(&self, match_id: &str) -> Option<DuosActor> {
        lock(&self.actors).get(match_id).cloned()
    }

    pub fn has(&self, match_id: &str) -> bool {
        lock(&self.actors).contains_key(match_id)
    }

    /// The match ids held in memory.
    pub fn live(&self) -> Vec<String> {
        lock(&self.actors).keys().cloned().collect()
    }

    /// Lets the actor go without waiting on its queue (an actor letting itself go calls this).
    pub(crate) fn forget(&self, match_id: &str) {
        lock(&self.actors).shift_remove(match_id);
        tracing::info!(event = "duos.forgotten", matchId = %match_id);
    }

    /// Drops the in-memory actor; the log stays, and the next socket folds it back.
    pub async fn stop(&self, match_id: &str) {
        let actor = lock(&self.actors).shift_remove(match_id);
        let Some(actor) = actor else { return };
        actor.stop().await;
        tracing::info!(event = "duos.stopped", matchId = %match_id);
    }

    /// Drops every finished actor no person holds a socket on, as the 1v1 reaper drops one: a
    /// socket that comes back folds it again. Answers the ids it dropped.
    pub async fn reap_finished(&self) -> Vec<String> {
        let reaped: Vec<(String, DuosActor)> = {
            let mut actors = lock(&self.actors);
            let due: Vec<String> = actors
                .iter()
                .filter(|(_, actor)| actor.is_finished() && !actor.has_open_socket())
                .map(|(match_id, _)| match_id.clone())
                .collect();
            due.into_iter()
                .filter_map(|match_id| actors.shift_remove(&match_id).map(|actor| (match_id, actor)))
                .collect()
        };
        let mut ids = Vec::with_capacity(reaped.len());
        for (match_id, actor) in reaped {
            actor.stop().await;
            tracing::info!(event = "duos.evicted", matchId = %match_id);
            ids.push(match_id);
        }
        ids
    }
}
