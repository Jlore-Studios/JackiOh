//! R4552000, R4552002, R4785000: the turn rows the AI's fold writes (`swing_turns`, `SwingReader`): one
//! row per turn at its start with the evaluation difference and each side's acting cards, a Created card
//! an effect put onto the field naming its maker. A turn whose start asks a question is read once the
//! start has resolved (R4785000).

use jackioh_ai::{AI_EVAL, NextSwing, SwingReader, evaluate};
use jackioh_engine::config::SWING_EVAL_SCALE;
use jackioh_engine::testkit::{
    ActingCard, ActionBody, CreatedActing, GameState, Phase, PlayerId, Scenario, Value, json,
};

use super::support::{register_cards, scenario};

const TOKEN_MAKER: &str = "core-015"; // Me and Mr Token: Cry: summon a Rush Token.
const RUSH_TOKEN: &str = "core-t-rush";
const MASK: &str = "core-065"; // Masochism Mask: Start of turn: choose one of three.
const SPIKEY_PILLOW: &str = "core-065-1"; // the token its third option summons
const SUMMON_PILLOW: &str = "summon Spikey Pillow";

fn step(reader: &mut SwingReader, s: &mut Scenario, verb: impl FnOnce(&mut Scenario)) {
    let before = s.state().clone();
    let from = s.events().len();
    verb(s);
    reader.step(&before, &s.events()[from..], s.state());
}

/// R4785000: p2's turn with `p1` as p1's side, so p2's End turn starts p1's turn on the Mask's question.
fn masked(p1: Value) -> Scenario {
    scenario(json!({
        "active": "p2",
        "p1": p1,
        "p2": { "hand": ["core-019"], "field": ["core-019"], "library": ["core-019", "core-011"] },
    }))
}

/// A row's `eval` read off `state` for `seat` (R4552000).
fn row_eval(state: &GameState, seat: PlayerId) -> i64 {
    let difference = evaluate(state, seat, NextSwing::Seat, &AI_EVAL)
        - evaluate(state, seat.opponent(), NextSwing::Enemy, &AI_EVAL);
    (difference * SWING_EVAL_SCALE).round() as i64
}

fn pillow() -> ActingCard {
    ActingCard::Created(CreatedActing {
        card: SPIKEY_PILLOW.to_string(),
        maker: Some(MASK.to_string()),
    })
}

#[test]
fn r4552002_a_cry_token_names_its_maker() {
    register_cards();
    let mut s = scenario(json!({ "p1": { "hand": [TOKEN_MAKER] } }));
    let mut reader = SwingReader::new();
    step(&mut reader, &mut s, |s| {
        s.play(TOKEN_MAKER, json!({}));
    });
    assert!(reader.turns()[PlayerId::P1].is_empty(), "no turn began");
    // Until p1's next turn begins. p2's turn may begin and end in the same step (p2 has nothing to do
    // and it ends by itself, R345), and each turn begun still has its row.
    while reader.turns()[PlayerId::P1].is_empty() {
        step(&mut reader, &mut s, |s| {
            s.end_turn();
        });
    }

    let turns = reader.turns();
    assert_eq!(turns[PlayerId::P2].len(), 1, "p2's turn has its row");
    let rows = &turns[PlayerId::P1];
    assert_eq!(rows.len(), 1);
    let row = &rows[0];
    assert_eq!(row.turn, s.state().turn);
    let made = ActingCard::Created(CreatedActing {
        card: RUSH_TOKEN.to_string(),
        maker: Some(TOKEN_MAKER.to_string()),
    });
    assert!(row.acting.contains(&made), "{:?}", row.acting);
    assert!(
        row.acting.contains(&ActingCard::Dealt(TOKEN_MAKER.to_string())),
        "{:?}",
        row.acting
    );
    // p2's row faces the same cards.
    assert!(turns[PlayerId::P2][0].facing.contains(&made));
    // R4552000: the raw evaluation difference, in thousandths, never a probability.
    let state = s.state();
    let difference = evaluate(state, PlayerId::P1, NextSwing::Seat, &AI_EVAL)
        - evaluate(state, PlayerId::P2, NextSwing::Enemy, &AI_EVAL);
    assert_eq!(row.eval, (difference * SWING_EVAL_SCALE).round() as i64);
}

#[test]
fn r4552002_a_created_card_a_hand_held_names_no_maker() {
    register_cards();
    let mut s = scenario(json!({
        "p1": { "hand": [{ "def": TOKEN_MAKER, "created": true }] },
    }));
    let mut reader = SwingReader::new();
    let made = s
        .hand(PlayerId::P1)
        .into_iter()
        .find(|card| card.created == Some(true))
        .expect("a Created card in hand");
    step(&mut reader, &mut s, |s| {
        s.play(made, json!({}));
    });
    step(&mut reader, &mut s, |s| {
        s.end_turn();
    });
    step(&mut reader, &mut s, |s| {
        s.end_turn();
    });
    let row = &reader.turns()[PlayerId::P1][0];
    // Its own play was the seat's decision; the token its Cry made names it as its maker all the same.
    assert!(
        row.acting.contains(&ActingCard::Created(CreatedActing {
            card: TOKEN_MAKER.to_string(),
            maker: None,
        })),
        "{:?}",
        row.acting
    );
    assert!(
        row.acting.contains(&ActingCard::Created(CreatedActing {
            card: RUSH_TOKEN.to_string(),
            maker: Some(TOKEN_MAKER.to_string()),
        })),
        "{:?}",
        row.acting
    );
}

#[test]
fn r4552002_a_copy_an_effect_casts_names_the_card_that_cast_it() {
    const WARDRUM: &str = "classicplus-037"; // End of turn: cast a copy of a random non-Unit you played this turn.
    const MANA_WELL: &str = "core-006"; // a Field Spell
    const FILLER: &str = "core-005";
    register_cards();
    let library = [FILLER; 8];
    let mut s = scenario(json!({
        "p1": { "hand": [MANA_WELL], "field": [WARDRUM], "library": library, "mana": 6 },
        "p2": { "hand": [FILLER], "library": library },
    }));
    let mut reader = SwingReader::new();
    step(&mut reader, &mut s, |s| {
        s.play(MANA_WELL, json!({}));
    });
    // p1's end of turn: Wardrum casts a copy of the Mana Well, its only non-Unit, into the backrow.
    while reader.turns()[PlayerId::P1].is_empty() {
        step(&mut reader, &mut s, |s| {
            s.end_turn();
        });
    }
    let row = &reader.turns()[PlayerId::P1][0];
    assert!(
        row.acting.contains(&ActingCard::Created(CreatedActing {
            card: MANA_WELL.to_string(),
            maker: Some(WARDRUM.to_string()),
        })),
        "{:?}",
        row.acting
    );
    assert!(
        row.acting.contains(&ActingCard::Dealt(MANA_WELL.to_string())),
        "{:?}",
        row.acting
    );
}

#[test]
fn r4785000_a_turn_whose_start_asks_has_its_row_read_once_the_start_resolved() {
    register_cards();
    let mut s = masked(json!({
        "hand": ["core-019"],
        "backrow": [{ "def": MASK }],
        "library": ["core-019", "core-011", "core-061"],
    }));
    let mut reader = SwingReader::new();
    step(&mut reader, &mut s, |s| {
        s.end_turn();
    });
    assert!(
        s.state().pending.is_some(),
        "the Mask asks at the start of p1's turn"
    );
    assert!(
        reader.turns()[PlayerId::P1].is_empty(),
        "p1's start is still asking: no row yet"
    );
    step(&mut reader, &mut s, |s| {
        s.answer(json!(SUMMON_PILLOW));
    });
    let state = s.state();
    assert!(state.pending.is_none());
    assert_eq!(state.phase, Phase::Main, "the start has resolved");
    assert_eq!(s.hand(PlayerId::P1).len(), 2, "the turn's draw has landed");

    let rows = &reader.turns()[PlayerId::P1];
    assert_eq!(rows.len(), 1, "one row for p1's turn");
    assert_eq!(rows[0].turn, state.turn);
    assert!(
        rows[0].acting.contains(&pillow()),
        "the row lists the cards acting once the start resolved: {:?}",
        rows[0].acting
    );
    assert_eq!(rows[0].eval, row_eval(state, PlayerId::P1));
}

#[test]
fn r4785000_a_start_that_asks_twice_is_read_after_the_second_answer() {
    register_cards();
    // The Radiant face chooses twice.
    let mut s = masked(json!({
        "hand": ["core-019"],
        "backrow": [{ "def": MASK, "radiant": true }],
        "library": ["core-019", "core-011", "core-061"],
    }));
    let mut reader = SwingReader::new();
    step(&mut reader, &mut s, |s| {
        s.end_turn();
    });
    step(&mut reader, &mut s, |s| {
        s.answer(json!(SUMMON_PILLOW));
    });
    assert!(s.state().pending.is_some(), "the Radiant Mask asks a second time");
    assert!(
        reader.turns()[PlayerId::P1].is_empty(),
        "p1's start is still asking: no row yet"
    );
    step(&mut reader, &mut s, |s| {
        s.answer(json!(SUMMON_PILLOW));
    });
    assert!(s.state().pending.is_none());
    let rows = &reader.turns()[PlayerId::P1];
    assert_eq!(rows.len(), 1, "one row for p1's turn");
    let pillows = rows[0].acting.iter().filter(|card| **card == pillow()).count();
    assert_eq!(pillows, 2, "{:?}", rows[0].acting);
}

#[test]
fn r4785000_a_turn_that_ends_with_its_answer_is_read_off_the_state_the_answer_left() {
    register_cards();
    // Silly Silas costs 6 and p1 has 4 mana, so after its draw p1 has nothing to do and the turn ends by
    // itself (R345): the answer that resolves p1's start also ends p1's turn and begins p2's.
    let mut s = masked(json!({
        "backrow": [{ "def": MASK }],
        "library": ["core-052", "core-052", "core-052"],
    }));
    let mut reader = SwingReader::new();
    step(&mut reader, &mut s, |s| {
        s.end_turn();
    });
    step(&mut reader, &mut s, |s| {
        s.answer(json!("exile bottom"));
    });
    let state = s.state();
    assert_eq!(state.active, PlayerId::P2, "p1's turn ended by itself");
    assert!(state.pending.is_none());

    let turns = reader.turns();
    assert_eq!(turns[PlayerId::P1].len(), 1, "one row for p1's turn");
    assert_eq!(turns[PlayerId::P2].len(), 1, "one row for p2's turn");
    assert_eq!(turns[PlayerId::P2][0].turn, turns[PlayerId::P1][0].turn + 1);
    assert_eq!(turns[PlayerId::P1][0].eval, row_eval(state, PlayerId::P1));
    assert_eq!(turns[PlayerId::P2][0].eval, row_eval(state, PlayerId::P2));
}

#[test]
fn r4785000_a_game_that_ends_inside_a_turns_start_writes_no_row_for_it() {
    register_cards();
    let mut s = masked(json!({
        "hand": ["core-019"],
        "backrow": [{ "def": MASK }],
        "library": ["core-019", "core-011", "core-061"],
    }));
    let mut reader = SwingReader::new();
    step(&mut reader, &mut s, |s| {
        s.end_turn();
    });
    assert!(
        s.state().pending.is_some(),
        "the Mask asks at the start of p1's turn"
    );
    step(&mut reader, &mut s, |s| {
        s.act(ActionBody::Concede, PlayerId::P1, "p1 concedes");
    });
    assert!(s.state().result.is_some(), "the game is over");
    assert!(
        reader.turns()[PlayerId::P1].is_empty(),
        "p1's start never resolved: no row"
    );
    assert!(reader.turns()[PlayerId::P2].is_empty());
}
