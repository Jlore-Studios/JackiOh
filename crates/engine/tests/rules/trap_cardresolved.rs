//! R17's second trap moment, step 7's `cardResolved` (SPEC §10.3, §10.5 step 7; R17, R61, R100, R119).
//!
//! The events are built by hand: the step-7 emission lands separately, and the trap side is tested
//! against the declared event. Fixtures are this file's own: `tr-` defs indexed from 2300 (BUILD §0).

use jackioh_engine::testkit::*;

use crate::rules::fixtures::harness::{
    Bench, apply_now, both, events_of_type, in_hand, live_mut, new_game, put, slot,
};

// Fixtures.

fn def(name: &str, index: u32, type_: &str, extra: Value) -> CardDef {
    let mut literal = json!({
        "id": format!("tr-{name}"),
        "index": index.to_string(),
        "name": format!("{name} (cardResolved)"),
        "set": "Core",
        "type": type_,
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": 0,
        "base": { "keywords": [], "text": name },
        "radiant": { "keywords": [], "text": name },
    });
    if let (Some(into), Some(from)) = (literal.as_object_mut(), extra.as_object()) {
        for (key, value) in from {
            into.insert(key.clone(), value.clone());
        }
    }
    json_as(literal)
}

/// A Trap that answers the moment a played card has resolved (R17).
fn after_trap() -> CardDef {
    def("after", 2301, "Trap", json!({}))
}
/// The same moment, but only for a card still in play — R61's `permanent` flag.
fn permanent_trap() -> CardDef {
    def("permanent", 2302, "Trap", json!({}))
}
/// A Trap on step 4's pair, for the other half of R119's arrival window.
fn arrival_trap() -> CardDef {
    def("arrival", 2303, "Trap", json!({}))
}
/// The played card the hand-built events name, so they are about something real.
fn body() -> CardDef {
    def(
        "body",
        2304,
        "Unit",
        json!({
            "base": { "attack": 2, "health": 3, "keywords": [], "text": "body" },
            "radiant": { "attack": 4, "health": 6, "keywords": [], "text": "body" },
        }),
    )
}

fn defs() -> Vec<CardDef> {
    vec![after_trap(), permanent_trap(), arrival_trap(), body()]
}

fn enemy_hero_damage(amount: i32) -> Effect {
    effects::damage(json_as(json!({ "to": { "of": "enemyHero" }, "amount": amount })))
}

fn scripts() -> Vec<(String, CardScripts)> {
    vec![
        (
            after_trap().id,
            both(Script {
                triggers: vec![TriggerDef::new(
                    "tr-after",
                    &[GameEventType::CardResolved],
                    |_ctx, _event| vec![enemy_hero_damage(2)],
                )],
                ..Script::default()
            }),
        ),
        (
            permanent_trap().id,
            both(Script {
                triggers: vec![
                    TriggerDef::new("tr-permanent", &[GameEventType::CardResolved], |_ctx, _event| {
                        vec![enemy_hero_damage(3)]
                    })
                    // R61: "only for played permanents", which is exactly what the event's flag reports.
                    .with_when(|_ctx, event| {
                        matches!(event, GameEvent::CardResolved { permanent: true, .. })
                    }),
                ],
                ..Script::default()
            }),
        ),
        (
            arrival_trap().id,
            both(Script {
                triggers: vec![TriggerDef::new(
                    "tr-arrival",
                    &[GameEventType::CardPlayed],
                    |_ctx, _event| vec![enemy_hero_damage(1)],
                )],
                ..Script::default()
            }),
        ),
    ]
}

// Harness.

fn game(seed: &str) -> GameState {
    let state = new_game(seed, None);
    let mut catalog = registered_catalog().clone();
    for def in defs() {
        catalog.insert(def.id.clone(), def);
    }
    register_catalog(catalog);
    let mut registry = registered_scripts().clone();
    for (id, script) in scripts() {
        registry.insert(id, script);
    }
    register_scripts(registry);
    let mut ready = begin_game(&state).state;
    let mut nonce_at = 0;
    for player in [PlayerId::P1, PlayerId::P2] {
        nonce_at += 1;
        let keep: Vec<String> = ready.players[player]
            .hand
            .iter()
            .map(|card| card.id.clone())
            .collect();
        let result = reduce(
            &ready,
            &json_as::<Action>(json!({
                "type": "mulligan",
                "keep": keep,
                "playerId": player,
                "nonce": format!("tr-mull-{seed}-{nonce_at}"),
            })),
        );
        if let Some(error) = result.error {
            panic!("{error}");
        }
        ready = result.state;
    }
    ready
}

/// §10.5 step 7's event. `costPaid` repeats what step 2 charged (0 for a cast, R70), carried on the
/// event rather than looked up (R89). Nothing here turns on the amount, so it defaults to 0.
fn resolved(instance: &CardInstance, permanent: bool) -> GameEvent {
    resolved_for(instance, permanent, 0)
}

fn resolved_for(instance: &CardInstance, permanent: bool, cost_paid: i32) -> GameEvent {
    GameEvent::CardResolved {
        player: instance.controller,
        instance_id: instance.id.clone(),
        def_id: instance.def_id.clone(),
        permanent,
        cost_paid,
        radiant: None,
        arrived_during: None,
        exits_from: None,
        answer: None,
        on_draw: None,
    }
}

/// The traps here are p2's, so `enemyHero` is p1's: what a trap firing is read off.
fn enemy_health(state: &GameState) -> i32 {
    state.players.p1.hero.health
}

fn trap_fired_ids(events: &[GameEvent]) -> Vec<String> {
    events_of_type(events, GameEventType::TrapFired)
        .iter()
        .filter_map(|event| match event {
            GameEvent::TrapFired { instance_id, .. } => Some(instance_id.clone()),
            _ => None,
        })
        .collect()
}

mod the_card_resolved_trap_moment_s10_5_step_7_r17_r61_r100_r119 {
    use super::*;

    #[test]
    fn r17_offers_a_trap_the_card_resolved_moment_and_resolves_it_there_on_the_immediate_path() {
        let mut state = game("r17-card-resolved");
        let trap = put(
            &mut state,
            &after_trap().id,
            slot(PlayerId::P2, Row::Backrow, 1),
            json!({}),
        );
        let played = put(
            &mut state,
            &body().id,
            slot(PlayerId::P1, Row::Units, 1),
            json!({}),
        );

        assert!(GAME_EVENT_TYPES.contains(&GameEventType::CardResolved));
        let watching: Vec<String> = traps_watching(&state, &resolved(&played, true))
            .iter()
            .map(|matched| matched.trap.id.clone())
            .collect();
        assert_eq!(watching, vec![trap.id.clone()]);

        let before = enemy_health(&state);
        let mut sink = Bench::new(&state);
        sink.events.push(resolved(&played, true));
        settle(&mut sink.sink(&mut state), SettleOptions::default());

        // §10.3: the trap fired immediately, to completion, and was consumed (§5.1).
        assert_eq!(trap_fired_ids(&sink.events), vec![trap.id.clone()]);
        assert_eq!(enemy_health(&state), before - 2);
        assert!(card_at(&state, slot(PlayerId::P2, Row::Backrow, 1)).is_none());
        assert!(state.players.p2.graveyard.iter().any(|card| card.id == trap.id));
    }

    #[test]
    fn r61_reads_the_events_permanent_flag_so_a_resolved_spell_does_not_count_as_a_played_permanent() {
        let mut state = game("r61-permanent-flag");
        let trap = put(
            &mut state,
            &permanent_trap().id,
            slot(PlayerId::P2, Row::Backrow, 1),
            json!({}),
        );
        let played = put(
            &mut state,
            &body().id,
            slot(PlayerId::P1, Row::Units, 1),
            json!({}),
        );
        let before = enemy_health(&state);

        // A card that has left play reports `permanent: false`: the trap declines and stays armed
        // (R99: a condition that must leave the trap armed belongs in `when`).
        let mut spent = Bench::new(&state);
        assert_eq!(
            fire_traps_for(&mut spent.sink(&mut state), &resolved(&played, false)).fired,
            Vec::<String>::new()
        );
        assert_eq!(enemy_health(&state), before);
        assert_eq!(
            card_at(&state, slot(PlayerId::P2, Row::Backrow, 1)).map(|c| c.id.clone()),
            Some(trap.id.clone())
        );
        assert_ne!(
            card_at(&state, slot(PlayerId::P2, Row::Backrow, 1)).and_then(|c| c.face_up),
            Some(true)
        );

        let mut kept = Bench::new(&state);
        assert_eq!(
            fire_traps_for(&mut kept.sink(&mut state), &resolved(&played, true)).fired,
            vec![trap.id.clone()]
        );
        assert_eq!(enemy_health(&state), before - 3);
    }

    #[test]
    fn r100_keeps_card_resolved_on_the_immediate_path_the_end_of_turn_window_never_delivers_it() {
        let mut state = game("r100-card-resolved-immediate");
        put(
            &mut state,
            &after_trap().id,
            slot(PlayerId::P2, Row::Backrow, 1),
            json!({}),
        );
        let played = put(
            &mut state,
            &body().id,
            slot(PlayerId::P1, Row::Units, 1),
            json!({}),
        );
        let event = resolved(&played, true);

        assert!(!TRAP_WINDOW_EVENTS.contains(&GameEventType::CardResolved));
        assert!(!is_trap_window_event(&event));

        let mut window = Bench::new(&state);
        let turn_ended = GameEvent::TurnEnded {
            player: PlayerId::P1,
            turn: state.turn,
            unspent_mana: 0,
        };
        assert_eq!(
            run_trap_window(&mut window.sink(&mut state), &turn_ended).fired,
            Vec::<String>::new()
        );
        assert!(state.work.is_empty());

        let mut immediate = Bench::new(&state);
        assert_eq!(
            fire_traps_for(&mut immediate.sink(&mut state), &event)
                .fired
                .len(),
            1
        );
    }

    #[test]
    fn r119_does_not_offer_a_trap_the_arrival_of_the_play_that_put_it_there_at_step_4_or_step_7() {
        let mut state = game("r119-own-arrival");
        let after = put(
            &mut state,
            &after_trap().id,
            slot(PlayerId::P2, Row::Backrow, 1),
            json!({}),
        );
        let arrival = put(
            &mut state,
            &arrival_trap().id,
            slot(PlayerId::P2, Row::Backrow, 2),
            json!({}),
        );
        let before = enemy_health(&state);

        // Step 7: a trap played this turn is on the field when its own `cardResolved` is dispatched,
        // so only R119 keeps it from answering its own arrival.
        let mut own = Bench::new(&state);
        assert!(traps_watching(&state, &resolved(&after, true)).is_empty());
        assert_eq!(
            fire_traps_for(&mut own.sink(&mut state), &resolved(&after, true)).fired,
            Vec::<String>::new()
        );

        // Step 4's half: a trap does not answer the `cardPlayed` that named it.
        let play = GameEvent::CardPlayed {
            player: PlayerId::P2,
            instance_id: arrival.id.clone(),
            def_id: arrival.def_id.clone(),
            cost_paid: 0,
            x: None,
            embiggened: None,
            former_id: None,
            from: None,
            arrived_during: None,
            exits_from: None,
            on_draw: None,
            modes: None,
        };
        assert!(traps_watching(&state, &play).is_empty());
        let mut answered = Bench::new(&state);
        assert_eq!(
            fire_traps_for(&mut answered.sink(&mut state), &play).fired,
            Vec::<String>::new()
        );

        assert_eq!(enemy_health(&state), before);
        assert_eq!(
            card_at(&state, slot(PlayerId::P2, Row::Backrow, 1)).map(|c| c.id.clone()),
            Some(after.id.clone())
        );
        assert_eq!(
            card_at(&state, slot(PlayerId::P2, Row::Backrow, 2)).map(|c| c.id.clone()),
            Some(arrival.id.clone())
        );
        assert_ne!(
            card_at(&state, slot(PlayerId::P2, Row::Backrow, 2)).and_then(|c| c.face_up),
            Some(true)
        );

        // The very next play is answered: R119 excludes this play, not the trap (§8 #33's wording).
        let played = put(
            &mut state,
            &body().id,
            slot(PlayerId::P1, Row::Units, 1),
            json!({}),
        );
        let mut next = Bench::new(&state);
        assert_eq!(
            fire_traps_for(&mut next.sink(&mut state), &resolved(&played, true)).fired,
            vec![after.id.clone()]
        );
        assert_eq!(enemy_health(&state), before - 2);
    }
}

/// R174, R212: step 4's `summoned`, as the later dispatches of a play meet it.
fn summoned(instance: &CardInstance) -> GameEvent {
    json_as(json!({
        "type": "summoned",
        "player": instance.controller,
        "instanceId": instance.id,
        "defId": instance.def_id,
        "row": "units",
        "lane": 1,
    }))
}

fn destroy(state: &mut GameState, instance: &CardInstance) {
    apply_now(
        state,
        effects::destroy(json_as(json!({
            "target": { "of": "instance", "instanceId": instance.id },
        }))),
    );
}

fn resolved_permanent(met: Option<GameEvent>) -> Option<bool> {
    match met {
        Some(GameEvent::CardResolved { permanent, .. }) => Some(permanent),
        _ => None,
    }
}

mod the_event_a_later_trap_meets_and_the_owed_dispatch_r61_r113_r174_r212 {
    use super::*;

    #[test]
    fn r174_r61_a_play_whose_card_left_the_field_since_the_dispatch_began_meets_no_later_trap() {
        let mut state = game("r174-standing-summoned");
        let played = put(
            &mut state,
            &body().id,
            slot(PlayerId::P1, Row::Units, 1),
            json!({}),
        );
        let event = summoned(&played);
        let mark = exit_mark(&state);
        let mut bench = Bench::new(&state);
        assert_eq!(
            standing_event(&bench.sink(&mut state), &event, mark),
            Some(event.clone())
        );

        // An earlier trap of the same dispatch destroys it: the next trap meets no play (#41 Sheepish
        // rewrites no bounced unit, R17).
        destroy(&mut state, &played);
        let mut after = Bench::new(&state);
        assert_eq!(standing_event(&after.sink(&mut state), &event, mark), None);
    }

    #[test]
    fn r61_r174_a_permanent_no_longer_on_the_field_resolves_as_no_longer_in_play() {
        let mut state = game("r61-standing-resolved");
        let played = put(
            &mut state,
            &body().id,
            slot(PlayerId::P1, Row::Units, 1),
            json!({}),
        );
        let event = resolved(&played, true);
        let began = exit_mark(&state);
        let mut standing = Bench::new(&state);
        assert_eq!(
            resolved_permanent(standing_event(&standing.sink(&mut state), &event, began)),
            Some(true)
        );

        // Destroyed since the dispatch began (#60's tokens kill it), and gone before it began: either
        // way the play is out of play for the trap that meets it (#85 fuses nothing out of a graveyard).
        destroy(&mut state, &played);
        let gone_before = exit_mark(&state);
        for mark in [began, gone_before] {
            let mut later = Bench::new(&state);
            assert_eq!(
                resolved_permanent(standing_event(&later.sink(&mut state), &event, mark)),
                Some(false),
                "mark {mark}"
            );
        }
    }

    #[test]
    fn r113_a_dispatch_a_prompt_stopped_fires_the_trap_it_still_owed_when_it_resumes() {
        let mut state = game("r113-owed-resume");
        let trap = put(
            &mut state,
            &after_trap().id,
            slot(PlayerId::P2, Row::Backrow, 1),
            json!({}),
        );
        let played = put(
            &mut state,
            &body().id,
            slot(PlayerId::P1, Row::Units, 1),
            json!({}),
        );
        let before = enemy_health(&state);
        let mark = exit_mark(&state);
        let mut bench = Bench::new(&state);
        let run = resume_event_to_traps(
            &mut bench.sink(&mut state),
            &resolved(&played, true),
            std::slice::from_ref(&trap.id),
            mark,
            &TrapControllers::new(),
        );
        assert_eq!(run.fired, vec![trap.id.clone()]);
        assert!(run.owed.is_empty());
        assert_eq!(enemy_health(&state), before - 2);
    }

    #[test]
    fn r61_r119_an_owed_trap_off_the_field_turned_face_up_or_meeting_its_own_arrival_does_not_fire() {
        let mut state = game("r61-owed-gone");
        let played = put(
            &mut state,
            &body().id,
            slot(PlayerId::P1, Row::Units, 1),
            json!({}),
        );
        let in_hand = in_hand(&mut state, &after_trap().id, PlayerId::P2, 1)
            .pop()
            .expect("a trap in hand");
        let face_up = put(
            &mut state,
            &after_trap().id,
            slot(PlayerId::P2, Row::Backrow, 1),
            json!({}),
        );
        live_mut(&mut state, &face_up.id).face_up = Some(true);
        let arriving = put(
            &mut state,
            &after_trap().id,
            slot(PlayerId::P2, Row::Backrow, 2),
            json!({}),
        );
        let before = enemy_health(&state);
        let mark = exit_mark(&state);

        // §5.1: a trap the answer returned to its owner's hand, or one an earlier trap turned face-up
        // (spent), is no trap set on the field.
        for owed in [&in_hand, &face_up] {
            let mut bench = Bench::new(&state);
            let run = resume_event_to_traps(
                &mut bench.sink(&mut state),
                &resolved(&played, true),
                std::slice::from_ref(&owed.id),
                mark,
                &TrapControllers::new(),
            );
            assert_eq!(run.fired, Vec::<String>::new(), "{}", owed.id);
        }
        // R119: nor does one answer the event of its own arrival.
        let mut own = Bench::new(&state);
        let run = resume_event_to_traps(
            &mut own.sink(&mut state),
            &resolved(&arriving, true),
            std::slice::from_ref(&arriving.id),
            mark,
            &TrapControllers::new(),
        );
        assert_eq!(run.fired, Vec::<String>::new());
        assert_eq!(enemy_health(&state), before);
    }

    #[test]
    fn r212_an_offered_trap_answers_for_the_player_who_held_it_when_the_event_happened() {
        let mut state = game("r212-offered-controller");
        let trap = put(
            &mut state,
            &after_trap().id,
            slot(PlayerId::P2, Row::Backrow, 1),
            json!({}),
        );
        let played = put(
            &mut state,
            &body().id,
            slot(PlayerId::P1, Row::Units, 1),
            json!({}),
        );
        let (p1_before, p2_before) = (state.players.p1.hero.health, state.players.p2.hero.health);

        // The events owed behind this one say the trap changed hands after it happened: it was p1's then.
        let later = |_: &EngineSink<'_>| LaterMoves {
            moved: IndexSet::new(),
            controller_before: [(trap.id.clone(), PlayerId::P1)].into_iter().collect(),
        };
        let mut bench = Bench::new(&state);
        let run = offer_event_to_traps(&mut bench.sink(&mut state), &resolved(&played, true), &later);
        assert_eq!(run.fired, vec![trap.id.clone()]);
        assert_eq!(
            run.controllers,
            [(trap.id.clone(), PlayerId::P1)]
                .into_iter()
                .collect::<TrapControllers>()
        );
        // As p1's, its enemy hero is p2's.
        assert_eq!(state.players.p2.hero.health, p2_before - 2);
        assert_eq!(state.players.p1.hero.health, p1_before);
    }
}
