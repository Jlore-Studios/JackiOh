// R1174000 (issue #1174): Frozen, a Hearthstone-style status (SPEC §6.1), set by the Freeze verb
// (§6.3): a Frozen Unit loses its next attack or position switch and thaws at the end of the turn in
// which it could have made one. `combat::has_exertion_in` answers false for a Frozen Unit, so
// `legal_actions` and the reducer's refusal agree (BUILD M2-T1); the thaw is stamped as the freeze
// lands (`freeze::thaw_turn`) and done at its controller's cleanup (`freeze::thaw_at_cleanup`).

mod r1174000_a_frozen_unit_loses_its_next_attack_or_switch_and_thaws_after_it {
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use crate::rules::fixtures::combat::{deft_duelist, plain, windfurier};
    use crate::rules::fixtures::freeze::{
        blizzard, chill, frost, goad, grab, hush, ice_trap, rally, recall, register_freeze_fixtures, shiver,
        shove, warded,
    };

    const FROZEN: &str = "that Unit is Frozen";

    /// `active`'s turn 9, its main phase; both sides hold `hand`s (and plain cards to draw, so no turn
    /// ends by itself, §2.5) and the given fields and backrows.
    fn board(active: &str, p1_hand: Value, p1_field: Value, p2_hand: Value, p2_field: Value) -> Scenario {
        register_freeze_fixtures();
        scenario(json!({
            "seed": "r1174000",
            "active": active,
            "p1": { "hand": p1_hand, "field": p1_field, "library": [plain.id, plain.id, plain.id, plain.id] },
            "p2": { "hand": p2_hand, "field": p2_field, "library": [plain.id, plain.id, plain.id, plain.id] },
        }))
    }

    fn id_at(s: &Scenario, player: PlayerId, lane: i32) -> String {
        s.unit(player, lane)
            .unwrap_or_else(|| panic!("no unit at {player:?}'s lane {lane}"))
            .id
    }

    fn pick(id: &str) -> Value {
        json!({ "targets": [{ "pick": "instance", "instanceId": id }] })
    }

    fn thaw_of(s: &Scenario, id: &str) -> Option<i32> {
        s.card(id).frozen
    }

    fn offers_attack(s: &Scenario, id: &str) -> bool {
        let unit = s.card(id).clone();
        legal_actions(s.state(), unit.controller)
            .iter()
            .any(|action| matches!(action, ActionBody::Attack { attacker_id, .. } if *attacker_id == unit.id))
    }

    fn offers_switch(s: &Scenario, id: &str) -> bool {
        let unit = s.card(id).clone();
        legal_actions(s.state(), unit.controller)
            .iter()
            .any(|action| matches!(action, ActionBody::SwitchPosition { instance_id } if *instance_id == unit.id))
    }

    /// The `marked` events of the last step that freeze (`true`) or thaw (`false`) `id`.
    fn frost_marks(events: &[GameEvent], id: &str) -> Vec<bool> {
        events
            .iter()
            .filter_map(|event| match event {
                GameEvent::Marked {
                    instance_id,
                    mark,
                    color,
                    added,
                } if instance_id == id && mark == FROZEN_MARK.mark => {
                    assert_eq!(color, FROZEN_MARK.color);
                    Some(*added)
                }
                _ => None,
            })
            .collect()
    }

    /// The reducer refuses `body` with the Frozen reason and hands the state back unchanged.
    fn refused_frozen(s: &Scenario, body: ActionBody, nonce: &str) {
        let before = hash_state(s.state());
        let player = s.state().active;
        let result = reduce(s.state(), &ActionInput { body, player_id: player }.with_nonce(nonce));
        assert_eq!(result.error.as_deref(), Some(FROZEN), "{nonce}");
        assert!(result.events.is_empty(), "{nonce}: a refusal emits nothing");
        assert_eq!(hash_state(&result.state), before, "{nonce}: a refusal leaves the state unchanged");
    }

    #[test]
    fn r1174000_frozen_on_the_opponents_turn_it_neither_attacks_nor_switches_on_yours_and_thaws_at_your_cleanup() {
        let mut s = board(
            "p2",
            json!([plain.id]),
            json!([{ "def": plain.id, "lane": 1 }]),
            json!([chill.id, plain.id]),
            json!([{ "def": plain.id, "lane": 5 }]),
        );
        let unit = id_at(&s, P1, 1);
        s.play(chill.id.as_str(), pick(&unit));
        assert_eq!(thaw_of(&s, &unit), Some(10), "frozen on turn 9, it thaws at its controller's turn 10");
        assert_eq!(frost_marks(s.last_events(), &unit), vec![true]);

        s.end_turn();
        assert_eq!((s.state().active, s.state().turn), (P1, 10));
        assert!(!offers_attack(&s, &unit));
        assert!(!offers_switch(&s, &unit));
        refused_frozen(
            &s,
            ActionBody::Attack {
                attacker_id: unit.clone(),
                target_id: "hero-p2".to_string(),
            },
            "fz-attack",
        );
        refused_frozen(
            &s,
            ActionBody::SwitchPosition {
                instance_id: unit.clone(),
            },
            "fz-switch",
        );

        // Both seats see it; its controller is told why it cannot attack, though it cannot act.
        let mine = s.view(P1).you.units[0].clone().expect("p1's unit");
        assert_eq!(
            (mine.frozen, mine.can_act, mine.attack_reason.as_deref()),
            (Some(true), false, Some(FROZEN))
        );
        let theirs = s.view(P2).opponent.units[0].clone().expect("p1's unit, seen by p2");
        assert_eq!((theirs.frozen, theirs.attack_reason), (Some(true), None));

        s.end_turn();
        assert_eq!(thaw_of(&s, &unit), None, "it thawed at its controller's cleanup");
        assert_eq!(frost_marks(s.last_events(), &unit), vec![false]);
        assert_eq!(s.view(P2).opponent.units[0].clone().expect("p1's unit").frozen, None);
        s.end_turn();
        assert!(offers_attack(&s, &unit), "thawed, it attacks on its controller's next turn");
    }

    #[test]
    fn r1174000_frozen_on_your_turn_after_it_attacked_it_stays_frozen_through_your_next_turn() {
        let mut s = board(
            "p1",
            json!([chill.id, plain.id]),
            json!([{ "def": plain.id, "lane": 1 }]),
            json!([plain.id, plain.id]),
            json!([]),
        );
        let unit = id_at(&s, P1, 1);
        s.attack(unit.as_str(), "hero");
        s.play(chill.id.as_str(), pick(&unit));
        assert_eq!(thaw_of(&s, &unit), Some(10));

        s.end_turn();
        assert_eq!(thaw_of(&s, &unit), Some(10), "it does not thaw at the end of the turn it attacked in");
        s.end_turn();
        assert_eq!((s.state().active, s.state().turn), (P1, 11));
        assert!(!offers_attack(&s, &unit));
        assert!(!offers_switch(&s, &unit));
        s.end_turn();
        assert_eq!(thaw_of(&s, &unit), None, "it thaws at the cleanup of its controller's next turn");
        s.end_turn();
        assert!(offers_attack(&s, &unit));
    }

    #[test]
    fn r1174000_frozen_on_your_turn_before_it_acted_it_loses_this_turns_action_and_thaws_at_this_cleanup() {
        let mut s = board(
            "p1",
            json!([chill.id, plain.id]),
            json!([{ "def": plain.id, "lane": 1 }]),
            json!([plain.id, plain.id]),
            json!([]),
        );
        let unit = id_at(&s, P1, 1);
        s.play(chill.id.as_str(), pick(&unit));
        assert_eq!(thaw_of(&s, &unit), Some(9));
        assert!(!offers_attack(&s, &unit));
        assert!(!offers_switch(&s, &unit));

        s.end_turn();
        assert_eq!(thaw_of(&s, &unit), None);
        assert_eq!(frost_marks(s.last_events(), &unit), vec![false]);
        s.end_turn();
        assert!(offers_attack(&s, &unit));
    }

    #[test]
    fn r1174000_r636_a_windfury_unit_frozen_after_one_attack_loses_the_second_and_thaws_at_this_cleanup() {
        let mut s = board(
            "p1",
            json!([chill.id, plain.id]),
            json!([{ "def": windfurier.id, "lane": 1 }]),
            json!([plain.id, plain.id]),
            json!([]),
        );
        let unit = id_at(&s, P1, 1);
        s.attack(unit.as_str(), "hero");
        assert!(offers_attack(&s, &unit), "Windfury: a second attack is left");
        s.play(chill.id.as_str(), pick(&unit));
        assert_eq!(thaw_of(&s, &unit), Some(9));
        assert!(!offers_attack(&s, &unit));

        s.end_turn();
        assert_eq!(thaw_of(&s, &unit), None);
    }

    #[test]
    fn r1174000_r49_a_deft_unit_frozen_after_one_half_loses_the_other_and_thaws_at_this_cleanup() {
        let mut s = board(
            "p1",
            json!([chill.id, plain.id]),
            json!([{ "def": deft_duelist.id, "lane": 1 }]),
            json!([plain.id, plain.id]),
            json!([]),
        );
        let unit = id_at(&s, P1, 1);
        s.attack(unit.as_str(), "hero");
        assert!(offers_switch(&s, &unit), "Deft: the switch is left");
        s.play(chill.id.as_str(), pick(&unit));
        assert_eq!(thaw_of(&s, &unit), Some(9));
        assert!(!offers_switch(&s, &unit));

        s.end_turn();
        assert_eq!(thaw_of(&s, &unit), None);
    }

    #[test]
    fn r1174000_a_summoning_sick_unit_frozen_loses_its_switch_and_thaws_at_this_cleanup() {
        let mut s = board(
            "p1",
            json!([plain.id, chill.id, plain.id]),
            json!([]),
            json!([plain.id, plain.id]),
            json!([]),
        );
        s.play(plain.id.as_str(), json!({ "zone": 2 }));
        let unit = id_at(&s, P1, 2);
        assert!(offers_switch(&s, &unit), "a sick unit may still switch (§4.1)");
        s.play(chill.id.as_str(), pick(&unit));
        assert_eq!(thaw_of(&s, &unit), Some(9));
        assert!(!offers_switch(&s, &unit));

        s.end_turn();
        assert_eq!(thaw_of(&s, &unit), None);
    }

    #[test]
    fn r1174000_frozen_in_your_end_phase_it_stays_frozen_through_your_next_turn() {
        let mut s = board(
            "p1",
            json!([plain.id]),
            json!([{ "def": plain.id, "lane": 1 }, { "def": shiver.id, "lane": 2 }]),
            json!([plain.id, plain.id]),
            json!([]),
        );
        let unit = id_at(&s, P1, 1);
        s.end_turn();
        assert_eq!(thaw_of(&s, &unit), Some(10), "frozen by an end-of-turn trigger, it is not thawed by that cleanup");
        assert!(s.unit(P1, 2).is_none(), "the shiver is gone, so nothing freezes it again");
        s.end_turn();
        assert!(!offers_attack(&s, &unit));
        s.end_turn();
        assert_eq!(thaw_of(&s, &unit), None);
    }

    #[test]
    fn r1174000_a_freeze_during_a_declared_attack_does_not_stop_it_and_lasts_through_the_next_turn() {
        let mut s = board(
            "p1",
            json!([plain.id]),
            json!([{ "def": plain.id, "lane": 1 }]),
            json!([plain.id, plain.id]),
            json!([{ "def": ice_trap.id, "lane": 1 }]),
        );
        let unit = id_at(&s, P1, 1);
        s.attack(unit.as_str(), "hero");
        assert_eq!(s.state().players.p2.hero.health, HERO_HEALTH - 3, "the attack went on");
        assert_eq!(thaw_of(&s, &unit), Some(10), "it spent its exertion, so it misses its next turn");
        s.end_turn();
        s.end_turn();
        assert!(!offers_attack(&s, &unit));
        s.end_turn();
        assert_eq!(thaw_of(&s, &unit), None);
    }

    #[test]
    fn r1174000_a_freeze_on_a_frozen_unit_keeps_the_later_thaw() {
        let mut s = board(
            "p1",
            json!([chill.id, rally.id, chill.id, plain.id]),
            json!([{ "def": plain.id, "lane": 1 }]),
            json!([plain.id, plain.id]),
            json!([]),
        );
        let unit = id_at(&s, P1, 1);
        s.attack(unit.as_str(), "hero");
        s.play(chill.id.as_str(), pick(&unit));
        assert_eq!(thaw_of(&s, &unit), Some(10));
        // A fresh exertion gives it an action left, which a freeze landing now would thaw after
        // this turn; the later thaw it already has stands.
        s.play(rally.id.as_str(), pick(&unit));
        assert!(!offers_attack(&s, &unit), "a fresh exertion is no attack while Frozen");
        s.play(chill.id.as_str(), pick(&unit));
        assert_eq!(thaw_of(&s, &unit), Some(10));
        s.end_turn();
        assert_eq!(thaw_of(&s, &unit), Some(10));
    }

    #[test]
    fn r1174000_r171_a_change_of_control_keeps_the_freeze_stamped_again_for_the_new_controller() {
        let mut s = board(
            "p1",
            json!([chill.id, grab.id, plain.id]),
            json!([]),
            json!([plain.id, plain.id]),
            json!([{ "def": plain.id, "lane": 5 }]),
        );
        let unit = id_at(&s, P2, 5);
        s.play(chill.id.as_str(), pick(&unit));
        assert_eq!(thaw_of(&s, &unit), Some(10), "frozen on its controller's opponent's turn");
        s.play(grab.id.as_str(), pick(&unit));
        assert_eq!(s.card(unit.as_str()).controller, P1);
        assert_eq!(
            thaw_of(&s, &unit),
            Some(9),
            "its new controller's main phase, a fresh exertion: it loses this turn's and thaws at this cleanup"
        );
        assert!(!offers_switch(&s, &unit));
        s.end_turn();
        assert_eq!(thaw_of(&s, &unit), None);
        s.end_turn();
        assert!(offers_attack(&s, &unit));
    }

    #[test]
    fn r1174000_a_vanilla_removes_the_freeze() {
        let mut s = board(
            "p1",
            json!([chill.id, hush.id, plain.id]),
            json!([]),
            json!([plain.id, plain.id]),
            json!([{ "def": plain.id, "lane": 5 }]),
        );
        let unit = id_at(&s, P2, 5);
        s.play(chill.id.as_str(), pick(&unit));
        assert!(thaw_of(&s, &unit).is_some());
        s.play(hush.id.as_str(), pick(&unit));
        assert_eq!(thaw_of(&s, &unit), None);
        assert_eq!(frost_marks(s.last_events(), &unit), vec![false]);
    }

    #[test]
    fn r1174000_r78_a_bounced_unit_leaves_its_freeze_behind_and_comes_back_unfrozen() {
        let mut s = board(
            "p1",
            json!([chill.id, recall.id, plain.id]),
            json!([{ "def": plain.id, "lane": 1 }]),
            json!([plain.id, plain.id]),
            json!([]),
        );
        let unit = id_at(&s, P1, 1);
        s.play(chill.id.as_str(), pick(&unit));
        s.play(recall.id.as_str(), pick(&unit));
        assert_eq!(s.card(unit.as_str()).zone.z(), ZoneName::Hand);
        assert_eq!(thaw_of(&s, &unit), None);
        s.play(unit.as_str(), json!({ "zone": 3 }));
        let back = id_at(&s, P1, 3);
        assert_eq!(thaw_of(&s, &back), None);
        assert!(offers_switch(&s, &back), "back on the field it is a new arrival, free to switch");
    }

    #[test]
    fn r1174000_r20_r53_an_effects_switch_and_attack_still_happen_to_a_frozen_unit() {
        let mut s = board(
            "p1",
            json!([chill.id, shove.id, goad.id, plain.id]),
            json!([{ "def": plain.id, "lane": 1 }]),
            json!([plain.id, plain.id]),
            json!([]),
        );
        let unit = id_at(&s, P1, 1);
        s.play(chill.id.as_str(), pick(&unit));
        s.play(shove.id.as_str(), pick(&unit));
        assert_eq!(s.card(unit.as_str()).position, Some(Position::Def), "the effect's switch happened");
        s.play(goad.id.as_str(), json!({}));
        assert_eq!(
            s.state().players.p2.hero.health,
            HERO_HEALTH - 3,
            "the forced attack happened"
        );
        assert_eq!(thaw_of(&s, &unit), Some(9), "and neither spent nor thawed anything");
    }

    #[test]
    fn r1174000_heroes_are_never_frozen_and_every_enemy_unit_is() {
        let mut s = board(
            "p1",
            json!([chill.id, frost.id, plain.id]),
            json!([{ "def": plain.id, "lane": 1 }]),
            json!([plain.id, plain.id]),
            json!([{ "def": plain.id, "lane": 4 }, { "def": plain.id, "lane": 5 }]),
        );
        s.play(chill.id.as_str(), json!({ "targets": [{ "pick": "hero", "player": "p2" }] }));
        assert!(
            !s.last_events().iter().any(|event| matches!(event, GameEvent::Marked { .. })),
            "a freeze aimed at a hero does nothing"
        );
        s.play(frost.id.as_str(), json!({ "zone": 2 }));
        let enemies = [id_at(&s, P2, 4), id_at(&s, P2, 5)];
        for id in &enemies {
            assert_eq!(thaw_of(&s, id), Some(10), "{id}: every enemy Unit is Frozen");
            assert_eq!(frost_marks(s.last_events(), id), vec![true]);
        }
        let friendly = id_at(&s, P1, 1);
        assert_eq!(thaw_of(&s, &friendly), None, "a friendly Unit is not");
        let marks = s
            .last_events()
            .iter()
            .filter(|event| matches!(event, GameEvent::Marked { .. }))
            .count();
        assert_eq!(marks, enemies.len(), "and nothing else, the enemy hero included");
    }

    #[test]
    fn r1174000_immune_to_spells_stops_a_spells_freeze_and_not_a_units() {
        let mut s = board(
            "p1",
            json!([blizzard.id, frost.id, plain.id]),
            json!([]),
            json!([plain.id, plain.id]),
            json!([{ "def": plain.id, "lane": 4 }, { "def": warded.id, "lane": 5 }]),
        );
        let (exposed, immune) = (id_at(&s, P2, 4), id_at(&s, P2, 5));
        s.play(blizzard.id.as_str(), json!({}));
        assert_eq!(thaw_of(&s, &exposed), Some(10));
        assert_eq!(thaw_of(&s, &immune), None, "a Spell may not affect it");
        s.play(frost.id.as_str(), json!({ "zone": 1 }));
        assert_eq!(thaw_of(&s, &immune), Some(10), "a Unit's Cry reaches it");
    }
}
