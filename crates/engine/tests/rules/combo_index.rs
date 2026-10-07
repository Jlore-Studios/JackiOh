//! Combo-Index (BUILD M3-T7): SPEC §8 #93, R27, R60, R62 and §4.4 step 8.
//!
//! The card is a Field Spell with a grade counter on its instance. At the end of its controller's
//! turn, "if cards played this turn >= grade, grade +1 and run every step from E up to the new
//! grade" (§8 #93), the steps run in order and S is terminal (R27).
//!
//! Port of `packages/engine/test/comboIndex.test.ts`.

use jackioh_engine::subsystems::combo_index::{
    GRADES, Grade, RaiseGradeArgs, StartGradeArgs, cascade_effects, combo_index_end_of_turn, grade_name,
    grade_name_of, grade_of, grade_rises, grade_step_effects, grade_value, is_terminal_grade,
    played_cards_this_turn, plays_this_turn, raise_grade, start_grade, step_a, step_b, step_c, step_d,
    step_e,
};
use jackioh_engine::testkit::*;
use jackioh_engine::wire::PlayerId::{P1, P2};

use crate::rules::fixtures::harness::{events_of_type, in_hand, new_game, put, slot};

/// TS's `def(name, type, extra)`, its running index written out (TS counted from 930 in file order).
fn def(name: &str, index: i32, type_: &str, extra: Value) -> CardDef {
    let mut out = json!({
        "id": format!("ci-{name}"),
        "index": index.to_string(),
        "name": name,
        "set": "Core",
        "type": type_,
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": 2,
        "base": { "keywords": [], "text": name },
        "radiant": { "keywords": [], "text": name },
    });
    if let (Some(into), Some(from)) = (out.as_object_mut(), extra.as_object()) {
        for (key, value) in from {
            into.insert(key.clone(), value.clone());
        }
    }
    json_as(out)
}

/// #93 Combo-Index itself: a Field Spell whose end-of-turn hook is the subsystem (§8, R62).
fn combo_index() -> CardDef {
    def(
        "combo-index",
        931,
        "Field Spell",
        json!({ "cost": 2, "rarity": "Legendary" }),
    )
}
/// Anything cheap to play and to hold: a Spell lands in the graveyard, so a copy can still be made.
fn trick() -> CardDef {
    def("trick", 932, "Spell", json!({ "cost": 1 }))
}
fn other_trick() -> CardDef {
    def("other-trick", 933, "Spell", json!({ "cost": 3 }))
}
fn body() -> CardDef {
    def(
        "body",
        934,
        "Unit",
        json!({
            "cost": 2,
            "base": { "attack": 2, "health": 2, "keywords": [], "text": "body" },
            "radiant": { "attack": 4, "health": 4, "keywords": [], "text": "body" },
        }),
    )
}

fn both(script: Script) -> CardScripts {
    CardScripts {
        base: script.clone(),
        radiant: script,
    }
}

fn scripts() -> Vec<(String, CardScripts)> {
    vec![(
        combo_index().id,
        both(Script {
            end_of_turn: Some(hook(|ctx| match ctx.self_.clone() {
                None => vec![],
                Some(me) => combo_index_end_of_turn(ctx, &me),
            })),
            ..Script::default()
        }),
    )]
}

fn game(seed: &str) -> GameState {
    let mut state = new_game(seed, None);
    let mut catalog = registered_catalog().clone();
    for def in [combo_index(), trick(), other_trick(), body()] {
        catalog.insert(def.id.clone(), def);
    }
    register_catalog(catalog);
    let mut registered = registered_scripts().clone();
    for (id, script) in scripts() {
        registered.insert(id, script);
    }
    register_scripts(registered);
    state.turn = 4;
    state.active = P1;
    state.phase = Phase::Main;
    state
}

/// The card on the field, at the grade the scenario needs.
fn on_field(state: &mut GameState, grade: Option<i32>) -> CardInstance {
    let card = put(state, &combo_index().id, slot(P1, Row::Backrow, 1), json!({}));
    if let Some(grade) = grade {
        find_instance_mut(state, &card.id)
            .expect("on the field")
            .counters
            .grade = Some(grade);
    }
    find_instance(state, &card.id).expect("on the field").clone()
}

/// Play a card for real: R70's cast counts as a play for everything that counts plays, so this is
/// the same `turnLog` bookkeeping §10.5 step 4 writes, not a parallel record.
fn play(sink: &mut EngineSink<'_>, def_id: &str, radiant: bool) -> CardInstance {
    let mut card = in_hand(sink.state, def_id, P1, 1).remove(0);
    if radiant {
        find_instance_mut(sink.state, &card.id).expect("in hand").radiant = true;
        card.radiant = true;
    }
    cast_card(sink, &card, CastOptions::default());
    card
}

fn run(sink: &mut EngineSink<'_>, self_: &CardInstance, effects: Vec<Effect>) {
    let current = find_instance(sink.state, &self_.id)
        .cloned()
        .unwrap_or_else(|| self_.clone());
    let mut ctx = make_context(sink, Some(&current), HookOptions::default());
    apply_effects(&effects, &mut ctx);
}

fn kinds_of(effects: &[Effect]) -> Vec<&'static str> {
    effects.iter().map(|effect| effect.kind).collect()
}

/// The five steps as the cascade names them. Grade B is the library's own `setRadiantRandom` (R60)
/// and grade A is a `damage` effect that states its Lifesteal (R85), so both carry library kinds.
const STEP_KINDS: [&str; 5] = [
    "comboIndexStepE",
    "comboIndexStepD",
    "comboIndexStepC",
    "setRadiantRandom",
    "damage",
];

fn json_of<T: serde::Serialize>(value: T) -> Value {
    serde_json::to_value(value).unwrap()
}

/// The live card (TS held the object itself; Rust looks it up by id).
fn card<'a>(state: &'a GameState, id: &str) -> &'a CardInstance {
    find_instance(state, id).unwrap_or_else(|| panic!("no card {id}"))
}

fn grade_now(state: &GameState, id: &str) -> Option<i32> {
    card(state, id).counters.grade
}

fn sink_events(state: &mut GameState, events: &mut Vec<GameEvent>, f: impl FnOnce(&mut EngineSink<'_>)) {
    let mut rng = Rng::new(&state.seed, state.rng_cursor);
    let mut sink = EngineSink::new(state, events, &mut rng);
    f(&mut sink);
}

mod r27_combo_index_s8_93_m3_t7 {
    use super::*;

    #[test]
    fn s8_93_keeps_the_grade_in_counters_grade_starting_at_e_and_names_e_d_c_b_a_s_for_1_to_6() {
        let mut state = game("grade-names");
        let mut events: Vec<GameEvent> = Vec::new();
        let card_id = {
            let mut rng = Rng::new(&state.seed, state.rng_cursor);
            let mut sink = EngineSink::new(&mut state, &mut events, &mut rng);
            let card = on_field(sink.state, None);

            // An instance that has never written the counter is at E (§8 #93 "starts at E").
            assert_eq!(card.counters.grade, None);
            assert_eq!(grade_of(&card), 1);
            assert_eq!(json_of(grade_name_of(&card)), json!("E"));

            assert_eq!(json_of(GRADES), json!(["E", "D", "C", "B", "A", "S"]));
            assert_eq!(
                json_of([1, 2, 3, 4, 5, 6].map(grade_name)),
                json!(["E", "D", "C", "B", "A", "S"])
            );
            assert_eq!(
                GRADES
                    .iter()
                    .map(|grade| grade_value(*grade))
                    .collect::<Vec<i32>>(),
                vec![1, 2, 3, 4, 5, 6]
            );
            assert_eq!(
                [1, 2, 3, 4, 5].map(is_terminal_grade),
                [false, false, false, false, false]
            );
            assert!(is_terminal_grade(6));

            // The counter is plain state, so the client sees it and a replay round-trips it (§10.1).
            run(&mut sink, &card, vec![start_grade(StartGradeArgs::default())]);
            card.id
        };
        assert_eq!(grade_now(&state, &card_id), Some(1));
        assert_eq!(
            json_of(events_of_type(&events, GameEventType::CounterChanged)),
            json!([{ "type": "counterChanged", "instanceId": card_id, "counter": "grade", "value": 1 }])
        );
        assert_eq!(
            serde_json::from_value::<GameState>(json_of(&state)).unwrap(),
            state
        );

        sink_events(&mut state, &mut events, |sink| {
            let card = card(sink.state, &card_id).clone();
            run(sink, &card, vec![raise_grade(RaiseGradeArgs::default())]);
        });
        assert_eq!(grade_now(&state, &card_id), Some(2));
        assert_eq!(json_of(grade_name_of(card(&state, &card_id))), json!("D"));
    }

    #[test]
    fn s8_93_does_nothing_at_the_end_of_a_turn_with_fewer_plays_than_the_grade() {
        let mut state = game("below-threshold");
        let mut events: Vec<GameEvent> = Vec::new();
        let mut rng = Rng::new(&state.seed, state.rng_cursor);
        let mut sink = EngineSink::new(&mut state, &mut events, &mut rng);
        let card = on_field(sink.state, Some(grade_value(Grade::C))); // grade 3 needs 3 plays

        play(&mut sink, &trick().id, false);
        play(&mut sink, &trick().id, false);
        assert_eq!(plays_this_turn(sink.state, P1), 2);

        let card = super::card(sink.state, &card.id).clone();
        assert!(!grade_rises(sink.state, &card));
        let effects = combo_index_end_of_turn(&sink, &card);
        assert!(effects.is_empty());

        run(&mut sink, &card, effects);
        assert_eq!(grade_now(sink.state, &card.id), Some(3));
        assert!(events_of_type(sink.events, GameEventType::CounterChanged).is_empty());
    }

    #[test]
    fn s8_93_raises_the_grade_when_the_plays_reach_it_grade_1_needs_1_play_grade_2_needs_2() {
        let mut first = game("at-threshold-1");
        sink_events(&mut first, &mut Vec::new(), |sink| {
            let at_e = on_field(sink.state, None); // grade 1
            play(sink, &trick().id, false);
            assert!(grade_rises(sink.state, &at_e));
            let effects = combo_index_end_of_turn(sink, &at_e);
            run(sink, &at_e, effects);
            assert_eq!(grade_now(sink.state, &at_e.id), Some(2));
        });

        let mut second = game("at-threshold-2");
        sink_events(&mut second, &mut Vec::new(), |sink| {
            let at_d = on_field(sink.state, Some(grade_value(Grade::D))); // grade 2
            play(sink, &trick().id, false);
            // One play is short of 2, so nothing happens; the second play reaches the threshold.
            assert!(!grade_rises(sink.state, &at_d));
            play(sink, &trick().id, false);
            assert_eq!(plays_this_turn(sink.state, P1), 2);
            assert!(grade_rises(sink.state, &at_d));
            let effects = combo_index_end_of_turn(sink, &at_d);
            run(sink, &at_d, effects);
            assert_eq!(grade_now(sink.state, &at_d.id), Some(3));
        });
    }

    #[test]
    fn r27_runs_every_step_from_e_up_to_the_new_grade_in_order() {
        let mut state = game("cascade-order");
        sink_events(&mut state, &mut Vec::new(), |sink| {
            let card = on_field(sink.state, Some(grade_value(Grade::D))); // rises to C
            play(sink, &trick().id, false);
            play(sink, &trick().id, false);

            // The rise first, then E, D and C — never a step above the new grade (R27).
            let mut expected = vec!["comboIndexRaiseGrade"];
            expected.extend_from_slice(&STEP_KINDS[..3]);
            assert_eq!(kinds_of(&combo_index_end_of_turn(sink, &card)), expected);
        });

        // The cascade of each grade on its own, so "E→new grade in order" is the whole list every time.
        assert_eq!(
            kinds_of(&cascade_effects(grade_value(Grade::E))),
            STEP_KINDS[..1].to_vec()
        );
        assert_eq!(
            kinds_of(&cascade_effects(grade_value(Grade::B))),
            STEP_KINDS[..4].to_vec()
        );
        assert_eq!(
            kinds_of(&cascade_effects(grade_value(Grade::A))),
            STEP_KINDS.to_vec()
        );
        // S is "run E–A again", so reaching S runs the five steps twice and never itself (R27).
        assert_eq!(
            kinds_of(&grade_step_effects(grade_value(Grade::S))),
            STEP_KINDS.to_vec()
        );
        assert_eq!(
            kinds_of(&cascade_effects(grade_value(Grade::S))),
            [STEP_KINDS, STEP_KINDS].concat()
        );
    }

    #[test]
    fn r27_the_e_to_s_cascade_runs_each_step_in_order_once_it_reaches_s() {
        let mut state = game("full-cascade");
        let mut events: Vec<GameEvent> = Vec::new();
        let (card_id, hand_len, from) = {
            let mut rng = Rng::new(&state.seed, state.rng_cursor);
            let mut sink = EngineSink::new(&mut state, &mut events, &mut rng);
            let card = on_field(sink.state, Some(grade_value(Grade::A))); // rises to S
            let hand = in_hand(sink.state, &trick().id, P1, 5);
            in_hand(sink.state, &other_trick().id, P2, 3);
            sink.state.players.p1.hero.health = 20;
            for _ in 0..5 {
                play(&mut sink, &trick().id, false);
            }
            let from = sink.events.len();

            let effects = combo_index_end_of_turn(&sink, &card);
            run(&mut sink, &card, effects);
            (card.id, hand.len(), from)
        };

        assert_eq!(grade_now(&state, &card_id), Some(6));
        assert_eq!(json_of(grade_name_of(card(&state, &card_id))), json!("S"));
        // Two full rounds of E, D, C, B, A, in order, after the counter moves (R27).
        let types: Vec<Value> = events[from..]
            .iter()
            .map(|event| json_of(event)["type"].clone())
            .collect();
        assert_eq!(
            Value::Array(types),
            json!([
                "counterChanged",
                "addedToHand",
                "costChanged",
                "costChanged",
                "exiled",
                "radiantSet",
                "damage",
                "healed",
                "addedToHand",
                "costChanged",
                "costChanged",
                "exiled",
                "radiantSet",
                "damage",
                "healed",
            ])
        );

        // Each step landed twice: 2 copies added, 2 enemy cards exiled, 2 cards radiant, 2 x 8 damage.
        assert_eq!(state.players.p1.hand.len(), hand_len + 2);
        assert_eq!(state.players.p2.hand.len(), 1);
        assert_eq!(state.players.p2.exile.len(), 2);
        assert_eq!(
            state.players.p1.hand.iter().filter(|held| held.radiant).count(),
            2
        );
        assert_eq!(state.players.p2.hero.health, 30 - 16);
        assert_eq!(state.players.p1.hero.health, 20 + 16);
    }

    #[test]
    fn r27_grade_s_is_terminal_so_the_end_of_turn_check_does_nothing_at_s() {
        let mut state = game("terminal-s");
        let mut events: Vec<GameEvent> = Vec::new();
        let mut rng = Rng::new(&state.seed, state.rng_cursor);
        let mut sink = EngineSink::new(&mut state, &mut events, &mut rng);
        let card = on_field(sink.state, Some(grade_value(Grade::S)));
        in_hand(sink.state, &trick().id, P1, 3);
        in_hand(sink.state, &other_trick().id, P2, 3);
        for _ in 0..10 {
            play(&mut sink, &trick().id, false);
        }
        let before: GameState = serde_json::from_value(json_of(&*sink.state)).unwrap();
        let events_before = sink.events.len();

        assert_eq!(plays_this_turn(sink.state, P1), 10);
        assert!(!grade_rises(sink.state, &card));
        let effects = combo_index_end_of_turn(&sink, &card);
        assert!(effects.is_empty());
        run(&mut sink, &card, effects);

        // No rise and no step: the state is exactly what it was (R27 "nothing further happens at S").
        assert_eq!(grade_now(sink.state, &card.id), Some(6));
        assert_eq!(*sink.state, before);
        assert_eq!(sink.events.len(), events_before);

        // Even asked directly, the counter refuses to pass S.
        run(&mut sink, &card, vec![raise_grade(RaiseGradeArgs::default())]);
        assert_eq!(grade_now(sink.state, &card.id), Some(6));
    }

    #[test]
    fn r27_grade_e_adds_a_fresh_copy_of_a_random_card_played_this_turn_radiant_flag_kept() {
        let mut state = game("step-e");
        let mut events: Vec<GameEvent> = Vec::new();
        let played = {
            let mut rng = Rng::new(&state.seed, state.rng_cursor);
            let mut sink = EngineSink::new(&mut state, &mut events, &mut rng);
            let card = on_field(sink.state, None);
            let played = play(&mut sink, &trick().id, true);
            assert_eq!(
                played_cards_this_turn(sink.state, P1)
                    .iter()
                    .map(|held| held.id.clone())
                    .collect::<Vec<String>>(),
                vec![played.id.clone()]
            );

            run(&mut sink, &card, grade_step_effects(grade_value(Grade::E)));
            played
        };

        let copy = state.players.p1.hand[0].clone();
        assert_eq!(state.players.p1.hand.len(), 1);
        assert_ne!(copy.id, played.id); // a fresh instance, not the card itself (R57)
        assert_eq!(copy.def_id, trick().id);
        assert!(copy.radiant); // R27: the radiant flag is kept
        assert_eq!(copy.zone, Zone::Hand { player: P1 });
        assert_eq!(copy.damage, 0);
        assert_eq!(json_of(copy.counters), json!({}));
        // The card it copied is untouched, still in the graveyard where its cast left it.
        assert_eq!(
            state
                .players
                .p1
                .graveyard
                .iter()
                .map(|held| held.id.clone())
                .collect::<Vec<String>>(),
            vec![played.id.clone()]
        );
        let added: Vec<Value> = events_of_type(&events, GameEventType::AddedToHand)
            .iter()
            .map(|event| json_of(event)["instanceId"].clone())
            .collect();
        assert_eq!(added, vec![json!(copy.id)]);

        // With nothing played this turn there is nothing to copy and the step does nothing.
        let mut empty = game("step-e-empty");
        sink_events(&mut empty, &mut Vec::new(), |sink| {
            let idle = on_field(sink.state, None);
            run(sink, &idle, vec![step_e()]);
        });
        assert!(empty.players.p1.hand.is_empty());
    }

    #[test]
    fn r60_grade_d_makes_2_different_random_hand_cards_cost_1_less() {
        let mut state = game("step-d");
        let hand_ids: Vec<String> = {
            let mut events: Vec<GameEvent> = Vec::new();
            let mut rng = Rng::new(&state.seed, state.rng_cursor);
            let mut sink = EngineSink::new(&mut state, &mut events, &mut rng);
            let card = on_field(sink.state, None);
            let hand = in_hand(sink.state, &trick().id, P1, 4);

            run(&mut sink, &card, vec![step_d()]);
            hand.into_iter().map(|held| held.id).collect()
        };

        let discounted: Vec<CardInstance> = hand_ids
            .iter()
            .map(|id| card(&state, id).clone())
            .filter(|held| held.cost_mod == -1)
            .collect();
        assert_eq!(discounted.len(), 2); // R60: N different cards
        assert_eq!(
            discounted
                .iter()
                .map(|held| held.id.clone())
                .collect::<IndexSet<String>>()
                .len(),
            2
        );
        let trick_cost = match trick().cost {
            CardCost::Fixed(cost) => cost,
            other => panic!("trick costs {other:?}"),
        };
        assert_eq!(
            effective_cost(&state, &discounted[0], CostOptions::default()),
            trick_cost - 1
        );
        assert_eq!(
            hand_ids
                .iter()
                .filter(|id| card(&state, id).cost_mod == 0)
                .count(),
            2
        );

        // R60: fewer cards than asked for means all of them, and an empty hand means nothing.
        let mut small = game("step-d-small");
        let mut only_id = String::new();
        sink_events(&mut small, &mut Vec::new(), |sink| {
            let small_card = on_field(sink.state, None);
            only_id = in_hand(sink.state, &trick().id, P1, 1).remove(0).id;
            run(sink, &small_card, vec![step_d()]);
        });
        assert_eq!(card(&small, &only_id).cost_mod, -1);

        // TS `expect(() => run(…)).not.toThrow()`: a panic here fails the test.
        let mut none = game("step-d-empty");
        sink_events(&mut none, &mut Vec::new(), |sink| {
            let none_card = on_field(sink.state, None);
            run(sink, &none_card, vec![step_d()]);
        });
        assert!(none.players.p1.hand.is_empty());
    }

    #[test]
    fn s8_93_grade_c_exiles_a_random_card_from_the_opponents_hand() {
        let mut state = game("step-c");
        let mut events: Vec<GameEvent> = Vec::new();
        let exiled_before = state.counters.exiled;
        let (mine, theirs) = {
            let mut rng = Rng::new(&state.seed, state.rng_cursor);
            let mut sink = EngineSink::new(&mut state, &mut events, &mut rng);
            let card = on_field(sink.state, None);
            let mine = in_hand(sink.state, &trick().id, P1, 2);
            let theirs = in_hand(sink.state, &other_trick().id, P2, 3);

            run(&mut sink, &card, vec![step_c()]);
            (mine, theirs)
        };

        assert_eq!(state.players.p2.hand.len(), 2);
        assert_eq!(state.players.p2.exile.len(), 1);
        let gone = state.players.p2.exile[0].clone();
        assert!(theirs.iter().map(|held| held.id.clone()).any(|id| id == gone.id));
        assert_eq!(gone.zone, Zone::Exile { player: P2 });
        assert_eq!(state.counters.exiled, exiled_before + 1); // R55 counts it
        let exiled: Vec<Value> = events_of_type(&events, GameEventType::Exiled)
            .iter()
            .map(|event| json_of(event)["instanceId"].clone())
            .collect();
        assert_eq!(exiled, vec![json!(gone.id)]);
        // It is the opponent's hand, never the controller's.
        assert_eq!(
            state
                .players
                .p1
                .hand
                .iter()
                .map(|held| held.id.clone())
                .collect::<Vec<String>>(),
            mine.iter().map(|held| held.id.clone()).collect::<Vec<String>>()
        );

        // An empty enemy hand leaves the step with nothing to do.
        let mut empty = game("step-c-empty");
        sink_events(&mut empty, &mut Vec::new(), |sink| {
            let card = on_field(sink.state, None);
            run(sink, &card, vec![step_c()]);
        });
        assert!(empty.players.p2.exile.is_empty());
    }

    #[test]
    fn r60_r177_grade_b_picks_only_among_non_radiant_hand_cards_and_changes_none_when_none_are_left_though_it_cues_the_hand()
     {
        let mut state = game("step-b");
        let (card_id, hand_ids) = {
            let mut events: Vec<GameEvent> = Vec::new();
            let mut rng = Rng::new(&state.seed, state.rng_cursor);
            let mut sink = EngineSink::new(&mut state, &mut events, &mut rng);
            let card = on_field(sink.state, None);
            let hand = in_hand(sink.state, &trick().id, P1, 4);
            for held in &hand[1..] {
                find_instance_mut(sink.state, &held.id).expect("in hand").radiant = true;
            }

            // Only one non-Radiant card is eligible, so the random pick must land on it (R60).
            run(&mut sink, &card, vec![step_b()]);
            (
                card.id,
                hand.into_iter().map(|held| held.id).collect::<Vec<String>>(),
            )
        };
        assert!(card(&state, &hand_ids[0]).radiant);
        assert!(hand_ids.iter().all(|id| card(&state, id).radiant));

        // With every hand card Radiant the pool is empty and nothing changes (R60): no card and no
        // random number (R129). The hidden hand is still cued once, as a pick would cue it (R177).
        let mut events_after: Vec<GameEvent> = Vec::new();
        {
            let mut rng = Rng::new(&state.seed, state.rng_cursor);
            let mut second_sink = EngineSink::new(&mut state, &mut events_after, &mut rng);
            let cursor = second_sink.rng.cursor();
            let me = card(second_sink.state, &card_id).clone();
            run(&mut second_sink, &me, vec![step_b()]);
            assert_eq!(second_sink.rng.cursor(), cursor);
        }
        assert_eq!(
            events_after
                .iter()
                .map(|event| event.event_type())
                .collect::<Vec<GameEventType>>(),
            vec![GameEventType::RadiantSet]
        );

        // A Radiant card on the field is not in the hand pool either.
        let mut field = game("step-b-field");
        let mut unit_id = String::new();
        sink_events(&mut field, &mut Vec::new(), |sink| {
            let field_card = on_field(sink.state, None);
            unit_id = put(sink.state, &body().id, slot(P1, Row::Units, 1), json!({})).id;
            run(sink, &field_card, vec![step_b()]);
        });
        assert!(!card(&field, &unit_id).radiant);
    }

    #[test]
    fn s4_4_step_8_grade_a_deals_8_to_the_enemy_hero_with_lifesteal_healing_the_controllers_hero() {
        let mut state = game("step-a");
        let mut events: Vec<GameEvent> = Vec::new();
        let card_id = {
            let mut rng = Rng::new(&state.seed, state.rng_cursor);
            let mut sink = EngineSink::new(&mut state, &mut events, &mut rng);
            let card = on_field(sink.state, None);
            sink.state.players.p1.hero.health = 12;

            run(&mut sink, &card, vec![step_a()]);
            card.id
        };

        assert_eq!(state.players.p2.hero.health, 22);
        assert_eq!(state.players.p1.hero.health, 20); // 12 + the 8 it dealt
        assert_eq!(
            json_of(events_of_type(&events, GameEventType::Damage)),
            json!([{ "type": "damage", "sourceId": card_id, "targetId": "hero-p2", "amount": 8, "combat": false }])
        );
        assert_eq!(
            json_of(events_of_type(&events, GameEventType::Healed)),
            json!([{ "type": "healed", "targetId": "hero-p1", "amount": 8 }])
        );

        // §4.4 step 8 heals by the amount dealt, so Armor cuts the heal with the hit.
        let mut armored = game("step-a-armor");
        sink_events(&mut armored, &mut Vec::new(), |sink| {
            let armored_card = on_field(sink.state, None);
            sink.state.players.p2.hero.armor = 3;
            sink.state.players.p1.hero.health = 10;
            run(sink, &armored_card, vec![step_a()]);
        });
        assert_eq!(armored.players.p2.hero.health, 25);
        assert_eq!(armored.players.p1.hero.health, 15);
    }

    #[test]
    fn s10_7_draws_every_pick_from_the_seeded_rng_one_seed_repeats_another_seed_differs() {
        fn cascade(seed: &str) -> String {
            let mut state = game(seed);
            sink_events(&mut state, &mut Vec::new(), |sink| {
                let card = on_field(sink.state, Some(grade_value(Grade::A)));
                in_hand(sink.state, &trick().id, P1, 5);
                in_hand(sink.state, &other_trick().id, P2, 4);
                for def_id in [
                    trick().id,
                    other_trick().id,
                    trick().id,
                    other_trick().id,
                    trick().id,
                ] {
                    play(sink, &def_id, false);
                }
                let effects = combo_index_end_of_turn(sink, &card);
                run(sink, &card, effects);
            });

            // Every random pick of the S cascade, in one string: the copies, the discounts, the exiles
            // and the cards that turned Radiant.
            serde_json::to_string(&json!({
                "hand": state
                    .players
                    .p1
                    .hand
                    .iter()
                    .map(|held| format!("{}:{}:{}", held.def_id, held.cost_mod, held.radiant))
                    .collect::<Vec<String>>(),
                "exiled": state.players.p2.exile.iter().map(|held| held.def_id.clone()).collect::<Vec<String>>(),
            }))
            .unwrap()
        }

        assert_eq!(cascade("combo-seed-a"), cascade("combo-seed-a"));
        assert_ne!(cascade("combo-seed-a"), cascade("combo-seed-b"));
    }

    #[test]
    fn r62_the_rise_is_an_ordinary_end_of_turn_trigger_so_ending_the_turn_runs_the_cascade() {
        let mut state = game("end-of-turn");
        let mut events: Vec<GameEvent> = Vec::new();
        let mut rng = Rng::new(&state.seed, state.rng_cursor);
        let mut sink = EngineSink::new(&mut state, &mut events, &mut rng);
        let card = on_field(sink.state, None);
        in_hand(sink.state, &trick().id, P1, 2);
        let played = play(&mut sink, &trick().id, false);

        end_turn(&mut sink);

        assert_eq!(grade_now(sink.state, &card.id), Some(2));
        assert_eq!(
            json_of(grade_name_of(super::card(sink.state, &card.id))),
            json!("D")
        );
        // E copied the played card and D discounted two hand cards, before the turn ended (§2.2).
        let added: Vec<Value> = events_of_type(sink.events, GameEventType::AddedToHand)
            .iter()
            .map(json_of)
            .filter(|event| event["player"] == json!("p1"))
            .collect();
        assert_eq!(added.len(), 1);
        assert_eq!(added[0]["defId"], json!(played.def_id));
        assert_eq!(
            sink.state
                .players
                .p1
                .hand
                .iter()
                .filter(|held| held.cost_mod == -1)
                .count(),
            2
        );

        let order: Vec<GameEventType> = sink.events.iter().map(|event| event.event_type()).collect();
        let index_of = |kind: GameEventType| order.iter().position(|each| *each == kind);
        assert!(index_of(GameEventType::CounterChanged) < index_of(GameEventType::TurnEnded));
        assert!(index_of(GameEventType::AddedToHand) < index_of(GameEventType::TurnEnded));

        // The end of the opponent's turn is not this card's end of turn (§6.2, R62), so the grade sits.
        assert_eq!(sink.state.active, P2);
        end_turn(&mut sink);
        assert_eq!(grade_now(sink.state, &card.id), Some(2));
        assert_eq!(sink.state.active, P1);
        assert_eq!(plays_this_turn(sink.state, P1), 0);
        let card = super::card(sink.state, &card.id).clone();
        assert!(!grade_rises(sink.state, &card));
    }
}

mod r86_what_a_pool_of_cards_played_this_turn_holds {
    use super::*;

    #[test]
    fn r86_skips_a_played_card_that_no_longer_exists_and_keeps_one_that_only_changed_zone() {
        let mut state = game("played-pool");
        let mut events: Vec<GameEvent> = Vec::new();
        let mut rng = Rng::new(&state.seed, state.rng_cursor);
        let mut sink = EngineSink::new(&mut state, &mut events, &mut rng);
        let card = on_field(sink.state, None);

        let spell = play(&mut sink, &trick().id, false); // a Spell: it resolves into the graveyard
        let mut unit = play(&mut sink, &body().id, false);
        assert_eq!(
            sink.state.players.p1.turn_log.played_ids,
            vec![spell.id.clone(), unit.id.clone()]
        );

        // Both are still findable, so both are in the pool: a card that moved zone is still a card.
        let pool = |state: &GameState| -> Vec<String> {
            played_cards_this_turn(state, P1)
                .iter()
                .map(|held| held.id.clone())
                .collect()
        };
        assert_eq!(pool(&*sink.state), vec![spell.id.clone(), unit.id.clone()]);

        // Make one cease to exist, as a unit token does when it leaves the field (R11): out of every
        // pile, and tagged `gone` rather than claiming a zone it is not in.
        remove_from_any_zone(sink.state, &mut unit);
        unit.zone = Zone::Gone { player: P1 };
        assert!(find_instance(sink.state, &unit.id).is_none());

        assert_eq!(pool(&*sink.state), vec![spell.id.clone()]);
        // And the E step picks from what is left rather than fizzling on the missing id.
        run(&mut sink, &card, vec![step_e()]);
        assert_eq!(
            sink.state
                .players
                .p1
                .hand
                .iter()
                .map(|held| held.def_id.clone())
                .collect::<Vec<String>>(),
            vec![trick().id]
        );
    }
}
