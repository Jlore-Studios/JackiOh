//! Positions and exertion (SPEC §4.1, R6, R7, R20, R49; BUILD M2-T1).
//!
//! Port of `packages/engine/test/combat-positions.test.ts`.

use std::sync::atomic::{AtomicU32, Ordering};

use serde::Serialize;

use jackioh_engine::testkit::PlayerId::{P1, P2};
use jackioh_engine::testkit::*;

use crate::rules::fixtures::combat::{armoured, big_dfender, deft_duelist, plain, spikey_pillow, taunter};
use crate::rules::fixtures::harness::{events_of_type, in_hand, new_game, put, sink_for, slot};

/// An `ActionInput` from its TS object literal (SURFACE §8: an object literal ports as `json!`).
fn input(body: Value) -> ActionInput {
    json_as(body)
}

fn json_of(value: impl Serialize) -> Value {
    serde_json::to_value(value).expect("serialises")
}

/// Jest's `toMatchObject`: every key `expected` names is in `actual` and matches, recursively; an
/// array matches element by element and in length; anything else is equal.
fn matches_object(actual: &Value, expected: &Value) -> bool {
    match (actual, expected) {
        (Value::Object(actual), Value::Object(expected)) => expected
            .iter()
            .all(|(key, want)| actual.get(key).is_some_and(|got| matches_object(got, want))),
        (Value::Array(actual), Value::Array(expected)) => {
            actual.len() == expected.len()
                && actual
                    .iter()
                    .zip(expected)
                    .all(|(got, want)| matches_object(got, want))
        }
        _ => actual == expected,
    }
}

fn expect_match(actual: impl Serialize, expected: Value) {
    let actual = json_of(actual);
    assert!(
        matches_object(&actual, &expected),
        "{actual} does not match {expected}"
    );
}

/// The refusal a check gives, or `None` when it allows (TS `{ error?: string }`).
fn refusal(check: Result<(), EngineError>) -> Option<String> {
    check.err().map(|error| error.message)
}

/// The exertion TS's object literal `{ attacked, switched }` writes (`attacks` absent).
fn exertion(attacked: bool, switched: bool) -> Exertion {
    Exertion {
        attacked,
        switched,
        attacks: None,
    }
}

fn by_id<'a>(state: &'a GameState, id: &str) -> &'a CardInstance {
    find_instance(state, id).unwrap_or_else(|| panic!("no card {id}"))
}

/// The card as it stands in `state` now, owned (TS held the live object).
fn live(state: &GameState, id: &str) -> CardInstance {
    by_id(state, id).clone()
}

/// The layered view of the card `id` names, as it stands now.
fn view_of(state: &GameState, id: &str) -> UnitView {
    unit_view(state, by_id(state, id))
}

/// TS's module `let nonce`.
static NONCE: AtomicU32 = AtomicU32::new(0);

fn attempt(state: &GameState, body: ActionInput) -> ReduceResult {
    let nonce = NONCE.fetch_add(1, Ordering::SeqCst) + 1;
    reduce(state, &body.with_nonce(format!("cp{nonce}")))
}

fn act(state: &GameState, body: ActionInput) -> GameState {
    let result = attempt(state, body);
    if let Some(error) = &result.error {
        panic!("{error}");
    }
    result.state
}

/// Past the mulligans, in the main phase of turn 1.
fn playing(seed: &str) -> GameState {
    let mut state = begin_game(&new_game(seed, None)).state;
    for player in [P1, P2] {
        let keep: Vec<String> = state.players[player]
            .hand
            .iter()
            .map(|card| card.id.clone())
            .collect();
        state = act(
            &state,
            input(json!({ "type": "mulligan", "keep": keep, "playerId": player })),
        );
    }
    state
}

fn end_turns(state: &GameState, count: usize) -> GameState {
    let mut next = state.clone();
    for _ in 0..count {
        let active = next.active;
        next = act(&next, input(json!({ "type": "endTurn", "playerId": active })));
    }
    next
}

/// `reduce` clones, so a unit is re-read from the state it lives in, by its lane.
fn unit_at(state: &GameState, player: PlayerId, lane: i32) -> CardInstance {
    state.players[player].units[(lane - 1) as usize]
        .as_ref()
        .and_then(|pile| pile.first())
        .cloned()
        .unwrap_or_else(|| panic!("no unit in {player} lane {lane}"))
}

fn hand_card(state: &mut GameState, def_id: &str, player: PlayerId) -> CardInstance {
    in_hand(state, def_id, player, 1)
        .into_iter()
        .next()
        .unwrap_or_else(|| panic!("could not add {def_id} to {player}'s hand"))
}

/// The spell path of R20: a switch that is an effect, not the player's action. The events it
/// emitted, and its refusal.
fn switch_by_effect(
    state: &mut GameState,
    unit: &CardInstance,
    to: Option<Position>,
) -> (Vec<GameEvent>, Option<String>) {
    let now = live(state, &unit.id);
    let mut sink = sink_for(state);
    let result = switch_position(
        &mut sink,
        &now,
        SwitchPositionOptions {
            spend_exertion: Some(false),
            to,
        },
    );
    (sink.events.clone(), refusal(result))
}

fn has_switch_for(state: &GameState, player: PlayerId, id: Option<&str>) -> bool {
    legal_actions(state, player).iter().any(|action| match action {
        ActionBody::SwitchPosition { instance_id } => id.is_none_or(|id| instance_id == id),
        _ => false,
    })
}

mod positions_and_exertion_m2_t1 {
    use super::*;

    #[test]
    fn units_enter_the_field_in_attack_position_4_1() {
        let mut state = playing("enter-atk");
        let card = hand_card(&mut state, &plain.id, P1);
        let mut played = act(
            &state,
            input(json!({
                "type": "play", "instanceId": card.id, "zone": { "row": "units", "lane": 2 }, "playerId": "p1"
            })),
        );

        let view = unit_view(&played, &unit_at(&played, P1, 2));
        assert_eq!(view.position, Position::Atk);
        assert!(!has_keyword(&view.keywords, KeywordKind::Taunt));
        assert_eq!(view.armor, 0);

        // A unit put on the field by anything else enters the same way.
        let summoned = put(
            &mut played,
            &plain.id,
            slot(P1, Row::Units, 4),
            Default::default(),
        );
        assert_eq!(view_of(&played, &summoned.id).position, Position::Atk);
    }

    #[test]
    fn r6_a_unit_that_switched_position_cannot_attack_that_turn() {
        let mut state = playing("switch-then-attack");
        put(&mut state, &plain.id, slot(P1, Row::Units, 1), Default::default());

        let first = unit_at(&state, P1, 1).id;
        state = act(
            &state,
            input(json!({ "type": "switchPosition", "instanceId": first, "playerId": "p1" })),
        );
        let mut unit = unit_at(&state, P1, 1);
        assert_eq!(unit_view(&state, &unit).position, Position::Def);
        assert!(!has_exertion(&state, &unit, ExertionKind::Attack));
        assert!(attack_targets(&state, &unit).is_empty());
        assert!(
            attempt(
                &state,
                input(json!({ "type": "attack", "attackerId": unit.id, "targetId": "hero-p2", "playerId": "p1" }))
            )
            .error
            .unwrap_or_default()
            .contains("already acted")
        );

        // R6 proper: switching back to Attack Position also spends the turn's exertion.
        state = end_turns(&state, 2); // p1's next turn, still in Defense
        unit = unit_at(&state, P1, 1);
        assert_eq!(unit_view(&state, &unit).position, Position::Def);

        state = act(
            &state,
            input(json!({ "type": "switchPosition", "instanceId": unit.id, "playerId": "p1" })),
        );
        unit = unit_at(&state, P1, 1);
        assert_eq!(unit_view(&state, &unit).position, Position::Atk);
        assert!(!has_exertion(&state, &unit, ExertionKind::Attack));
        assert!(attack_targets(&state, &unit).is_empty());
        assert!(
            attempt(
                &state,
                input(json!({ "type": "attack", "attackerId": unit.id, "targetId": "hero-p2", "playerId": "p1" }))
            )
            .error
            .unwrap_or_default()
            .contains("already acted")
        );
        assert_eq!(state.players.p2.hero.health, 30);
    }

    #[test]
    fn a_unit_that_attacked_cannot_switch_position_that_turn_4_1() {
        let mut state = playing("attack-then-switch");
        put(&mut state, &plain.id, slot(P1, Row::Units, 1), Default::default());

        let attacker = unit_at(&state, P1, 1).id;
        state = act(
            &state,
            input(
                json!({ "type": "attack", "attackerId": attacker, "targetId": "hero-p2", "playerId": "p1" }),
            ),
        );
        assert_eq!(state.players.p2.hero.health, 27);

        let unit = unit_at(&state, P1, 1);
        assert!(unit.exertion.attacked);
        assert!(!has_exertion(&state, &unit, ExertionKind::Switch));
        assert!(!has_switch_for(&state, P1, None));
        assert!(
            attempt(
                &state,
                input(json!({ "type": "switchPosition", "instanceId": unit.id, "playerId": "p1" }))
            )
            .error
            .unwrap_or_default()
            .contains("already acted")
        );
        assert_eq!(unit_view(&state, &unit).position, Position::Atk);
    }

    #[test]
    fn resets_both_exertions_at_the_controllers_next_turn_not_the_opponents_4_1() {
        let mut state = playing("exertion-reset");
        put(&mut state, &plain.id, slot(P1, Row::Units, 1), Default::default());
        put(&mut state, &plain.id, slot(P1, Row::Units, 2), Default::default());

        let first = unit_at(&state, P1, 1).id;
        state = act(
            &state,
            input(json!({ "type": "attack", "attackerId": first, "targetId": "hero-p2", "playerId": "p1" })),
        );
        let second = unit_at(&state, P1, 2).id;
        state = act(
            &state,
            input(json!({ "type": "switchPosition", "instanceId": second, "playerId": "p1" })),
        );
        assert_eq!(unit_at(&state, P1, 1).exertion, exertion(true, false));
        assert_eq!(unit_at(&state, P1, 2).exertion, exertion(false, true));

        state = end_turns(&state, 1); // the opponent's turn: p1's units stay spent
        assert_eq!(state.active, P2);
        assert!(!has_exertion(
            &state,
            &unit_at(&state, P1, 1),
            ExertionKind::Switch
        ));
        assert!(!has_exertion(
            &state,
            &unit_at(&state, P1, 2),
            ExertionKind::Attack
        ));

        state = end_turns(&state, 1); // p1's own turn start resets both
        assert_eq!(state.active, P1);
        assert_eq!(unit_at(&state, P1, 1).exertion, exertion(false, false));
        assert_eq!(unit_at(&state, P1, 2).exertion, exertion(false, false));

        let attacker = unit_at(&state, P1, 1);
        assert!(has_exertion(&state, &attacker, ExertionKind::Attack));
        assert!(
            attack_targets(&state, &attacker)
                .iter()
                .any(|target| matches!(target, AttackTarget::Hero { .. }))
        );
        state = act(
            &state,
            input(
                json!({ "type": "attack", "attackerId": attacker.id, "targetId": "hero-p2", "playerId": "p1" }),
            ),
        );
        assert_eq!(state.players.p2.hero.health, 24);
        let second = unit_at(&state, P1, 2).id;
        state = act(
            &state,
            input(json!({ "type": "switchPosition", "instanceId": second, "playerId": "p1" })),
        );
        assert_eq!(unit_view(&state, &unit_at(&state, P1, 2)).position, Position::Atk);
    }

    #[test]
    fn r49_deft_duelist_attacks_and_switches_in_one_turn() {
        let mut state = playing("deft-duelist");
        put(
            &mut state,
            &deft_duelist.id,
            slot(P1, Row::Units, 1),
            Default::default(),
        );
        put(&mut state, &plain.id, slot(P1, Row::Units, 2), Default::default());

        // The plain unit gets one exertion only; the Duelist gets both.
        let first = unit_at(&state, P1, 1).id;
        state = act(
            &state,
            input(json!({ "type": "attack", "attackerId": first, "targetId": "hero-p2", "playerId": "p1" })),
        );
        assert_eq!(state.players.p2.hero.health, 26);

        let mut duelist = unit_at(&state, P1, 1);
        assert!(has_exertion(&state, &duelist, ExertionKind::Switch));
        assert!(!has_exertion(&state, &duelist, ExertionKind::Attack));

        state = act(
            &state,
            input(json!({ "type": "switchPosition", "instanceId": duelist.id, "playerId": "p1" })),
        );
        duelist = unit_at(&state, P1, 1);
        assert_eq!(unit_view(&state, &duelist).position, Position::Def);
        assert_eq!(duelist.exertion, exertion(true, true));

        // Two of the same kind is still refused: one attack and one switch, not two switches.
        assert!(!has_exertion(&state, &duelist, ExertionKind::Switch));
        assert!(
            attempt(
                &state,
                input(json!({ "type": "switchPosition", "instanceId": duelist.id, "playerId": "p1" }))
            )
            .error
            .unwrap_or_default()
            .contains("already acted")
        );
        assert!(
            attempt(
                &state,
                input(json!({ "type": "attack", "attackerId": duelist.id, "targetId": "hero-p2", "playerId": "p1" }))
            )
            .error
            .unwrap_or_default()
            .contains("already acted")
        );
        assert_eq!(state.players.p2.hero.health, 26);

        // R6 is lifted for the Duelist in the other order too: switch to Attack, then attack.
        state = end_turns(&state, 2);
        duelist = unit_at(&state, P1, 1);
        assert_eq!(unit_view(&state, &duelist).position, Position::Def);
        state = act(
            &state,
            input(json!({ "type": "switchPosition", "instanceId": duelist.id, "playerId": "p1" })),
        );
        duelist = unit_at(&state, P1, 1);
        assert_eq!(unit_view(&state, &duelist).position, Position::Atk);
        state = act(
            &state,
            input(
                json!({ "type": "attack", "attackerId": duelist.id, "targetId": "hero-p2", "playerId": "p1" }),
            ),
        );
        assert_eq!(state.players.p2.hero.health, 22);
    }

    #[test]
    fn r20_a_unit_switched_by_a_spell_keeps_its_exertion() {
        let mut state = playing("r20");
        let unit = put(&mut state, &plain.id, slot(P1, Row::Units, 1), Default::default());

        let (to_defense_events, to_defense_error) = switch_by_effect(&mut state, &unit, None);
        assert_eq!(to_defense_error, None);
        assert_eq!(view_of(&state, &unit.id).position, Position::Def);
        assert_eq!(by_id(&state, &unit.id).exertion, exertion(false, false));
        expect_match(
            json_of(events_of_type(
                &to_defense_events,
                GameEventType::PositionSwitched,
            ))[0]
                .clone(),
            json!({ "instanceId": unit.id, "position": "DEF" }),
        );

        // Switched back by a second effect, it can still take its own exertion this turn.
        assert_eq!(switch_by_effect(&mut state, &unit, Some(Position::Atk)).1, None);
        assert_eq!(view_of(&state, &unit.id).position, Position::Atk);
        assert!(has_exertion(
            &state,
            by_id(&state, &unit.id),
            ExertionKind::Attack
        ));
        assert!(has_exertion(
            &state,
            by_id(&state, &unit.id),
            ExertionKind::Switch
        ));

        let now = live(&state, &unit.id);
        {
            let mut sink = sink_for(&mut state);
            assert_eq!(
                refusal(declare_attack(
                    &mut sink,
                    &now,
                    &AttackTarget::Hero { player: P2 }
                )),
                None
            );
        }
        assert_eq!(state.players.p2.hero.health, 27);
        assert_eq!(by_id(&state, &unit.id).exertion, exertion(true, false));

        // And an effect may still flip a unit that has spent its exertion.
        assert_eq!(switch_by_effect(&mut state, &unit, None).1, None);
        assert_eq!(view_of(&state, &unit.id).position, Position::Def);
        assert_eq!(by_id(&state, &unit.id).exertion, exertion(true, false));
    }

    #[test]
    fn defense_position_grants_taunt_4_1() {
        let mut state = playing("defense-taunt");
        let attacker = put(&mut state, &plain.id, slot(P1, Row::Units, 1), Default::default());
        let open = put(&mut state, &plain.id, slot(P2, Row::Units, 1), Default::default());
        let defender = put(&mut state, &plain.id, slot(P2, Row::Units, 2), Default::default());

        assert_eq!(attack_targets(&state, by_id(&state, &attacker.id)).len(), 3); // both units and the hero

        switch_by_effect(&mut state, &defender, Some(Position::Def));
        let view = view_of(&state, &defender.id);
        assert_eq!(view.position, Position::Def);
        assert!(has_keyword(&view.keywords, KeywordKind::Taunt));

        // The granted Taunt is the real thing: it is now the only legal target (§4.2 step 3).
        let targets = attack_targets(&state, by_id(&state, &attacker.id));
        assert_eq!(targets.len(), 1);
        assert!(matches!(targets[0], AttackTarget::Unit { .. }));
        let first = match &targets[0] {
            AttackTarget::Unit { instance, .. } => Some(instance.id.clone()),
            _ => None,
        };
        assert_eq!(first, Some(defender.id.clone()));
        assert!(!has_keyword(
            &view_of(&state, &open.id).keywords,
            KeywordKind::Taunt
        ));

        // Back in Attack Position the Taunt is gone again.
        switch_by_effect(&mut state, &defender, Some(Position::Atk));
        assert!(!has_keyword(
            &view_of(&state, &defender.id).keywords,
            KeywordKind::Taunt
        ));
        assert_eq!(attack_targets(&state, by_id(&state, &attacker.id)).len(), 3);
    }

    #[test]
    fn defense_armor_1_stacks_with_printed_armor_and_big_d_fenders_aura_4_1() {
        let mut state = playing("defense-armor");
        let armour = put(
            &mut state,
            &armoured.id,
            slot(P1, Row::Units, 1),
            Default::default(),
        );
        let dfender = put(
            &mut state,
            &big_dfender.id,
            slot(P1, Row::Units, 2),
            Default::default(),
        );
        let enemy = put(
            &mut state,
            &armoured.id,
            slot(P2, Row::Units, 1),
            Default::default(),
        );

        // In Attack Position neither the position bonus nor the aura applies.
        assert_eq!(view_of(&state, &armour.id).armor, 7);
        assert_eq!(view_of(&state, &dfender.id).armor, 0);

        switch_by_effect(&mut state, &armour, Some(Position::Def));
        assert_eq!(view_of(&state, &armour.id).armor, 10); // 7 printed + 1 Defense + 2 aura
        assert_eq!(
            view_of(&state, &armour.id)
                .keywords
                .iter()
                .filter(|keyword| keyword.kind() == KeywordKind::Armor)
                .count(),
            3
        );

        // Big D-fender's own aura covers itself when it is in Defense.
        switch_by_effect(&mut state, &dfender, Some(Position::Def));
        assert_eq!(view_of(&state, &dfender.id).armor, 3); // 0 printed + 1 Defense + 2 aura

        // The aura is controller-scoped: the enemy's Defense unit gets the +1 only.
        switch_by_effect(&mut state, &enemy, Some(Position::Def));
        assert_eq!(view_of(&state, &enemy.id).armor, 8);

        // Radiant Big D-fender gives +4 instead.
        let mut radiant_state = playing("defense-armor-radiant");
        let radiant_armour = put(
            &mut radiant_state,
            &armoured.id,
            slot(P1, Row::Units, 1),
            Default::default(),
        );
        put(
            &mut radiant_state,
            &big_dfender.id,
            slot(P1, Row::Units, 2),
            json_as(json!({ "radiant": true })),
        );
        switch_by_effect(&mut radiant_state, &radiant_armour, Some(Position::Def));
        assert_eq!(view_of(&radiant_state, &radiant_armour.id).armor, 12); // 7 + 1 + 4
    }

    #[test]
    fn spikey_pillow_cannot_be_switched_to_defense_position_4_1() {
        let mut state = playing("spikey-pillow");
        put(
            &mut state,
            &spikey_pillow.id,
            slot(P1, Row::Units, 1),
            Default::default(),
        );
        put(
            &mut state,
            &taunter.id,
            slot(P1, Row::Units, 2),
            Default::default(),
        );

        let pillow = unit_at(&state, P1, 1);
        assert_eq!(unit_view(&state, &pillow).position, Position::Atk);
        assert!(!has_switch_for(&state, P1, Some(&pillow.id)));
        assert!(has_switch_for(&state, P1, None)); // the other unit may

        let refused = attempt(
            &state,
            input(json!({ "type": "switchPosition", "instanceId": pillow.id, "playerId": "p1" })),
        );
        assert!(
            refused
                .error
                .unwrap_or_default()
                .contains("cannot be in Defense Position")
        );

        // The spell path (R20) cannot sneak it into Defense either.
        let (by_effect_events, by_effect_error) = switch_by_effect(&mut state, &pillow, Some(Position::Def));
        assert!(
            by_effect_error
                .unwrap_or_default()
                .contains("cannot be in Defense Position")
        );
        assert_eq!(
            events_of_type(&by_effect_events, GameEventType::PositionSwitched).len(),
            0
        );

        let after = view_of(&state, &pillow.id);
        assert_eq!(after.position, Position::Atk);
        assert!(!has_keyword(&after.keywords, KeywordKind::Taunt));
        assert_eq!(after.armor, 0);
        assert_eq!(by_id(&state, &pillow.id).exertion, exertion(false, false));

        // The refusal costs nothing, so the unit's exertion is still there next action.
        let other = unit_at(&state, P1, 2).id;
        state = act(
            &state,
            input(json!({ "type": "switchPosition", "instanceId": other, "playerId": "p1" })),
        );
        assert_eq!(unit_view(&state, &unit_at(&state, P1, 2)).position, Position::Def);
    }

    #[test]
    fn a_summoning_sick_unit_may_still_switch_to_defense_4_1() {
        let mut state = playing("sick-switch");
        let card = hand_card(&mut state, &plain.id, P1);
        state = act(
            &state,
            input(json!({
                "type": "play", "instanceId": card.id, "zone": { "row": "units", "lane": 1 }, "playerId": "p1"
            })),
        );

        let mut unit = unit_at(&state, P1, 1);
        assert_eq!(unit.summoned_turn, Some(state.turn));
        assert!(attack_targets(&state, &unit).is_empty());
        assert!(
            attempt(
                &state,
                input(json!({ "type": "attack", "attackerId": unit.id, "targetId": "hero-p2", "playerId": "p1" }))
            )
            .error
            .unwrap_or_default()
            .contains("summoning sick")
        );

        assert!(has_switch_for(&state, P1, Some(&unit.id)));
        state = act(
            &state,
            input(json!({ "type": "switchPosition", "instanceId": unit.id, "playerId": "p1" })),
        );
        unit = unit_at(&state, P1, 1);
        let view = unit_view(&state, &unit);
        assert_eq!(view.position, Position::Def);
        assert!(has_keyword(&view.keywords, KeywordKind::Taunt));
        assert_eq!(view.armor, 1);
        assert_eq!(unit.exertion, exertion(false, true));
    }
}
