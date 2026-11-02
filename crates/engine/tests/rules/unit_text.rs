//! Copy a Unit's text (`subsystems/unit_text.rs`, R1850000; Stinkyscape #20 Tnack Trav): the copier
//! runs the text the target runs now, on its own stats, and leaving the field ends it. Through the
//! fixtures in `fixtures/card_on_card.rs` (#2273); the real card's tests cover the same cases again.

use jackioh_engine::testkit::*;
use jackioh_engine::wire::PlayerId::P1;

use super::fixtures::card_on_card::{
    LEGEND_PING, LEGEND_RADIANT_PING, apply, at, cheap, copier, destroy, has, immutable_copier, instance,
    legend, on_field, pass, play, playing, radiant_copier, top,
};
use super::fixtures::harness::{act_ok, events_of_type, put, slot};

mod r1850000_copy_a_units_text {
    use super::*;

    #[test]
    fn r1850000_has_the_units_keywords_aura_and_triggers_on_its_own_stats() {
        let mut state = playing("copy");
        let legend = put(&mut state, &legend().id, slot(P1, Row::Units, 1), json!({}));
        let plain = put(&mut state, &cheap().id, slot(P1, Row::Units, 3), json!({}));
        let played = play(&state, P1, &copier().id, false, at("units", 2, &legend.id));
        let state = played.state;
        let me = top(&state, P1, 2).expect("the copier in lane 2");
        // Reported as a Vanilla is: an in-place `transformed`, its definition unchanged.
        assert!(played.events.iter().any(|event| matches!(
            event,
            GameEvent::Transformed { instance_id, new_instance_id, from_def_id, to_def_id, .. }
                if *instance_id == me.id && *new_instance_id == me.id && from_def_id == to_def_id
        )));
        assert_eq!(me.def_id, copier().id);
        // The Legendary's Taunt, on the copier's own 2/2 (+1 from the Legendary's aura).
        assert!(has(&state, &me.id, KeywordKind::Taunt));
        let view = unit_view(&state, &me);
        assert_eq!((view.attack, view.max_health), (3, 2));
        // Its copied aura lends +1 to its other Units, the Legendary among them.
        assert_eq!(unit_view(&state, &plain).attack, 1 + 2);
        assert_eq!(unit_view(&state, &legend).attack, 3 + 1);
        // Both "whenever you play a card" triggers answer the next play.
        let before = state.players.p2.hero.health;
        let state = play(
            &state,
            P1,
            &cheap().id,
            false,
            json!({ "zone": { "row": "units", "lane": 4 } }),
        )
        .state;
        assert_eq!(state.players.p2.hero.health, before - 2 * LEGEND_PING);
        // The copied Activate is the copier's to use.
        let before = state.players.p2.hero.health;
        let state = act_ok(
            &state,
            json!({ "type": "activate", "instanceId": me.id, "ability": "ping", "playerId": "p1" }),
        )
        .state;
        assert_eq!(state.players.p2.hero.health, before - LEGEND_PING);
        // Both end-of-turn hooks deal their number.
        let before = state.players.p2.hero.health;
        let ended = pass(&state);
        assert_eq!(ended.state.players.p2.hero.health, before - 2 * LEGEND_PING);
    }

    #[test]
    fn r1850000_a_copied_death_runs() {
        let mut state = playing("death");
        let legend = put(&mut state, &legend().id, slot(P1, Row::Units, 1), json!({}));
        let me = put(&mut state, &copier().id, slot(P1, Row::Units, 2), json!({}));
        apply(
            &mut state,
            Some(&me.id),
            P1,
            effects::copy_unit_text(json_as(json!({ "target": instance(&legend.id) }))),
        );
        let before = state.players.p2.hero.health;
        destroy(&mut state, &me.id);
        assert!(!on_field(&state, &me.id));
        assert_eq!(state.players.p2.hero.health, before - LEGEND_PING);
    }

    #[test]
    fn r1850000_the_radiant_option_copies_the_radiant_face() {
        let mut state = playing("radiant");
        let legend = put(&mut state, &legend().id, slot(P1, Row::Units, 1), json!({}));
        let played = play(
            &state,
            P1,
            &radiant_copier().id,
            false,
            at("units", 2, &legend.id),
        );
        let mut state = played.state;
        let me = top(&state, P1, 2).expect("the copier in lane 2");
        // The Radiant text's Lifesteal, on the copier's base face: it is no Radiant card.
        assert!(has(&state, &me.id, KeywordKind::Lifesteal));
        assert!(has(&state, &me.id, KeywordKind::Taunt));
        assert!(!me.radiant);
        assert_eq!(unit_view(&state, &me).max_health, 2);
        // Its Death reads the Radiant face's number.
        let before = state.players.p2.hero.health;
        destroy(&mut state, &me.id);
        assert_eq!(state.players.p2.hero.health, before - LEGEND_RADIANT_PING);
    }

    #[test]
    fn r1850000_an_immutable_copier_copies_nothing() {
        let mut state = playing("immutable");
        let legend = put(&mut state, &legend().id, slot(P1, Row::Units, 1), json!({}));
        let me = put(
            &mut state,
            &immutable_copier().id,
            slot(P1, Row::Units, 2),
            json!({}),
        );
        let events = apply(
            &mut state,
            Some(&me.id),
            P1,
            effects::copy_unit_text(json_as(json!({ "target": instance(&legend.id) }))),
        );
        assert!(events_of_type(&events, GameEventType::Transformed).is_empty());
        assert!(!has(&state, &me.id, KeywordKind::Taunt));

        // Nor does a copier copy a Vanilla Unit, which has no text, or itself.
        let other = put(&mut state, &copier().id, slot(P1, Row::Units, 3), json!({}));
        apply(
            &mut state,
            None,
            P1,
            effects::vanilla(json_as(json!({ "target": instance(&legend.id) }))),
        );
        for target in [&legend.id, &other.id] {
            let events = apply(
                &mut state,
                Some(&other.id),
                P1,
                effects::copy_unit_text(json_as(json!({ "target": instance(target) }))),
            );
            assert!(events_of_type(&events, GameEventType::Transformed).is_empty());
        }
        assert!(!has(&state, &other.id, KeywordKind::Taunt));
    }

    #[test]
    fn r1850000_leaving_the_field_ends_it() {
        let mut state = playing("leaves");
        let legend = put(&mut state, &legend().id, slot(P1, Row::Units, 1), json!({}));
        let plain = put(&mut state, &cheap().id, slot(P1, Row::Units, 3), json!({}));
        let me = put(&mut state, &copier().id, slot(P1, Row::Units, 2), json!({}));
        apply(
            &mut state,
            Some(&me.id),
            P1,
            effects::copy_unit_text(json_as(json!({ "target": instance(&legend.id) }))),
        );
        assert_eq!(unit_view(&state, &plain).attack, 3);
        apply(
            &mut state,
            None,
            P1,
            effects::bounce(json_as(json!({ "target": instance(&me.id) }))),
        );
        let held = state
            .players
            .p1
            .hand
            .iter()
            .find(|card| card.id == me.id)
            .cloned()
            .expect("the copier back in hand");
        assert!(held.memory.is_empty());
        assert!(!unit_has(&state, &held, KeywordKind::Taunt));
        // Its copied aura went with it.
        assert_eq!(unit_view(&state, &plain).attack, 2);
    }
}
