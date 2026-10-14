//! R1453 (SPEC §9.5): the empty-queue fallback. With `LADDER_BOTS=on`, a ranked ticket that has
//! waited past its own fallback time (15–40 s, from its id) with no person to pair with is paired with
//! a ladder bot: free, on its schedule, the closest in rating, and not one the player faced in their
//! last games. A player rated above every bot never gets one, and a person inside the window always
//! pairs first.
//!
//! R641000 (SPEC §9.5): a Conquest ticket's bot holds a trio, and the two play a ranked series. The
//! bot is held by the series between games, so nothing else seats it until the series ends; it makes
//! its picks from the series sweeper after a person's delay; the series is rated once, as it ends,
//! its side as the bot it is; and a series game draws it no rematch plan.
//!
//! R1451 (SPEC §9.11): the person's side of a bot game counts toward the card statistics and R654's
//! gate like any live game; the bot's side is recorded as `ai` and read only when a query asks.
//!
//! R1452 (SPEC §9.12): nothing a client reads tells a bot's seat from a person's. R640000: the match
//! screen shows a bot's own season row, as the leaderboard does, and Jlorious is people's alone.
//!
//! R640001 (SPEC §9.5): the rating window stops at `MAX_FAIR_GAP`, so past it the fallback seats a
//! bot rather than pairing two people that far apart.
//!
//! R639000: once a ranked match ends a bot stays on it a while, as a person on the death screen
//! does, and offers or meets a rematch as its plan says, a rematch that claims it as the fallback
//! does and that it then plays.
//!
//! Everything runs through `try_pair` on tokio's paused clock, on the fake store, the real catalog and
//! the real registry (a started match is a real game). Bots are written through the store, as the pool
//! writes them (`bots_create`, `bots_set_decks`).

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::time::Duration;

use indexmap::IndexMap;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use tokio::sync::MutexGuard;

use jackioh_engine::{PlayerId, pick_portrait_from_seed};
use jackioh_server::api::queue::try_pair;
use jackioh_server::api::rematch::STAKE_NORMAL;
use jackioh_server::api::series::sweep_series;
use jackioh_server::app::{App, load_server_env, now_ms};
use jackioh_server::bots::fallback::{fallback_due, fallback_seconds, fallback_seconds_keyed, hour_of};
use jackioh_server::bots::pick::{pick_due_at, pick_slot};
use jackioh_server::bots::signals::{RematchPlan, rematch_plan};
use jackioh_server::config::{
    BOT_FALLBACK_MAX_SECONDS, BOT_FALLBACK_MIN_SECONDS, BOT_RECENT_OPPONENTS, BOT_REMATCH_DELAY_MAX_MS,
    BOT_REMATCH_FIT, BOT_REMATCH_LINGER_MAX_MS, BOT_SEED_RATINGS, JLORIOUS_SIZE, MAX_FAIR_GAP,
    QUEUE_TICKET_STALE_MS, RANK_PLACEMENT_GAMES, RATING_WINDOW_FULL_AFTER_SECONDS,
};
use jackioh_server::db::fake::FakeData;
use jackioh_server::db::store::{BotDeck, Db, FrozenTrio, NewBot, SeriesStatus, StoreError, Ticket};
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

/// R641000: deals the bot a Conquest trio for this catalog, as the pool does; answers it.
async fn give_trio(app: &App, bot: &str) -> FrozenTrio {
    let trio: FrozenTrio = from(json!({
        "name": "Conquest",
        "decks": [
            { "name": "Deck 1", "cards": deck(2), "portrait": "vanilla" },
            { "name": "Deck 2", "cards": deck(3), "portrait": "vanilla" },
            { "name": "Deck 3", "cards": deck(4), "portrait": "vanilla" },
        ],
    }));
    q!(app, bots_set_trio(bot, &trio, &app.catalog.version));
    trio
}

/// R641000: a Conquest ticket for `profile_id`, its trio frozen in, that has waited `waited_ms`.
async fn conquest_ticket(app: &App, profile_id: &str, rating: f64, waited_ms: i64) -> Ticket {
    let row: Ticket = from(json!({
        "id": next_id("ticket"),
        "profileId": profile_id,
        "rating": rating,
        "mode": "bo3",
        "deck": [],
        "portrait": null,
        "trio": {
            "name": "Mine",
            "decks": [
                { "name": "Aggro", "cards": deck(5) },
                { "name": "Control", "cards": deck(6) },
                { "name": "Tempo", "cards": deck(7) },
            ],
        },
        "catalogVersion": app.catalog.version,
        "enqueuedAt": now_ms() - waited_ms,
        "status": "open",
        "matchId": null,
    }));
    q!(app, tickets_insert(&row));
    row
}

/// `POST /api/series/:id/{action}` as `token`'s holder, answering the series view.
async fn series_post(app: &Arc<App>, series_id: &str, action: &str, token: &str, body: Value) -> Value {
    let (status, _, view) = call(
        app,
        "POST",
        &format!("/api/series/{series_id}/{action}"),
        Some(token),
        body,
    )
    .await;
    assert_eq!(status, 200, "{action}: {view}");
    view
}

/// R641000: a person and a bot paired into a Conquest series by the fallback, and game 1 begun: the
/// person picks their first deck, and the sweeper makes the bot's pick once its delay has passed.
/// Answers `(person, their token, bot, series id)`.
async fn series_in_game_1(app: &Arc<App>) -> (String, String, String, String) {
    let (person, token) = new_person(app, 1000.0).await;
    let bot = new_bot(app, BotSpec::rated(1000.0)).await;
    give_trio(app, &bot).await;
    conquest_ticket(app, &person, 1000.0, past_any_fallback()).await;
    assert_eq!(try_pair(app).await.expect("a sweep").made, 1);
    let opened = fake(app).await.tables.series[0].clone();
    series_post(app, &opened.id, "pick", &token, json!({ "slot": 0, "gameNo": 1 })).await;
    let due = pick_due_at(&opened, PlayerId::P2).expect("a pick phase");
    tokio::time::advance(Duration::from_millis(
        u64::try_from(due - now_ms()).expect("a wait"),
    ))
    .await;
    let swept = sweep_series(app).await.expect("a sweep");
    assert_eq!(swept.bot_picked, vec![opened.id.clone()]);
    (person, token, bot, opened.id)
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

/// Lets `ms` pass on the paused clock, then every task woken by then run.
async fn pass(ms: i64) {
    tokio::time::sleep(Duration::from_millis(u64::try_from(ms.max(0)).unwrap_or(0))).await;
    for _ in 0..50 {
        tokio::task::yield_now().await;
    }
}

/// R639000: a seed whose bot, in p2, draws a rematch plan `wanted` holds.
fn seed_where(wanted: impl Fn(&RematchPlan) -> bool) -> String {
    (0..10_000)
        .map(|k| format!("seed-rematch-{k}"))
        .find(|seed| wanted(&rematch_plan(seed, PlayerId::P2, &BOT_REMATCH_FIT)))
        .expect("a seed whose bot plans so")
}

/// R639000: a ranked Best of 1 between `person` (p1) and `bot` (p2) on `seed`, started as the
/// fallback starts one but with the seed chosen. Answers its id.
async fn bot_match_on(app: &Arc<App>, person: &str, bot: &str, seed: &str) -> String {
    let match_id = next_id("match");
    app.matches
        .start(
            app,
            from(json!({
                "matchId": match_id,
                "seed": seed,
                "catalogVersion": app.catalog.version,
                "ranked": true,
                "seats": [
                    { "profileId": person, "player": "p1", "deck": deck(0) },
                    { "profileId": bot, "player": "p2", "deck": deck(1) },
                ],
            })),
        )
        .await
        .expect("the registry starts the match");
    match_id
}

/// `GET /api/matches/:matchId/rematch`, as `token`'s holder.
async fn rematch_status(app: &Arc<App>, match_id: &str, token: &str) -> Value {
    let (status, _, body) = call(
        app,
        "GET",
        &format!("/api/matches/{match_id}/rematch"),
        Some(token),
        Value::Null,
    )
    .await;
    assert_eq!(status, 200, "{body}");
    body
}

/// `POST /api/matches/:matchId/rematch` at normal stakes, as `token`'s holder.
async fn offer_rematch(app: &Arc<App>, match_id: &str, token: &str) -> Value {
    let (status, _, body) = call(
        app,
        "POST",
        &format!("/api/matches/{match_id}/rematch"),
        Some(token),
        json!({ "stakes": STAKE_NORMAL }),
    )
    .await;
    assert_eq!(status, 200, "{body}");
    body
}

#[test]
fn r1453_fallback_time_is_15_to_40_s_and_stable() {
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
fn r1453_no_client_can_work_out_the_fallback_time() {
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
async fn r1453_no_bot_before_the_fallback_time() {
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

    assert_eq!(try_pair(&app).await.expect("a sweep").made, 0);
    assert!(matches(&app).await.is_empty());

    // Once it is due, the next sweep pairs it.
    tokio::time::advance(std::time::Duration::from_millis(1_000)).await;
    assert!(fallback_due(&early, now_ms()));
    assert_eq!(try_pair(&app).await.expect("a sweep").made, 1);
    assert_eq!(seated(&app, &[&bot]).await, vec![bot.clone()]);
}

#[tokio::test(start_paused = true)]
async fn r897000_a_stale_ticket_seats_no_ladder_bot() {
    let app = app_with_bots(true).await;
    let (person, _) = new_person(&app, 1000.0).await;
    let bot = new_bot(&app, BotSpec::rated(1000.0)).await;
    let waiting = ticket(&app, &person, 1000.0, past_any_fallback(), "bo1").await;
    // PREMISE: past its fallback time, so without presence the bot would seat it at once.
    assert!(fallback_due(&waiting, now_ms()));
    // Unheard from for a full window: the sweep cancels it before the fallback runs.
    tokio::time::advance(Duration::from_millis((QUEUE_TICKET_STALE_MS + 1) as u64)).await;

    assert_eq!(try_pair(&app).await.expect("a sweep").made, 0);
    let held = q!(app, tickets_get(&waiting.id)).expect("the ticket");
    assert_eq!(
        serde_json::to_value(&held).expect("a ticket")["status"],
        "cancelled"
    );
    assert!(matches(&app).await.is_empty());
    assert!(seated(&app, &[&bot]).await.is_empty());
}

#[tokio::test(start_paused = true)]
async fn r897000_a_touched_ticket_still_meets_its_bot_past_the_fallback() {
    let app = app_with_bots(true).await;
    let (person, _) = new_person(&app, 1000.0).await;
    let bot = new_bot(&app, BotSpec::rated(1000.0)).await;
    let waiting = ticket(&app, &person, 1000.0, past_any_fallback(), "bo1").await;
    assert!(fallback_due(&waiting, now_ms()));
    // Heard from now: live, though fallback-due and older than one window.
    tokio::time::advance(Duration::from_millis((QUEUE_TICKET_STALE_MS + 1) as u64)).await;
    app.queue_presence.touch(&person, now_ms());

    assert_eq!(try_pair(&app).await.expect("a sweep").made, 1);
    assert_eq!(seated(&app, &[&bot]).await, vec![bot.clone()]);
}

#[tokio::test(start_paused = true)]
async fn r1453_the_closest_free_scheduled_bot_pairs() {
    let app = app_with_bots(true).await;
    let (person, _) = new_person(&app, 1000.0).await;
    let far_below = new_bot(&app, BotSpec::rated(900.0)).await;
    let closest = new_bot(&app, BotSpec::rated(1050.0)).await;
    let far_above = new_bot(&app, BotSpec::rated(1300.0)).await;
    let waiting = ticket(&app, &person, 1000.0, past_any_fallback(), "bo1").await;

    assert_eq!(try_pair(&app).await.expect("a sweep").made, 1);
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
async fn r1453_the_player_is_player_1_as_the_older_ticket_is() {
    // Between two people the older ticket is p1 and goes first; a ticket past its fallback time is
    // older than anyone who would come to pair with it, so a bot match seats the person p1 every time.
    // A coin for the seats would put the person second about half of these.
    let app = app_with_bots(true).await;
    for round in 0..8 {
        let (person, _) = new_person(&app, 1000.0).await;
        let bot = new_bot(&app, BotSpec::rated(1000.0)).await;
        let mode = if round % 2 == 0 { "bo1" } else { "random" };
        ticket(&app, &person, 1000.0, past_any_fallback(), mode).await;
        assert_eq!(try_pair(&app).await.expect("a sweep").made, 1, "round {round}");
        let row = fake(&app).await.tables.matches.last().cloned().expect("a match");
        assert_eq!(row.players, (person, bot), "round {round} ({mode})");
    }
}

#[tokio::test(start_paused = true)]
async fn r1453_a_human_inside_the_window_pairs_first() {
    let app = app_with_bots(true).await;
    let (a, _) = new_person(&app, 1000.0).await;
    let (b, _) = new_person(&app, 1010.0).await;
    let bot = new_bot(&app, BotSpec::rated(1000.0)).await;
    ticket(&app, &a, 1000.0, past_any_fallback(), "bo1").await;
    ticket(&app, &b, 1010.0, past_any_fallback(), "bo1").await;

    assert_eq!(try_pair(&app).await.expect("a sweep").made, 1);
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
async fn r641000_a_conquest_ticket_pairs_with_a_bot_holding_a_trio() {
    let app = app_with_bots(true).await;
    let (person, _) = new_person(&app, 1000.0).await;
    let bot = new_bot(&app, BotSpec::rated(1000.0)).await;
    let conquest = conquest_ticket(&app, &person, 1000.0, past_any_fallback()).await;
    assert!(
        fallback_due(&conquest, now_ms()),
        "a Conquest ticket falls back too"
    );
    // A bot with no trio dealt for this catalog plays no Conquest.
    assert_eq!(try_pair(&app).await.expect("a sweep").made, 0);
    assert!(fake(&app).await.tables.series.is_empty());

    let trio = give_trio(&app, &bot).await;
    assert_eq!(try_pair(&app).await.expect("a sweep").made, 1);
    let made = fake(&app).await.tables.series.clone();
    assert_eq!(made.len(), 1);
    let series = &made[0];
    // The person is series p1, as the older ticket of a pair is (R335), with the trio they queued;
    // the bot p2, with its own. It is ranked (R604) and opens on game 1's pick phase.
    assert_eq!(series.sides.0.profile_id, person);
    assert_eq!(Some(&series.sides.0.trio), conquest.trio.as_ref());
    assert_eq!(series.sides.1.profile_id, bot);
    assert_eq!(series.sides.1.trio, trio);
    assert_eq!(series.ranked, Some(true));
    assert_eq!(series.status, SeriesStatus::Picking);
    assert!(series.games.is_empty());
    assert!(
        matches(&app).await.is_empty(),
        "no game starts before both have picked"
    );
    // The ticket is matched to game 1's reserved id, and neither side is in a match yet.
    let held = q!(app, tickets_get(&conquest.id)).expect("the ticket");
    assert_eq!(
        serde_json::to_value(&held).expect("a ticket")["status"],
        "matched"
    );
    assert_eq!(held.match_id.as_deref(), Some(series.next_match_id.as_str()));
    for id in [&person, &bot] {
        let profile = q!(app, profiles_get_by_id(id)).expect("the profile");
        assert_eq!(profile.in_match_id, None, "{id} is in no match");
    }

    // All Random gets one too, the bot in the series aside.
    let free = new_bot(&app, BotSpec::rated(1000.0)).await;
    let (other, _) = new_person(&app, 1000.0).await;
    ticket(&app, &other, 1000.0, past_any_fallback(), "random").await;
    assert_eq!(try_pair(&app).await.expect("a sweep").made, 1);
    assert_eq!(seated(&app, &[&bot, &free]).await, vec![free.clone()]);
}

#[tokio::test(start_paused = true)]
async fn r641000_a_bot_in_a_series_is_seated_nowhere_else_until_it_ends() {
    let app = app_with_bots(true).await;
    let (person, token) = new_person(&app, 1000.0).await;
    let bot = new_bot(&app, BotSpec::rated(1000.0)).await;
    give_trio(&app, &bot).await;
    conquest_ticket(&app, &person, 1000.0, past_any_fallback()).await;
    assert_eq!(try_pair(&app).await.expect("a sweep").made, 1);
    let series_id = fake(&app).await.tables.series[0].id.clone();

    // Between games the bot is in no match, but the series holds it: a Best-of-1 ticket past its
    // fallback time, with that bot the only one, waits on.
    let (other, _) = new_person(&app, 1000.0).await;
    ticket(&app, &other, 1000.0, past_any_fallback(), "bo1").await;
    assert_eq!(try_pair(&app).await.expect("a sweep").made, 0);
    assert!(matches(&app).await.is_empty());

    // The person forfeits between games, the series is over, and the bot is free again.
    series_post(&app, &series_id, "forfeit", &token, json!({})).await;
    assert_eq!(try_pair(&app).await.expect("a sweep").made, 1);
    assert_eq!(matches(&app).await, vec![(other, bot)]);
}

#[tokio::test(start_paused = true)]
async fn r641000_the_sweeper_makes_the_bot_s_pick_after_its_delay() {
    let app = app_with_bots(true).await;
    let (person, token) = new_person(&app, 1000.0).await;
    let bot = new_bot(&app, BotSpec::rated(1000.0)).await;
    let trio = give_trio(&app, &bot).await;
    conquest_ticket(&app, &person, 1000.0, past_any_fallback()).await;
    assert_eq!(try_pair(&app).await.expect("a sweep").made, 1);
    let opened = fake(&app).await.tables.series[0].clone();

    // The person picks through the route; the bot's pick is not in yet.
    let view = series_post(
        &app,
        &opened.id,
        "pick",
        &token,
        json!({ "slot": 0, "gameNo": 1 }),
    )
    .await;
    assert_eq!(view["opponent"]["picked"], json!(false), "{view}");
    let due = pick_due_at(&opened, PlayerId::P2).expect("a pick phase");
    let wanted = pick_slot(&opened, PlayerId::P2, &app.catalog.defs).expect("a pick");

    // A sweep a moment before its delay ends picks nothing.
    tokio::time::advance(Duration::from_millis(
        u64::try_from(due - 1 - now_ms()).expect("a wait"),
    ))
    .await;
    let swept = sweep_series(&app).await.expect("a sweep");
    assert!(swept.bot_picked.is_empty(), "{swept:?}");
    let waiting = q!(app, series_get(&opened.id)).expect("the series");
    assert_eq!(waiting.status, SeriesStatus::Picking);
    assert_eq!(waiting.sides.1.pick, None);

    // The sweep at its delay makes it, and the pick that completes both begins game 1.
    tokio::time::advance(Duration::from_millis(1)).await;
    let swept = sweep_series(&app).await.expect("a sweep");
    assert_eq!(swept.bot_picked, vec![opened.id.clone()]);
    let begun = q!(app, series_get(&opened.id)).expect("the series");
    assert_eq!(begun.status, SeriesStatus::Playing);
    assert_eq!(begun.games[0].slots, (0, i64::try_from(wanted).expect("a slot")));
    // Game 1 is started: the person, series p1, goes first (R335); the bot plays its picked deck.
    let row = fake(&app)
        .await
        .tables
        .matches
        .last()
        .cloned()
        .expect("game 1 started");
    assert_eq!(row.id, begun.next_match_id);
    assert_eq!(row.players, (person, bot));
    let decks = [&trio.decks.0, &trio.decks.1, &trio.decks.2];
    assert_eq!(row.decks.1, decks[wanted].cards);
    assert_eq!(row.ranked, Some(true));
}

#[tokio::test(start_paused = true)]
async fn r641000_a_ranked_series_against_a_bot_is_rated_once_when_it_ends() {
    let app = app_with_bots(true).await;
    let (person, token, bot, series_id) = series_in_game_1(&app).await;
    let person_before = q!(app, profiles_get_by_id(&person)).expect("the person").rating;
    let bot_before = q!(app, ranked_bot(&bot)).expect("the bot's rating");

    // The person concedes game 1: it is recorded and rates nothing (R262).
    let (game_1, _) = concede_bot_match(&app, &person).await;
    assert!(
        fake(&app).await.tables.rated_games.is_empty(),
        "a series game is not rated"
    );
    let result = q!(app, results_get_by_match(&game_1)).expect("game 1's result");
    assert_eq!(result.rating_before, result.rating_after);
    assert_eq!(
        q!(app, profiles_get_by_id(&person)).expect("the person").rating,
        person_before
    );
    assert_eq!(q!(app, ranked_bot(&bot)), Some(bot_before.clone()));
    let between = q!(app, series_get(&series_id)).expect("the series");
    assert_eq!(between.status, SeriesStatus::Picking, "game 2's pick phase");
    assert_eq!(between.sides.1.wins, 1);

    // The person forfeits between games: the series is rated once, as one game (R262, R604), the
    // bot's side as the bot it is (R610, R611).
    series_post(&app, &series_id, "forfeit", &token, json!({})).await;
    let rated = fake(&app).await.tables.rated_games.clone();
    assert_eq!(rated.len(), 1, "one rated game for the series");
    let game = serde_json::to_value(&rated[0]).expect("a rated game");
    assert_eq!(game["kind"], "series", "{game}");
    assert_eq!(game["id"], series_id.as_str(), "{game}");
    assert_eq!(game["sides"][0]["profileId"], person.as_str(), "{game}");
    assert_eq!(game["sides"][0]["pilot"], "human", "{game}");
    assert_eq!(game["sides"][1]["botId"], bot.as_str(), "{game}");
    assert_eq!(game["sides"][1]["pilot"], "ai", "{game}");
    let bot_after = q!(app, ranked_bot(&bot)).expect("the bot's rating");
    assert_eq!(bot_after.games, bot_before.games + 1);
    assert!(
        bot_after.glicko.rating > bot_before.glicko.rating,
        "the bot took the series"
    );
    assert!(
        q!(app, profiles_get_by_id(&person)).expect("the person").rating < person_before,
        "the person lost it"
    );
    // R640000: the series moved the bot's season row, as it moved the person's.
    let season = season_id_of(jackioh_cards::catalog_version());
    for id in [&person, &bot] {
        let rank = q!(app, ranked_rank(&season, id)).expect("a season row");
        assert_eq!(rank.games, 1, "{id}");
    }
}

#[tokio::test(start_paused = true)]
async fn r641000_a_series_game_s_bot_draws_no_rematch_plan() {
    let app = app_with_bots(true).await;
    let (person, _, _, _) = series_in_game_1(&app).await;
    let game_1 = fake(&app)
        .await
        .tables
        .matches
        .last()
        .cloned()
        .expect("game 1 started");
    let actor = app
        .matches
        .actor_for(&app, &game_1.id)
        .await
        .expect("game 1's actor");
    assert!(
        actor.presence()[PlayerId::P2],
        "the bot is on its game while it goes on"
    );

    // A series game offers no rematch (R672), so the bot stays for none: the series' next pick
    // phase is what comes next.
    concede_bot_match(&app, &person).await;
    assert!(
        !actor.presence()[PlayerId::P2],
        "the bot lingers on a series game"
    );
}

#[tokio::test(start_paused = true)]
async fn r1453_recent_retired_off_schedule_and_busy_bots_are_skipped() {
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
    // R1454: decks dealt for a catalog this server no longer plays seat it in no Best of 1.
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
    assert_eq!(try_pair(&app).await.expect("a sweep").made, 1);
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
async fn r1453_a_start_that_fails_lets_the_bot_go() {
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
    let report = try_pair(&app).await.expect("a sweep carries on");
    assert_eq!(report.made, 0);
    assert_eq!(report.failed.len(), 1);
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
async fn r1453_above_the_strongest_bot_gets_none() {
    let app = app_with_bots(true).await;
    let (person, _) = new_person(&app, 1500.0).await;
    new_bot(&app, BotSpec::rated(1100.0)).await;
    new_bot(&app, BotSpec::rated(1200.0)).await;
    ticket(&app, &person, 1500.0, 10 * past_any_fallback(), "bo1").await;
    assert_eq!(try_pair(&app).await.expect("a sweep").made, 0);
    assert!(matches(&app).await.is_empty());

    // A retired bot rated above the player does not count as the strongest.
    let strong = new_bot(&app, BotSpec::rated(1600.0)).await;
    q!(app, bots_set_retired(&strong, Some(now_ms())));
    assert_eq!(try_pair(&app).await.expect("a sweep").made, 0);
}

/// R748000: a pool holding one fresh bot at each seed rating, as the pool seeds it; answers the
/// profile ids in `BOT_SEED_RATINGS`' order.
async fn seeded_pool(app: &App) -> Vec<String> {
    let mut ids = Vec::new();
    for seed in BOT_SEED_RATINGS {
        ids.push(new_bot(app, BotSpec::rated(seed)).await);
    }
    ids
}

/// R748000, R1453: the ceiling gate. With a pool seeded from `BOT_SEED_RATINGS` the strongest bot
/// is the highest seed: a person rated above it is never given a bot, and one rated at it is given
/// that one.
#[tokio::test(start_paused = true)]
async fn r748000_the_strongest_seed_is_the_ceiling() {
    let top = BOT_SEED_RATINGS.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let strongest = BOT_SEED_RATINGS
        .iter()
        .position(|seed| *seed == top)
        .expect("a highest seed");

    let app = app_with_bots(true).await;
    seeded_pool(&app).await;
    let (above, _) = new_person(&app, top + 1.0).await;
    ticket(&app, &above, top + 1.0, 10 * past_any_fallback(), "bo1").await;
    assert_eq!(try_pair(&app).await.expect("a sweep").made, 0);
    assert!(matches(&app).await.is_empty(), "no match started");

    let app = app_with_bots(true).await;
    let pool = seeded_pool(&app).await;
    let (level, _) = new_person(&app, top).await;
    ticket(&app, &level, top, 10 * past_any_fallback(), "bo1").await;
    assert_eq!(try_pair(&app).await.expect("a sweep").made, 1);
    let ids: Vec<&str> = pool.iter().map(String::as_str).collect();
    assert_eq!(seated(&app, &ids).await, vec![pool[strongest].clone()]);
}

#[test]
fn r1453_ladder_bots_takes_on_or_off() {
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
async fn r1453_ladder_bots_off_seats_none() {
    let app = app_with_bots(false).await;
    assert!(!app.env.ladder_bots);
    let (person, _) = new_person(&app, 1000.0).await;
    new_bot(&app, BotSpec::rated(1000.0)).await;
    ticket(&app, &person, 1000.0, 10 * past_any_fallback(), "bo1").await;
    assert_eq!(try_pair(&app).await.expect("a sweep").made, 0);
    assert!(matches(&app).await.is_empty());
}

#[tokio::test(start_paused = true)]
async fn r1451_the_bot_side_is_ai_and_the_human_side_counts() {
    let app = app_with_bots(true).await;
    let (person, _) = new_person(&app, 1000.0).await;
    let bot = new_bot(&app, BotSpec::rated(1000.0)).await;
    ticket(&app, &person, 1000.0, past_any_fallback(), "bo1").await;
    assert_eq!(try_pair(&app).await.expect("a sweep").made, 1);
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
async fn r807000_public_stats_split_human_bot_and_merged_and_display_merged() {
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

    let games_of = |data: &Value, id: &str| {
        data["cards"]
            .as_array()
            .and_then(|cards| cards.iter().find(|card| card["id"] == id))
            .map_or(json!(0), |card| card["games"].clone())
    };
    // The gate counts the game, and the display counts every counted seat, human and bot alike.
    let (status, _, data) = call(&app, "GET", "/api/stats/cards", None, Value::Null).await;
    assert_eq!(status, 200);
    assert_eq!(data["gate"]["liveGames"], json!(1));
    assert_eq!(data["pilot"], json!("merged"));
    assert_eq!(games_of(&data, "core-001"), json!(1));
    assert_eq!(games_of(&data, "core-003"), json!(1));

    // `?pilot=human` reads the people's sides only.
    let (status, _, data) = call(&app, "GET", "/api/stats/cards?pilot=human", None, Value::Null).await;
    assert_eq!(status, 200);
    assert_eq!(data["pilot"], json!("human"));
    assert_eq!(games_of(&data, "core-001"), json!(1));
    assert_eq!(games_of(&data, "core-003"), json!(0));

    // `?pilot=ai` reads the bots' sides only.
    let (status, _, data) = call(&app, "GET", "/api/stats/cards?pilot=ai", None, Value::Null).await;
    assert_eq!(status, 200);
    assert_eq!(data["pilot"], json!("ai"));
    assert_eq!(games_of(&data, "core-001"), json!(0));
    assert_eq!(games_of(&data, "core-003"), json!(1));

    let (status, _, card) = call(&app, "GET", "/api/stats/cards/core-003", None, Value::Null).await;
    assert_eq!(status, 200);
    assert!(card.to_string().contains("\"games\":1"), "{card}");
    let (status, _, card) = call(
        &app,
        "GET",
        "/api/stats/cards/core-003?pilot=human",
        None,
        Value::Null,
    )
    .await;
    assert_eq!(status, 200);
    assert!(
        !card.to_string().contains("\"games\":1"),
        "the bot's card drew a human-side game: {card}"
    );
}

#[tokio::test(start_paused = true)]
async fn r1452_no_read_tells_a_bot_seat() {
    let app = app_with_bots(true).await;
    let (person, token) = new_person(&app, 1000.0).await;
    let bot = new_bot(&app, BotSpec::rated(1000.0)).await;
    ticket(&app, &person, 1000.0, past_any_fallback(), "bo1").await;
    assert_eq!(try_pair(&app).await.expect("a sweep").made, 1);
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
    let (bot_seat, person_seat) = if row.players.0 == bot {
        ("p1", "p2")
    } else {
        ("p2", "p1")
    };
    let shown = &ranks["seats"][bot_seat];
    assert_eq!(shown["profileId"], bot.as_str());
    assert!(
        shown["username"].is_string(),
        "the bot has a username like anyone: {shown}"
    );
    // R640000: its own season row, read as a person's: neither has played this season, so both are
    // Raisins with no placements played.
    assert_eq!(shown["rank"], ranks["seats"][person_seat]["rank"], "{ranks}");

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
        // R807000: the stats reads name their merged split, which is the same whatever the seat
        // is; a split value here would be the tell, so those stay banned.
        for tell in [
            "\"kind\"",
            "\"pilot\":\"ai\"",
            "\"pilot\":\"human\"",
            "srv-bot",
            "\"bot\"",
            "ladderBot",
        ] {
            assert!(!text.contains(tell), "{tell} in {text}");
        }
    }

    // R639000: once the game ends the bot stays on the match a while, as a person on the death
    // screen does, then reads as gone.
    concede_bot_match(&app, &person).await;
    let (status, _, rematch) = call(&app, "GET", &rematch_path, Some(&token), Value::Null).await;
    assert_eq!(status, 200, "{rematch}");
    assert_eq!(rematch["opponentHere"], json!(true), "{rematch}");
    pass(BOT_REMATCH_LINGER_MAX_MS + 1).await;
    let (status, _, rematch) = call(&app, "GET", &rematch_path, Some(&token), Value::Null).await;
    assert_eq!(status, 200, "{rematch}");
    assert_eq!(rematch["opponentHere"], json!(false), "{rematch}");
}

#[tokio::test(start_paused = true)]
async fn r640000_a_bot_in_mythic_shows_mythic_grape_while_jlorious_has_room() {
    // Jlorious is people's (R640000): a bot rated above all of them, with its own Mythic row, is
    // Mythic Grape while Jlorious still has room, and Jlorious lists the people alone.
    const PEOPLE_IN_MYTHIC: usize = 3;
    const { assert!(PEOPLE_IN_MYTHIC < JLORIOUS_SIZE) };
    let app = app_with_bots(true).await;
    let season = season_id_of(jackioh_cards::catalog_version());
    let mythic = GrapeTier::Mythic as i32;
    let mythic_row = |id: &str| {
        let mut rank = fresh_rank(&season, id, 0);
        rank.games = RANK_PLACEMENT_GAMES;
        rank.ladder = Some(tier_bottom(mythic));
        rank.floor = mythic;
        rank.peak_ladder = rank.ladder;
        rank
    };
    let mut people = Vec::new();
    for _ in 0..PEOPLE_IN_MYTHIC {
        let id = next_id("mythic");
        fake(&app)
            .await
            .seed_profile(json!({ "id": id, "status": "active", "rating": 1000 }));
        q!(app, ranked_put_rank(&mythic_row(&id)));
        people.push(json!(id));
    }
    let (person, token) = new_person(&app, 1000.0).await;
    let bot = new_bot(&app, BotSpec::rated(3000.0)).await;
    q!(app, ranked_put_rank(&mythic_row(&bot)));
    ticket(&app, &person, 1000.0, past_any_fallback(), "bo1").await;
    assert_eq!(try_pair(&app).await.expect("a sweep").made, 1);
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
    assert_eq!(ranks["seats"][bot_seat]["rank"]["tier"], "mythic", "{ranks}");

    let (status, _, listed) = call(&app, "GET", "/api/leaderboard", Some(&token), Value::Null).await;
    assert_eq!(status, 200, "{listed}");
    let jlorious: Vec<Value> = listed["jlorious"]
        .as_array()
        .expect("a list")
        .iter()
        .map(|place| place["profileId"].clone())
        .collect();
    assert_eq!(jlorious, people, "{listed}");
}

#[tokio::test(start_paused = true)]
async fn r640001_past_the_gap_a_far_person_is_not_paired_and_the_near_one_gets_a_bot() {
    // Two people 500 apart who have both waited 120 s: once the window went uncapped at 60 s they
    // paired; it stops at MAX_FAIR_GAP now, so the one a bot is near gets the bot and the other
    // (rated above every bot, R1453) waits on.
    const WAITED_MS: i64 = 120 * MS_PER_SECOND;
    const {
        assert!(1500 - 1000 > MAX_FAIR_GAP);
        assert!(WAITED_MS > RATING_WINDOW_FULL_AFTER_SECONDS * MS_PER_SECOND);
        assert!(WAITED_MS > (BOT_FALLBACK_MAX_SECONDS + 1) * MS_PER_SECOND);
    };
    let app = app_with_bots(true).await;
    let (near, _) = new_person(&app, 1000.0).await;
    let (far, _) = new_person(&app, 1500.0).await;
    let bot = new_bot(&app, BotSpec::rated(1000.0)).await;
    ticket(&app, &near, 1000.0, WAITED_MS, "bo1").await;
    let waiting = ticket(&app, &far, 1500.0, WAITED_MS, "bo1").await;

    assert_eq!(try_pair(&app).await.expect("a sweep").made, 1);
    assert_eq!(matches(&app).await, vec![(near, bot)]);
    let held = q!(app, tickets_get(&waiting.id)).expect("the ticket");
    assert_eq!(serde_json::to_value(&held).expect("a ticket")["status"], "open");
}

#[tokio::test(start_paused = true)]
async fn r639000_a_bot_accepts_a_rematch_when_its_plan_says_and_plays_it() {
    let app = app_with_bots(true).await;
    let (person, token) = new_person(&app, 1000.0).await;
    let bot = new_bot(&app, BotSpec::rated(1000.0)).await;
    let seed = seed_where(|plan| plan.accepts && !plan.offers);
    let plan = rematch_plan(&seed, PlayerId::P2, &BOT_REMATCH_FIT);
    let finished = bot_match_on(&app, &person, &bot, &seed).await;
    concede_bot_match(&app, &person).await;

    // The person offers while the bot is still on the match.
    let offered = offer_rematch(&app, &finished, &token).await;
    assert!(offered["matchId"].is_null(), "{offered}");
    // The bot meets it a person's reply later, not at once.
    pass(plan.reply_ms - 1).await;
    let waiting = rematch_status(&app, &finished, &token).await;
    assert!(waiting["matchId"].is_null(), "{waiting}");
    assert!(waiting["opponentOffer"].is_null(), "{waiting}");
    pass(1).await;
    let made = rematch_status(&app, &finished, &token).await;
    let rematch = made["matchId"]
        .as_str()
        .expect("the rematch was made")
        .to_string();
    assert_eq!(made["opponentOffer"], json!(STAKE_NORMAL), "{made}");

    // The rematch seats the same two in the same seats, ranked as the fallback's match was, and the
    // bot is claimed for it.
    let row = fake(&app)
        .await
        .tables
        .matches
        .iter()
        .find(|row| row.id == rematch)
        .cloned()
        .expect("the rematch's row");
    assert_eq!(row.players, (person.clone(), bot.clone()));
    assert_eq!(row.ranked, Some(true));
    let bots = q!(app, bots_list());
    assert_eq!(bots[0].in_match_id.as_deref(), Some(rematch.as_str()));
    // And it plays it: its mulligan lands in the rematch's log as a bot's server action.
    let actor = app
        .matches
        .actor_for(&app, &rematch)
        .await
        .expect("the rematch's actor");
    let mut mulliganed = false;
    for _ in 0..200 {
        actor.idle().await;
        let log = q!(app, matches_actions(&rematch));
        if log.iter().any(|row| {
            row.action.player_id == PlayerId::P2
                && row.action.nonce.starts_with("srv-bot-")
                && serde_json::to_value(&row.action).is_ok_and(|action| action["type"] == "mulligan")
        }) {
            mulliganed = true;
            break;
        }
        pass(250).await;
    }
    assert!(mulliganed, "the bot never moved in its rematch");
}

#[tokio::test(start_paused = true)]
async fn r639000_a_bot_that_declines_lets_the_offer_lapse() {
    let app = app_with_bots(true).await;
    let (person, token) = new_person(&app, 1000.0).await;
    let bot = new_bot(&app, BotSpec::rated(1000.0)).await;
    let seed = seed_where(|plan| !plan.accepts && !plan.offers);
    let finished = bot_match_on(&app, &person, &bot, &seed).await;
    concede_bot_match(&app, &person).await;

    offer_rematch(&app, &finished, &token).await;
    pass(BOT_REMATCH_DELAY_MAX_MS + 1).await;
    let status = rematch_status(&app, &finished, &token).await;
    assert_eq!(status["youOffered"], json!(STAKE_NORMAL), "{status}");
    assert!(status["opponentOffer"].is_null(), "{status}");
    assert!(status["matchId"].is_null(), "{status}");
    // Then it leaves, as a person who did not want another game does.
    pass(BOT_REMATCH_LINGER_MAX_MS).await;
    let status = rematch_status(&app, &finished, &token).await;
    assert_eq!(status["opponentHere"], json!(false), "{status}");
    assert!(status["matchId"].is_null(), "{status}");
    assert_eq!(fake(&app).await.tables.matches.len(), 1, "no rematch was made");

    // A bot whose plan accepts meets no offer that comes once it has left.
    let (late, late_token) = new_person(&app, 1000.0).await;
    let seed = seed_where(|plan| plan.accepts && !plan.offers);
    let finished = bot_match_on(&app, &late, &bot, &seed).await;
    concede_bot_match(&app, &late).await;
    pass(BOT_REMATCH_LINGER_MAX_MS + 1).await;
    offer_rematch(&app, &finished, &late_token).await;
    pass(BOT_REMATCH_DELAY_MAX_MS + 1).await;
    let status = rematch_status(&app, &finished, &late_token).await;
    assert!(status["opponentOffer"].is_null(), "{status}");
    assert!(status["matchId"].is_null(), "{status}");
    assert_eq!(fake(&app).await.tables.matches.len(), 2, "no rematch was made");
}

#[tokio::test(start_paused = true)]
async fn r639000_a_bot_offers_a_rematch_when_its_plan_says() {
    let app = app_with_bots(true).await;
    let (person, token) = new_person(&app, 1000.0).await;
    let bot = new_bot(&app, BotSpec::rated(1000.0)).await;
    let seed = seed_where(|plan| plan.offers);
    let plan = rematch_plan(&seed, PlayerId::P2, &BOT_REMATCH_FIT);
    let finished = bot_match_on(&app, &person, &bot, &seed).await;
    concede_bot_match(&app, &person).await;

    // Its offer comes a person's beat after the end, at normal stakes, while it is on the match.
    pass(plan.offer_after_ms - 1).await;
    let before = rematch_status(&app, &finished, &token).await;
    assert!(before["opponentOffer"].is_null(), "{before}");
    pass(1).await;
    let offered = rematch_status(&app, &finished, &token).await;
    assert_eq!(offered["opponentOffer"], json!(STAKE_NORMAL), "{offered}");
    assert_eq!(offered["opponentHere"], json!(true), "{offered}");
    // R1442: recorded among the match's signals, as a person's offer is.
    let signals = serde_json::to_value(&fake(&app).await.tables.match_signals).expect("the signals");
    let bot_row = signals
        .as_array()
        .and_then(|rows| {
            rows.iter()
                .find(|row| row["matchId"] == finished.as_str() && row["seat"] == "p2")
        })
        .cloned()
        .expect("the bot's signals row");
    assert_eq!(bot_row["rematchOffered"], json!(true), "{bot_row}");

    // The person meets it: the rematch is made at once.
    let made = offer_rematch(&app, &finished, &token).await;
    assert!(made["matchId"].is_string(), "{made}");
}

#[tokio::test(start_paused = true)]
async fn r639000_a_bot_stays_on_the_match_while_its_result_is_written() {
    // A person's socket stays open from the end of the game on, so a bot that read as gone while
    // the result is written, then as present for its linger, would flicker where a person never does.
    let app = app_with_bots(true).await;
    let (person, token) = new_person(&app, 1000.0).await;
    let bot = new_bot(&app, BotSpec::rated(1000.0)).await;
    let finished = bot_match_on(&app, &person, &bot, "seed-present").await;
    let actor = app
        .matches
        .actor_for(&app, &finished)
        .await
        .expect("the match's actor");
    let seen = Arc::new(std::sync::Mutex::new(Vec::new()));
    {
        let (actor, seen) = (actor.clone(), Arc::clone(&seen));
        fake(&app).await.on_call = Some(Arc::new(move |method: &str| {
            if method == "results.insert" {
                seen.lock()
                    .expect("the presence seen")
                    .push(actor.presence()[PlayerId::P2]);
            }
            Ok(())
        }));
    }
    concede_bot_match(&app, &person).await;
    fake(&app).await.on_call = None;

    assert_eq!(
        *seen.lock().expect("the presence seen"),
        vec![true],
        "the bot is on the match as its result is written"
    );
    let status = rematch_status(&app, &finished, &token).await;
    assert_eq!(status["opponentHere"], json!(true), "{status}");
}

#[tokio::test(start_paused = true)]
async fn r639000_a_bot_that_means_to_answer_stays_until_its_answer_lands() {
    // A person who reaches for the button as their linger runs out does not read as gone first: a
    // bot meeting an offer made late in its linger stays on the match until its answer lands.
    let app = app_with_bots(true).await;
    let (person, token) = new_person(&app, 1000.0).await;
    let bot = new_bot(&app, BotSpec::rated(1000.0)).await;
    let seed = seed_where(|plan| plan.accepts && !plan.offers);
    let plan = rematch_plan(&seed, PlayerId::P2, &BOT_REMATCH_FIT);
    let finished = bot_match_on(&app, &person, &bot, &seed).await;
    concede_bot_match(&app, &person).await;

    let early = 100;
    pass(plan.linger_ms - early).await;
    let offered = offer_rematch(&app, &finished, &token).await;
    assert!(offered["matchId"].is_null(), "{offered}");
    // Past the linger, the answer not yet in: still on the match.
    pass(plan.reply_ms - early / 2).await;
    let waiting = rematch_status(&app, &finished, &token).await;
    assert!(waiting["matchId"].is_null(), "{waiting}");
    assert_eq!(waiting["opponentHere"], json!(true), "{waiting}");
    pass(early / 2).await;
    let made = rematch_status(&app, &finished, &token).await;
    assert!(made["matchId"].is_string(), "{made}");
}
