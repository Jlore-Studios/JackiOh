//! The Conquest series' pure rules (`src/api/series_rules.rs`, SPEC §9.5, R330–R337, R259–R263).
//!
//! No store, no clock and no router: every transition is a function of a row and a time, checked
//! here on hand-built rows; `series.rs` and `tests/actor/series_recovery.rs` check the same rulings
//! through the server.
//!
//! The trios are built from the owners' names ("alice", "bob"), so "the projection never carries
//! the opponent's deck names or cards" is a substring search: nothing in bob's view may contain
//! "alice".
//!
//! Rows and views are read as camelCase JSON, so cases depend on the wire shape, not the Rust field
//! types. A refused transition returns `Result<_, SeriesRefusal>`; `refusal_of` reads its `reason`.

use jackioh_engine::validator::TRIO_DECKS;
use jackioh_engine::{GameOverReason, PlayerId, Winner};
use jackioh_server::api::series_rules::{
    NewSeriesInput, RatingMove, SeriesRefusal, SeriesRefusalReason, already_picked, begin_game, both_picked,
    first_unwon, forfeit_series, game_ended, game_seats, new_series, pick_deck, project_series, rate_series,
    series_score, timeout_picks, unwon_slots, won_slots,
};
use jackioh_server::config::{SERIES_MAX_GAMES, SERIES_PICK_SECONDS, SERIES_WINS_NEEDED};
use jackioh_server::db::store::{FrozenTrio, SeriesRow};
use serde::Serialize;
use serde_json::{Value, json};

const NOW: i64 = 1_700_000_000_000;
const PICK_MS: i64 = SERIES_PICK_SECONDS * 1000;
const ALICE: &str = "profile-alice";
const BOB: &str = "profile-bob";

const P1: PlayerId = PlayerId::P1;
const P2: PlayerId = PlayerId::P2;
/// The two series seats in seat order.
const SEATS: [PlayerId; 2] = [PlayerId::P1, PlayerId::P2];

/// The JSON a row or a view serialises to, as the client sees it.
fn j<T: Serialize>(value: &T) -> Value {
    serde_json::to_value(value).expect("the value serialises")
}

/// The last element, or null for an empty or missing array.
fn last(array: &Value) -> Value {
    array
        .as_array()
        .and_then(|items| items.last())
        .cloned()
        .unwrap_or(Value::Null)
}

fn map(array: &Value, f: impl Fn(&Value) -> Value) -> Value {
    Value::Array(
        array
            .as_array()
            .map(|items| items.iter().map(&f).collect())
            .unwrap_or_default(),
    )
}

fn keys(object: &Value) -> Vec<String> {
    let mut keys: Vec<String> = object
        .as_object()
        .map(|map| map.keys().cloned().collect())
        .unwrap_or_default();
    keys.sort();
    keys
}

fn has(set: &Value, x: &Value) -> bool {
    set.as_array().is_some_and(|items| items.contains(x))
}

/// Every key `expected` names holds the same value in `actual` (objects partially, arrays whole).
fn assert_match(actual: &Value, expected: &Value) {
    fn matches(actual: &Value, expected: &Value) -> bool {
        match (actual, expected) {
            (Value::Object(have), Value::Object(want)) => want
                .iter()
                .all(|(key, value)| have.get(key).is_some_and(|got| matches(got, value))),
            (Value::Array(have), Value::Array(want)) => {
                have.len() == want.len() && have.iter().zip(want).all(|(got, value)| matches(got, value))
            }
            _ => actual == expected,
        }
    }
    assert!(matches(actual, expected), "expected {actual} to match {expected}");
}

/// A slot or a game number in whatever integer type the rules take.
fn int<T: TryFrom<i64>>(n: i64) -> T
where
    T::Error: std::fmt::Debug,
{
    T::try_from(n).expect("a slot or game number the rules' signature can hold")
}

/// `series_score` read as a number (`1`, `0.5`, `0`) or `None`.
fn score(series: &SeriesRow) -> Option<f64> {
    j(&series_score(series)).as_f64()
}

fn trio_json(owner: &str) -> Value {
    let deck = |slot: i64| {
        json!({
            "name": format!("{owner} deck {slot}"),
            "cards": [format!("{owner}-card-{slot}a"), format!("{owner}-card-{slot}b")],
        })
    };
    json!({ "name": format!("{owner}'s trio"), "decks": [deck(0), deck(1), deck(2)] })
}

fn trio(owner: &str) -> FrozenTrio {
    serde_json::from_value(trio_json(owner)).expect("a FrozenTrio")
}

/// A fresh series at `NOW`.
fn fresh() -> SeriesRow {
    let input: NewSeriesInput = serde_json::from_value(json!({
        "seriesId": "series-1",
        "firstMatchId": "match-1",
        "sides": [
            { "profileId": ALICE, "trio": trio_json("alice") },
            { "profileId": BOB, "trio": trio_json("bob") },
        ],
        "seedBase": "seed-base",
        "catalogVersion": "v1",
        "ranked": true,
    }))
    .expect("a NewSeriesInput");
    new_series(&input, NOW)
}

/// Both sides pick (when the game is not already under way), then the game ends. A side whose pick
/// the rules already made (R332) is not asked again, and the test says so when the pick it asked for
/// is not the one made.
fn play(series: &SeriesRow, slots: [i64; 2], winner: Winner, next_match_id: &str) -> SeriesRow {
    let now = NOW;
    let mut row = series.clone();
    for (index, seat) in SEATS.into_iter().enumerate() {
        let slot = slots[index];
        let view = j(&row);
        if view["status"] == "picking" {
            let made = &view["sides"][index]["pick"];
            if made.is_null() {
                row = pick_deck(&row, seat, int(slot), now, None).expect("the pick applies");
            } else if made.as_i64() != Some(slot) {
                panic!("{seat:?}'s pick was made for it: slot {made}, not {slot}");
            }
        } else {
            let game = last(&view["games"]);
            if game["slots"][index].as_i64() != Some(slot) {
                panic!("{seat:?} is already playing slot {}", game["slots"][index]);
            }
        }
    }
    let reason = if winner == Winner::Draw {
        GameOverReason::TurnCap
    } else {
        GameOverReason::HeroDeath
    };
    game_ended(&row, winner, reason, now, next_match_id).expect("the game ends")
}

/// A seeded walk for the invariant sweeps: a small LCG, so the test states its own randomness.
fn lcg(seed: u32) -> impl FnMut() -> f64 {
    let mut state = seed;
    move || {
        state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        f64::from(state) / 4_294_967_296.0
    }
}

/// The reason a transition was refused, or `None` when it applied.
fn refusal_of<T>(run: Result<T, SeriesRefusal>) -> Option<SeriesRefusalReason> {
    run.err().map(|refusal| refusal.reason)
}

/// The projection for one profile at `NOW`.
fn view_of(series: &SeriesRow, profile_id: &str) -> Value {
    match project_series(series, profile_id, NOW) {
        Some(view) => j(&view),
        None => panic!("{profile_id} is not in the series"),
    }
}

/// A rating move, as `rateSeries` takes it.
fn rating_move(before: (f64, f64), after: (f64, f64)) -> RatingMove {
    RatingMove { before, after }
}

mod r330_conquest_a_win_with_every_deck {
    use super::*;

    #[test]
    fn r330_series_wins_needed_is_the_validators_trio_decks_one_win_with_each_deck_of_a_trio() {
        assert_eq!(SERIES_WINS_NEEDED as i64, TRIO_DECKS as i64);
    }

    #[test]
    fn r330_a_deck_that_wins_is_locked_a_deck_that_lost_or_drew_may_be_picked_again() {
        let after_game1 = play(&fresh(), [0, 1], Winner::P1, "match-2");
        assert_eq!(j(&after_game1)["status"], "picking");
        assert_eq!(j(&won_slots(P1, &after_game1.games)), json!([0]));
        assert_eq!(
            j(&unwon_slots(&after_game1, P1, &after_game1.games)),
            json!([1, 2])
        );
        assert_eq!(
            refusal_of(pick_deck(&after_game1, P1, int(0), NOW, None)),
            Some(SeriesRefusalReason::SlotWon)
        );
        // Bob lost with slot 1: it is his to pick again.
        assert_eq!(
            j(&unwon_slots(&after_game1, P2, &after_game1.games)),
            json!([0, 1, 2])
        );
        assert_eq!(refusal_of(pick_deck(&after_game1, P2, int(1), NOW, None)), None);

        let alice = view_of(&after_game1, ALICE);
        assert_eq!(
            map(&alice["you"]["decks"], |deck| json!([deck["won"], deck["games"]])),
            json!([[true, 1], [false, 0], [false, 0]]),
        );
        assert_eq!(
            alice["opponent"]["decks"],
            json!([
                { "slot": 0, "won": false },
                { "slot": 1, "won": false },
                { "slot": 2, "won": false },
            ]),
        );
    }

    #[test]
    fn r330_the_side_that_has_won_with_all_three_decks_takes_the_series_decided() {
        let sweep = play(
            &play(
                &play(&fresh(), [0, 0], Winner::P1, "m2"),
                [1, 0],
                Winner::P1,
                "m3",
            ),
            [2, 0],
            Winner::P1,
            "unused",
        );
        assert_match(
            &j(&sweep),
            &json!({ "status": "over", "winner": "p1", "endReason": "decided", "endedAt": NOW }),
        );
        assert_eq!(sweep.games.len(), SERIES_WINS_NEEDED as usize);
        assert_eq!(score(&sweep), Some(1.0));

        // Three–two in five games: each side's losing decks came back until one side had all three.
        let mut row = fresh();
        row = play(&row, [0, 0], Winner::P1, "m2");
        row = play(&row, [1, 0], Winner::P2, "m3");
        row = play(&row, [1, 1], Winner::P1, "m4");
        row = play(&row, [2, 2], Winner::P2, "m5");
        assert_eq!(map(&j(&row)["sides"], |side| side["wins"].clone()), json!([2, 2]));
        // Both sides are down to their last deck, so game 5 began by itself (R332).
        assert_eq!(j(&row)["status"], "playing");
        assert_eq!(last(&j(&row)["games"])["slots"], json!([2, 1]));
        row = play(&row, [2, 1], Winner::P1, "unused");
        assert_match(
            &j(&row),
            &json!({ "status": "over", "winner": "p1", "endReason": "decided" }),
        );
        assert_eq!(row.games.len(), 2 * SERIES_WINS_NEEDED as usize - 1);
        assert_eq!(view_of(&row, BOB)["result"]["outcome"], "loss");
        assert_eq!(view_of(&row, ALICE)["result"]["outcome"], "win");
    }

    #[test]
    fn r330_a_sides_wins_always_equal_the_decks_that_won_and_a_locked_deck_is_never_played_again() {
        for run in 0..400u32 {
            let mut random = lcg(run + 1);
            let mut row = fresh();
            let mut guard = 0;
            while j(&row)["status"] != "over" && guard < SERIES_MAX_GAMES as i64 + 1 {
                guard += 1;
                if j(&row)["status"] == "picking" {
                    for (index, seat) in SEATS.into_iter().enumerate() {
                        let view = j(&row);
                        if view["status"] != "picking" || !view["sides"][index]["pick"].is_null() {
                            continue;
                        }
                        let open = unwon_slots(&row, seat, &row.games);
                        let at = (random() * open.len() as f64).floor() as usize;
                        let slot = open.get(at).copied().unwrap_or(int(0));
                        row = pick_deck(&row, seat, i32::try_from(slot).expect("a slot"), NOW, None)
                            .expect("the pick applies");
                    }
                }
                let game = last(&j(&row)["games"]);
                assert!(game["winner"].is_null(), "a game is under way");
                for (index, seat) in SEATS.into_iter().enumerate() {
                    let before = &row.games[..row.games.len().saturating_sub(1)];
                    let slot = if game["slots"][index].is_null() {
                        json!(-1)
                    } else {
                        game["slots"][index].clone()
                    };
                    assert!(
                        !has(&j(&won_slots(seat, before)), &slot),
                        "a locked deck is never played"
                    );
                }
                let roll = random();
                let winner = if roll < 0.45 {
                    Winner::P1
                } else if roll < 0.9 {
                    Winner::P2
                } else {
                    Winner::Draw
                };
                let reason = if winner == Winner::Draw {
                    GameOverReason::TurnCap
                } else {
                    GameOverReason::HeroDeath
                };
                let next_match_id = format!("m-{}", row.games.len() + 1);
                row = game_ended(&row, winner, reason, NOW, &next_match_id).expect("the game ends");
                for (index, seat) in SEATS.into_iter().enumerate() {
                    assert_eq!(
                        j(&row)["sides"][index]["wins"].as_u64(),
                        Some(won_slots(seat, &row.games).len() as u64),
                    );
                }
            }
            assert_eq!(j(&row)["status"], "over");
            assert!(row.games.len() <= SERIES_MAX_GAMES as usize);
            let decisive = j(&row)["games"]
                .as_array()
                .map(|games| games.iter().filter(|game| game["winner"] != "draw").count())
                .unwrap_or(0);
            assert!(decisive < 2 * SERIES_WINS_NEEDED as usize);
            if j(&row)["endReason"] == "decided" {
                let wins = j(&row)["sides"]
                    .as_array()
                    .map(|sides| {
                        sides
                            .iter()
                            .filter_map(|side| side["wins"].as_i64())
                            .max()
                            .unwrap_or(0)
                    })
                    .unwrap_or(0);
                assert_eq!(wins, SERIES_WINS_NEEDED as i64);
            }
        }
    }
}

mod r331_the_sealed_pick {
    use super::*;

    #[test]
    fn r331_a_pick_is_hidden_from_the_opponent_until_both_have_picked_they_see_only_that_one_is_in_r259() {
        let before = fresh();
        let picked = pick_deck(&before, P1, int(2), NOW, None).expect("the pick applies");

        // The row holds it, and its owner sees it.
        assert_eq!(j(&picked)["sides"][0]["pick"], json!(2));
        assert!(!both_picked(&picked));
        assert_eq!(view_of(&picked, ALICE)["you"]["pick"], json!(2));
        assert_eq!(view_of(&picked, ALICE)["you"]["autoPick"], json!(false));

        // Bob's view changes in exactly one bit: `opponent.picked`.
        let bob_before = view_of(&before, BOB);
        let bob_after = view_of(&picked, BOB);
        assert_eq!(bob_before["opponent"]["picked"], json!(false));
        assert_eq!(bob_after["opponent"]["picked"], json!(true));
        let mut patched = bob_after.clone();
        patched["opponent"]["picked"] = json!(false);
        assert_eq!(patched, bob_before);
        assert_eq!(keys(&bob_after["opponent"]), ["decks", "picked", "wins"]);
    }

    #[test]
    fn r331_a_pick_is_final_a_second_pick_the_same_or_another_is_refused_as_sealed() {
        let first = pick_deck(&fresh(), P1, int(0), NOW, None).expect("the pick applies");
        assert_eq!(
            refusal_of(pick_deck(&first, P1, int(1), NOW + 1, None)),
            Some(SeriesRefusalReason::PickSealed)
        );
        assert_eq!(
            refusal_of(pick_deck(&first, P1, int(0), NOW + 1, None)),
            Some(SeriesRefusalReason::PickSealed)
        );
        // What a retried request finds, before and after the game began.
        assert!(already_picked(&first, P1, int(0), None));
        assert!(!already_picked(&first, P1, int(1), None));
        assert!(!already_picked(&first, P2, int(0), None));
        let playing = pick_deck(&first, P2, int(2), NOW, None).expect("the pick applies");
        assert!(already_picked(&playing, P1, int(0), None));
        assert!(already_picked(&playing, P2, int(2), None));
        assert!(!already_picked(&playing, P2, int(1), None));
        let ended = game_ended(&playing, Winner::P1, GameOverReason::HeroDeath, NOW, "match-2")
            .expect("the game ends");
        assert!(!already_picked(&ended, P1, int(0), None));
    }

    #[test]
    fn r331_a_sealed_pick_is_found_sealed_first_even_after_the_deadline_or_for_a_locked_slot() {
        let first = pick_deck(&fresh(), P1, int(0), NOW, None).expect("the pick applies");
        assert_eq!(
            refusal_of(pick_deck(&first, P1, int(0), NOW + PICK_MS, None)),
            Some(SeriesRefusalReason::PickSealed),
        );
        assert_eq!(
            refusal_of(pick_deck(&first, P1, int(7), NOW, None)),
            Some(SeriesRefusalReason::PickSealed)
        );
    }

    #[test]
    fn r331_a_pick_naming_another_game_is_refused_as_stale_and_a_retry_is_recognised_by_its_game() {
        let after_game1 = play(&fresh(), [0, 1], Winner::P2, "match-2");
        // A late duplicate of game 1's pick of slot 0 must not become game 2's.
        assert_eq!(
            refusal_of(pick_deck(&after_game1, P1, int(0), NOW, Some(int(1)))),
            Some(SeriesRefusalReason::StalePick),
        );
        assert!(already_picked(&after_game1, P1, int(0), Some(int(1))));
        assert!(!already_picked(&after_game1, P1, int(1), Some(int(1))));
        assert!(!already_picked(&after_game1, P1, int(0), Some(int(2))));
        let picked = pick_deck(&after_game1, P1, int(2), NOW, Some(int(2))).expect("the pick applies");
        assert_eq!(j(&picked)["sides"][0]["pick"], json!(2));
        assert!(already_picked(&picked, P1, int(2), Some(int(2))));
        assert_eq!(
            refusal_of(pick_deck(&after_game1, P1, int(2), NOW, Some(int(3)))),
            Some(SeriesRefusalReason::StalePick),
        );
    }

    #[test]
    fn r331_the_pick_that_completes_both_begins_the_game_at_once_series_p1_first_in_game_1() {
        let series = pick_deck(
            &pick_deck(&fresh(), P2, int(2), NOW, None).expect("p2 picks"),
            P1,
            int(1),
            NOW + 5,
            None,
        )
        .expect("p1 picks");

        assert_eq!(j(&series)["status"], "playing");
        assert!(j(&series)["pickDeadline"].is_null());
        assert_eq!(
            map(&j(&series)["sides"], |side| side["pick"].clone()),
            json!([null, null])
        );
        assert_eq!(
            j(&series)["games"],
            json!([{ "gameNo": 1, "matchId": "match-1", "slots": [1, 2], "first": "p1", "winner": null, "reason": null }]),
        );

        let seated = game_seats(&series).expect("a game is in play");
        assert_eq!(seated.seed, "seed-base:1");
        assert_eq!(
            j(&seated.seats),
            json!([
                { "profileId": ALICE, "player": "p1", "deck": ["alice-card-1a", "alice-card-1b"] },
                { "profileId": BOB, "player": "p2", "deck": ["bob-card-2a", "bob-card-2b"] },
            ]),
        );

        let bob = view_of(&series, BOB);
        assert_eq!(bob["currentMatchId"], "match-1");
        assert_eq!(bob["gameNo"], json!(1));
        assert_eq!(
            bob["games"],
            json!([{
                "gameNo": 1,
                "matchId": "match-1",
                "yourSlot": 2,
                "opponentSlot": 1,
                "youWentFirst": false,
                "result": null,
                "reason": null,
            }]),
        );
    }

    #[test]
    fn r331_a_pick_must_be_a_whole_slot_of_the_trio_whose_deck_has_not_won_made_while_picking_and_in_time() {
        let series = fresh();
        assert_eq!(
            refusal_of(pick_deck(&series, P1, int(-1), NOW, None)),
            Some(SeriesRefusalReason::SlotOutOfRange)
        );
        assert_eq!(
            refusal_of(pick_deck(&series, P1, int(3), NOW, None)),
            Some(SeriesRefusalReason::SlotOutOfRange)
        );
        // A non-integer slot never reaches `pick_deck`: the route's body check refuses it first.

        // R333: the clock closes picking at its deadline.
        assert_eq!(
            refusal_of(pick_deck(&series, P1, int(0), NOW + PICK_MS, None)),
            Some(SeriesRefusalReason::PickClosed),
        );
        assert_eq!(
            refusal_of(pick_deck(&series, P1, int(0), NOW + PICK_MS - 1, None)),
            None
        );

        let after_game1 = play(&series, [0, 1], Winner::P2, "match-2");
        assert_eq!(
            refusal_of(pick_deck(&after_game1, P2, int(1), NOW, None)),
            Some(SeriesRefusalReason::SlotWon)
        );
        // Alice lost with slot 0, and the other side's won slot is no business of this side's pick.
        assert_eq!(refusal_of(pick_deck(&after_game1, P1, int(0), NOW, None)), None);
        assert_eq!(refusal_of(pick_deck(&after_game1, P1, int(1), NOW, None)), None);

        let playing = pick_deck(
            &pick_deck(&series, P1, int(0), NOW, None).expect("p1 picks"),
            P2,
            int(0),
            NOW,
            None,
        )
        .expect("p2 picks");
        assert_eq!(
            refusal_of(pick_deck(&playing, P1, int(1), NOW, None)),
            Some(SeriesRefusalReason::NotPicking)
        );
        assert_eq!(
            refusal_of(begin_game(&playing, NOW)),
            Some(SeriesRefusalReason::NotPicking)
        );
        assert_eq!(
            refusal_of(begin_game(&series, NOW)),
            Some(SeriesRefusalReason::PicksMissing)
        );

        let over = forfeit_series(&series, P1, NOW).expect("the forfeit applies");
        assert_eq!(
            refusal_of(pick_deck(&over, P1, int(0), NOW, None)),
            Some(SeriesRefusalReason::Over)
        );
    }
}

mod r332_the_last_deck_is_picked_for_you {
    use super::*;

    #[test]
    fn r332_a_side_with_one_deck_left_that_has_not_won_has_it_picked_when_the_pick_phase_opens() {
        let two_wins = play(
            &play(&fresh(), [0, 0], Winner::P1, "m2"),
            [1, 0],
            Winner::P1,
            "m3",
        );
        assert_eq!(j(&two_wins)["status"], "picking");
        assert_eq!(
            map(&j(&two_wins)["sides"], |side| side["pick"].clone()),
            json!([2, null])
        );
        let alice = view_of(&two_wins, ALICE);
        assert_eq!(alice["you"]["pick"], json!(2));
        assert_eq!(alice["you"]["autoPick"], json!(true));
        // Bob sees only that a pick is in, as for any pick (R331).
        assert_eq!(view_of(&two_wins, BOB)["opponent"]["picked"], json!(true));
        assert_eq!(view_of(&two_wins, BOB)["you"]["autoPick"], json!(false));
        // Alice cannot change it; the game begins when Bob picks.
        assert_eq!(
            refusal_of(pick_deck(&two_wins, P1, int(2), NOW, None)),
            Some(SeriesRefusalReason::PickSealed)
        );
        let game3 = pick_deck(&two_wins, P2, int(1), NOW, None).expect("the pick applies");
        assert_eq!(j(&game3)["status"], "playing");
        assert_eq!(j(&game3)["games"][2]["slots"], json!([2, 1]));
    }

    #[test]
    fn r332_when_both_sides_have_one_deck_left_the_game_begins_at_once_with_no_pick_phase() {
        let mut row = fresh();
        row = play(&row, [0, 0], Winner::P1, "m2");
        row = play(&row, [1, 0], Winner::P1, "m3");
        row = play(&row, [2, 0], Winner::P2, "m4");
        let before = row.clone();
        row = play(&row, [2, 1], Winner::P2, "m5");
        assert_eq!(j(&row)["status"], "playing");
        assert!(j(&row)["pickDeadline"].is_null());
        assert_eq!(
            last(&j(&row)["games"]),
            json!({ "gameNo": 5, "matchId": "m5", "slots": [2, 2], "first": "p1", "winner": null, "reason": null }),
        );
        // One write, however much it did.
        assert_eq!(
            j(&row)["version"].as_i64(),
            j(&before)["version"].as_i64().map(|version| version + 2)
        );
        assert_eq!(game_seats(&row).expect("a game is in play").seed, "seed-base:5");
    }

    #[test]
    fn r332_a_side_with_two_or_three_decks_left_still_picks_for_itself() {
        let after_game1 = play(&fresh(), [0, 0], Winner::P1, "m2");
        assert_eq!(
            map(&j(&after_game1)["sides"], |side| side["pick"].clone()),
            json!([null, null])
        );
        assert_eq!(view_of(&after_game1, ALICE)["you"]["autoPick"], json!(false));
    }
}

mod r333_the_pick_clock {
    use super::*;

    #[test]
    fn r333_each_pick_phase_runs_series_pick_seconds_from_when_it_opens() {
        assert_eq!(j(&fresh())["pickDeadline"].as_i64(), Some(NOW + PICK_MS));
        // Game 1 was picked quickly and played for a long time: game 2's clock starts when it ended.
        let playing = pick_deck(
            &pick_deck(&fresh(), P1, int(0), NOW, None).expect("p1 picks"),
            P2,
            int(0),
            NOW,
            None,
        )
        .expect("p2 picks");
        let later = NOW + 10 * PICK_MS;
        let next = game_ended(&playing, Winner::P1, GameOverReason::HeroDeath, later, "match-2")
            .expect("the game ends");
        assert_eq!(j(&next)["pickDeadline"].as_i64(), Some(later + PICK_MS));
        assert_eq!(
            view_of(&next, ALICE)["pickDeadline"].as_i64(),
            Some(later + PICK_MS)
        );
    }

    #[test]
    fn r333_at_the_deadline_a_player_who_has_not_picked_gets_their_first_deck_that_has_not_won_and_the_game_starts()
     {
        let bob_picked = pick_deck(&fresh(), P2, int(2), NOW, None).expect("the pick applies");
        let started = timeout_picks(&bob_picked, NOW + PICK_MS).expect("the clock settles");
        assert_eq!(j(&started)["status"], "playing");
        assert_eq!(j(&started)["games"][0]["slots"], json!([0, 2]));

        // Game 2: alice won with slot 0, so her first unwon deck is 1; bob lost with 1, so his is 0.
        let after_game1 = play(&fresh(), [0, 1], Winner::P1, "match-2");
        assert_eq!(j(&first_unwon(&after_game1, P1, &after_game1.games)), json!(1));
        assert_eq!(j(&first_unwon(&after_game1, P2, &after_game1.games)), json!(0));
        let alice_picked = pick_deck(&after_game1, P1, int(2), NOW, None).expect("the pick applies");
        let game2 = timeout_picks(&alice_picked, NOW + PICK_MS).expect("the clock settles");
        assert_eq!(j(&game2)["games"][1]["slots"], json!([2, 0]));
        assert_eq!(
            j(&game_seats(&game2).expect("a game is in play").seats)[0]["profileId"],
            BOB
        );
    }

    #[test]
    fn r333_a_pick_made_for_a_player_counts_the_other_is_given_a_deck_and_the_game_starts() {
        let two_wins = play(
            &play(&fresh(), [0, 0], Winner::P1, "m2"),
            [1, 0],
            Winner::P1,
            "m3",
        );
        let deadline = j(&two_wins)["pickDeadline"].as_i64().unwrap_or(0);
        let game3 = timeout_picks(&two_wins, deadline + 1).expect("the clock settles");
        assert_eq!(j(&game3)["status"], "playing");
        assert_eq!(j(&game3)["games"][2]["slots"], json!([2, 0]));
    }

    #[test]
    fn r333_if_neither_has_picked_by_the_deadline_the_series_is_abandoned_no_winner_unrated_r260() {
        let abandoned = timeout_picks(&fresh(), NOW + PICK_MS).expect("the clock settles");
        assert_match(
            &j(&abandoned),
            &json!({
                "status": "over",
                "winner": null,
                "endReason": "abandoned",
                "pickDeadline": null,
                "endedAt": NOW + PICK_MS,
            }),
        );
        assert_eq!(score(&abandoned), None);
        assert_match(
            &j(&rate_series(
                &abandoned,
                Some(rating_move((1200.0, 1000.0), (1210.0, 990.0))),
            )),
            &json!({ "ratingBefore": null, "ratingAfter": null }),
        );
        assert_eq!(
            view_of(&abandoned, BOB)["result"],
            json!({ "outcome": "abandoned", "endReason": "abandoned", "ranked": false }),
        );

        // After a played game it is still abandoned, not a result for the side that won game 1.
        let after_game1 = play(&fresh(), [0, 0], Winner::P1, "match-2");
        let later = timeout_picks(&after_game1, NOW + PICK_MS).expect("the clock settles");
        assert_match(
            &j(&later),
            &json!({ "status": "over", "winner": null, "endReason": "abandoned" }),
        );
    }

    #[test]
    fn r333_the_clock_is_not_settled_before_its_deadline_nor_outside_a_pick_phase() {
        assert_eq!(
            refusal_of(timeout_picks(&fresh(), NOW + PICK_MS - 1)),
            Some(SeriesRefusalReason::PickOpen)
        );
        let playing = pick_deck(
            &pick_deck(&fresh(), P1, int(0), NOW, None).expect("p1 picks"),
            P2,
            int(0),
            NOW,
            None,
        )
        .expect("p2 picks");
        assert_eq!(
            refusal_of(timeout_picks(&playing, NOW + 10 * PICK_MS)),
            Some(SeriesRefusalReason::NotPicking),
        );
    }
}

mod r334_draws_the_game_cap_and_forfeits {
    use super::*;

    #[test]
    fn r334_a_drawn_game_counts_for_neither_side_and_locks_neither_deck() {
        let series = play(&fresh(), [0, 1], Winner::Draw, "match-2");
        assert_eq!(j(&series)["status"], "picking");
        assert_eq!(
            map(&j(&series)["sides"], |side| side["wins"].clone()),
            json!([0, 0])
        );
        assert_match(
            &j(&series)["games"][0],
            &json!({ "winner": "draw", "reason": "turn-cap" }),
        );
        assert_eq!(refusal_of(pick_deck(&series, P1, int(0), NOW, None)), None);
        assert_eq!(refusal_of(pick_deck(&series, P2, int(1), NOW, None)), None);

        let alice = view_of(&series, ALICE);
        assert_eq!(
            map(&alice["you"]["decks"], |deck| deck["won"].clone()),
            json!([false, false, false])
        );
        assert_eq!(
            map(&alice["you"]["decks"], |deck| deck["games"].clone()),
            json!([1, 0, 0])
        );
        assert_eq!(alice["games"][0]["result"], "draw");
    }

    #[test]
    fn r334_series_max_games_leaves_room_for_two_drawn_games_in_the_longest_series() {
        // Without a draw a series is decided by game 2 × SERIES_WINS_NEEDED − 1 at the latest.
        assert_eq!(2 * SERIES_WINS_NEEDED as i64 - 1, 5);
        assert_eq!(SERIES_MAX_GAMES as i64, 7);
        assert!(SERIES_MAX_GAMES as i64 > 2 * SERIES_WINS_NEEDED as i64 - 1);
    }

    #[test]
    fn r334_at_the_cap_equal_wins_is_a_series_draw_exhausted() {
        let mut row = fresh();
        row = play(&row, [0, 0], Winner::Draw, "m2");
        row = play(&row, [0, 0], Winner::Draw, "m3");
        row = play(&row, [0, 0], Winner::Draw, "m4");
        row = play(&row, [0, 0], Winner::P1, "m5");
        row = play(&row, [1, 0], Winner::P2, "m6");
        row = play(&row, [1, 1], Winner::Draw, "m7");
        assert_eq!(j(&row)["status"], "picking");
        row = play(&row, [2, 2], Winner::Draw, "unused");
        assert_eq!(row.games.len(), SERIES_MAX_GAMES as usize);
        assert_match(
            &j(&row),
            &json!({ "status": "over", "winner": "draw", "endReason": "exhausted" }),
        );
        assert_eq!(score(&row), Some(0.5));
        assert_eq!(view_of(&row, ALICE)["result"]["outcome"], "draw");
        assert_eq!(view_of(&row, BOB)["result"]["outcome"], "draw");
    }

    #[test]
    fn r334_at_the_cap_more_wins_takes_the_series_even_one_win_to_none() {
        let mut row = fresh();
        for game in 1..SERIES_MAX_GAMES as i64 {
            row = play(&row, [0, 0], Winner::Draw, &format!("m{}", game + 1));
        }
        row = play(&row, [2, 1], Winner::P2, "unused");
        assert_match(
            &j(&row),
            &json!({ "status": "over", "winner": "p2", "endReason": "exhausted" }),
        );
        assert_eq!(map(&j(&row)["sides"], |side| side["wins"].clone()), json!([0, 1]));
        assert_eq!(score(&row), Some(0.0));
        assert_eq!(view_of(&row, BOB)["result"]["outcome"], "win");
    }

    #[test]
    fn r334_a_concede_or_a_disconnect_loses_the_game_not_the_series_r261() {
        let playing = pick_deck(
            &pick_deck(&fresh(), P1, int(0), NOW, None).expect("p1 picks"),
            P2,
            int(0),
            NOW,
            None,
        )
        .expect("p2 picks");
        let conceded =
            game_ended(&playing, Winner::P2, GameOverReason::Concede, NOW, "m2").expect("the game ends");
        assert_eq!(j(&conceded)["status"], "picking");
        assert!(j(&conceded)["winner"].is_null());
        let playing = pick_deck(
            &pick_deck(&conceded, P1, int(1), NOW, None).expect("p1 picks"),
            P2,
            int(1),
            NOW,
            None,
        )
        .expect("p2 picks");
        let disconnected =
            game_ended(&playing, Winner::P1, GameOverReason::Disconnect, NOW, "m3").expect("the game ends");
        assert_eq!(j(&disconnected)["status"], "picking");
        assert_eq!(
            map(&j(&disconnected)["sides"], |side| side["wins"].clone()),
            json!([1, 1])
        );
    }

    #[test]
    fn r334_between_games_a_player_may_forfeit_the_series_and_the_other_side_wins_it_r261() {
        let before_any_game = forfeit_series(
            &pick_deck(&fresh(), P1, int(1), NOW, None).expect("p1 picks"),
            P2,
            NOW + 1,
        )
        .expect("the forfeit applies");
        assert_match(
            &j(&before_any_game),
            &json!({
                "status": "over",
                "winner": "p1",
                "endReason": "forfeit",
                "endedAt": NOW + 1,
                "pickDeadline": null,
            }),
        );
        assert_eq!(
            map(&j(&before_any_game)["sides"], |side| side["pick"].clone()),
            json!([null, null])
        );

        let after_game1 = forfeit_series(&play(&fresh(), [0, 0], Winner::P2, "match-2"), P2, NOW)
            .expect("the forfeit applies");
        assert_match(
            &j(&after_game1),
            &json!({ "status": "over", "winner": "p1", "endReason": "forfeit" }),
        );
        assert_match(
            &view_of(&after_game1, BOB)["result"],
            &json!({ "outcome": "loss", "endReason": "forfeit" }),
        );
    }

    #[test]
    fn r334_a_forfeit_is_refused_while_a_game_is_being_played_and_after_the_series() {
        let playing = pick_deck(
            &pick_deck(&fresh(), P1, int(0), NOW, None).expect("p1 picks"),
            P2,
            int(0),
            NOW,
            None,
        )
        .expect("p2 picks");
        assert_eq!(
            refusal_of(forfeit_series(&playing, P1, NOW)),
            Some(SeriesRefusalReason::NotPicking)
        );
        let over = forfeit_series(&fresh(), P1, NOW).expect("the forfeit applies");
        assert_eq!(
            refusal_of(forfeit_series(&over, P2, NOW)),
            Some(SeriesRefusalReason::Over)
        );
        assert_eq!(
            refusal_of(game_ended(&over, Winner::P1, GameOverReason::Concede, NOW, "x")),
            Some(SeriesRefusalReason::Over),
        );
        assert_eq!(
            refusal_of(game_ended(
                &fresh(),
                Winner::P1,
                GameOverReason::Concede,
                NOW,
                "x"
            )),
            Some(SeriesRefusalReason::NotPlaying),
        );
    }
}

mod r335_seats_and_seeds {
    use super::*;

    #[test]
    fn r335_seats_alternate_by_game_number_drawn_games_counted_p1_first_in_odd_games_p2_in_even_ones() {
        let mut row = fresh();
        let mut firsts: Vec<Value> = Vec::new();
        let outcomes = [Winner::Draw, Winner::P1, Winner::Draw, Winner::P2, Winner::P1];
        for (index, outcome) in outcomes.into_iter().enumerate() {
            if j(&row)["status"] == "picking" {
                for (side, seat) in SEATS.into_iter().enumerate() {
                    let view = j(&row);
                    if view["status"] == "picking" && view["sides"][side]["pick"].is_null() {
                        let slot = first_unwon(&row, seat, &row.games).unwrap_or(int(0));
                        row = pick_deck(&row, seat, i32::try_from(slot).expect("a slot"), NOW, None)
                            .expect("the pick applies");
                    }
                }
            }
            let first = last(&j(&row)["games"])["first"].clone();
            firsts.push(if first.is_null() { json!("p1") } else { first });
            assert_eq!(
                game_seats(&row).expect("a game is in play").seed,
                format!("seed-base:{}", index + 1)
            );
            let reason = if outcome == Winner::Draw {
                GameOverReason::TurnCap
            } else {
                GameOverReason::HeroDeath
            };
            row = game_ended(&row, outcome, reason, NOW, &format!("m{}", index + 2)).expect("the game ends");
        }
        assert_eq!(Value::Array(firsts), json!(["p1", "p2", "p1", "p2", "p1"]));
    }

    #[test]
    fn r335_the_matchs_p1_is_whoever_goes_first_with_the_deck_they_picked_r259() {
        let after_game1 = play(&fresh(), [0, 0], Winner::P1, "match-2");
        let game2 = pick_deck(
            &pick_deck(&after_game1, P1, int(1), NOW, None).expect("p1 picks"),
            P2,
            int(2),
            NOW,
            None,
        )
        .expect("p2 picks");

        assert_match(
            &j(&game2)["games"][1],
            &json!({ "gameNo": 2, "matchId": "match-2", "first": "p2" }),
        );
        let seated = game_seats(&game2).expect("a game is in play");
        assert_eq!(seated.seed, "seed-base:2");
        assert_eq!(
            j(&seated.seats),
            json!([
                { "profileId": BOB, "player": "p1", "deck": ["bob-card-2a", "bob-card-2b"] },
                { "profileId": ALICE, "player": "p2", "deck": ["alice-card-1a", "alice-card-1b"] },
            ]),
        );
        assert_eq!(view_of(&game2, BOB)["games"][1]["youWentFirst"], json!(true));
        assert_eq!(view_of(&game2, ALICE)["games"][1]["youWentFirst"], json!(false));
    }
}

mod r336_what_each_side_sees {
    use super::*;

    #[test]
    fn r336_the_projection_never_carries_the_opponents_deck_names_cards_or_pending_pick_at_any_point_r259() {
        let mut states: Vec<SeriesRow> = Vec::new();
        let mut row = fresh();
        states.push(row.clone());
        row = pick_deck(&row, P1, int(2), NOW, None).expect("p1 picks");
        states.push(row.clone());
        row = pick_deck(&row, P2, int(0), NOW, None).expect("p2 picks");
        states.push(row.clone());
        row = game_ended(&row, Winner::Draw, GameOverReason::DrawAccepted, NOW, "match-2")
            .expect("the game ends");
        states.push(row.clone());
        row = pick_deck(&row, P2, int(1), NOW, None).expect("p2 picks");
        states.push(row.clone());
        row = pick_deck(&row, P1, int(1), NOW, None).expect("p1 picks");
        states.push(row.clone());
        row = game_ended(&row, Winner::P1, GameOverReason::Concede, NOW, "match-3").expect("the game ends");
        states.push(row.clone());
        // Alice wins with her third deck and has one left: it is picked for her (R332), and Bob must
        // not learn which.
        row = play(&row, [2, 2], Winner::P1, "match-4");
        assert_eq!(j(&row)["sides"][0]["pick"], json!(0));
        states.push(row.clone());
        row = pick_deck(&row, P2, int(0), NOW, None).expect("p2 picks");
        states.push(row.clone());
        row = game_ended(&row, Winner::P1, GameOverReason::HeroDeath, NOW, "unused").expect("the game ends");
        states.push(row.clone());
        assert_eq!(j(&row)["status"], "over");

        for state in &states {
            let bob = view_of(state, BOB).to_string();
            let alice = view_of(state, ALICE).to_string();
            assert!(!bob.contains("alice"), "bob's view names alice's trio: {bob}");
            assert!(!alice.contains("bob"), "alice's view names bob's trio: {alice}");
            // A pending pick is never in the other side's view, as a number or otherwise.
            let row = j(state);
            if row["status"] == "picking" {
                let opponent = &view_of(state, BOB)["opponent"];
                assert_eq!(keys(opponent), ["decks", "picked", "wins"]);
                assert_eq!(opponent["wins"], row["sides"][0]["wins"]);
                assert!(opponent["decks"].is_array());
                assert_eq!(opponent["picked"], json!(!row["sides"][0]["pick"].is_null()));
            }
        }
        assert!(project_series(&row, "a-stranger", NOW).is_none());
    }

    #[test]
    fn r336_each_side_sees_both_sides_won_decks_its_own_by_name_the_others_by_slot() {
        let mut row = fresh();
        row = play(&row, [0, 2], Winner::P1, "m2");
        row = play(&row, [1, 2], Winner::P2, "m3");
        let alice = view_of(&row, ALICE);
        assert_eq!(
            map(
                &alice["you"]["decks"],
                |deck| json!({ "slot": deck["slot"], "name": deck["name"], "won": deck["won"] })
            ),
            json!([
                { "slot": 0, "name": "alice deck 0", "won": true },
                { "slot": 1, "name": "alice deck 1", "won": false },
                { "slot": 2, "name": "alice deck 2", "won": false },
            ]),
        );
        assert_eq!(
            alice["opponent"]["decks"],
            json!([
                { "slot": 0, "won": false },
                { "slot": 1, "won": false },
                { "slot": 2, "won": true },
            ]),
        );
        assert_eq!(
            view_of(&row, BOB)["opponent"]["decks"],
            json!([
                { "slot": 0, "won": true },
                { "slot": 1, "won": false },
                { "slot": 2, "won": false },
            ]),
        );
    }

    #[test]
    fn r336_the_projection_has_exactly_the_fields_of_the_clients_series_view() {
        let over = play(
            &play(
                &play(&fresh(), [0, 0], Winner::P1, "m2"),
                [1, 1],
                Winner::P1,
                "m3",
            ),
            [2, 2],
            Winner::P1,
            "unused",
        );
        let view = view_of(&over, ALICE);
        let mut expected = vec![
            "currentMatchId",
            "gameNo",
            "games",
            "id",
            "maxGames",
            "now",
            "opponent",
            "pickDeadline",
            "ranked",
            "result",
            "status",
            "winsNeeded",
            "you",
        ];
        expected.sort_unstable();
        assert_eq!(keys(&view), expected);
        assert_eq!(
            keys(&view["you"]),
            ["autoPick", "decks", "pick", "seat", "trioName", "wins"]
        );
        assert_eq!(
            keys(&view["you"]["decks"][0]),
            ["cards", "games", "name", "slot", "won"]
        );
        assert_eq!(keys(&view["opponent"]["decks"][0]), ["slot", "won"]);
        let mut game_keys = vec![
            "gameNo",
            "matchId",
            "opponentSlot",
            "reason",
            "result",
            "youWentFirst",
            "yourSlot",
        ];
        game_keys.sort_unstable();
        assert_eq!(keys(&view["games"][0]), game_keys);
        let mut result_keys = vec!["endReason", "outcome", "ranked"];
        result_keys.sort_unstable();
        assert_eq!(keys(&view["result"]), result_keys);
        assert_match(
            &view,
            &json!({
                "winsNeeded": SERIES_WINS_NEEDED,
                "maxGames": SERIES_MAX_GAMES,
                "gameNo": 3,
                "now": NOW,
                "currentMatchId": null,
                "pickDeadline": null,
                "ranked": true,
            }),
        );
    }
}

mod r337_a_series_begun_before_conquest {
    use super::*;

    #[test]
    fn r337_a_best_of_3_row_at_one_win_each_playing_its_third_game_goes_on_as_a_conquest_series() {
        // The shape R259 wrote: game 3's decks were the last unplayed ones, picked for both sides.
        let mut legacy = j(&fresh());
        legacy["status"] = json!("playing");
        legacy["nextMatchId"] = json!("m3");
        legacy["pickDeadline"] = Value::Null;
        legacy["version"] = json!(6);
        for side in 0..2 {
            legacy["sides"][side]["wins"] = json!(1);
            legacy["sides"][side]["pick"] = Value::Null;
        }
        legacy["games"] = json!([
            { "gameNo": 1, "matchId": "match-1", "slots": [0, 1], "first": "p1", "winner": "p1", "reason": "hero-death" },
            { "gameNo": 2, "matchId": "m2", "slots": [1, 0], "first": "p2", "winner": "p2", "reason": "concede" },
            { "gameNo": 3, "matchId": "m3", "slots": [2, 2], "first": "p1", "winner": null, "reason": null },
        ]);
        let legacy: SeriesRow = serde_json::from_value(legacy).expect("a SeriesRow");
        let after =
            game_ended(&legacy, Winner::P1, GameOverReason::HeroDeath, NOW, "m4").expect("the game ends");
        // Two wins no longer end it: Alice has won with decks 0 and 2 and must still win with deck 1.
        assert_eq!(j(&after)["status"], "picking");
        assert_eq!(
            map(&j(&after)["sides"], |side| side["wins"].clone()),
            json!([2, 1])
        );
        assert_eq!(
            map(&j(&after)["sides"], |side| side["pick"].clone()),
            json!([1, null])
        );
        assert_eq!(j(&unwon_slots(&after, P2, &after.games)), json!([1, 2]));
        assert_eq!(
            map(&view_of(&after, ALICE)["you"]["decks"], |deck| deck["won"]
                .clone()),
            json!([true, false, true])
        );
    }
}

mod r259_r260_r261_what_stands_of_the_best_of_3_rulings {
    use super::*;

    #[test]
    fn r259_a_new_series_opens_game_1s_pick_phase_on_the_reserved_match_id_with_each_sides_frozen_trio() {
        let series = fresh();
        assert_match(
            &j(&series),
            &json!({
                "status": "picking",
                "games": [],
                "nextMatchId": "match-1",
                "pickDeadline": NOW + PICK_MS,
                "winner": null,
                "endReason": null,
                "ratingBefore": null,
                "ratingAfter": null,
                "createdAt": NOW,
                "updatedAt": NOW,
                "endedAt": null,
                "version": 1,
            }),
        );
        assert_eq!(
            map(&j(&series)["sides"], |side| json!([
                side["profileId"],
                side["wins"],
                side["pick"]
            ])),
            json!([[ALICE, 0, null], [BOB, 0, null]]),
        );
        assert_eq!(j(&series)["sides"][0]["trio"], j(&trio("alice")));
        let view = view_of(&series, ALICE);
        assert_eq!(view["gameNo"], json!(1));
        assert!(view["currentMatchId"].is_null());
        assert_eq!(view["you"]["seat"], "p1");
        assert_eq!(view_of(&series, BOB)["you"]["seat"], "p2");
    }

    #[test]
    fn r260_the_pick_clock_still_abandons_a_series_nobody_picks_in_unrated_now_r333() {
        assert_match(
            &j(&timeout_picks(&fresh(), NOW + PICK_MS).expect("the clock settles")),
            &json!({ "status": "over", "winner": null, "endReason": "abandoned" }),
        );
    }

    #[test]
    fn r261_a_concede_loses_the_game_and_never_the_series_by_itself_now_r334() {
        let playing = pick_deck(
            &pick_deck(&fresh(), P1, int(0), NOW, None).expect("p1 picks"),
            P2,
            int(0),
            NOW,
            None,
        )
        .expect("p2 picks");
        let row =
            game_ended(&playing, Winner::P1, GameOverReason::Concede, NOW, "m2").expect("the game ends");
        assert_eq!(j(&row)["status"], "picking");
    }
}

mod r262_how_a_series_is_rated {
    use super::*;

    #[test]
    fn r262_a_series_scores_once_for_series_p1_1_0_5_or_0_and_not_at_all_while_it_runs() {
        assert_eq!(score(&fresh()), None);
        let playing = pick_deck(
            &pick_deck(&fresh(), P1, int(0), NOW, None).expect("p1 picks"),
            P2,
            int(0),
            NOW,
            None,
        )
        .expect("p2 picks");
        assert_eq!(score(&playing), None);
        assert_eq!(
            score(&forfeit_series(&fresh(), P1, NOW).expect("the forfeit applies")),
            Some(0.0)
        );
        assert_eq!(
            score(&forfeit_series(&fresh(), P2, NOW).expect("the forfeit applies")),
            Some(1.0)
        );
    }

    #[test]
    fn r262_the_one_move_is_recorded_on_the_ending_row_without_a_second_write_and_its_rating_never_reaches_a_player_r612()
     {
        let decided = play(
            &play(
                &play(&fresh(), [0, 0], Winner::P2, "m2"),
                [1, 1],
                Winner::P2,
                "m3",
            ),
            [2, 2],
            Winner::P2,
            "unused",
        );
        let rated = rate_series(&decided, Some(rating_move((1200.0, 1000.0), (1180.5, 1019.5))));
        assert_eq!(j(&rated)["ratingBefore"], json!([1200.0, 1000.0]));
        assert_eq!(j(&rated)["ratingAfter"], json!([1180.5, 1019.5]));
        assert_eq!(j(&rated)["version"], j(&decided)["version"]);
        assert!(j(&decided)["ratingBefore"].is_null());

        assert_eq!(
            view_of(&rated, BOB)["result"],
            json!({ "outcome": "win", "endReason": "decided", "ranked": true })
        );
        assert_eq!(
            view_of(&rated, ALICE)["result"],
            json!({ "outcome": "loss", "endReason": "decided", "ranked": true })
        );
        let alice = view_of(&rated, ALICE).to_string();
        assert!(
            !alice.contains("1180.5") && !alice.contains("1200") && !alice.contains("1019.5"),
            "{alice}"
        );
        // R604: a room's series is unranked, and its row records no move.
        let mut room = j(&decided);
        room["ranked"] = json!(false);
        let room: SeriesRow = serde_json::from_value(room).expect("a SeriesRow");
        assert_match(
            &view_of(&rate_series(&room, None), BOB)["result"],
            &json!({ "ranked": false }),
        );
        // The flag rides on the view from the start, not only in `result`, so a mid-series screen
        // can say whether a forfeit moves the rating.
        assert_eq!(view_of(&fresh(), BOB)["ranked"], json!(true));
        let mut unranked = j(&fresh());
        unranked["ranked"] = json!(false);
        let unranked: SeriesRow = serde_json::from_value(unranked).expect("a SeriesRow");
        assert_eq!(view_of(&unranked, BOB)["ranked"], json!(false));
        let mut absent = j(&fresh());
        if let Some(fields) = absent.as_object_mut() {
            fields.remove("ranked");
        }
        let absent: SeriesRow = serde_json::from_value(absent).expect("a SeriesRow");
        assert_eq!(view_of(&absent, BOB)["ranked"], json!(false));
    }
}

mod r263_a_series_is_written_by_compare_and_set {
    use super::*;

    /// The transition moved the version by exactly one and stamped `updatedAt`.
    fn step(row: &mut SeriesRow, next: SeriesRow, at: i64) {
        assert_eq!(
            j(&next)["version"].as_i64(),
            j(row)["version"].as_i64().map(|version| version + 1)
        );
        assert_eq!(j(&next)["updatedAt"].as_i64(), Some(at));
        *row = next;
    }

    #[test]
    fn r263_every_transition_moves_the_version_by_exactly_one_and_stamps_updated_at() {
        let mut row = fresh();
        let next = pick_deck(&row, P1, int(0), NOW + 1, None).expect("p1 picks");
        step(&mut row, next, NOW + 1);
        let next = pick_deck(&row, P2, int(0), NOW + 2, None).expect("p2 picks");
        step(&mut row, next, NOW + 2);
        let next = game_ended(&row, Winner::P1, GameOverReason::HeroDeath, NOW + 3, "match-2")
            .expect("the game ends");
        step(&mut row, next, NOW + 3);
        let next = pick_deck(&row, P2, int(1), NOW + 4, None).expect("p2 picks");
        step(&mut row, next, NOW + 4);
        let deadline = j(&row)["pickDeadline"].as_i64().unwrap_or(0);
        let next = timeout_picks(&row, deadline).expect("the clock settles");
        step(&mut row, next, deadline);
        let at = j(&row)["updatedAt"].as_i64().unwrap_or(0) + 1;
        let next =
            game_ended(&row, Winner::P1, GameOverReason::Concede, at, "match-3").expect("the game ends");
        step(&mut row, next, at);
        let next = forfeit_series(&row, P2, at + 1).expect("the forfeit applies");
        step(&mut row, next, at + 1);
        assert_eq!(j(&row)["status"], "over");
        assert_match(
            &j(&forfeit_series(&fresh(), P1, NOW + 9).expect("the forfeit applies")),
            &json!({ "version": 2, "updatedAt": NOW + 9 }),
        );
        let picked = pick_deck(
            &pick_deck(&fresh(), P1, int(0), NOW, None).expect("p1 picks"),
            P2,
            int(0),
            NOW,
            None,
        )
        .expect("p2 picks");
        assert_eq!(j(&picked)["version"], json!(3));
    }

    #[test]
    fn r263_each_new_pick_phase_is_for_the_match_id_it_was_handed_and_the_game_is_played_under_it() {
        let next = play(&fresh(), [0, 0], Winner::P1, "match-2");
        assert_eq!(j(&next)["nextMatchId"], "match-2");
        assert!(view_of(&next, ALICE)["currentMatchId"].is_null());
        let playing = pick_deck(
            &pick_deck(&next, P1, int(1), NOW, None).expect("p1 picks"),
            P2,
            int(1),
            NOW,
            None,
        )
        .expect("p2 picks");
        assert_eq!(j(&playing)["games"][1]["matchId"], "match-2");
        assert_eq!(view_of(&playing, ALICE)["currentMatchId"], "match-2");
    }

    #[test]
    fn r263_a_transition_never_changes_the_row_it_was_given_so_a_lost_write_can_re_apply_to_a_fresh_read() {
        let series = pick_deck(&fresh(), P1, int(0), NOW, None).expect("p1 picks");
        let snapshot = j(&series);
        let _ = pick_deck(&series, P2, int(1), NOW, None);
        let _ = forfeit_series(&series, P1, NOW);
        let _ = timeout_picks(&series, NOW + PICK_MS);
        let _ = rate_series(
            &forfeit_series(&series, P1, NOW).expect("the forfeit applies"),
            Some(rating_move((1000.0, 1000.0), (990.0, 1010.0))),
        );
        assert_eq!(j(&series), snapshot);
    }
}
