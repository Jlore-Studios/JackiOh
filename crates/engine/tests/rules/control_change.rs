//! Port of `packages/engine/test/control-change.test.ts`.
//!
//! R171 and R172 at the verb level (SPEC §4.1, §11; docs/polish/4-edge-cases.md behaviours 7, 8 and
//! 10 to 14). A change of control on the field is an entry: the card takes the current turn as its
//! `summonedTurn` and a fresh exertion, on every path that changes control — Steal, Steal all, the
//! board swap and a rotation across the centre line — and on nothing else. R172 is the other half
//! the brief asked about: a stolen unit dies as its controller's.
//!
//! Engine fixtures only (the engine never depends on `packages/cards`). The real cards prove the
//! same rows again in `packages/cards/test/control-change.test.ts`, and
//! `control-change.property.test.ts` checks them over random boards.
//!
//! Every game here is past both mulligans, in p1's main phase (the `playing()` pattern of
//! `rulings-b.test.ts`), so `legalActions` and `reduce` answer as they would in a match. The verbs
//! are applied the way `effects-steal.test.ts` applies them: straight through `effect.apply` with
//! the actor as the context's controller, which is also how an effect on the opponent's turn is
//! written (behaviour 12).

use std::collections::BTreeSet;
use std::sync::atomic::{AtomicU32, Ordering};

use jackioh_engine::effects::{steal, steal_all, summon, swap_board};
use jackioh_engine::subsystems::rotation::{RotationArgs, rotate_rings};
use jackioh_engine::testkit::*;
use jackioh_engine::wire::PlayerId::{P1, P2};

use crate::rules::fixtures::catalog::{spell_def, token_def};
use crate::rules::fixtures::combat::{charger, plain, rusher, stacker};
use crate::rules::fixtures::harness::{events_of_type, new_game, put, slot};

// ---------------------------------------------------------------------------
// Fixtures.
// ---------------------------------------------------------------------------

/// A face-down backrow card: the board swap moves the backrow row too (R73).
fn trap() -> CardDef {
    spell_def(
        730,
        json!({ "id": "cc-trap", "index": "730", "name": "Control Trap (fixture)", "type": "Trap" }),
    )
}

/// The shared fixture unit token, which the Reborn fixture's Death summons for "you".
fn token_id() -> String {
    token_def("rush", [Tag::Token]).id
}

/// R172: Reborn, and a Death that summons for its controller, so "you" is observable.
fn reborner() -> CardDef {
    json_as(json!({
        "id": "cc-reborner",
        "index": "731",
        "name": "Reborn Summoner (control-change fixture)",
        "set": "Core",
        "type": "Unit",
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": 1,
        "base": { "attack": 2, "health": 2, "keywords": [{ "kind": "Reborn" }], "text": "Reborn; Death: summon a token" },
        "radiant": { "attack": 4, "health": 4, "keywords": [{ "kind": "Reborn" }], "text": "Reborn; Death: summon a token" },
    }))
}

fn reborner_script() -> Script {
    Script {
        death: Some(hook(|_ctx| vec![summon(json_as(json!({ "defId": token_id() })))])),
        ..Script::default()
    }
}

/// TS's module `let nonce = 0`.
static NONCE: AtomicU32 = AtomicU32::new(0);

fn act(state: &GameState, body: Value) -> GameState {
    let nonce = NONCE.fetch_add(1, Ordering::SeqCst) + 1;
    let input: ActionInput = json_as(body);
    let kind = input.body.action_type();
    let result = reduce(state, &input.with_nonce(format!("cc{nonce}")));
    if let Some(error) = result.error {
        panic!("{kind} refused: {error}");
    }
    result.state
}

/// Past both mulligans, in p1's main phase, with an empty board and this file's fixtures.
fn playing(seed: &str) -> GameState {
    let fresh = new_game(seed, None);
    let mut catalog = registered_catalog().clone();
    catalog.insert(trap().id, trap());
    catalog.insert(reborner().id, reborner());
    register_catalog(catalog);
    let mut scripts = registered_scripts().clone();
    scripts.insert(
        reborner().id,
        CardScripts {
            base: reborner_script(),
            radiant: reborner_script(),
        },
    );
    register_scripts(scripts);
    let mut state = begin_game(&fresh).state;
    let keep: Vec<String> = state.players[P1].hand.iter().map(|c| c.id.clone()).collect();
    state = act(&state, json!({ "type": "mulligan", "keep": keep, "playerId": "p1" }));
    let keep: Vec<String> = state.players[P2].hand.iter().map(|c| c.id.clone()).collect();
    state = act(&state, json!({ "type": "mulligan", "keep": keep, "playerId": "p2" }));
    assert_eq!(state.phase, Phase::Main);
    assert_eq!(state.active, P1);
    assert_eq!(state.pending, None);
    state
}

/// Apply one effect for `actor`, in place, and hand back what it emitted.
fn run(state: &mut GameState, effect: Effect, actor: PlayerId) -> Vec<GameEvent> {
    let mut events: Vec<GameEvent> = Vec::new();
    let mut rng = Rng::new(&state.seed, state.rng_cursor);
    {
        let mut sink = EngineSink::new(state, &mut events, &mut rng);
        let mut ctx = make_context(
            &mut sink,
            None,
            HookOptions {
                controller: Some(actor),
                ..Default::default()
            },
        );
        (effect.apply)(&mut ctx);
    }
    state.rng_cursor = rng.cursor();
    events
}

fn rotate(state: &mut GameState, direction: RotationDirection, radiant: bool) -> Vec<GameEvent> {
    let mut events: Vec<GameEvent> = Vec::new();
    let mut rng = Rng::new(&state.seed, state.rng_cursor);
    let perspective = state.active;
    {
        let mut sink = EngineSink::new(state, &mut events, &mut rng);
        rotate_rings(
            &mut sink,
            &RotationArgs {
                direction,
                perspective,
                radiant: Some(radiant),
            },
        );
    }
    state.rng_cursor = rng.cursor();
    events
}

/// A unit that entered on an earlier turn and has acted in every way it can.
fn exhausted(state: &mut GameState, card: CardInstance, turn: i32) -> CardInstance {
    let held = find_instance_mut(state, &card.id).expect("on the field");
    held.summoned_turn = Some(turn);
    held.exertion = Exertion {
        attacked: true,
        switched: true,
        attacks: None,
    };
    held.clone()
}

/// A unit that entered on an earlier turn and has not acted.
fn ready(state: &mut GameState, card: CardInstance, turn: i32) -> CardInstance {
    let held = find_instance_mut(state, &card.id).expect("on the field");
    held.summoned_turn = Some(turn);
    held.exertion = Exertion {
        attacked: false,
        switched: false,
        attacks: None,
    };
    held.clone()
}

const FRESH: Exertion = Exertion {
    attacked: false,
    switched: false,
    attacks: None,
};

const SPENT: Exertion = Exertion {
    attacked: true,
    switched: true,
    attacks: None,
};

fn hero(player: PlayerId) -> AttackTarget {
    AttackTarget::Hero { player }
}

fn unit(instance: &CardInstance) -> AttackTarget {
    AttackTarget::Unit {
        instance: instance.clone(),
    }
}

/// TS's `string | null` refusal, from the engine's `Result` (SURFACE §4.4.9).
fn refusal(result: Result<(), EngineError>) -> Option<String> {
    result.err().map(|error| error.message)
}

/// The attacks `legalActions` offers this unit, as target ids.
fn offered_attacks(state: &GameState, card: &CardInstance) -> Vec<String> {
    legal_actions(state, card.controller)
        .into_iter()
        .filter_map(|action| match action {
            ActionBody::Attack {
                attacker_id,
                target_id,
            } if attacker_id == card.id => Some(target_id),
            _ => None,
        })
        .collect()
}

fn offers_switch(state: &GameState, card: &CardInstance) -> bool {
    legal_actions(state, card.controller)
        .iter()
        .any(|action| matches!(action, ActionBody::SwitchPosition { instance_id } if *instance_id == card.id))
}

fn live(state: &GameState, card: &CardInstance) -> CardInstance {
    match find_instance(state, &card.id) {
        Some(found) => found.clone(),
        None => panic!("{} is gone", card.id),
    }
}

fn control_changed(events: &[GameEvent]) -> Vec<String> {
    events_of_type(events, GameEventType::ControlChanged)
        .iter()
        .map(|event| match event {
            GameEvent::ControlChanged { instance_id, .. } => instance_id.clone(),
            other => panic!("not a controlChanged event: {other:?}"),
        })
        .collect()
}

fn card_id_at(state: &GameState, at: ZoneSlot) -> Option<String> {
    card_at(state, at).map(|card| card.id.clone())
}

// ---------------------------------------------------------------------------
// R171.
// ---------------------------------------------------------------------------

/// `describe("R171 a change of control is an entry (§4.1)")`.
mod r171_a_change_of_control_is_an_entry_s4_1 {
    use super::*;

    #[test]
    fn r171_a_stolen_unit_with_neither_rush_nor_charge_is_summoning_sick_for_the_rest_of_the_turn() {
        let mut state = playing("cc-steal-sick");
        let turn = state.turn;
        let victim = put(&mut state, &plain.id, slot(P2, Row::Units, 3), json!({}));
        let victim = ready(&mut state, victim, turn - 1);
        let bystander = put(&mut state, &plain.id, slot(P2, Row::Units, 5), json!({}));
        let bystander = ready(&mut state, bystander, turn - 1);

        let events = run(&mut state, steal(json_as(json!({ "instanceId": victim.id }))), P1);

        assert_eq!(control_changed(&events), vec![victim.id.clone()]);
        let victim = live(&state, &victim);
        assert_eq!(victim.controller, P1);
        assert_eq!(victim.summoned_turn, Some(turn));
        assert!(is_sick(&state, &victim));
        assert_eq!(
            refusal(why_cannot_attack(&state, &victim, &hero(P2))).as_deref(),
            Some("that unit is summoning sick")
        );
        assert_eq!(
            refusal(why_cannot_attack(&state, &victim, &unit(&live(&state, &bystander)))).as_deref(),
            Some("that unit is summoning sick")
        );
        assert!(attack_targets(&state, &victim).is_empty());
        assert_eq!(offered_attacks(&state, &victim), Vec::<String>::new());
        // §4.1: a sick unit may still switch position, and legalActions offers it.
        assert!(offers_switch(&state, &victim));
    }

    #[test]
    fn r171_a_stolen_rush_unit_may_attack_units_but_not_the_hero_a_stolen_charge_unit_may_attack_both() {
        let mut state = playing("cc-steal-keywords");
        let turn = state.turn;
        let rush = put(&mut state, &rusher.id, slot(P2, Row::Units, 1), json!({}));
        let rush = ready(&mut state, rush, turn - 1);
        let charge = put(&mut state, &charger.id, slot(P2, Row::Units, 2), json!({}));
        let charge = ready(&mut state, charge, turn - 1);
        let bystander = put(&mut state, &plain.id, slot(P2, Row::Units, 5), json!({}));
        let bystander = ready(&mut state, bystander, turn - 1);

        run(&mut state, steal(json_as(json!({ "instanceId": rush.id }))), P1);
        run(&mut state, steal(json_as(json!({ "instanceId": charge.id }))), P1);

        let rush = live(&state, &rush);
        let charge = live(&state, &charge);
        assert_eq!([rush.summoned_turn, charge.summoned_turn], [Some(turn), Some(turn)]);
        assert_eq!(
            refusal(why_cannot_attack(&state, &rush, &hero(P2))).as_deref(),
            Some("Rush cannot hit the hero on its summon turn")
        );
        assert_eq!(offered_attacks(&state, &rush), vec![bystander.id.clone()]);
        assert_eq!(
            offered_attacks(&state, &charge),
            vec![bystander.id.clone(), "hero-p2".to_string()]
        );
    }

    #[test]
    fn r171_the_exertion_is_fresh_a_unit_that_attacked_or_switched_for_the_player_it_left_may_act_again() {
        let mut state = playing("cc-steal-exertion");
        let turn = state.turn;
        let charge = put(&mut state, &charger.id, slot(P2, Row::Units, 1), json!({}));
        let charge = exhausted(&mut state, charge, turn - 1);
        let switcher = put(&mut state, &plain.id, slot(P2, Row::Units, 2), json!({}));
        let switcher = exhausted(&mut state, switcher, turn - 1);

        run(&mut state, steal(json_as(json!({ "instanceId": charge.id }))), P1);
        run(&mut state, steal(json_as(json!({ "instanceId": switcher.id }))), P1);

        let charge = live(&state, &charge);
        let switcher = live(&state, &switcher);
        assert_eq!(charge.exertion, FRESH);
        assert_eq!(switcher.exertion, FRESH);
        assert_eq!(offered_attacks(&state, &charge), vec!["hero-p2".to_string()]);
        assert!(offers_switch(&state, &switcher));
    }

    #[test]
    fn r171_steal_all_c86_marks_every_unit_it_takes_and_nothing_it_leaves_behind() {
        let mut state = playing("cc-steal-all");
        let turn = state.turn;
        for lane in 1..=3 {
            let mine = put(&mut state, &plain.id, slot(P1, Row::Units, lane), json!({}));
            ready(&mut state, mine, turn - 1);
        }
        let mut taken: Vec<CardInstance> = Vec::new();
        for lane in [1, 2] {
            let card = put(&mut state, &plain.id, slot(P2, Row::Units, lane), json!({}));
            taken.push(exhausted(&mut state, card, turn - 1));
        }
        let mut left_behind: Vec<CardInstance> = Vec::new();
        for lane in [3, 4, 5] {
            let card = put(&mut state, &plain.id, slot(P2, Row::Units, lane), json!({}));
            left_behind.push(exhausted(&mut state, card, turn - 1));
        }

        let events = run(&mut state, steal_all(Default::default()), P1);

        assert_eq!(
            control_changed(&events),
            taken.iter().map(|c| c.id.clone()).collect::<Vec<_>>()
        );
        for card in &taken {
            let card = live(&state, card);
            assert_eq!(card.controller, P1);
            assert_eq!(card.summoned_turn, Some(turn));
            assert_eq!(card.exertion, FRESH);
        }
        // R15: no free zone, so these stay with their owner, untouched.
        for card in &left_behind {
            let card = live(&state, card);
            assert_eq!(card.controller, P2);
            assert_eq!(card.summoned_turn, Some(turn - 1));
            assert_eq!(card.exertion, SPENT);
        }
    }

    #[test]
    fn r171_the_board_swap_c87_marks_every_card_that_changes_sides_dormant_stack_cards_and_backrow_cards_included() {
        let mut state = playing("cc-swap-marks");
        let turn = state.turn;
        let mine = put(&mut state, &plain.id, slot(P1, Row::Units, 1), json!({}));
        let mine = exhausted(&mut state, mine, turn - 1);
        let beneath = put(&mut state, &plain.id, slot(P1, Row::Units, 2), json!({}));
        let beneath = exhausted(&mut state, beneath, turn - 2);
        let mut top = new_instance(&mut state, &stacker.id, P1, Zone::Hand { player: P1 });
        let top_id = top.id.clone();
        assert!(place_on_field(
            &mut state,
            &mut top,
            slot(P1, Row::Units, 2),
            json_as(json!({ "stack": true }))
        ));
        let top = live_by_id(&state, &top_id);
        let top = exhausted(&mut state, top, turn - 1);
        let my_trap = put(&mut state, &trap().id, slot(P1, Row::Backrow, 3), json!({}));
        find_instance_mut(&mut state, &my_trap.id).expect("in the backrow").summoned_turn = Some(turn - 1);
        let theirs = put(&mut state, &plain.id, slot(P2, Row::Units, 4), json!({}));
        let theirs = exhausted(&mut state, theirs, turn - 1);
        let their_trap = put(&mut state, &trap().id, slot(P2, Row::Backrow, 5), json!({}));
        find_instance_mut(&mut state, &their_trap.id).expect("in the backrow").summoned_turn = Some(turn - 1);

        let events = run(&mut state, swap_board(), P1);

        let moved = [&mine, &top, &beneath, &my_trap, &theirs, &their_trap];
        assert_eq!(
            control_changed(&events).into_iter().collect::<BTreeSet<_>>(),
            moved.iter().map(|card| card.id.clone()).collect::<BTreeSet<_>>()
        );
        for card in moved {
            let now = live(&state, card);
            assert_eq!(now.summoned_turn, Some(turn), "{}", card.id);
            assert_eq!(now.exertion, FRESH, "{}", card.id);
        }
        // The pile crossed whole, top still on top (§3.2), and the dormant card is marked too.
        assert_eq!(card_id_at(&state, slot(P2, Row::Units, 2)), Some(top.id.clone()));
        assert_eq!(live(&state, &beneath).controller, P2);
    }

    #[test]
    fn r171_the_board_swap_units_the_caster_receives_are_sick_units_the_opponent_receives_attack_on_its_next_turn() {
        let mut state = playing("cc-swap-turns");
        let turn = state.turn;
        let own = put(&mut state, &plain.id, slot(P1, Row::Units, 1), json!({}));
        ready(&mut state, own, turn - 1);
        let their_plain = put(&mut state, &plain.id, slot(P2, Row::Units, 3), json!({}));
        let their_plain = ready(&mut state, their_plain, turn - 1);
        let their_charge = put(&mut state, &charger.id, slot(P2, Row::Units, 4), json!({}));
        let their_charge = ready(&mut state, their_charge, turn - 1);
        let their_rush = put(&mut state, &rusher.id, slot(P2, Row::Units, 5), json!({}));
        let their_rush = ready(&mut state, their_rush, turn - 1);

        run(&mut state, swap_board(), P1);

        // What p1 received entered this turn: sick, with Rush and Charge applying as usual.
        assert!(attack_targets(&state, &live(&state, &their_plain)).is_empty());
        assert!(offered_attacks(&state, &live(&state, &their_charge)).contains(&"hero-p2".to_string()));
        assert!(!offered_attacks(&state, &live(&state, &their_rush)).contains(&"hero-p2".to_string()));
        assert!(!offered_attacks(&state, &live(&state, &their_rush)).is_empty());

        // What p2 received is free on p2's next turn, which is the next turn of the game.
        state = act(&state, json!({ "type": "endTurn", "playerId": "p1" }));
        assert_eq!(state.active, P2);
        let Some(received) = card_at(&state, slot(P2, Row::Units, 1)).cloned() else {
            panic!("p2 should hold p1's old unit in lane 1");
        };
        assert!(!is_sick(&state, &received));
        assert!(offered_attacks(&state, &received).contains(&"hero-p1".to_string()));
    }

    #[test]
    fn r171_a_rotation_marks_the_cards_that_cross_the_centre_line_and_nothing_that_moves_along_its_own_side() {
        let mut state = playing("cc-rotate");
        let turn = state.turn;
        // Rotating right from p1's seat: p1 lane n → n+1, p1 lane 5 → p2 lane 5, p2 lane 1 → p1 lane 1.
        let along_ready = put(&mut state, &plain.id, slot(P1, Row::Units, 2), json!({}));
        let along_ready = ready(&mut state, along_ready, turn - 1);
        let along_spent = put(&mut state, &plain.id, slot(P1, Row::Units, 3), json!({}));
        {
            let held = find_instance_mut(&mut state, &along_spent.id).expect("on the field");
            held.summoned_turn = Some(turn - 1);
            held.exertion = Exertion {
                attacked: true,
                switched: false,
                attacks: None,
            };
        }
        let outbound = put(&mut state, &plain.id, slot(P1, Row::Units, 5), json!({}));
        let outbound = exhausted(&mut state, outbound, turn - 1);
        let inbound = put(&mut state, &plain.id, slot(P2, Row::Units, 1), json!({}));
        let inbound = exhausted(&mut state, inbound, turn - 1);

        let events = rotate(&mut state, RotationDirection::Right, false);

        assert_eq!(
            control_changed(&events).into_iter().collect::<BTreeSet<_>>(),
            [outbound.id.clone(), inbound.id.clone()].into_iter().collect::<BTreeSet<_>>()
        );
        for card in [&outbound, &inbound] {
            let now = live(&state, card);
            assert_eq!(now.summoned_turn, Some(turn), "{}", card.id);
            assert_eq!(now.exertion, FRESH, "{}", card.id);
        }
        let inbound = live(&state, &inbound);
        assert_eq!(inbound.controller, P1);
        assert!(attack_targets(&state, &inbound).is_empty());

        // Along its own side a card has entered nothing: the ready one still attacks, the spent one
        // still cannot.
        let along_ready = live(&state, &along_ready);
        assert_eq!(along_ready.summoned_turn, Some(turn - 1));
        assert_eq!(along_ready.exertion, FRESH);
        assert!(offered_attacks(&state, &along_ready).contains(&"hero-p2".to_string()));
        let along_spent = live(&state, &along_spent);
        assert_eq!(along_spent.summoned_turn, Some(turn - 1));
        assert_eq!(
            along_spent.exertion,
            Exertion {
                attacked: true,
                switched: false,
                attacks: None
            }
        );
        assert_eq!(
            refusal(why_cannot_attack(&state, &along_spent, &hero(P2))).as_deref(),
            Some("that unit has already acted this turn")
        );
    }

    #[test]
    fn r171_a_radiant_rotation_bounces_what_it_would_lose_marks_what_crosses_onto_its_side_and_nothing_that_moves_along_a_side(
    ) {
        let mut state = playing("cc-rotate-radiant");
        let turn = state.turn;
        // Rotating right from p1's seat: p1 lane 5 would cross to p2 and is bounced instead (R14);
        // p2 lane 1 crosses onto p1's side as it would on the base face.
        let along = put(&mut state, &plain.id, slot(P1, Row::Units, 2), json!({}));
        let along = exhausted(&mut state, along, turn - 1);
        let their_along = put(&mut state, &plain.id, slot(P2, Row::Units, 3), json!({}));
        let their_along = ready(&mut state, their_along, turn - 1);
        let outbound = put(&mut state, &plain.id, slot(P1, Row::Units, 5), json!({}));
        let inbound = put(&mut state, &plain.id, slot(P2, Row::Units, 1), json!({}));
        let inbound = exhausted(&mut state, inbound, turn - 1);

        let events = rotate(&mut state, RotationDirection::Right, true);

        assert_eq!(control_changed(&events), vec![inbound.id.clone()]);
        assert_eq!(live(&state, &outbound).zone.z(), ZoneName::Hand);
        let inbound = live(&state, &inbound);
        assert_eq!(inbound.controller, P1);
        assert_eq!(inbound.owner, P2);
        assert_eq!(inbound.summoned_turn, Some(turn));
        assert_eq!(inbound.exertion, FRESH);
        assert!(attack_targets(&state, &inbound).is_empty());

        let along = live(&state, &along);
        assert_eq!(along.summoned_turn, Some(turn - 1));
        assert_eq!(along.exertion, SPENT);
        let their_along = live(&state, &their_along);
        assert_eq!(their_along.summoned_turn, Some(turn - 1));
        assert_eq!(their_along.controller, P2);
    }

    #[test]
    fn r171_a_round_trip_in_one_turn_is_two_entries_so_a_ready_unit_that_crosses_away_and_back_is_sick_again() {
        let mut state = playing("cc-round-trip");
        let turn = state.turn;
        // Right then left from p1's seat: p1 lane 5 → p2 lane 5 → p1 lane 5.
        let traveller = put(&mut state, &plain.id, slot(P1, Row::Units, 5), json!({}));
        let traveller = ready(&mut state, traveller, turn - 1);
        assert!(offered_attacks(&state, &traveller).contains(&"hero-p2".to_string()));

        rotate(&mut state, RotationDirection::Right, false);
        assert_eq!(live(&state, &traveller).controller, P2);
        rotate(&mut state, RotationDirection::Left, false);

        let now = live(&state, &traveller);
        assert_eq!(now.controller, P1);
        assert_eq!(card_id_at(&state, slot(P1, Row::Units, 5)), Some(traveller.id.clone()));
        assert_eq!(now.summoned_turn, Some(turn));
        assert!(attack_targets(&state, &now).is_empty());
    }

    #[test]
    fn r171_a_charge_unit_that_attacked_and_made_the_round_trip_may_attack_once_more() {
        let mut state = playing("cc-round-trip-charge");
        let turn = state.turn;
        // Left then right from p1's seat: p1 lane 1 → p2 lane 1 → p1 lane 1.
        let charge = put(&mut state, &charger.id, slot(P1, Row::Units, 1), json!({}));
        {
            let held = find_instance_mut(&mut state, &charge.id).expect("on the field");
            held.summoned_turn = Some(turn - 1);
            held.exertion = Exertion {
                attacked: true,
                switched: false,
                attacks: None,
            };
        }
        assert_eq!(offered_attacks(&state, &live(&state, &charge)), Vec::<String>::new());

        rotate(&mut state, RotationDirection::Left, false);
        assert_eq!(live(&state, &charge).controller, P2);
        rotate(&mut state, RotationDirection::Right, false);

        let now = live(&state, &charge);
        assert_eq!(now.controller, P1);
        assert_eq!(now.summoned_turn, Some(turn));
        assert_eq!(now.exertion, FRESH);
        assert!(offered_attacks(&state, &now).contains(&"hero-p2".to_string()));
    }

    #[test]
    fn r171_r76_r15_a_steal_that_does_nothing_touches_neither_summonedturn_nor_exertion() {
        let mut state = playing("cc-noop");
        let turn = state.turn;
        let mine = put(&mut state, &plain.id, slot(P1, Row::Units, 1), json!({}));
        let mine = exhausted(&mut state, mine, turn - 1);
        for lane in 2..=5 {
            put(&mut state, &plain.id, slot(P1, Row::Units, lane), json!({}));
        }
        let theirs = put(&mut state, &plain.id, slot(P2, Row::Units, 3), json!({}));
        let theirs = exhausted(&mut state, theirs, turn - 1);

        // R76: already yours. R15: no free zone on the thief's side.
        assert_eq!(run(&mut state, steal(json_as(json!({ "instanceId": mine.id }))), P1), vec![]);
        assert_eq!(run(&mut state, steal(json_as(json!({ "instanceId": theirs.id }))), P1), vec![]);

        for card in [&mine, &theirs] {
            let now = live(&state, card);
            assert_eq!(now.summoned_turn, Some(turn - 1));
            assert_eq!(now.exertion, SPENT);
        }
        assert_eq!(live(&state, &theirs).controller, P2);
    }

    #[test]
    fn r171_a_change_of_control_on_the_opponent_s_turn_leaves_the_unit_ready_on_its_new_controller_s_next_turn() {
        let mut state = playing("cc-opponents-turn");
        let turn = state.turn;
        let first = put(&mut state, &plain.id, slot(P1, Row::Units, 1), json!({}));
        ready(&mut state, first, turn - 1);
        let victim = put(&mut state, &plain.id, slot(P1, Row::Units, 2), json!({}));
        let victim = exhausted(&mut state, victim, turn - 1);
        let other = put(&mut state, &plain.id, slot(P2, Row::Units, 5), json!({}));
        ready(&mut state, other, turn - 1);

        // p1 is active, so this is the inactive player stealing: the unit enters p2's side on turn T.
        run(&mut state, steal(json_as(json!({ "instanceId": victim.id }))), P2);
        let now = live(&state, &victim);
        assert_eq!(now.controller, P2);
        assert_eq!(now.summoned_turn, Some(turn));

        state = act(&state, json!({ "type": "endTurn", "playerId": "p1" }));

        assert_eq!(state.active, P2);
        assert_eq!(state.turn, turn + 1);
        let stolen = live(&state, &victim);
        assert!(!is_sick(&state, &stolen));
        assert!(offered_attacks(&state, &stolen).contains(&"hero-p1".to_string()));
    }

    #[test]
    fn r171_r53_forced_attacks_still_ignore_sickness_and_spend_nothing() {
        let mut state = playing("cc-forced");
        let turn = state.turn;
        let victim = put(&mut state, &plain.id, slot(P2, Row::Units, 2), json!({}));
        let victim = ready(&mut state, victim, turn - 1);
        find_instance_mut(&mut state, &victim.id).expect("on the field").buffs = AttackHealth { attack: 0, health: 10 };
        let target = put(&mut state, &plain.id, slot(P2, Row::Units, 4), json!({}));
        let target = ready(&mut state, target, turn - 1);
        find_instance_mut(&mut state, &target.id).expect("on the field").buffs = AttackHealth { attack: 0, health: 10 };
        run(&mut state, steal(json_as(json!({ "instanceId": victim.id }))), P1);
        assert!(is_sick(&state, &live(&state, &victim)));

        let mut events: Vec<GameEvent> = Vec::new();
        let mut rng = Rng::new(&state.seed, state.rng_cursor);
        {
            let mut sink = EngineSink::new(&mut state, &mut events, &mut rng);
            let attacker = live(sink.state, &victim);
            let aim = unit(&live(sink.state, &target));
            force_attack(&mut sink, &attacker, &aim);
        }

        assert_eq!(
            serde_json::to_value(events_of_type(&events, GameEventType::AttackDeclared)).expect("serialises"),
            json!([{ "type": "attackDeclared", "attackerId": victim.id, "targetId": target.id, "forced": true }])
        );
        let victim = live(&state, &victim);
        assert_eq!(victim.exertion, FRESH);
        assert_eq!(live(&state, &target).damage, unit_view(&state, &victim).attack);
    }
}

fn live_by_id(state: &GameState, id: &str) -> CardInstance {
    find_instance(state, id).cloned().unwrap_or_else(|| panic!("{id} is gone"))
}

// ---------------------------------------------------------------------------
// R172.
// ---------------------------------------------------------------------------

/// `describe("R172 a stolen unit dies as its controller's")`.
mod r172_a_stolen_unit_dies_as_its_controller_s {
    use super::*;

    #[test]
    fn r172_a_stolen_reborn_unit_returns_to_the_zone_it_reserved_on_the_thief_s_side_owned_by_its_owner_and_sick() {
        let mut state = playing("cc-reborn");
        let turn = state.turn;
        let first = put(&mut state, &plain.id, slot(P1, Row::Units, 1), json!({}));
        ready(&mut state, first, turn - 1);
        let body = put(&mut state, &reborner().id, slot(P2, Row::Units, 3), json!({}));
        let body = ready(&mut state, body, turn - 1);
        let other = put(&mut state, &plain.id, slot(P2, Row::Units, 5), json!({}));
        ready(&mut state, other, turn - 1);
        run(&mut state, steal(json_as(json!({ "instanceId": body.id }))), P1);
        assert_eq!(card_id_at(&state, slot(P1, Row::Units, 3)), Some(body.id.clone()));

        let max_health = unit_view(&state, &live(&state, &body)).max_health;
        find_instance_mut(&mut state, &body.id).expect("on the field").damage = max_health;
        let mut events: Vec<GameEvent> = Vec::new();
        let mut rng = Rng::new(&state.seed, state.rng_cursor);
        state_check(&mut EngineSink::new(&mut state, &mut events, &mut rng));

        // It died as p1's, went to its owner's pile on the way (R12), and came back where it died.
        let destroyed: Vec<(String, PlayerId)> = events_of_type(&events, GameEventType::Destroyed)
            .iter()
            .map(|event| match event {
                GameEvent::Destroyed { instance_id, owner, .. } => (instance_id.clone(), *owner),
                other => panic!("not a destroyed event: {other:?}"),
            })
            .collect();
        assert_eq!(destroyed, vec![(body.id.clone(), P2)]);
        let back = live(&state, &body);
        assert_eq!(card_id_at(&state, slot(P1, Row::Units, 3)), Some(body.id.clone()));
        assert_eq!(back.controller, P1);
        assert_eq!(back.owner, P2);
        assert!(!state.players[P1].graveyard.iter().any(|c| c.id == body.id));
        assert!(!state.players[P2].graveyard.iter().any(|c| c.id == body.id));
        // R83: the return is an entry, so it is sick for the rest of the turn.
        assert_eq!(back.summoned_turn, Some(turn));
        assert!(attack_targets(&state, &back).is_empty());
    }

    #[test]
    fn r172_a_stolen_unit_s_death_runs_for_the_player_who_controlled_it_when_it_died() {
        let mut state = playing("cc-death");
        let turn = state.turn;
        let body = put(&mut state, &reborner().id, slot(P2, Row::Units, 2), json!({}));
        let body = ready(&mut state, body, turn - 1);
        let other = put(&mut state, &plain.id, slot(P2, Row::Units, 5), json!({}));
        ready(&mut state, other, turn - 1);
        run(&mut state, steal(json_as(json!({ "instanceId": body.id }))), P1);

        let max_health = unit_view(&state, &live(&state, &body)).max_health;
        find_instance_mut(&mut state, &body.id).expect("on the field").damage = max_health;
        let mut events: Vec<GameEvent> = Vec::new();
        let mut rng = Rng::new(&state.seed, state.rng_cursor);
        state_check(&mut EngineSink::new(&mut state, &mut events, &mut rng));

        // "Summon a token" for "you": p1, into p1's leftmost free zone, since the Reborn body holds lane 2.
        let token = token_id();
        let summoned: Vec<(PlayerId, i32)> = events_of_type(&events, GameEventType::Summoned)
            .iter()
            .filter_map(|event| match event {
                GameEvent::Summoned { def_id, player, lane, .. } if *def_id == token => Some((*player, *lane)),
                _ => None,
            })
            .collect();
        assert_eq!(summoned, vec![(P1, 1)]);
        assert_eq!(
            card_at(&state, slot(P1, Row::Units, 1)).map(|card| card.def_id.clone()),
            Some(token.clone())
        );
        assert_eq!(card_id_at(&state, slot(P1, Row::Units, 2)), Some(body.id.clone()));
        assert!(card_at(&state, slot(P2, Row::Units, 1)).is_none());
    }
}
