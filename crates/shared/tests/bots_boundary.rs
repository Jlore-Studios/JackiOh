//! The ladder bots' JSON boundary (`jackioh-wasm-core`'s `bots(call)` and `bot_decide`, #3103): every
//! call answers what the function it names answers, over the plain records a host without row types hands
//! over, and the bot seat's decision runs from the machine as JSON.

use indexmap::IndexSet;
use jackioh_engine::PlayerId::{P1, P2};
use jackioh_engine::validator::CatalogSnapshot;
use jackioh_engine::{PerPlayer, Rng, WinRateRow, WinRateSource, WinRateTable, deckable};
use jackioh_shared::bots::boundary::{BotAnswer, BotCall, GameFacts, TicketFacts, apply};
use jackioh_shared::bots::cosmetics::{CosmeticOwners, Holding, eligible, redraw};
use jackioh_shared::bots::fallback::{
    BotMode, bot_match_gap_passed, bot_matches, choose_bot, fallback_due_keyed, fallback_seconds_keyed,
    hour_of,
};
use jackioh_shared::bots::pool::{
    BotDeck, BotFacts, BotLadder, draw_decks, draw_new_bot, draw_trio, plan_grind, plan_pool, pool_target,
};
use jackioh_shared::config::{
    BOT_ACTIVE_HOURS_MIN, BOT_COSMETIC_FITS, BOT_COSMETIC_MIN_AGE_DAYS, BOT_COSMETIC_MIN_OWNERS,
    BOT_FALLBACK_MAX_SECONDS, BOT_POOL_MIN, BOT_VS_BOT_MIN_GAP_SECONDS, MS_PER_DAY, MS_PER_SECOND,
    PersonaName,
};
use jackioh_shared::cosmetics::registry;
use jackioh_shared::host::bot::BotSeat;
use jackioh_shared::host::boundary::{DecideCall, decide};
use jackioh_shared::host::clock::initial_clocks;
use jackioh_shared::host::{DecideRequest, DuelHost, DuelSetup};

const NOW: i64 = 1_800_000_000_000;

fn bot(id: &str, rating: f64) -> BotFacts {
    BotFacts {
        profile_id: id.to_string(),
        created_at: NOW - 1_000,
        retired_at: None,
        grinding: false,
        rating,
        random_rating: rating,
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

fn ticket(id: &str, mode: BotMode, rating: f64) -> TicketFacts {
    TicketFacts {
        id: id.to_string(),
        rating,
        mode,
        enqueued_at: NOW,
    }
}

/// A call as a host sends it: through JSON and back.
fn over_json(call: &BotCall) -> BotCall {
    serde_json::from_str(&serde_json::to_string(call).expect("a call serializes")).expect("a call reads back")
}

#[test]
fn the_pool_calls_answer_what_the_functions_they_name_answer() {
    let bots = vec![bot("b", 1500.0), bot("a", 1600.0), {
        let mut retired = bot("c", 1400.0);
        retired.retired_at = Some(NOW - 5);
        retired
    }];
    let call = BotCall::PoolTarget { weekly: 3 };
    assert_eq!(
        apply(over_json(&call)),
        BotAnswer::Count {
            count: u32::try_from(pool_target(3)).unwrap()
        }
    );
    assert_eq!(pool_target(3), BOT_POOL_MIN);
    let call = BotCall::PlanPool {
        bots: bots.clone(),
        target: 4,
    };
    assert_eq!(
        apply(over_json(&call)),
        BotAnswer::Change {
            change: plan_pool(&bots, 4)
        }
    );
    let call = BotCall::PlanGrind {
        bots: bots.clone(),
        people: vec![2000.0, 1900.0, 1850.0],
        ladder: BotLadder::Constructed,
    };
    assert_eq!(
        apply(over_json(&call)),
        BotAnswer::Ids {
            ids: plan_grind(&bots, &[2000.0, 1900.0, 1850.0], BotLadder::Constructed)
        }
    );
    // A new bot is the same draw the server's refresh makes, and its username is the host's to give.
    let call = BotCall::NewBot {
        seed: "s".to_string(),
        profile_id: "p".to_string(),
        now: NOW,
    };
    let BotAnswer::Bot { bot: drawn } = apply(over_json(&call)) else {
        panic!("a bot")
    };
    let (expected, _) = draw_new_bot("s", "p", NOW, &mut |_| None);
    assert_eq!(*drawn, expected);
    assert!(drawn.active_hours >= BOT_ACTIVE_HOURS_MIN);
}

/// R3049002: the deal calls lean on the win-rate table a host hands them (`winRates`), as the server's
/// refresh hands its snapshot of the cached card stats, and on nothing when it hands none.
#[test]
fn r3049002_the_deal_calls_lean_on_the_win_rate_table_they_are_handed() {
    jackioh_cards::register_all();
    let catalog = CatalogSnapshot {
        version: jackioh_cards::CATALOG_VERSION.to_string(),
        cards: jackioh_cards::CATALOG.clone(),
        banned: Some(Vec::new()),
    };
    // Every other deckable card wins nine games in ten, the rest one in ten, over many games.
    let table = WinRateTable {
        patch: "test".to_string(),
        source: WinRateSource::Live,
        cards: jackioh_cards::CATALOG
            .iter()
            .filter(|(_, def)| deckable(def))
            .enumerate()
            .map(|(at, (id, _))| WinRateRow {
                id: id.clone(),
                wins: if at % 2 == 0 { 900 } else { 100 },
                games: 1_000,
            })
            .collect(),
    };
    let handed = BotCall::DrawDecks {
        seed: "lean".to_string(),
        lists: Vec::new(),
        catalog: Box::new(catalog.clone()),
        strength: 1.0,
        win_rates: Some(table.clone()),
    };
    let leaned = draw_decks("lean", &[], &catalog, 1.0, Some(&table));
    assert!(!leaned.is_empty());
    assert_eq!(
        apply(over_json(&handed)),
        BotAnswer::Decks {
            decks: leaned.clone()
        }
    );
    // A host that hands none is not sent a key, and the deal leans on nothing.
    let none = BotCall::DrawDecks {
        seed: "lean".to_string(),
        lists: Vec::new(),
        catalog: Box::new(catalog.clone()),
        strength: 1.0,
        win_rates: None,
    };
    let json = serde_json::to_value(&none).expect("a call serializes");
    assert!(json.get("winRates").is_none(), "{json}");
    assert!(
        serde_json::to_value(&handed)
            .expect("a call serializes")
            .get("winRates")
            .is_some()
    );
    let BotAnswer::Decks { decks: unleaned } = apply(over_json(&none)) else {
        panic!("decks")
    };
    assert_eq!(unleaned, draw_decks("lean", &[], &catalog, 1.0, None));
    assert_eq!(unleaned, draw_decks("lean", &[], &catalog, 0.0, None));
    assert_ne!(unleaned, leaned, "the table moved the deal");
    // The trio's deals lean the same way.
    let trio = BotCall::DrawTrio {
        seed: "lean".to_string(),
        own: Vec::new(),
        catalog: Box::new(catalog.clone()),
        strength: 1.0,
        win_rates: Some(table.clone()),
    };
    let expected = draw_trio("lean", &[], &catalog, 1.0, Some(&table)).expect("a trio");
    assert_eq!(
        apply(over_json(&trio)),
        BotAnswer::Trio {
            trio: Some(Box::new(expected.clone()))
        }
    );
    assert_ne!(
        draw_trio("lean", &[], &catalog, 1.0, None),
        Some(expected),
        "the table moved the trio"
    );
}

#[test]
fn r3103000_the_fallback_calls_pair_a_ticket_when_it_is_due_and_a_bot_that_is_not_hidden_has_no_wait() {
    let waiting = ticket("ticket-1", BotMode::Bo1, 1500.0);
    let call = |now: i64, hidden: bool| BotCall::FallbackDue {
        key: "key".to_string(),
        ticket: waiting.clone(),
        now,
        hidden,
    };
    // Hidden, a ticket waits its fallback time (15 to 40 s) before a bot may pair with it.
    assert_eq!(
        apply(over_json(&call(NOW, true))),
        BotAnswer::Due {
            due: fallback_due_keyed("key", &waiting, NOW, true)
        }
    );
    assert_eq!(apply(over_json(&call(NOW, true))), BotAnswer::Due { due: false });
    assert_eq!(
        apply(over_json(&call(
            NOW + BOT_FALLBACK_MAX_SECONDS * MS_PER_SECOND,
            true
        ))),
        BotAnswer::Due { due: true }
    );
    // R3103000: not hidden, every ticket is due the moment it is queued.
    assert_eq!(apply(over_json(&call(NOW, false))), BotAnswer::Due { due: true });

    // The pairing: the bot closest in rating, on schedule, free and not recent.
    let mut bots = vec![bot("far", 900.0), bot("near", 1480.0), bot("busy", 1500.0)];
    bots[2].in_match_id = Some("m".to_string());
    for bot in &mut bots {
        bot.decks_catalog_version = None;
    }
    let call = BotCall::ChooseBot {
        ticket: ticket("t", BotMode::Random, 1500.0),
        bots: bots.clone(),
        recent: Vec::new(),
        busy: Vec::new(),
        hour: hour_of(NOW),
        catalog_version: "v".to_string(),
    };
    assert_eq!(
        apply(over_json(&call)),
        BotAnswer::Chosen {
            bot: choose_bot(
                &ticket("t", BotMode::Random, 1500.0),
                &bots,
                &[],
                &IndexSet::new(),
                hour_of(NOW),
                "v"
            )
            .map(|chosen| chosen.profile_id.clone())
        }
    );
    assert_eq!(
        apply(over_json(&call)),
        BotAnswer::Chosen {
            bot: Some("near".to_string())
        }
    );
    assert_eq!(
        apply(over_json(&BotCall::HourOf { now: NOW })),
        BotAnswer::Hour { hour: hour_of(NOW) }
    );
}

#[test]
fn the_kickoff_calls_count_the_matches_between_bots_and_the_gap_since_the_last() {
    let call = BotCall::BotMatches {
        in_match: vec![
            Some("between-bots".to_string()),
            Some("with-a-person".to_string()),
            None,
            Some("between-bots".to_string()),
        ],
    };
    assert_eq!(
        apply(over_json(&call)),
        BotAnswer::Ids {
            ids: bot_matches([
                Some("between-bots"),
                Some("with-a-person"),
                None,
                Some("between-bots")
            ])
        }
    );
    let games = vec![GameFacts {
        between_bots: true,
        ended_at: NOW,
    }];
    let gap = BOT_VS_BOT_MIN_GAP_SECONDS * MS_PER_SECOND;
    let call = |now: i64| BotCall::BotMatchGapPassed {
        games: games.clone(),
        now,
    };
    assert_eq!(
        apply(over_json(&call(NOW + gap - 1))),
        BotAnswer::Passed { passed: false }
    );
    assert_eq!(
        apply(over_json(&call(NOW + gap))),
        BotAnswer::Passed { passed: true }
    );
    assert!(bot_match_gap_passed(&games, NOW + gap));
}

/// R1453: a hidden bot pairs with a ticket once that ticket has waited its own fallback time, drawn
/// from its id, and not a millisecond before.
#[test]
fn r1453_a_tickets_fallback_time_is_its_own_drawn_from_its_id() {
    let ids = ["ticket-1", "ticket-2", "ticket-3", "ticket-4", "ticket-5"];
    let waits: IndexSet<i64> = ids.iter().map(|id| fallback_seconds_keyed("key", id)).collect();
    assert!(
        waits.len() > 1,
        "PREMISE: the tickets wait different times, {waits:?}"
    );
    for id in ids {
        let waiting = ticket(id, BotMode::Bo1, 1500.0);
        let due_at = waiting.enqueued_at + fallback_seconds_keyed("key", id) * MS_PER_SECOND;
        let due = |now: i64| {
            apply(over_json(&BotCall::FallbackDue {
                key: "key".to_string(),
                ticket: waiting.clone(),
                now,
                hidden: true,
            }))
        };
        assert_eq!(due(due_at - 1), BotAnswer::Due { due: false }, "{id}");
        assert_eq!(due(due_at), BotAnswer::Due { due: true }, "{id}");
    }
}

/// R1453, R1555000: a bot the host holds busy is never chosen, however close its rating; and only a
/// game between two bots holds the gap before the next, never one with a person.
#[test]
fn r1555000_a_busy_bot_is_not_chosen_and_only_a_game_between_bots_holds_the_gap() {
    let bots = vec![bot("far", 900.0), bot("near", 1480.0)];
    let choose = |busy: Vec<String>| {
        apply(over_json(&BotCall::ChooseBot {
            ticket: ticket("t", BotMode::Random, 1500.0),
            bots: bots.clone(),
            recent: Vec::new(),
            busy,
            hour: hour_of(NOW),
            catalog_version: "v".to_string(),
        }))
    };
    assert_eq!(
        choose(Vec::new()),
        BotAnswer::Chosen {
            bot: Some("near".to_string())
        }
    );
    let held = choose(vec!["near".to_string()]);
    assert_ne!(
        held,
        BotAnswer::Chosen {
            bot: Some("near".to_string())
        }
    );
    let busy: IndexSet<String> = ["near".to_string()].into_iter().collect();
    assert_eq!(
        held,
        BotAnswer::Chosen {
            bot: choose_bot(
                &ticket("t", BotMode::Random, 1500.0),
                &bots,
                &[],
                &busy,
                hour_of(NOW),
                "v"
            )
            .map(|chosen| chosen.profile_id.clone())
        }
    );

    let with_a_person = vec![GameFacts {
        between_bots: false,
        ended_at: NOW,
    }];
    assert_eq!(
        apply(over_json(&BotCall::BotMatchGapPassed {
            games: with_a_person,
            now: NOW + 1,
        })),
        BotAnswer::Passed { passed: true }
    );
}

#[test]
fn the_bot_seats_decision_runs_from_the_machine_as_json() {
    jackioh_cards::register_all();
    let deck = |salt: &str| {
        jackioh_ai::build_ai_deck(
            &mut Rng::new(&format!("boundary:{salt}"), 0),
            jackioh_engine::DECK_SIZE,
            &jackioh_ai::AiDeckOptions {
                banned: Some(vec![]),
                gated_sets: Some(vec![]),
                ..jackioh_ai::AiDeckOptions::default()
            },
        )
    };
    let options = serde_json::from_value(serde_json::json!({
        "seed": "boundary",
        "decks": [deck("a"), deck("b")],
    }))
    .expect("create options");
    let begun = jackioh_engine::begin_game(&jackioh_engine::create_game(&options));
    let setup = DuelSetup {
        match_id: "match".to_string(),
        seed: "seed-boundary".to_string(),
        created_at: NOW,
        game: begun.state,
        ranked: true,
        series_game: false,
        finished: false,
        next_seq: 1,
        acked: Vec::new(),
        clocks: initial_clocks(NOW),
        bots: PerPlayer::new(
            None,
            Some(BotSeat {
                strength: 0,
                speed: 1.0,
                persona: PersonaName::Balanced,
                hidden: true,
            }),
        ),
        bot_moved_turn: PerPlayer::new(None, None),
    };
    let (mut host, _) = DuelHost::new(setup, NOW);
    let outputs = host.step(jackioh_shared::host::DuelEvent::Arm, NOW);
    let request: DecideRequest = outputs
        .into_iter()
        .find_map(|output| match output {
            jackioh_shared::host::DuelOutput::Decide { request } => Some(request),
            _ => None,
        })
        .expect("the bot owes its mulligan");
    assert_eq!((request.home, request.seat), (P2, P2));

    let call = DecideCall {
        host: Box::new(host),
        request,
    };
    // The call is JSON for a host with no Rust types; the answer is the choice the machine is fed back.
    let json = serde_json::to_string(&call).expect("a decide call serializes");
    let read: DecideCall = serde_json::from_str(&json).expect("a decide call reads back");
    let choice = decide(&read, None, None).expect("a bot plays at p2");
    assert!(choice.body.is_some(), "{choice:?}");
    assert_eq!(decide(&call, None, None), Some(choice));
    // Nobody to decide for at p1.
    let nobody = DecideCall {
        request: DecideRequest {
            home: P1,
            ..call.request.clone()
        },
        ..call
    };
    assert!(decide(&nobody, None, None).is_none());
}

#[test]
fn r1834005_the_cosmetics_calls_answer_what_the_functions_they_name_answer() {
    // A cosmetic that enough people own, long enough: the Mythic card back that drops, and the Jlorious one,
    // which no bot may wear however widely owned (R2755000).
    let long_ago = NOW - (BOT_COSMETIC_MIN_AGE_DAYS + 30) * MS_PER_DAY;
    let widely = |cosmetic: &str| CosmeticOwners {
        cosmetic_id: cosmetic.to_string(),
        card_id: None,
        owners: BOT_COSMETIC_MIN_OWNERS * 100,
        first_granted_at: Some(long_ago),
    };
    let owners = vec![widely("card-back-jlorious"), widely("card-back-nebula")];
    let call = BotCall::EligibleCosmetics {
        owners: owners.clone(),
        now: NOW,
    };
    let BotAnswer::Holdings { holdings } = apply(over_json(&call)) else {
        panic!("holdings")
    };
    assert_eq!(holdings, eligible(registry(), &owners, NOW));
    assert_eq!(holdings, vec![("card-back-nebula".to_string(), None)]);

    // A bot that has dealt its decks and has not been dressed draws everything, as the server's refresh does.
    let deck = |first: usize| BotDeck {
        cards: (first..first + 30).map(|n| format!("core-{n:03}")).collect(),
        portrait: "vanilla".to_string(),
        slots: None,
    };
    let mut undressed = bot("dressed", 1000.0);
    undressed.created_at = NOW - 40 * MS_PER_DAY;
    undressed.decks = vec![deck(1), deck(31)];
    let held: Vec<Holding> = holdings;
    let call = BotCall::RedrawCosmetics {
        seed: "pool:dressed".to_string(),
        bot: Box::new(undressed.clone()),
        eligible: held.clone(),
        now: NOW,
    };
    let BotAnswer::Dressed { draw } = apply(over_json(&call)) else {
        panic!("a draw")
    };
    let expected = redraw(
        "pool:dressed",
        &undressed,
        BOT_COSMETIC_FITS,
        registry(),
        &held,
        NOW,
    );
    assert_eq!(draw.map(|draw| *draw), expected);
    let drawn = expected.expect("a first draw");
    assert!(
        drawn
            .decks
            .is_some_and(|decks| decks.iter().all(|deck| deck.slots.is_some()))
    );

    // Dressed and nothing due: no draw. The draw is its own JSON, so a host keeps it as it was handed back.
    let mut dressed = undressed.clone();
    dressed.cosmetics = Some(drawn.cosmetics);
    dressed.decks = vec![BotDeck {
        slots: Some(Default::default()),
        ..deck(1)
    }];
    let call = BotCall::RedrawCosmetics {
        seed: "pool:again".to_string(),
        bot: Box::new(dressed),
        eligible: held,
        now: NOW,
    };
    assert_eq!(apply(over_json(&call)), BotAnswer::Dressed { draw: None });
}
