//! Port of `packages/engine/test/mana.test.ts`: R65's cost calculation (M1-T6), `canAfford`, and
//! §2.3's mana refresh.

use jackioh_engine::testkit::*;

use crate::rules::fixtures::harness::new_game;
use crate::rules::fixtures::scripts::{going_long, heroic_power, x_bolt};

/// TS `handCard`: a new instance of `def_id` pushed onto p1's hand. Answers its id: the card is read
/// back from the state by id (TS held the live object).
fn hand_card(state: &mut GameState, def_id: &str) -> String {
    let card = new_instance(state, def_id, PlayerId::P1, Zone::Hand { player: PlayerId::P1 });
    let id = card.id.clone();
    state.players.p1.hand.push(card);
    id
}

fn card<'a>(state: &'a GameState, id: &str) -> &'a CardInstance {
    find_instance(state, id).expect("the card is in the state")
}

fn card_mut<'a>(state: &'a mut GameState, id: &str) -> &'a mut CardInstance {
    find_instance_mut(state, id).expect("the card is in the state")
}

/// `effectiveCost(state, card)`.
fn cost(state: &GameState, id: &str) -> i32 {
    mana::effective_cost(state, card(state, id), CostOptions::default())
}

/// `addModifier(sinkFor(state), player, { kind…, expiry })`: the modifier's kind and expiry written
/// as TS's literal. The sink's rng starts at the state's cursor, as `sinkFor` did; nothing writes the
/// cursor back, as TS did not.
fn add_mod(state: &mut GameState, player: PlayerId, kind: Value, expiry: Value) {
    let mut events: Vec<GameEvent> = Vec::new();
    let mut rng = Rng::new(&state.seed, state.rng_cursor);
    let mut sink = EngineSink::new(state, &mut events, &mut rng);
    modifiers::add_modifier(
        &mut sink,
        player,
        json_as::<ModifierExpiry>(expiry),
        json_as::<ModifierKind>(kind),
    );
}

mod r65_cost_calculation_m1_t6 {
    use super::*;

    /// "starts from the printed cost and adds the instance's costMod"
    #[test]
    fn starts_from_the_printed_cost_and_adds_the_instances_cost_mod() {
        let mut state = new_game("cost-base", None);
        let id = hand_card(&mut state, "fx-1"); // printed cost 1
        assert_eq!(mana::printed_cost(&state, card(&state, &id)), 1);
        assert_eq!(cost(&state, &id), 1);

        card_mut(&mut state, &id).cost_mod = 2;
        assert_eq!(cost(&state, &id), 3);
        card_mut(&mut state, &id).cost_mod = -5;
        assert_eq!(cost(&state, &id), 0); // floors at 0
    }

    /// "R65 prefers costOverride over the printed cost, then applies costMod"
    #[test]
    fn r65_prefers_cost_override_over_the_printed_cost_then_applies_cost_mod() {
        let mut state = new_game("cost-override", None);
        let id = hand_card(&mut state, "fx-1");
        card_mut(&mut state, &id).cost_override = Some(4);
        assert_eq!(cost(&state, &id), 4);
        card_mut(&mut state, &id).cost_mod = -1;
        assert_eq!(cost(&state, &id), 3);
    }

    /// "R65 applies a player discount, and only to the types it names"
    #[test]
    fn r65_applies_a_player_discount_and_only_to_the_types_it_names() {
        let mut state = new_game("cost-discount", None);
        let unit = hand_card(&mut state, "fx-1");
        add_mod(
            &mut state,
            PlayerId::P1,
            json!({ "kind": "costDiscount", "amount": 1 }),
            json!({ "until": "thisTurn", "turn": 1 }),
        );
        assert_eq!(cost(&state, &unit), 0);

        let mut state2 = new_game("cost-discount-spell", None);
        let unit2 = hand_card(&mut state2, "fx-1");
        card_mut(&mut state2, &unit2).cost_mod = 3; // cost 4, so the discount below would otherwise bite
        add_mod(
            &mut state2,
            PlayerId::P1,
            json!({ "kind": "costDiscount", "amount": 2, "onlyType": "Spell" }),
            json!({ "until": "thisTurn", "turn": 1 }),
        );
        assert_eq!(cost(&state2, &unit2), 4);
    }

    /// "R65 applies Professor Curvature last, and only when the cost is then 4 (R48)"
    #[test]
    fn r65_r48_applies_professor_curvature_last_and_only_when_the_cost_is_then_4() {
        let mut state = new_game("curvature", None);
        state.turn = 2; // the turn after the one that made the modifier, so it is live (R48)
        let id = hand_card(&mut state, "fx-1");
        card_mut(&mut state, &id).cost_mod = 3; // printed 1 + 3 = 4
        add_mod(
            &mut state,
            PlayerId::P1,
            json!({ "kind": "costDiscount", "amount": 1, "minCurrentCost": 4 }),
            json!({ "until": "nextTurnOf", "player": "p1", "fromTurn": 0 }),
        );
        assert_eq!(cost(&state, &id), 3);

        // A card that is not at 4 after the earlier steps is untouched.
        let other = hand_card(&mut state, "fx-2");
        assert_eq!(cost(&state, &other), 1);
    }

    /// "R65 lets a general discount bring a card to 4, which Curvature then discounts"
    #[test]
    fn r65_lets_a_general_discount_bring_a_card_to_4_which_curvature_then_discounts() {
        let mut state = new_game("curvature-chain", None);
        let id = hand_card(&mut state, "fx-1");
        card_mut(&mut state, &id).cost_mod = 4; // printed 1 + 4 = 5
        add_mod(
            &mut state,
            PlayerId::P1,
            json!({ "kind": "costDiscount", "amount": 1 }),
            json!({ "until": "thisTurn", "turn": 1 }),
        );
        add_mod(
            &mut state,
            PlayerId::P1,
            json!({ "kind": "costDiscount", "amount": 1, "minCurrentCost": 4 }),
            json!({ "until": "thisTurn", "turn": 1 }),
        );
        assert_eq!(cost(&state, &id), 3);
    }

    /// "R65 makes an X-cost card cost exactly X, ignoring modifiers, and 0 with an override"
    #[test]
    fn r65_makes_an_x_cost_card_cost_exactly_x_ignoring_modifiers_and_0_with_an_override() {
        let mut state = new_game("cost-x", None);
        let bolt = hand_card(&mut state, &x_bolt().id);
        assert_eq!(cost(&state, &bolt), 0); // out of play, X counts as 0

        card_mut(&mut state, &bolt).x = Some(3);
        card_mut(&mut state, &bolt).cost_mod = -2;
        add_mod(
            &mut state,
            PlayerId::P1,
            json!({ "kind": "costDiscount", "amount": 2 }),
            json!({ "until": "thisTurn", "turn": 1 }),
        );
        assert_eq!(cost(&state, &bolt), 3);

        // An override makes it free, whatever the override's own number says (R65).
        card_mut(&mut state, &bolt).cost_override = Some(2);
        assert_eq!(cost(&state, &bolt), 0);
    }

    /// "R48 a next-turn discount does nothing on the turn it was created"
    #[test]
    fn r48_a_next_turn_discount_does_nothing_on_the_turn_it_was_created() {
        let mut state = new_game("curvature-timing", None);
        state.turn = 1;
        let id = hand_card(&mut state, "fx-1");
        card_mut(&mut state, &id).cost_mod = 3; // cost 4, which is what Curvature looks for
        let from_turn = state.turn;
        add_mod(
            &mut state,
            PlayerId::P1,
            json!({ "kind": "costDiscount", "amount": 1, "minCurrentCost": 4 }),
            json!({ "until": "nextTurnOf", "player": "p1", "fromTurn": from_turn }),
        );

        assert_eq!(cost(&state, &id), 4); // this turn: untouched
        state.turn = 3; // their next turn
        assert_eq!(cost(&state, &id), 3);
    }

    /// "R65 reads an embiggen card at its base price until the bigger price is chosen"
    #[test]
    fn r65_reads_an_embiggen_card_at_its_base_price_until_the_bigger_price_is_chosen() {
        let mut state = new_game("cost-embiggen", None);
        let id = hand_card(&mut state, &going_long().id); // 2 embiggen 4
        assert_eq!(cost(&state, &id), 2);
        card_mut(&mut state, &id).embiggened = Some(true);
        assert_eq!(cost(&state, &id), 4);
        card_mut(&mut state, &id).cost_mod = -1;
        assert_eq!(cost(&state, &id), 3);
    }

    /// "R752 Heroic Power costs (0) to play, whatever power it rolled"
    #[test]
    fn r752_heroic_power_costs_0_to_play_whatever_power_it_rolled() {
        let mut state = new_game("cost-power", None);
        let power = hand_card(&mut state, &heroic_power().id);
        assert_eq!(cost(&state, &power), 0);
        // X 3, paid as the power is activated, never to play the card
        card_mut(&mut state, &power)
            .memory
            .insert("power".to_string(), json!("tricks"));
        assert_eq!(cost(&state, &power), 0);
    }

    /// "canAfford compares against current mana, temporary mana included"
    #[test]
    fn can_afford_compares_against_current_mana_temporary_mana_included() {
        let mut state = new_game("afford", None);
        let id = hand_card(&mut state, "fx-1");
        state.players.p1.mana.current = 0;
        assert!(!mana::can_afford(&state, card(&state, &id)));
        mana::gain_mana(&mut state.players.p1, 1);
        assert!(mana::can_afford(&state, card(&state, &id)));
    }

    /// "refreshes to min(turns started, 4) plus persistent modifiers, and the one-shot moves only
    /// current (§2.3)"
    #[test]
    fn refreshes_to_min_turns_started_4_plus_persistent_modifiers_and_the_one_shot_moves_only_current() {
        let mut state = new_game("refresh", None);
        let side = &mut state.players.p1;

        side.turns_started = 3;
        mana::refresh_mana(side);
        assert_eq!((side.mana.current, side.mana.max), (3, 3));

        side.turns_started = 9;
        mana::refresh_mana(side);
        assert_eq!(side.mana.max, MAX_MANA);

        // §2.3: max mana is min(turns, 4) plus the persistent modifier; the one-shot rider (Hinder's −2,
        // Efficiency Dividend's +) moves only what this refresh gives, and is then cleared.
        side.mana.next_turn_mod = -2;
        side.mana.perm_mod = 1;
        assert_eq!(mana::max_mana_for(side), MAX_MANA + 1);
        mana::refresh_mana(side);
        assert_eq!((side.mana.current, side.mana.max), (MAX_MANA - 1, MAX_MANA + 1));
        assert_eq!(side.mana.next_turn_mod, 0);

        side.mana.next_turn_mod = -20;
        mana::refresh_mana(side);
        assert_eq!((side.mana.current, side.mana.max), (0, MAX_MANA + 1)); // never below 0

        mana::refresh_mana(side);
        mana::gain_mana(side, 3);
        assert_eq!(side.mana.current, MAX_MANA + 4); // temporary mana may exceed max
        mana::spend_mana(side, 10);
        assert_eq!(side.mana.current, 0); // and never goes below 0
    }
}
