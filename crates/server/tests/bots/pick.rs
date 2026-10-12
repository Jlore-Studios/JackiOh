//! R641000 (SPEC §9.5): a ladder bot's sealed picks in a Conquest series, against `series_rules.rs`'s
//! own transitions. Its rule takes the fastest deck that has not won on the play and the slowest on
//! the draw, `BOT_PICK_RULE_SHARE` of the time, and a uniform deck that has not won otherwise; its
//! pick goes through `pick_deck`, sealed as a person's is (R331); and it comes a person's delay after
//! the pick phase opens, well inside the clock (R333), never when R332 already picked for it.
//!
//! `jackioh_server::bots::pick`, pure, on series made by `new_series` and the real catalog's costs.

use jackioh_engine::config::DECK_SIZE;
use jackioh_engine::{CardDefs, GameOverReason, PlayerId, Winner, query_cost};
use jackioh_server::api::series_rules::{
    NewSeriesInput, NewSeriesSide, SeriesRefusalReason, game_ended, new_series, pick_deck, project_series,
    unwon_slots,
};
use jackioh_server::bots::pick::{bot_pick, pick_due_at, pick_is_due, pick_slot, rule_slot};
use jackioh_server::config::{
    BOT_PICK_DELAY_MAX_MS, BOT_PICK_DELAY_MIN_MS, BOT_PICK_RULE_SHARE, SERIES_PICK_SECONDS,
    SERIES_SWEEP_INTERVAL_SECONDS,
};
use jackioh_server::db::store::{FrozenDeck, FrozenTrio, SeriesRow, SeriesStatus};

/// Unit conversion, not configuration: the series constants are stated in seconds.
const MS_PER_SECOND: i64 = 1000;

/// When the series is made, in epoch ms.
const NOW: i64 = 1_800_000_000_000;

/// The person is series p1, as the fallback seats them (R335, R1453); the bot is p2.
const PERSON: &str = "person";
const BOT: &str = "bot";
const BOT_SEAT: PlayerId = PlayerId::P2;

/// The bot's trio's slots by pace: the middle deck, the slowest and the fastest.
const MIDDLE: usize = 0;
const SLOWEST: usize = 1;
const FASTEST: usize = 2;

/// Seeds the counted tests run over.
const SEEDS: usize = 2_000;

fn defs() -> &'static CardDefs {
    jackioh_cards::register_all();
    &jackioh_cards::CATALOG
}

/// A deck's pace as the rule reads it: its cards' mean cost.
fn pace(cards: &[String]) -> f64 {
    let costs: Vec<f64> = cards
        .iter()
        .filter_map(|id| defs().get(id))
        .map(|def| f64::from(query_cost(def)))
        .collect();
    costs.iter().sum::<f64>() / costs.len() as f64
}

fn frozen(name: &str, cards: Vec<String>) -> FrozenDeck {
    FrozenDeck {
        name: name.to_string(),
        cards,
        portrait: Some(Some("vanilla".to_string())),
    }
}

/// The person's trio and the bot's. The bot's three are the middle, the dearest and the cheapest of
/// the catalog's playable cards by cost; the person's share none of their cards.
fn trios() -> (FrozenTrio, FrozenTrio) {
    let size = usize::try_from(DECK_SIZE).expect("a deck size");
    let mut by_cost: Vec<(i32, String)> = defs()
        .values()
        .filter(|def| !def.token)
        .map(|def| (query_cost(def), def.id.clone()))
        .collect();
    by_cost.sort();
    let ids: Vec<String> = by_cost.into_iter().map(|(_, id)| id).collect();
    let count = ids.len();
    let middle_from = count / 2 - size / 2;
    let fastest = ids[..size].to_vec();
    let slowest = ids[count - size..].to_vec();
    let middle = ids[middle_from..middle_from + size].to_vec();
    let rest: Vec<String> = ids
        .iter()
        .filter(|id| !fastest.contains(id) && !slowest.contains(id) && !middle.contains(id))
        .cloned()
        .collect();
    let person = FrozenTrio {
        name: "Mine".to_string(),
        decks: (
            frozen("Mine one", rest[..size].to_vec()),
            frozen("Mine two", rest[size..2 * size].to_vec()),
            frozen("Mine three", rest[2 * size..3 * size].to_vec()),
        ),
    };
    let bot = FrozenTrio {
        name: "Bot trio".to_string(),
        decks: (
            frozen("Bot middle", middle),
            frozen("Bot slow", slowest),
            frozen("Bot fast", fastest),
        ),
    };
    (person, bot)
}

/// A ranked series between the person and the bot in game 1's pick phase, its clock opened at `NOW`.
fn series(trios: &(FrozenTrio, FrozenTrio), seed_base: &str) -> SeriesRow {
    new_series(
        &NewSeriesInput {
            series_id: "series-1".to_string(),
            first_match_id: "match-1".to_string(),
            sides: (
                NewSeriesSide {
                    profile_id: PERSON.to_string(),
                    trio: trios.0.clone(),
                },
                NewSeriesSide {
                    profile_id: BOT.to_string(),
                    trio: trios.1.clone(),
                },
            ),
            seed_base: seed_base.to_string(),
            catalog_version: "test".to_string(),
            ranked: true,
        },
        NOW,
    )
}

/// One game played out: both pick, it begins, `winner` takes it, and the next pick phase opens at
/// `at` under `next_match`.
fn play(
    series: &SeriesRow,
    person_slot: i32,
    bot_slot: i32,
    winner: Winner,
    at: i64,
    next_match: &str,
) -> SeriesRow {
    let picked = pick_deck(series, PlayerId::P1, person_slot, at, None).expect("the person's pick");
    let begun = pick_deck(&picked, BOT_SEAT, bot_slot, at, None).expect("the bot's pick");
    assert_eq!(begun.status, SeriesStatus::Playing);
    game_ended(&begun, winner, GameOverReason::Concede, at, next_match).expect("the game ends")
}

/// Game 3's pick phase after a drawn game 1 and a game 2 the bot won with its fastest deck.
fn at_game_3(trios: &(FrozenTrio, FrozenTrio), seed_base: &str) -> SeriesRow {
    let game_2 = play(
        &series(trios, seed_base),
        0,
        SLOWEST as i32,
        Winner::Draw,
        NOW,
        "match-2",
    );
    play(&game_2, 0, FASTEST as i32, Winner::P2, NOW, "match-3")
}

#[test]
fn r641000_the_rule_takes_the_slowest_deck_on_the_draw_and_the_fastest_on_the_play() {
    let trios = trios();
    let decks = [&trios.1.decks.0, &trios.1.decks.1, &trios.1.decks.2];
    assert!(
        pace(&decks[FASTEST].cards) < pace(&decks[MIDDLE].cards)
            && pace(&decks[MIDDLE].cards) < pace(&decks[SLOWEST].cards),
        "PREMISE: the three paces are strictly ordered"
    );

    // Game 1: series p1 goes first (R335), so the bot is on the draw and takes its slowest deck.
    let game_1 = series(&trios, "rule");
    assert_eq!(rule_slot(&game_1, BOT_SEAT, defs()), Some(SLOWEST));

    // Game 2, after a draw that locks nothing: the bot is on the play, so its fastest.
    let game_2 = play(&game_1, 0, SLOWEST as i32, Winner::Draw, NOW, "match-2");
    assert_eq!(game_2.status, SeriesStatus::Picking);
    assert_eq!(rule_slot(&game_2, BOT_SEAT, defs()), Some(FASTEST));

    // Game 3, after the bot won with its fastest deck, which is now locked (R330): on the draw
    // again, the slowest of the two left.
    let game_3 = at_game_3(&trios, "rule");
    assert_eq!(
        unwon_slots(&game_3, BOT_SEAT, &game_3.games),
        vec![MIDDLE, SLOWEST]
    );
    assert_eq!(rule_slot(&game_3, BOT_SEAT, defs()), Some(SLOWEST));
    for k in 0..200 {
        let seeded = at_game_3(&trios, &format!("locked-{k}"));
        assert_ne!(
            pick_slot(&seeded, BOT_SEAT, defs()),
            Some(FASTEST),
            "seed {k} picked a deck that has won"
        );
    }
}

#[test]
fn r641000_the_pick_follows_the_rule_about_as_often_as_its_share() {
    let trios = trios();
    let mut agreed = 0;
    let mut drawn = [0usize; 3];
    for k in 0..SEEDS {
        let row = series(&trios, &format!("share-{k}"));
        let slot = pick_slot(&row, BOT_SEAT, defs()).expect("a pick");
        assert!(unwon_slots(&row, BOT_SEAT, &row.games).contains(&slot));
        assert_eq!(
            pick_slot(&row, BOT_SEAT, defs()),
            Some(slot),
            "the same series, the same pick"
        );
        if Some(slot) == rule_slot(&row, BOT_SEAT, defs()) {
            agreed += 1;
        }
        drawn[slot] += 1;
    }
    // The rule's share, plus the third of the rest that the uniform pick lands on the rule's deck.
    let expected = BOT_PICK_RULE_SHARE + (1.0 - BOT_PICK_RULE_SHARE) / 3.0;
    let share = f64::from(agreed) / SEEDS as f64;
    assert!(
        (share - expected).abs() <= 0.05,
        "the rule's deck {share}, expected {expected}"
    );
    assert!(
        drawn.iter().all(|&count| count > 0),
        "every deck is picked sometimes: {drawn:?}"
    );
}

#[test]
fn r641000_the_bot_s_pick_goes_through_pick_deck_sealed() {
    let trios = trios();
    let row = series(&trios, "sealed");
    let slot = pick_slot(&row, BOT_SEAT, defs()).expect("a pick");
    let at = NOW + BOT_PICK_DELAY_MIN_MS;

    let picked = bot_pick(&row, BOT_SEAT, defs(), at).expect("the bot's pick");
    assert_eq!(picked.sides.1.pick, Some(slot as i64));
    assert_eq!(picked.status, SeriesStatus::Picking, "the person has not picked");
    assert_eq!(picked.version, row.version + 1);

    // The person learns only that it is in (R331, R336): no deck name, no card of the bot's.
    let view = project_series(&picked, PERSON, at).expect("the person's view");
    assert!(view.opponent.picked);
    let text = serde_json::to_string(&view).expect("a view");
    let bot_decks = [&trios.1.decks.0, &trios.1.decks.1, &trios.1.decks.2];
    for deck in bot_decks {
        assert!(!text.contains(&deck.name), "{} in {text}", deck.name);
        for card in &deck.cards {
            assert!(!text.contains(&format!("\"{card}\"")), "{card} in {text}");
        }
    }
    assert!(!text.contains(&trios.1.name), "{text}");

    // Sealed: a second pick is refused, whatever it would draw.
    let again = bot_pick(&picked, BOT_SEAT, defs(), at).expect_err("a second pick");
    assert_eq!(again.reason, SeriesRefusalReason::PickSealed);

    // The person's pick begins game 1 with the bot's deck.
    let begun = pick_deck(&picked, PlayerId::P1, 0, at, Some(1)).expect("the person's pick");
    assert_eq!(begun.status, SeriesStatus::Playing);
    assert_eq!(begun.games[0].slots, (0, slot as i64));

    // The other order: the person first, then the bot's pick begins it.
    let person_first = pick_deck(&row, PlayerId::P1, 2, at, Some(1)).expect("the person's pick");
    let begun = bot_pick(&person_first, BOT_SEAT, defs(), at).expect("the bot's pick");
    assert_eq!(begun.status, SeriesStatus::Playing);
    assert_eq!(begun.games[0].slots, (2, slot as i64));
}

#[test]
fn r641000_the_pick_waits_a_person_s_delay_inside_the_clock() {
    const {
        assert!(
            BOT_PICK_DELAY_MAX_MS + SERIES_SWEEP_INTERVAL_SECONDS * MS_PER_SECOND
                < SERIES_PICK_SECONDS * MS_PER_SECOND,
            "PREMISE: the latest pick, one sweep late, still beats the clock"
        );
        assert!(BOT_PICK_DELAY_MIN_MS < BOT_PICK_DELAY_MAX_MS);
    };
    let trios = trios();
    let (mut soonest, mut latest) = (i64::MAX, i64::MIN);
    for k in 0..SEEDS {
        let row = series(&trios, &format!("delay-{k}"));
        let opened = row.pick_deadline.expect("a pick clock") - SERIES_PICK_SECONDS * MS_PER_SECOND;
        assert_eq!(opened, NOW);
        let due = pick_due_at(&row, BOT_SEAT).expect("a pick phase");
        let delay = due - opened;
        assert!(
            (BOT_PICK_DELAY_MIN_MS..=BOT_PICK_DELAY_MAX_MS).contains(&delay),
            "seed {k}: {delay} ms"
        );
        assert_eq!(
            pick_due_at(&row, BOT_SEAT),
            Some(due),
            "the same series, the same delay"
        );
        soonest = soonest.min(delay);
        latest = latest.max(delay);
    }
    assert!(
        soonest < BOT_PICK_DELAY_MIN_MS + MS_PER_SECOND && latest > BOT_PICK_DELAY_MAX_MS - MS_PER_SECOND,
        "both ends of the window are drawn: {soonest}..{latest}"
    );

    let row = series(&trios, "due");
    let due = pick_due_at(&row, BOT_SEAT).expect("a pick phase");
    let deadline = row.pick_deadline.expect("a pick clock");
    assert!(!pick_is_due(&row, BOT_SEAT, due - 1));
    assert!(pick_is_due(&row, BOT_SEAT, due));
    assert!(pick_is_due(&row, BOT_SEAT, deadline - 1));
    // Not once the clock has run out: R333's settlement is the sweeper's then.
    assert!(!pick_is_due(&row, BOT_SEAT, deadline));
    // Not once its pick is in.
    let picked = bot_pick(&row, BOT_SEAT, defs(), due).expect("the bot's pick");
    assert!(!pick_is_due(&picked, BOT_SEAT, due));
    // Never outside a pick phase.
    let begun = pick_deck(&picked, PlayerId::P1, 0, due, None).expect("the person's pick");
    assert_eq!(pick_due_at(&begun, BOT_SEAT), None);
    assert!(!pick_is_due(&begun, BOT_SEAT, due));

    // R332: with one deck left that has not won, the pick is made for the bot, and its own is never
    // due. The bot has won with its fastest and its slowest deck.
    let game_2 = play(
        &series(&trios, "auto"),
        0,
        FASTEST as i32,
        Winner::P2,
        NOW,
        "match-2",
    );
    let game_3 = play(&game_2, 0, SLOWEST as i32, Winner::P2, NOW, "match-3");
    assert_eq!(game_3.status, SeriesStatus::Picking);
    assert_eq!(
        game_3.sides.1.pick,
        Some(MIDDLE as i64),
        "PREMISE: R332 picked for it"
    );
    let deadline = game_3.pick_deadline.expect("a pick clock");
    for at in [
        NOW,
        NOW + BOT_PICK_DELAY_MIN_MS,
        NOW + BOT_PICK_DELAY_MAX_MS,
        deadline - 1,
    ] {
        assert!(!pick_is_due(&game_3, BOT_SEAT, at), "due at {at}");
    }
}
