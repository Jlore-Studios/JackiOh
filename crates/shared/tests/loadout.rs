//! The loadout visibility rule (`src/loadout.rs`, SPEC §10.8; R1834003, R3013002), moved from the
//! server's `loadout.rs` with the functions: which cards of a seat's deck a receiver may be sent the
//! picks and stamps of, and when. These tests deal a real game from the shipped cards, so what they hold
//! is what a match's views show.

use std::collections::{BTreeMap, BTreeSet};

use jackioh_ai::{AiDeckOptions, build_ai_deck};
use jackioh_engine::{
    CreateGameOptions, DECK_SIZE, GameState, HandView, PlayerId, PlayerView, Rng, begin_game, create_game,
    view_for,
};
use jackioh_shared::loadout::{
    CardPicks, CardStamp, CardsDue, FullArtPick, Loadout, LoadoutAnswer, LoadoutCall, SeatCards, SeatLoadout,
    apply, cards_due, dealt_cards, shown_defs,
};
use serde_json::{Value, json};

type Pairs = BTreeSet<(PlayerId, String)>;

/// The deck `deal_random_deck` deals on `seed` (R258): All Random's, from every set.
fn deck(seed: &str) -> Vec<String> {
    jackioh_cards::register_all();
    build_ai_deck(
        &mut Rng::new(seed, 0),
        DECK_SIZE,
        &AiDeckOptions {
            banned: Some(vec![]),
            lean_set: None,
            gated_sets: Some(vec![]),
            strength: Some(0.0),
            ..AiDeckOptions::default()
        },
    )
}

/// A game as a match begins it: both decks dealt, the opening hands drawn.
fn begun() -> (GameState, (Vec<String>, Vec<String>)) {
    let decks = (deck("loadout-test:p1-deck"), deck("loadout-test:p2-deck"));
    let state = create_game(&CreateGameOptions {
        seed: "loadout-test".to_owned(),
        decks: decks.clone(),
        catalog: None,
        handicaps: None,
        dealt: None,
        last_boards: None,
        glitch_boards: None,
        sandbox: None,
        custom_cards: None,
        deck_marks: None,
    });
    (begin_game(&state).state, decks)
}

fn pair(seat: PlayerId, card: &str) -> (PlayerId, String) {
    (seat, card.to_owned())
}

fn stamp(card: &str, stamp: i64) -> CardStamp {
    CardStamp {
        card_id: card.to_owned(),
        stamp,
    }
}

/// A loadout whose seat-level half is the same everywhere, with `cards` and `stamps` of its own.
fn loadout(cards: Vec<CardPicks>, stamps: Vec<CardStamp>) -> Loadout {
    Loadout {
        seat: SeatLoadout {
            table: "table-default".to_owned(),
            field: "field-default".to_owned(),
            card_back: "card-back-default".to_owned(),
            foil: "foil-default".to_owned(),
            finish: "finish-default".to_owned(),
            border: "border-default".to_owned(),
        },
        cards,
        stamps,
    }
}

#[test]
fn r1834003_shown_defs_follow_the_deal_and_skip_hidden() {
    let (state, decks) = begun();
    let dealt = dealt_cards(&state);
    // Every card of both decks was dealt, each to its own seat.
    assert_eq!(dealt.len(), decks.0.len() + decks.1.len());
    for (seat, deck) in [(PlayerId::P1, &decks.0), (PlayerId::P2, &decks.1)] {
        let mut of_seat: Vec<&str> = dealt
            .values()
            .filter(|(owner, _)| *owner == seat)
            .map(|(_, def)| def.as_str())
            .collect();
        let mut listed: Vec<&str> = deck.iter().map(String::as_str).collect();
        of_seat.sort_unstable();
        listed.sort_unstable();
        assert_eq!(of_seat, listed, "{seat}");
    }
    let view = view_for(&state, PlayerId::P1);
    let HandView::Cards(hand) = &view.you.hand else {
        panic!("p1 reads its own hand");
    };
    assert!(!hand.is_empty());
    let shown = shown_defs(&view, &dealt);
    // p1's hand shows, p2's hand and both libraries do not.
    let expected: Pairs = hand
        .iter()
        .map(|card| (PlayerId::P1, card.def_id.clone()))
        .collect();
    assert_eq!(shown, expected);
    // A card that shows as something it was not dealt as (a transform), or that was not dealt
    // at all, names nothing.
    let first = hand[0].def_id.clone();
    let mut changed = dealt.clone();
    for card in hand.iter().filter(|card| card.def_id == first) {
        changed.insert(card.instance_id.clone(), (PlayerId::P1, "core-999".to_owned()));
    }
    let transformed = shown_defs(&view, &changed);
    assert!(!transformed.contains(&(PlayerId::P1, first)));
    assert_eq!(transformed.len() + 1, shown.len());
    assert!(shown_defs(&view, &BTreeMap::new()).is_empty());
}

/// R943: a card minted after the deal (a token, a copy) is no deck's, so it never names the deck of the
/// card it copies.
#[test]
fn r1834003_a_minted_card_is_dealt_from_no_deck() {
    let (mut state, decks) = begun();
    let minted = state.players[PlayerId::P1]
        .hand
        .first()
        .map(|card| card.id.clone())
        .expect("p1 holds a card");
    for card in state.players[PlayerId::P1]
        .hand
        .iter_mut()
        .filter(|card| card.id == minted)
    {
        card.created = Some(true);
    }
    let dealt = dealt_cards(&state);
    assert!(!dealt.contains_key(&minted));
    assert_eq!(dealt.len(), decks.0.len() + decks.1.len() - 1);
}

#[test]
fn r1834003_cards_due_sends_each_pair_once_and_only_entries() {
    let first = loadout(
        vec![CardPicks {
            card_id: "core-001".to_owned(),
            full_art: Some(FullArtPick::On(true)),
            ..CardPicks::default()
        }],
        Vec::new(),
    );
    let second = loadout(
        vec![CardPicks {
            card_id: "core-002".to_owned(),
            foil: Some("foil-cracked-ice".to_owned()),
            ..CardPicks::default()
        }],
        Vec::new(),
    );
    let loadouts = (first, second);
    let mut sent = Pairs::new();
    let shown: Pairs = [
        pair(PlayerId::P2, "core-002"),
        pair(PlayerId::P1, "core-001"),
        pair(PlayerId::P1, "core-003"),
    ]
    .into_iter()
    .collect();
    let due = cards_due(&shown, &loadouts, &mut sent);
    assert_eq!(
        due,
        vec![
            SeatCards {
                seat: PlayerId::P1,
                cards: vec![loadouts.0.cards[0].clone()],
                stamps: Vec::new(),
            },
            SeatCards {
                seat: PlayerId::P2,
                cards: vec![loadouts.1.cards[0].clone()],
                stamps: Vec::new(),
            },
        ]
    );
    assert_eq!(sent.len(), 3);
    // Nothing twice.
    assert!(cards_due(&shown, &loadouts, &mut sent).is_empty());
    // The seat's picks for core-002 never travel for p1's core-002.
    let mut fresh = Pairs::new();
    let theirs_only: Pairs = [pair(PlayerId::P1, "core-002")].into_iter().collect();
    assert!(cards_due(&theirs_only, &loadouts, &mut fresh).is_empty());
}

#[test]
fn r3013002_a_stamp_is_due_once_its_card_is_shown() {
    // A stamp with no picks of its own travels alone, once, and only once its card is shown.
    let loadouts = (
        loadout(Vec::new(), vec![stamp("core-001", 2)]),
        loadout(Vec::new(), Vec::new()),
    );
    let mut sent = Pairs::new();
    let unshown: Pairs = [pair(PlayerId::P1, "core-002")].into_iter().collect();
    assert!(cards_due(&unshown, &loadouts, &mut sent).is_empty());
    let shown: Pairs = [pair(PlayerId::P1, "core-001"), pair(PlayerId::P2, "core-001")]
        .into_iter()
        .collect();
    assert_eq!(
        cards_due(&shown, &loadouts, &mut sent),
        vec![SeatCards {
            seat: PlayerId::P1,
            cards: Vec::new(),
            stamps: vec![stamp("core-001", 2)],
        }]
    );
    assert!(cards_due(&shown, &loadouts, &mut sent).is_empty());
    // With picks too, the stamp rides in the same entry.
    let both = (
        loadout(
            vec![CardPicks {
                card_id: "core-001".to_owned(),
                foil: Some("foil-cracked-ice".to_owned()),
                ..CardPicks::default()
            }],
            vec![stamp("core-001", 4)],
        ),
        loadout(Vec::new(), Vec::new()),
    );
    let due = cards_due(&shown, &both, &mut Pairs::new());
    assert_eq!(due.len(), 1);
    assert_eq!(due[0].cards, both.0.cards);
    assert_eq!(due[0].stamps, vec![stamp("core-001", 4)]);
}

#[test]
fn r3013002_a_frame_without_stamps_is_unchanged() {
    let bare = loadout(Vec::new(), Vec::new());
    let json = serde_json::to_value(&bare).expect("a loadout serializes");
    assert!(json.get("stamps").is_none(), "{json}");
    let cards = SeatCards {
        seat: PlayerId::P1,
        cards: Vec::new(),
        stamps: Vec::new(),
    };
    let json = serde_json::to_value(&cards).expect("a seat's cards serialize");
    assert!(json.get("stamps").is_none(), "{json}");
    // A loadout frozen before #3013 reads back with none.
    let mut old = serde_json::to_value(&bare).expect("a loadout serializes");
    if let Some(fields) = old.as_object_mut() {
        fields.remove("stamps");
    }
    let read: Loadout = serde_json::from_value(old).expect("an older loadout reads");
    assert!(read.stamps.is_empty());
    // One with a stamp carries it.
    let worn = loadout(Vec::new(), vec![stamp("core-001", 3)]);
    let json = serde_json::to_value(&worn).expect("a loadout serializes");
    assert_eq!(
        json.get("stamps"),
        Some(&json!([{ "cardId": "core-001", "stamp": 3 }]))
    );
}

/// A call as a host sends it and the answer as it reads it: both are JSON, nothing else crosses.
fn call(request: Value) -> Value {
    let request: LoadoutCall = serde_json::from_value(request).expect("a LoadoutCall");
    serde_json::to_value(apply(&request)).expect("a LoadoutAnswer")
}

fn to_pairs(value: &Value) -> Pairs {
    serde_json::from_value(value.clone()).expect("a list of (seat, card) pairs")
}

/// The three decisions made through JSON calls, the way the browser and Roblox make them, are the ones the
/// functions make: the host holds the deal and each receiver's `sent` set between calls.
#[test]
fn r1834003_the_json_calls_answer_what_the_functions_answer() {
    let (state, _) = begun();
    let dealt_answer = call(json!({ "call": "dealtCards", "state": state }));
    let dealt: BTreeMap<String, (PlayerId, String)> =
        serde_json::from_value(dealt_answer["dealt"].clone()).expect("a deal");
    assert_eq!(dealt, dealt_cards(&state));

    let view: PlayerView = view_for(&state, PlayerId::P1);
    let shown_answer = call(json!({ "call": "shownDefs", "view": view, "dealt": dealt_answer["dealt"] }));
    let shown = to_pairs(&shown_answer["shown"]);
    assert_eq!(shown, shown_defs(&view, &dealt));
    assert!(!shown.is_empty());

    // The first card shown has picks of its own and a stamp; the host hands `sent` back on the next call.
    let (seat, card) = shown.iter().next().cloned().expect("a shown card");
    let own = CardPicks {
        card_id: card.clone(),
        foil: Some("foil-cracked-ice".to_owned()),
        ..CardPicks::default()
    };
    let (mine, theirs) = (
        loadout(vec![own.clone()], vec![stamp(&card, 2)]),
        loadout(Vec::new(), Vec::new()),
    );
    let loadouts = if seat == PlayerId::P1 {
        (mine, theirs)
    } else {
        (theirs, mine)
    };
    let request = |sent: &Value| json!({ "call": "cardsDue", "shown": shown_answer["shown"], "loadouts": loadouts, "sent": sent });
    let first = call(request(&json!([])));
    let mut expected_sent = Pairs::new();
    let expected = cards_due(&shown, &loadouts, &mut expected_sent);
    assert_eq!(
        serde_json::from_value::<CardsDue>(first["due"].clone()).expect("an answer"),
        CardsDue {
            cards: expected.clone(),
            sent: expected_sent.clone()
        }
    );
    assert_eq!(expected.len(), 1);
    assert_eq!(expected[0].cards, vec![own]);
    // Handed back, the pairs are not sent twice; a call without `sent` starts afresh.
    let again = call(request(&first["due"]["sent"]));
    assert_eq!(again["due"]["cards"], json!([]));
    assert_eq!(to_pairs(&again["due"]["sent"]), expected_sent);
    let fresh = call(json!({ "call": "cardsDue", "shown": shown_answer["shown"], "loadouts": loadouts }));
    assert_eq!(fresh, first);
    assert_eq!(
        apply(&serde_json::from_value(request(&json!([]))).expect("a call")),
        LoadoutAnswer::Due(CardsDue {
            cards: expected,
            sent: expected_sent
        })
    );
}

#[test]
fn r1834003_a_call_that_does_not_parse_is_an_error() {
    for request in [
        json!({ "call": "nope" }),
        json!({ "call": "shownDefs", "dealt": {} }),
        json!({ "call": "cardsDue", "shown": [["p3", "core-001"]], "loadouts": [] }),
        json!({ "shown": [] }),
    ] {
        assert!(
            serde_json::from_value::<LoadoutCall>(request.clone()).is_err(),
            "{request}"
        );
    }
}
