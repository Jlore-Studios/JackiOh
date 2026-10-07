//! A card's stay, read off the event stream (SPEC §10.3, R174, R212): `stays.ts`'s two readers.
//!
//! Port of `packages/engine/test/stays.test.ts`.

use jackioh_engine::testkit::*;

use crate::rules::fixtures::harness::new_game;

fn summoned(id: &str) -> GameEvent {
    GameEvent::Summoned {
        player: PlayerId::P1,
        instance_id: id.into(),
        def_id: "fx".into(),
        row: Row::Units,
        lane: 1,
        former_id: None,
        arrived_during: None,
        exits_from: None,
    }
}

fn destroyed(id: &str) -> GameEvent {
    GameEvent::Destroyed {
        instance_id: id.into(),
        def_id: "fx".into(),
        owner: PlayerId::P1,
        controller: PlayerId::P1,
        attack: 1,
        max_health: 1,
        killer_id: None,
        radiant: None,
    }
}

fn damage(id: &str) -> GameEvent {
    GameEvent::Damage {
        source_id: None,
        target_id: id.into(),
        amount: 1,
        combat: false,
    }
}

fn stolen(id: &str, controller: PlayerId) -> GameEvent {
    GameEvent::ControlChanged {
        instance_id: id.into(),
        controller,
        row: Row::Units,
        lane: 1,
        former_id: None,
    }
}

/// `[...later.moved].sort()`.
fn sorted_moved(later: &LaterMoves) -> Vec<String> {
    let mut moved: Vec<String> = later.moved.iter().cloned().collect();
    moved.sort();
    moved
}

mod stays_left_field_since_r174 {
    use super::*;

    #[test]
    fn r174_finds_a_card_that_left_the_field_after_a_point_and_nothing_before_it() {
        let events = [destroyed("c1"), damage("c2"), destroyed("c2"), summoned("c2")];
        assert!(left_field_since(&events, 1, "c2"));
        assert!(!left_field_since(&events, 1, "c1"));
        assert!(left_field_since(&events, 0, "c1"));
    }
}

mod stays_moves_in_r212 {
    use super::*;

    #[test]
    fn r212_names_every_card_a_zone_change_moved_and_a_reborn_body_among_them() {
        let events = [destroyed("c1"), summoned("c1"), damage("c2")];
        let later = moves_in(&events, None);
        assert_eq!(sorted_moved(&later), vec!["c1".to_string()]);
    }

    #[test]
    fn r212_moves_both_sides_of_a_replace_and_only_the_ingredients_a_fuse_used_up() {
        let events = [
            GameEvent::Transformed {
                instance_id: "c1".into(),
                from_def_id: "a".into(),
                to_def_id: "b".into(),
                new_instance_id: "c9".into(),
                hidden_from: None,
            },
            GameEvent::Fused {
                instance_ids: vec!["c2".into(), "c3".into()],
                result_instance_id: "c3".into(),
                def_id: "t-1:a+b".into(),
            },
        ];
        let later = moves_in(&events, None);
        assert_eq!(
            sorted_moved(&later),
            vec!["c1".to_string(), "c2".to_string(), "c9".to_string()]
        );
    }

    #[test]
    fn r212_r171_reads_the_controller_a_card_had_before_the_first_change_of_control_and_a_change_of_control_is_no_move()
     {
        let events = [
            stolen("c1", PlayerId::P2),
            stolen("c1", PlayerId::P1),
            stolen("c2", PlayerId::P1),
        ];
        let later = moves_in(&events, None);
        assert_eq!(later.controller_before.get("c1"), Some(&PlayerId::P1));
        assert_eq!(later.controller_before.get("c2"), Some(&PlayerId::P2));
        assert_eq!(later.moved.len(), 0);
    }
}

mod stays_a_stack_note_belongs_to_one_removal_r212_s3_2 {
    use super::*;

    fn exiled(id: &str) -> GameEvent {
        GameEvent::Exiled {
            instance_id: id.into(),
            def_id: "fx".into(),
            owner: PlayerId::P1,
        }
    }

    #[test]
    fn r212_a_removals_note_is_read_off_its_reports_until_the_card_that_left_has_moved_again_and_the_loop_has_dispatched_them()
     {
        let mut state = new_game("stays-uncovered", None);
        note_uncovered(&mut state, "c1", Some("c4"));
        // Still owed: the death that uncovered c4, and anything owed behind it, read as its resume.
        assert_eq!(uncovered_by(&state, &destroyed("c1")), vec!["c4".to_string()]);
        assert!(
            moves_in(&[damage("c9"), destroyed("c1")], Some(&state))
                .moved
                .contains("c4")
        );
        // An event that reports no removal of c1 is no report of it.
        note_reported(&mut state, &damage("c1"));
        // The loop dispatched the death; the death's other reports still read it while c1 lies where it fell.
        note_reported(&mut state, &destroyed("c1"));
        assert_eq!(uncovered_by(&state, &destroyed("c1")), vec!["c4".to_string()]);

        // c1 moves on out of its graveyard: that move is no removal from a pile's top.
        note_moved(&mut state, "c1");
        assert_eq!(uncovered_by(&state, &exiled("c1")), Vec::<String>::new());
        assert!(
            !moves_in(&[damage("c9"), exiled("c1")], Some(&state))
                .moved
                .contains("c4")
        );
    }

    #[test]
    fn r212_a_card_that_moves_on_before_the_loop_reaches_its_removals_report_keeps_the_note_for_that_report_alone()
     {
        let mut state = new_game("stays-uncovered-moved", None);
        note_uncovered(&mut state, "c1", Some("c4"));
        note_moved(&mut state, "c1");
        // The death is still owed, and so is everything before it: c4 resumed after all of that.
        assert!(
            moves_in(&[damage("c9"), destroyed("c1"), exiled("c1")], Some(&state))
                .moved
                .contains("c4")
        );
        note_reported(&mut state, &destroyed("c1"));
        assert_eq!(uncovered_by(&state, &exiled("c1")), Vec::<String>::new());
    }

    #[test]
    fn r212_a_removal_that_uncovers_nothing_still_ends_the_note_of_an_earlier_one_once_that_one_is_reported()
    {
        let mut state = new_game("stays-uncovered-again", None);
        note_uncovered(&mut state, "c1", Some("c4"));
        note_reported(&mut state, &stolen("c1", PlayerId::P2));
        // Stolen off the top of its pile, and later killed alone on the thief's side.
        note_uncovered(&mut state, "c1", None);
        assert_eq!(uncovered_by(&state, &destroyed("c1")), Vec::<String>::new());
    }
}
