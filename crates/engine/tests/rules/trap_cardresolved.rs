//! The second of R17's two trap moments: `cardResolved` (SPEC §10.3's event list, §10.5 step 7;
//! R17, R61, R100, R119).
//!
//! R17 splits trap timing in two. Sheepish answers step 4's `summoned`/`cardPlayed` pair, before the
//! Cry, and costs the card its Cry. Bear Honeypot, Unstable Clone Machine and Unlicensed
//! Experimentation fire "after the card resolves", which is step 7's `cardResolved` — emitted once
//! per play or cast, after the Cry and after step 6 has drained every Echo repeat, carrying
//! `permanent: true` when the card is still in play (R61's "played permanents only").
//!
//! What this file pins about the trap side of that event:
//!
//!   * R17 — a trap whose trigger names `cardResolved` is offered it, and the trap resolves to
//!     completion there, like any other immediate response (§10.3).
//!   * R100 — `cardResolved` travels the immediate path and not the scheduled one: it is not a
//!     `TRAP_WINDOW_EVENTS` member, so the end-of-turn window never delivers it and the two paths
//!     stay disjoint.
//!   * R119 — a trap is itself a card someone played, so it does not answer the arrival event that
//!     names it: not `cardPlayed` or `summoned` at step 4, and not its own `cardResolved` at step 7.
//!     It stays armed and face-down for the next play instead.
//!
//! The events here are built by hand rather than played out, because the step-7 emission is
//! `playSteps.ts`'s and lands separately (`echo.landAfterResolution` is written and not yet called).
//! That is the point of the seam: the trap side is complete against the declared event.
//!
//! Fixtures are this file's own: defs are prefixed `tr-` and indexed from 2300 (BUILD §0).
//!
//! Port of `packages/engine/test/trap-cardresolved.test.ts`.

use jackioh_engine::testkit::*;

use crate::rules::fixtures::harness::{events_of_type, new_game, put, slot};

// ---------------------------------------------------------------------------
// The sink: TS `sinkFor(state)`, a sink whose rng starts at the state's cursor, as reduce does.
// Rust's `EngineSink` borrows the state, so the event list and the rng live here and each call
// borrows the state again.
// ---------------------------------------------------------------------------

struct SinkFor {
    events: Vec<GameEvent>,
    rng: Rng,
}

fn sink_for(state: &GameState) -> SinkFor {
    SinkFor {
        events: Vec::new(),
        rng: Rng::new(&state.seed, state.rng_cursor),
    }
}

impl SinkFor {
    fn on<'a>(&'a mut self, state: &'a mut GameState) -> EngineSink<'a> {
        EngineSink::new(state, &mut self.events, &mut self.rng)
    }
}

// ---------------------------------------------------------------------------
// Fixtures. TS numbered them from a module counter starting at 2300; each def's index is written
// out here in the order TS created them.
// ---------------------------------------------------------------------------

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

fn both(script: Script) -> CardScripts {
    CardScripts {
        base: script.clone(),
        radiant: script,
    }
}

/// #60's shape: a Trap that answers the moment a played card has resolved (R17).
fn after_trap() -> CardDef {
    def("after", 2301, "Trap", json!({}))
}
/// #85's shape: the same moment, but only for a card still in play — R61's `permanent` flag.
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

// ---------------------------------------------------------------------------
// Harness.
// ---------------------------------------------------------------------------

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

/// §10.5 step 7's event, as `echo.landAfterResolution` builds it. `costPaid` repeats what step 2
/// charged (0 for a cast, R70) and is carried on the event rather than looked up, per R89 — #60
/// reads it for "costing 1 or less". Nothing here turns on the amount, so it defaults to 0.
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

// ---------------------------------------------------------------------------

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

        // The event type is declared, so nothing here is reaching ahead of the engine (§10.3's list).
        assert!(GAME_EVENT_TYPES.contains(&GameEventType::CardResolved));
        // Matching needs no list of its own: a trigger that names the type is woken by it.
        let watching: Vec<String> = traps_watching(&state, &resolved(&played, true))
            .iter()
            .map(|matched| matched.trap.id.clone())
            .collect();
        assert_eq!(watching, vec![trap.id.clone()]);

        let before = enemy_health(&state);
        let mut sink = sink_for(&state);
        sink.events.push(resolved(&played, true));
        settle(&mut sink.on(&mut state), SettleOptions::default());

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

        // A card that has left play by now reports `permanent: false`: the trap declines and stays
        // armed, which is R99's "a condition that must leave the trap armed belongs in `when`".
        let mut spent = sink_for(&state);
        assert_eq!(
            fire_traps_for(&mut spent.on(&mut state), &resolved(&played, false)).fired,
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

        // Still in play, so the trap fires.
        let mut kept = sink_for(&state);
        assert_eq!(
            fire_traps_for(&mut kept.on(&mut state), &resolved(&played, true)).fired,
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

        // The two paths are disjoint, and only the window's events are withheld from the immediate one.
        assert!(!TRAP_WINDOW_EVENTS.contains(&GameEventType::CardResolved));
        assert!(!is_trap_window_event(&event));

        // The window's own event still fires nothing here: this trap does not watch a turn end.
        let mut window = sink_for(&state);
        let turn_ended = GameEvent::TurnEnded {
            player: PlayerId::P1,
            turn: state.turn,
            unspent_mana: 0,
        };
        assert_eq!(
            run_trap_window(&mut window.on(&mut state), &turn_ended).fired,
            Vec::<String>::new()
        );
        assert!(state.work.is_empty());

        // And the immediate check delivers `cardResolved`, which is the only path that does.
        let mut immediate = sink_for(&state);
        assert_eq!(
            fire_traps_for(&mut immediate.on(&mut state), &event).fired.len(),
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

        // Step 7, the subtle case: a trap played this turn is on the field when its own `cardResolved`
        // is dispatched, so only R119 keeps it from answering its own arrival.
        let mut own = sink_for(&state);
        assert!(traps_watching(&state, &resolved(&after, true)).is_empty());
        assert_eq!(
            fire_traps_for(&mut own.on(&mut state), &resolved(&after, true)).fired,
            Vec::<String>::new()
        );

        // Step 4's half of the same rule: a trap does not answer the `cardPlayed` that named it.
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
        };
        assert!(traps_watching(&state, &play).is_empty());
        let mut answered = sink_for(&state);
        assert_eq!(
            fire_traps_for(&mut answered.on(&mut state), &play).fired,
            Vec::<String>::new()
        );

        // Nothing fired, nothing was spent, and both traps are still armed and face-down.
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
        let mut next = sink_for(&state);
        assert_eq!(
            fire_traps_for(&mut next.on(&mut state), &resolved(&played, true)).fired,
            vec![after.id.clone()]
        );
        assert_eq!(enemy_health(&state), before - 2);
    }
}
