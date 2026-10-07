//! Port of `packages/engine/test/glow-facts.test.ts`.
//!
//! R662: the board facts the yellow glow reads for the cards R195 left out (`src/query.ts`), and the
//! granted half of `conditionActive` (`src/condition.ts` rule 5): a hand card glows while a condition
//! another card grants it holds. The real cards (#38, #64, #78, #96, #85) prove the same again in
//! their own test files; here the granting cards are test-only definitions carrying the same static
//! flags and modifiers the pipeline reads, registered on top of the fixture catalog and put back in
//! `afterAll`.
//!
//! (TS saved the registries in `beforeAll` and put them back in `afterAll`. The testkit's registries
//! are thread-local and every `#[test]` runs on its own thread, so nothing outlives a test and there
//! is nothing to put back.)

use jackioh_engine::testkit::*;

use crate::rules::fixtures::call_to_chaos_plus::immutable;
use crate::rules::fixtures::combat::{plain, trampler};
use crate::rules::fixtures::harness::{in_hand, new_game, put, slot};

/// A Field Spell with no other text, indexed from 1981 in the order TS made them.
fn field_spell(name: &str, index: u32) -> CardDef {
    json_as(json!({
        "id": format!("gf-{name}"),
        "index": index.to_string(),
        "name": name,
        "set": "Core",
        "type": "Field Spell",
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": 1,
        "base": { "keywords": [], "text": name },
        "radiant": { "keywords": [], "text": name },
    }))
}

/// #38's flag and #64's threshold, on Field Spells with no other text.
fn striker() -> CardDef {
    field_spell("striker", 1981)
}

fn gifted() -> CardDef {
    field_spell("gifted", 1982)
}

fn two_cost() -> CardDef {
    CardDef { cost: CardCost::Fixed(2), ..field_spell("two-cost", 1983) }
}

fn both(script: Script) -> CardScripts {
    CardScripts { base: script.clone(), radiant: script }
}

fn defs() -> Vec<CardDef> {
    vec![striker(), gifted(), two_cost(), immutable.clone()]
}

fn local_scripts() -> IndexMap<String, CardScripts> {
    let mut scripts = IndexMap::new();
    scripts.insert(
        striker().id,
        both(Script { static_flags: Some(json_as(json!({ "quickstriker": true }))), ..Script::default() }),
    );
    scripts.insert(
        gifted().id,
        both(Script { static_flags: Some(json_as(json!({ "giftedProgram": 1 }))), ..Script::default() }),
    );
    scripts
}

fn board(seed: &str) -> GameState {
    let mut state = new_game(seed, None);
    let mut catalog = catalog::registered_catalog().clone();
    for def in defs() {
        catalog.insert(def.id.clone(), def);
    }
    register_catalog(catalog);
    let mut scripts = scripts::registered_scripts();
    scripts.extend(local_scripts());
    register_scripts(scripts);
    state.turn = 4;
    state.active = PlayerId::P1;
    state.phase = Phase::Main;
    state.pending = None;
    state.players.p1.mana.current = 5;
    state
}

/// As §10.5 step 4 counts a play this turn.
fn played_one(state: &mut GameState, paid: i32) {
    let log = &mut state.players.p1.turn_log;
    log.cards_played += 1;
    log.played_ids.push(format!("played-{}", log.cards_played));
    let mut costs = log.costs_paid.clone().unwrap_or_default();
    costs.push(paid);
    log.costs_paid = Some(costs);
}

/// `{ kind: "comboDraw"; amount } | { kind: "quickstrikerDamage" }`, as a JSON literal.
fn add_mod(state: &mut GameState, body: Value) {
    let mut rider = json!({
        "id": format!("gf-mod-{}", state.players.p1.mods.len()),
        "expiry": { "until": "thisTurn", "turn": state.turn },
    });
    for (key, value) in body.as_object().expect("a rider is an object") {
        rider[key] = value.clone();
    }
    state.players.p1.mods.push(json_as::<PlayerModifier>(rider));
}

/// The card as the state holds it now (TS read its live object).
fn live(state: &GameState, id: &str) -> CardInstance {
    find_instance(state, id).expect("the card is in the game").clone()
}

fn first_in_hand(state: &mut GameState, def_id: &str) -> CardInstance {
    in_hand(state, def_id, PlayerId::P1, 1).first().expect("a card in hand").clone()
}

mod r662_granted_combo_live_a_granted_combo_answers_the_next_play {
    use super::*;

    #[test]
    fn r662_a_quickstrikers_flag_on_the_field_is_live_once_a_card_has_been_played_this_turn() {
        let mut state = board("gf-striker");
        put(&mut state, &striker().id, slot(PlayerId::P1, Row::Backrow, 1), json!({}));
        assert!(!query::granted_combo_live(&state, PlayerId::P1));
        played_one(&mut state, 3);
        assert!(query::granted_combo_live(&state, PlayerId::P1));
        // The other seat's plays and permanents are its own.
        assert!(!query::granted_combo_live(&state, PlayerId::P2));
    }

    #[test]
    fn r662_a_combo_draw_or_quickstriker_damage_rider_counts_a_spent_turns_rider_or_a_draw_of_0_does_not() {
        let mut state = board("gf-riders");
        played_one(&mut state, 3);
        assert!(!query::granted_combo_live(&state, PlayerId::P1));
        add_mod(&mut state, json!({ "kind": "comboDraw", "amount": 0 }));
        assert!(!query::granted_combo_live(&state, PlayerId::P1));
        add_mod(&mut state, json!({ "kind": "comboDraw", "amount": 1 }));
        assert!(query::granted_combo_live(&state, PlayerId::P1));

        let mut later = board("gf-riders-2");
        played_one(&mut later, 3);
        later.players.p1.mods.push(json_as(json!({
            "id": "old", "kind": "quickstrikerDamage", "expiry": { "until": "thisTurn", "turn": 1 }
        })));
        assert!(!query::granted_combo_live(&later, PlayerId::P1));
        add_mod(&mut later, json!({ "kind": "quickstrikerDamage" }));
        assert!(query::granted_combo_live(&later, PlayerId::P1));
    }
}

mod r662_gifted_would_make_radiant_step_3s_question_asked_of_a_hand_card_now {
    use super::*;

    #[test]
    fn r662_the_first_card_costing_the_threshold_or_less_this_turn_and_not_one_already_radiant() {
        let mut state = board("gf-gifted");
        put(&mut state, &gifted().id, slot(PlayerId::P1, Row::Backrow, 1), json!({}));
        let cheap = first_in_hand(&mut state, &plain.id);
        let dear = first_in_hand(&mut state, &two_cost().id);
        assert!(query::gifted_would_make_radiant(&state, PlayerId::P1, &live(&state, &cheap.id)));
        assert!(!query::gifted_would_make_radiant(&state, PlayerId::P1, &live(&state, &dear.id)));

        find_instance_mut(&mut state, &cheap.id).expect("in hand").radiant = true;
        assert!(!query::gifted_would_make_radiant(&state, PlayerId::P1, &live(&state, &cheap.id)));
        find_instance_mut(&mut state, &cheap.id).expect("in hand").radiant = false;

        // R213: a cheap card played earlier this turn was the first.
        played_one(&mut state, 1);
        assert!(!query::gifted_would_make_radiant(&state, PlayerId::P1, &live(&state, &cheap.id)));
    }

    #[test]
    fn r662_without_a_gifted_program_on_the_players_side_nothing_qualifies() {
        let mut state = board("gf-gifted-none");
        put(&mut state, &gifted().id, slot(PlayerId::P2, Row::Backrow, 1), json!({}));
        let cheap = first_in_hand(&mut state, &plain.id);
        assert!(!query::gifted_would_make_radiant(&state, PlayerId::P1, &live(&state, &cheap.id)));
    }
}

mod r662_lethal_attackers_of_r44s_projection_over_the_enemy_units_acting_now {
    use super::*;

    fn lethal_ids(state: &GameState, player: PlayerId) -> Vec<String> {
        query::lethal_attackers_of(state, player).iter().map(|unit| unit.id.clone()).collect()
    }

    #[test]
    fn r662_names_the_enemy_units_whose_attack_on_the_hero_would_be_lethal_and_only_those() {
        let mut state = board("gf-lethal");
        let big = put(&mut state, &trampler.id, slot(PlayerId::P2, Row::Units, 1), json!({})); // 6/4
        let small = put(&mut state, &plain.id, slot(PlayerId::P2, Row::Units, 2), json!({})); // 3/3
        let mine = put(&mut state, &plain.id, slot(PlayerId::P1, Row::Units, 1), json!({}));
        assert_eq!(lethal_ids(&state, PlayerId::P1), Vec::<String>::new());

        state.players.p1.hero.health = 6;
        assert_eq!(lethal_ids(&state, PlayerId::P1), vec![big.id.clone()]);
        state.players.p1.hero.health = 3;
        assert_eq!(lethal_ids(&state, PlayerId::P1), vec![big.id.clone(), small.id.clone()]);
        // Armor counts (§4.4 step 2): neither blow gets through 10 Armor.
        state.players.p1.hero.armor = 10;
        assert_eq!(lethal_ids(&state, PlayerId::P1), Vec::<String>::new());
        // A player's own units are never a threat to their own hero: p2's list is p1's unit alone.
        state.players.p2.hero.health = 1;
        assert_eq!(lethal_ids(&state, PlayerId::P2), vec![mine.id.clone()]);
    }
}

mod r662_fusable_permanents_of_where_85s_fuse_could_land {
    use super::*;

    fn fusable_ids(state: &GameState, player: PlayerId, except: Option<&str>) -> Vec<String> {
        query::fusable_permanents_of(state, player, except).iter().map(|card| card.id.clone()).collect()
    }

    #[test]
    fn r662_every_permanent_of_the_players_but_the_one_excepted_and_never_an_immutable_one() {
        let mut state = board("gf-fusable");
        let trap = put(&mut state, &striker().id, slot(PlayerId::P1, Row::Backrow, 1), json!({}));
        assert_eq!(fusable_ids(&state, PlayerId::P1, Some(trap.id.as_str())), Vec::<String>::new());
        put(&mut state, &immutable.id, slot(PlayerId::P1, Row::Units, 1), json!({}));
        assert_eq!(fusable_ids(&state, PlayerId::P1, Some(trap.id.as_str())), Vec::<String>::new());
        let body = put(&mut state, &plain.id, slot(PlayerId::P1, Row::Units, 2), json!({}));
        assert_eq!(fusable_ids(&state, PlayerId::P1, Some(trap.id.as_str())), vec![body.id.clone()]);
        let mut all = fusable_ids(&state, PlayerId::P1, None);
        all.sort();
        let mut expected = vec![body.id.clone(), trap.id.clone()];
        expected.sort();
        assert_eq!(all, expected);
    }
}

mod r662_condition_active_rule_5_a_hand_card_glows_for_a_condition_another_card_grants {
    use super::*;

    #[test]
    fn r662_a_quickstriker_on_the_field_and_a_card_played_every_hand_card_glows_in_the_owners_view_only() {
        let mut state = board("gf-glow-striker");
        put(&mut state, &striker().id, slot(PlayerId::P1, Row::Backrow, 1), json!({}));
        let card = first_in_hand(&mut state, &plain.id);
        assert!(!condition::condition_active(&state, &live(&state, &card.id), PlayerId::P1, ConditionZone::Hand));
        played_one(&mut state, 3);
        assert!(condition::condition_active(&state, &live(&state, &card.id), PlayerId::P1, ConditionZone::Hand));
        let hand = view_for(&state, PlayerId::P1).you.hand;
        assert!(matches!(&hand, HandView::Cards(cards) if cards.iter().all(|view| view.condition_active == Some(true))));
        // Rule 3 still stands first: not in the other seat's turn.
        state.active = PlayerId::P2;
        assert!(!condition::condition_active(&state, &live(&state, &card.id), PlayerId::P1, ConditionZone::Hand));
    }

    #[test]
    fn r662_a_gifted_program_lights_the_hand_cards_it_would_make_radiant_and_not_the_rest() {
        let mut state = board("gf-glow-gifted");
        put(&mut state, &gifted().id, slot(PlayerId::P1, Row::Backrow, 1), json!({}));
        let cheap = first_in_hand(&mut state, &plain.id);
        let dear = first_in_hand(&mut state, &two_cost().id);
        assert!(condition::condition_active(&state, &live(&state, &cheap.id), PlayerId::P1, ConditionZone::Hand));
        assert!(!condition::condition_active(&state, &live(&state, &dear.id), PlayerId::P1, ConditionZone::Hand));
    }

    #[test]
    fn r662_a_granted_condition_never_lights_a_card_on_the_field() {
        let mut state = board("gf-glow-field");
        put(&mut state, &striker().id, slot(PlayerId::P1, Row::Backrow, 1), json!({}));
        let unit = put(&mut state, &plain.id, slot(PlayerId::P1, Row::Units, 1), json!({}));
        played_one(&mut state, 3);
        assert!(!condition::condition_active(&state, &live(&state, &unit.id), PlayerId::P1, ConditionZone::Field));
    }
}
