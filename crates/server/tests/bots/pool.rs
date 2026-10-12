//! R1454 (SPEC §9.12): the bot pool. Its size is `clamp(BOT_POOL_MIN, BOT_POOL_PER_WEEKLY_ACTIVE ×
//! weekly active people, BOT_POOL_MAX)`; an excess retires the youngest accounts and a shortfall puts
//! the most recently retired back before it makes new ones. A new bot is drawn like a player (a
//! strength whose rating seeds its R610 row, a speed, a persona, a schedule, an account age, a name or
//! the default `Player#n`), and its Best-of-1 decks are the patch's human decklists first, then the
//! AI's own, each legal, dealt again whenever the catalog version changes. A bot whose claimed match
//! never started is let go (`release_stale_starts`, which the queue's sweeper runs).
//!
//! `jackioh_server::bots::pool`, against the fake store and the real catalog.

use std::sync::Arc;
use std::time::Duration;

use indexmap::{IndexMap, IndexSet};
use serde_json::{Value, json};

use jackioh_engine::validator::{CatalogSnapshot, DeckInput, LoadoutDeck, validate_deck};
use jackioh_engine::wire::GameRecord;
use jackioh_server::actor::engine::deal_random_deck;
use jackioh_server::api::catalog::Catalog;
use jackioh_server::api::queue::run_matchmaker;
use jackioh_server::app::now_ms;
use jackioh_server::bots::pool::{
    PoolChange, draw_decks, draw_new_bot, plan_pool, pool_target, refresh_pool, release_stale_starts,
};
use jackioh_server::config::{
    BOT_ACCOUNT_AGE_MAX_DAYS, BOT_ACCOUNT_AGE_MIN_DAYS, BOT_ACTIVE_HOURS_MAX, BOT_ACTIVE_HOURS_MIN,
    BOT_DECK_DRAW_ATTEMPTS, BOT_DECKS_MAX, BOT_DECKS_MIN, BOT_POOL_MAX, BOT_POOL_MIN,
    BOT_POOL_PER_WEEKLY_ACTIVE, BOT_SPEED_MAX, BOT_SPEED_MIN, BOT_STALE_START_SECONDS, BOT_STRENGTHS,
    MATCHMAKER_SWEEP_INTERVAL_SECONDS, PersonaName,
};
use jackioh_server::db::store::{BotRow, Ticket};
use jackioh_server::username::normalize_username;

use crate::support::deps::empty_test_app;

/// Unit conversion, not configuration: a day in milliseconds.
const DAY: i64 = 86_400_000;

/// One store call in its own transaction.
macro_rules! q {
    ($app:expr, $method:ident($($arg:expr),* $(,)?)) => {{
        let mut tx = $app.db.begin(None).await.expect("begin");
        let out = tx.$method($($arg),*).await.expect(stringify!($method));
        tx.commit().await.expect("commit");
        out
    }};
}

fn bot(id: &str, created_at: i64, retired_at: Option<i64>) -> BotRow {
    BotRow {
        profile_id: id.to_string(),
        strength: 0,
        speed: 1.0,
        persona: PersonaName::Balanced,
        active_from_hour: 0,
        active_hours: 24,
        decks: Vec::new(),
        decks_catalog_version: None,
        created_at,
        retired_at,
        rating: 1000.0,
        in_match_id: None,
    }
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

fn deck_size() -> usize {
    usize::try_from(jackioh_engine::config::DECK_SIZE).expect("a deck size")
}

/// A record of one game: `p1` piloted by `pilot` on `deck`, in `source` and `mode`.
fn record(id: &str, source: &str, mode: &str, pilot: &str, deck: &[String], patch: &str) -> GameRecord {
    let value: Value = json!({
        "id": id,
        "source": source,
        "mode": mode,
        "patch": patch,
        "pilots": { "p1": pilot, "p2": "ai" },
        "game": {
            "first": "p1",
            "winner": "p1",
            "reason": "hero-death",
            "turns": 9,
            "seats": {
                "p1": { "deck": deck, "opening": [], "drawn": [], "played": [] },
                "p2": { "deck": ["core-002"], "opening": [], "drawn": [], "played": [] }
            }
        }
    });
    serde_json::from_value(value).expect("a well-formed game record")
}

#[test]
fn r1454_pool_target_clamps() {
    assert_eq!(pool_target(0), BOT_POOL_MIN);
    assert_eq!(pool_target(-5), BOT_POOL_MIN);
    assert_eq!(pool_target(i64::from(u32::MAX)), BOT_POOL_MAX);
    let weekly = 100;
    let scaled = (weekly as f64 * BOT_POOL_PER_WEEKLY_ACTIVE).round() as usize;
    assert!(
        (BOT_POOL_MIN..=BOT_POOL_MAX).contains(&scaled),
        "PREMISE: 100 people land inside the clamp"
    );
    assert_eq!(pool_target(weekly), scaled);
}

#[test]
fn r1454_plan_pool_retires_newest_reinstates_first() {
    let bots = vec![
        bot("a", 1_000, None),
        bot("b", 3_000, None),
        bot("c", 2_000, None),
        bot("old-retired", 500, Some(10_000)),
        bot("new-retired", 600, Some(20_000)),
    ];

    // Three active, one wanted: the two youngest accounts (latest `created_at`) go.
    assert_eq!(
        plan_pool(&bots, 1),
        PoolChange {
            retire: vec!["b".to_string(), "c".to_string()],
            reinstate: Vec::new(),
            create: 0,
        }
    );
    // Exactly enough: nothing changes.
    assert_eq!(plan_pool(&bots, 3), PoolChange::default());
    // One short: the most recently retired comes back.
    assert_eq!(
        plan_pool(&bots, 4),
        PoolChange {
            retire: Vec::new(),
            reinstate: vec!["new-retired".to_string()],
            create: 0,
        }
    );
    // Four short: both come back, then two are made.
    assert_eq!(
        plan_pool(&bots, 7),
        PoolChange {
            retire: Vec::new(),
            reinstate: vec!["new-retired".to_string(), "old-retired".to_string()],
            create: 2,
        }
    );
}

#[test]
fn r1454_drawn_bots_in_range_valid_names() {
    let now = 1_800_000_000_000;
    let mut named = 0;
    let mut defaults = 0;
    let mut personas: IndexSet<&str> = IndexSet::new();
    let mut strengths: IndexSet<usize> = IndexSet::new();
    for k in 0..200 {
        let seed = format!("pool-seed-{k}");
        let (drawn, name) = draw_new_bot(&seed, "bot-id", now);
        // The same seed draws the same bot.
        assert_eq!(draw_new_bot(&seed, "bot-id", now), (drawn.clone(), name.clone()));
        assert_eq!(drawn.profile_id, "bot-id");
        assert!(drawn.strength < BOT_STRENGTHS.len());
        assert_eq!(drawn.seed_rating, BOT_STRENGTHS[drawn.strength].seed_rating);
        assert!(
            (BOT_SPEED_MIN..=BOT_SPEED_MAX).contains(&drawn.speed),
            "{}",
            drawn.speed
        );
        assert!((0..24).contains(&drawn.active_from_hour));
        assert!((BOT_ACTIVE_HOURS_MIN..=BOT_ACTIVE_HOURS_MAX).contains(&drawn.active_hours));
        let age = now - drawn.created_at;
        assert!(
            (BOT_ACCOUNT_AGE_MIN_DAYS * DAY..(BOT_ACCOUNT_AGE_MAX_DAYS + 1) * DAY).contains(&age),
            "an account {age} ms old"
        );
        assert!(drawn.decks.is_empty() && drawn.decks_catalog_version.is_none());
        match name {
            Some(name) => {
                named += 1;
                assert!(name.is_ascii(), "{name}");
                assert!(
                    normalize_username(&name).is_ok(),
                    "{name} is not a valid username"
                );
            }
            None => defaults += 1,
        }
        personas.insert(drawn.persona.as_str());
        strengths.insert(drawn.strength);
    }
    // Both kinds of name, every persona and every strength turn up.
    assert!(named > 0 && defaults > 0, "{named} named, {defaults} defaults");
    assert_eq!(personas.len(), 4);
    assert_eq!(strengths.len(), BOT_STRENGTHS.len());
}

#[tokio::test]
async fn r1454_decks_prefer_human_bo1_lists_legal() {
    let app = empty_test_app().await;
    let pool = playable();
    let size = deck_size();
    let human: Vec<String> = pool[..size].to_vec();
    let ai_piloted: Vec<String> = pool[size..size * 2].to_vec();
    let dev_list: Vec<String> = pool[size * 2..size * 3].to_vec();
    let patch = app.catalog.version.clone();
    let records = vec![
        record("live-1", "live", "bo1", "human", &human, &patch),
        // The same list reordered is the same list.
        record(
            "live-2",
            "live",
            "bo1",
            "human",
            &human.iter().rev().cloned().collect::<Vec<_>>(),
            &patch,
        ),
        // A bot's side, a development run's and an All Random deal are no human Best-of-1 list.
        record("live-3", "live", "bo1", "ai", &ai_piloted, &patch),
        record("dev-1", "dev", "bo1", "human", &dev_list, &patch),
        record("live-4", "live", "random", "human", &dev_list, &patch),
    ];

    for k in 0..10 {
        let seed = format!("decks-{k}");
        let decks = draw_decks(&seed, &records, &app.catalog);
        assert_eq!(
            draw_decks(&seed, &records, &app.catalog),
            decks,
            "the same seed deals the same decks"
        );
        assert!(
            (BOT_DECKS_MIN..=BOT_DECKS_MAX).contains(&decks.len()),
            "{} decks",
            decks.len()
        );
        // The one human list comes first; the rest are the AI's own.
        assert_eq!(decks[0].cards, human);
        for deck in &decks[1..] {
            assert_ne!(deck.cards, ai_piloted);
            assert_ne!(deck.cards, dev_list);
            assert_eq!(deck.cards.len(), size);
        }
        assert!(
            decks.iter().all(|deck| !deck.portrait.is_empty()),
            "every deck has its portrait"
        );
    }

    // With no human list at all, every deck is one All Random would deal a player (R1453: no shadow
    // ban and no soft gate, which would tell the bot's deck from a person's), and still legal.
    let decks = draw_decks("decks-none", &[], &app.catalog);
    assert!((BOT_DECKS_MIN..=BOT_DECKS_MAX).contains(&decks.len()));
    assert!(decks.iter().all(|deck| queueable(&app.catalog, &deck.cards)));
    let dealt: Vec<Vec<String>> = (0..BOT_DECK_DRAW_ATTEMPTS)
        .map(|index| deal_random_deck(&format!("decks-none:deck:{index}"), None))
        .collect();
    assert!(
        decks.iter().all(|deck| dealt.contains(&deck.cards)),
        "every deck is a player's random deal"
    );
}

/// Whether a player owning exactly `cards` could queue them for Best of 1: §9.4's validator, as the
/// queue runs it.
fn queueable(catalog: &Catalog, cards: &[String]) -> bool {
    let mut owned: IndexMap<String, i32> = IndexMap::new();
    for card in cards {
        *owned.entry(card.clone()).or_insert(0) += 1;
    }
    validate_deck(&DeckInput {
        deck: LoadoutDeck {
            name: None,
            cards: cards.to_vec(),
        },
        catalog: CatalogSnapshot {
            version: catalog.version.clone(),
            cards: catalog.defs.clone(),
            banned: Some(catalog.banned.clone()),
        },
        collection: owned,
    })
    .ok
}

#[tokio::test]
async fn r1454_decks_skip_a_list_no_player_could_queue() {
    let app = empty_test_app().await;
    let pool = playable();
    let size = deck_size();
    let legal: Vec<String> = pool[..size].to_vec();
    // People's records can hold lists that no longer queue: one card short, one card past its copy
    // limit, a card the catalog no longer has.
    let short: Vec<String> = pool[size..size * 2 - 1].to_vec();
    let over_copied: Vec<String> = vec![pool[size].clone(); size];
    let mut unknown: Vec<String> = pool[size * 2..size * 3 - 1].to_vec();
    unknown.push("no-such-card".to_string());
    let illegal = [short, over_copied, unknown];
    for cards in &illegal {
        assert!(!queueable(&app.catalog, cards), "{cards:?} queues");
    }
    let patch = app.catalog.version.clone();
    let mut records: Vec<GameRecord> = illegal
        .iter()
        .enumerate()
        .map(|(i, cards)| record(&format!("live-bad-{i}"), "live", "bo1", "human", cards, &patch))
        .collect();
    records.push(record("live-good", "live", "bo1", "human", &legal, &patch));

    for k in 0..10 {
        let decks = draw_decks(&format!("decks-legal-{k}"), &records, &app.catalog);
        assert!(
            (BOT_DECKS_MIN..=BOT_DECKS_MAX).contains(&decks.len()),
            "{} decks",
            decks.len()
        );
        // The one legal human list is dealt; none of the others ever is, and every deck queues.
        assert!(decks.iter().any(|deck| deck.cards == legal), "seed {k}");
        for deck in &decks {
            assert!(!illegal.contains(&deck.cards), "seed {k} dealt {:?}", deck.cards);
            assert!(
                queueable(&app.catalog, &deck.cards),
                "seed {k} dealt {:?}",
                deck.cards
            );
        }
    }
}

#[tokio::test(start_paused = true)]
async fn r1454_refresh_seeds_ratings_no_collection() {
    let app = empty_test_app().await;
    let now = now_ms();
    let done = refresh_pool(&app, "refresh", now).await.expect("a refresh");
    assert_eq!(done.target, BOT_POOL_MIN, "no one has played, so the floor");
    assert_eq!(done.created, BOT_POOL_MIN);
    assert_eq!(done.dealt, BOT_POOL_MIN);

    let bots = q!(app, bots_list());
    assert_eq!(bots.len(), BOT_POOL_MIN);
    for bot in &bots {
        // Its R610 row starts at its strength's rating, with no games.
        let rating = q!(app, ranked_bot(&bot.profile_id)).expect("an R610 row");
        assert_eq!(rating.glicko.rating, BOT_STRENGTHS[bot.strength].seed_rating);
        assert_eq!(rating.games, 0);
        assert_eq!(bot.rating, rating.glicko.rating);
        assert!(bot.retired_at.is_none());
        assert!((BOT_DECKS_MIN..=BOT_DECKS_MAX).contains(&bot.decks.len()));
        assert_eq!(
            bot.decks_catalog_version.as_deref(),
            Some(app.catalog.version.as_str())
        );
        // A bot owns nothing: no grant, no collection.
        assert!(q!(app, collection_get(&bot.profile_id)).is_empty());
        let profile = q!(app, profiles_get_by_id(&bot.profile_id)).expect("its profile");
        assert!(profile.created_at < now);
    }

    // A second refresh with nothing changed changes nothing.
    let again = refresh_pool(&app, "refresh-2", now).await.expect("a refresh");
    assert_eq!(
        (again.created, again.retired, again.reinstated, again.dealt),
        (0, 0, 0, 0)
    );
    assert_eq!(q!(app, bots_list()), bots);
}

#[tokio::test(start_paused = true)]
async fn r1454_new_catalog_redraws_decks() {
    let app = empty_test_app().await;
    let now = now_ms();
    refresh_pool(&app, "catalog", now).await.expect("a refresh");
    let bots = q!(app, bots_list());
    let stale = &bots[0];
    let kept = &bots[1];
    q!(app, bots_set_decks(&stale.profile_id, &stale.decks, "v0.0.1"));
    // A retired bot's decks wait until it is back.
    q!(app, bots_set_retired(&kept.profile_id, Some(now)));
    q!(app, bots_set_decks(&kept.profile_id, &kept.decks, "v0.0.1"));

    let done = refresh_pool(&app, "catalog-2", now).await.expect("a refresh");
    // The retired one is put back to keep the floor, and both are dealt for this catalog.
    assert_eq!((done.reinstated, done.created, done.dealt), (1, 0, 2));
    for bot in q!(app, bots_list()) {
        assert_eq!(
            bot.decks_catalog_version.as_deref(),
            Some(app.catalog.version.as_str())
        );
    }
}

#[tokio::test(start_paused = true)]
async fn r1454_a_bot_whose_start_was_cut_off_is_let_go() {
    let app = empty_test_app().await;
    let now = now_ms();
    refresh_pool(&app, "cut", now).await.expect("a refresh");
    let bots = q!(app, bots_list());
    let (cut, going) = (&bots[0], &bots[1]);
    let stale_ms = BOT_STALE_START_SECONDS * 1000;
    let later = now + stale_ms;
    // Two claims whose matches never started: one a whole BOT_STALE_START_SECONDS ago (a start a
    // restart cut off), one just now (a start still going on).
    let mut claimed = Vec::new();
    for (bot, at) in [(cut, now), (going, later)] {
        let ticket: Ticket = serde_json::from_value(json!({
            "id": format!("ticket-{}", bot.profile_id),
            "profileId": format!("person-{}", bot.profile_id),
            "rating": 1000,
            "mode": "bo1",
            "deck": [],
            "portrait": null,
            "trio": null,
            "catalogVersion": app.catalog.version,
            "enqueuedAt": at,
            "status": "open",
            "matchId": null,
        }))
        .expect("a ticket");
        q!(app, tickets_insert(&ticket));
        let match_id = format!("match-{}", bot.profile_id);
        assert!(q!(
            app,
            tickets_claim_for_bot(&ticket.id, &bot.profile_id, &match_id, at)
        ));
        claimed.push(match_id);
    }

    assert_eq!(release_stale_starts(&app, later + 1).await.expect("a release"), 1);
    let after = q!(app, bots_list());
    let in_match = |id: &str| {
        after
            .iter()
            .find(|bot| bot.profile_id == id)
            .and_then(|bot| bot.in_match_id.clone())
    };
    assert_eq!(in_match(&cut.profile_id), None, "the cut-off start's bot is free");
    assert_eq!(in_match(&going.profile_id), Some(claimed[1].clone()));
    assert_eq!(release_stale_starts(&app, later + 1).await.expect("a release"), 0);
}

#[tokio::test(start_paused = true)]
async fn r1454_the_sweeper_lets_a_cut_off_start_go_with_the_bots_off() {
    // A start cut off by a restart whose new deploy turned the bots off: nothing will seat a bot
    // again, but the bot and its player are still let go.
    let app = empty_test_app().await;
    assert!(!app.env.ladder_bots, "PREMISE: the bots are off");
    let now = now_ms();
    refresh_pool(&app, "off", now).await.expect("a refresh");
    let bot = q!(app, bots_list()).remove(0);
    let ticket: Ticket = serde_json::from_value(json!({
        "id": "ticket-cut-off",
        "profileId": "person-cut-off",
        "rating": 1000,
        "mode": "bo1",
        "deck": [],
        "portrait": null,
        "trio": null,
        "catalogVersion": app.catalog.version,
        "enqueuedAt": now,
        "status": "open",
        "matchId": null,
    }))
    .expect("a ticket");
    q!(app, tickets_insert(&ticket));
    assert!(q!(
        app,
        tickets_claim_for_bot(&ticket.id, &bot.profile_id, "match-cut-off", now)
    ));

    let sweeper = tokio::spawn(run_matchmaker(Arc::clone(&app)));
    let wait = BOT_STALE_START_SECONDS + 2 * MATCHMAKER_SWEEP_INTERVAL_SECONDS;
    tokio::time::sleep(Duration::from_secs(u64::try_from(wait).expect("a wait"))).await;
    sweeper.abort();
    let held = q!(app, bots_list())
        .into_iter()
        .find(|row| row.profile_id == bot.profile_id)
        .and_then(|row| row.in_match_id);
    assert_eq!(held, None, "the cut-off start's bot is free");
}
