//! Port of `packages/engine/test/draw.test.ts`.

use jackioh_engine::draw::{draw, draw_one, shuffle_into_library, DrawOutcome, ShuffleInOutcome};
use jackioh_engine::effects::draw as draw_effect;
use jackioh_engine::testkit::*;

use crate::rules::fixtures::harness::{events_of_type, new_game, put, set_library, slot};
use crate::rules::fixtures::scripts::{anti_oneshot, cn_virus, hinder, infinite_reserves};

/// TS `sinkFor(state, events)`: a sink's events and rng (from the state's cursor), lent with the state
/// to one engine call at a time, so the test reads the state between calls as TS did.
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

fn strings(ids: &[&str]) -> Vec<String> {
    ids.iter().map(|id| id.to_string()).collect()
}

/// `eventsOfType(events, type)`, each event as its JSON, so a test reads its fields by TS's names.
fn of_type(events: &[GameEvent], kind: GameEventType) -> Vec<Value> {
    events_of_type(events, kind)
        .iter()
        .map(|event| serde_json::to_value(event).expect("an event serialises"))
        .collect()
}

fn def_ids(cards: &[CardInstance]) -> Vec<String> {
    cards.iter().map(|card| card.def_id.clone()).collect()
}

/// TS `drawFrom(state, library, count = 1)`: the state is drawn in place; the events come back.
fn draw_from(state: &mut GameState, library: &[String], count: i32) -> Vec<GameEvent> {
    set_library(state, PlayerId::P1, library);
    let mut bench = Bench::new(state);
    draw(&mut bench.sink(state), PlayerId::P1, count);
    bench.events
}

mod draw_m1_t7 {
    use super::*;

    #[test]
    fn takes_the_top_card_into_the_hand_and_counts_the_draw() {
        let mut state = new_game("engine-test", None);
        let events = draw_from(&mut state, &strings(&["fx-1", "fx-2"]), 1);
        assert_eq!(def_ids(&state.players.p1.hand), strings(&["fx-1"]));
        assert_eq!(def_ids(&state.players.p1.library), strings(&["fx-2"]));
        assert_eq!(state.counters.drawn, 1);
        assert_eq!(of_type(&events, GameEventType::Drawn).len(), 1);
    }

    #[test]
    fn casts_a_drawn_hinder_applies_the_opponents_modifier_and_the_hand_gains_the_next_card() {
        let mut state = new_game("engine-test", None);
        let events = draw_from(&mut state, &[hinder().id, "fx-2".to_string()], 1);

        assert_eq!(state.players.p2.mana.next_turn_mod, -1);
        assert_eq!(def_ids(&state.players.p1.hand), strings(&["fx-2"]));
        assert_eq!(def_ids(&state.players.p1.graveyard), vec![hinder().id]);
        assert_eq!(state.players.p1.turn_log.cards_played, 1);
        assert_eq!(state.counters.played, 1);
        assert_eq!(state.counters.drawn, 2);
        assert_eq!(
            of_type(&events, GameEventType::CardPlayed).first().map(|event| event["costPaid"].clone()),
            Some(json!(0))
        );
    }

    #[test]
    fn r3_deals_1_2_then_3_fatigue_damage_on_three_empty_draws() {
        let mut state = new_game("engine-test", None);
        let events = draw_from(&mut state, &[], 3);
        assert_eq!(state.players.p1.hero.health, HERO_HEALTH - 6);
        assert_eq!(state.players.p1.fatigue_count, 3);
        let amounts: Vec<Value> =
            of_type(&events, GameEventType::Damage).iter().map(|event| event["amount"].clone()).collect();
        assert_eq!(amounts, vec![json!(1), json!(2), json!(3)]);
    }

    #[test]
    fn r4_burns_the_eleventh_card_to_the_graveyard_while_still_counting_the_draw() {
        let mut state = new_game("engine-test", None);
        let library: Vec<String> = (0..HAND_CAP + 1).map(|i| format!("fx-{}", i + 1)).collect();
        let events = draw_from(&mut state, &library, HAND_CAP + 1);

        assert_eq!(state.players.p1.hand.len(), HAND_CAP as usize);
        assert_eq!(def_ids(&state.players.p1.graveyard), vec![format!("fx-{}", HAND_CAP + 1)]);
        assert_eq!(state.counters.drawn, HAND_CAP + 1);
        assert_eq!(of_type(&events, GameEventType::Burned).len(), 1);
    }

    #[test]
    fn cn_virus_deals_1_through_the_pipeline_shuffles_2_copies_and_draws_again() {
        let mut state = new_game("engine-test", None);
        let events = draw_from(&mut state, &[cn_virus().id, "fx-2".to_string()], 1);

        assert_eq!(state.players.p1.hero.health, HERO_HEALTH - 1);
        assert_eq!(state.players.p1.library.iter().filter(|c| c.def_id == cn_virus().id).count(), 2);
        assert_eq!(def_ids(&state.players.p1.hand), strings(&["fx-2"]));
        assert_eq!(of_type(&events, GameEventType::ShuffledIn).len(), 2);
        assert_eq!(def_ids(&state.players.p1.graveyard), vec![cn_virus().id]);
    }

    #[test]
    fn r63_anti_oneshot_armor_caps_the_heros_damage_and_armor_can_reduce_a_virus_to_nothing() {
        let mut state = new_game("engine-test", None);
        put(&mut state, &anti_oneshot().id, slot(PlayerId::P1, Row::Backrow, 1), Default::default());
        state.players.p1.hero.armor = 2;
        let events = draw_from(&mut state, &[cn_virus().id, "fx-2".to_string()], 1);

        assert_eq!(state.players.p1.hero.health, HERO_HEALTH);
        assert_eq!(of_type(&events, GameEventType::Damage).len(), 0);
        assert_eq!(state.players.p1.library.iter().filter(|c| c.def_id == cn_virus().id).count(), 2);
    }

    #[test]
    fn r58_stops_a_cast_on_draw_chain_at_the_cap_and_leaves_the_next_card_in_hand() {
        let mut state = new_game("engine-test", None);
        state.players.p1.hero.armor = 5; // keep the hero alive; the cap is what ends the chain
        let library: Vec<String> = (0..40).map(|_| cn_virus().id).collect();
        let events = draw_from(&mut state, &library, 1);

        assert_eq!(of_type(&events, GameEventType::CardPlayed).len(), CAST_ON_DRAW_CHAIN_CAP as usize);
        assert_eq!(def_ids(&state.players.p1.hand), vec![cn_virus().id]);
    }

    #[test]
    fn r217_counts_the_casts_of_a_draw_a_cast_makes_into_the_chain_that_cast_it_so_the_cap_bounds_them_all() {
        // A cast-on-draw Spell whose own text draws 1, the shape /fullsend's Combo draw gives any cast
        // (R70): every draw it makes lands on another one, nested inside the cast that made it.
        let drawer: CardDef = json_as(json!({
            "id": "fx-drawing-cast",
            "index": "fx-drawing-cast",
            "name": "Drawing cast (fixture)",
            "set": "Core",
            "type": "Spell",
            "tags": [],
            "rarity": "Common",
            "token": false,
            "cost": 0,
            "base": { "keywords": [], "text": "" },
            "radiant": { "keywords": [], "text": "" },
        }));
        let mut state = new_game("engine-test", None);
        let mut catalog = registered_catalog().clone();
        catalog.insert(drawer.id.clone(), drawer.clone());
        register_catalog(catalog);
        let script = Script {
            static_flags: Some(json_as(json!({ "castOnDraw": true }))),
            cry: Some(hook(|_ctx| vec![draw_effect(json_as(json!({ "count": 1 })))])),
            ..Script::default()
        };
        let mut scripts = registered_scripts().clone();
        scripts.insert(drawer.id.clone(), CardScripts { base: script.clone(), radiant: script });
        register_scripts(scripts);
        let library: Vec<String> = (0..40).map(|_| drawer.id.clone()).collect();
        let events = draw_from(&mut state, &library, 1);

        // One draw set all of it off, so it is one chain: 20 casts, however deeply they nested, and the
        // cast-on-draw card each open draw then met went to the hand uncast (R58).
        assert_eq!(of_type(&events, GameEventType::CardPlayed).len(), CAST_ON_DRAW_CHAIN_CAP as usize);
        assert!(state.players.p1.hand.iter().all(|card| card.def_id == drawer.id));
        // The draw that began the chain closed it (R217).
        assert_eq!(state.cast_chain, None);
    }

    #[test]
    fn infinite_reserves_turns_an_empty_library_draw_into_a_rush_token_card_with_no_fatigue_c75() {
        let mut state = new_game("engine-test", None);
        put(&mut state, &infinite_reserves().id, slot(PlayerId::P1, Row::Backrow, 2), Default::default());
        let events = draw_from(&mut state, &[], 1);

        assert_eq!(state.players.p1.fatigue_count, 0);
        assert_eq!(state.players.p1.hero.health, HERO_HEALTH);
        assert_eq!(state.players.p1.hand.len(), 1);
        assert_eq!(of_type(&events, GameEventType::Damage).len(), 0);
    }

    #[test]
    fn r58_casts_a_cast_on_draw_card_even_when_the_hand_is_full() {
        let mut state = new_game("engine-test", None);
        // Fill the hand to the cap, then draw Hinder with one more card behind it.
        for _ in 0..HAND_CAP {
            let filler = new_instance(&mut state, "fx-1", PlayerId::P1, Zone::Hand { player: PlayerId::P1 });
            state.players.p1.hand.push(filler);
        }
        let events = draw_from(&mut state, &[hinder().id, "fx-2".to_string()], 1);

        assert_eq!(state.players.p1.hand.len(), HAND_CAP as usize);
        assert_eq!(state.players.p2.mana.next_turn_mod, -1); // it was cast, not burned
        assert_eq!(of_type(&events, GameEventType::CardPlayed).len(), 1);
        // The card drawn behind it had nowhere to go, so that one burned.
        assert_eq!(of_type(&events, GameEventType::Burned).len(), 1);
        assert!(def_ids(&state.players.p1.graveyard).contains(&hinder().id));
    }

    #[test]
    fn draw_one_reports_what_happened() {
        let mut state = new_game("engine-test", None);
        set_library(&mut state, PlayerId::P1, &[hinder().id, "fx-2".to_string()]);
        let mut bench = Bench::new(&state);
        assert_eq!(draw_one(&mut bench.sink(&mut state), PlayerId::P1, None), DrawOutcome::Cast);
        assert_eq!(draw_one(&mut bench.sink(&mut state), PlayerId::P1, None), DrawOutcome::Fatigue);
    }
}

mod r80_library_cap_m1_t7 {
    use super::*;

    #[test]
    fn creates_nothing_once_a_library_holds_60_cards() {
        let mut state = new_game("engine-test", None);
        let full: Vec<String> = (0..LIBRARY_CAP).map(|_| "fx-1".to_string()).collect();
        set_library(&mut state, PlayerId::P1, &full);
        let mut bench = Bench::new(&state);
        let mut fresh = new_instance(&mut state, "fx-2", PlayerId::P1, Zone::Library { player: PlayerId::P1 });

        let outcome = shuffle_into_library(&mut bench.sink(&mut state), &mut fresh, false, None);
        assert!(matches!(outcome, ShuffleInOutcome::Dropped));
        assert_eq!(state.players.p1.library.len(), LIBRARY_CAP as usize);
        assert_eq!(state.players.p1.graveyard.len(), 0);
    }

    #[test]
    fn sends_an_existing_card_to_its_owners_graveyard_instead() {
        let mut state = new_game("engine-test", None);
        let full: Vec<String> = (0..LIBRARY_CAP).map(|_| "fx-1".to_string()).collect();
        set_library(&mut state, PlayerId::P1, &full);
        let mut bench = Bench::new(&state);
        let mut card = new_instance(&mut state, "fx-2", PlayerId::P1, Zone::Hand { player: PlayerId::P1 });
        state.players.p1.hand.push(card.clone());

        let outcome = shuffle_into_library(&mut bench.sink(&mut state), &mut card, true, None);
        assert!(matches!(outcome, ShuffleInOutcome::Dropped));
        let ids: Vec<String> = state.players.p1.graveyard.iter().map(|c| c.id.clone()).collect();
        assert_eq!(ids, vec![card.id.clone()]);
    }

    #[test]
    fn shuffles_into_a_random_position_below_the_cap() {
        let mut state = new_game("engine-test", None);
        set_library(&mut state, PlayerId::P1, &strings(&["fx-1", "fx-2", "fx-3"]));
        let mut bench = Bench::new(&state);
        let mut card = new_instance(&mut state, "fx-4", PlayerId::P1, Zone::Library { player: PlayerId::P1 });

        let outcome = shuffle_into_library(&mut bench.sink(&mut state), &mut card, false, None);
        assert!(matches!(outcome, ShuffleInOutcome::Library));
        assert_eq!(state.players.p1.library.len(), 4);
        assert!(state.players.p1.library.iter().any(|c| c.id == card.id));
    }
}
