//! Port of `packages/engine/test/cost-rules.test.ts`.
//!
//! Cost rules (docs/classic-sets.md B5 E15, E39; R65, R363, R396, R455): the auras and player
//! modifiers that price a play, the ladder `mana.effectiveCost` climbs, the ban on (N)+ Cost plays, the
//! return-after-resolve enchantment's return and floor, and R396's cost reader.

use jackioh_engine::effects::{add_cost_rule, cast, cast_new, discard, CastNewArgs, CastNewDef};
use jackioh_engine::mana::{cost_now, effective_cost, play_cost};
use jackioh_engine::modifiers::add_modifier;
use jackioh_engine::testkit::*;

use crate::rules::fixtures::harness::{events_of_type, in_hand, put, slot};
use crate::rules::fixtures::play_pipeline_b::{
    cast_trap, embiggen_field, five_unit, forever, grave_spell, in_graveyard, lobbyist, monkey, only, pb_act,
    pb_playing, pb_reduce, plays_of, tax, three_spell, toe_cracker, trickster, two_field, x_spell, x_unit, zero_spell,
};

/// R345: no automatic turn ends, so a test walks the turns it names.
fn manual_turns(state: &GameState) -> GameState {
    let one = pb_act(state, json_as(json!({ "type": "setAutoEndTurn", "enabled": false, "playerId": "p1" })));
    pb_act(&one, json_as(json!({ "type": "setAutoEndTurn", "enabled": false, "playerId": "p2" })))
}

fn hand_card(state: &mut GameState, def_id: &str, player: PlayerId) -> CardInstance {
    only(&in_hand(state, def_id, player, 1))
}

/// TS `sinkFor(state)` and `sinkOf(state)`: a sink's events and rng (from the state's cursor), lent
/// with the state to one engine call at a time.
struct Bench {
    events: Vec<GameEvent>,
    rng: Rng,
}

impl Bench {
    fn new(state: &GameState) -> Bench {
        Bench { events: Vec::new(), rng: Rng::new(&state.seed, state.rng_cursor) }
    }

    fn sink<'a>(&'a mut self, state: &'a mut GameState) -> EngineSink<'a> {
        EngineSink::new(state, &mut self.events, &mut self.rng)
    }
}

/// TS `addModifier(sinkOf(state), player, modifier)`.
fn add_modifier_to(state: &mut GameState, player: PlayerId, expiry: ModifierExpiry, kind: ModifierKind) {
    let mut bench = Bench::new(state);
    add_modifier(&mut bench.sink(state), player, expiry, kind);
}

/// TS `run(state, effects, controller = "p1")`: the effects in one context, the loop settled, the
/// cursor written back; the sink's events come back.
fn run(state: &mut GameState, effects: Vec<Effect>, controller: PlayerId) -> Vec<GameEvent> {
    let mut bench = Bench::new(state);
    {
        let mut sink = bench.sink(state);
        {
            let mut ctx =
                make_context(&mut sink, None, HookOptions { controller: Some(controller), ..Default::default() });
            apply_effects(&effects, &mut ctx);
        }
        settle(&mut sink, Default::default());
    }
    state.rng_cursor = bench.rng.cursor();
    bench.events
}

fn cost(state: &GameState, card: &CardInstance) -> i32 {
    effective_cost(state, card, Default::default())
}

fn cost_discount(amount: i32, min_current_cost: Option<i32>) -> ModifierKind {
    ModifierKind::CostDiscount { amount, only_type: None, min_current_cost, once_per_turn: None }
}

fn cast_new_of(def_id: String) -> Effect {
    cast_new(CastNewArgs { def: CastNewDef::from(def_id), radiant: None, how: Default::default() })
}

fn plays_json(state: &GameState, instance_id: &str) -> Vec<Value> {
    plays_of(state, instance_id, PlayerId::P1)
        .iter()
        .map(|play| serde_json::to_value(play).expect("a play serialises"))
        .collect()
}

fn instance_ids(events: &[GameEvent], kind: GameEventType) -> Vec<String> {
    events_of_type(events, kind)
        .iter()
        .filter_map(|event| {
            serde_json::to_value(event).expect("an event serialises")["instanceId"].as_str().map(str::to_string)
        })
        .collect()
}

fn ids(cards: &[CardInstance]) -> Vec<String> {
    cards.iter().map(|card| card.id.clone()).collect()
}

fn is_cost_rule(modifier: &PlayerModifier) -> bool {
    matches!(modifier.kind, ModifierKind::CostRule { .. })
}

fn held(cards: &[CardInstance], id: &str) -> Vec<CardInstance> {
    cards.iter().filter(|card| card.id == id).cloned().collect()
}

fn refusal(result: &ReduceResult) -> String {
    result.error.clone().unwrap_or_default()
}

mod r455_e15_price_rules_from_auras {
    use super::*;

    #[test]
    fn r455_classic_6s_aura_its_controllers_traps_and_field_traps_cost_0_no_other_card_and_nobody_elses() {
        let mut state = pb_playing("r455-traps");
        put(&mut state, &toe_cracker().id, slot(PlayerId::P1, Row::Units, 1), Default::default());
        let trap = hand_card(&mut state, &cast_trap().id, PlayerId::P1);
        let spell = hand_card(&mut state, &grave_spell().id, PlayerId::P1);
        let theirs = hand_card(&mut state, &cast_trap().id, PlayerId::P2);
        assert_eq!(cost(&state, &trap), 0);
        assert_eq!(cost(&state, &spell), 1);
        assert_eq!(cost(&state, &theirs), 1);
        // In a library it is its own cost (R65): an aura prices a play.
        let in_library = new_instance(&mut state, &cast_trap().id, PlayerId::P1, Zone::Library { player: PlayerId::P1 });
        assert_eq!(cost(&state, &in_library), 1);

        let mana = state.players.p1.mana.current;
        let after = pb_act(
            &state,
            json_as(json!({ "type": "play", "instanceId": trap.id, "zone": { "row": "backrow", "lane": 2 }, "playerId": "p1" })),
        );
        assert_eq!(after.players.p1.mana.current, mana);
    }

    #[test]
    fn r455_r65_classic_77s_aura_both_players_spells_cost_1_more_2_on_its_radiant_face_an_x_card_is_untouched() {
        let mut state = pb_playing("r455-spells");
        let monkey_card = put(&mut state, &monkey().id, slot(PlayerId::P2, Row::Units, 1), Default::default());
        let mine = hand_card(&mut state, &grave_spell().id, PlayerId::P1);
        let theirs = hand_card(&mut state, &grave_spell().id, PlayerId::P2);
        let unit = hand_card(&mut state, &five_unit().id, PlayerId::P1);
        let x = hand_card(&mut state, &x_spell().id, PlayerId::P1);
        assert_eq!(cost(&state, &mine), 2);
        assert_eq!(cost(&state, &theirs), 2);
        assert_eq!(cost(&state, &unit), 5);
        let mut x_for_two = x.clone();
        x_for_two.x = Some(2);
        assert_eq!(cost(&state, &x_for_two), 2);
        find_instance_mut(&mut state, &monkey_card.id).expect("the monkey stands").radiant = true;
        assert_eq!(cost(&state, &mine), 3);
        // A Vanilla card lays no aura (§6.3, R115).
        find_instance_mut(&mut state, &monkey_card.id).expect("the monkey stands").vanilla = true;
        assert_eq!(cost(&state, &mine), 1);
    }

    #[test]
    fn r455_r363_classic_68s_3_plus_surcharge_reads_the_price_the_flat_rules_left_as_it_reads_curvature() {
        let mut state = pb_playing("r455-threshold");
        put(&mut state, &lobbyist().id, slot(PlayerId::P1, Row::Units, 1), Default::default());
        let two = hand_card(&mut state, &two_field().id, PlayerId::P1);
        let three = hand_card(&mut state, &three_spell().id, PlayerId::P1);
        let five = hand_card(&mut state, &five_unit().id, PlayerId::P2);
        assert_eq!(cost(&state, &two), 2);
        assert_eq!(cost(&state, &three), 4);
        assert_eq!(cost(&state, &five), 6);

        // A flat surcharge first: #77's Radiant face takes a (1) Spell to 3, which then meets (3)+.
        let monkey_card = put(&mut state, &monkey().id, slot(PlayerId::P2, Row::Units, 2), Default::default());
        find_instance_mut(&mut state, &monkey_card.id).expect("the monkey stands").radiant = true; // Spells +2
        let one = hand_card(&mut state, &grave_spell().id, PlayerId::P1);
        assert_eq!(cost(&state, &one), 4); // 1 + 2 = 3, then (3)+: +1
        // A flat discount first: the (3) Spell taken to 2 no longer meets it.
        let turn = state.turn;
        add_modifier_to(&mut state, PlayerId::P1, ModifierExpiry::ThisTurn { turn }, cost_discount(3, None));
        assert_eq!(cost(&state, &three), 2); // 3 + 2 − 3 = 2, under the threshold
        // And Curvature and #68 each read the one number the flat rules left (R363): a (3) card meets
        // #68's (3)+ and not Curvature's (4)+, even though #68 then takes it to 4.
        let mut fresh = pb_playing("r455-threshold-curvature");
        put(&mut fresh, &lobbyist().id, slot(PlayerId::P1, Row::Units, 1), Default::default());
        let turn = fresh.turn;
        add_modifier_to(&mut fresh, PlayerId::P1, ModifierExpiry::ThisTurn { turn }, cost_discount(1, Some(4)));
        let three = hand_card(&mut fresh, &three_spell().id, PlayerId::P1);
        assert_eq!(cost(&fresh, &three), 4); // 3 → +1
        let five = hand_card(&mut fresh, &five_unit().id, PlayerId::P1);
        assert_eq!(cost(&fresh, &five), 5); // 5 → −1 +1
    }

    #[test]
    fn r455_classic_68s_radiant_ban_the_opponent_is_offered_no_play_whose_price_is_3_or_more_and_refused_one_casts_go_on() {
        let mut state = pb_playing("r455-ban");
        put(&mut state, &lobbyist().id, slot(PlayerId::P2, Row::Units, 1), json_as(json!({ "radiant": true })));
        let three = hand_card(&mut state, &three_spell().id, PlayerId::P1);
        let one = hand_card(&mut state, &grave_spell().id, PlayerId::P1);
        assert_eq!(plays_json(&state, &three.id), Vec::<Value>::new());
        assert_eq!(plays_json(&state, &one.id).len(), 1);
        assert!(refusal(&pb_reduce(&state, json_as(json!({ "type": "play", "instanceId": three.id, "playerId": "p1" }))))
            .contains("can't play (3)+ Cost cards"));
        // An X card: X below 3 only.
        let x = hand_card(&mut state, &x_spell().id, PlayerId::P1);
        let xs: Vec<Value> = plays_json(&state, &x.id).iter().map(|play| play["x"].clone()).collect();
        assert_eq!(xs, vec![json!(1), json!(2)]);
        // The price is what bans: a discount under 3 lifts it.
        let turn = state.turn;
        add_modifier_to(&mut state, PlayerId::P1, ModifierExpiry::ThisTurn { turn }, cost_discount(1, None));
        assert_eq!(plays_json(&state, &three.id).len(), 1);
        // Its controller is not banned, and a cast is not a play from hand (R70).
        let theirs = hand_card(&mut state, &three_spell().id, PlayerId::P2);
        assert_eq!(play_cost(&state, &theirs), 3);
        let before = state.counters.played;
        run(&mut state, vec![cast_new_of(three_spell().id)], PlayerId::P1);
        assert_eq!(state.counters.played, before + 1);
    }
}

mod r455_e15_price_rules_on_a_player {
    use super::*;

    #[test]
    fn r455_classic_2_the_next_trap_or_field_spell_costs_2_less_until_one_is_played_across_turns_a_spell_or_a_cast_leaves_it() {
        let mut state = manual_turns(&pb_playing("r455-next"));
        let card = hand_card(&mut state, &trickster().id, PlayerId::P1);
        let mut after = pb_act(
            &state,
            json_as(json!({ "type": "play", "instanceId": card.id, "zone": { "row": "units", "lane": 1 }, "playerId": "p1" })),
        );
        let rules: Vec<PlayerModifier> = after.players.p1.mods.iter().filter(|m| is_cost_rule(m)).cloned().collect();
        let rule = only(&rules);
        let shown: Vec<ModifierView> = view_for(&after, PlayerId::P2)
            .opponent
            .modifiers
            .into_iter()
            .filter(|modifier| modifier.id == rule.id)
            .collect();
        assert_eq!(only(&shown).label, "Your next Trap or Field Spell costs (2) less");
        let field = hand_card(&mut after, &two_field().id, PlayerId::P1);
        let trap = hand_card(&mut after, &cast_trap().id, PlayerId::P1);
        let spell = hand_card(&mut after, &grave_spell().id, PlayerId::P1);
        assert_eq!(cost(&after, &field), 0);
        assert_eq!(cost(&after, &trap), 0);
        assert_eq!(cost(&after, &spell), 1);

        // A Spell played leaves it; a cast Trap leaves it (a cast never uses a discount, R70).
        after = pb_act(&after, json_as(json!({ "type": "play", "instanceId": spell.id, "playerId": "p1" })));
        run(&mut after, vec![cast_new_of(cast_trap().id)], PlayerId::P1);
        assert!(after.players.p1.mods.iter().any(|modifier| modifier.id == rule.id));
        // It waits across turns ("next" has no "this turn").
        after = pb_act(&after, json_as(json!({ "type": "endTurn", "playerId": "p1" })));
        after = pb_act(&after, json_as(json!({ "type": "endTurn", "playerId": "p2" })));
        assert!(after.players.p1.mods.iter().any(|modifier| modifier.id == rule.id));
        // The Field Spell it prices spends it.
        let mana = after.players.p1.mana.current;
        after = pb_act(
            &after,
            json_as(json!({ "type": "play", "instanceId": field.id, "zone": { "row": "backrow", "lane": 3 }, "playerId": "p1" })),
        );
        assert_eq!(after.players.p1.mana.current, mana);
        assert!(!after.players.p1.mods.iter().any(|modifier| modifier.id == rule.id));
        assert_eq!(cost(&after, &trap), 1);
    }

    #[test]
    fn r455_classic_2s_radiant_face_the_next_trap_or_field_spell_costs_0_which_wins_over_every_add() {
        let mut state = pb_playing("r455-next-zero");
        put(&mut state, &lobbyist().id, slot(PlayerId::P2, Row::Units, 1), Default::default());
        let card = hand_card(&mut state, &trickster().id, PlayerId::P1);
        find_instance_mut(&mut state, &card.id).expect("the hand card").radiant = true;
        let mut after = pb_act(
            &state,
            json_as(json!({ "type": "play", "instanceId": card.id, "zone": { "row": "units", "lane": 1 }, "playerId": "p1" })),
        );
        let mut big = new_instance(&mut after, &two_field().id, PlayerId::P1, Zone::Hand { player: PlayerId::P1 });
        big.cost_mod = 3; // (5), and (3)+ would add 1
        after.players.p1.hand.push(big.clone());
        assert_eq!(cost(&after, &big), 0);
        let next: Vec<ModifierView> = view_for(&after, PlayerId::P1)
            .you
            .modifiers
            .into_iter()
            .filter(|modifier| modifier.label.contains("next"))
            .collect();
        assert_eq!(only(&next).label, "Your next Trap or Field Spell costs (0)");
    }

    #[test]
    fn r455_r48_ai_alignment_tax_the_opponents_cards_cost_1_more_during_their_next_turn_only_its_timing_turned_outward() {
        let mut state = manual_turns(&pb_playing("r455-tax"));
        let card = hand_card(&mut state, &tax().id, PlayerId::P1);
        let theirs = hand_card(&mut state, &grave_spell().id, PlayerId::P2);
        let mut after = pb_act(&state, json_as(json!({ "type": "play", "instanceId": card.id, "playerId": "p1" })));
        // Not on the turn it was cast.
        assert_eq!(cost(&after, &only(&held(&after.players.p2.hand, &theirs.id))), 1);
        assert_eq!(only(&view_for(&after, PlayerId::P1).opponent.modifiers).label, "Your cards cost (1) more (next turn)");
        after = pb_act(&after, json_as(json!({ "type": "endTurn", "playerId": "p1" })));
        // On their next turn: +1.
        assert_eq!(after.active, PlayerId::P2);
        assert_eq!(cost(&after, &only(&held(&after.players.p2.hand, &theirs.id))), 2);
        after = pb_act(&after, json_as(json!({ "type": "endTurn", "playerId": "p2" })));
        // Gone at their cleanup.
        assert!(!after.players.p2.mods.iter().any(is_cost_rule));
    }

    #[test]
    fn r455_an_added_cost_rules_spans_this_turn_ends_at_cleanup_never_stays() {
        let mut state = manual_turns(&pb_playing("r455-spans"));
        run(
            &mut state,
            vec![
                add_cost_rule(json_as(json!({ "rule": { "amount": 2 }, "lasts": "thisTurn" }))),
                add_cost_rule(json_as(json!({ "rule": { "types": ["Spell"], "amount": -1 }, "lasts": "never" }))),
            ],
            PlayerId::P1,
        );
        let spell = hand_card(&mut state, &grave_spell().id, PlayerId::P1);
        assert_eq!(cost(&state, &spell), 2);
        let after = pb_act(&state, json_as(json!({ "type": "endTurn", "playerId": "p1" })));
        assert_eq!(after.players.p1.mods.iter().filter(|m| is_cost_rule(m)).count(), 1);
        assert_eq!(cost(&after, &only(&held(&after.players.p1.hand, &spell.id))), 0);
    }
}

mod r455_e39_return_after_resolving_and_its_floor {
    use super::*;

    #[test]
    fn r455_classic_plus_14_the_next_spell_played_gains_the_return_it_comes_back_after_it_resolves_every_time_and_cant_cost_less_than_2() {
        let mut state = pb_playing("r455-forever");
        let card = hand_card(&mut state, &forever().id, PlayerId::P1);
        let mut after = pb_act(&state, json_as(json!({ "type": "play", "instanceId": card.id, "playerId": "p1" })));
        assert_eq!(
            only(&view_for(&after, PlayerId::P1).you.modifiers).label,
            "Your next Spell returns to your hand after it resolves (it can't cost less than (2))"
        );
        let spell = hand_card(&mut after, &grave_spell().id, PlayerId::P1);
        after.players.p1.mana.current = 4;
        let before = after.players.p2.hero.health;
        after = pb_act(&after, json_as(json!({ "type": "play", "instanceId": spell.id, "playerId": "p1" })));
        // Resolved (1 damage), back in hand, carrying the enchantment; the rider is spent.
        assert_eq!(after.players.p2.hero.health, before - 1);
        let back = only(&held(&after.players.p1.hand, &spell.id));
        assert_eq!(back.enchantments, Some(vec![Enchantment::ReturnAfterResolve { floor: 2 }]));
        assert!(
            !after.players.p1.mods.iter().any(|modifier| matches!(modifier.kind, ModifierKind::EnchantNextSpell { .. }))
        );
        // The floor: a (1) Spell costs (2), after every discount.
        assert_eq!(cost(&after, &back), 2);
        let turn = after.turn;
        add_modifier_to(&mut after, PlayerId::P1, ModifierExpiry::ThisTurn { turn }, cost_discount(5, None));
        assert_eq!(cost(&after, &back), 2);
        // And it comes back again, and in any pile it keeps the floor.
        after = pb_act(&after, json_as(json!({ "type": "play", "instanceId": back.id, "playerId": "p1" })));
        assert!(ids(&after.players.p1.hand).contains(&back.id));
    }

    #[test]
    fn r410_r455_a_spell_with_the_return_comes_back_from_the_exile_it_lands_in_and_not_when_discarded_a_full_hand_burns_it() {
        let mut state = pb_playing("r455-no-return");
        let spell = hand_card(&mut state, &grave_spell().id, PlayerId::P1);
        find_instance_mut(&mut state, &spell.id).expect("the hand card").enchantments =
            Some(vec![Enchantment::ReturnAfterResolve { floor: 1 }]);
        // Discarded: not played, so nothing resolves and nothing returns.
        {
            let mut bench = Bench::new(&state);
            let mut sink = bench.sink(&mut state);
            let mut ctx =
                make_context(&mut sink, None, HookOptions { controller: Some(PlayerId::P1), ..Default::default() });
            apply_effects(
                &[discard(json_as(json!({ "target": { "of": "instance", "instanceId": spell.id } })))],
                &mut ctx,
            );
        }
        assert!(ids(&state.players.p1.graveyard).contains(&spell.id));

        // Cast with "then exile it": it resolves, lands in exile, and comes back from there (R410).
        let other = in_graveyard(&mut state, &grave_spell().id, PlayerId::P1);
        find_instance_mut(&mut state, &other.id).expect("the graveyard card").enchantments =
            Some(vec![Enchantment::ReturnAfterResolve { floor: 1 }]);
        let back = run(
            &mut state,
            vec![cast(json_as(json!({ "target": { "of": "instance", "instanceId": other.id }, "afterward": "exile" })))],
            PlayerId::P1,
        );
        assert!(instance_ids(&back, GameEventType::Exiled).contains(&other.id));
        assert!(ids(&state.players.p1.hand).contains(&other.id));
        assert!(!ids(&state.players.p1.exile).contains(&other.id));

        // A full hand burns it on its way back (§2.4).
        let mut full = pb_playing("r455-burn");
        let burnt = hand_card(&mut full, &grave_spell().id, PlayerId::P1);
        find_instance_mut(&mut full, &burnt.id).expect("the hand card").enchantments =
            Some(vec![Enchantment::ReturnAfterResolve { floor: 1 }]);
        let filler = HAND_CAP - full.players.p1.hand.len() as i32 + 1;
        in_hand(&mut full, &zero_spell().id, PlayerId::P1, filler.max(0));
        let played = pb_reduce(&full, json_as(json!({ "type": "play", "instanceId": burnt.id, "playerId": "p1" })));
        assert_eq!(played.error, None);
        assert!(instance_ids(&played.events, GameEventType::Burned).contains(&burnt.id));
    }

    #[test]
    fn r455_r70_forevers_radiant_face_floors_at_1_and_a_cast_spell_takes_the_rider_too() {
        let mut state = pb_playing("r455-forever-radiant");
        let card = hand_card(&mut state, &forever().id, PlayerId::P1);
        find_instance_mut(&mut state, &card.id).expect("the hand card").radiant = true;
        let mut after = pb_act(&state, json_as(json!({ "type": "play", "instanceId": card.id, "playerId": "p1" })));
        run(&mut after, vec![cast_new_of(zero_spell().id)], PlayerId::P1);
        let returned: Vec<CardInstance> = after
            .players
            .p1
            .hand
            .iter()
            .filter(|held| held.def_id == zero_spell().id && held.enchantments.is_some())
            .cloned()
            .collect();
        let back = only(&returned);
        assert_eq!(back.enchantments, Some(vec![Enchantment::ReturnAfterResolve { floor: 1 }]));
        assert_eq!(cost(&after, &back), 1);
    }
}

mod r396_an_x_card_on_the_field_costs_its_x {
    use super::*;

    #[test]
    fn r396_cost_now_an_x_card_played_for_x_costs_x_on_the_field_0_elsewhere_and_when_it_arrived_with_no_x_an_embiggen_card_its_base() {
        let mut state = pb_playing("r396-x");
        let in_hand_x = hand_card(&mut state, &x_unit().id, PlayerId::P1);
        assert_eq!(cost_now(&state, &in_hand_x), 0);
        state.players.p1.mana.current = 3;
        let mut after = pb_act(
            &state,
            json_as(json!({ "type": "play", "instanceId": in_hand_x.id, "x": 3, "zone": { "row": "units", "lane": 1 }, "playerId": "p1" })),
        );
        let on_field = only(after.players.p1.units[0].as_deref().unwrap_or(&[]));
        assert_eq!(on_field.x, Some(3));
        assert_eq!(cost_now(&after, &on_field), 3);
        // A summon with no chosen X: 0 (R65).
        let summoned = put(&mut after, &x_unit().id, slot(PlayerId::P1, Row::Units, 2), Default::default());
        assert_eq!(cost_now(&after, &summoned), 0);
        // An embiggen card on the field paid its bigger price, and still costs its base (R65).
        let big = put(&mut after, &embiggen_field().id, slot(PlayerId::P1, Row::Backrow, 1), Default::default());
        find_instance_mut(&mut after, &big.id).expect("the embiggen card stands").embiggened = Some(true);
        let big = find_instance(&after, &big.id).expect("the embiggen card stands").clone();
        assert_eq!(cost_now(&after, &big), 2);
        // Any other card: R65's cost as it stands, a hand card at its hand cost.
        let spell = hand_card(&mut after, &grave_spell().id, PlayerId::P1);
        put(&mut after, &monkey().id, slot(PlayerId::P2, Row::Units, 1), Default::default());
        assert_eq!(cost_now(&after, &spell), 2);
        // The graveyard and the library read their own cost.
        let grave = in_graveyard(&mut after, &x_unit().id, PlayerId::P1);
        assert_eq!(cost_now(&after, &grave), 0);
    }
}
