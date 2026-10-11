//! Play telemetry through the actor (SPEC §9.11, R1442): a real match over the fake store, on tokio's
//! paused clock, so every think time is exact. It writes its think times, emotes and end-of-game
//! signals once the result has landed; a failed write costs nothing; and `timing-backfill` folds the
//! log the match left into the same action timings, less the clock and the rank the log does not
//! hold. The store's half is `tests/store/contract.rs`'s `r1442_play_telemetry`.
//! Surface contract: docs/v0.3.0/SURFACE.md §11.2, §11.3.

use std::sync::Arc;
use std::time::Duration;

use serde_json::{Value, json};

use jackioh_ai::{AI_EVAL, NextSwing, evaluate};
use jackioh_engine::wire::PlayerId;
use jackioh_engine::{Action, GameEventType, deal_emote_hand};
use jackioh_server::actor::match_actor::MatchActor;
use jackioh_server::app::App;
use jackioh_server::cli::timing_backfill::{BackfillOutcome, backfill};
use jackioh_server::config::{MULLIGAN_CLOCK_MS, TURN_CLOCK_MS};
use jackioh_server::db::fake::FakeData;
use jackioh_server::db::store::{Db, MatchStatus, PlayTelemetry, StoreError};

use crate::support::deps::test_app;
use crate::support::socket::{FakeSocket, create_fake_socket, settle};

const P1: &str = "profile-1";
const P2: &str = "profile-2";
const SEED: &str = "telemetry-real";

/// One store call in its own transaction.
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

/// The fake store behind the test app.
fn fake(app: &App) -> Arc<tokio::sync::Mutex<FakeData>> {
    match &app.db {
        Db::Fake(data) => Arc::clone(data),
        _ => panic!("the test app runs on the fake store"),
    }
}

/// Every store method named `method` fails until the test ends.
async fn fail_on(app: &App, method: &'static str) {
    fake(app).lock().await.on_call = Some(Arc::new(move |called: &str| {
        if called == method {
            Err(StoreError::Other("injected".to_string()))
        } else {
            Ok(())
        }
    }));
}

/// The test app with both profiles seeded.
async fn world() -> Arc<App> {
    let app = test_app().await;
    let data = fake(&app);
    let mut data = data.lock().await;
    data.seed_profile(json!({ "id": P1, "rating": 1000 }));
    data.seed_profile(json!({ "id": P2, "rating": 1000 }));
    drop(data);
    app
}

/// Two decks of real Core cards.
fn decks() -> (Vec<String>, Vec<String>) {
    jackioh_cards::register_all();
    let core: Vec<String> = jackioh_cards::CATALOG
        .iter()
        .filter(|(_, def)| !def.token && to_json(&def.set) == "Core")
        .map(|(id, _)| id.clone())
        .collect();
    (core[..20].to_vec(), core[20..40].to_vec())
}

/// Starts the match on the real registry and attaches both accounts' sockets at once (R744: an
/// account never attached would be away from the first attach on).
async fn start(app: &Arc<App>, match_id: &str) -> (MatchActor, FakeSocket, FakeSocket) {
    let decks = decks();
    app.matches
        .start(
            app,
            from(json!({
                "matchId": match_id,
                "seed": SEED,
                "catalogVersion": jackioh_cards::catalog_version(),
                "ranked": false,
                "seats": [
                    { "profileId": P1, "player": "p1", "deck": decks.0 },
                    { "profileId": P2, "player": "p2", "deck": decks.1 },
                ],
            })),
        )
        .await
        .expect("the registry starts the match");
    let (one, two) = (create_fake_socket(), create_fake_socket());
    app.matches
        .attach(app, match_id, P1, one.socket())
        .await
        .expect("p1 attaches");
    app.matches
        .attach(app, match_id, P2, two.socket())
        .await
        .expect("p2 attaches");
    let actor = app
        .matches
        .actor_for(app, match_id)
        .await
        .expect("the match's actor");
    settle().await;
    (actor, one, two)
}

async fn sleep_ms(ms: u64) {
    tokio::time::sleep(Duration::from_millis(ms)).await;
}

/// What the test saw of each move as it made it.
struct Move {
    seat: PlayerId,
    legal: usize,
}

/// One action as `player`, under the next `tel-<n>` nonce; a refusal fails the test. Answers the
/// move as the test saw it, and the state it was made on.
struct Submitter {
    actor: MatchActor,
    n: u32,
}

impl Submitter {
    async fn submit(&mut self, player: PlayerId, body: Value) -> (Move, jackioh_engine::GameState) {
        self.n += 1;
        let before = self.actor.engine_state();
        let legal = jackioh_engine::legal_actions(&before, player).len();
        let reply = to_json(
            &self
                .actor
                .submit(player, format!("tel-{}", self.n), from(body.clone()))
                .await,
        );
        if reply["type"] == "error" {
            panic!("{} by {player}: {reply}", body["type"]);
        }
        (Move { seat: player, legal }, before)
    }
}

fn keep_all(actor: &MatchActor, player: PlayerId) -> Value {
    let view = to_json(&actor.view_for(player));
    let keep: Vec<Value> = view["you"]["hand"]
        .as_array()
        .expect("a seat's own hand is a list")
        .iter()
        .map(|card| card["instanceId"].clone())
        .collect();
    json!({ "type": "mulligan", "keep": keep })
}

/// The match as both accounts play it: p1 mulligans after 1 s, p2 after 3 s, the seat that then
/// owes a move ends its turn after half a second, the other emotes and concedes.
struct Played {
    moves: Vec<Move>,
    /// The seat that ended its turn, and the one that conceded.
    ender: PlayerId,
    conceder: PlayerId,
    /// The last event the turn's end produced, and the state the concede was made on.
    end_event: Option<GameEventType>,
    concede_state: jackioh_engine::GameState,
}

async fn play(app: &Arc<App>, match_id: &str) -> Played {
    let (actor, one, two) = start(app, match_id).await;
    let mut walk = Submitter {
        actor: actor.clone(),
        n: 0,
    };
    let mut moves = Vec::new();

    sleep_ms(1_000).await;
    moves.push(walk.submit(PlayerId::P1, keep_all(&actor, PlayerId::P1)).await.0);
    sleep_ms(2_000).await;
    moves.push(walk.submit(PlayerId::P2, keep_all(&actor, PlayerId::P2)).await.0);

    sleep_ms(500).await;
    let snapshot = actor.snapshot();
    assert!(
        snapshot.pending_for.is_none(),
        "no prompt opens the first turn under this seed"
    );
    let ender = snapshot.active;
    let end_turn: Action = from(json!({ "type": "endTurn", "playerId": ender, "nonce": "probe" }));
    let end_event = jackioh_engine::reduce(&actor.engine_state(), &end_turn)
        .events
        .last()
        .map(|event| event.event_type());
    let (ended, _) = walk.submit(ender, json!({ "type": "endTurn" })).await;
    moves.push(ended);
    let conceder = ender.opponent();
    assert_eq!(actor.snapshot().active, conceder);

    // 200 ms on, the seat that ended its turn emotes; 500 ms after that its opponent answers.
    let socket = |home: PlayerId| if home == PlayerId::P1 { &one } else { &two };
    sleep_ms(200).await;
    socket(ender).receive_json(json!({ "type": "emote", "emote": deal_emote_hand(SEED, ender)[0] }));
    settle().await;
    sleep_ms(500).await;
    socket(conceder).receive_json(json!({ "type": "emote", "emote": deal_emote_hand(SEED, conceder)[0] }));
    settle().await;

    sleep_ms(300).await;
    let (conceded, concede_state) = walk.submit(conceder, json!({ "type": "concede" })).await;
    moves.push(conceded);
    actor.idle().await;
    Played {
        moves,
        ender,
        conceder,
        end_event,
        concede_state,
    }
}

async fn telemetry_of(app: &App, match_id: &str) -> PlayTelemetry {
    store!(app, t => t.play_telemetry_of(match_id).await.expect("playTelemetry.of"))
}

mod r1442_play_telemetry_through_the_actor {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn r1442_a_finished_match_writes_think_times_emotes_and_signals_after_its_result() {
        let app = world().await;
        let played = play(&app, "m-tel-1").await;
        let written = to_json(&telemetry_of(&app, "m-tel-1").await);

        // p1 owed its mulligan from the match's creation, p2 too, the seat whose turn began from the
        // push of p2's mulligan, and the conceder from the push of the turn's end.
        let think: Vec<i64> = vec![1_000, 3_000, 500, 1_000];
        let clock_left: Vec<i64> = vec![
            MULLIGAN_CLOCK_MS - 1_000,
            MULLIGAN_CLOCK_MS - 3_000,
            TURN_CLOCK_MS - 500,
            TURN_CLOCK_MS - 1_000,
        ];
        let kinds = ["mulligan", "mulligan", "endTurn", "concede"];
        let timings = written["actionTimings"].as_array().expect("the timings");
        assert_eq!(timings.len(), 4, "{written}");
        for (n, row) in timings.iter().enumerate() {
            let seen = &played.moves[n];
            assert_eq!(row["matchId"], "m-tel-1");
            assert_eq!(row["seq"], json!(n + 1));
            assert_eq!(row["seat"], json!(seen.seat), "row {n}");
            assert_eq!(row["actionKind"], kinds[n], "row {n}");
            assert_eq!(row["legalCount"], json!(seen.legal), "row {n}");
            assert_eq!(row["thinkMs"], json!(think[n]), "row {n}");
            assert_eq!(row["clockLeftMs"], json!(clock_left[n]), "row {n}");
            assert_eq!(row["firstInTurn"], true, "row {n}");
            assert_eq!(row["rankBucket"], "raisin", "row {n}");
            assert_eq!(row["pilot"], "human", "row {n}");
        }
        assert_eq!(timings[0]["turn"], timings[1]["turn"]);
        assert_eq!(
            timings[3]["turn"],
            json!(timings[2]["turn"].as_i64().map(|turn| turn + 1))
        );

        let turn = timings[3]["turn"].clone();
        assert_eq!(
            written["emoteEvents"],
            json!([
                {
                    "matchId": "m-tel-1", "seat": played.ender, "ordinal": 0,
                    "emoteId": deal_emote_hand(SEED, played.ender)[0], "turn": turn,
                    "triggerEvent": played.end_event, "msSinceTrigger": 200, "pilot": "human",
                },
                {
                    "matchId": "m-tel-1", "seat": played.conceder, "ordinal": 1,
                    "emoteId": deal_emote_hand(SEED, played.conceder)[0], "turn": turn,
                    "triggerEvent": played.end_event, "msSinceTrigger": 700,
                    "replyToOpponentMs": 500, "pilot": "human",
                },
            ])
        );

        let deficit = -evaluate(&played.concede_state, played.conceder, NextSwing::Seat, &AI_EVAL);
        let signals = written["matchSignals"].as_array().expect("the signals");
        assert_eq!(signals.len(), 2, "{written}");
        for row in signals {
            let conceded = row["seat"] == json!(played.conceder);
            assert_eq!(row["matchId"], "m-tel-1");
            assert_eq!(row["drawOffers"], 0);
            assert_eq!(row["drawAccepted"], false);
            assert_eq!(row["rematchOffered"], false);
            assert_eq!(row["rematchAccepted"], false);
            assert_eq!(row["timeouts"], 0);
            assert_eq!(row["pilot"], "human");
            if conceded {
                assert_eq!(row["concededTurn"], turn);
                assert_eq!(row["concedeEvalDeficit"], json!(deficit));
            } else {
                assert!(row.get("concededTurn").is_none(), "{row}");
                assert!(row.get("concedeEvalDeficit").is_none(), "{row}");
            }
        }
        assert_eq!(signals[0]["seat"], "p1");

        // Written after the result, which stands beside it.
        assert!(
            store!(app, t => t.results_get_by_match("m-tel-1").await.expect("results.getByMatch")).is_some()
        );
    }

    #[tokio::test(start_paused = true)]
    async fn r1442_a_failed_telemetry_write_never_costs_the_result() {
        let app = world().await;
        fail_on(&app, "playTelemetry.insert").await;
        play(&app, "m-tel-2").await;

        assert!(
            store!(app, t => t.results_get_by_match("m-tel-2").await.expect("results.getByMatch")).is_some()
        );
        let row =
            store!(app, t => t.matches_get("m-tel-2").await.expect("matches.get")).expect("the match row");
        assert_eq!(row.status, MatchStatus::Finished);
        assert_eq!(telemetry_of(&app, "m-tel-2").await, PlayTelemetry::default());
        for profile in [P1, P2] {
            let held = store!(app, t => t.profiles_get_by_id(profile).await.expect("profiles.getById"))
                .expect("the profile");
            assert_eq!(held.in_match_id, None, "{profile} is let go");
        }
    }

    #[tokio::test(start_paused = true)]
    async fn r1442_a_rebuilt_actor_carries_on_from_the_telemetry_its_log_holds() {
        let app = world().await;
        let (actor, _one, _two) = start(&app, "m-tel-4").await;
        let mut walk = Submitter {
            actor: actor.clone(),
            n: 0,
        };
        sleep_ms(1_000).await;
        walk.submit(PlayerId::P1, keep_all(&actor, PlayerId::P1)).await;
        sleep_ms(2_000).await;
        walk.submit(PlayerId::P2, keep_all(&actor, PlayerId::P2)).await;
        actor.idle().await;

        // The server restarts; the next socket rebuilds the actor from the log.
        app.matches.stop("m-tel-4").await;
        let revived = app
            .matches
            .actor_for(&app, "m-tel-4")
            .await
            .expect("the rebuilt actor");
        let (one, two) = (create_fake_socket(), create_fake_socket());
        for (profile, socket) in [(P1, &one), (P2, &two)] {
            app.matches
                .attach(&app, "m-tel-4", profile, socket.socket())
                .await
                .expect("the account attaches again");
        }
        settle().await;
        let mut walk = Submitter {
            actor: revived.clone(),
            n: walk.n,
        };
        sleep_ms(500).await;
        let ender = revived.snapshot().active;
        walk.submit(ender, json!({ "type": "endTurn" })).await;
        sleep_ms(1_000).await;
        walk.submit(ender.opponent(), json!({ "type": "concede" })).await;
        revived.idle().await;

        // The moves before the restart come from the log, with no clock; the turn's end counts from
        // the last logged move, and the concede from the push of the turn's end.
        let timings = to_json(&telemetry_of(&app, "m-tel-4").await.action_timings);
        let seen: Vec<Value> = timings
            .as_array()
            .expect("the timings")
            .iter()
            .map(|row| {
                json!([
                    row["seq"],
                    row["actionKind"],
                    row["thinkMs"],
                    row.get("clockLeftMs").is_some(),
                    row["rankBucket"]
                ])
            })
            .collect();
        assert_eq!(
            seen,
            vec![
                json!([1, "mulligan", 1_000, false, "raisin"]),
                json!([2, "mulligan", 3_000, false, "raisin"]),
                json!([3, "endTurn", 500, true, "raisin"]),
                json!([4, "concede", 1_000, true, "raisin"]),
            ]
        );
    }

    #[tokio::test(start_paused = true)]
    async fn r1442_the_backfill_folds_the_log_into_the_rows_the_live_path_wrote() {
        let app = world().await;
        play(&app, "m-tel-3").await;
        let live = telemetry_of(&app, "m-tel-3").await;
        assert_eq!(live.action_timings.len(), 4);
        fake(&app).lock().await.tables.action_timings.clear();

        let outcome = backfill(&app.db).await.expect("the backfill runs");
        assert_eq!(
            outcome,
            BackfillOutcome {
                matches: 1,
                rows: 4,
                skipped: 0
            }
        );
        let folded = telemetry_of(&app, "m-tel-3").await;
        let expected: Vec<Value> = live
            .action_timings
            .iter()
            .map(|row| {
                let mut row = to_json(row);
                let fields = row.as_object_mut().expect("a row is an object");
                fields.remove("clockLeftMs");
                fields.remove("rankBucket");
                row
            })
            .collect();
        assert_eq!(to_json(&folded.action_timings), json!(expected));
        // Nothing but the timings is folded, and what the live path wrote of the rest is untouched.
        assert_eq!(folded.emote_events, live.emote_events);
        assert_eq!(folded.match_signals, live.match_signals);

        // A match with its timings is not folded again.
        assert_eq!(
            backfill(&app.db).await.expect("the backfill runs again"),
            BackfillOutcome::default()
        );
    }

    #[tokio::test(start_paused = true)]
    async fn r1442_the_backfill_never_puts_back_the_moves_of_a_deleted_account() {
        let app = world().await;
        play(&app, "m-tel-5").await;
        let live = telemetry_of(&app, "m-tel-5").await;
        fake(&app).lock().await.tables.action_timings.clear();
        assert!(store!(app, t => t.profiles_remove(P1).await.expect("profiles.remove")));

        let outcome = backfill(&app.db).await.expect("the backfill runs");
        let folded = telemetry_of(&app, "m-tel-5").await.action_timings;
        let theirs: Vec<Value> = live
            .action_timings
            .iter()
            .filter(|row| row.seat == PlayerId::P2)
            .map(|row| {
                let mut row = to_json(row);
                let fields = row.as_object_mut().expect("a row is an object");
                fields.remove("clockLeftMs");
                fields.remove("rankBucket");
                row
            })
            .collect();
        assert_eq!(theirs.len(), 2, "p2's mulligan and one move of the turn");
        assert_eq!(to_json(&folded), json!(theirs));
        assert_eq!(
            outcome,
            BackfillOutcome {
                matches: 1,
                rows: 2,
                skipped: 0
            }
        );
    }
}
