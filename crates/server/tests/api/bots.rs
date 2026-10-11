//! R1448 (SPEC §9.5): the empty-queue fallback. With `LADDER_BOTS=on`, a ranked Best of 1 or All
//! Random ticket that has waited past its own fallback time (15–40 s, from its id) with no person to
//! pair with is paired with a ladder bot: free, on its schedule, the closest in rating, and not one the
//! player faced in their last games. A player rated above every bot never gets one, Conquest never
//! does, and a person inside the window always pairs first.
//!
//! R1450 (SPEC §9.11): the person's side of a bot game counts toward the card statistics and R654's
//! gate like any live game; the bot's side is recorded as `ai` and read only when a query asks.
//!
//! R1447 (SPEC §9.12): nothing a client reads tells a bot's seat from a person's; the match screen
//! shows the rank the bot's rating calls for.
//!
//! Everything runs through `try_pair` on tokio's paused clock, on the fake store, the real catalog and
//! the real registry (a started match is a real game). Bots are written through the store, as the pool
//! writes them (`bots_create`, `bots_set_decks`).

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use indexmap::IndexMap;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use tokio::sync::MutexGuard;

use jackioh_engine::{PlayerId, pick_portrait_from_seed};
use jackioh_server::api::queue::try_pair;
use jackioh_server::app::{App, load_server_env, now_ms};
use jackioh_server::bots::fallback::{fallback_due, fallback_seconds, fallback_seconds_keyed, hour_of};
use jackioh_server::config::{
    BOT_FALLBACK_MAX_SECONDS, BOT_FALLBACK_MIN_SECONDS, BOT_RECENT_OPPONENTS, JLORIOUS_SIZE,
    RANK_PLACEMENT_GAMES,
};
use jackioh_server::db::fake::FakeData;
use jackioh_server::db::store::{BotDeck, Db, NewBot, StoreError, Ticket};
use jackioh_server::env::SERVER_ONLY_ENV_VARS;
use jackioh_server::ranked::ladder::{GrapeTier, fresh_rank, tier_bottom};
use jackioh_server::ranked::season::season_id_of;

use crate::support::deps::{TestAppOptions, add_user, call, test_app_with};

// Plumbing (private copies: each test file of this binary keeps its own)

fn from<T: DeserializeOwned>(value: Value) -> T {
    match serde_json::from_value(value.clone()) {
        Ok(parsed) => parsed,
        Err(error) => panic!("{error}: {value}"),
    }
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

async fn fake(app: &App) -> MutexGuard<'_, FakeData> {
    match &app.db {
        Db::Fake(data) => data.lock().await,
        Db::Pg(_) => panic!("the server's unit tests run on the fake store"),
    }
}

/// The app with the fallback switched on (or off), and no fixtures.
async fn app_with_bots(on: bool) -> Arc<App> {
    let env = if on {
        vec![("LADDER_BOTS".to_string(), "on".to_string())]
    } else {
        Vec::new()
    };
    test_app_with(TestAppOptions {
        env,
        skip_fixtures: true,
        ..TestAppOptions::default()
    })
    .await
}

/// Unit conversion, not configuration: the fallback time speaks seconds.
const MS_PER_SECOND: i64 = 1000;

static NEXT_ID: AtomicU32 = AtomicU32::new(0);

fn next_id(prefix: &str) -> String {
    format!("{prefix}-{}", NEXT_ID.fetch_add(1, Ordering::SeqCst) + 1)
}

/// The catalog's playable ids (no tokens), in catalog order.
fn playable() -> Vec<String> {
    jackioh_cards::register_all();
    jackioh_cards::CATALOG
        .values()
        .filter(|def| !def.token)
        .map(|def| def.id.clone())
        .collect()
}

fn deck(slot: usize) -> Vec<String> {
    let size = usize::try_from(jackioh_engine::config::DECK_SIZE).expect("a deck size");
    playable()[slot * size..(slot + 1) * size].to_vec()
}

/// An active person rated `rating`, with a token that verifies as them.
async fn new_person(app: &App, rating: f64) -> (String, String) {
    let id = next_id("person");
    let user_id = format!("user-{id}");
    fake(app)
        .await
        .seed_profile(json!({ "id": id, "userId": user_id, "status": "active", "rating": rating }));
    let token = add_user(app, &user_id, &format!("{id}@example.test"), true);
    (id, token)
}

/// What a test bot is: its rating, schedule and whether it holds a deck.
struct BotSpec {
    rating: f64,
    /// `(active_from_hour, active_hours)`; on all day when `None`.
    schedule: Option<(i32, i32)>,
    decks: bool,
}

impl BotSpec {
    fn rated(rating: f64) -> BotSpec {
        BotSpec {
            rating,
            schedule: None,
            decks: true,
        }
    }
}

/// A bot in the pool, as the pool writes one; answers its profile id.
async fn new_bot(app: &App, spec: BotSpec) -> String {
    let id = next_id("bot");
    let (from_hour, hours) = spec.schedule.unwrap_or((0, 24));
    let new: NewBot = from(json!({
        "profileId": id,
        "strength": 2,
        "speed": 1.0,
        "persona": "balanced",
        "activeFromHour": from_hour,
        "activeHours": hours,
        "decks": [],
        "decksCatalogVersion": null,
        "createdAt": now_ms() - 30 * 86_400_000_i64,
        "seedRating": spec.rating,
    }));
    q!(app, bots_create(&new));
    if spec.decks {
        let decks = vec![BotDeck {
            cards: deck(1),
            portrait: pick_portrait_from_seed("bot-deck").to_string(),
        }];
        q!(app, bots_set_decks(&id, &decks, &app.catalog.version));
    }
    id
}

/// A ticket for `profile_id`, written straight into the store, that has waited `waited_ms`.
async fn ticket(app: &App, profile_id: &str, rating: f64, waited_ms: i64, mode: &str) -> Ticket {
    let row: Ticket = from(json!({
        "id": next_id("ticket"),
        "profileId": profile_id,
        "rating": rating,
        "mode": mode,
        "deck": if mode == "bo1" { json!(deck(0)) } else { json!([]) },
        "portrait": null,
        "trio": null,
        "catalogVersion": app.catalog.version,
        "enqueuedAt": now_ms() - waited_ms,
        "status": "open",
        "matchId": null,
    }));
    q!(app, tickets_insert(&row));
    row
}

/// Past the longest fallback time any ticket draws.
fn past_any_fallback() -> i64 {
    (BOT_FALLBACK_MAX_SECONDS + 1) * MS_PER_SECOND
}

/// Each match row's players, in table order.
async fn matches(app: &App) -> Vec<(String, String)> {
    fake(app)
        .await
        .tables
        .matches
        .iter()
        .map(|row| row.players.clone())
        .collect()
}

/// The bot each match seated, of those in `bots`.
async fn seated(app: &App, bots: &[&str]) -> Vec<String> {
    matches(app)
        .await
        .into_iter()
        .flat_map(|(a, b)| [a, b])
        .filter(|id| bots.contains(&id.as_str()))
        .collect()
}

/// A finished bot match: the person concedes it at once. Answers `(match id, the person's seat)`.
async fn concede_bot_match(app: &Arc<App>, person_id: &str) -> (String, PlayerId) {
    let row = fake(app)
        .await
        .tables
        .matches
        .last()
        .cloned()
        .expect("a bot match was started");
    let seat = if row.players.0 == person_id {
        PlayerId::P1
    } else {
        PlayerId::P2
    };
    let actor = app
        .matches
        .actor_for(app, &row.id)
        .await
        .expect("the match's actor");
    let reply = actor
        .submit(seat, "concede-1".to_string(), from(json!({ "type": "concede" })))
        .await;
    assert_eq!(
        serde_json::to_value(&reply).expect("a reply")["type"],
        "ack",
        "the concession was refused"
    );
    for _ in 0..10 {
        actor.idle().await;
        tokio::task::yield_now().await;
    }
    (row.id, seat)
}

#[test]
fn r1448_fallback_time_is_15_to_40_s_and_stable() {
    let mut seen_min = false;
    let mut seen_max = false;
    for k in 0..2_000 {
        let id = format!("00000000-0000-4000-8000-{k:012}");
        let seconds = fallback_seconds(&id);
        assert!(
            (BOT_FALLBACK_MIN_SECONDS..=BOT_FALLBACK_MAX_SECONDS).contains(&seconds),
            "{id}: {seconds} s"
        );
        assert_eq!(fallback_seconds(&id), seconds, "the same ticket, the same time");
        seen_min |= seconds == BOT_FALLBACK_MIN_SECONDS;
        seen_max |= seconds == BOT_FALLBACK_MAX_SECONDS;
    }
    assert!(seen_min && seen_max, "both ends of the range are drawn");
}

#[test]
fn r1448_no_client_can_work_out_the_fallback_time() {
    // What a client holds: its ticket's id (`ticketId`), the range, and the engine's `Rng`, which runs
    // in its browser. A time drawn from those alone would tell it the moment a bot would come.
    let span = i32::try_from(BOT_FALLBACK_MAX_SECONDS - BOT_FALLBACK_MIN_SECONDS + 1).expect("a small range");
    let from_the_id_alone = |id: &str| {
        BOT_FALLBACK_MIN_SECONDS
            + i64::from(jackioh_engine::Rng::new(&format!("{id}:bot-fallback"), 0).int(span))
    };
    let ids: Vec<String> = (0..2_000)
        .map(|k| format!("00000000-0000-4000-8000-{k:012}"))
        .collect();
    let guessed = ids
        .iter()
        .filter(|id| fallback_seconds(id) == from_the_id_alone(id))
        .count();
    // A guess agrees about one time in 26 by chance; anything near every time is a leak.
    assert!(
        guessed < ids.len() / 4,
        "{guessed} of {} guessed from the id alone",
        ids.len()
    );
    // The server's key is what the time turns on.
    let differs = ids
        .iter()
        .filter(|id| fallback_seconds_keyed("key-a", id) != fallback_seconds_keyed("key-b", id))
        .count();
    assert!(differs > ids.len() / 2, "only {differs} times moved with the key");
}

#[tokio::test(start_paused = true)]
async fn r1448_no_bot_before_the_fallback_time() {
    let app = app_with_bots(true).await;
    let (person, _) = new_person(&app, 1000.0).await;
    let bot = new_bot(&app, BotSpec::rated(1000.0)).await;
    // A Best-of-1 ticket one second short of its own fallback time.
    let id = next_id("ticket");
    let wait = fallback_seconds(&id) * MS_PER_SECOND;
    let early: Ticket = from(json!({
        "id": id,
        "profileId": person,
        "rating": 1000,
        "mode": "bo1",
        "deck": deck(0),
        "portrait": null,
        "trio": null,
        "catalogVersion": app.catalog.version,
        "enqueuedAt": now_ms() - (wait - MS_PER_SECOND),
        "status": "open",
        "matchId": null,
    }));
    q!(app, tickets_insert(&early));
    assert!(!fallback_due(&early, now_ms()));

    assert_eq!(try_pair(&app).await.expect("a sweep"), 0);
    assert!(matches(&app).await.is_empty());

    // Once it is due, the next sweep pairs it.
    tokio::time::advance(std::time::Duration::from_millis(1_000)).await;
    assert!(fallback_due(&early, now_ms()));
    assert_eq!(try_pair(&app).await.expect("a sweep"), 1);
    assert_eq!(seated(&app, &[&bot]).await, vec![bot.clone()]);
}

#[tokio::test(start_paused = true)]
async fn r1448_the_closest_free_scheduled_bot_pairs() {
    let app = app_with_bots(true).await;
    let (person, _) = new_person(&app, 1000.0).await;
    let far_below = new_bot(&app, BotSpec::rated(900.0)).await;
    let closest = new_bot(&app, BotSpec::rated(1050.0)).await;
    let far_above = new_bot(&app, BotSpec::rated(1300.0)).await;
    let waiting = ticket(&app, &person, 1000.0, past_any_fallback(), "bo1").await;

    assert_eq!(try_pair(&app).await.expect("a sweep"), 1);
    assert_eq!(
        seated(&app, &[&far_below, &closest, &far_above]).await,
        vec![closest.clone()]
    );

    // The ticket is matched, both seats are in the match, and it is ranked (R604) and Best of 1.
    let row = fake(&app).await.tables.matches[0].clone();
    let held = q!(app, tickets_get(&waiting.id)).expect("the ticket");
    assert_eq!(
        serde_json::to_value(&held).expect("a ticket")["status"],
        "matched"
    );
    assert_eq!(held.match_id.as_deref(), Some(row.id.as_str()));
    assert_eq!(row.ranked, Some(true));
    for id in [&person, &closest] {
        let profile = q!(app, profiles_get_by_id(id)).expect("the profile");
        assert_eq!(profile.in_match_id.as_deref(), Some(row.id.as_str()));
    }
    // The person plays their frozen deck, the bot the deck it holds.
    let (person_deck, bot_deck) = if row.players.0 == person {
        (&row.decks.0, &row.decks.1)
    } else {
        (&row.decks.1, &row.decks.0)
    };
    assert_eq!(person_deck, &deck(0));
    assert_eq!(bot_deck, &deck(1));
}

#[tokio::test(start_paused = true)]
async fn r1448_the_player_is_player_1_as_the_older_ticket_is() {
    // Between two people the older ticket is p1 and goes first; a ticket past its fallback time is
    // older than anyone who would come to pair with it, so a bot match seats the person p1 every time.
    // A coin for the seats would put the person second about half of these.
    let app = app_with_bots(true).await;
    for round in 0..8 {
        let (person, _) = new_person(&app, 1000.0).await;
        let bot = new_bot(&app, BotSpec::rated(1000.0)).await;
        let mode = if round % 2 == 0 { "bo1" } else { "random" };
        ticket(&app, &person, 1000.0, past_any_fallback(), mode).await;
        assert_eq!(try_pair(&app).await.expect("a sweep"), 1, "round {round}");
        let row = fake(&app).await.tables.matches.last().cloned().expect("a match");
        assert_eq!(row.players, (person, bot), "round {round} ({mode})");
    }
}

#[tokio::test(start_paused = true)]
async fn r1448_a_human_inside_the_window_pairs_first() {
    let app = app_with_bots(true).await;
    let (a, _) = new_person(&app, 1000.0).await;
    let (b, _) = new_person(&app, 1010.0).await;
    let bot = new_bot(&app, BotSpec::rated(1000.0)).await;
    ticket(&app, &a, 1000.0, past_any_fallback(), "bo1").await;
    ticket(&app, &b, 1010.0, past_any_fallback(), "bo1").await;

    assert_eq!(try_pair(&app).await.expect("a sweep"), 1);
    let pairs = matches(&app).await;
    assert_eq!(pairs.len(), 1);
    let (p1, p2) = &pairs[0];
    let mut players = vec![p1.clone(), p2.clone()];
    players.sort();
    let mut people = vec![a.clone(), b.clone()];
    people.sort();
    assert_eq!(players, people, "the two people pair with each other");
    assert!(seated(&app, &[&bot]).await.is_empty());
}

#[tokio::test(start_paused = true)]
async fn r1448_conquest_never_gets_a_bot() {
    let app = app_with_bots(true).await;
    let (person, _) = new_person(&app, 1000.0).await;
    new_bot(&app, BotSpec::rated(1000.0)).await;
    let conquest = ticket(&app, &person, 1000.0, 10 * past_any_fallback(), "bo3").await;
    assert!(!fallback_due(&conquest, now_ms()));
    assert_eq!(try_pair(&app).await.expect("a sweep"), 0);
    assert!(matches(&app).await.is_empty());

    // All Random does get one.
    let (other, _) = new_person(&app, 1000.0).await;
    ticket(&app, &other, 1000.0, past_any_fallback(), "random").await;
    assert_eq!(try_pair(&app).await.expect("a sweep"), 1);
}

#[tokio::test(start_paused = true)]
async fn r1448_recent_retired_off_schedule_and_busy_bots_are_skipped() {
    let app = app_with_bots(true).await;
    let (person, _) = new_person(&app, 1000.0).await;
    let now = now_ms();
    let hour = hour_of(now);

    // The six closest are each out of reach for one reason; the far one is the only choice.
    let recent = new_bot(&app, BotSpec::rated(1000.0)).await;
    let retired = new_bot(&app, BotSpec::rated(1001.0)).await;
    let off_schedule = new_bot(
        &app,
        BotSpec {
            rating: 1002.0,
            schedule: Some(((hour + 2).rem_euclid(24), 20)),
            decks: true,
        },
    )
    .await;
    let busy = new_bot(&app, BotSpec::rated(1003.0)).await;
    let deckless = new_bot(
        &app,
        BotSpec {
            rating: 1004.0,
            schedule: None,
            decks: false,
        },
    )
    .await;
    // R1449: decks dealt for a catalog this server no longer plays seat it in no Best of 1.
    let stale_decks = new_bot(&app, BotSpec::rated(1005.0)).await;
    let older: Vec<BotDeck> = vec![from(json!({ "cards": deck(1), "portrait": "vanilla" }))];
    q!(app, bots_set_decks(&stale_decks, &older, "an-older-catalog"));
    let far = new_bot(&app, BotSpec::rated(1150.0)).await;

    // The person played `recent` last; it is among their last BOT_RECENT_OPPONENTS games.
    let played = next_id("played");
    {
        let mut data = fake(&app).await;
        data.tables.results.push(from(json!({
            "matchId": played,
            "players": [person, recent],
            "winnerProfileId": person,
            "reason": "concede",
            "turns": 3,
            "endedAt": now - 60_000,
            "ratingBefore": [1000, 1000],
            "ratingAfter": [1000, 1000],
        })));
    }
    const { assert!(BOT_RECENT_OPPONENTS >= 1) };
    q!(app, bots_set_retired(&retired, Some(now)));
    q!(app, profiles_set_in_match(&busy, Some("another-match")));

    ticket(&app, &person, 1000.0, past_any_fallback(), "bo1").await;
    assert_eq!(try_pair(&app).await.expect("a sweep"), 1);
    assert_eq!(
        seated(
            &app,
            &[
                &recent,
                &retired,
                &off_schedule,
                &busy,
                &deckless,
                &stale_decks,
                &far
            ]
        )
        .await,
        vec![far.clone()]
    );
}

#[tokio::test(start_paused = true)]
async fn r1448_a_start_that_fails_lets_the_bot_go() {
    let app = app_with_bots(true).await;
    let (person, _) = new_person(&app, 1000.0).await;
    let bot = new_bot(&app, BotSpec::rated(1000.0)).await;
    ticket(&app, &person, 1000.0, past_any_fallback(), "bo1").await;
    // The claim put the bot in the match; the next write, the player's in-match state, fails.
    let refused = Arc::new(AtomicBool::new(false));
    let once = Arc::clone(&refused);
    fake(&app).await.on_call = Some(Arc::new(move |method: &str| {
        if method == "profiles.setInMatch" && !once.swap(true, Ordering::SeqCst) {
            return Err(StoreError::Other(format!("{method} refused by the test")));
        }
        Ok(())
    }));
    assert!(try_pair(&app).await.is_err(), "the start's failure stands");
    assert!(
        refused.load(Ordering::SeqCst),
        "the write after the claim was refused"
    );
    fake(&app).await.on_call = None;

    let bots = q!(app, bots_list());
    assert_eq!(bots.len(), 1);
    assert_eq!(bots[0].profile_id, bot);
    assert_eq!(bots[0].in_match_id, None, "the bot is free again");
    let profile = q!(app, profiles_get_by_id(&person)).expect("the person");
    assert_eq!(profile.in_match_id, None);
    assert!(matches(&app).await.is_empty(), "no match started");
}

#[tokio::test(start_paused = true)]
async fn r1448_above_the_strongest_bot_gets_none() {
    let app = app_with_bots(true).await;
    let (person, _) = new_person(&app, 1500.0).await;
    new_bot(&app, BotSpec::rated(1100.0)).await;
    new_bot(&app, BotSpec::rated(1200.0)).await;
    ticket(&app, &person, 1500.0, 10 * past_any_fallback(), "bo1").await;
    assert_eq!(try_pair(&app).await.expect("a sweep"), 0);
    assert!(matches(&app).await.is_empty());

    // A retired bot rated above the player does not count as the strongest.
    let strong = new_bot(&app, BotSpec::rated(1600.0)).await;
    q!(app, bots_set_retired(&strong, Some(now_ms())));
    assert_eq!(try_pair(&app).await.expect("a sweep"), 0);
}

#[test]
fn r1448_ladder_bots_takes_on_or_off() {
    let env_of = |value: Option<&str>| {
        let mut source: IndexMap<String, String> = IndexMap::new();
        source.insert("E2E".to_string(), "1".to_string());
        source.insert("NODE_ENV".to_string(), "test".to_string());
        if let Some(value) = value {
            source.insert("LADDER_BOTS".to_string(), value.to_string());
        }
        load_server_env(&source)
    };
    let on = |value: Option<&str>| env_of(value).expect("a valid environment").ladder_bots;
    assert!(!on(None), "off unless it is set");
    assert!(!on(Some("")));
    assert!(!on(Some("off")));
    assert!(on(Some("on")));
    let refused = format!("{:#}", env_of(Some("yes")).expect_err("only on or off"));
    assert!(
        refused.contains("LADDER_BOTS") && refused.contains("'on'") && refused.contains("'off'"),
        "{refused}"
    );
    assert!(SERVER_ONLY_ENV_VARS.contains(&"LADDER_BOTS"));
}

#[tokio::test(start_paused = true)]
async fn r1448_ladder_bots_off_seats_none() {
    let app = app_with_bots(false).await;
    assert!(!app.env.ladder_bots);
    let (person, _) = new_person(&app, 1000.0).await;
    new_bot(&app, BotSpec::rated(1000.0)).await;
    ticket(&app, &person, 1000.0, 10 * past_any_fallback(), "bo1").await;
    assert_eq!(try_pair(&app).await.expect("a sweep"), 0);
    assert!(matches(&app).await.is_empty());
}

#[tokio::test(start_paused = true)]
async fn r1450_the_bot_side_is_ai_and_the_human_side_counts() {
    let app = app_with_bots(true).await;
    let (person, _) = new_person(&app, 1000.0).await;
    let bot = new_bot(&app, BotSpec::rated(1000.0)).await;
    ticket(&app, &person, 1000.0, past_any_fallback(), "bo1").await;
    assert_eq!(try_pair(&app).await.expect("a sweep"), 1);
    let (match_id, seat) = concede_bot_match(&app, &person).await;

    let record = fake(&app)
        .await
        .tables
        .game_records
        .iter()
        .find(|record| record.id == match_id)
        .cloned()
        .expect("the bot game was recorded");
    let record = serde_json::to_value(&record).expect("a record");
    assert_eq!(record["source"], "live");
    assert_eq!(record["pilots"][seat.as_str()], "human");
    assert_eq!(record["pilots"][seat.opponent().as_str()], "ai");

    // R611: the rated game names the person's side and the bot's.
    let rated = fake(&app).await.tables.rated_games.clone();
    let rated = serde_json::to_value(&rated).expect("rated games");
    let text = rated.to_string();
    assert!(text.contains(&person) && text.contains(&bot), "{text}");
}

#[tokio::test]
async fn r1450_public_stats_skip_the_bot_side_and_the_gate_counts_the_game() {
    let app = app_with_bots(true).await;
    let patch = jackioh_cards::catalog_version().to_string();
    let seat = |deck: &[&str]| json!({ "deck": deck, "opening": [], "drawn": deck, "played": deck, "playedTurns": [1, 2] });
    let record: jackioh_engine::GameRecord = from(json!({
        "id": "bot-game-1",
        "source": "live",
        "mode": "bo1",
        "patch": patch,
        "pilots": { "p1": "human", "p2": "ai" },
        "game": {
            "first": "p1",
            "winner": "p2",
            "reason": "hero-death",
            "turns": 7,
            "seats": {
                "p1": seat(&["core-001", "core-002"]),
                "p2": seat(&["core-003", "core-004"]),
            }
        }
    }));
    q!(app, game_records_insert(&record));

    let (status, _, data) = call(&app, "GET", "/api/stats/cards", None, Value::Null).await;
    assert_eq!(status, 200);
    // The gate counts the game, the person's cards count it, the bot's do not.
    assert_eq!(data["gate"]["liveGames"], json!(1));
    let games_of = |id: &str| {
        data["cards"]
            .as_array()
            .and_then(|cards| cards.iter().find(|card| card["id"] == id))
            .map_or(json!(0), |card| card["games"].clone())
    };
    assert_eq!(games_of("core-001"), json!(1));
    assert_eq!(games_of("core-003"), json!(0));

    let (status, _, card) = call(&app, "GET", "/api/stats/cards/core-003", None, Value::Null).await;
    assert_eq!(status, 200);
    assert!(
        !card.to_string().contains("\"games\":1"),
        "the bot's card drew a game: {card}"
    );
    let (status, _, card) = call(&app, "GET", "/api/stats/cards/core-001", None, Value::Null).await;
    assert_eq!(status, 200);
    assert!(card.to_string().contains("\"games\":1"), "{card}");
}

#[tokio::test(start_paused = true)]
async fn r1447_no_read_tells_a_bot_seat() {
    let app = app_with_bots(true).await;
    let (person, token) = new_person(&app, 1000.0).await;
    let bot = new_bot(&app, BotSpec::rated(1000.0)).await;
    ticket(&app, &person, 1000.0, past_any_fallback(), "bo1").await;
    assert_eq!(try_pair(&app).await.expect("a sweep"), 1);
    let row = fake(&app).await.tables.matches[0].clone();

    let (status, _, ranks) = call(
        &app,
        "GET",
        &format!("/api/matches/{}/ranks", row.id),
        Some(&token),
        Value::Null,
    )
    .await;
    assert_eq!(status, 200, "{ranks}");
    let bot_seat = if row.players.0 == bot { "p1" } else { "p2" };
    let shown = &ranks["seats"][bot_seat];
    assert_eq!(shown["profileId"], bot.as_str());
    assert!(
        shown["username"].is_string(),
        "the bot has a username like anyone: {shown}"
    );
    assert!(
        shown["rank"]["tier"].is_string() && shown["rank"]["tier"] != "jlorious",
        "a Grape rank: {shown}"
    );
    // On its own tier's floor, as a placed player often stands: never one no player could hold.
    assert_eq!(shown["rank"]["floor"], shown["rank"]["tier"], "{shown}");

    // R672: while the game goes on the bot is on the match, as a person playing it is, though it
    // holds no socket.
    let rematch_path = format!("/api/matches/{}/rematch", row.id);
    let (status, _, rematch) = call(&app, "GET", &rematch_path, Some(&token), Value::Null).await;
    assert_eq!(status, 200, "{rematch}");
    assert_eq!(rematch["opponentHere"], json!(true), "{rematch}");

    let mut bodies = vec![ranks, rematch];
    for path in [
        "/api/ranked",
        "/api/leaderboard",
        "/api/stats/cards",
        "/api/auth/me",
    ] {
        let (status, _, body) = call(&app, "GET", path, Some(&token), Value::Null).await;
        assert_eq!(status, 200, "{path}: {body}");
        bodies.push(body);
    }
    for body in bodies {
        let text = body.to_string();
        for tell in ["\"kind\"", "\"pilot", "srv-bot", "\"bot\"", "ladderBot"] {
            assert!(!text.contains(tell), "{tell} in {text}");
        }
    }

    // Once the game ends the bot reads as gone, so the person is never offered a rematch.
    concede_bot_match(&app, &person).await;
    let (status, _, rematch) = call(&app, "GET", &rematch_path, Some(&token), Value::Null).await;
    assert_eq!(status, 200, "{rematch}");
    assert_eq!(rematch["opponentHere"], json!(false), "{rematch}");
}

#[tokio::test(start_paused = true)]
async fn r1447_a_bot_shows_mythic_grape_only_once_a_player_holds_it() {
    // Jlorious takes the first JLORIOUS_SIZE placed Mythic players (R608): with exactly that many
    // nobody holds Mythic Grape, and one more is the first who does.
    for (in_mythic, shown) in [(JLORIOUS_SIZE, "golden"), (JLORIOUS_SIZE + 1, "mythic")] {
        let app = app_with_bots(true).await;
        let season = season_id_of(jackioh_cards::catalog_version());
        let mythic = GrapeTier::Mythic as i32;
        for _ in 0..in_mythic {
            let id = next_id("mythic");
            fake(&app)
                .await
                .seed_profile(json!({ "id": id, "status": "active", "rating": 1000 }));
            let mut rank = fresh_rank(&season, &id, 0);
            rank.games = RANK_PLACEMENT_GAMES;
            rank.ladder = Some(tier_bottom(mythic));
            rank.floor = mythic;
            rank.peak_ladder = rank.ladder;
            q!(app, ranked_put_rank(&rank));
        }
        // The bot is rated above every one of them, so its rating calls for Mythic.
        let (person, token) = new_person(&app, 1000.0).await;
        let bot = new_bot(&app, BotSpec::rated(3000.0)).await;
        ticket(&app, &person, 1000.0, past_any_fallback(), "bo1").await;
        assert_eq!(try_pair(&app).await.expect("a sweep"), 1);
        let row = fake(&app).await.tables.matches[0].clone();
        let (status, _, ranks) = call(
            &app,
            "GET",
            &format!("/api/matches/{}/ranks", row.id),
            Some(&token),
            Value::Null,
        )
        .await;
        assert_eq!(status, 200, "{ranks}");
        let bot_seat = if row.players.0 == bot { "p1" } else { "p2" };
        assert_eq!(
            ranks["seats"][bot_seat]["rank"]["tier"], shown,
            "{in_mythic} in Mythic: {ranks}"
        );
    }
}
