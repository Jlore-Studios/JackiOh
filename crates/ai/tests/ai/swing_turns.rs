//! R4552000, R4552002: the turn rows the AI's fold writes (`swing_turns`, `SwingReader`): one row per
//! turn at its start with the evaluation difference and each side's acting cards, a Created card an
//! effect put onto the field naming its maker.

use jackioh_ai::{AI_EVAL, NextSwing, SwingReader, evaluate};
use jackioh_engine::config::SWING_EVAL_SCALE;
use jackioh_engine::testkit::{ActingCard, CreatedActing, PlayerId, Scenario, json};

use super::support::{register_cards, scenario};

const TOKEN_MAKER: &str = "core-015"; // Me and Mr Token: Cry: summon a Rush Token.
const RUSH_TOKEN: &str = "core-t-rush";

fn step(reader: &mut SwingReader, s: &mut Scenario, verb: impl FnOnce(&mut Scenario)) {
    let before = s.state().clone();
    let from = s.events().len();
    verb(s);
    reader.step(&before, &s.events()[from..], s.state());
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
