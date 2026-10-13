//! Crater (#783, R783010, R783011): a Unit Locks its own zone as it arrives on the field, every way it
//! arrives, before its Cry; a Trap Locks its backrow zone as it is revealed. The Lock is R688's: it
//! refuses plays, a Stack play included, and outlasts the card. The fixtures in
//! `fixtures/keywords_783.rs` stand in for the cards #781 prints.

use jackioh_engine::testkit::*;
use jackioh_engine::zones::is_locked;

use super::fixtures::harness::{events_of_type, put, set_library, slot};
use super::fixtures::keywords_783::{
    crater_cloaked, crater_field, crater_moon, crater_reborn, crater_spell, crater_trap, crater_unit,
    flicker_all, plain_unit, recruiter, register_keywords_783, stacker, summon_crater,
};
use super::fixtures::play_pipeline_b::{pb_reduce, plays_of};
use super::fixtures::prompt_harness::{act, board, cast_now, must};

const P1: PlayerId = PlayerId::P1;
const P2: PlayerId = PlayerId::P2;

/// p1's main phase on turn 3 with 4 mana a side and #783's cards registered.
fn playing(seed: &str) -> GameState {
    let state = board(seed);
    register_keywords_783();
    state
}

fn in_hand(state: &mut GameState, def_id: &str, player: PlayerId) -> CardInstance {
    let card = new_instance(state, def_id, player, Zone::Hand { player });
    state.players[player].hand.push(card.clone());
    card
}

fn unit_zone(lane: i32) -> ZoneSlot {
    slot(P1, Row::Units, lane)
}

/// Play this card from p1's hand into this zone, answering the whole result.
fn play_into(state: &GameState, id: &str, row: &str, lane: i32) -> ReduceResult {
    pb_reduce(
        state,
        json!({
            "type": "play",
            "instanceId": id,
            "zone": { "row": row, "lane": lane },
            "playerId": "p1",
        }),
    )
}

/// Apply one effect for p1 and settle, as a resolving card's would.
fn apply_now(state: &mut GameState, effect: Effect) {
    let mut events: Vec<GameEvent> = Vec::new();
    let mut rng = Rng::new(&state.seed.clone(), state.rng_cursor);
    {
        let mut sink = EngineSink::new(state, &mut events, &mut rng);
        {
            let mut ctx = make_context(
                &mut sink,
                None,
                HookOptions {
                    controller: Some(P1),
                    ..Default::default()
                },
            );
            (effect.apply)(&mut ctx);
        }
        settle(&mut sink, SettleOptions::default());
    }
    state.rng_cursor = rng.cursor();
}

fn destroy(state: &mut GameState, id: &str) {
    apply_now(
        state,
        jackioh_engine::effects::destroy(json_as(
            json!({ "target": { "of": "instance", "instanceId": id } }),
        )),
    );
}

/// The id of the top card in p1's unit zone `lane`, if any.
fn unit_at(state: &GameState, lane: i32) -> Option<String> {
    state.players.p1.units[(lane - 1) as usize]
        .as_ref()
        .and_then(|pile| pile.first())
        .map(|card| card.id.clone())
}

fn locked_count(state: &GameState) -> usize {
    [P1, P2]
        .into_iter()
        .flat_map(|player| {
            [Row::Units, Row::Backrow]
                .into_iter()
                .flat_map(move |row| (1..=5).map(move |lane| slot(player, row, lane)))
        })
        .filter(|at| is_locked(state, *at))
        .count()
}

mod r783010_crater_arrivals {
    use super::*;

    #[test]
    fn r783010_played_it_locks_its_zone_before_its_cry() {
        let mut state = playing("r783010-played");
        let card = in_hand(&mut state, &crater_unit().id, P1);
        let result = play_into(&state, &card.id, "units", 2);
        assert_eq!(result.error, None);
        let state = result.state;
        assert!(is_locked(&state, unit_zone(2)));
        assert_eq!(locked_count(&state), 1);
        // The zone is Locked before the Cry's damage lands.
        let locked_at = must(
            result.events.iter().position(|event| {
                matches!(
                    event,
                    GameEvent::Locked {
                        row: Row::Units,
                        lane: 2,
                        ..
                    }
                )
            }),
            "a locked event",
        );
        let cry_at = must(
            result
                .events
                .iter()
                .position(|event| matches!(event, GameEvent::Damage { .. })),
            "the Cry's damage",
        );
        assert!(locked_at < cry_at);
        // A card that is not Crater locks nothing.
        let mut plain = playing("r783010-plain");
        let other = in_hand(&mut plain, &plain_unit().id, P1);
        let plain = act(
            &plain,
            json!({ "type": "play", "instanceId": other.id, "zone": { "row": "units", "lane": 2 }, "playerId": "p1" }),
            None,
        );
        assert_eq!(locked_count(&plain), 0);
    }

    #[test]
    fn r783010_summon_recruit_reborn_and_flicker_lock() {
        // Summoned by an effect: into R64's leftmost zone, Locked.
        let mut summoned = playing("r783010-summon");
        cast_now(&mut summoned, &summon_crater().id, P1, false);
        assert!(unit_at(&summoned, 1).is_some());
        assert!(is_locked(&summoned, unit_zone(1)));

        // Recruited from the library.
        let mut recruited = playing("r783010-recruit");
        set_library(&mut recruited, P1, &[crater_unit().id]);
        cast_now(&mut recruited, &recruiter().id, P1, false);
        assert!(unit_at(&recruited, 1).is_some());
        assert!(is_locked(&recruited, unit_zone(1)));

        // Back through Reborn into the zone it died in.
        let mut reborn = playing("r783010-reborn");
        let body = put(&mut reborn, &crater_reborn().id, unit_zone(3), json!({}));
        assert!(!is_locked(&reborn, unit_zone(3)));
        destroy(&mut reborn, &body.id);
        assert!(unit_at(&reborn, 3).is_some(), "Reborn brought it back");
        assert!(is_locked(&reborn, unit_zone(3)));

        // Flickered: it re-enters its zone.
        let mut flickered = playing("r783010-flicker");
        put(&mut flickered, &crater_unit().id, unit_zone(4), json!({}));
        assert!(!is_locked(&flickered, unit_zone(4)));
        cast_now(&mut flickered, &flicker_all().id, P1, false);
        assert!(unit_at(&flickered, 4).is_some());
        assert!(is_locked(&flickered, unit_zone(4)));
    }

    #[test]
    fn r783010_a_cloaked_crater_unit_locks_the_unit_zone_it_reveals_into() {
        let mut state = playing("r783010-cloaked");
        let card = in_hand(&mut state, &crater_cloaked().id, P1);
        // A card still to play, so the turn does not end by itself.
        in_hand(&mut state, &plain_unit().id, P1);
        state = act(
            &state,
            json!({
                "type": "play",
                "instanceId": card.id,
                "zone": { "row": "backrow", "lane": 2 },
                "faceDown": "cloaked",
                "playerId": "p1",
            }),
            None,
        );
        // Set face-down, it is no Unit and has no Crater to use.
        assert_eq!(locked_count(&state), 0);
        let set = must(state.players.p1.backrow[1].clone(), "the set card");
        state = act(
            &state,
            json!({ "type": "activate", "instanceId": set.id, "ability": "reveal", "playerId": "p1" }),
            None,
        );
        assert_eq!(unit_at(&state, 2), Some(set.id.clone()));
        assert!(is_locked(&state, unit_zone(2)));
        assert!(!is_locked(&state, slot(P1, Row::Backrow, 2)));
        assert_eq!(locked_count(&state), 1);
    }

    #[test]
    fn r783010_a_stack_play_onto_it_is_refused_and_the_lock_outlasts_it() {
        let mut state = playing("r783010-stack");
        let card = in_hand(&mut state, &crater_unit().id, P1);
        let stack = in_hand(&mut state, &stacker().id, P1);
        state = act(
            &state,
            json!({ "type": "play", "instanceId": card.id, "zone": { "row": "units", "lane": 1 }, "playerId": "p1" }),
            None,
        );
        let placed = must(unit_at(&state, 1), "the Crater Unit in lane 1");
        // Offered into the other zones, never onto the Crater Unit's.
        let offered: Vec<Value> = plays_of(&state, &stack.id, P1)
            .iter()
            .map(|play| serde_json::to_value(play).expect("a play serialises"))
            .collect();
        assert!(!offered.is_empty());
        assert!(
            offered
                .iter()
                .all(|play| !(play["zone"]["row"] == json!("units") && play["zone"]["lane"] == json!(1)))
        );
        assert!(play_into(&state, &stack.id, "units", 1).error.is_some());

        // The Crater Unit leaves; the zone stays Locked, and a play into it is still refused.
        destroy(&mut state, &placed);
        assert!(unit_at(&state, 1).is_none());
        assert!(is_locked(&state, unit_zone(1)));
        let other = in_hand(&mut state, &plain_unit().id, P1);
        assert!(play_into(&state, &other.id, "units", 1).error.is_some());
        assert_eq!(play_into(&state, &other.id, "units", 2).error, None);
    }
}

mod r783011_crater_traps {
    use super::*;

    #[test]
    fn r783011_a_crater_trap_locks_as_it_fires_a_spell_locks_nothing() {
        // The enemy's Crater Trap fires on p1's play and Locks its own backrow zone.
        let mut state = playing("r783011-trap");
        put(
            &mut state,
            &crater_trap().id,
            slot(P2, Row::Backrow, 3),
            json!({}),
        );
        let card = in_hand(&mut state, &plain_unit().id, P1);
        let before = state.players.p1.hero.health;
        let result = play_into(&state, &card.id, "units", 1);
        assert_eq!(result.error, None);
        assert_eq!(events_of_type(&result.events, GameEventType::TrapFired).len(), 1);
        assert_eq!(result.state.players.p1.hero.health, before - 1);
        assert!(is_locked(&result.state, slot(P2, Row::Backrow, 3)));
        assert_eq!(locked_count(&result.state), 1);

        // One that fires as it replaces an event (§5.1, R33) Locks its zone the same way.
        let mut moon = playing("r783011-replacement");
        put(&mut moon, &crater_moon().id, slot(P2, Row::Backrow, 4), json!({}));
        moon.players.p1.hero.health -= 5;
        let hurt = moon.players.p1.hero.health;
        let mut events: Vec<GameEvent> = Vec::new();
        let mut rng = Rng::new(&moon.seed.clone(), moon.rng_cursor);
        {
            let mut sink = EngineSink::new(&mut moon, &mut events, &mut rng);
            {
                let mut ctx = make_context(
                    &mut sink,
                    None,
                    HookOptions {
                        controller: Some(P1),
                        ..Default::default()
                    },
                );
                (jackioh_engine::effects::heal(json_as(
                    json!({ "target": { "of": "selfHero" }, "amount": 2 }),
                ))
                .apply)(&mut ctx);
            }
            settle(&mut sink, SettleOptions::default());
        }
        assert_eq!(events_of_type(&events, GameEventType::TrapFired).len(), 1);
        assert_eq!(moon.players.p1.hero.health, hurt - 2);
        assert!(is_locked(&moon, slot(P2, Row::Backrow, 4)));
        assert_eq!(locked_count(&moon), 1);

        // Crater on a Spell or a Field Spell does nothing.
        let mut spells = playing("r783011-spell");
        let spell = in_hand(&mut spells, &crater_spell().id, P1);
        let field = in_hand(&mut spells, &crater_field().id, P1);
        spells = act(
            &spells,
            json!({ "type": "play", "instanceId": spell.id, "playerId": "p1" }),
            None,
        );
        let spells = act(
            &spells,
            json!({ "type": "play", "instanceId": field.id, "zone": { "row": "backrow", "lane": 2 }, "playerId": "p1" }),
            None,
        );
        assert!(spells.players.p1.backrow[1].is_some());
        assert_eq!(locked_count(&spells), 0);
    }
}
