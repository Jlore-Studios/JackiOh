//! R1454 (SPEC §9.12): the bot pool. Its size is `clamp(BOT_POOL_MIN, BOT_POOL_PER_WEEKLY_ACTIVE ×
//! weekly active people, BOT_POOL_MAX)`; an excess retires the youngest accounts and a shortfall puts
//! the most recently retired back before it makes new ones. A new bot is drawn like a player (a
//! strength whose rating seeds its R610 row, a speed, a persona, a schedule, an account age, a name or
//! the default `Player#n`), and its Best-of-1 decks are the patch's human decklists first, then the
//! AI's own, each legal, dealt again whenever the catalog version changes. A bot whose claimed match
//! never started is let go (`release_stale_starts`, which the queue's sweeper runs).
//!
//! R641000: each active bot also holds a Conquest trio, three decks a player could queue as a trio
//! sharing no card (R253): its own Best-of-1 decks first, then decks dealt with the cards already
//! taken left out, dealt again whenever the catalog version changes.
//!
//! `jackioh_server::bots::pool`, against the fake store and the real catalog.

use std::sync::Arc;
use std::time::Duration;

use indexmap::{IndexMap, IndexSet};
use serde_json::{Value, json};

use jackioh_engine::validator::{
    CatalogSnapshot, DeckInput, LoadoutDeck, LoadoutInput, validate_deck, validate_trio,
};
use jackioh_engine::wire::{GameRecord, ListTotal, StatsTotals};
use jackioh_server::actor::engine::{deal_random_deck, deal_random_deck_excluding};
use jackioh_server::api::catalog::Catalog;
use jackioh_server::api::queue::run_matchmaker;
use jackioh_server::app::now_ms;
use jackioh_server::bots::pool::{
    PoolChange, bot_target_strength, draw_decks, draw_new_bot, draw_trio, plan_pool, pool_target,
    refresh_pool, release_stale_starts,
};
use jackioh_server::config::{
    BOT_ACCOUNT_AGE_MAX_DAYS, BOT_ACCOUNT_AGE_MIN_DAYS, BOT_ACTIVE_HOURS_MAX, BOT_ACTIVE_HOURS_MIN,
    BOT_DECK_DRAW_ATTEMPTS, BOT_DECKS_MAX, BOT_DECKS_MIN, BOT_POOL_MAX, BOT_POOL_MIN,
    BOT_POOL_PER_WEEKLY_ACTIVE, BOT_SPEED_MAX, BOT_SPEED_MIN, BOT_STALE_START_SECONDS, BOT_STRENGTHS,
    MATCHMAKER_SWEEP_INTERVAL_SECONDS, PersonaName,
};
use jackioh_server::db::store::{BotDeck, BotRow, FrozenTrio, Ticket};
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
        trio: None,
        trio_catalog_version: None,
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

/// `record`, with `p1` winning or losing.
fn scored(id: &str, deck: &[String], patch: &str, won: bool) -> GameRecord {
    let mut value = serde_json::to_value(record(id, "live", "bo1", "human", deck, patch)).expect("a record");
    value["game"]["winner"] = json!(if won { "p1" } else { "p2" });
    serde_json::from_value(value).expect("a well-formed game record")
}

/// `games` live human Best-of-1 games of `deck`, `wins` of them won.
fn games_of(name: &str, deck: &[String], patch: &str, wins: usize, games: usize) -> Vec<GameRecord> {
    (0..games)
        .map(|n| scored(&format!("{name}-{n}"), deck, patch, n < wins))
        .collect()
}

/// R1044000: the patch's live Best-of-1 human decklists in `records`, as the store's totals hold
/// them once the records are written (`Tx::stats_decklists`).
fn lists_of(records: &[GameRecord], patch: &str) -> Vec<ListTotal> {
    let mut totals = StatsTotals::default();
    for record in records {
        totals.fold(record, false);
    }
    totals.decklists(patch)
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
        let decks = draw_decks(&seed, &lists_of(&records, &patch), &app.catalog, 0.0);
        assert_eq!(
            draw_decks(&seed, &lists_of(&records, &patch), &app.catalog, 0.0),
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
    let decks = draw_decks("decks-none", &[], &app.catalog, 0.0);
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
        let decks = draw_decks(
            &format!("decks-legal-{k}"),
            &lists_of(&records, &patch),
            &app.catalog,
            0.0,
        );
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

/// A trio's three decks' cards, in slot order.
fn trio_cards(trio: &FrozenTrio) -> [&Vec<String>; 3] {
    [&trio.decks.0.cards, &trio.decks.1.cards, &trio.decks.2.cards]
}

/// Whether a player owning exactly the trio's cards could queue it for Conquest: §9.4's L1–L6, as
/// the queue runs them (R253).
fn queueable_trio(catalog: &Catalog, trio: &FrozenTrio) -> bool {
    let mut owned: IndexMap<String, i32> = IndexMap::new();
    for card in trio_cards(trio).into_iter().flatten() {
        *owned.entry(card.clone()).or_insert(0) += 1;
    }
    validate_trio(&LoadoutInput {
        decks: trio_cards(trio)
            .into_iter()
            .map(|cards| LoadoutDeck {
                name: None,
                cards: cards.clone(),
            })
            .collect(),
        catalog: CatalogSnapshot {
            version: catalog.version.clone(),
            cards: catalog.defs.clone(),
            banned: Some(catalog.banned.clone()),
        },
        collection: owned,
    })
    .ok
}

#[test]
fn r641000_a_dealt_deck_leaves_out_the_cards_it_is_told_to() {
    let pool = playable();
    let excluded: Vec<String> = pool[..pool.len() / 2].to_vec();
    for k in 0..20 {
        let seed = format!("excluding-{k}");
        let dealt = deal_random_deck_excluding(&seed, &excluded);
        assert_eq!(dealt.len(), deck_size());
        assert!(
            dealt.iter().all(|card| !excluded.contains(card)),
            "seed {k} dealt an excluded card: {dealt:?}"
        );
        assert_eq!(
            deal_random_deck_excluding(&seed, &excluded),
            dealt,
            "the same seed, the same deck"
        );
        // Leaving nothing out is a player's All Random deal with no lean (R258).
        assert_eq!(
            deal_random_deck_excluding(&seed, &[]),
            deal_random_deck(&seed, None)
        );
    }
}

#[tokio::test]
async fn r641000_a_trio_takes_the_bot_s_own_disjoint_decks_first() {
    let app = empty_test_app().await;
    let pool = playable();
    let size = deck_size();
    let own_deck = |from: usize, portrait: &str| BotDeck {
        cards: pool[from..from + size].to_vec(),
        portrait: portrait.to_string(),
    };
    // B shares half its cards with A, so it is passed over; C shares none with A.
    let (a, b, c) = (
        own_deck(0, "portrait-a"),
        own_deck(size / 2, "portrait-b"),
        own_deck(2 * size, "portrait-c"),
    );
    for deck in [&a, &b, &c] {
        assert!(
            queueable(&app.catalog, &deck.cards),
            "PREMISE: {:?} queues",
            deck.cards
        );
    }
    let own = vec![a.clone(), b.clone(), c.clone()];

    for k in 0..10 {
        let seed = format!("trio-{k}");
        let trio = draw_trio(&seed, &own, &app.catalog, 0.0).expect("a trio");
        assert_eq!(
            draw_trio(&seed, &own, &app.catalog, 0.0),
            Some(trio.clone()),
            "the same seed, the same trio"
        );
        assert_eq!(trio.decks.0.cards, a.cards);
        assert_eq!(trio.decks.0.portrait, Some(Some(a.portrait.clone())));
        assert_eq!(trio.decks.1.cards, c.cards);
        assert_eq!(trio.decks.1.portrait, Some(Some(c.portrait.clone())));
        let third = &trio.decks.2;
        assert!(
            third
                .cards
                .iter()
                .all(|card| !a.cards.contains(card) && !c.cards.contains(card)),
            "seed {k}: the dealt deck shares a card: {:?}",
            third.cards
        );
        assert!(matches!(&third.portrait, Some(Some(portrait)) if !portrait.is_empty()));
        for cards in trio_cards(&trio) {
            assert!(
                queueable(&app.catalog, cards),
                "seed {k}: {cards:?} does not queue"
            );
        }
        assert!(
            queueable_trio(&app.catalog, &trio),
            "seed {k}: the trio does not queue"
        );
    }

    // A bot with no deck of its own is dealt all three.
    let trio = draw_trio("trio-none", &[], &app.catalog, 0.0).expect("a trio");
    let [first, second, third] = trio_cards(&trio);
    assert!(
        first
            .iter()
            .all(|card| !second.contains(card) && !third.contains(card))
    );
    assert!(second.iter().all(|card| !third.contains(card)));
    assert!(queueable_trio(&app.catalog, &trio));
}

#[tokio::test(start_paused = true)]
async fn r641000_refresh_deals_every_active_bot_a_trio_once_per_catalog() {
    let app = empty_test_app().await;
    let now = now_ms();
    let done = refresh_pool(&app, "trios", now).await.expect("a refresh");
    assert_eq!(done.trios, BOT_POOL_MIN, "every new bot is dealt a trio");
    let bots = q!(app, bots_list());
    for bot in &bots {
        let trio = bot.trio.as_ref().expect("a trio");
        assert_eq!(
            bot.trio_catalog_version.as_deref(),
            Some(app.catalog.version.as_str())
        );
        assert!(
            queueable_trio(&app.catalog, trio),
            "{}'s trio does not queue",
            bot.profile_id
        );
    }

    // Nothing changed: nothing is dealt again.
    let again = refresh_pool(&app, "trios-2", now).await.expect("a refresh");
    assert_eq!(again.trios, 0);
    assert_eq!(q!(app, bots_list()), bots);

    // A trio dealt for another catalog is dealt again.
    let stale = &bots[0];
    let old_trio = stale.trio.clone().expect("a trio");
    q!(app, bots_set_trio(&stale.profile_id, &old_trio, "v0.0.1"));
    let done = refresh_pool(&app, "trios-3", now).await.expect("a refresh");
    assert_eq!(done.trios, 1);
    let dealt = q!(app, bots_list())
        .into_iter()
        .find(|bot| bot.profile_id == stale.profile_id)
        .expect("the bot");
    assert_eq!(
        dealt.trio_catalog_version.as_deref(),
        Some(app.catalog.version.as_str())
    );
}

#[test]
fn r750000_bot_target_strength_runs_from_the_weakest_to_the_strongest() {
    assert_eq!(bot_target_strength(0), 0.0);
    assert_eq!(bot_target_strength(BOT_STRENGTHS.len() - 1), 1.0);
    assert_eq!(
        bot_target_strength(BOT_STRENGTHS.len()),
        0.0,
        "an index no strength holds"
    );
    let shares: Vec<f64> = (0..BOT_STRENGTHS.len()).map(bot_target_strength).collect();
    assert!(
        shares.windows(2).all(|pair| pair[0] <= pair[1]),
        "a stronger bot never leans less: {shares:?}"
    );
}

#[tokio::test]
async fn r750000_ranked_lists_deal_in_the_order_of_the_bots_strength() {
    let app = empty_test_app().await;
    let pool = playable();
    let size = deck_size();
    let best: Vec<String> = pool[..size].to_vec();
    let worse: Vec<String> = pool[size..size * 2].to_vec();
    let thin: Vec<String> = pool[size * 2..size * 3].to_vec();
    let patch = app.catalog.version.clone();
    // The worse list's games come first, and the thin list won the one game it played: neither goes
    // ahead of the list that won 25 of its 30.
    let mut records = games_of("worse", &worse, &patch, 5, 30);
    records.push(scored("thin", &thin, &patch, true));
    records.extend(games_of("best", &best, &patch, 25, 30));
    // The strongest bot takes the best list first and the weakest the worst; the thin list follows.
    for (strength, first, second) in [(1.0, &best, &worse), (0.0, &worse, &best)] {
        for k in 0..10 {
            let decks = draw_decks(
                &format!("ranked-{k}"),
                &lists_of(&records, &patch),
                &app.catalog,
                strength,
            );
            assert_eq!(&decks[0].cards, first, "strength {strength}, seed {k}");
            assert_eq!(&decks[1].cards, second, "strength {strength}, seed {k}");
            if let Some(third) = decks.get(2) {
                assert_eq!(third.cards, thin, "strength {strength}, seed {k}");
            }
        }
    }
}

#[tokio::test]
async fn r750000_below_threshold_lists_fall_back_to_the_shuffle() {
    let app = empty_test_app().await;
    let pool = playable();
    let size = deck_size();
    let first: Vec<String> = pool[..size].to_vec();
    let second: Vec<String> = pool[size..size * 2].to_vec();
    let patch = app.catalog.version.clone();
    // Each list one game short of a ranking: the seed's shuffle decides, as it did before any list
    // had its games.
    let mut records = games_of("first", &first, &patch, 19, 19);
    records.extend(games_of("second", &second, &patch, 0, 19));
    let mut led: IndexSet<Vec<String>> = IndexSet::new();
    for k in 0..10 {
        let decks = draw_decks(
            &format!("unranked-{k}"),
            &lists_of(&records, &patch),
            &app.catalog,
            1.0,
        );
        assert!(decks[..2].iter().any(|deck| deck.cards == first), "seed {k}");
        assert!(decks[..2].iter().any(|deck| deck.cards == second), "seed {k}");
        led.insert(decks[0].cards.clone());
    }
    assert_eq!(led.len(), 2, "both lists lead some seed");
}
