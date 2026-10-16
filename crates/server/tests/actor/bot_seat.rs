//! R1452 (SPEC §9.12): the bot seat. A ladder bot's seat has no socket: the match actor plays it
//! itself, deciding off the queue and moving at a human pace inside its clock, with server nonces
//! (`srv-bot-…`, R270). It gets no disconnect grace (R744), emotes through its persona (R645), is
//! rated as R610 rates a bot, and its match replays from `(seed, log)` and survives a restart like
//! any other.
//!
//! R639000: it answers a draw offer and its mulligan at a person's pace, lets a turn run out at the
//! fitted rate, and shows its aim a beat before a targeted attack, each drawn from the fitted rows.
//!
//! These run the real engine and the real cards (`jackioh_cards::register_all`), not the testkit's
//! scripted ones: the bot decides on the blocking pool, whose threads do not see a test's
//! thread-local card override. The one exception puts a single testkit card in the person's hand and
//! never lets the bot decide while it is in play. Time is tokio's paused clock, which does not move on its own while a
//! decision is running there, so the think times cost no real time and the decisions only theirs.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use serde_json::{Value, json};

use jackioh_engine::wire::{EmoteId, Phase, PlayerId};
use jackioh_engine::{ActionBody, ActionType, Rng, deal_emote_hand};
use jackioh_server::actor::engine::{Pinned, fold, fold_args, hash_state, legal_actions};
use jackioh_server::actor::match_actor::{MatchActor, bot_moved_turns};
use jackioh_server::actor::protocol::SERVER_NONCE_PREFIX;
use jackioh_server::app::{App, now_ms};
use jackioh_server::bots::seat::{THINK_MODEL, ThinkFor, think_ms};
use jackioh_server::bots::signals::{accepts_draw, idles_turn, standing};
use jackioh_server::config::{
    AI_PERSONAS, BOT_DRAW_ANSWER_FIT, BOT_STRENGTHS, BOT_THINK_MAX_MS, BOT_THINK_MIN_MS, BOT_TIMEOUT_FIT,
    EmoteTrigger, MULLIGAN_CLOCK_MS, PersonaName, TURN_CLOCK_MS,
};
use jackioh_server::db::fake::FakeData;
use jackioh_server::db::store::{Db, NewBot, StoreError};

use crate::support::deps::{TestAppOptions, test_app_with};
use crate::support::engine::{TEST_HEARS_EMOTES, install_test_cards};
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
    bot_match_with(seed, person, persona, ranked, &[]).await
}

/// `bot_match`, with `extra` cards first in the person's deck (the testkit's, which the caller
/// installed: each has Quickdraw, so the opening deal puts it in the person's hand).
async fn bot_match_with(
    seed: &str,
    person: PlayerId,
    persona: PersonaName,
    ranked: bool,
    extra: &[&str],
) -> BotMatch {
    let (app, actor) = unopened_bot_match(seed, person, persona, ranked, extra).await;
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

/// `bot_match_with`'s match before the person opens it: started as the queue starts one, with no
/// socket on either seat.
async fn unopened_bot_match(
    seed: &str,
    person: PlayerId,
    persona: PersonaName,
    ranked: bool,
    extra: &[&str],
) -> (Arc<App>, MatchActor) {
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
        // The strongest row: R747000's weaker rows hand some decisions, or all, to the random policy, and
        // these tests are about what the bot's seat does with the AI's turn.
        "strength": BOT_STRENGTHS.len() - 1,
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

    let (mut person_deck, bot_deck) = decks();
    for (at, id) in extra.iter().enumerate() {
        person_deck[at] = (*id).to_string();
    }
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
    (app, actor)
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
async fn r1452_bot_mulligans_and_plays_without_a_socket() {
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
    // Nothing the person's socket carried tells the bot's seat from a person's: no bot nonce, no
    // pilot.
    let frames = m.socket.sent();
    assert!(!frames.is_empty(), "the person was sent their views");
    for frame in frames {
        assert!(!frame.contains(&format!("{SERVER_NONCE_PREFIX}bot-")), "{frame}");
        assert!(!frame.contains("\"pilot\""), "{frame}");
    }
}

#[tokio::test(start_paused = true)]
async fn r1452_a_move_whose_write_failed_is_decided_again_not_traded_for_the_fallback() {
    let m = bot_match("seed-bot-write", PlayerId::P1, PersonaName::Silent, false).await;
    to_the_persons_turn(&m).await;
    submit(&m, "person-end-write", json!({ "type": "endTurn" })).await;
    let before = log_of(&m.app).await.len();
    // The bot's first write of its turn fails, as a store that blips would.
    let refused = Arc::new(AtomicBool::new(false));
    let once = Arc::clone(&refused);
    fake(&m.app).lock().await.on_call = Some(Arc::new(move |method: &str| {
        if method == "matches.appendActions" && !once.swap(true, Ordering::SeqCst) {
            return Err(StoreError::Other(format!("{method} refused by the test")));
        }
        Ok(())
    }));
    let bot = m.bot().as_str().to_string();
    let app = Arc::clone(&m.app);
    for _ in 0..WAIT_STEPS {
        m.actor.idle().await;
        if refused.load(Ordering::SeqCst)
            && log_of(&app).await[before..]
                .iter()
                .any(|(seat, nonce, _)| *seat == bot && bot_nonce(nonce))
        {
            break;
        }
        tokio::time::sleep(Duration::from_millis(WAIT_STEP_MS)).await;
    }
    assert!(refused.load(Ordering::SeqCst), "the bot's write was refused");
    let log = log_of(&m.app).await;
    let moves: Vec<&(String, String, String)> = log[before..]
        .iter()
        .filter(|(seat, nonce, _)| *seat == bot && bot_nonce(nonce))
        .collect();
    assert!(!moves.is_empty(), "the bot moved after all: {log:?}");
    // Decided again, not swapped for the fallback move (its `-r` retry).
    assert!(
        moves.iter().all(|(_, nonce, _)| !nonce.ends_with("-r")),
        "{log:?}"
    );
}

#[tokio::test(start_paused = true)]
async fn r1452_the_persons_actions_never_hold_the_bots_move_back() {
    let m = bot_match("seed-bot-stale", PlayerId::P1, PersonaName::Silent, false).await;
    to_the_persons_turn(&m).await;
    submit(&m, "person-end-stale", json!({ "type": "endTurn" })).await;
    let turn_began = tokio::time::Instant::now();
    let before = log_of(&m.app).await.len();
    let bot = m.bot().as_str().to_string();
    // Through the bot's turn the person flips their auto-end-turn switch faster than the bot's
    // quickest move, so every move the bot decides arrives after the log has moved on.
    let flip_ms = u64::try_from(BOT_THINK_MIN_MS / 3).expect("a wait");
    let mut moved = false;
    for at in 0..WAIT_STEPS {
        let flip = json!({ "type": "setAutoEndTurn", "enabled": at % 2 == 1 });
        submit(&m, &format!("person-flip-{at}"), flip).await;
        let log = log_of(&m.app).await;
        if log[before..]
            .iter()
            .any(|(seat, nonce, _)| *seat == bot && bot_nonce(nonce))
        {
            moved = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(flip_ms)).await;
    }
    let log = log_of(&m.app).await;
    assert!(moved, "the bot never moved: {log:?}");
    // Its first move went out after the think time it first drew, not only once its clock ran so
    // low that its wait came in under the person's flips.
    let waited = turn_began.elapsed();
    let longest = Duration::from_millis(u64::try_from(BOT_THINK_MAX_MS).expect("a wait") + flip_ms);
    assert!(waited <= longest, "the bot's first move took {waited:?}: {log:?}");
    assert!(
        !log[before..].iter().any(|(_, _, kind)| kind == "timeout"),
        "the bot's clock ran out first: {log:?}"
    );
}

#[tokio::test(start_paused = true)]
async fn r1452_bot_seat_never_gets_a_grace() {
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
async fn r1452_r744_a_person_who_never_opens_a_bot_match_loses_after_the_grace() {
    let (app, actor) =
        unopened_bot_match("seed-bot-absent", PlayerId::P2, PersonaName::Silent, false, &[]).await;
    actor.idle().await;
    // The bot is at the table as the match arms, as an opponent whose socket attached, so the person's
    // grace runs from then; the bot's seat never gets one.
    let row = q!(app, matches_get(MATCH_ID)).expect("the match row");
    let grace = to_json(&row.clocks)["graceDeadline"].clone();
    assert!(grace["p2"].is_i64(), "the person is away: {grace}");
    assert!(grace["p1"].is_null(), "the bot never is: {grace}");

    wait_until(&actor, "the match's end", |actor| {
        actor.snapshot().result.is_some()
    })
    .await;
    let log = log_of(&app).await;
    let last = log.last().expect("a logged action");
    assert_eq!(
        (last.0.as_str(), last.2.as_str()),
        ("p2", "disconnectExpired"),
        "{log:?}"
    );
    let result = q!(app, results_get_by_match(MATCH_ID)).expect("the result");
    assert_eq!(result.winner_profile_id.as_deref(), Some(BOT));
    // The result let the bot go, free for the next ticket.
    let bots = q!(app, bots_list());
    assert!(bots[0].in_match_id.is_none(), "{:?}", bots[0].in_match_id);
}

#[tokio::test(start_paused = true)]
async fn r1452_r610_bot_match_rated_by_r610() {
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
async fn r1452_bot_match_replays_from_seed_and_log() {
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
    let folded = fold(
        Pinned::default(),
        &fold_args(&m.seed, &row.decks, log, None, None, None),
    );
    assert!(folded.errors.is_empty(), "{:?}", folded.errors);
    assert_eq!(hash_state(&folded.state), hash_state(&m.actor.engine_state()));
}

#[tokio::test(start_paused = true)]
async fn r1452_restart_resumes_the_bot_seat() {
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
async fn r1452_a_restart_mid_game_resumes_the_bot_on_its_next_turn() {
    let m = bot_match("seed-bot-restart-mid", PlayerId::P2, PersonaName::Silent, false).await;
    to_the_persons_turn(&m).await;
    let turn = m.actor.snapshot().turn;
    m.app.matches.stop(MATCH_ID).await;
    let before = log_of(&m.app).await.len();

    let revived = m
        .app
        .matches
        .actor_for(&m.app, MATCH_ID)
        .await
        .expect("the rebuilt actor");
    let socket = create_fake_socket();
    revived.attach(m.person, socket.socket());
    revived.idle().await;
    let reply = revived
        .submit(
            m.person,
            "person-end-after-restart".to_string(),
            from(json!({ "type": "endTurn" })),
        )
        .await;
    assert_eq!(to_json(&reply)["type"], "ack", "{}", to_json(&reply));
    let person = m.person;
    wait_until(&revived, "the rebuilt bot's turn to end", |actor| {
        let snapshot = actor.snapshot();
        snapshot.active == person && snapshot.turn > turn
    })
    .await;

    let log = log_of(&m.app).await;
    let bot = m.bot().as_str().to_string();
    assert!(
        log[before..]
            .iter()
            .any(|(seat, nonce, kind)| *seat == bot && bot_nonce(nonce) && kind == "endTurn"),
        "{log:?}"
    );
    let row = q!(m.app, matches_get(MATCH_ID)).expect("the match row");
    let actions: Vec<_> = q!(m.app, matches_actions(MATCH_ID))
        .into_iter()
        .map(|entry| entry.action)
        .collect();
    let folded = fold(
        Pinned::default(),
        &fold_args(&m.seed, &row.decks, actions, None, None, None),
    );
    assert!(folded.errors.is_empty(), "{:?}", folded.errors);
    assert_eq!(hash_state(&folded.state), hash_state(&revived.engine_state()));
}

/// The match's action log as `(seq, action, at)`, in order.
async fn rows_of(app: &App) -> Vec<(i64, Value, i64)> {
    q!(app, matches_actions(MATCH_ID))
        .into_iter()
        .map(|row| (row.seq, to_json(&row.action), row.at))
        .collect()
}

/// The think time the bot that began in `home` draws for one move: its timing stream at `cursor`,
/// on `state` as it decided, at speed 1 (`unopened_bot_match`'s bot).
fn drawn_think_ms(
    seed: &str,
    home: PlayerId,
    cursor: u32,
    action_kind: ActionType,
    first_in_turn: bool,
    legal_count: usize,
    clock_left_ms: Option<i64>,
) -> i64 {
    think_ms(
        &mut Rng::new(&format!("{seed}:bot-timing:{home}"), cursor),
        &ThinkFor {
            action_kind,
            first_in_turn,
            legal_count,
            lethal: false,
        },
        &THINK_MODEL,
        1.0,
        clock_left_ms,
    )
}

#[tokio::test(start_paused = true)]
async fn r639000_a_bot_answers_a_draw_offer_at_its_fitted_pace() {
    let m = bot_match("seed-bot-draw", PlayerId::P1, PersonaName::Silent, false).await;
    to_the_persons_turn(&m).await;
    submit(&m, "person-offer", json!({ "type": "offerDraw" })).await;
    // The state the bot answers on, as the offer left it.
    let state = m.actor.engine_state();
    let bot = m.bot();
    let app = Arc::clone(&m.app);
    for _ in 0..WAIT_STEPS {
        m.actor.idle().await;
        if log_of(&app)
            .await
            .iter()
            .any(|(seat, _, kind)| *seat == bot.as_str() && kind == "answerDraw")
        {
            break;
        }
        tokio::time::sleep(Duration::from_millis(WAIT_STEP_MS)).await;
    }
    let rows = rows_of(&m.app).await;
    let offer = rows
        .iter()
        .find(|(_, action, _)| action["type"] == "offerDraw")
        .expect("the person's offer");
    let answers: Vec<&(i64, Value, i64)> = rows
        .iter()
        .filter(|(_, action, _)| action["type"] == "answerDraw")
        .collect();
    assert_eq!(answers.len(), 1, "{rows:?}");
    let (_, answer, answered_at) = answers[0];
    assert_eq!(answer["playerId"], bot.as_str());
    assert!(
        bot_nonce(answer["nonce"].as_str().unwrap_or_default()),
        "{answer}"
    );
    // Not at once: a person's think over an offer, the `answerDraw` prior, drawn on the bot's timing
    // stream after its mulligan's two draws, on the person's turn, when no clock runs for the bot.
    let expected = drawn_think_ms(
        &m.seed,
        bot,
        2,
        ActionType::AnswerDraw,
        false,
        legal_actions(Pinned::default(), &state, bot).len(),
        None,
    );
    assert_eq!(answered_at - offer.2, expected, "{rows:?}");
    // Accepted exactly as the bot's draw-answer roll says, from how far behind it stood.
    let accept = accepts_draw(
        &m.seed,
        bot,
        state.turn,
        &standing(&state, bot, &m.seed, bot),
        &BOT_DRAW_ANSWER_FIT,
    );
    assert_eq!(answer["accept"], json!(accept), "{rows:?}");
    assert_eq!(m.actor.snapshot().result.is_some(), accept);
}

#[tokio::test(start_paused = true)]
async fn r639000_a_seeded_bot_game_draws_its_mulligan_and_idle_turn_from_the_fits() {
    // A seed whose bot, the second seat, lets its first turn run out.
    let seed = (0..20_000)
        .map(|k| format!("seed-idle-{k}"))
        .find(|seed| idles_turn(seed, PlayerId::P2, 2, &BOT_TIMEOUT_FIT))
        .expect("a seed whose bot idles turn 2");
    let m = bot_match(&seed, PlayerId::P1, PersonaName::Silent, false).await;
    let bot = m.bot();
    let row = q!(m.app, matches_get(MATCH_ID)).expect("the match row");
    // The opening state, before the bot's mulligan lands.
    let opening = m.actor.engine_state();
    assert!(log_of(&m.app).await.is_empty(), "PREMISE: no move yet");

    to_the_persons_turn(&m).await;
    let rows = rows_of(&m.app).await;
    let mulligan = rows
        .iter()
        .find(|(_, action, _)| action["type"] == "mulligan" && action["playerId"] == bot.as_str())
        .expect("the bot's mulligan");
    // A person's mulligan, the `mulligan` prior, not the practice AI's instant answer (R266); the
    // shared mulligan clock (R268) leaves more than the longest think time.
    let expected = drawn_think_ms(
        &seed,
        bot,
        0,
        ActionType::Mulligan,
        true,
        legal_actions(Pinned::default(), &opening, bot).len(),
        Some(MULLIGAN_CLOCK_MS),
    );
    assert_eq!(mulligan.2 - row.created_at, expected, "{rows:?}");

    // The bot's first turn: it plays nothing, and its turn clock times it out (R79).
    let turn = m.actor.snapshot().turn;
    assert_eq!(turn, 1, "PREMISE: the person opens");
    submit(&m, "person-end-idle", json!({ "type": "endTurn" })).await;
    let ended = rows_of(&m.app).await.len();
    let person = m.person;
    wait_until(&m.actor, "the person's next turn", |actor| {
        let snapshot = actor.snapshot();
        snapshot.active == person && snapshot.turn > turn + 1
    })
    .await;
    let rows = rows_of(&m.app).await;
    let end_at = rows[ended - 1].2;
    let since: Vec<&(i64, Value, i64)> = rows[ended..].iter().collect();
    assert!(
        since
            .iter()
            .all(|(_, action, _)| !bot_nonce(action["nonce"].as_str().unwrap_or_default())),
        "the bot moved on its idle turn: {rows:?}"
    );
    let timeouts: Vec<&&(i64, Value, i64)> = since
        .iter()
        .filter(|(_, action, _)| action["type"] == "timeout")
        .collect();
    assert_eq!(timeouts.len(), 1, "{rows:?}");
    assert_eq!(timeouts[0].1["playerId"], bot.as_str());
    assert_eq!(timeouts[0].2 - end_at, TURN_CLOCK_MS, "{rows:?}");
}

#[tokio::test(start_paused = true)]
async fn r639000_a_bot_aims_a_beat_before_a_targeted_attack() {
    let m = bot_match("seed-bot-aims", PlayerId::P1, PersonaName::Silent, false).await;
    to_the_persons_turn(&m).await;
    let bot = m.bot();
    let person = m.person;
    // The person ends each turn until the bot has attacked.
    let attacked = |frames: &[Value]| {
        frames.iter().position(|frame| {
            frame["type"] == "view"
                && frame["view"]["events"]
                    .as_array()
                    .is_some_and(|events| events.iter().any(|event| event["type"] == "attackDeclared"))
        })
    };
    let frames = |m: &BotMatch| -> Vec<Value> {
        m.socket
            .sent()
            .iter()
            .map(|text| serde_json::from_str(text).expect("every frame is JSON"))
            .collect()
    };
    for at in 0..8 {
        if attacked(&frames(&m)).is_some() || m.actor.snapshot().result.is_some() {
            break;
        }
        submit(&m, &format!("person-end-aim-{at}"), json!({ "type": "endTurn" })).await;
        let turn = m.actor.snapshot().turn;
        wait_until(&m.actor, "the bot's turn to end", |actor| {
            let snapshot = actor.snapshot();
            snapshot.result.is_some() || (snapshot.active == person && snapshot.turn > turn)
        })
        .await;
    }
    let frames = frames(&m);
    let at = attacked(&frames).expect("the bot attacked");
    let aims: Vec<(usize, &Value)> = frames
        .iter()
        .enumerate()
        .filter(|(_, frame)| frame["type"] == "aim")
        .collect();
    assert!(
        aims.iter().all(|(_, frame)| frame["from"] == bot.as_str()),
        "{aims:?}"
    );
    // The aim that showed as the attack went: from the attacker's zone on the bot's side, at a target.
    let (_, before) = aims
        .iter()
        .rev()
        .find(|(index, _)| *index < at)
        .expect("an aim before the attack");
    assert_eq!(before["aim"]["source"]["at"], "zone", "{before}");
    assert_eq!(before["aim"]["source"]["player"], bot.as_str(), "{before}");
    assert!(!before["aim"]["target"].is_null(), "{before}");
    // Then taken down, after the attack.
    assert!(
        aims.iter()
            .any(|(index, frame)| *index > at && frame["aim"].is_null()),
        "{aims:?}"
    );
}

#[tokio::test(start_paused = true)]
async fn r1452_persona_greets_through_the_relay() {
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
async fn r1452_r1127_a_card_that_hears_emotes_hears_the_bots() {
    // The person holds the testkit's Field Spell that hears emotes. Its card is the only test card
    // in the match, and the person plays it on their own turn, which they never end, so the bot
    // never decides while it is in play (the blocking pool's threads do not see the testkit).
    install_test_cards();
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
    let seed = (0..200)
        .map(|k| format!("seed-bot-heard-{k}"))
        .find(|seed| {
            let hand = deal_emote_hand(seed, PlayerId::P2);
            greeting.pool.iter().any(|emote| hand.contains(emote))
        })
        .expect("a seed whose bot holds a greeting");

    let m = bot_match_with(
        &seed,
        PlayerId::P1,
        PersonaName::Balanced,
        false,
        &[TEST_HEARS_EMOTES],
    )
    .await;
    to_the_persons_turn(&m).await;
    // The bot's greeting is still on its delay (R645) when the card goes down.
    assert!(
        m.socket.of_type("emote").is_empty(),
        "PREMISE: the bot greeted already"
    );
    submit(
        &m,
        "person-stay",
        json!({ "type": "setAutoEndTurn", "enabled": false }),
    )
    .await;
    let card = to_json(&m.actor.view_for(m.person))["you"]["hand"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .find(|card| card["defId"] == TEST_HEARS_EMOTES)
        .map(|card| card["instanceId"].clone())
        .expect("the hearing card is in the person's hand");
    submit(&m, "person-hear", json!({ "type": "play", "instanceId": card })).await;

    let bot = m.bot().as_str().to_string();
    let minted = |log: &[(String, String, String)]| {
        log.iter()
            .filter(|(seat, nonce, kind)| {
                *seat == bot && nonce.starts_with(&format!("{SERVER_NONCE_PREFIX}emote-")) && kind == "emote"
            })
            .count()
    };
    for _ in 0..WAIT_STEPS {
        m.actor.idle().await;
        if minted(&log_of(&m.app).await) > 0 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(WAIT_STEP_MS)).await;
    }
    // The greeting went to the person as a person's would, then ran as the bot's `Emote` action.
    let relayed = m.socket.of_type("emote");
    assert_eq!(relayed.len(), 1, "{relayed:?}");
    assert_eq!(relayed[0]["from"], bot.as_str());
    let log = log_of(&m.app).await;
    assert_eq!(minted(&log), 1, "{log:?}");
}

#[tokio::test(start_paused = true)]
async fn r1452_bot_telemetry_is_ai() {
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

#[tokio::test(start_paused = true)]
async fn r639000_a_restart_mid_turn_does_not_begin_the_bots_turn_again() {
    // The bot's turn begins once (its first move's think time, its concession, its idle roll): a
    // rebuild reads off the log the turn the bot last moved in, so its next move after a restart in
    // the middle of its turn is a later move's, not a first one's.
    let m = bot_match("seed-bot-mid-turn", PlayerId::P1, PersonaName::Silent, false).await;
    let bot = m.bot();
    to_the_persons_turn(&m).await;
    submit(&m, "person-end-mid", json!({ "type": "endTurn" })).await;
    let app = Arc::clone(&m.app);
    let mut moved = false;
    for _ in 0..WAIT_STEPS {
        m.actor.idle().await;
        let log = log_of(&app).await;
        let ended = log
            .iter()
            .position(|(_, nonce, _)| nonce == "person-end-mid")
            .expect("the person's end of turn");
        if log[ended + 1..].iter().any(|(_, nonce, _)| bot_nonce(nonce)) {
            moved = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(WAIT_STEP_MS)).await;
    }
    assert!(moved, "the bot never moved in its turn");
    let snapshot = m.actor.snapshot();
    assert!(
        snapshot.active == bot && snapshot.result.is_none(),
        "PREMISE: the bot's first move left its turn going: {:?}",
        log_of(&app).await
    );
    let turn = snapshot.turn;
    m.app.matches.stop(MATCH_ID).await;
    let before = log_of(&app).await.len();

    let row = q!(m.app, matches_get(MATCH_ID)).expect("the match row");
    let log = q!(m.app, matches_actions(MATCH_ID));
    assert_eq!(
        bot_moved_turns(Pinned::default(), &row, &log, None)[bot],
        Some(turn),
        "the log says the bot has moved this turn"
    );

    let rebuilt_at = now_ms();
    let revived = m
        .app
        .matches
        .actor_for(&m.app, MATCH_ID)
        .await
        .expect("the rebuilt actor");
    let state = revived.engine_state();
    let socket = create_fake_socket();
    revived.attach(m.person, socket.socket());
    let mut next = None;
    for _ in 0..WAIT_STEPS {
        revived.idle().await;
        let rows = rows_of(&app).await;
        if let Some(found) = rows[before..]
            .iter()
            .find(|(_, action, _)| action["playerId"] == bot.as_str())
        {
            next = Some(found.clone());
            break;
        }
        tokio::time::sleep(Duration::from_millis(WAIT_STEP_MS)).await;
    }
    let (_, action, at) = next.expect("the rebuilt bot's next move");
    let kind: ActionType = from(action["type"].clone());
    let legal = legal_actions(Pinned::default(), &state, bot).len();
    // The rebuilt actor's timing stream starts over (R1452); the move is a later one of its turn.
    let later = drawn_think_ms(&m.seed, bot, 0, kind, false, legal, None);
    let first = drawn_think_ms(&m.seed, bot, 0, kind, true, legal, None);
    assert_ne!(later, first, "PREMISE: the two priors draw apart");
    assert_eq!(at - rebuilt_at, later, "{action}");
}
