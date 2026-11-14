// R3821000 (issue #3821): the view says why the viewer's own unit cannot make its own switch of
// position now (`UnitView.switchReason`, §4.1, §10.8), in the words `combat::why_cannot_switch_in`
// refuses with, in its controller's main phase whether or not the unit may still act, so a unit
// that has attacked says it has already acted. The web shows it and works none of it out (CLAUDE.md
// rule 7); the golden view text leaves it out, so no recorded hash moves.

mod r3821000_the_view_says_why_your_own_unit_cannot_switch {
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use crate::rules::fixtures::combat::{indestructible, plain};
    use crate::rules::fixtures::harness::{json_of, setup_catalog};

    const NO_DEFENSE: &str = "that unit cannot be in Defense Position";
    const ALREADY_ACTED: &str = "that unit has already acted this turn";

    /// Each side has an Indestructible unit in lane 1 and a plain one in lane 5, both from an earlier
    /// turn and in Attack Position. It is p1's main phase.
    fn board(seed: &str) -> Scenario {
        setup_catalog();
        let field = json!([
            { "def": indestructible.id, "lane": 1 },
            { "def": plain.id, "lane": 5 },
        ]);
        scenario(json!({
            "seed": seed,
            "p1": { "hand": [plain.id], "field": field, "library": [plain.id, plain.id] },
            "p2": { "hand": [plain.id], "field": field, "library": [plain.id, plain.id] },
        }))
    }

    fn id_at(s: &Scenario, player: PlayerId, lane: i32) -> String {
        s.unit(player, lane)
            .unwrap_or_else(|| panic!("no unit at {player:?}'s lane {lane}"))
            .id
    }

    /// The unit as `viewer` reads it, on either side of the board.
    fn unit_of(s: &Scenario, viewer: PlayerId, id: &str) -> wire::UnitView {
        let view = s.view(viewer);
        view.you
            .units
            .iter()
            .chain(view.opponent.units.iter())
            .flatten()
            .find(|unit| unit.instance_id == id)
            .cloned()
            .unwrap_or_else(|| panic!("{viewer:?} sees no unit {id}"))
    }

    fn switch_reason(s: &Scenario, viewer: PlayerId, id: &str) -> Option<String> {
        unit_of(s, viewer, id).switch_reason
    }

    /// Whether `legal_actions` offers `player` the unit's own switch.
    fn switch_offered(s: &Scenario, player: PlayerId, id: &str) -> bool {
        s.offered(player)
            .iter()
            .any(|action| matches!(action, ActionBody::SwitchPosition { instance_id } if instance_id == id))
    }

    #[test]
    fn r3821000_an_indestructible_unit_in_attack_position_says_it_cannot_be_in_defense_position() {
        let s = board("r3821000-indestructible");
        let wall = id_at(&s, P1, 1);
        assert!(
            !switch_offered(&s, P1, &wall),
            "PREMISE: an Indestructible unit's switch to Defense Position is not offered (R2179000)"
        );

        let mine = unit_of(&s, P1, &wall);
        assert_eq!(mine.switch_reason.as_deref(), Some(NO_DEFENSE));
        assert_eq!(mine.attack_reason, None, "it may still attack");
        assert_eq!(switch_reason(&s, P2, &wall), None);
    }

    #[test]
    fn r3821000_a_unit_that_has_attacked_says_it_has_already_acted() {
        let mut s = board("r3821000-acted");
        let attacker = id_at(&s, P1, 5);
        s.attack(attacker.as_str(), "hero");
        assert!(
            !switch_offered(&s, P1, &attacker),
            "PREMISE: its one exertion is spent (§4.1)"
        );

        let mine = unit_of(&s, P1, &attacker);
        assert_eq!(mine.switch_reason.as_deref(), Some(ALREADY_ACTED));
        assert!(!mine.can_act, "PREMISE: a spent unit may not act");
        assert_eq!(
            mine.attack_reason, None,
            "R849000 gives a spent unit no attack reason"
        );
        assert_eq!(switch_reason(&s, P2, &attacker), None);
    }

    #[test]
    fn r3821000_a_unit_free_to_switch_the_opponents_units_and_the_opponents_turn_carry_none() {
        let mut s = board("r3821000-none");
        let free = id_at(&s, P1, 5);
        let my_wall = id_at(&s, P1, 1);
        let their_wall = id_at(&s, P2, 1);
        assert!(
            switch_offered(&s, P1, &free),
            "PREMISE: the plain unit may switch"
        );

        assert_eq!(switch_reason(&s, P1, &free), None);
        // p1's turn: p2's Indestructible unit would be refused, and p1 is told nothing of it, nor is
        // p2, whose turn it is not.
        assert_eq!(switch_reason(&s, P1, &their_wall), None);
        assert_eq!(switch_reason(&s, P2, &their_wall), None);

        // p2's turn: p2 reads the refusal on its own unit, and p1 reads none on either side.
        s.end_turn();
        assert_eq!(switch_reason(&s, P2, &their_wall).as_deref(), Some(NO_DEFENSE));
        assert_eq!(switch_reason(&s, P1, &my_wall), None);
        assert_eq!(switch_reason(&s, P1, &their_wall), None);
    }

    #[test]
    fn r3821000_the_golden_view_text_leaves_it_out() {
        let s = board("r3821000-golden");
        assert!(
            json_of(s.view(P1)).to_string().contains("\"switchReason\""),
            "PREMISE: p1's view carries a switch reason"
        );
        for text in jackioh_engine::testkit::golden::view_texts(s.state()) {
            assert!(!text.contains("switchReason"), "{text}");
        }
    }
}
