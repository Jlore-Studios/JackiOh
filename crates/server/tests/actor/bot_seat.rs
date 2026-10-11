//! R1444 (SPEC §9.12): the bot seat. A ladder bot's seat has no socket: the match actor plays it
//! itself, deciding off the queue and moving at a human pace inside its clock, with server nonces
//! (`srv-bot-…`, R270). It gets no disconnect grace (R744), never concedes or takes a draw (R188),
//! emotes through its persona (R645), is rated as R610 rates a bot, and its match replays from
//! `(seed, log)` and survives a restart like any other.
//!
//! These run the real engine and the real cards (`jackioh_cards::register_all`), not the testkit's
//! scripted ones: the bot decides on the blocking pool, whose threads do not see a test's
//! thread-local card override. Time is tokio's paused clock, which does not move on its own while a
//! decision is running there, so the think times cost no real time and the decisions only theirs.

use std::sync::Arc;
use std::time::Duration;

use serde_json::{Value, json};

use jackioh_engine::wire::{EmoteId, Phase, PlayerId};
use jackioh_engine::{ActionBody, deal_emote_hand};
use jackioh_server::actor::engine::{fold, fold_args, hash_state};
use jackioh_server::actor::match_actor::MatchActor;
use jackioh_server::actor::protocol::SERVER_NONCE_PREFIX;
use jackioh_server::app::{App, now_ms};
use jackioh_server::config::{AI_PERSONAS, EmoteTrigger, PersonaName};
use jackioh_server::db::fake::FakeData;
use jackioh_server::db::store::{Db, NewBot};

use crate::support::deps::{TestAppOptions, test_app_with};
use crate::support::socket::{FakeSocket, create_fake_socket};

const MATCH_ID: &str = "match-bot-seat";
const PERSON: &str = "profile-person";
const BOT: &str = "profile-bot";

/// How many quarter-second steps `wait_until` takes before it gives up: far longer than any turn.
const WAIT_STEPS: usize = 4_000;
const WAIT_STEP_MS: u64 = 250;

fn from<T: serde::de::DeserializeOwned>(value: Value) -> T {
    serde_json::from_value(value.clone()).unwrap_or_else(|error| panic!("{error}: {value}"))
}

fn to_json<T: serde::Serialize>(value: &T) -> Value {
    serde_json::to_value(value).expect("a serialisable value")
}

/// One store call in its own transaction.
macro_rules! q {
    ($app:expr, $method:ident($($arg:expr),* $(,)?)) => {{
        let mut tx = $app.db.begin(None).await.expect("begin");
        let out = tx.$method($($arg),*).await.expect(stringify!($method));
        tx.commit().await.expect("commit");
        out
    }};
}

fn fake(app: &App) -> Arc<tokio::sync::Mutex<FakeData>> {
    match &app.db {
        Db::Fake(data) => Arc::clone(data),
        Db::Pg(_) => panic!("the test app runs on the fake store"),
    }
}

/// The catalog's playable ids (no tokens), sliced into two disjoint decks.
fn decks() -> (Vec<String>, Vec<String>) {
    jackioh_cards::register_all();
    let pool: Vec<String> = jackioh_cards::CATALOG
        .values()
        .filter(|def| !def.token)
        .map(|def| def.id.clone())
        .collect();
    let size = usize::try_from(jackioh_engine::config::DECK_SIZE).expect("a deck size");
    (pool[..size].to_vec(), pool[size..size * 2].to_vec())
}

struct BotMatch {
    app: Arc<App>,
    actor: MatchActor,
    socket: FakeSocket,
    /// The seat the person began in; the bot holds the other.
    person: PlayerId,
    seed: String,
}

impl BotMatch {
    fn bot(&self) -> PlayerId {
        self.person.opponent()
    }
}

/// A match between a person in `person` and a ladder bot of `persona` in the other seat, started as
/// the queue starts one, with the person's socket attached and the bot's never.
async fn bot_match(seed: &str, person: PlayerId, persona: PersonaName, ranked: bool) -> BotMatch {
    let app = test_app_with(TestAppOptions {
        skip_fixtures: true,
        ..TestAppOptions::default()
    })
    .await;
    fake(&app)
        .lock()
        .await
        .seed_profile(json!({ "id": PERSON, "rating": 1000 }));
    let bot: NewBot = from(json!({
        "profileId": BOT,
        "strength": 0,
        "speed": 1.0,
        "persona": persona.as_str(),
        "activeFromHour": 0,
        "activeHours": 24,
        "decks": [],
        "decksCatalogVersion": null,
        "createdAt": now_ms() - 86_400_000,
        "seedRating": 1000,
    }));
    q!(app, bots_create(&bot));

    let (person_deck, bot_deck) = decks();
    let seat = |profile: &str, player: PlayerId, deck: &[String]| json!({ "profileId": profile, "player": player.as_str(), "deck": deck });
    let seats = if person == PlayerId::P1 {
        [
            seat(PERSON, PlayerId::P1, &person_deck),
            seat(BOT, PlayerId::P2, &bot_deck),
        ]
    } else {
        [
            seat(BOT, PlayerId::P1, &bot_deck),
            seat(PERSON, PlayerId::P2, &person_deck),
        ]
    };
    app.matches
        .start(
            &app,
            from(json!({
                "matchId": MATCH_ID,
                "seed": seed,
                "catalogVersion": jackioh_cards::catalog_version(),
                "ranked": ranked,
                "seats": seats,
            })),
        )
        .await
        .expect("the registry starts the match");
    let actor = app
        .matches
        .actor_for(&app, MATCH_ID)
        .await
        .expect("the match's actor");
    let socket = create_fake_socket();
    actor.attach(person, socket.socket());
    actor.idle().await;
    BotMatch {
        app,
        actor,
        socket,
        person,
        seed: seed.to_string(),
    }
}

/// Lets time pass, a quarter second at a time, until `done` holds.
async fn wait_until(actor: &MatchActor, what: &str, done: impl Fn(&MatchActor) -> bool) {
    for _ in 0..WAIT_STEPS {
        actor.idle().await;
        if done(actor) {
            return;
        }
        tokio::time::sleep(Duration::from_millis(WAIT_STEP_MS)).await;
    }
    panic!("{what} never happened");
}

/// The match's action log as `(seat, nonce, type)`, in order.
async fn log_of(app: &App) -> Vec<(String, String, String)> {
    q!(app, matches_actions(MATCH_ID))
        .into_iter()
        .map(|row| {
            let action = to_json(&row.action);
            (
                action["playerId"].as_str().unwrap_or_default().to_string(),
                action["nonce"].as_str().unwrap_or_default().to_string(),
                action["type"].as_str().unwrap_or_default().to_string(),
            )
        })
        .collect()
}

fn bot_nonce(nonce: &str) -> bool {
    nonce.starts_with(&format!("{SERVER_NONCE_PREFIX}bot-"))
}

/// The person keeps their whole opening hand.
async fn person_mulligans(m: &BotMatch) {
    let view = to_json(&m.actor.view_for(m.person));
    let keep: Vec<Value> = view["you"]["hand"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .map(|card| card["instanceId"].clone())
        .collect();
    let reply = m
        .actor
        .submit(
            m.person,
            "person-keep".to_string(),
            from(json!({ "type": "mulligan", "keep": keep })),
        )
        .await;
    assert_eq!(
        to_json(&reply)["type"],
        "ack",
        "the person's mulligan was refused"
    );
}

async fn submit(m: &BotMatch, nonce: &str, body: Value) {
    let body: ActionBody = from(body);
    let reply = m.actor.submit(m.person, nonce.to_string(), body).await;
    assert_eq!(
        to_json(&reply)["type"],
        "ack",
        "{nonce} was refused: {}",
        to_json(&reply)
    );
}

/// Both mulligans in, and the person to act.
async fn to_the_persons_turn(m: &BotMatch) {
    person_mulligans(m).await;
    let person = m.person;
    wait_until(&m.actor, "the person's first turn", |actor| {
        let snapshot = actor.snapshot();
        snapshot.phase == Phase::Main && snapshot.active == person && snapshot.pending_for.is_none()
    })
    .await;
}

async fn concede(m: &BotMatch) {
    submit(m, "person-concede", json!({ "type": "concede" })).await;
    for _ in 0..10 {
        m.actor.idle().await;
        tokio::task::yield_now().await;
    }
}

#[tokio::test(start_paused = true)]
async fn r1444_bot_mulligans_and_plays_without_a_socket() {
    let m = bot_match("seed-bot-plays", PlayerId::P1, PersonaName::Silent, false).await;
    to_the_persons_turn(&m).await;
    // The person ends their turn; the bot plays its turn and it comes back.
    let turn = m.actor.snapshot().turn;
    submit(&m, "person-end-1", json!({ "type": "endTurn" })).await;
    let person = m.person;
    wait_until(&m.actor, "the bot's turn to end", |actor| {
        let snapshot = actor.snapshot();
        snapshot.active == person && snapshot.turn > turn + 1
    })
    .await;

    let log = log_of(&m.app).await;
    let bot = m.bot().as_str().to_string();
    let bots: Vec<&(String, String, String)> = log.iter().filter(|(seat, _, _)| *seat == bot).collect();
    assert!(
        bots.iter().any(|(_, _, kind)| kind == "mulligan"),
        "the bot kept a hand: {log:?}"
    );
    // It played its turn (ending it itself, or played out until the engine ended it, R82).
    assert!(
        bots.iter().any(|(_, _, kind)| kind != "mulligan"),
        "the bot moved in its turn: {log:?}"
    );
    // Every bot move is a server action, and only the bot's are.
    for (seat, nonce, _) in &log {
        assert_eq!(bot_nonce(nonce), *seat == bot, "{seat} {nonce}");
    }
    assert!(m.actor.snapshot().result.is_none());
}

#[tokio::test(start_paused = true)]
async fn r1444_bot_seat_never_gets_a_grace() {
    let m = bot_match("seed-bot-grace", PlayerId::P2, PersonaName::Silent, false).await;
    // The person attached; the bot never will. No grace runs for either seat.
    for _ in 0..8 {
        tokio::time::sleep(Duration::from_millis(WAIT_STEP_MS)).await;
        m.actor.idle().await;
    }
    let row = q!(m.app, matches_get(MATCH_ID)).expect("the match row");
    assert_eq!(
        to_json(&row.clocks)["graceDeadline"],
        json!({ "p1": null, "p2": null }),
        "a bot's seat is never away"
    );
    assert!(m.actor.snapshot().result.is_none());
}

#[tokio::test(start_paused = true)]
async fn r1444_r610_bot_match_rated_by_r610() {
    let m = bot_match("seed-bot-rated", PlayerId::P1, PersonaName::Silent, true).await;
    to_the_persons_turn(&m).await;
    concede(&m).await;

    let result = q!(m.app, results_get_by_match(MATCH_ID)).expect("the result");
    assert_eq!(result.winner_profile_id.as_deref(), Some(BOT));
    // R610: the bot's own rating moved, on its own row, and the person's moved the other way.
    let bot = q!(m.app, ranked_bot(BOT)).expect("the bot's R610 row");
    assert_eq!(bot.games, 1);
    assert!(bot.glicko.rating > 1000.0, "the bot won: {}", bot.glicko.rating);
    let person = q!(m.app, profiles_get_by_id(PERSON)).expect("the person");
    assert!(person.rating < 1000.0, "the person lost: {}", person.rating);
    // The result let the bot go, as it lets a person go: it is free for the next ticket.
    let freed = q!(m.app, bots_list());
    assert_eq!(freed.len(), 1);
    assert!(freed[0].in_match_id.is_none(), "{:?}", freed[0].in_match_id);
    // R611: one rated game, its sides the person and the bot.
    let rated = to_json(&fake(&m.app).lock().await.tables.rated_games);
    let rows = rated.as_array().cloned().unwrap_or_default();
    assert_eq!(rows.len(), 1);
    let text = rows[0].to_string();
    assert!(text.contains(PERSON) && text.contains(BOT), "{text}");
}

#[tokio::test(start_paused = true)]
async fn r1444_bot_match_replays_from_seed_and_log() {
    let m = bot_match("seed-bot-replay", PlayerId::P2, PersonaName::Silent, false).await;
    to_the_persons_turn(&m).await;
    submit(&m, "person-end-1", json!({ "type": "endTurn" })).await;
    let person = m.person;
    let turn = m.actor.snapshot().turn;
    wait_until(&m.actor, "the bot's turn to end", |actor| {
        let snapshot = actor.snapshot();
        snapshot.active == person && snapshot.turn > turn
    })
    .await;

    let row = q!(m.app, matches_get(MATCH_ID)).expect("the match row");
    assert!(
        row.last_boards.is_none() && row.glitch_boards.is_none(),
        "PREMISE: a plain opening"
    );
    let log: Vec<_> = q!(m.app, matches_actions(MATCH_ID))
        .into_iter()
        .map(|entry| entry.action)
        .collect();
    assert!(
        log.iter().any(|action| bot_nonce(&action.nonce)),
        "PREMISE: the bot moved"
    );
    let folded = fold(&fold_args(&m.seed, &row.decks, log, None, None, None));
    assert!(folded.errors.is_empty(), "{:?}", folded.errors);
    assert_eq!(hash_state(&folded.state), hash_state(&m.actor.engine_state()));
}

#[tokio::test(start_paused = true)]
async fn r1444_restart_resumes_the_bot_seat() {
    let m = bot_match("seed-bot-restart", PlayerId::P1, PersonaName::Silent, false).await;
    // The restart comes before the bot's mulligan lands: the move it was deciding is dropped.
    m.app.matches.stop(MATCH_ID).await;
    assert!(log_of(&m.app).await.is_empty());

    let revived = m
        .app
        .matches
        .actor_for(&m.app, MATCH_ID)
        .await
        .expect("the rebuilt actor");
    let socket = create_fake_socket();
    revived.attach(m.person, socket.socket());
    revived.idle().await;
    let bot = m.bot();
    wait_until(&revived, "the rebuilt bot's mulligan", |actor| {
        !actor.snapshot().mulligan_owed.contains(&bot)
    })
    .await;
    // Give a dropped job every chance to land a second move.
    for _ in 0..8 {
        tokio::time::sleep(Duration::from_millis(WAIT_STEP_MS)).await;
        revived.idle().await;
    }
    let log = log_of(&m.app).await;
    let mulligans: Vec<_> = log
        .iter()
        .filter(|(seat, _, kind)| *seat == bot.as_str() && kind == "mulligan")
        .collect();
    assert_eq!(mulligans.len(), 1, "{log:?}");
    assert!(bot_nonce(&mulligans[0].1));
}

#[tokio::test(start_paused = true)]
async fn r1444_bot_declines_a_draw_offer() {
    let m = bot_match("seed-bot-draw", PlayerId::P1, PersonaName::Silent, false).await;
    to_the_persons_turn(&m).await;
    submit(&m, "person-offer", json!({ "type": "offerDraw" })).await;
    let bot = m.bot().as_str().to_string();
    let app = Arc::clone(&m.app);
    for _ in 0..WAIT_STEPS {
        m.actor.idle().await;
        if log_of(&app)
            .await
            .iter()
            .any(|(seat, _, kind)| *seat == bot && kind == "answerDraw")
        {
            break;
        }
        tokio::time::sleep(Duration::from_millis(WAIT_STEP_MS)).await;
    }
    let answers: Vec<Value> = q!(m.app, matches_actions(MATCH_ID))
        .into_iter()
        .map(|row| to_json(&row.action))
        .filter(|action| action["type"] == "answerDraw")
        .collect();
    assert_eq!(answers.len(), 1, "{answers:?}");
    assert_eq!(answers[0]["accept"], json!(false));
    assert_eq!(answers[0]["playerId"], bot.as_str());
    assert!(
        m.actor.snapshot().result.is_none(),
        "R188: a bot never takes a draw"
    );
}

#[tokio::test(start_paused = true)]
async fn r1444_persona_greets_through_the_relay() {
    // Balanced greets at the end of the mulligan every time (R645), from its pool within its dealt
    // hand (R1344): a seed whose hand holds one of the pool.
    let balanced = AI_PERSONAS
        .iter()
        .find(|(name, _)| *name == PersonaName::Balanced)
        .map(|(_, spec)| spec)
        .expect("the balanced persona");
    let greeting = balanced
        .triggers
        .iter()
        .find(|(trigger, _)| *trigger == EmoteTrigger::MulliganEnd)
        .map(|(_, row)| row)
        .expect("a mulligan-end row");
    assert_eq!(greeting.chance, 1.0, "PREMISE: balanced always greets");
    let seed = (0..200)
        .map(|k| format!("seed-bot-greets-{k}"))
        .find(|seed| {
            let hand = deal_emote_hand(seed, PlayerId::P2);
            greeting.pool.iter().any(|emote| hand.contains(emote))
        })
        .expect("a seed whose bot holds a greeting");

    let m = bot_match(&seed, PlayerId::P1, PersonaName::Balanced, false).await;
    to_the_persons_turn(&m).await;
    let bot = m.bot();
    let socket = &m.socket;
    for _ in 0..WAIT_STEPS {
        m.actor.idle().await;
        if !socket.of_type("emote").is_empty() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(WAIT_STEP_MS)).await;
    }
    let emotes = socket.of_type("emote");
    assert!(!emotes.is_empty(), "the bot never greeted");
    assert_eq!(emotes[0]["from"], bot.as_str());
    let sent: EmoteId = from(emotes[0]["emote"].clone());
    assert!(greeting.pool.contains(&sent), "{sent} is not a greeting");
}

#[tokio::test(start_paused = true)]
async fn r1444_bot_telemetry_is_ai() {
    let m = bot_match("seed-bot-telemetry", PlayerId::P2, PersonaName::Silent, false).await;
    to_the_persons_turn(&m).await;
    concede(&m).await;
    assert!(m.actor.snapshot().result.is_some());

    let data = fake(&m.app);
    let data = data.lock().await;
    let signals = to_json(&data.tables.match_signals);
    let pilot_of = |seat: PlayerId| {
        signals
            .as_array()
            .and_then(|rows| {
                rows.iter()
                    .find(|row| row["matchId"] == MATCH_ID && row["seat"] == seat.as_str())
            })
            .map(|row| row["pilot"].clone())
    };
    assert_eq!(pilot_of(m.person), Some(json!("human")));
    assert_eq!(pilot_of(m.bot()), Some(json!("ai")));
    // R1442: a server action leaves no think time, so the bot's seat has no timing row.
    let timings = to_json(&data.tables.action_timings);
    assert!(
        timings
            .as_array()
            .is_none_or(|rows| rows.iter().all(|row| row["seat"] != m.bot().as_str())),
        "{timings}"
    );
}
