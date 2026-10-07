//! Steal (SPEC §6.3, BUILD M3-T1): R15's placement, R12's ownership, R33's face-down trap and
//! #86's "steal all enemy units". The fixture Trap this file needs is registered here, so no shared
//! fixture has to grow for it (BUILD §0).
//!
//! Port of `packages/engine/test/effects-steal.test.ts`.

use jackioh_engine::testkit::*;

use jackioh_engine::catalog::registered_catalog;
use jackioh_engine::effects::{steal, steal_all};
use jackioh_engine::layers::unit_view;
use jackioh_engine::resolve::{HookOptions, make_context};
use jackioh_engine::rng::Rng;
use jackioh_engine::script::{Effect, EngineSink};
use jackioh_engine::state::{
    CardInstance, Exertion, GameState, find_instance, find_instance_mut, new_instance,
};
use jackioh_engine::zones::{
    OffFieldZone, PlaceOnFieldOptions, active_units_of, card_at, lock_zone, move_to_zone, place_on_field,
};

use super::fixtures::catalog::spell_def;
use super::fixtures::combat::{plain, stacker};
use super::fixtures::harness::{in_hand, new_game, put, slot};

/// A face-down Trap for R33; #36 radiant steals a backrow card.
fn trap() -> CardDef {
    spell_def(
        710,
        json!({ "id": "st-trap", "index": "710", "name": "Steal Trap (fixture)", "type": "Trap" }),
    )
}

fn game() -> GameState {
    let state = new_game("steal-test", None);
    let mut catalog = registered_catalog().clone();
    let def = trap();
    catalog.insert(def.id.clone(), def);
    register_catalog(catalog);
    state
}

/// Apply one effect the way `resolve.ts` does, and hand back the events it emitted.
fn run(state: &mut GameState, effect: Effect, options: HookOptions) -> Vec<GameEvent> {
    let mut rng = Rng::new(&state.seed, state.rng_cursor);
    let mut events = Vec::new();
    {
        let mut sink = EngineSink::new(state, &mut events, &mut rng);
        let mut ctx = make_context(&mut sink, None, options);
        (effect.apply)(&mut ctx);
    }
    state.rng_cursor = rng.cursor();
    events
}

fn as_p1() -> HookOptions {
    HookOptions {
        controller: Some(PlayerId::P1),
        ..Default::default()
    }
}

fn controls(state: &mut GameState, card: &CardInstance) -> Vec<GameEvent> {
    run(state, steal(json_as(json!({ "instanceId": card.id }))), as_p1())
}

/// `fixtures/harness.ts`'s `eventsOfType`, as JSON: the events of one type, each as TS writes it.
fn of_type(events: &[GameEvent], kind: &str) -> Vec<Value> {
    events
        .iter()
        .map(|event| serde_json::to_value(event).expect("an event serialises"))
        .filter(|event| event["type"] == kind)
        .collect()
}

/// The card under `id` as it stands in the state now (TS read the live object).
fn live<'a>(state: &'a GameState, id: &str) -> &'a CardInstance {
    find_instance(state, id).expect("the card is in the state")
}

/// `cardAt(state, slot(player, row, lane))?.id`.
fn id_at(state: &GameState, player: PlayerId, row: Row, lane: i32) -> Option<String> {
    card_at(state, slot(player, row, lane)).map(|card| card.id.clone())
}

mod r15_steal_s6_3_m3_t1 {
    use super::*;

    #[test]
    fn r15_takes_the_same_lane_when_it_is_free_and_moves_control_only() {
        let mut state = game();
        let victim = put(
            &mut state,
            &plain.id,
            slot(PlayerId::P2, Row::Units, 3),
            json!({}),
        );
        {
            let card = find_instance_mut(&mut state, &victim.id).expect("the victim");
            card.damage = 1;
            card.buffs = AttackHealth { attack: 2, health: 0 };
            card.position = Some(Position::Def);
            card.counters = Counters {
                plague: Some(2),
                ..Default::default()
            };
            card.exertion = Exertion {
                attacked: true,
                switched: false,
                attacks: None,
            };
        }

        let events = controls(&mut state, &victim);

        assert_eq!(
            id_at(&state, PlayerId::P1, Row::Units, 3),
            Some(victim.id.clone())
        );
        assert_eq!(id_at(&state, PlayerId::P2, Row::Units, 3), None);
        let now = live(&state, &victim.id);
        assert_eq!(now.controller, PlayerId::P1);
        // R12: ownership never moves.
        assert_eq!(now.owner, PlayerId::P2);
        assert_eq!(
            now.zone,
            Zone::Field {
                player: PlayerId::P1,
                row: Row::Units,
                lane: 3
            }
        );

        // The card never left the field, so R78's reset does not apply: damage, buffs, position and
        // counters stay.
        assert_eq!(now.damage, 1);
        assert_eq!(now.buffs, AttackHealth { attack: 2, health: 0 });
        assert_eq!(now.position, Some(Position::Def));
        assert_eq!(
            now.counters,
            Counters {
                plague: Some(2),
                ..Default::default()
            }
        );
        assert_eq!(unit_view(&state, now).attack, 5);
        // R171: but it has entered p1's side on this turn, so it is summoning sick with a fresh exertion.
        assert_eq!(now.summoned_turn, Some(state.turn));
        assert_eq!(
            now.exertion,
            Exertion {
                attacked: false,
                switched: false,
                attacks: None
            }
        );

        assert_eq!(
            of_type(&events, "controlChanged"),
            vec![
                json!({ "type": "controlChanged", "instanceId": victim.id, "controller": "p1", "row": "units", "lane": 3 })
            ]
        );
    }

    #[test]
    fn r15_falls_back_to_the_first_free_zone_when_the_same_lane_is_taken_or_locked() {
        let mut state = game();
        put(
            &mut state,
            &plain.id,
            slot(PlayerId::P1, Row::Units, 1),
            json!({}),
        );
        put(
            &mut state,
            &plain.id,
            slot(PlayerId::P1, Row::Units, 3),
            json!({}),
        );
        let victim = put(
            &mut state,
            &plain.id,
            slot(PlayerId::P2, Row::Units, 3),
            json!({}),
        );

        let events = controls(&mut state, &victim);

        assert_eq!(
            id_at(&state, PlayerId::P1, Row::Units, 2),
            Some(victim.id.clone())
        );
        assert_eq!(of_type(&events, "controlChanged")[0]["lane"], json!(2));

        // An empty but Locked same lane is not free either, so the fallback applies again (§3.2).
        let mut locked = game();
        let other = put(
            &mut locked,
            &plain.id,
            slot(PlayerId::P2, Row::Units, 4),
            json!({}),
        );
        lock_zone(&mut locked, slot(PlayerId::P1, Row::Units, 4));

        assert_eq!(
            of_type(&controls(&mut locked, &other), "controlChanged")[0]["lane"],
            json!(1)
        );
        assert_eq!(
            id_at(&locked, PlayerId::P1, Row::Units, 1),
            Some(other.id.clone())
        );
    }

    #[test]
    fn r15_leaves_a_card_with_nowhere_to_go_with_its_owner() {
        let mut state = game();
        for lane in 1..=5 {
            put(
                &mut state,
                &plain.id,
                slot(PlayerId::P1, Row::Units, lane),
                json!({}),
            );
        }
        let victim = put(
            &mut state,
            &plain.id,
            slot(PlayerId::P2, Row::Units, 2),
            json!({}),
        );

        let events = controls(&mut state, &victim);

        assert_eq!(
            id_at(&state, PlayerId::P2, Row::Units, 2),
            Some(victim.id.clone())
        );
        assert_eq!(live(&state, &victim.id).controller, PlayerId::P2);
        assert_eq!(events.len(), 0);
    }

    #[test]
    fn r12_keeps_the_owner_so_a_stolen_unit_that_dies_goes_to_its_owners_graveyard() {
        let mut state = game();
        let victim = put(
            &mut state,
            &plain.id,
            slot(PlayerId::P2, Row::Units, 1),
            json!({}),
        );
        controls(&mut state, &victim);
        assert_eq!(live(&state, &victim.id).controller, PlayerId::P1);

        let mut moving = live(&state, &victim.id).clone();
        move_to_zone(
            &mut state,
            &mut moving,
            OffFieldZone::Graveyard,
            Default::default(),
        );

        assert_eq!(
            state
                .players
                .p2
                .graveyard
                .iter()
                .map(|card| card.id.clone())
                .collect::<Vec<_>>(),
            vec![victim.id.clone()]
        );
        assert_eq!(state.players.p1.graveyard.len(), 0);
        // R78: leaving the field hands control back to the owner.
        assert_eq!(live(&state, &victim.id).controller, PlayerId::P2);
    }

    #[test]
    fn r33_leaves_a_stolen_face_down_trap_face_down_under_its_new_controller() {
        let mut state = game();
        let hidden = put(
            &mut state,
            &trap().id,
            slot(PlayerId::P2, Row::Backrow, 4),
            json!({}),
        );
        assert_eq!(live(&state, &hidden.id).face_up, None);

        let events = controls(&mut state, &hidden);

        assert_eq!(
            id_at(&state, PlayerId::P1, Row::Backrow, 4),
            Some(hidden.id.clone())
        );
        let now = live(&state, &hidden.id);
        assert_eq!(now.controller, PlayerId::P1);
        assert_eq!(now.owner, PlayerId::P2);
        // The steal never flips the card: `controller` is what decides who may read it (R33).
        assert_eq!(now.face_up, None);
        assert_eq!(
            of_type(&events, "controlChanged"),
            vec![
                json!({ "type": "controlChanged", "instanceId": hidden.id, "controller": "p1", "row": "backrow", "lane": 4 })
            ]
        );
    }

    #[test]
    fn r13_steals_the_top_of_a_stack_pile_and_leaves_the_card_beneath_to_resume() {
        let mut state = game();
        let beneath = put(
            &mut state,
            &plain.id,
            slot(PlayerId::P2, Row::Units, 2),
            json!({}),
        );
        let mut top = new_instance(
            &mut state,
            &stacker.id,
            PlayerId::P2,
            Zone::Hand { player: PlayerId::P2 },
        );
        assert!(place_on_field(
            &mut state,
            &mut top,
            slot(PlayerId::P2, Row::Units, 2),
            PlaceOnFieldOptions { stack: Some(true) }
        ));

        controls(&mut state, &top);

        assert_eq!(id_at(&state, PlayerId::P1, Row::Units, 2), Some(top.id.clone()));
        assert_eq!(
            id_at(&state, PlayerId::P2, Row::Units, 2),
            Some(beneath.id.clone())
        );
        assert_eq!(
            active_units_of(&state, PlayerId::P2)
                .iter()
                .map(|card| card.id.clone())
                .collect::<Vec<_>>(),
            vec![beneath.id.clone()]
        );
        assert_eq!(state.players.p1.units[1].as_ref().map(Vec::len), Some(1));
    }

    #[test]
    fn r15_c86_steals_every_enemy_unit_in_lane_order_and_leaves_the_excess_with_its_owner() {
        let mut state = game();
        put(
            &mut state,
            &plain.id,
            slot(PlayerId::P1, Row::Units, 1),
            json!({}),
        );
        put(
            &mut state,
            &plain.id,
            slot(PlayerId::P1, Row::Units, 2),
            json!({}),
        );
        let enemies: Vec<CardInstance> = (1..=5)
            .map(|lane| {
                put(
                    &mut state,
                    &plain.id,
                    slot(PlayerId::P2, Row::Units, lane),
                    json!({}),
                )
            })
            .collect();

        let events = run(&mut state, steal_all(Default::default()), as_p1());

        // Lane 1 and 2 are taken, so the first two land in lanes 3 and 4; the third takes lane 5 and
        // the last two find nothing free.
        assert_eq!(
            id_at(&state, PlayerId::P1, Row::Units, 3),
            Some(enemies[0].id.clone())
        );
        assert_eq!(
            id_at(&state, PlayerId::P1, Row::Units, 4),
            Some(enemies[1].id.clone())
        );
        assert_eq!(
            id_at(&state, PlayerId::P1, Row::Units, 5),
            Some(enemies[2].id.clone())
        );
        assert_eq!(
            id_at(&state, PlayerId::P2, Row::Units, 4),
            Some(enemies[3].id.clone())
        );
        assert_eq!(
            id_at(&state, PlayerId::P2, Row::Units, 5),
            Some(enemies[4].id.clone())
        );
        assert_eq!(
            enemies
                .iter()
                .map(|card| live(&state, &card.id).controller)
                .collect::<Vec<_>>(),
            vec![
                PlayerId::P1,
                PlayerId::P1,
                PlayerId::P1,
                PlayerId::P2,
                PlayerId::P2
            ]
        );
        assert_eq!(
            of_type(&events, "controlChanged")
                .iter()
                .map(|event| json!([event["instanceId"], event["lane"]]))
                .collect::<Vec<_>>(),
            vec![
                json!([enemies[0].id, 3]),
                json!([enemies[1].id, 4]),
                json!([enemies[2].id, 5]),
            ]
        );
    }

    #[test]
    fn steals_nothing_off_the_field_nothing_already_yours_and_nothing_when_no_target_was_picked() {
        let mut state = game();
        let in_your_hand = in_hand(&mut state, &plain.id, PlayerId::P2, 1)
            .into_iter()
            .next()
            .expect("a hand card");
        let mine = put(
            &mut state,
            &plain.id,
            slot(PlayerId::P1, Row::Units, 1),
            json!({}),
        );
        let theirs = put(
            &mut state,
            &plain.id,
            slot(PlayerId::P2, Row::Units, 2),
            json!({}),
        );

        // Off the field control means nothing, so a hand card is untouched (R12).
        assert_eq!(controls(&mut state, &in_your_hand).len(), 0);
        assert_eq!(live(&state, &in_your_hand.id).controller, PlayerId::P2);
        assert_eq!(
            state
                .players
                .p2
                .hand
                .iter()
                .map(|card| card.id.clone())
                .collect::<Vec<_>>(),
            vec![in_your_hand.id.clone()]
        );

        // R76: a card already under your control is not stolen again, and nothing moves lane.
        assert_eq!(controls(&mut state, &mine).len(), 0);
        assert_eq!(id_at(&state, PlayerId::P1, Row::Units, 1), Some(mine.id.clone()));

        // An empty `targets` list fizzles rather than picking something (§8 conventions).
        assert_eq!(run(&mut state, steal(Default::default()), as_p1()).len(), 0);
        assert_eq!(live(&state, &theirs.id).controller, PlayerId::P2);
    }
}
