//! An unevolved card's Evolve text in the AI's evaluation (R4648000): the decision's root evolves each
//! readable card that can still evolve on a sandbox copy, every unit on either field and every Unit in the
//! seat's own hand, placed first (`evolve_prices`), and `unit_worth` adds `AI_EVAL.evolve` of what one run
//! gave, once for each time the text would run now (R3536003), times how close the card stands to
//! evolving: the share of its side's unit zones filled, of the (0) Cost cards its own hand holds of those
//! it needs, of its health lost, or `evolve_event_share` when it waits on an event. The enemy's hand is
//! never priced, hidden cards change no decision (R185), and a zero weight, the frozen baseline's, reads no
//! price.
//!
//! The cards: Lone Drifter, (1) 2/2, Radiant 4/4, "Evolves when all your Unit zones are full. / Evolve:
//! Gain +4/+4." (Radiant +8/+8); Jay Fungus, (2) 3/6 Taunt, "… Evolves when your hand has at least 3 (0)
//! Cost cards. / Evolve: Gain Poisonous."; A Wee Whelp, (1) 1/1, "Evolves after this destroys a Unit. /
//! Evolve: Set stats to 10/10."; High Evolutionary, (3) 7/7, "Aura: Your evolutions trigger twice.";
//! The Propagation's Wasp, Space Invader and Lantern Distiller, whose own attack or turn text evolves them;
//! the Felinor Token, a (1) 1/1 with no text; Hotfix, (1) "Give a friendly Unit +2/+2."; Polymorph, (3)
//! "Transform a Unit into a Sheep Token"; Rapid Replenish, a (0) Spell; Hit Job, a (3) Spell; and Mr.
//! Vanilla, a (1) 4/4 with no text (`plain`).

use jackioh_ai::*;
use jackioh_engine::testkit::*;

use super::support::{AI, HUMAN, close, register_cards, unit};

const DRIFTER: &str = "stinkyscapeplus-002";
const FUNGUS: &str = "classicplus-049";
const WHELP: &str = "stinkyscapeplus-101";
const HIGH_EVOLUTIONARY: &str = "stinkyscapeplus-003";
const WASP: &str = "stinkyscape-091";
const SPACE_INVADER: &str = "stinkyscapeplus-161";
const DISTILLER: &str = "stinkyscapeplus-t-ai-05";
const FELINOR: &str = "core-t-felinor";
const HOTFIX: &str = "infiniteassault-006";
const POLYMORPH: &str = "meditative-024";
const FREE: &str = "core-010";
const DEAR: &str = "core-016";
const VANILLA: &str = "core-008";
const LIBRARY: &[&str] = &[VANILLA, VANILLA, VANILLA, VANILLA];

/// The AI's turn with the AI's side `p1` and the human's `p2`, each with `LIBRARY` added.
fn board(seed: &str, mut p1: Value, mut p2: Value) -> GameState {
    register_cards();
    p1["library"] = json!(LIBRARY);
    p2["library"] = json!(LIBRARY);
    scenario(json!({ "seed": seed, "p1": p1, "p2": p2 }))
        .state()
        .clone()
}

/// A Mr. Vanilla set to `attack`/`health`: a body with no text and no keyword.
fn plain(attack: i32, health: i32) -> Value {
    json!({ "def": VANILLA, "statsOverride": { "attack": attack, "health": health } })
}

/// A Radiant Lone Drifter, 4/4 → 12/12.
fn drifter() -> Value {
    json!({ "def": DRIFTER, "radiant": true })
}

/// The AI's evolution prices on `state` and the nodes their probes spent.
fn priced(state: &GameState) -> (Vec<EvolvePrice>, usize) {
    let counter = create_node_counter(AI_BUDGET.nodes, None);
    let prices = evolve_prices(state, AI, &counter);
    (prices, counter.used())
}

/// `evolutions` as a decision passes them, with no other price.
fn prices_of(evolutions: Vec<EvolvePrice>) -> Prices {
    Prices {
        evolutions,
        ..Prices::default()
    }
}

/// The entry of `controller`'s `def_id`, which must be priced.
fn entry<'a>(prices: &'a [EvolvePrice], def_id: &str, controller: PlayerId) -> &'a EvolvePrice {
    prices
        .iter()
        .find(|entry| entry.def_id == def_id && entry.controller == controller)
        .unwrap_or_else(|| panic!("{controller}'s {def_id}'s evolution is priced: {prices:?}"))
}

fn eval_with(state: &GameState, prices: &Prices) -> f64 {
    evaluate_priced(state, AI, NextSwing::Enemy, &AI_EVAL, prices)
}

/// What `prices` add to the AI's evaluation of `state`.
fn diff(state: &GameState, prices: &Prices) -> f64 {
    eval_with(state, prices) - eval_with(state, Prices::NONE)
}

/// `player`'s side holding `field`, the other side nothing on its field.
fn sides(player: PlayerId, field: Vec<Value>) -> (Value, Value) {
    let side = json!({ "field": field });
    if player == AI {
        (side, json!({}))
    } else {
        (json!({}), side)
    }
}

/// A Radiant Drifter on `player`'s field beside `filled − 1` 1/1s.
fn drifter_board(player: PlayerId, filled: usize) -> GameState {
    let mut field = vec![drifter()];
    field.extend((1..filled).map(|_| plain(1, 1)));
    let (p1, p2) = sides(player, field);
    board("ai-evolve-zones", p1, p2)
}

#[test]
fn r4648000_a_drifter_counts_its_evolution_by_the_zones_its_side_has_filled() {
    for (player, sign) in [(AI, 1.0), (HUMAN, -1.0)] {
        let mut added = Vec::new();
        for filled in [4, 1] {
            let state = drifter_board(player, filled);
            let (evolutions, used) = priced(&state);
            assert_eq!(evolutions.len(), 1, "{player:?} {filled}: {evolutions:?}");
            let found = entry(&evolutions, DRIFTER, player);
            assert_eq!(found.condition, EvolveCondition::ZonesFull, "{player:?} {filled}");
            assert!(found.price > 0.0, "{player:?} {filled}: {}", found.price);
            assert!((1..=AI_EVOLVE_PROBE_NODES).contains(&used), "{used} nodes");
            let closeness = filled as f64 / f64::from(jackioh_engine::config::UNIT_ZONES);
            let prices = prices_of(evolutions.clone());
            let gain = diff(&state, &prices);
            close(gain, sign * closeness * AI_EVAL.evolve * found.price);
            added.push(gain.abs());

            // An evolved Drifter has nothing coming, and the root probes nothing.
            let mut evolved = state.clone();
            let id = unit(&evolved, player, DRIFTER).id.clone();
            find_instance_mut(&mut evolved, &id).expect("the Drifter").evolved = Some(true);
            close(eval_with(&evolved, &prices), eval_with(&evolved, Prices::NONE));
            assert_eq!(priced(&evolved), (Vec::new(), 0));
        }
        assert!(
            added[0] > added[1],
            "{player:?}: four zones {} against one {}",
            added[0],
            added[1]
        );
    }
}

#[test]
fn r4648000_an_own_jay_fungus_counts_the_free_cards_held_and_an_enemy_one_the_event_share() {
    for (seed, hand, held) in [
        ("ai-evolve-fungus-two", json!([FREE, FREE, DEAR]), 2.0),
        ("ai-evolve-fungus-none", json!([DEAR, DEAR]), 0.0),
    ] {
        let state = board(seed, json!({ "field": [FUNGUS], "hand": hand }), json!({}));
        let (evolutions, _) = priced(&state);
        let found = entry(&evolutions, FUNGUS, AI).clone();
        assert_eq!(found.condition, EvolveCondition::FreeCards(3), "{seed}");
        assert!(found.price > 0.0, "{seed}: {}", found.price);
        let prices = prices_of(evolutions);
        close(diff(&state, &prices), held / 3.0 * AI_EVAL.evolve * found.price);
    }
    // The human's Fungus: its hand is hidden, so the free cards it holds are no fact the AI has.
    let state = board(
        "ai-evolve-fungus-enemy",
        json!({}),
        json!({ "field": [FUNGUS], "hand": [FREE, FREE] }),
    );
    let (evolutions, _) = priced(&state);
    let found = entry(&evolutions, FUNGUS, HUMAN).clone();
    assert_eq!(found.condition, EvolveCondition::Event);
    assert!(found.price > 0.0, "{}", found.price);
    close(
        diff(&state, &prices_of(evolutions)),
        -AI_EVAL.evolve_event_share * AI_EVAL.evolve * found.price,
    );
}

#[test]
fn r4648000_an_event_condition_counts_at_the_event_share() {
    for (player, sign) in [(AI, 1.0), (HUMAN, -1.0)] {
        let (p1, p2) = sides(player, vec![json!(WHELP)]);
        let state = board("ai-evolve-whelp", p1, p2);
        let (evolutions, _) = priced(&state);
        let found = entry(&evolutions, WHELP, player).clone();
        assert_eq!(found.condition, EvolveCondition::Event, "{player:?}");
        assert!(found.price > 0.0, "{player:?}: {}", found.price);
        close(
            diff(&state, &prices_of(evolutions)),
            sign * AI_EVAL.evolve_event_share * AI_EVAL.evolve * found.price,
        );
    }
}

#[test]
fn r4648000_the_evolution_multiplier_counts_each_run_where_it_acts() {
    let state = board(
        "ai-evolve-twice",
        json!({ "field": [drifter(), HIGH_EVOLUTIONARY, plain(1, 1), plain(1, 1)] }),
        json!({}),
    );
    let (evolutions, _) = priced(&state);
    let price = entry(&evolutions, DRIFTER, AI).price;
    assert!(price > 0.0, "{price}");
    let prices = prices_of(evolutions);
    close(diff(&state, &prices), 0.8 * AI_EVAL.evolve * 2.0 * price);
    // Without High Evolutionary acting, the same price counts once.
    let once = board(
        "ai-evolve-twice",
        json!({ "field": [drifter(), plain(7, 7), plain(1, 1), plain(1, 1)] }),
        json!({}),
    );
    close(diff(&once, &prices), 0.8 * AI_EVAL.evolve * price);
}

#[test]
fn r4648000_a_card_its_own_turn_or_attack_text_evolves_is_left_to_that_price() {
    let state = board(
        "ai-evolve-own-hook",
        json!({ "field": [WASP, SPACE_INVADER, DISTILLER] }),
        json!({}),
    );
    assert_eq!(priced(&state), (Vec::new(), 0));

    const ZERO: EvalWeights = EvalWeights {
        evolve: 0.0,
        ..AI_EVAL
    };
    assert_eq!(GREEDY_EVAL.evolve, 0.0);
    let state = drifter_board(AI, 4);
    let prices = prices_of(priced(&state).0);
    assert!(!prices.evolutions.is_empty());
    for w in [&ZERO, &GREEDY_EVAL] {
        assert_eq!(
            evaluate_priced(&state, AI, NextSwing::Enemy, w, &prices),
            evaluate_priced(&state, AI, NextSwing::Enemy, w, Prices::NONE)
        );
    }
}

/// The AI's turn 3, its second, at `tier`, with three Radiant Drifters in lanes 1 to 3, one crystal and
/// a Felinor Token and Hotfix in hand; the human holds `human_hand` and has an empty field.
fn setup_board(tier: Handicap, human_hand: &[&str]) -> GameState {
    register_cards();
    let mut state = scenario(json!({
        "seed": "ai-evolve-setup",
        "turn": 3,
        "p1": {
            "field": [drifter(), drifter(), drifter()],
            "hand": [FELINOR, HOTFIX],
            "mana": 1,
            "library": LIBRARY,
        },
        "p2": { "hand": human_hand, "library": LIBRARY },
    }))
    .state()
    .clone();
    state.players[AI].handicap = Some(tier);
    state
}

/// Whether `action` plays an AI hand card of `def_id`.
fn plays_card(action: &ActionBody, state: &GameState, def_id: &str) -> bool {
    matches!(action, ActionBody::Play { instance_id, .. }
        if state.players[AI].hand.iter().any(|card| card.id == *instance_id && card.def_id == def_id))
}

/// The best state a play of the AI's `def_id` leaves, by `prices`, over every candidate that plays it.
fn best_play(state: &GameState, def_id: &str, prices: &Prices) -> f64 {
    candidate_actions(state, AI, Prices::NONE)
        .into_iter()
        .filter(|action| plays_card(action, state, def_id))
        .filter_map(|play| {
            simulate(
                state,
                AI,
                &play,
                &create_node_counter(AI_BUDGET.nodes, None),
                Prices::NONE,
            )
            .expect("a node")
            .ok()
        })
        .map(|after| eval_with(&after, prices))
        .fold(f64::NEG_INFINITY, f64::max)
}

#[test]
fn r4648000_given_one_crystal_the_ai_fills_a_zone_toward_three_drifters_evolving() {
    for tier in [AI_DIFFICULTY.medium, AI_DIFFICULTY.hard] {
        let state = setup_board(tier, &[VANILLA]);
        // At body value the Hotfix's +2/+2 beats a 1/1; the zone the Felinor fills tips it.
        let (felinor, hotfix) = (
            best_play(&state, FELINOR, Prices::NONE),
            best_play(&state, HOTFIX, Prices::NONE),
        );
        assert!(
            felinor < hotfix,
            "{tier:?} at body value: Felinor {felinor} against Hotfix {hotfix}"
        );
        let prices = prices_of(priced(&state).0);
        let (felinor, hotfix) = (
            best_play(&state, FELINOR, &prices),
            best_play(&state, HOTFIX, &prices),
        );
        assert!(
            felinor > hotfix,
            "{tier:?} priced: Felinor {felinor} against Hotfix {hotfix}"
        );
        for cursor in 0..2 {
            let turn = play_ai_turn(
                &state,
                AI,
                &mut AiOptions::new(create_rng("ai-evolve-setup", cursor)),
            );
            let felinor = turn
                .actions
                .iter()
                .any(|action| plays_card(&action.body, &state, FELINOR));
            assert!(felinor, "{tier:?} cursor {cursor}: {:?}", turn.actions);
        }
    }
}

#[test]
fn r4648000_given_one_removal_the_ai_polymorphs_the_drifter_a_zone_from_evolving() {
    for (seed, field) in [
        (
            "ai-evolve-removal",
            json!([drifter(), plain(5, 5), plain(1, 1), plain(1, 1)]),
        ),
        (
            "ai-evolve-removal-flipped",
            json!([plain(1, 1), plain(1, 1), plain(5, 5), drifter()]),
        ),
    ] {
        let state = board(
            seed,
            json!({ "hand": [POLYMORPH], "mana": 3 }),
            json!({ "field": field }),
        );
        let drifter = unit(&state, HUMAN, DRIFTER);
        let big = active_units_of(&state, HUMAN)
            .into_iter()
            .find(|card| unit_view(&state, card).health == 5)
            .expect("the 5/5");
        let (drifter_body, big_body) = (
            kill_worth(&state, drifter, &AI_EVAL, Prices::NONE),
            kill_worth(&state, big, &AI_EVAL, Prices::NONE),
        );
        assert!(
            drifter_body < big_body,
            "{seed}: {drifter_body} against {big_body}"
        );
        let prices = prices_of(priced(&state).0);
        let (drifter_priced, big_priced) = (
            kill_worth(&state, drifter, &AI_EVAL, &prices),
            kill_worth(&state, big, &AI_EVAL, &prices),
        );
        assert!(
            drifter_priced > big_priced,
            "{seed}: {drifter_priced} against {big_priced}"
        );

        let drifter = drifter.id.clone();
        let polymorph = state.players[AI].hand[0].id.clone();
        let turn = play_ai_turn(&state, AI, &mut AiOptions::new(create_rng(seed, 0)));
        let targets = turn.actions.iter().find_map(|action| match &action.body {
            ActionBody::Play {
                instance_id, targets, ..
            } if *instance_id == polymorph => Some(targets.clone()),
            _ => None,
        });
        assert_eq!(
            targets,
            Some(Some(vec![Selection::Instance { instance_id: drifter }])),
            "{seed}: {:?}",
            turn.actions
        );
    }
}

#[test]
fn r4648000_an_enemy_hand_card_is_never_priced_and_hidden_cards_change_no_decision() {
    let enemy = board(
        "ai-evolve-hidden",
        json!({ "hand": [VANILLA] }),
        json!({ "hand": [DRIFTER] }),
    );
    assert_eq!(priced(&enemy), (Vec::new(), 0));
    let own = board(
        "ai-evolve-hidden",
        json!({ "hand": [DRIFTER] }),
        json!({ "hand": [VANILLA] }),
    );
    let (evolutions, used) = priced(&own);
    assert_eq!(evolutions.len(), 1, "{evolutions:?}");
    assert_eq!(
        entry(&evolutions, DRIFTER, AI).condition,
        EvolveCondition::ZonesFull
    );
    assert!(
        (2..=AI_EVOLVE_PROBE_NODES).contains(&used),
        "a spawn and an evolve: {used}"
    );

    let decide_on = |state: &GameState| {
        decide(state, AI, &mut AiOptions::new(create_rng("ai-evolve-hidden", 0))).expect("a decision")
    };
    let (one, other) = (
        setup_board(AI_DIFFICULTY.hard, &[DRIFTER]),
        setup_board(AI_DIFFICULTY.hard, &[VANILLA]),
    );
    assert_ne!(hash_state(&one), hash_state(&other));
    assert_eq!(decide_on(&one), decide_on(&other));
}
