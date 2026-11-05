//! The ladder bots' JSON boundary (`jackioh-wasm-core`'s `bots(call)` and `bot_decide`, #3103): every
//! call answers what the function it names answers, over the plain records a host without row types hands
//! over, and the bot seat's decision runs from the machine as JSON.

use indexmap::IndexSet;
use jackioh_engine::PlayerId::{P1, P2};
use jackioh_engine::{PerPlayer, Rng};
use jackioh_shared::bots::boundary::{BotAnswer, BotCall, GameFacts, TicketFacts, apply};
use jackioh_shared::bots::fallback::{
    BotMode, bot_match_gap_passed, bot_matches, choose_bot, fallback_due_keyed, hour_of,
};
use jackioh_shared::bots::pool::{BotFacts, BotLadder, draw_new_bot, plan_grind, plan_pool, pool_target};
use jackioh_shared::config::{
    BOT_ACTIVE_HOURS_MIN, BOT_FALLBACK_MAX_SECONDS, BOT_POOL_MIN, BOT_VS_BOT_MIN_GAP_SECONDS, MS_PER_SECOND,
    PersonaName,
};
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
    let choice = decide(&read, None).expect("a bot plays at p2");
    assert!(choice.body.is_some(), "{choice:?}");
    assert_eq!(decide(&call, None), Some(choice));
    // Nobody to decide for at p1.
    let nobody = DecideCall {
        request: DecideRequest {
            home: P1,
            ..call.request.clone()
        },
        ..call
    };
    assert!(decide(&nobody, None).is_none());
}
