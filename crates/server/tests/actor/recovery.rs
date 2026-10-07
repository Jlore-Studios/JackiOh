//! BUILD M6-T4 acceptance, item 2: "killing the actor mid-game and reconnecting yields the same
//! `viewFor` for both players". It is docs/v0.3.0/README.md §6's V15 (a match survives a restart by
//! folding `(seed, log)`) and, on the server, V9 (the fold equals the live state).
//!
//! SPEC §9.5: "A crashed actor rebuilds its state by folding `(seed, log)`", and §9.3: "`(seed, log)`
//! reconstructs any match". Dropping the actor out of the registry and leaving the log alone is
//! exactly what a crash looks like from the outside (`registry.stop`), so that is the kill used here.
//!
//! This file drives the *real* clock (`actor/clock.rs`) on tokio's paused clock: the view carries
//! `clockMs` (R79), so a stub that always answers `null` would hide the very field a rebuild is most
//! likely to get wrong. No time is advanced across the crash, which is M6-T4's claim; re-arming a
//! clock from the deadlines stored on the match is M7-T1's (`docs/architecture.md` §5.2, "re-arm
//! the clocks from the stored deadlines").
//!
//! The item is made twice, for the same reason `match_actor.rs` makes its item 4 twice: the first
//! block runs the scripted cards, which reach a mid-prompt state in four actions, and the block at
//! the bottom deals real cards, because "the same `viewFor`" is not a claim scripted cards can
//! settle.
//!
//! Port of `apps/server/test/match/recovery.test.ts`. The Rust server has no scripted engine and no
//! injected results writer (SURFACE §11.3): the scripted block runs the real engine with
//! `support::engine`'s test cards (dealt into the opening hands by a searched seed, both mulligans
//! answered keeping everything before any socket attaches, which is where TS's scripted game
//! began), and TS's `recordResult` spy is the real results writer, read back off the `results`
//! table (both profiles are seeded so it can write).

use std::any::Any;
use std::sync::Arc;
use std::time::Duration;

use serde_json::{Value, json};

use jackioh_engine::CreateGameArgs;
use jackioh_engine::wire::{Phase, PlayerId};
use jackioh_server::actor::match_actor::MatchActor;
use jackioh_server::api::results::reap_stuck_matches;
use jackioh_server::app::App;
use jackioh_server::config::{
    DISCONNECT_GRACE_SECONDS, MATCH_CEILING_MINUTES, PROMPT_CLOCK_SECONDS, TURN_CLOCK_SECONDS,
};
use jackioh_server::db::fake::FakeData;
use jackioh_server::db::store::Db;

use crate::support::deps::empty_test_app;
use crate::support::engine::{fake_deck, install_test_cards};
use crate::support::socket::{FakeSocket, create_fake_socket};

const MATCH_ID: &str = "match-recovery";

/// How many seeds `seed_dealing` tries before it gives up.
const SEED_SEARCH: usize = 5_000;

/// One store call in its own transaction, as TS's `deps.store.<sub>.<method>(…)` was.
macro_rules! store {
    ($app:expr, $t:ident => $call:expr) => {{
        let mut $t = $app
            .db
            .begin(None)
            .await
            .expect("the fake store opens a transaction");
        let out = $call;
        $t.commit().await.expect("the fake store commits");
        out
    }};
}

fn from<T: serde::de::DeserializeOwned>(value: Value) -> T {
    serde_json::from_value(value.clone()).unwrap_or_else(|error| panic!("{error}: {value}"))
}

fn to_json<T: serde::Serialize>(value: &T) -> Value {
    serde_json::to_value(value).expect("a serialisable value")
}

/// The fake store behind the test app (SURFACE §11.2's `Db::Fake`).
fn fake(app: &App) -> Arc<tokio::sync::Mutex<FakeData>> {
    match &app.db {
        Db::Fake(data) => Arc::clone(data),
        _ => panic!("the test app runs on the fake store"),
    }
}

/// One table of the fake store, as TS's `deps.store.tables.<name>` read it: rows as JSON.
async fn table(app: &App, pick: impl Fn(&FakeData) -> Value) -> Vec<Value> {
    let data = fake(app);
    let data = data.lock().await;
    match pick(&data) {
        Value::Array(rows) => rows,
        other => panic!("not a table: {other}"),
    }
}

// ---------------------------------------------------------------------------------------------
// Time and logs: TS's manual `Timers` and recording `Logger`
// ---------------------------------------------------------------------------------------------

/// TS's `deps.timers.now()`: the server's clock, read as the epoch-ms stamp it wrote on the match
/// row at the start plus the tokio time (paused, moved only by `advance`) since.
#[derive(Clone, Copy)]
struct Clock {
    base_ms: i64,
    at: tokio::time::Instant,
}

impl Clock {
    fn now(&self) -> i64 {
        self.base_ms + i64::try_from(self.at.elapsed().as_millis()).expect("milliseconds fit")
    }
}

/// TS's `deps.timers.advance(ms)`: every timer due fires, and the woken actor gets to run.
async fn advance(ms: i64) {
    tokio::time::advance(Duration::from_millis(
        u64::try_from(ms).expect("time moves forward"),
    ))
    .await;
    tokio::task::yield_now().await;
}

/// Every `tracing` line the server writes on this thread while the guard lives, as JSON. On the
/// current-thread runtime `#[tokio::test]` builds, the actor's tasks run on this thread too.
#[derive(Clone, Default)]
struct Logs(Arc<std::sync::Mutex<Vec<u8>>>);

struct LogWriter(Arc<std::sync::Mutex<Vec<u8>>>);

impl std::io::Write for LogWriter {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.lock().expect("the log buffer").extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for Logs {
    type Writer = LogWriter;

    fn make_writer(&'a self) -> LogWriter {
        LogWriter(Arc::clone(&self.0))
    }
}

impl Logs {
    fn record() -> (Logs, tracing::subscriber::DefaultGuard) {
        let logs = Logs::default();
        let subscriber = tracing_subscriber::fmt()
            .json()
            .flatten_event(true)
            .with_max_level(tracing::Level::TRACE)
            .with_writer(logs.clone())
            .finish();
        let guard = crate::support::deps::set_log_default(subscriber);
        (logs, guard)
    }

    fn entries(&self) -> Vec<Value> {
        let bytes = self.0.lock().expect("the log buffer").clone();
        String::from_utf8_lossy(&bytes)
            .lines()
            .filter_map(|line| serde_json::from_str(line).ok())
            .collect()
    }

    /// Whether any line names `event` (SURFACE §11.3 keeps TS's event names).
    fn has(&self, event: &str) -> bool {
        self.entries().iter().any(|entry| event_of(entry) == Some(event))
    }

    fn count(&self, event: &str) -> usize {
        self.entries()
            .iter()
            .filter(|entry| event_of(entry) == Some(event))
            .count()
    }
}

fn event_of(entry: &Value) -> Option<&str> {
    ["event", "message"].iter().find_map(|key| {
        entry
            .get(*key)
            .or_else(|| entry.get("fields").and_then(|fields| fields.get(*key)))
            .and_then(Value::as_str)
    })
}

// ---------------------------------------------------------------------------------------------
// Frames
// ---------------------------------------------------------------------------------------------

fn views(socket: &FakeSocket) -> Vec<Value> {
    socket
        .of_type("view")
        .into_iter()
        .map(|frame| frame["view"].clone())
        .collect()
}

fn last_view(socket: &FakeSocket) -> Value {
    views(socket)
        .pop()
        .unwrap_or_else(|| panic!("no view frame was sent"))
}

fn acks(socket: &FakeSocket) -> Vec<Value> {
    socket.of_type("ack")
}

async fn send(actor: &MatchActor, socket: &FakeSocket, nonce: &str, body: Value) {
    let mut action = body;
    action["nonce"] = json!(nonce);
    socket.receive_json(json!({ "type": "action", "action": action }));
    actor.idle().await;
}

/// The answer to the seat's open prompt, read off the seat's own legal array. TS sent
/// `{ type: "answer", choiceId, selection: [{ pick: "none" }] }`, its scripted port's one answer; the
/// scripted card's prompt is the engine's own now (`choose_mode`), which takes one of its options.
fn answer_of(socket: &FakeSocket) -> Value {
    let choice = open_choice(socket);
    socket
        .of_type("view")
        .pop()
        .and_then(|frame| frame["legal"].as_array().cloned())
        .unwrap_or_default()
        .into_iter()
        .find(|action| action["type"] == "answer" && action["choiceId"] == choice.as_str())
        .unwrap_or_else(|| panic!("the seat may answer its prompt {choice}"))
}

/// The choiceId of the prompt this player holds; §10.6 sends it to nobody else.
fn open_choice(socket: &FakeSocket) -> String {
    let pending = last_view(socket)["pending"].clone();
    if pending.is_null() || pending["forYou"] != json!(true) {
        panic!("no prompt is open for this player");
    }
    pending["choiceId"].as_str().expect("a choice id").to_string()
}

fn hand_of(view: &Value) -> Vec<String> {
    view["you"]["hand"]
        .as_array()
        .map(|cards| {
            cards
                .iter()
                .filter_map(|card| card["instanceId"].as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

fn hand_ids(socket: &FakeSocket) -> Vec<String> {
    hand_of(&last_view(socket))
}

/// The instance of `def_id` in this player's hand. TS's `firstInHand` read slot 0, where the
/// scripted engine had dealt `fakeDeck`'s scripted card; the real deal is shuffled, so the card is
/// found by its definition.
fn in_hand(socket: &FakeSocket, def_id: &str) -> String {
    let view = last_view(socket);
    view["you"]["hand"]
        .as_array()
        .and_then(|cards| cards.iter().find(|card| card["defId"] == def_id))
        .and_then(|card| card["instanceId"].as_str())
        .unwrap_or_else(|| panic!("{def_id} is not in the hand"))
        .to_string()
}

async fn view_of(actor: &MatchActor, player: PlayerId) -> Value {
    to_json(&actor.view_for(player))
}

// ---------------------------------------------------------------------------------------------
// The harness
// ---------------------------------------------------------------------------------------------

struct Harness {
    app: Arc<App>,
    clock: Clock,
    /// The rows the opening's mulligans wrote before the test's first action (scripted cards only).
    opening: usize,
    /// Whatever `install_test_cards` hands back (the testkit override), held for the test.
    _engine: Box<dyn Any>,
}

#[derive(Default)]
struct StartOptions {
    /// Real decks (the real catalog, nothing installed): TS's `{ engine: enginePort(), decks }`.
    real: Option<(Vec<String>, Vec<String>)>,
}

fn hand_holds(view: &Value, def_id: &str) -> bool {
    view["you"]["hand"]
        .as_array()
        .is_some_and(|cards| cards.iter().any(|card| card["defId"] == def_id))
}

/// The first seed `<prefix>-<k>` whose opening deal puts each `(seat, card)` in that seat's hand.
fn seed_dealing(prefix: &str, decks: &(Vec<String>, Vec<String>), wanted: &[(PlayerId, &str)]) -> String {
    let (first, second) = decks;
    for k in 0..SEED_SEARCH {
        let seed = format!("{prefix}-{k}");
        let args: CreateGameArgs = from(json!({ "seed": seed, "decks": [first, second] }));
        let state = jackioh_engine::begin_game(&jackioh_engine::create_game(&args)).state;
        if wanted
            .iter()
            .all(|(seat, def_id)| hand_holds(&to_json(&jackioh_engine::view_for(&state, *seat)), def_id))
        {
            return seed;
        }
    }
    panic!("no seed under {prefix} deals {wanted:?}");
}

/// Both mulligans answered keeping the whole hand, in seat order; answers the rows they wrote.
async fn open_turn_one(actor: &MatchActor) -> usize {
    let owed = actor.snapshot().mulligan_owed;
    for seat in &owed {
        let keep = hand_of(&view_of(actor, *seat).await);
        let reply = to_json(
            &actor
                .submit(
                    *seat,
                    format!("keep-{seat}"),
                    from(json!({ "type": "mulligan", "keep": keep })),
                )
                .await,
        );
        assert_eq!(reply["type"], "ack", "the opening mulligan was refused: {reply}");
    }
    owed.len()
}

async fn start_match(options: StartOptions) -> Harness {
    let scripted = options.real.is_none();
    if scripted {
        install_test_cards();
    }
    let engine: Box<dyn Any> = Box::new(());
    let app = empty_test_app().await;
    {
        let data = fake(&app);
        let mut data = data.lock().await;
        data.seed_profile(json!({ "id": "profile-1", "rating": 1000 }));
        data.seed_profile(json!({ "id": "profile-2", "rating": 1000 }));
    }

    let decks = options.real.unwrap_or_else(|| {
        (
            fake_deck(&["test-prompt-self"]),
            fake_deck(&["test-prompt-enemy"]),
        )
    });
    let seed = if scripted {
        seed_dealing(
            "seed-recovery",
            &decks,
            &[
                (PlayerId::P1, "test-prompt-self"),
                (PlayerId::P2, "test-prompt-enemy"),
            ],
        )
    } else {
        "seed-recovery".to_string()
    };
    let started_at = tokio::time::Instant::now();
    app.matches
        .start(
            &app,
            from(json!({
                "matchId": MATCH_ID,
                "seed": seed,
                "catalogVersion": jackioh_cards::catalog_version(),
                "ranked": false,
                "seats": [
                    { "profileId": "profile-1", "player": "p1", "deck": decks.0 },
                    { "profileId": "profile-2", "player": "p2", "deck": decks.1 },
                ],
            })),
        )
        .await
        .expect("the registry starts the match");
    let row = match_row(&app).await;
    let clock = Clock {
        base_ms: row["createdAt"].as_i64().expect("the row's start"),
        at: started_at,
    };
    let opening = if scripted {
        let actor = app
            .matches
            .actor_for(&app, MATCH_ID)
            .await
            .expect("the match's actor");
        open_turn_one(&actor).await
    } else {
        0
    };
    Harness {
        app,
        clock,
        opening,
        _engine: engine,
    }
}

async fn match_row(app: &App) -> Value {
    table(app, |data| json!(data.tables.matches))
        .await
        .into_iter()
        .next()
        .expect("the match row")
}

async fn actor_of(h: &Harness) -> MatchActor {
    h.app
        .matches
        .actor_for(&h.app, MATCH_ID)
        .await
        .expect("the match's actor")
}

async fn results(h: &Harness) -> Vec<Value> {
    table(&h.app, |data| json!(data.tables.results)).await
}

mod m6_t4_crash_recovery {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn a_start_the_engine_refuses_writes_no_match_row_and_registers_no_actor() {
        // The opening draw runs before the row is written, so a game the engine cannot build (a
        // deck the catalog change stranded, say) fails the start cleanly. Written the other way
        // round the row went `live` first: a live match no socket could ever build an actor for —
        // `(seed, decks, log)` reconstructs nothing the engine refuses — and nothing but the ceiling
        // reaper could ever end it. TS made `beginGame` throw; here the engine refuses the decks.
        let (logs, _recording) = Logs::record();
        let app = empty_test_app().await;
        let refused = app
            .matches
            .start(
                &app,
                from(json!({
                    "matchId": MATCH_ID,
                    "seed": "seed-recovery",
                    "catalogVersion": jackioh_cards::catalog_version(),
                    "ranked": false,
                    "seats": [
                        { "profileId": "profile-1", "player": "p1", "deck": ["not-a-card"] },
                        { "profileId": "profile-2", "player": "p2", "deck": ["not-a-card"] },
                    ],
                })),
            )
            .await
            .expect_err("the engine refuses the decks");
        assert!(
            format!("{refused:?}").contains("deck must hold exactly"),
            "{refused:?}"
        );
        assert_eq!(
            table(&app, |data| json!(data.tables.matches)).await,
            Vec::<Value>::new()
        );
        assert_eq!(app.matches.live(), Vec::<String>::new());
        assert!(!logs.has("match.started"));
    }

    #[tokio::test(start_paused = true)]
    async fn killing_the_actor_mid_game_and_reconnecting_yields_the_same_view_for_for_both_players() {
        let (logs, _recording) = Logs::record();
        let h = start_match(StartOptions::default()).await;
        let actor = actor_of(&h).await;
        let p1 = create_fake_socket();
        let p2 = create_fake_socket();
        actor.attach(PlayerId::P1, p1.socket());
        actor.attach(PlayerId::P2, p2.socket());
        actor.idle().await;

        // Mid-game, and deliberately mid-*prompt*: p1 plays and answers its own prompt, ends the turn,
        // and p2 then opens a prompt p1 still owes an answer to (a trap firing on your turn, R79). A
        // rebuild has to bring back the open choice, not just the board (§10.6, e2e `05`).
        send(
            &actor,
            &p1,
            "n1",
            json!({ "type": "play", "instanceId": in_hand(&p1, "test-prompt-self") }),
        )
        .await;
        send(&actor, &p1, "n2", answer_of(&p1)).await;
        send(&actor, &p1, "n3", json!({ "type": "endTurn" })).await;
        send(
            &actor,
            &p2,
            "n4",
            json!({ "type": "play", "instanceId": in_hand(&p2, "test-prompt-enemy") }),
        )
        .await;

        let before_p1 = view_of(&actor, PlayerId::P1).await;
        let before_p2 = view_of(&actor, PlayerId::P2).await;
        assert_eq!(before_p1["pending"]["forYou"], json!(true));
        assert_eq!(before_p2["pending"]["forYou"], json!(false));
        assert_eq!(before_p2["pending"]["pendingFor"], "p1");
        // R79: the view carries a clock, and the paused turn clock and the prompt clock are both live.
        assert_eq!(before_p1["clockMs"], json!(PROMPT_CLOCK_SECONDS * 1000));
        assert_eq!(before_p2["clockMs"], json!(TURN_CLOCK_SECONDS * 1000));
        let before_snapshot = actor.snapshot();
        let before_hash = jackioh_engine::hash_state(&actor.engine_state());

        // The crash: the actor leaves memory, the log stays exactly where it was.
        h.app.matches.stop(MATCH_ID).await;
        assert_eq!(h.app.matches.live(), Vec::<String>::new());
        assert!(!p1.is_open());
        assert!(!p2.is_open());
        let log: Vec<Value> = table(&h.app, |data| json!(data.tables.match_actions))
            .await
            .into_iter()
            .filter(|row| row["matchId"] == MATCH_ID)
            .collect();
        let nonces: Vec<Value> = log
            .iter()
            .skip(h.opening)
            .map(|row| row["action"]["nonce"].clone())
            .collect();
        assert_eq!(nonces, vec![json!("n1"), json!("n2"), json!("n3"), json!("n4")]);
        assert_eq!(match_row(&h.app).await["status"], "live");

        // The rebuild: `(seed, decks, log)` and nothing else. No derived state was persisted.
        let revived = actor_of(&h).await;
        assert!(logs.has("match.rebuilt"));
        assert!(!logs.has("match.fold.errors"));
        assert_eq!(jackioh_engine::hash_state(&revived.engine_state()), before_hash);
        assert_eq!(revived.snapshot(), before_snapshot);

        // §9.5: "Reconnect gets a fresh full view, never a log replay" — one view frame each, and the
        // same one both players had before the kill, `clockMs` included.
        let back_p1 = create_fake_socket();
        let back_p2 = create_fake_socket();
        revived.attach(PlayerId::P1, back_p1.socket());
        revived.attach(PlayerId::P2, back_p2.socket());
        revived.idle().await;

        assert_eq!(views(&back_p1).len(), 1);
        assert_eq!(views(&back_p2).len(), 1);
        assert_eq!(last_view(&back_p1), before_p1);
        assert_eq!(last_view(&back_p2), before_p2);
        // The open prompt is the same prompt, still owed by the same player.
        assert_eq!(last_view(&back_p1)["pending"]["forYou"], json!(true));
        assert_eq!(last_view(&back_p2)["pending"]["pendingFor"], "p1");

        // §9.3: the nonce map is rebuilt from the log, so a client retrying an action it sent before
        // the crash gets its original ack and appends no second row.
        send(&revived, &back_p1, "n3", json!({ "type": "endTurn" })).await;
        assert_eq!(
            acks(&back_p1),
            vec![json!({ "type": "ack", "nonce": "n3", "seq": 3 + h.opening })]
        );
        assert_eq!(
            table(&h.app, |data| json!(data.tables.match_actions)).await.len(),
            4 + h.opening
        );

        // And the rebuilt actor carries the match on at the next gapless seq.
        send(&revived, &back_p1, "n5", answer_of(&back_p1)).await;
        assert_eq!(
            acks(&back_p1).last(),
            Some(&json!({ "type": "ack", "nonce": "n5", "seq": 5 + h.opening }))
        );
        assert_eq!(revived.snapshot().pending_for, None);
        assert_eq!(results(&h).await, Vec::<Value>::new());
    }

    #[tokio::test(start_paused = true)]
    async fn rebuilds_an_actor_for_a_socket_that_arrives_when_nothing_is_in_memory() {
        let h = start_match(StartOptions::default()).await;
        h.app.matches.stop(MATCH_ID).await;
        assert_eq!(h.app.matches.live(), Vec::<String>::new());

        // The upgrade path (`ws_server.rs`) only ever calls `attach`; folding is the registry's job.
        // TS: the attach resolves with the seat, "p2" (SURFACE §11.2 answers `()`; the view names it).
        let socket = create_fake_socket();
        h.app
            .matches
            .attach(&h.app, MATCH_ID, "profile-2", socket.socket())
            .await
            .expect("profile-2 attaches");
        let actor = actor_of(&h).await;
        actor.idle().await;
        assert_eq!(last_view(&socket)["viewer"], "p2");

        // §9.1: a profile that is not in this match learns only that there is no such match.
        let stranger = create_fake_socket();
        let refused = h
            .app
            .matches
            .attach(&h.app, MATCH_ID, "profile-99", stranger.socket())
            .await
            .expect_err("not a player");
        assert!(format!("{refused:?}").contains("no such match"), "{refused:?}");
        // Nothing but the opening was logged.
        assert_eq!(
            table(&h.app, |data| json!(data.tables.match_actions)).await.len(),
            h.opening
        );
    }

    #[tokio::test(start_paused = true)]
    async fn folds_the_log_once_when_both_sockets_arrive_together() {
        let (logs, _recording) = Logs::record();
        let h = start_match(StartOptions::default()).await;
        h.app.matches.stop(MATCH_ID).await;
        let rebuilt_before = logs.count("match.rebuilt");

        let (first, second) = tokio::join!(
            h.app.matches.actor_for(&h.app, MATCH_ID),
            h.app.matches.actor_for(&h.app, MATCH_ID)
        );
        // TS: the same actor object (`toBe`); here both calls answer, off one fold.
        first.expect("the first caller's actor");
        second.expect("the second caller's actor");
        assert_eq!(logs.count("match.rebuilt") - rebuilt_before, 1);
    }
}

/// The same acceptance item against the engine itself.
///
/// The block above runs on `support::engine`'s scripted cards, whose behaviour is written by this
/// test suite. What §9.5 and §9.3 actually promise is that the *real* engine is deterministic enough
/// for `(seed, decks, log)` to be the whole truth, and that is the property a crash depends on.
///
/// So this one deals two decks of real §8 card ids, walks the real opening through the socket
/// protocol, kills the actor and folds the log back. Only real behaviour can satisfy it: the shuffle
/// is the match rng replayed from `seed` (`setup.rs`), the opening draw is §2.1's table, and the
/// mulligan answers in the log have to land on the same instances they did the first time or the
/// two hands — and therefore the two views — come back different.
mod m6_t4_crash_recovery_with_the_real_engine {
    use super::*;

    /// Two legal, disjoint decks of real ids (TS `decksTheEngineAccepts`, which grew the slices until
    /// `createGame` stopped objecting because `apps/server` could not import `DECK_SIZE`; the engine's
    /// own constant is importable here, and the validator says why if the slices are refused).
    fn decks_the_engine_accepts(pool: &[String]) -> (Vec<String>, Vec<String>) {
        let size = usize::try_from(jackioh_engine::config::DECK_SIZE).expect("a deck size");
        let decks = (pool[..size].to_vec(), pool[size..size * 2].to_vec());
        let catalog = jackioh_engine::registered_catalog();
        for (label, deck) in [("p1", &decks.0), ("p2", &decks.1)] {
            if let Err(error) =
                jackioh_engine::validate_deck(deck, catalog, label, jackioh_engine::config::DECK_SIZE)
            {
                panic!("the real engine refused the deck built from the catalog: {error}");
            }
        }
        decks
    }

    struct RealMatch {
        h: Harness,
        actor: MatchActor,
        p1: FakeSocket,
        p2: FakeSocket,
        pool: Vec<String>,
    }

    async fn real_match() -> RealMatch {
        jackioh_cards::register_all();
        let pool: Vec<String> = jackioh_cards::CATALOG
            .iter()
            .filter(|(_, def)| {
                !def.token
                    && !to_json(&def.tags)
                        .as_array()
                        .is_some_and(|tags| tags.contains(&json!("Token")))
            })
            .map(|(id, _)| id.clone())
            .collect();
        let decks = decks_the_engine_accepts(&pool);
        let h = start_match(StartOptions { real: Some(decks) }).await;

        let actor = actor_of(&h).await;
        let p1 = create_fake_socket();
        let p2 = create_fake_socket();
        actor.attach(PlayerId::P1, p1.socket());
        actor.attach(PlayerId::P2, p2.socket());
        actor.idle().await;
        RealMatch {
            h,
            actor,
            p1,
            p2,
            pool,
        }
    }

    #[tokio::test(start_paused = true)]
    async fn r265_folding_seed_decks_log_through_the_real_engine_yields_the_same_view_for_whichever_seat_mulliganed_first()
     {
        let (logs, _recording) = Logs::record();
        for first in [PlayerId::P1, PlayerId::P2] {
            let RealMatch { h, actor, p1, p2, .. } = real_match().await;
            let (mine, theirs) = if first == PlayerId::P1 {
                (&p1, &p2)
            } else {
                (&p2, &p1)
            };
            let second = if first == PlayerId::P1 {
                PlayerId::P2
            } else {
                PlayerId::P1
            };

            // §2.1, R265: both mulligans open at once; `first` keeps everything and `second` keeps
            // nothing, so the fold has to replay R9's "draw the replacements, then shuffle the returned
            // cards back" in seat order whichever order the answers were logged in — the one step in setup
            // where the rng is consulted *after* an action in the log, and so the step a fold that merely
            // re-dealt, or resolved in log order, would get wrong.
            let snapshot = actor.snapshot();
            assert_eq!(snapshot.phase, Phase::Mulligan);
            assert_eq!(snapshot.mulligan_owed, vec![PlayerId::P1, PlayerId::P2]);
            let keep = hand_ids(mine);
            assert!(!keep.is_empty());
            assert!(!hand_ids(theirs).is_empty());
            send(
                &actor,
                mine,
                &format!("m-{first}"),
                json!({ "type": "mulligan", "keep": keep }),
            )
            .await;
            send(
                &actor,
                theirs,
                &format!("m-{second}"),
                json!({ "type": "mulligan", "keep": [] }),
            )
            .await;
            assert_eq!(actor.snapshot().phase, Phase::Main);

            let before_p1 = view_of(&actor, PlayerId::P1).await;
            let before_p2 = view_of(&actor, PlayerId::P2).await;
            let before_hash = jackioh_engine::hash_state(&actor.engine_state());
            h.app.matches.stop(MATCH_ID).await;
            let revived = actor_of(&h).await;
            assert!(!logs.has("match.fold.errors"));
            assert_eq!(jackioh_engine::hash_state(&revived.engine_state()), before_hash);
            assert_eq!(view_of(&revived, PlayerId::P1).await, before_p1);
            assert_eq!(view_of(&revived, PlayerId::P2).await, before_p2);
            h.app.matches.stop(MATCH_ID).await;
        }
    }

    #[tokio::test(start_paused = true)]
    async fn r266_a_crash_while_one_mulligan_answer_is_sealed_brings_back_the_same_sealed_window() {
        let (logs, _recording) = Logs::record();
        let RealMatch { h, actor, p2, .. } = real_match().await;

        // p2 answers first and p1 has not: the answer is sealed, in the log, and in no hand yet.
        let p2_hand = hand_ids(&p2);
        send(
            &actor,
            &p2,
            "sealed",
            json!({ "type": "mulligan", "keep": p2_hand[1..] }),
        )
        .await;
        assert_eq!(actor.snapshot().mulligan_owed, vec![PlayerId::P1]);
        let before_p1 = view_of(&actor, PlayerId::P1).await;
        let before_p2 = view_of(&actor, PlayerId::P2).await;
        assert_eq!(
            before_p2["mulligan"],
            json!({ "youReady": true, "opponentReady": false, "kept": p2_hand[1..] })
        );
        assert_eq!(
            before_p1["mulligan"],
            json!({ "youReady": false, "opponentReady": true })
        );
        let before_hash = jackioh_engine::hash_state(&actor.engine_state());

        h.app.matches.stop(MATCH_ID).await;
        let revived = actor_of(&h).await;
        assert_eq!(jackioh_engine::hash_state(&revived.engine_state()), before_hash);
        assert_eq!(revived.snapshot().mulligan_owed, vec![PlayerId::P1]);

        let back_p1 = create_fake_socket();
        let back_p2 = create_fake_socket();
        revived.attach(PlayerId::P1, back_p1.socket());
        revived.attach(PlayerId::P2, back_p2.socket());
        revived.idle().await;
        // A rebuilt clock arms a fresh mulligan window (the NOT IN SPEC note in `clock.rs`), so `clockMs`
        // is the one field that could differ — and no time passed across this crash, so it does not.
        assert_eq!(last_view(&back_p1), before_p1);
        assert_eq!(last_view(&back_p2), before_p2);

        // p1 answers on the rebuilt actor, and the sealed answer resolves with it at the next seq.
        send(
            &revived,
            &back_p1,
            "after-crash",
            json!({ "type": "mulligan", "keep": hand_ids(&back_p1) }),
        )
        .await;
        assert_eq!(
            acks(&back_p1).last(),
            Some(&json!({ "type": "ack", "nonce": "after-crash", "seq": 2 }))
        );
        let snapshot = revived.snapshot();
        assert_eq!(snapshot.phase, Phase::Main);
        assert_eq!(snapshot.mulligan_owed, Vec::<PlayerId>::new());
        // Only now does p2's sealed answer act: the one card it did not keep has left its hand.
        assert!(!hand_ids(&back_p2).contains(&p2_hand[0]));
        assert!(!logs.has("match.fold.errors"));
    }

    #[tokio::test(start_paused = true)]
    async fn folding_seed_decks_log_through_the_real_engine_yields_the_same_view_for_for_both_players() {
        let (logs, _recording) = Logs::record();
        let RealMatch {
            h,
            actor,
            p1,
            p2,
            pool,
        } = real_match().await;

        // §2.1, R265: both mulligans open at once. p1 keeps everything and p2 keeps nothing, so the
        // fold replays R9's replacement draws and shuffle-back (see the test above for either order).
        assert_eq!(actor.snapshot().phase, Phase::Mulligan);
        let p1_keep = hand_ids(&p1);
        assert!(!p1_keep.is_empty());
        send(&actor, &p1, "m1", json!({ "type": "mulligan", "keep": p1_keep })).await;
        assert!(!hand_ids(&p2).is_empty());
        send(&actor, &p2, "m2", json!({ "type": "mulligan", "keep": [] })).await;

        assert_eq!(actor.snapshot().phase, Phase::Main);
        send(&actor, &p1, "t1", json!({ "type": "endTurn" })).await;

        let before_p1 = view_of(&actor, PlayerId::P1).await;
        let before_p2 = view_of(&actor, PlayerId::P2).await;
        let before_snapshot = actor.snapshot();
        let before_hash = jackioh_engine::hash_state(&actor.engine_state());
        // PREMISE: the views really carry the real game — hands of real ids and a redacted count for
        // the other side — so the deep-equals below are comparing something. p1 holds what it kept plus
        // the card §2.1's first turn drew for it (R10).
        let p1_cards = before_p1["you"]["hand"]
            .as_array()
            .cloned()
            .unwrap_or_else(|| panic!("p1's own hand came back as a count"));
        assert_eq!(p1_cards.len(), p1_keep.len() + 1);
        assert_eq!(before_p2["opponent"]["hand"], json!({ "count": p1_cards.len() }));
        for card in &p1_cards {
            assert!(
                pool.iter().any(|id| card["defId"] == id.as_str()),
                "{card} is not a catalog card"
            );
        }
        // p2 mulliganed its whole hand away, so its cards are ones the fold had to redraw in R9's order.
        assert!(
            before_p2["you"]["hand"]
                .as_array()
                .is_some_and(|hand| !hand.is_empty())
        );

        // The crash: the actor leaves memory, the log stays.
        h.app.matches.stop(MATCH_ID).await;
        assert_eq!(h.app.matches.live(), Vec::<String>::new());
        let nonces: Vec<Value> = table(&h.app, |data| json!(data.tables.match_actions))
            .await
            .into_iter()
            .filter(|row| row["matchId"] == MATCH_ID)
            .map(|row| row["action"]["nonce"].clone())
            .collect();
        assert_eq!(nonces, vec![json!("m1"), json!("m2"), json!("t1")]);

        // The rebuild: `(seed, decks, log)` and nothing else.
        let revived = actor_of(&h).await;
        // The real engine accepted every action on the way back; an action it once took and now
        // refuses is the determinism break `registry.rebuild` shouts about.
        assert!(!logs.has("match.fold.errors"));
        assert_eq!(jackioh_engine::hash_state(&revived.engine_state()), before_hash);
        assert_eq!(revived.snapshot(), before_snapshot);

        let back_p1 = create_fake_socket();
        let back_p2 = create_fake_socket();
        revived.attach(PlayerId::P1, back_p1.socket());
        revived.attach(PlayerId::P2, back_p2.socket());
        revived.idle().await;

        // M6-T4, in full: the same `viewFor` for both players, built by the real `view_for` off a state
        // that was rebuilt from the log — hands, libraries, redacted event stream and all.
        assert_eq!(views(&back_p1).len(), 1);
        assert_eq!(views(&back_p2).len(), 1);
        assert_eq!(last_view(&back_p1), before_p1);
        assert_eq!(last_view(&back_p2), before_p2);
    }
}

/// R744: a seat that is not there when the first socket reaches its match's actor starts its
/// disconnect grace then (§2.5, §9.5). `registry.stop` is the restart: the log and the match row stay,
/// the actor and every clock in memory go, and the next socket rebuilds both (docs/architecture.md
/// §5.2). The scripted cards are enough: grace is the clock and the actor's, not a rule.
mod r744_disconnect_grace_for_a_seat_that_is_not_there {
    use super::*;

    fn grace_of() -> i64 {
        DISCONNECT_GRACE_SECONDS * 1000
    }

    async fn expired(h: &Harness) -> Vec<Value> {
        table(&h.app, |data| json!(data.tables.match_actions))
            .await
            .into_iter()
            .filter(|row| row["action"]["type"] == "disconnectExpired")
            .collect()
    }

    /// TS `lossOf(winner)`: the one result the writer was handed, a loss by disconnect.
    async fn assert_loss_of(h: &Harness, winner: &str) {
        let rows = results(h).await;
        assert_eq!(rows.len(), 1, "{rows:?}");
        assert_eq!(rows[0]["reason"], "disconnect");
        let profile = if winner == "p1" { "profile-1" } else { "profile-2" };
        assert_eq!(rows[0]["winnerProfileId"], profile);
    }

    async fn grace_deadline(h: &Harness) -> Value {
        match_row(&h.app).await["clocks"]["graceDeadline"].clone()
    }

    /// Both seats play a while, then the server restarts: the actor is gone and no socket is open.
    async fn joined_then_crashed() -> Harness {
        let h = start_match(StartOptions::default()).await;
        let actor = actor_of(&h).await;
        let p1 = create_fake_socket();
        let p2 = create_fake_socket();
        actor.attach(PlayerId::P1, p1.socket());
        actor.attach(PlayerId::P2, p2.socket());
        actor.idle().await;
        h.app.matches.stop(MATCH_ID).await;
        h
    }

    #[tokio::test(start_paused = true)]
    async fn r744_a_seat_that_never_comes_back_to_a_rebuilt_actor_loses_when_its_grace_runs_out() {
        let h = joined_then_crashed().await;
        let grace_ms = grace_of();

        // Nothing runs at boot: with no socket there is no actor, and so no grace.
        assert_eq!(h.app.matches.live(), Vec::<String>::new());
        assert_eq!(grace_deadline(&h).await, json!({ "p1": null, "p2": null }));

        // p2 comes back, p1 never does.
        let back = create_fake_socket();
        h.app
            .matches
            .attach(&h.app, MATCH_ID, "profile-2", back.socket())
            .await
            .expect("profile-2 attaches");
        let revived = actor_of(&h).await;
        revived.idle().await;
        assert_eq!(
            to_json(&revived.clocks())["graceDeadline"],
            json!({ "p1": h.clock.now() + grace_ms, "p2": null })
        );
        assert_eq!(grace_deadline(&h).await["p1"], json!(h.clock.now() + grace_ms));

        advance(grace_ms - 1).await;
        revived.idle().await;
        assert_eq!(results(&h).await, Vec::<Value>::new());
        assert_eq!(expired(&h).await, Vec::<Value>::new());

        advance(1).await;
        revived.idle().await;
        let actions = table(&h.app, |data| json!(data.tables.match_actions)).await;
        let last = &actions.last().expect("a logged action")["action"];
        assert_eq!(last["type"], "disconnectExpired");
        assert_eq!(last["player"], "p1");
        assert_eq!(last["playerId"], "p1");
        assert_loss_of(&h, "p2").await;
    }

    #[tokio::test(start_paused = true)]
    async fn r744_a_grace_deadline_stored_before_the_restart_fires_at_that_deadline_not_a_fresh_window_later()
    {
        let h = start_match(StartOptions::default()).await;
        let grace_ms = grace_of();
        let actor = actor_of(&h).await;
        let p1 = create_fake_socket();
        let p2 = create_fake_socket();
        actor.attach(PlayerId::P1, p1.socket());
        actor.attach(PlayerId::P2, p2.socket());
        actor.idle().await;

        // p1's socket drops, and the countdown is stored on the match (§9.5).
        let dropped_at = h.clock.now();
        p1.drop();
        actor.idle().await;
        let stored = grace_deadline(&h).await["p1"].clone();
        assert_eq!(stored, json!(dropped_at + grace_ms));

        // Half the window passes, then the server restarts and p2 is the first back.
        advance(grace_ms / 2).await;
        h.app.matches.stop(MATCH_ID).await;
        let revived = actor_of(&h).await;
        revived.idle().await;
        // The rebuilt actor's first write of its clocks must not erase the stored grace.
        assert_eq!(grace_deadline(&h).await["p1"], stored);

        let back = create_fake_socket();
        h.app
            .matches
            .attach(&h.app, MATCH_ID, "profile-2", back.socket())
            .await
            .expect("profile-2 attaches");
        revived.idle().await;
        assert_eq!(to_json(&revived.clocks())["graceDeadline"]["p1"], stored);

        advance(grace_ms / 2 - 1).await;
        revived.idle().await;
        assert_eq!(expired(&h).await, Vec::<Value>::new());
        advance(1).await;
        revived.idle().await;
        assert_eq!(expired(&h).await.len(), 1);
        assert_eq!(json!(h.clock.now()), stored);
    }

    #[tokio::test(start_paused = true)]
    async fn r744_a_seat_that_attaches_inside_the_window_cancels_its_grace_and_the_game_goes_on() {
        let h = joined_then_crashed().await;
        let grace_ms = grace_of();

        let back_p2 = create_fake_socket();
        h.app
            .matches
            .attach(&h.app, MATCH_ID, "profile-2", back_p2.socket())
            .await
            .expect("profile-2 attaches");
        let revived = actor_of(&h).await;
        revived.idle().await;
        assert!(!to_json(&revived.clocks())["graceDeadline"]["p1"].is_null());

        advance(grace_ms - 1).await;
        revived.idle().await;
        let back_p1 = create_fake_socket();
        h.app
            .matches
            .attach(&h.app, MATCH_ID, "profile-1", back_p1.socket())
            .await
            .expect("profile-1 attaches");
        revived.idle().await;
        assert_eq!(
            to_json(&revived.clocks())["graceDeadline"],
            json!({ "p1": null, "p2": null })
        );
        assert_eq!(grace_deadline(&h).await, json!({ "p1": null, "p2": null }));

        advance(1).await;
        revived.idle().await;
        assert_eq!(expired(&h).await, Vec::<Value>::new());
        assert_eq!(results(&h).await, Vec::<Value>::new());
        assert!(revived.snapshot().result.is_none());
    }

    #[tokio::test(start_paused = true)]
    async fn r744_a_fresh_matchs_seat_that_never_opens_its_socket_loses_after_the_grace() {
        let h = start_match(StartOptions::default()).await;
        let grace_ms = grace_of();

        let socket = create_fake_socket();
        h.app
            .matches
            .attach(&h.app, MATCH_ID, "profile-1", socket.socket())
            .await
            .expect("profile-1 attaches");
        let actor = actor_of(&h).await;
        actor.idle().await;
        assert_eq!(
            to_json(&actor.clocks())["graceDeadline"],
            json!({ "p1": null, "p2": h.clock.now() + grace_ms })
        );

        advance(grace_ms).await;
        actor.idle().await;
        let actions = table(&h.app, |data| json!(data.tables.match_actions)).await;
        let last = &actions.last().expect("a logged action")["action"];
        assert_eq!(last["type"], "disconnectExpired");
        assert_eq!(last["player"], "p2");
        assert_eq!(last["playerId"], "p2");
        assert_loss_of(&h, "p1").await;
    }

    #[tokio::test(start_paused = true)]
    async fn r744_two_seats_that_attach_seconds_apart_as_at_a_normal_start_fire_no_grace() {
        let h = start_match(StartOptions::default()).await;
        let grace_ms = grace_of();

        let first = create_fake_socket();
        h.app
            .matches
            .attach(&h.app, MATCH_ID, "profile-1", first.socket())
            .await
            .expect("profile-1 attaches");
        let actor = actor_of(&h).await;
        actor.idle().await;
        // The second client is still loading, and its seat is already counted away.
        assert!(!to_json(&actor.clocks())["graceDeadline"]["p2"].is_null());

        advance(grace_ms / 10).await;
        let second = create_fake_socket();
        h.app
            .matches
            .attach(&h.app, MATCH_ID, "profile-2", second.socket())
            .await
            .expect("profile-2 attaches");
        actor.idle().await;
        assert_eq!(
            to_json(&actor.clocks())["graceDeadline"],
            json!({ "p1": null, "p2": null })
        );

        advance(grace_ms).await;
        actor.idle().await;
        assert_eq!(expired(&h).await, Vec::<Value>::new());
        assert_eq!(results(&h).await, Vec::<Value>::new());
    }

    #[tokio::test(start_paused = true)]
    async fn r744_a_match_no_socket_returns_to_after_a_restart_is_still_drawn_by_the_reaper_r112() {
        let h = start_match(StartOptions::default()).await;
        for id in ["profile-1", "profile-2"] {
            store!(h.app, t => t.profiles_set_in_match(id, Some(MATCH_ID)).await.expect("profiles.setInMatch"));
        }
        let actor = actor_of(&h).await;
        let p1 = create_fake_socket();
        let p2 = create_fake_socket();
        actor.attach(PlayerId::P1, p1.socket());
        actor.attach(PlayerId::P2, p2.socket());
        actor.idle().await;
        p1.drop();
        actor.idle().await;
        h.app.matches.stop(MATCH_ID).await;

        // Nobody comes back: no actor is rebuilt, so no grace runs and nobody loses by it.
        advance(MATCH_CEILING_MINUTES * 60_000).await;
        assert_eq!(
            reap_stuck_matches(&h.app).await.expect("the reaper runs"),
            vec![MATCH_ID.to_string()]
        );
        let rows = results(&h).await;
        assert_eq!(rows[0]["matchId"], MATCH_ID);
        assert_eq!(rows[0]["reason"], "match-ceiling");
        assert_eq!(rows[0]["winnerProfileId"], Value::Null);
        assert_eq!(rows[0]["turns"], json!(0));
        assert_eq!(expired(&h).await, Vec::<Value>::new());
    }
}
