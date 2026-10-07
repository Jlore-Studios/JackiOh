//! C+ #73 Call to Chaos (Classic+ Edition)'s table (subsystems/callToChaosPlus.ts), rolled by Core #95's
//! subsystem (R28, R87, R423, R436): each of the ten entries against fixture pools, the Radiant's three
//! different entries in list order, the recursion counting casts of either edition and resolving into
//! nothing at the cap, and fused deck cards through JSON (R179, R468). The real card's test covers the
//! same cases against the real catalog.
//!
//! Port of `packages/engine/test/callToChaosPlus.test.ts`.

use jackioh_engine::subsystems::call_to_chaos::{
    CHAOS_CHAIN_KEY, CallToChaosArgs, ChaosEffectDef, call_to_chaos, cast_random_call_to_chaos, chaos_chain_of,
    roll_chaos_effects,
};
use jackioh_engine::subsystems::call_to_chaos_plus::CHAOS_PLUS_EFFECTS;
use jackioh_engine::testkit::*;
use jackioh_engine::wire::PlayerId::{P1, P2};

use crate::rules::fixtures::call_to_chaos_plus::{
    book, book_token, chaos_plus_catalog, classic, core95, fruit, golem, grape, hard_field, immutable, plus, trap,
};
use crate::rules::fixtures::harness::{events_of_type, in_hand, new_game, put, set_library, slot};

fn scripts(cry: Hook) -> CardScripts {
    CardScripts {
        base: Script {
            cry: Some(cry.clone()),
            ..Script::default()
        },
        radiant: Script {
            cry: Some(cry),
            ..Script::default()
        },
    }
}

/// TS `game(seed, { plus?, core? })`: the cries of the two editions' cast copies.
fn game(seed: &str, plus_cry: Option<Hook>, core_cry: Option<Hook>) -> GameState {
    let mut state = new_game(seed, None);
    register_catalog(chaos_plus_catalog(registered_catalog().clone()));
    let mut registered = registered_scripts().clone();
    registered.insert(
        plus.id.clone(),
        scripts(plus_cry.unwrap_or_else(|| hook(|_ctx| vec![call_to_chaos(plus_table())]))),
    );
    registered.insert(core95.id.clone(), scripts(core_cry.unwrap_or_else(|| hook(|_ctx| vec![]))));
    register_scripts(registered);
    state.turn = 4;
    state.active = P1;
    state.phase = Phase::Main;
    state.players.p1.hand = Vec::new();
    state.players.p2.hand = Vec::new();
    set_library(&mut state, P1, &[] as &[&str]);
    state
}

/// `{ table: CHAOS_PLUS_EFFECTS }`.
fn plus_table() -> CallToChaosArgs {
    CallToChaosArgs {
        table: Some(CHAOS_PLUS_EFFECTS),
        ..CallToChaosArgs::default()
    }
}

/// A C+ #73 mid-resolution, as `self` is while its Spell script runs (§10.5).
fn plus_card(state: &mut GameState, radiant: bool, chain: Option<i32>) -> CardInstance {
    let mut card = new_instance(&mut *state, &plus.id, P1, Zone::Resolving { player: P1 });
    if radiant {
        card.radiant = true;
    }
    if let Some(chain) = chain {
        card.memory.insert(CHAOS_CHAIN_KEY.to_string(), json!(chain));
    }
    card
}

fn entry(name: &str) -> Effect {
    let found = CHAOS_PLUS_EFFECTS
        .iter()
        .find(|effect| effect.name == name)
        .unwrap_or_else(|| panic!("no entry {name}"));
    (found.build)()
}

/// TS `run(state, effect, self = plusCard(state), controller = "p1")`: `None` is the omitted `self`,
/// a fresh C+ #73, made as TS's default made it.
fn run(state: &mut GameState, effect: Effect, self_: Option<CardInstance>) -> Vec<GameEvent> {
    let self_ = self_.unwrap_or_else(|| plus_card(state, false, None));
    let mut events: Vec<GameEvent> = Vec::new();
    let mut rng = Rng::new(&state.seed, state.rng_cursor);
    {
        let mut sink = EngineSink::new(&mut *state, &mut events, &mut rng);
        let options = HookOptions {
            controller: Some(P1),
            ..HookOptions::default()
        };
        {
            let mut ctx = make_context(&mut sink, Some(&self_), options);
            apply_effects(&[effect], &mut ctx);
        }
        state_check(&mut sink);
    }
    state.rng_cursor = rng.cursor();
    events
}

fn names_of<D: std::borrow::Borrow<ChaosEffectDef>>(effects: &[D]) -> Vec<String> {
    effects.iter().map(|effect| effect.borrow().name.to_string()).collect()
}

fn labels_of<D: std::borrow::Borrow<ChaosEffectDef>>(effects: &[D]) -> Vec<String> {
    effects.iter().map(|effect| effect.borrow().label.to_string()).collect()
}

/// A seed whose roll (base or Radiant) the predicate accepts.
fn seed_where(radiant: bool, accept: impl Fn(&[String]) -> bool, tag: &str) -> String {
    for i in 0..4000 {
        let seed = format!("{tag}-{i}");
        if accept(&names_of(&roll_chaos_effects(&mut Rng::new(&seed, 0), radiant, Some(CHAOS_PLUS_EFFECTS)))) {
            return seed;
        }
    }
    panic!("no seed for {tag}");
}

fn json_of<T: serde::Serialize>(value: T) -> Value {
    serde_json::to_value(value).unwrap()
}

/// One field of every event of a type, as JSON.
fn field_of(events: &[GameEvent], kind: GameEventType, field: &str) -> Vec<Value> {
    events_of_type(events, kind)
        .iter()
        .map(|event| json_of(event)[field].clone())
        .collect()
}

/// vitest's `toMatchObject`: every key the expected object names matches, recursively; an array
/// matches element for element and in length.
fn matches_object(actual: &Value, expected: &Value) -> bool {
    match (actual, expected) {
        (Value::Object(actual), Value::Object(expected)) => expected
            .iter()
            .all(|(key, value)| actual.get(key).is_some_and(|found| matches_object(found, value))),
        (Value::Array(actual), Value::Array(expected)) => {
            actual.len() == expected.len() && actual.iter().zip(expected).all(|(a, e)| matches_object(a, e))
        }
        _ => actual == expected,
    }
}

/// TS `Array.prototype.indexOf`: -1 when absent.
fn index_of(types: &[GameEventType], kind: GameEventType) -> i64 {
    types
        .iter()
        .position(|each| *each == kind)
        .map(|at| at as i64)
        .unwrap_or(-1)
}

/// The live card (TS held the object itself; Rust looks it up by id).
fn card<'a>(state: &'a GameState, id: &str) -> &'a CardInstance {
    find_instance(state, id).unwrap_or_else(|| panic!("no card {id}"))
}

fn sorted(mut ids: Vec<String>) -> Vec<String> {
    ids.sort();
    ids.dedup();
    ids
}

mod r423_c_plus_73s_table {
    use super::*;

    #[test]
    fn is_the_ten_entries_of_s8_7_row_73_in_the_cards_order_each_with_its_printed_clause() {
        assert_eq!(
            names_of(CHAOS_PLUS_EFFECTS),
            vec!["fruits", "books", "destroy", "classic", "upgrade", "fuse", "degrade", "golem", "replace", "recast"]
        );
        assert_eq!(
            labels_of(CHAOS_PLUS_EFFECTS),
            vec![
                "Add 5 random Fruits to your hand, which cost (0)",
                "Add 3 random Books to your hand, which cost (0)",
                "Destroy all enemy permanents",
                "Add 3 random Classic cards to your hand, which cost (0)",
                "Upgrade every card in your hand and deck twice",
                "Fuse a random card into each card in your deck, each keeping its cost",
                "Degrade every card on your opponent's field and in their hand three times",
                "Summon a Classic Golem",
                "Replace your deck with random Call to Chaos cards, which cost (0)",
                "Cast a random Call to Chaos",
            ]
        );
    }

    #[test]
    fn r382_r60_entry_1_5_fruits_that_cost_0_the_grapes_in_the_pool_repeats_allowed() {
        let mut seen: Vec<String> = Vec::new();
        for i in 0..6 {
            let mut state = game(&format!("fruits-{i}"), None, None);
            run(&mut state, entry("fruits"), None);
            let hand = &state.players.p1.hand;
            assert_eq!(hand.len() as i32, CHAOS_PLUS_FRUITS);
            for card in hand {
                assert!([fruit.id.clone(), grape.id.clone()].contains(&card.def_id));
                assert_eq!(card.cost_override, Some(0));
                seen.push(card.def_id.clone());
            }
        }
        assert_eq!(sorted(seen), sorted(vec![fruit.id.clone(), grape.id.clone()]));
    }

    #[test]
    fn s2_4_r4_a_full_hand_burns_what_entry_1_cannot_fit() {
        let mut state = game("fruits-full", None, None);
        in_hand(&mut state, "fx-1", P1, HAND_CAP - 2);
        let events = run(&mut state, entry("fruits"), None);
        assert_eq!(state.players.p1.hand.len() as i32, HAND_CAP);
        assert_eq!(
            events_of_type(&events, GameEventType::Burned).len() as i32,
            CHAOS_PLUS_FRUITS - 2
        );
    }

    #[test]
    fn r380_entry_2_3_non_token_books_of_any_set_which_cost_0() {
        let mut state = game("books", None, None);
        run(&mut state, entry("books"), None);
        assert_eq!(
            state.players.p1.hand.iter().map(|card| card.def_id.clone()).collect::<Vec<String>>(),
            (0..CHAOS_PLUS_BOOKS).map(|_| book.id.clone()).collect::<Vec<String>>()
        );
        assert!(!state.players.p1.hand.iter().any(|card| card.def_id == book_token.id));
        assert!(state.players.p1.hand.iter().all(|card| effective_cost(&state, card, CostOptions::default()) == 0));
    }

    #[test]
    fn r46_r59_entry_3_every_enemy_permanent_is_destroyed_the_top_of_each_pile_face_down_cards_too_indestructible_ones_staying()
     {
        let mut state = game("destroy", None, None);
        let unit = put(&mut state, "fx-2", slot(P2, Row::Units, 1), json!({}));
        let face_down = put(&mut state, &trap.id, slot(P2, Row::Backrow, 1), json!({}));
        let hard = put(&mut state, &hard_field.id, slot(P2, Row::Backrow, 2), json!({}));
        let mine = put(&mut state, "fx-3", slot(P1, Row::Units, 1), json!({}));
        run(&mut state, entry("destroy"), None);
        assert!(card(&state, &face_down.id).face_up != Some(true));
        assert_eq!(
            [&unit, &face_down, &hard, &mine].map(|each| card(&state, &each.id).zone.z()),
            [ZoneName::Graveyard, ZoneName::Graveyard, ZoneName::Field, ZoneName::Field]
        );
    }

    #[test]
    fn r380_entry_4_3_non_token_cards_of_the_classic_set_only_which_cost_0() {
        let mut state = game("classic", None, None);
        run(&mut state, entry("classic"), None);
        let hand = &state.players.p1.hand;
        assert_eq!(hand.len() as i32, CHAOS_PLUS_CLASSIC_CARDS);
        for card in hand {
            assert_eq!(def_of(Some(&state), &card.def_id).set, SetName::Classic);
            assert!(!def_of(Some(&state), &card.def_id).token);
            assert_eq!(card.cost_override, Some(0));
        }
    }

    #[test]
    fn r386_entry_5_two_separate_upgrades_of_each_card_in_your_hand_and_your_deck_none_of_the_opponents() {
        let mut state = game("upgrade", None, None);
        let hand = in_hand(&mut state, &classic.id, P1, 2);
        let deck = set_library(&mut state, P1, &[classic.id.as_str(), classic.id.as_str()]);
        let theirs = in_hand(&mut state, &classic.id, P2, 1);
        let events = run(&mut state, entry("upgrade"), None);
        assert_eq!(
            events_of_type(&events, GameEventType::Upgraded).len() as i32,
            (hand.len() + deck.len()) as i32 * CHAOS_PLUS_UPGRADES
        );
        assert_eq!(card(&state, &theirs[0].id).tuning, None);
        assert!(hand.iter().chain(deck.iter()).all(|each| {
            let now = card(&state, &each.id);
            now.tuning.is_some() || now.cost_mod != 0
        }));
    }

    #[test]
    fn r311_r177_entry_5s_deck_upgrades_stay_unread_by_the_decks_owner_too() {
        let mut state = game("upgrade-hidden", None, None);
        set_library(&mut state, P1, &[classic.id.as_str()]);
        let events = run(&mut state, entry("upgrade"), None);
        state.applied = vec![AppliedAction {
            nonce: "upgrade-hidden".into(),
            events,
        }];
        for viewer in [P1, P2] {
            let upgrades: Vec<Value> = view_for(&state, viewer)
                .events
                .iter()
                .filter(|event| event.event_type() == GameEventType::Upgraded)
                .map(json_of)
                .collect();
            assert!(!upgrades.is_empty());
            assert!(upgrades.iter().all(|event| event["instanceId"] == json!(HIDDEN_ID)));
        }
    }

    #[test]
    fn r77_r470_r387_r23_entry_6_a_random_card_fused_into_each_deck_card_which_keeps_its_cost_never_this_card_immutable_skipped()
     {
        let mut state = game("fuse", None, None);
        let deck = set_library(&mut state, P1, &[classic.id.as_str(), "fx-1", immutable.id.as_str()]);
        let events = run(&mut state, entry("fuse"), None);
        assert_eq!(events_of_type(&events, GameEventType::Fused).len(), 2);
        let library = state.players.p1.library.clone();
        let (first, second, third) = (&library[0], &library[1], &library[2]);
        assert_eq!(first.id, deck[0].id);
        assert_eq!(def_of(Some(&state), &first.def_id).type_, CardType::Unit);
        assert_eq!(effective_cost(&state, first, CostOptions::default()), 3);
        assert_eq!(effective_cost(&state, second, CostOptions::default()), 1);
        assert_eq!(third.def_id, immutable.id.clone());
        for each in [first, second] {
            assert!(state.transient_defs.contains_key(&each.def_id));
            assert!(
                !def_of(Some(&state), &each.def_id)
                    .ingredients
                    .iter()
                    .flatten()
                    .any(|part| part.def_id == plus.id)
            );
        }
    }

    #[test]
    fn r179_r468_entry_6s_fused_deck_cards_survive_json() {
        let mut state = game("fuse-json", None, None);
        set_library(&mut state, P1, &[classic.id.as_str(), "fx-1", "fx-2"]);
        run(&mut state, entry("fuse"), None);
        let round: GameState = serde_json::from_value(json_of(&state)).unwrap();
        assert_eq!(round, state);
        assert_eq!(hash_state(&round), hash_state(&state));
        assert!(round
            .players
            .p1
            .library
            .iter()
            .all(|card| round.transient_defs.contains_key(&card.def_id)));
    }

    #[test]
    fn r386_entry_7_three_separate_degrades_of_each_card_on_the_opponents_field_and_in_their_hand_none_of_yours() {
        let mut state = game("degrade", None, None);
        let field = put(&mut state, &classic.id, slot(P2, Row::Units, 1), json!({}));
        let hand = in_hand(&mut state, &classic.id, P2, 1);
        let mine = put(&mut state, &classic.id, slot(P1, Row::Units, 1), json!({}));
        let deck = set_library(&mut state, P2, &[classic.id.as_str()]);
        let events = run(&mut state, entry("degrade"), None);
        // Up to three each: a Degrade with nothing left it can change is no Degrade (B3.4 rule 1).
        let degraded: Vec<String> = field_of(&events, GameEventType::Degraded, "instanceId")
            .iter()
            .map(|id| id.as_str().unwrap_or_default().to_string())
            .collect();
        for id in [&field.id, &hand[0].id] {
            let times = degraded.iter().filter(|each| *each == id).count() as i32;
            assert!(times > 0);
            assert!(times <= CHAOS_PLUS_DEGRADES);
        }
        assert_eq!(sorted(degraded.clone()), sorted(vec![field.id.clone(), hand[0].id.clone()]));
        assert_eq!(card(&state, &mine.id).tuning, None);
        assert_eq!(card(&state, &deck[0].id).tuning, None);
        let field_now = card(&state, &field.id);
        assert!(field_now.tuning.is_some() || field_now.cost_mod != 0);
        let hand_now = card(&state, &hand[0].id);
        assert!(hand_now.tuning.is_some() || hand_now.cost_mod != 0);
    }

    #[test]
    fn r64_entry_8_a_classic_golem_summoned_into_the_leftmost_free_zone() {
        let mut state = game("golem", None, None);
        put(&mut state, "fx-1", slot(P1, Row::Units, 1), json!({}));
        let events = run(&mut state, entry("golem"), None);
        assert!(matches_object(
            &json_of(events_of_type(&events, GameEventType::Summoned)),
            &json!([{ "defId": golem.id, "lane": 2 }])
        ));
    }

    #[test]
    fn r35_r311_r387_entry_9_each_deck_card_replaced_one_for_one_by_a_call_to_chaos_of_either_edition_which_costs_0()
     {
        let mut seen: Vec<String> = Vec::new();
        for i in 0..4 {
            let mut state = game(&format!("replace-{i}"), None, None);
            // A copy: `setLibrary` hands back the library array itself.
            let old = set_library(&mut state, P1, &["fx-1", "fx-2", "fx-3", "fx-4", "fx-5"]);
            let events = run(&mut state, entry("replace"), None);
            assert_eq!(events_of_type(&events, GameEventType::Transformed).len(), old.len());
            let deck = &state.players.p1.library;
            assert_eq!(deck.len(), old.len());
            for card in deck {
                assert!([plus.id.clone(), core95.id.clone()].contains(&card.def_id));
                assert_eq!(card.cost_override, Some(0));
                assert_eq!(card.known_as, None);
                seen.push(card.def_id.clone());
            }
            // TS also read `zone.z === "gone"` off each old object it still held; a Rust test holds no
            // live handle on a card that has ceased to exist, so what it can read is that none is found.
            assert!(old.iter().all(|card| find_instance(&state, &card.id).is_none()));
        }
        assert_eq!(sorted(seen), sorted(vec![plus.id.clone(), core95.id.clone()]));
    }

    #[test]
    fn r129_entry_9_on_an_empty_deck_changes_nothing_and_draws_nothing() {
        let mut state = game("replace-empty", None, None);
        let me = plus_card(&mut state, false, None);
        let events = run(&mut state, entry("replace"), Some(me));
        assert!(events.is_empty());
        assert_eq!(state.rng_cursor, 0);
    }

    #[test]
    fn r28_r87_entry_10_the_chain_counts_casts_of_either_edition_against_the_cap() {
        let mut state = game(
            "chain",
            Some(hook(|_ctx| vec![cast_random_call_to_chaos()])),
            Some(hook(|_ctx| vec![cast_random_call_to_chaos()])),
        );
        let events = run(&mut state, entry("recast"), None);
        let played: Vec<String> = field_of(&events, GameEventType::CardPlayed, "defId")
            .iter()
            .map(|id| id.as_str().unwrap_or_default().to_string())
            .collect();
        assert_eq!(played.len() as i32, CALL_TO_CHAOS_CHAIN_CAP);
        assert_eq!(sorted(played), sorted(vec![plus.id.clone(), core95.id.clone()]));
    }

    #[test]
    fn r87_a_recursion_rolled_at_the_cap_resolves_into_nothing_the_radiants_other_two_still_run() {
        let seed = seed_where(
            true,
            |names| names.iter().any(|name| name == "recast") && names.iter().any(|name| name == "golem"),
            "cap",
        );
        let mut state = game(&seed, None, None);
        let me = plus_card(&mut state, true, Some(CALL_TO_CHAOS_CHAIN_CAP));
        let events = run(&mut state, call_to_chaos(plus_table()), Some(me.clone()));
        assert!(events_of_type(&events, GameEventType::CardPlayed).is_empty());
        assert_eq!(field_of(&events, GameEventType::Summoned, "defId"), vec![json!(golem.id.clone())]);
        assert_eq!(chaos_chain_of(Some(&me)), CALL_TO_CHAOS_CHAIN_CAP);
    }

    #[test]
    fn r423_the_base_face_rolls_one_entry_and_all_ten_are_reachable() {
        let mut reached: IndexSet<String> = IndexSet::new();
        for i in 0..200 {
            let rolled = roll_chaos_effects(&mut Rng::new(&format!("plus-base-{i}"), 0), false, Some(CHAOS_PLUS_EFFECTS));
            assert_eq!(rolled.len(), 1);
            reached.insert(names_of(&rolled).first().cloned().unwrap_or_default());
        }
        assert_eq!(reached.len(), CHAOS_PLUS_EFFECTS.len());
    }

    #[test]
    fn r423_the_radiant_face_rolls_three_different_entries_resolved_in_the_lists_order() {
        for i in 0..200 {
            let names = names_of(&roll_chaos_effects(
                &mut Rng::new(&format!("plus-radiant-{i}"), 0),
                true,
                Some(CHAOS_PLUS_EFFECTS),
            ));
            assert_eq!(names.iter().collect::<IndexSet<_>>().len() as i32, CALL_TO_CHAOS_RADIANT_EFFECTS);
            let order: Vec<i64> = names
                .iter()
                .map(|name| {
                    CHAOS_PLUS_EFFECTS
                        .iter()
                        .position(|effect| effect.name == name.as_str())
                        .map(|at| at as i64)
                        .unwrap_or(-1)
                })
                .collect();
            let mut in_order = order.clone();
            in_order.sort();
            assert_eq!(order, in_order);
        }
        let seed = seed_where(true, |names| names.join(",") == ["fruits", "golem", "recast"].join(","), "order");
        let mut state = game(&seed, None, None);
        let me = plus_card(&mut state, true, None);
        let events = run(&mut state, call_to_chaos(plus_table()), Some(me));
        let types: Vec<GameEventType> = events.iter().map(|event| event.event_type()).collect();
        assert!(index_of(&types, GameEventType::AddedToHand) < index_of(&types, GameEventType::Summoned));
        assert!(index_of(&types, GameEventType::Summoned) < index_of(&types, GameEventType::CardPlayed));
    }

    #[test]
    fn r436_both_players_are_told_the_rolled_clauses_by_this_card_before_any_of_it_resolves() {
        let seed = seed_where(true, |names| !names.iter().any(|name| name == "recast"), "announce");
        let mut state = game(&seed, None, None);
        let me = plus_card(&mut state, true, None);
        let events = run(&mut state, call_to_chaos(plus_table()), Some(me.clone()));
        let expected = labels_of(&roll_chaos_effects(&mut Rng::new(&seed, 0), true, Some(CHAOS_PLUS_EFFECTS)));
        assert_eq!(
            events.first().map(json_of),
            Some(json!({ "type": "chaosRolled", "player": "p1", "instanceId": me.id, "defId": plus.id, "effects": expected }))
        );
    }
}
