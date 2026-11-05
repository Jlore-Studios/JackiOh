//! The JSON boundary (`src/series.rs`'s `SeriesCall` and `SeriesAnswer`, README.md's host contract): a host
//! holds the row as JSON, hands it back on every call, and writes what the answer says. Each call is made
//! here as the browser and Roblox make it, as JSON, and held to the function it names.

use jackioh_engine::{GameOverReason, PlayerId, Winner};
use jackioh_shared::config::{SERIES_PICK_SECONDS, SERIES_WINS_NEEDED};
use jackioh_shared::deck::FrozenDeck;
use jackioh_shared::series::{
    NewSeriesInput, SeriesAnswer, SeriesCall, SeriesRow, apply, game_ended, game_seats, new_series,
    pick_deck, project_series,
};
use serde_json::{Value, json};

const NOW: i64 = 1_700_000_000_000;
const ALICE: &str = "profile-alice";
const BOB: &str = "profile-bob";

fn trio_json(owner: &str) -> Value {
    let deck = |slot: i64| {
        json!({
            "name": format!("{owner} deck {slot}"),
            "cards": [format!("{owner}-card-{slot}a"), format!("{owner}-card-{slot}b")],
        })
    };
    json!({ "name": format!("{owner}'s trio"), "decks": [deck(0), deck(1), deck(2)] })
}

fn new_series_input() -> Value {
    json!({
        "seriesId": "series-1",
        "firstMatchId": "match-1",
        "sides": [
            { "profileId": ALICE, "trio": trio_json("alice") },
            { "profileId": BOB, "trio": trio_json("bob") },
        ],
        "seedBase": "seed-base",
        "catalogVersion": "v1",
        "ranked": true,
    })
}

/// A call as a host sends it and the answer as it reads it: both are JSON, nothing else crosses.
fn call(request: Value) -> Value {
    let request: SeriesCall = serde_json::from_value(request).expect("a SeriesCall");
    serde_json::to_value(apply(&request)).expect("a SeriesAnswer")
}

/// The next row a `row` answer holds, as the JSON the host stores.
fn row(answer: &Value) -> Value {
    assert!(answer.get("row").is_some(), "expected a row, got {answer}");
    answer["row"].clone()
}

fn refusal(answer: &Value) -> String {
    answer["refused"]["reason"]
        .as_str()
        .unwrap_or_else(|| panic!("expected a refusal, got {answer}"))
        .to_owned()
}

#[test]
fn a_series_played_through_json_calls_is_the_one_the_functions_play() {
    // Game 1's pick phase: the row, and the same row the function builds.
    let input: NewSeriesInput = serde_json::from_value(new_series_input()).expect("a NewSeriesInput");
    let started = call(json!({ "call": "newSeries", "input": new_series_input(), "now": NOW }));
    assert_eq!(
        row(&started),
        serde_json::to_value(new_series(&input, NOW)).unwrap()
    );
    let picking = row(&started);
    assert_eq!(picking["status"], "picking");
    assert_eq!(picking["version"], 1);
    assert_eq!(picking["pickDeadline"], NOW + SERIES_PICK_SECONDS * 1000);

    // Nothing to read yet: no game is in play, and nobody has won.
    assert_eq!(
        refusal(&call(json!({ "call": "gameSeats", "series": picking }))),
        "not_playing"
    );
    assert_eq!(
        call(json!({ "call": "seriesScore", "series": picking })),
        json!({ "score": null })
    );
    assert_eq!(
        refusal(&call(
            json!({ "call": "beginGame", "series": picking, "now": NOW })
        )),
        "picks_missing"
    );
    assert_eq!(
        refusal(&call(
            json!({ "call": "timeoutPicks", "series": picking, "now": NOW })
        )),
        "pick_open"
    );

    // Alice's pick is sealed, and the opponent sees only that it is in.
    let alice_picked = row(&call(
        json!({ "call": "pickDeck", "series": picking, "seat": "p1", "slot": 2, "now": NOW + 1, "gameNo": 1 }),
    ));
    assert_eq!(alice_picked["sides"][0]["pick"], 2);
    assert_eq!(alice_picked["version"], 2);
    assert_eq!(
        call(json!({ "call": "alreadyPicked", "series": alice_picked, "seat": "p1", "slot": 2 })),
        json!({ "picked": true })
    );
    assert_eq!(
        call(
            json!({ "call": "alreadyPicked", "series": alice_picked, "seat": "p1", "slot": 1, "gameNo": 1 })
        ),
        json!({ "picked": false })
    );
    assert_eq!(
        refusal(&call(
            json!({ "call": "pickDeck", "series": alice_picked, "seat": "p1", "slot": 0, "now": NOW + 2 })
        )),
        "pick_sealed"
    );
    assert_eq!(
        refusal(&call(
            json!({ "call": "pickDeck", "series": alice_picked, "seat": "p2", "slot": 0, "now": NOW + 2, "gameNo": 4 })
        )),
        "stale_pick"
    );
    let bobs_view = call(
        json!({ "call": "projectSeries", "series": alice_picked, "viewerProfileId": BOB, "now": NOW + 2 }),
    );
    assert_eq!(bobs_view["view"]["opponent"]["picked"], true);
    assert_eq!(bobs_view["view"]["you"]["pick"], Value::Null);
    assert_eq!(
        bobs_view["view"]["opponent"]["decks"],
        json!([{ "slot": 0, "won": false }, { "slot": 1, "won": false }, { "slot": 2, "won": false }])
    );

    // The clock runs out with Bob's pick missing: his first unwon deck is picked and the game begins.
    let timed_out = row(&call(json!({
        "call": "timeoutPicks", "series": alice_picked, "now": NOW + SERIES_PICK_SECONDS * 1000,
    })));
    assert_eq!(timed_out["status"], "playing");
    assert_eq!(timed_out["games"][0]["slots"], json!([2, 0]));
    assert_eq!(timed_out["games"][0]["first"], "p1");

    // The game in play: seated, and Alice wins it.
    let seats = call(json!({ "call": "gameSeats", "series": timed_out }));
    assert_eq!(seats["seats"]["seats"][0]["profileId"], ALICE);
    assert_eq!(seats["seats"]["seats"][0]["player"], "p1");
    assert_eq!(
        seats["seats"]["seats"][1]["deck"],
        json!(["bob-card-0a", "bob-card-0b"])
    );
    assert_eq!(seats["seats"]["seed"], "seed-base:1");
    let ended = call(json!({
        "call": "gameEnded", "series": timed_out, "winner": "p1", "reason": "hero-death",
        "now": NOW + 70_000, "newMatchId": "match-2",
    }));
    let game_2 = row(&ended);
    assert_eq!(game_2["status"], "picking");
    assert_eq!(game_2["nextMatchId"], "match-2");
    assert_eq!(game_2["sides"][0]["wins"], 1);
    let direct = game_ended(
        &serde_json::from_value::<SeriesRow>(timed_out.clone()).unwrap(),
        Winner::P1,
        GameOverReason::HeroDeath,
        NOW + 70_000,
        "match-2",
    )
    .expect("the game ends");
    assert_eq!(game_2, serde_json::to_value(&direct).unwrap());

    // Alice's winning deck is locked, and her view of it says so.
    let alices_view = call(
        json!({ "call": "projectSeries", "series": game_2, "viewerProfileId": ALICE, "now": NOW + 71_000 }),
    );
    assert_eq!(alices_view["view"]["you"]["decks"][2]["won"], true);
    assert_eq!(alices_view["view"]["winsNeeded"], SERIES_WINS_NEEDED);
    assert_eq!(
        refusal(&call(
            json!({ "call": "pickDeck", "series": game_2, "seat": "p1", "slot": 2, "now": NOW + 71_000 })
        )),
        "slot_won"
    );
    assert_eq!(
        call(json!({ "call": "projectSeries", "series": game_2, "viewerProfileId": "stranger", "now": NOW })),
        json!({ "view": null })
    );

    // Between games Bob leaves: Alice takes the series, and the one rating move is recorded on the row.
    let forfeited = row(&call(
        json!({ "call": "forfeitSeries", "series": game_2, "seat": "p2", "now": NOW + 72_000 }),
    ));
    assert_eq!(forfeited["status"], "over");
    assert_eq!(forfeited["endReason"], "forfeit");
    assert_eq!(forfeited["winner"], "p1");
    assert_eq!(
        call(json!({ "call": "seriesScore", "series": forfeited })),
        json!({ "score": 1.0 })
    );
    let rated = row(&call(json!({
        "call": "rateSeries", "series": forfeited,
        "ratingMove": { "before": [1500.0, 1500.0], "after": [1516.0, 1484.0] },
    })));
    assert_eq!(rated["ratingBefore"], json!([1500.0, 1500.0]));
    assert_eq!(rated["ratingAfter"], json!([1516.0, 1484.0]));
    assert_eq!(
        rated["version"], forfeited["version"],
        "rating rides on the ending's own write"
    );
    let unrated = row(&call(json!({ "call": "rateSeries", "series": rated })));
    assert_eq!(unrated["ratingBefore"], Value::Null);
}

#[test]
fn a_game_that_cannot_start_is_taken_off_the_record() {
    let picking = row(&call(
        json!({ "call": "newSeries", "input": new_series_input(), "now": NOW }),
    ));
    let mut playing = picking;
    for (seat, slot) in [("p1", 0), ("p2", 1)] {
        playing = row(&call(
            json!({ "call": "pickDeck", "series": playing, "seat": seat, "slot": slot, "now": NOW + 1 }),
        ));
    }
    assert_eq!(playing["status"], "playing");
    let abandoned = row(&call(
        json!({ "call": "abandonUnstarted", "series": playing, "now": NOW + 2 }),
    ));
    assert_eq!(abandoned["status"], "over");
    assert_eq!(abandoned["endReason"], "abandoned");
    assert_eq!(abandoned["games"], json!([]));
    assert_eq!(
        call(json!({ "call": "seriesScore", "series": abandoned })),
        json!({ "score": null })
    );
}

#[test]
fn the_functions_and_the_calls_name_the_same_answers() {
    let input: NewSeriesInput = serde_json::from_value(new_series_input()).expect("a NewSeriesInput");
    let mut series = new_series(&input, NOW);
    series = pick_deck(&series, PlayerId::P1, 0, NOW, None).expect("alice picks");
    series = pick_deck(&series, PlayerId::P2, 1, NOW, None).expect("bob picks");
    let seats = game_seats(&series).expect("a game in play");
    assert_eq!(
        serde_json::to_value(apply(&SeriesCall::GameSeats {
            series: series.clone()
        }))
        .unwrap(),
        json!({ "seats": serde_json::to_value(&seats).unwrap() })
    );
    let view = project_series(&series, ALICE, NOW);
    assert_eq!(
        apply(&SeriesCall::ProjectSeries {
            series,
            viewer_profile_id: ALICE.to_owned(),
            now: NOW,
        }),
        SeriesAnswer::View(view)
    );
}

#[test]
fn a_call_that_does_not_parse_is_an_error_and_no_refusal() {
    for request in [
        json!({ "call": "nope" }),
        json!({ "call": "beginGame", "now": NOW }),
        json!({ "call": "pickDeck", "series": {}, "seat": "p1", "slot": 0, "now": NOW }),
        json!({ "series": {} }),
    ] {
        assert!(
            serde_json::from_value::<SeriesCall>(request.clone()).is_err(),
            "{request}"
        );
    }
}

/// The stored shape of a frozen deck keeps a portrait's three states apart (R642): absent, `null` and chosen.
#[test]
fn a_frozen_decks_portrait_keeps_absent_null_and_chosen_apart() {
    let base = json!({ "name": "deck", "cards": ["a"] });
    for (portrait, wanted) in [
        (None, None),
        (Some(Value::Null), Some(None)),
        (Some(json!("ember")), Some(Some("ember".to_owned()))),
    ] {
        let mut stored = base.clone();
        if let Some(portrait) = portrait {
            stored["portrait"] = portrait;
        }
        let deck: FrozenDeck = serde_json::from_value(stored.clone()).expect("a FrozenDeck");
        assert_eq!(deck.portrait, wanted);
        assert_eq!(
            serde_json::to_value(&deck).unwrap(),
            stored,
            "written back as it was read"
        );
    }
}
