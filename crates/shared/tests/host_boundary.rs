//! The match host's JSON boundary (`src/host/boundary.rs`'s `HostCall` and `HostAnswer`, README.md's host
//! contract): a host holds the `DuelHost` as JSON, hands it back on every call and carries out the outputs.
//! Each call is made here as the browser and Roblox make it, as JSON, and held to the function it names.

use jackioh_ai::{AiDeckOptions, build_ai_deck};
use jackioh_engine::{DECK_SIZE, PerPlayer, PlayerId, Rng};
use jackioh_shared::host::boundary::{HostAnswer, HostCall, apply};
use jackioh_shared::host::clock::initial_clocks;
use jackioh_shared::host::{DuelHost, DuelSetup};
use serde_json::{Value, json};

const NOW: i64 = 1_700_000_000_000;

fn setup() -> DuelSetup {
    jackioh_cards::register_all();
    let deck = |salt: &str| {
        build_ai_deck(
            &mut Rng::new(&format!("boundary:{salt}"), 0),
            DECK_SIZE,
            &AiDeckOptions {
                banned: Some(vec![]),
                gated_sets: Some(vec![]),
                ..AiDeckOptions::default()
            },
        )
    };
    let options = serde_json::from_value(json!({ "seed": "boundary", "decks": [deck("a"), deck("b")] }))
        .expect("create options");
    let begun = jackioh_engine::begin_game(&jackioh_engine::create_game(&options));
    DuelSetup {
        match_id: "match-1".to_string(),
        seed: "boundary".to_string(),
        created_at: NOW,
        game: begun.state,
        ranked: false,
        series_game: false,
        finished: false,
        next_seq: 1,
        acked: Vec::new(),
        clocks: initial_clocks(NOW),
        bots: PerPlayer::new(None, None),
        bot_moved_turn: PerPlayer::new(None, None),
    }
}

/// A call as a host sends it and the answer as it reads it: both are JSON, nothing else crosses.
fn call(request: Value) -> Value {
    let request: HostCall = serde_json::from_value(request).expect("a HostCall");
    serde_json::to_value(apply(request)).expect("a HostAnswer")
}

#[test]
fn a_host_is_made_stepped_and_handed_back_as_json() {
    let made = call(json!({ "call": "newDuel", "setup": setup(), "now": NOW }));
    assert_eq!(made["answer"], "stepped");
    // The ceiling and the mulligan clock are armed at once, as data.
    let timers: Vec<&Value> = made["outputs"]
        .as_array()
        .expect("outputs")
        .iter()
        .filter(|output| output["output"] == "timer")
        .collect();
    assert!(
        timers
            .iter()
            .any(|output| output["command"]["key"]["kind"] == "ceiling"),
        "{timers:?}"
    );
    assert!(
        timers
            .iter()
            .any(|output| output["command"]["key"]["kind"] == "mulligan"),
        "{timers:?}"
    );

    // The host it answered is the next call's argument: the mulligan clock runs out, and each seat is
    // timed out under a server nonce the host appends.
    let timer = timers
        .iter()
        .find(|output| output["command"]["key"]["kind"] == "mulligan")
        .expect("the mulligan timer");
    let id = timer["command"]["id"].clone();
    let at = timer["command"]["at"].as_i64().expect("at");
    let stepped = call(json!({
        "call": "stepDuel", "host": made["host"],
        "event": { "event": "timerFired", "key": { "kind": "mulligan" }, "id": id },
        "now": at,
    }));
    assert_eq!(stepped["answer"], "stepped");
    let appended: Vec<&Value> = stepped["outputs"]
        .as_array()
        .expect("outputs")
        .iter()
        .filter(|output| output["output"] == "append")
        .collect();
    assert_eq!(
        appended.len(),
        1,
        "the first timeout is asked for, the second after its row"
    );
    assert_eq!(appended[0]["action"]["nonce"], "srv-mulligan-1");
    assert_eq!(appended[0]["seq"], 1);
    // The row is written and fed back: the answer is the ack, and the second seat's timeout follows.
    let logged = call(json!({
        "call": "stepDuel", "host": stepped["host"],
        "event": { "event": "logged", "outcome": "written" },
        "now": at,
    }));
    let next: Vec<&Value> = logged["outputs"]
        .as_array()
        .expect("outputs")
        .iter()
        .filter(|output| output["output"] == "append")
        .collect();
    assert_eq!(next.len(), 1);
    assert_eq!(next[0]["action"]["nonce"], "srv-mulligan-2");
}

#[test]
fn the_views_a_host_reads_are_the_seats_own_with_their_clock() {
    let made = call(json!({ "call": "newDuel", "setup": setup(), "now": NOW }));
    let viewed = call(json!({ "call": "duelView", "host": made["host"], "seat": "p2", "now": NOW + 1000 }));
    assert_eq!(viewed["answer"], "view");
    assert_eq!(viewed["view"]["viewer"], "p2");
    // R268: the one mulligan clock, both seats' own, 45 s less the second that passed.
    assert_eq!(viewed["view"]["clockMs"], 44_000);
    let present = call(json!({ "call": "duelPresence", "host": made["host"], "now": NOW }));
    assert_eq!(present["presence"], json!({ "p1": false, "p2": false }));
}

#[test]
fn a_host_the_json_carries_is_the_host_the_machine_made() {
    let (host, _) = DuelHost::new(setup(), NOW);
    let made = call(json!({ "call": "newDuel", "setup": setup(), "now": NOW }));
    let answer: HostAnswer = serde_json::from_value(made).expect("an answer");
    let HostAnswer::Stepped { host: carried, .. } = answer else {
        panic!("a host was made")
    };
    assert_eq!(*carried, host);
}

#[test]
fn the_rematch_calls_are_the_books_functions() {
    let refusal = call(json!({
        "call": "checkOffer",
        "facts": { "stakes": 2, "ranked": false, "finished": true, "seriesGame": false, "catalogCurrent": true },
    }));
    assert_eq!(refusal["refusal"], "double_requires_ranked");
    let allowed = call(json!({
        "call": "checkOffer",
        "facts": { "stakes": 1, "ranked": false, "finished": true, "seriesGame": false, "catalogCurrent": true },
    }));
    assert_eq!(allowed["refusal"], Value::Null);
    let window = call(json!({ "call": "rematchWindow", "finishedAt": NOW, "now": NOW + 600_000 }));
    assert_eq!(window["open"], true);
    let closed = call(json!({ "call": "rematchWindow", "finishedAt": NOW, "now": NOW + 600_001 }));
    assert_eq!(closed["open"], false);

    let book = serde_json::to_value(jackioh_shared::host::rematch::RematchBook::default()).expect("book");
    let first = call(json!({
        "call": "offer", "book": book, "matchId": "m1", "finishedAt": NOW, "seat": "p1", "stakes": 1,
        "lean": Value::Null, "now": NOW + 1, "newMatchId": "g1",
    }));
    assert_eq!(
        first["outcome"],
        json!({ "outcome": "waiting", "answered": null })
    );
    let second = call(json!({
        "call": "offer", "book": first["book"], "matchId": "m1", "finishedAt": NOW, "seat": "p2", "stakes": 1,
        "lean": "newest", "now": NOW + 2, "newMatchId": "g2",
    }));
    assert_eq!(second["outcome"]["outcome"], "claimed");
    assert_eq!(second["outcome"]["matchId"], "g2");
    let seen = call(
        json!({ "call": "seen", "book": second["book"], "matchId": "m1", "seat": "p1", "now": NOW + 3 }),
    );
    assert_eq!(seen["seen"]["matchId"], "g2");
    let released = call(json!({
        "call": "release", "book": seen["book"], "matchId": "m1",
        "generation": second["outcome"]["generation"],
    }));
    let swept =
        call(json!({ "call": "sweep", "book": released["book"], "now": NOW + 2_000_000, "live": [] }));
    assert_eq!(swept["removed"], 1);
    let _ = PlayerId::P1;
}

#[test]
fn a_call_that_does_not_parse_is_no_call() {
    for broken in [
        json!({}),
        json!({ "call": "nope" }),
        json!({ "call": "stepDuel" }),
    ] {
        assert!(serde_json::from_value::<HostCall>(broken).is_err());
    }
}

#[test]
fn a_duos_host_is_made_stepped_and_handed_back_as_json() {
    jackioh_cards::register_all();
    let deck = |salt: &str| {
        build_ai_deck(
            &mut Rng::new(&format!("duos-boundary:{salt}"), 0),
            DECK_SIZE,
            &AiDeckOptions {
                banned: Some(vec![]),
                gated_sets: Some(vec![]),
                ..AiDeckOptions::default()
            },
        )
    };
    let options = serde_json::from_value(json!({
        "seed": "duos-boundary",
        "decks": { "a1": deck("a1"), "a2": deck("a2"), "b1": deck("b1"), "b2": deck("b2") },
    }))
    .expect("create options");
    let begun = jackioh_engine::duos::duos_begin(&jackioh_engine::duos::duos_create(&options));
    let setup = jackioh_shared::host::DuosSetup {
        match_id: "duos-1".to_string(),
        created_at: NOW,
        game: begun.state,
        finished: false,
        next_seq: 1,
        acked: Vec::new(),
    };
    let made = call(json!({ "call": "newDuos", "setup": setup, "now": NOW }));
    assert_eq!(made["answer"], "steppedDuos");
    let armed = call(json!({
        "call": "stepDuos", "host": made["host"], "event": { "event": "arm" }, "now": NOW,
    }));
    let timers: Vec<&Value> = armed["outputs"]
        .as_array()
        .expect("outputs")
        .iter()
        .filter(|output| output["output"] == "timer")
        .collect();
    for kind in ["duosMulligan", "duosCeiling"] {
        assert!(
            timers
                .iter()
                .any(|output| output["command"]["key"]["kind"] == kind),
            "{kind}: {timers:?}"
        );
    }
    // The mulligan clock runs out: the people's hands are kept, by the AI, after its pause.
    let viewed =
        call(json!({ "call": "duosSeatView", "host": armed["host"], "seat": "a1", "now": NOW + 1000 }));
    assert_eq!(viewed["answer"], "duosView");
    assert_eq!(viewed["view"]["seat"], "a1");
    assert_eq!(viewed["view"]["own"]["clockMs"], 44_000);
}

#[test]
fn the_again_calls_are_the_books_functions() {
    let window = call(json!({ "call": "againWindow", "endedAt": NOW, "now": NOW + 600_000 }));
    assert_eq!(window["open"], true);
    let closed = call(json!({ "call": "againWindow", "endedAt": NOW, "now": NOW + 600_001 }));
    assert_eq!(closed["open"], false);
    let book = serde_json::to_value(jackioh_shared::host::again::AgainBook::default()).expect("book");
    let first = call(json!({
        "call": "againClaim", "book": book, "matchId": "m1", "seat": "a1", "lean": Value::Null, "now": NOW,
    }));
    assert_eq!(first["claim"], json!({ "claim": "done", "answered": null }));
    let second = call(json!({
        "call": "againClaim", "book": first["book"], "matchId": "m1", "seat": "a2", "lean": "newest", "now": NOW + 1,
    }));
    assert_eq!(second["claim"]["claim"], "make");
    let creating = second["claim"]["creating"].clone();
    let made = call(json!({
        "call": "againMade", "book": second["book"], "matchId": "m1", "creating": creating, "id": "g1", "now": NOW + 2,
    }));
    let seen = call(
        json!({ "call": "againSeen", "book": made["book"], "matchId": "m1", "seat": "a1", "now": NOW + 3 }),
    );
    assert_eq!(
        seen["seen"],
        json!({ "youOffered": true, "teammateOffered": true, "matchId": "g1" })
    );
    let released =
        call(json!({ "call": "againRelease", "book": seen["book"], "matchId": "m1", "creating": creating }));
    assert_eq!(released["answer"], "againBook");
}
