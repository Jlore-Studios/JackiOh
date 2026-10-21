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
//! R1555000: when the season's `BOT_GRIND_REFERENCE_RANK`-th placed person is more than
//! `BOT_GRIND_GAP` above every active bot, a refresh sets the top `BOT_GRINDERS` grinding, and clears
//! them once a bot is within the gap less `BOT_GRIND_HYSTERESIS`; a grinder is on more hours, and
//! the queue's sweep pairs it with another bot while nobody is queued (`choose_bot_pair`).
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
use jackioh_server::app::{App, now_ms};
use jackioh_server::bots::fallback::{bot_match_gap_passed, bot_matches, choose_bot_pair, on_schedule};
use jackioh_server::bots::pool::{
    PoolChange, bot_target_strength, draw_decks, draw_new_bot, draw_trio, plan_grind, plan_pool, pool_target,
    refresh_pool, release_stale_starts,
};
use jackioh_server::config::{
    BOT_ACCOUNT_AGE_MAX_DAYS, BOT_ACCOUNT_AGE_MIN_DAYS, BOT_ACTIVE_HOURS_MAX, BOT_ACTIVE_HOURS_MIN,
    BOT_DECK_DRAW_ATTEMPTS, BOT_DECKS_MAX, BOT_DECKS_MIN, BOT_GRIND_ACTIVE_HOURS, BOT_GRIND_GAP,
    BOT_GRIND_HYSTERESIS, BOT_GRIND_REFERENCE_RANK, BOT_GRIND_STEP_GAMES, BOT_GRINDERS, BOT_HOURS_PER_DAY,
    BOT_POOL_MAX, BOT_POOL_MIN, BOT_POOL_PER_WEEKLY_ACTIVE, BOT_SPEED_MAX, BOT_SPEED_MIN,
    BOT_STALE_START_SECONDS, BOT_STRENGTHS, BOT_VS_BOT_MAX_LIVE, BOT_VS_BOT_MIN_GAP_SECONDS,
    MATCHMAKER_SWEEP_INTERVAL_SECONDS, MAX_FAIR_GAP, PersonaName,
};
use jackioh_server::db::store::{BotDeck, BotRow, FrozenTrio, NewBot, RatedGameRow, Ticket};
use jackioh_server::ranked::ladder::fresh_rank;
use jackioh_server::ranked::season::season_id_of;
use jackioh_server::username::normalize_username;

use crate::support::deps::empty_test_app;
use crate::support::store::{fake_lock, q};

/// Unit conversion, not configuration: a day in milliseconds.
const DAY: i64 = 86_400_000;

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
        grinding: false,
        grind_wins: 0,
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

// R1555000 — the top bots' grind

/// Unit conversion, not configuration: the gap between two bots' matches speaks seconds.
const MS_PER_SECOND: i64 = 1000;

/// A bot of the grind's tests: active, on all day, rated `rating`, and grinding or not.
fn rated_bot(id: &str, rating: f64, grinding: bool) -> BotRow {
    BotRow {
        rating,
        grinding,
        ..bot(id, 0, None)
    }
}

/// The 3rd person's rating at which the grind is exactly at its gap from a top bot rated 1200.
const AT_THE_GAP: f64 = 1200.0 + BOT_GRIND_GAP;

#[test]
fn r1555000_the_grind_s_constants_hold_together() {
    const {
        // A grind that starts and stops at the same gap would flap; one that started beyond the fair
        // gap would leave the people who started it unable to meet a grinder at all.
        assert!(0.0 < BOT_GRIND_HYSTERESIS);
        assert!(BOT_GRIND_HYSTERESIS < BOT_GRIND_GAP);
        assert!(BOT_GRIND_GAP <= MAX_FAIR_GAP as f64);
        // A grinder plays more than any bot's own schedule has it, and a day holds that many hours.
        assert!(BOT_ACTIVE_HOURS_MAX < BOT_GRIND_ACTIVE_HOURS);
        assert!(BOT_GRIND_ACTIVE_HOURS <= BOT_HOURS_PER_DAY);
        assert!(BOT_GRIND_REFERENCE_RANK >= 1);
        assert!(BOT_GRINDERS >= 1);
        assert!(BOT_GRIND_STEP_GAMES >= 1);
        assert!(BOT_VS_BOT_MAX_LIVE >= 1);
        assert!(BOT_VS_BOT_MIN_GAP_SECONDS >= 1);
    };
}

#[test]
fn r1555000_grinding_starts_past_the_gap_for_the_top_bots_alone() {
    // A retired bot rated above them all is not the top: only active bots grind or set the gap.
    let mut retired = rated_bot("e", 1500.0, false);
    retired.retired_at = Some(1);
    let bots = vec![
        rated_bot("d", 1100.0, false),
        rated_bot("a", 1200.0, false),
        retired,
        rated_bot("c", 1100.0, false),
        rated_bot("b", 1150.0, false),
    ];
    // The 3rd-highest person is what counts: one or two far ahead start nothing on their own.
    let people = [AT_THE_GAP + 50.0, 900.0, AT_THE_GAP + 40.0, AT_THE_GAP + 1.0];
    const { assert!(BOT_GRIND_REFERENCE_RANK == 3 && BOT_GRINDERS <= 4) };
    let top: Vec<String> = ["a", "b", "c", "d"][..BOT_GRINDERS]
        .iter()
        .map(|id| id.to_string())
        .collect();
    assert_eq!(plan_grind(&bots, &people), top, "the highest first, ties on id");

    // At the gap, not past it: none.
    let at = [AT_THE_GAP + 50.0, AT_THE_GAP + 40.0, AT_THE_GAP];
    assert!(plan_grind(&bots, &at).is_empty());
    // Too few placed people: none, however far ahead.
    assert!(plan_grind(&bots, &[AT_THE_GAP + 500.0, AT_THE_GAP + 400.0]).is_empty());
    // No active bot: none.
    let mut all_retired = bots.clone();
    for bot in &mut all_retired {
        bot.retired_at = Some(1);
    }
    assert!(plan_grind(&all_retired, &people).is_empty());
}

#[test]
fn r1555000_grinding_stops_only_past_the_hysteresis() {
    let people_at = |third: f64| [third + 50.0, third + 40.0, third];
    let stays = 1200.0 + BOT_GRIND_GAP - BOT_GRIND_HYSTERESIS + 1.0;
    let stops = 1200.0 + BOT_GRIND_GAP - BOT_GRIND_HYSTERESIS;
    let grinding = vec![
        rated_bot("a", 1200.0, true),
        rated_bot("b", 1150.0, true),
        rated_bot("c", 1100.0, true),
        rated_bot("d", 1000.0, false),
    ];
    let top: Vec<String> = ["a", "b", "c", "d"][..BOT_GRINDERS]
        .iter()
        .map(|id| id.to_string())
        .collect();
    assert_eq!(plan_grind(&grinding, &people_at(stays)), top);
    assert!(plan_grind(&grinding, &people_at(stops)).is_empty());

    // The grinders are chosen again at each refresh: a bot that grinds now but is no longer among the
    // top hands its place on.
    let mut handed = grinding.clone();
    handed[2].grinding = false;
    handed[3].grinding = true;
    assert_eq!(plan_grind(&handed, &people_at(stays)), top);

    // A grind not yet started needs the whole gap.
    let idle: Vec<BotRow> = grinding
        .iter()
        .map(|bot| BotRow {
            grinding: false,
            ..bot.clone()
        })
        .collect();
    assert!(plan_grind(&idle, &people_at(stays)).is_empty());
}

#[test]
fn r1555000_a_grinder_s_schedule_widens_and_its_own_comes_back() {
    let mut late = bot("late", 0, None);
    late.active_from_hour = 22;
    late.active_hours = 8;
    let hours_on = |bot: &BotRow| {
        (0..BOT_HOURS_PER_DAY)
            .filter(|hour| on_schedule(bot, *hour))
            .count()
    };
    let own: Vec<i32> = (0..BOT_HOURS_PER_DAY)
        .filter(|hour| on_schedule(&late, *hour))
        .collect();
    assert_eq!(own, vec![0, 1, 2, 3, 4, 5, 22, 23], "on from 22:00 for 8 hours");

    late.grinding = true;
    assert_eq!(
        hours_on(&late),
        usize::try_from(BOT_GRIND_ACTIVE_HOURS).expect("hours")
    );
    assert!(
        own.iter().all(|hour| on_schedule(&late, *hour)),
        "widened from its own start, never moved"
    );

    late.grinding = false;
    let back: Vec<i32> = (0..BOT_HOURS_PER_DAY)
        .filter(|hour| on_schedule(&late, *hour))
        .collect();
    assert_eq!(back, own, "its own schedule comes back");
}

/// A rated game between two sides, each a bot's (`Some`) or a person's, that ended at `ended_at`.
fn rated_game(bots: [Option<&str>; 2], ended_at: i64) -> RatedGameRow {
    let side = |bot: Option<&str>| {
        let glicko = json!({ "rating": 1000.0, "deviation": 100.0, "volatility": 0.06 });
        json!({
            "profileId": if bot.is_some() { Value::Null } else { json!("person") },
            "botId": bot,
            "pilot": if bot.is_some() { "ai" } else { "human" },
            "before": glicko,
            "after": glicko,
            "rankBefore": null,
            "rankAfter": null,
        })
    };
    serde_json::from_value(json!({
        "id": format!("game-{ended_at}"),
        "kind": "match",
        "seasonId": "v0.1",
        "patchVersion": "v0.1.1",
        "catalogVersion": "0.1.1",
        "sides": [side(bots[0]), side(bots[1])],
        "winnerSide": 0,
        "reason": "hero-death",
        "endedAt": ended_at,
    }))
    .expect("a rated game")
}

/// What puts a bot out of a match between two bots' reach.
type Unready = fn(&mut BotRow);

#[test]
fn r1555000_the_bot_pair_is_the_lowest_grinder_and_its_closest_ready_bot() {
    let ready = |id: &str, rating: f64, grinding: bool| BotRow {
        decks: vec![BotDeck {
            cards: Vec::new(),
            portrait: "vanilla".to_string(),
        }],
        decks_catalog_version: Some("v".to_string()),
        ..rated_bot(id, rating, grinding)
    };
    let pool = vec![
        ready("g2", 1300.0, true),
        ready("n3", 1000.0, false),
        ready("n2", 1210.0, false),
        ready("g1", 1200.0, true),
        ready("n1", 1190.0, false),
    ];
    let none_recent: IndexMap<String, Vec<String>> = IndexMap::new();
    let none_busy: IndexSet<String> = IndexSet::new();
    let pair = |bots: &[BotRow], recent: &IndexMap<String, Vec<String>>, busy: &IndexSet<String>| {
        choose_bot_pair(bots, recent, busy, 12, "v")
            .map(|(grinder, opponent)| (grinder.profile_id.clone(), opponent.profile_id.clone()))
    };
    let named = |a: &str, b: &str| Some((a.to_string(), b.to_string()));

    // The lowest grinder, against the closest bot: n1 and n2 are 10 off each, so the id decides.
    assert_eq!(pair(&pool, &none_recent, &none_busy), named("g1", "n1"));
    // Not one of the grinder's recent opponents.
    let mut recent = IndexMap::new();
    recent.insert("g1".to_string(), vec!["n1".to_string()]);
    assert_eq!(pair(&pool, &recent, &none_busy), named("g1", "n2"));

    // n1 and n2 each out of reach the same way: the closest ready bot is the other grinder.
    let mut busy = IndexSet::new();
    busy.insert("n1".to_string());
    busy.insert("n2".to_string());
    assert_eq!(pair(&pool, &none_recent, &busy), named("g1", "g2"), "busy");
    let unready: [(&str, Unready); 4] = [
        ("in a match", |bot| {
            bot.in_match_id = Some("elsewhere".to_string())
        }),
        ("retired", |bot| bot.retired_at = Some(1)),
        ("off schedule", |bot| {
            bot.active_from_hour = 13;
            bot.active_hours = 8;
        }),
        ("another catalog", |bot| {
            bot.decks_catalog_version = Some("old".to_string())
        }),
    ];
    for (why, make) in unready {
        let mut bots = pool.clone();
        for bot in bots
            .iter_mut()
            .filter(|bot| ["n1", "n2"].contains(&bot.profile_id.as_str()))
        {
            make(bot);
        }
        assert_eq!(pair(&bots, &none_recent, &none_busy), named("g1", "g2"), "{why}");
    }

    // g1 out of reach: g2 grinds against its own closest.
    let mut bots = pool.clone();
    bots[3].in_match_id = Some("elsewhere".to_string());
    assert_eq!(pair(&bots, &none_recent, &none_busy), named("g2", "n2"));
    // No grinder: no pair.
    let idle: Vec<BotRow> = pool
        .iter()
        .map(|bot| BotRow {
            grinding: false,
            ..bot.clone()
        })
        .collect();
    assert_eq!(pair(&idle, &none_recent, &none_busy), None);

    // A match two bots hold is a match between two bots; one a bot holds alone (a person's) is not.
    let mut held = pool.clone();
    held[0].in_match_id = Some("between-bots".to_string());
    held[1].in_match_id = Some("with-a-person".to_string());
    held[2].in_match_id = Some("between-bots".to_string());
    assert_eq!(bot_matches(&held), vec!["between-bots".to_string()]);

    // The gap runs from the last game between two bots; a game against a person does not count.
    let gap = BOT_VS_BOT_MIN_GAP_SECONDS * MS_PER_SECOND;
    let ended = 1_000_000;
    let games = [rated_game([Some("g1"), Some("n1")], ended)];
    assert!(!bot_match_gap_passed(&games, ended + gap - 1));
    assert!(bot_match_gap_passed(&games, ended + gap));
    let with_a_person = [rated_game([None, Some("g1")], ended)];
    assert!(bot_match_gap_passed(&with_a_person, ended));
}

/// A person rated `rating` with a row in this season, placed or not.
async fn season_person(app: &App, id: &str, rating: f64, placed: bool) {
    fake_lock(app)
        .await
        .seed_profile(json!({ "id": id, "status": "active", "rating": rating }));
    let mut rank = fresh_rank(&season_id_of(jackioh_cards::catalog_version()), id, 0);
    if placed {
        rank.ladder = Some(0);
    }
    q!(app, ranked_put_rank(&rank));
}

#[tokio::test(start_paused = true)]
async fn r1555000_a_refresh_marks_the_top_bots_grinding_and_clears_them() {
    let app = empty_test_app().await;
    let now = now_ms();
    let refreshed = refresh_pool(&app, "grind", now).await.expect("a refresh");
    assert_eq!(refreshed.grinding, 0, "nobody has played");
    let bots = q!(app, bots_list());
    let top = bots
        .iter()
        .map(|bot| bot.rating)
        .fold(f64::NEG_INFINITY, f64::max);
    let mut by_rating = bots.clone();
    by_rating.sort_by(|a, b| {
        b.rating
            .total_cmp(&a.rating)
            .then_with(|| a.profile_id.cmp(&b.profile_id))
    });
    let expected: Vec<String> = by_rating
        .iter()
        .take(BOT_GRINDERS)
        .map(|bot| bot.profile_id.clone())
        .collect();

    // Two placed people past the gap and one who has not placed yet: too few placed to start it.
    let season = season_id_of(jackioh_cards::catalog_version());
    season_person(&app, "grind-person-1", top + BOT_GRIND_GAP + 50.0, true).await;
    season_person(&app, "grind-person-2", top + BOT_GRIND_GAP + 20.0, true).await;
    season_person(&app, "grind-person-unplaced", top + BOT_GRIND_GAP + 30.0, false).await;
    // A retired bot's placed row, rated past the gap too, is no person's.
    let retired: NewBot = serde_json::from_value(json!({
        "profileId": "grind-retired-bot",
        "strength": 0,
        "speed": 1.0,
        "persona": "balanced",
        "activeFromHour": 0,
        "activeHours": 24,
        "decks": [],
        "decksCatalogVersion": null,
        "createdAt": now - DAY,
        "seedRating": top + BOT_GRIND_GAP + 40.0,
    }))
    .expect("a bot");
    q!(app, bots_create(&retired));
    q!(app, bots_set_retired(&retired.profile_id, Some(now)));
    let mut retired_rank = fresh_rank(&season, &retired.profile_id, 0);
    retired_rank.ladder = Some(0);
    q!(app, ranked_put_rank(&retired_rank));
    let refreshed = refresh_pool(&app, "grind-2", now).await.expect("a refresh");
    assert_eq!(refreshed.grinding, 0);
    assert!(q!(app, bots_list()).iter().all(|bot| !bot.grinding));

    // A third placed person past the gap: exactly the top bots grind, their hours and strengths as
    // they were (the widening is read, never written).
    season_person(&app, "grind-person-3", top + BOT_GRIND_GAP + 1.0, true).await;
    let refreshed = refresh_pool(&app, "grind-3", now).await.expect("a refresh");
    assert_eq!(refreshed.grinding, BOT_GRINDERS);
    let after = q!(app, bots_list());
    let mut grinding: Vec<String> = after
        .iter()
        .filter(|bot| bot.grinding)
        .map(|bot| bot.profile_id.clone())
        .collect();
    grinding.sort();
    let mut wanted = expected.clone();
    wanted.sort();
    assert_eq!(grinding, wanted);
    for before in &bots {
        let now_row = after
            .iter()
            .find(|bot| bot.profile_id == before.profile_id)
            .expect("the bot after");
        assert_eq!(
            (now_row.active_from_hour, now_row.active_hours, now_row.strength),
            (before.active_from_hour, before.active_hours, before.strength)
        );
    }

    // The top bot climbs to within the gap less the hysteresis of the third person: the grind stops.
    let climber = &expected[0];
    let mut rating = q!(app, ranked_bot(climber)).expect("its rating");
    rating.glicko.rating = top + BOT_GRIND_GAP + 1.0 - (BOT_GRIND_GAP - BOT_GRIND_HYSTERESIS);
    q!(app, ranked_put_bot(&rating));
    let refreshed = refresh_pool(&app, "grind-4", now).await.expect("a refresh");
    assert_eq!(refreshed.grinding, 0);
    assert!(
        q!(app, bots_list())
            .iter()
            .all(|bot| !bot.grinding && bot.grind_wins == 0)
    );
}
