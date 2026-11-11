//! The ladder bots' pool (`src/bots/pool.rs`, R1454, R1555000, R750000, R641000): its size, the change
//! that brings it there, the top bots' grind, a new bot's draw, and the decks and trio a bot is dealt.
//! The server's tests (crates/server/tests/bots/pool.rs) hold the refresh that calls these; these hold
//! the functions themselves, which a mutant of this crate is tested against.

use jackioh_engine::deckable;
use jackioh_engine::validator::CatalogSnapshot;
use jackioh_shared::bots::pool::{
    BotDeck, BotFacts, BotLadder, PoolChange, bot_target_strength, deal_bot_deck, draw_decks, draw_new_bot,
    draw_trio, plan_grind, plan_pool, pool_target,
};
use jackioh_shared::config::{
    BOT_ACCOUNT_AGE_MAX_DAYS, BOT_ACCOUNT_AGE_MIN_DAYS, BOT_GRIND_GAP, BOT_GRIND_HYSTERESIS,
    BOT_GRIND_REFERENCE_RANK, BOT_GRINDERS, BOT_POOL_MAX, BOT_POOL_MIN, BOT_POOL_PER_WEEKLY_ACTIVE,
    BOT_SPEED_MAX, BOT_SPEED_MIN, BOT_STRENGTHS, MS_PER_DAY,
};
use jackioh_shared::deck::FrozenTrio;

const NOW: i64 = 1_800_000_000_000;

/// A bot made at `created_at`, retired at `retired_at` or active, on all day, rated 1000.
fn bot(id: &str, created_at: i64, retired_at: Option<i64>) -> BotFacts {
    BotFacts {
        profile_id: id.to_string(),
        created_at,
        retired_at,
        grinding: false,
        rating: 1000.0,
        random_rating: 1000.0,
        active_from_hour: 0,
        active_hours: 24,
        in_match_id: None,
        decks: Vec::new(),
        decks_catalog_version: None,
        trio: None,
        trio_catalog_version: None,
        cosmetics: None,
    }
}

fn ids(ids: &[&str]) -> Vec<String> {
    ids.iter().map(|id| id.to_string()).collect()
}

/// R1454: the pool holds one bot for each weekly person, never fewer than BOT_POOL_MIN and never more
/// than BOT_POOL_MAX.
#[test]
fn r1454_the_pool_holds_its_share_of_the_weekly_people_inside_its_bounds() {
    assert_eq!(pool_target(0), BOT_POOL_MIN);
    assert_eq!(pool_target(-5), BOT_POOL_MIN);
    assert_eq!(pool_target(i64::from(u32::MAX)), BOT_POOL_MAX);
    let weekly = 100;
    let scaled = (weekly as f64 * BOT_POOL_PER_WEEKLY_ACTIVE).round() as usize;
    assert!(
        (BOT_POOL_MIN..=BOT_POOL_MAX).contains(&scaled),
        "PREMISE: 100 people land inside the bounds"
    );
    assert_eq!(pool_target(weekly), scaled);
}

/// R1454: a refresh retires the youngest active accounts down to the target, and below it puts back
/// the most recently retired first and makes new bots only for the rest.
#[test]
fn r1454_a_refresh_retires_the_youngest_and_puts_back_the_latest_retired_first() {
    let bots = [
        bot("a", 1_000, None),
        bot("b", 3_000, None),
        bot("c", 2_000, None),
        bot("old-retired", 500, Some(10_000)),
        bot("new-retired", 600, Some(20_000)),
    ];
    // Three active, one wanted: the two youngest accounts go.
    assert_eq!(
        plan_pool(&bots, 1),
        PoolChange {
            retire: ids(&["b", "c"]),
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
            reinstate: ids(&["new-retired"]),
            create: 0,
        }
    );
    // Four short: both come back, then two are made.
    assert_eq!(
        plan_pool(&bots, 7),
        PoolChange {
            retire: Vec::new(),
            reinstate: ids(&["new-retired", "old-retired"]),
            create: 2,
        }
    );
}

/// A bot of the grind's tests: active, on all day, rated `rating`, and grinding or not.
fn rated(id: &str, rating: f64, grinding: bool) -> BotFacts {
    BotFacts {
        rating,
        grinding,
        ..bot(id, 0, None)
    }
}

/// The 3rd person's rating at which the grind is exactly at its gap from a top bot rated 1200.
const AT_THE_GAP: f64 = 1200.0 + BOT_GRIND_GAP;

/// R1555000: the top BOT_GRINDERS active bots grind once the BOT_GRIND_REFERENCE_RANK-th person is more
/// than BOT_GRIND_GAP above the top active bot, and none at the gap or with too few people placed.
#[test]
fn r1555000_the_top_bots_grind_once_the_third_person_is_past_the_gap() {
    const { assert!(BOT_GRIND_REFERENCE_RANK == 3 && BOT_GRINDERS <= 4) };
    // A retired bot rated above them all is not the top: only active bots grind or set the gap.
    let bots = [
        rated("d", 1100.0, false),
        rated("a", 1200.0, false),
        BotFacts {
            retired_at: Some(1),
            ..rated("e", 1500.0, false)
        },
        rated("c", 1100.0, false),
        rated("b", 1150.0, false),
    ];
    let top = ids(&["a", "b", "c", "d"][..BOT_GRINDERS]);
    // The 3rd-highest person is what counts: one or two far ahead start nothing on their own.
    let people = [AT_THE_GAP + 50.0, 900.0, AT_THE_GAP + 40.0, AT_THE_GAP + 1.0];
    assert_eq!(plan_grind(&bots, &people, BotLadder::Constructed), top);
    // At the gap, not past it: none.
    let at = [AT_THE_GAP + 50.0, AT_THE_GAP + 40.0, AT_THE_GAP];
    assert!(plan_grind(&bots, &at, BotLadder::Constructed).is_empty());
    // Too few people placed: none, however far ahead.
    let two = [AT_THE_GAP + 500.0, AT_THE_GAP + 400.0];
    assert!(plan_grind(&bots, &two, BotLadder::Constructed).is_empty());
}

/// R1555000: a grind under way goes on until the third person is within BOT_GRIND_GAP less
/// BOT_GRIND_HYSTERESIS of the top bot; one not yet started needs the whole gap.
#[test]
fn r1555000_a_grind_stops_only_past_the_hysteresis() {
    let people_at = |third: f64| [third + 50.0, third + 40.0, third];
    let stays = 1200.0 + BOT_GRIND_GAP - BOT_GRIND_HYSTERESIS + 1.0;
    let stops = 1200.0 + BOT_GRIND_GAP - BOT_GRIND_HYSTERESIS;
    let grinding = [
        rated("a", 1200.0, true),
        rated("b", 1150.0, true),
        rated("c", 1100.0, true),
        rated("d", 1000.0, false),
    ];
    let top = ids(&["a", "b", "c", "d"][..BOT_GRINDERS]);
    assert_eq!(
        plan_grind(&grinding, &people_at(stays), BotLadder::Constructed),
        top
    );
    assert!(plan_grind(&grinding, &people_at(stops), BotLadder::Constructed).is_empty());
    let idle = grinding.clone().map(|bot| BotFacts {
        grinding: false,
        ..bot
    });
    assert!(plan_grind(&idle, &people_at(stays), BotLadder::Constructed).is_empty());
}

/// R1454: a new bot plays at a speed from BOT_SPEED_MIN to BOT_SPEED_MAX, and its account is
/// BOT_ACCOUNT_AGE_MIN_DAYS to BOT_ACCOUNT_AGE_MAX_DAYS whole days old and part of one more, the
/// youngest and the oldest each drawn for some bot.
#[test]
fn r1454_a_new_bot_s_speed_and_account_age_are_inside_their_ranges() {
    let ages = BOT_ACCOUNT_AGE_MIN_DAYS * MS_PER_DAY..(BOT_ACCOUNT_AGE_MAX_DAYS + 1) * MS_PER_DAY;
    let (mut youngest, mut oldest) = (i64::MAX, i64::MIN);
    for k in 0..3_000 {
        let (drawn, _) = draw_new_bot(&format!("new-bot-{k}"), "bot-id", NOW, &mut |name| {
            Some(name.to_string())
        });
        assert!(
            (BOT_SPEED_MIN..=BOT_SPEED_MAX).contains(&drawn.speed),
            "seed {k}: speed {}",
            drawn.speed
        );
        let age = NOW - drawn.created_at;
        assert!(ages.contains(&age), "seed {k}: an account {age} ms old");
        youngest = youngest.min(age);
        oldest = oldest.max(age);
    }
    assert!(
        youngest < (BOT_ACCOUNT_AGE_MIN_DAYS + 1) * MS_PER_DAY
            && oldest >= BOT_ACCOUNT_AGE_MAX_DAYS * MS_PER_DAY,
        "accounts from {youngest} to {oldest} ms old"
    );
}

/// R750000: a bot's lean runs from 0.0 for the weakest strength to 1.0 for the strongest, each strength
/// at its seed rating's place between the two, and 0.0 for an index no strength holds.
#[test]
fn r750000_a_bot_s_lean_is_its_seed_rating_s_place_among_the_strengths() {
    let ratings: Vec<f64> = BOT_STRENGTHS
        .iter()
        .map(|strength| strength.seed_rating)
        .collect();
    let low = ratings.iter().copied().fold(f64::INFINITY, f64::min);
    let high = ratings.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    assert!(
        low < high && ratings.len() > 2,
        "PREMISE: three strengths or more, rated apart"
    );
    for (index, rating) in ratings.iter().enumerate() {
        assert_eq!(
            bot_target_strength(index),
            (rating - low) / (high - low),
            "strength {index}"
        );
    }
    assert_eq!(bot_target_strength(ratings.len()), 0.0);
}

/// The registered catalog as a snapshot, banning `banned`.
fn catalog(banned: Vec<String>) -> CatalogSnapshot {
    jackioh_cards::register_all();
    CatalogSnapshot {
        version: jackioh_cards::CATALOG_VERSION.to_string(),
        cards: jackioh_cards::CATALOG.clone(),
        banned: Some(banned),
    }
}

/// One deckable card in 50 of the registered catalog: a deal of DECK_SIZE cards holds one of them
/// about half the time.
fn every_fiftieth_card() -> Vec<String> {
    jackioh_cards::register_all();
    jackioh_cards::CATALOG
        .iter()
        .filter(|(_, def)| deckable(def))
        .step_by(50)
        .map(|(id, _)| id.clone())
        .collect()
}

fn trio_cards(trio: &FrozenTrio) -> [&Vec<String>; 3] {
    [&trio.decks.0.cards, &trio.decks.1.cards, &trio.decks.2.cards]
}

/// R1454, R641000: a bot is dealt only decks, and a trio only of decks, that a player could queue on
/// the catalog it is handed: a deal holding a card that catalog bans is passed over for the next.
#[test]
fn r1454_a_bot_is_dealt_only_decks_a_player_could_queue_on_its_catalog() {
    let banned = every_fiftieth_card();
    assert!(!banned.is_empty(), "PREMISE: some cards are banned");
    let catalog = catalog(banned.clone());
    let clean = |cards: &[String]| !cards.iter().any(|card| banned.contains(card));
    for k in 0..8 {
        let decks = draw_decks(&format!("banned-{k}"), &[], &catalog, 0.0, None);
        assert!(!decks.is_empty(), "seed {k}: no decks");
        assert!(
            decks.iter().all(|deck| clean(deck.cards.as_slice())),
            "seed {k}: a deck holds a banned card"
        );
        let trio = draw_trio(&format!("banned-trio-{k}"), &[], &catalog, 0.0, None)
            .unwrap_or_else(|| panic!("seed {k}: no trio"));
        assert!(
            trio_cards(&trio).iter().all(|cards| clean(cards.as_slice())),
            "seed {k}: a trio deck holds a banned card"
        );
    }
}

/// R641000, R253: a trio keeps the bot's own decks, in their order, passing over one that shares a card
/// with a deck it already kept, and deals none while it has three.
#[test]
fn r641000_a_trio_keeps_the_bot_s_own_decks_that_share_no_card() {
    let catalog = catalog(Vec::new());
    let a = deal_bot_deck("own-a", None, 0.0, None);
    let b = deal_bot_deck("own-b", Some(a.as_slice()), 0.0, None);
    let used: Vec<String> = a.iter().chain(&b).cloned().collect();
    let c = deal_bot_deck("own-c", Some(used.as_slice()), 0.0, None);
    let own = |cards: &[String]| BotDeck {
        cards: cards.to_vec(),
        portrait: "vanilla".to_string(),
        slots: None,
    };
    let trio = draw_trio("own", &[own(&a), own(&a), own(&b), own(&c)], &catalog, 0.0, None)
        .expect("a trio of the bot's own decks");
    assert_eq!(trio_cards(&trio), [&a, &b, &c]);
}
