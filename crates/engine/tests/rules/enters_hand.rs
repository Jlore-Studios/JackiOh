//! The `enters_hand` hook (docs/meditative-set.md, group B's Systems; MD-B18, R925): "When this
//! enters your hand" runs on every arrival in a hand through §2.4's add-to-hand — a draw, an add or
//! a Discover, a bounce or return, a gift or a steal off the field, the opening deal and the
//! mulligan's replacements — after R151's start-of-game clause. A card a full hand burns never
//! entered it, and a deck arrival never runs it. The `arrives` rider on `transform_random` runs the
//! replacement's own arrival hooks (Meditative #37).

use jackioh_engine::effects::{add_to_hand, bounce, draw, shuffle_into, transform_random};
use jackioh_engine::testkit::*;

use crate::rules::fixtures::catalog::vanilla_deck;
use crate::rules::fixtures::combat::plain;
use crate::rules::fixtures::harness::{in_hand, put, set_library, slot};
use crate::rules::fixtures::instance_data::{bottle, greeter, instance_game, opener};

const P1: PlayerId = PlayerId::P1;

fn game(seed: &str) -> GameState {
    let mut state = instance_game(seed, None);
    state.turn = 5;
    state.active = P1;
    state.phase = Phase::Main;
    state
}

/// The effect applied for p1 on a sink over `state`, the rng cursor written back; its events.
fn run(state: &mut GameState, effect: Effect) -> Vec<GameEvent> {
    let mut events = Vec::new();
    let mut rng = Rng::new(&state.seed, state.rng_cursor);
    {
        let mut sink = EngineSink::new(state, &mut events, &mut rng);
        let mut ctx = make_context(
            &mut sink,
            None,
            HookOptions {
                controller: Some(P1),
                ..HookOptions::default()
            },
        );
        (effect.apply)(&mut ctx);
    }
    state.rng_cursor = rng.cursor();
    events
}

fn live(state: &GameState, id: &str) -> CardInstance {
    find_instance(state, id)
        .cloned()
        .unwrap_or_else(|| panic!("no card {id} in the state"))
}

fn first(cards: Vec<CardInstance>) -> CardInstance {
    cards.into_iter().next().expect("a card")
}

fn is_chinese(state: &GameState, id: &str) -> bool {
    live(state, id).chinese == Some(true)
}

mod r925_enters_hand_md_b18 {
    use super::*;

    #[test]
    fn r925_a_draw_an_add_and_a_bounce_run_the_clause() {
        // A draw: the greeter on top of the library arrives Chinese.
        let mut state = game("enters-draw");
        set_library(&mut state, P1, &[greeter.id.as_str()]);
        run(&mut state, draw(json_as(json!({ "count": 1 }))));
        let drawn = state.players.p1.hand.last().cloned().expect("the drawn card");
        assert_eq!(drawn.def_id, greeter.id);
        assert!(is_chinese(&state, &drawn.id));

        // An add: a fresh greeter added to the hand arrives Chinese.
        let mut state = game("enters-add");
        run(&mut state, add_to_hand(json_as(json!({ "defId": greeter.id }))));
        let added = state.players.p1.hand.last().cloned().expect("the added card");
        assert_eq!(added.def_id, greeter.id);
        assert!(is_chinese(&state, &added.id));

        // A bounce: the greeter returns from the field to the hand Chinese.
        let mut state = game("enters-bounce");
        let fielded = put(&mut state, &greeter.id, slot(P1, Row::Units, 1), json!({}));
        assert!(!is_chinese(&state, &fielded.id));
        run(
            &mut state,
            bounce(json_as(
                json!({ "target": { "of": "instance", "instanceId": fielded.id } }),
            )),
        );
        let back = live(&state, &fielded.id);
        assert!(matches!(back.zone, Zone::Hand { .. }));
        assert!(is_chinese(&state, &back.id));
    }

    #[test]
    fn r925_a_shuffle_into_the_deck_and_a_burn_do_not() {
        // A deck arrival runs no hand clause: the greeter shuffled in stays as printed.
        let mut state = game("enters-deck");
        set_library(&mut state, P1, &[] as &[&str]);
        run(
            &mut state,
            shuffle_into(json_as(json!({ "defId": greeter.id, "count": 1 }))),
        );
        let shuffled = state
            .players
            .p1
            .library
            .last()
            .cloned()
            .expect("the shuffled card");
        assert_eq!(shuffled.def_id, greeter.id);
        assert!(!is_chinese(&state, &shuffled.id));

        // A burn never entered the hand: the greeter a full hand refuses reaches the graveyard
        // as printed.
        let mut state = game("enters-burn");
        in_hand(&mut state, &plain.id, P1, HAND_CAP);
        run(&mut state, add_to_hand(json_as(json!({ "defId": greeter.id }))));
        assert_eq!(state.players.p1.hand.len() as i32, HAND_CAP);
        let burned = state
            .players
            .p1
            .graveyard
            .last()
            .cloned()
            .expect("the burned card");
        assert_eq!(burned.def_id, greeter.id);
        assert!(!is_chinese(&state, &burned.id));
    }

    #[test]
    fn r925_the_opening_deal_runs_it() {
        // The greeter first in p1's deck: over 40 seeds, every dealt greeter is Chinese, and some
        // seed deals it.
        let mut dealt = 0;
        for n in 0..40 {
            let seed = format!("enters-deal-{n}");
            let mut deck = vec![greeter.id.clone()];
            deck.extend(vanilla_deck(DECK_SIZE - 1, 1));
            let state = begin_game(&instance_game(&seed, Some((deck, vanilla_deck(DECK_SIZE, 21))))).state;
            let greeters: Vec<&CardInstance> = state
                .players
                .p1
                .hand
                .iter()
                .filter(|card| card.def_id == greeter.id)
                .collect();
            if greeters.is_empty() {
                continue;
            }
            dealt += 1;
            for dealt_greeter in greeters {
                assert_eq!(dealt_greeter.chinese, Some(true), "{seed}");
            }
        }
        assert!(dealt > 0, "some seed deals the greeter over 40 tries");
    }

    #[test]
    fn r925_arrives_runs_the_new_card_s_arrival_hooks() {
        // Adding the bottle replaces it in place: the opener lands at its index, Radiant, Chinese,
        // costing (0), and having run its own start-of-game clause (R151).
        let mut state = game("enters-arrives");
        let hand_before = state.players.p1.hand.len();
        run(&mut state, add_to_hand(json_as(json!({ "defId": bottle.id }))));
        assert_eq!(state.players.p1.hand.len(), hand_before + 1);
        let new_card = state.players.p1.hand.last().cloned().expect("the replacement");
        assert_eq!(new_card.def_id, opener.id);
        assert!(new_card.radiant);
        assert_eq!(new_card.chinese, Some(true));
        assert_eq!(new_card.cost_override, Some(0));
        assert_eq!(new_card.memory.get("opened"), Some(&Value::Bool(true)));

        // Without `arrives`, the same replacement lands but its own clauses do not run: the
        // opener arrives with no memory of `opened`.
        let mut state = game("enters-no-arrives");
        let held = first(in_hand(&mut state, &bottle.id, P1, 1));
        run(
            &mut state,
            transform_random(json_as(json!({
                "target": { "of": "instance", "instanceId": held.id },
                "query": { "defId": ["id-opener"] },
                "radiant": true,
                "chinese": true,
                "costOverride": 0,
            }))),
        );
        let plain_replacement = state.players.p1.hand.last().cloned().expect("the replacement");
        assert_eq!(plain_replacement.def_id, opener.id);
        assert!(plain_replacement.radiant);
        assert_eq!(plain_replacement.chinese, Some(true));
        assert_eq!(plain_replacement.cost_override, Some(0));
        assert_eq!(plain_replacement.memory.get("opened"), None);
    }
}
