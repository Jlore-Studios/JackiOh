//! Meditative #71.1, The Cane (docs/meditative-set.md M6): the token Pareto Optimality summons.
//!
//! Both faces are `Script::default()` — the card is catalog data (M10): a 3/1 (Radiant 6/2) Field
//! Spell token (printed Mythic) with Animated on your turn, Indestructible and Pierce (Radiant: plus
//! Trample). Its attack is `subsystems::pareto`'s `cane_strike`/`cane_return` (MD-D30, R1126): from
//! its backrow zone it animates and makes one forced attack on a random enemy, then returns home
//! after that combat's state check unless it is now its controller's turn. It relies on #531's R383
//! revision for the second-turn attack (M10).

use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-071-1";

pub fn base() -> Script {
    Script::default()
}

pub fn radiant() -> Script {
    Script::default()
}

pub fn script() -> CardScripts {
    CardScripts {
        base: base(),
        radiant: radiant(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::resolve::{HookOptions, apply_effects, make_context};
    use jackioh_engine::rng::Rng;
    use jackioh_engine::script::EngineSink;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;

    const SCARAB: &str = "core-007";

    /// The Cane in p1's backrow, facing one enemy; the R383 revision this relies on is absent, so
    /// the second-turn attack it enables is not tested here (M10).
    fn cane_game() -> Scenario {
        crate::register_all();
        scenario(json!({
            "p1": { "backrow": [ID] },
            "p2": { "field": [SCARAB] },
        }))
    }

    /// Apply Pareto's pair for the standing Cane, returning the events they emitted.
    fn strike_and_return(s: &mut Scenario) -> Vec<GameEvent> {
        let cane = s.card(ID).id.clone();
        let (seed, cursor) = (s.state().seed.clone(), s.state().rng_cursor);
        let mut events: Vec<GameEvent> = Vec::new();
        let mut rng = Rng::new(&seed, cursor);
        {
            let mut sink = EngineSink::new(s.state_mut(), &mut events, &mut rng);
            let mut ctx = make_context(
                &mut sink,
                None,
                HookOptions {
                    controller: Some(P1),
                    ..HookOptions::default()
                },
            );
            apply_effects(
                &[
                    subsystems::cane_strike(json_as(json!({ "cane": cane }))),
                    subsystems::cane_return(json_as(json!({ "cane": cane }))),
                ],
                &mut ctx,
            );
        }
        s.state_mut().rng_cursor = rng.cursor();
        events
    }

    #[test]
    fn animates_your_turn_indestructible_pierce() {
        crate::register_all();
        let def = crate::card_def(ID);
        assert_eq!(def.cost, CardCost::Fixed(2));
        assert!(def.token);
        assert_eq!(
            [
                def.base.attack,
                def.base.health,
                def.radiant.attack,
                def.radiant.health
            ],
            [Some(3), Some(1), Some(6), Some(2)]
        );
        let kinds = |face: &CardFace| {
            face.keywords
                .iter()
                .map(|keyword| keyword.kind().as_str())
                .collect::<Vec<_>>()
        };
        assert_eq!(
            kinds(&def.base),
            vec!["Animated on your turn", "Indestructible", "Pierce"]
        );
        assert_eq!(
            kinds(&def.radiant),
            vec!["Animated on your turn", "Indestructible", "Pierce", "Trample"]
        );
        // And it animates on its controller's turn: after the opponent's turn it stands a Unit.
        let mut s = cane_game();
        s.end_turn();
        s.end_turn();
        assert_eq!(s.unit(P1, 1).map(|unit| unit.def_id), Some(ID.to_string()));
    }

    #[test]
    fn r1126_pareto_attack_animates_and_returns() {
        let mut s = cane_game();
        // Pareto's trigger answers the opponent's play, so the attack comes on p2's turn.
        s.end_turn();
        let events = strike_and_return(&mut s);
        let kinds: Vec<&str> = events.iter().map(|event| event.event_type().as_str()).collect();
        assert!(kinds.contains(&"animated"), "the Cane animated: {kinds:?}");
        assert!(
            events
                .iter()
                .any(|event| matches!(event, GameEvent::AttackDeclared { forced: true, .. })),
            "the Cane made a forced attack: {kinds:?}"
        );
        // After that combat's state check it is home, it not being p1's turn: no unit stands, the
        // backrow holds it.
        assert!(kinds.contains(&"deanimated"), "the Cane went home: {kinds:?}");
        assert!(s.unit(P1, 1).is_none());
        assert_eq!(s.backrow(P1, 1).map(|card| card.def_id), Some(ID.to_string()));
    }

    #[test]
    fn r1126_on_its_own_turn_it_stays_a_unit() {
        // On its controller's turn the attack happens all the same, and the Cane stays a Unit.
        let mut s = cane_game();
        let events = strike_and_return(&mut s);
        let kinds: Vec<&str> = events.iter().map(|event| event.event_type().as_str()).collect();
        assert!(kinds.contains(&"attackDeclared"), "the Cane attacked: {kinds:?}");
        assert!(
            !kinds.contains(&"deanimated"),
            "the Cane stayed a Unit: {kinds:?}"
        );
        assert_eq!(s.unit(P1, 1).map(|unit| unit.def_id), Some(ID.to_string()));
    }

    #[test]
    fn r1126_no_open_zone_no_attack() {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": {
                "field": ["core-008", "core-008", "core-008", "core-008", "core-008"],
                "backrow": [ID],
            },
            "p2": { "field": [SCARAB] },
        }));
        let events = strike_and_return(&mut s);
        let kinds: Vec<&str> = events.iter().map(|event| event.event_type().as_str()).collect();
        assert!(
            !kinds.contains(&"attackDeclared"),
            "no room, no attack: {kinds:?}"
        );
        assert_eq!(s.backrow(P1, 1).map(|card| card.def_id), Some(ID.to_string()));
    }

    const PARETO: &str = "meditative-071";
    const FILLER: &str = "core-005";

    #[test]
    fn summoned_on_your_turn_it_attacks_from_its_second_turn() {
        crate::register_all();
        let _open = preview_sets(&[SetName::Meditative]);
        let mut s = scenario(json!({
            "p1": { "mana": 10, "hand": [PARETO, FILLER], "library": [FILLER, FILLER] },
            "p2": { "hand": [FILLER], "library": [FILLER, FILLER] },
        }));
        s.play(PARETO, json!({}));
        // Animated as it entered, but summoning sick this turn.
        assert_eq!(s.unit(P1, 2).map(|unit| unit.def_id), Some(ID.to_string()));
        s.expect_refused(|s| s.attack(ID, "hero"));
        s.end_turn().end_turn();
        // Its second turn: animated again, and it may attack.
        s.attack(ID, "hero");
        s.expect_health(PlayerId::P2, 30 - 3);
    }

    #[test]
    fn indestructible_a_destroy_leaves_it_standing() {
        let mut s = cane_game();
        s.end_turn().end_turn();
        let cane = s.unit(P1, 1).expect("the animated Cane").id;
        let (seed, cursor) = (s.state().seed.clone(), s.state().rng_cursor);
        let mut events: Vec<GameEvent> = Vec::new();
        let mut rng = Rng::new(&seed, cursor);
        {
            let mut sink = EngineSink::new(s.state_mut(), &mut events, &mut rng);
            {
                let mut ctx = make_context(
                    &mut sink,
                    None,
                    HookOptions {
                        controller: Some(PlayerId::P2),
                        ..HookOptions::default()
                    },
                );
                apply_effects(
                    &[destroy(json_as(
                        json!({ "target": { "of": "instance", "instanceId": cane } }),
                    ))],
                    &mut ctx,
                );
            }
            state_check(&mut sink);
        }
        s.state_mut().rng_cursor = rng.cursor();
        assert_eq!(s.unit(P1, 1).map(|unit| unit.id), Some(cane));
    }

    #[test]
    fn pierce_its_hit_ignores_hero_armor() {
        crate::register_all();
        let _open = preview_sets(&[SetName::Meditative]);
        let mut s = scenario(json!({
            "p1": { "backrow": [ID], "hand": [FILLER], "library": [FILLER, FILLER] },
            "p2": { "hand": [FILLER], "library": [FILLER, FILLER], "armor": 3 },
        }));
        s.end_turn().end_turn();
        s.attack(ID, "hero");
        s.expect_health(PlayerId::P2, 30 - 3);
    }

    #[test]
    fn a_cane_pareto_did_not_summon_is_never_sent() {
        crate::register_all();
        // Pareto set straight onto the field ran no Cry, so it remembers no Cane; a stray Cane
        // stands beside it.
        let mut s = scenario(json!({
            "p1": { "backrow": [PARETO, ID] },
            "p2": { "mana": 10, "hand": ["core-008", "core-t-felinor"], "field": [SCARAB] },
        }));
        s.end_turn();
        s.play("core-t-felinor", json!({}));
        let kinds: Vec<&str> = s
            .last_events()
            .iter()
            .map(|event| event.event_type().as_str())
            .collect();
        assert!(
            !kinds.contains(&"attackDeclared"),
            "the stray Cane stays home: {kinds:?}"
        );
        assert_eq!(s.backrow(P1, 2).map(|card| card.def_id), Some(ID.to_string()));
    }

    #[test]
    fn the_radiant_face_is_6_2() {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": { "mana": 10, "hand": [{ "def": PARETO, "radiant": true }, FILLER] },
            "p2": { "hand": [FILLER] },
        }));
        s.play(PARETO, json!({}));
        let cane = s.unit(P1, 2).expect("the Radiant Cane");
        assert!(cane.radiant);
        s.expect_stats(cane.id.as_str(), json!({ "attack": 6, "health": 2 }));
    }

    #[test]
    fn radiant_trample() {
        crate::register_all();
        let def = crate::card_def(ID);
        assert!(
            def.radiant
                .keywords
                .iter()
                .any(|keyword| keyword.kind().as_str() == "Trample")
        );
    }
}
