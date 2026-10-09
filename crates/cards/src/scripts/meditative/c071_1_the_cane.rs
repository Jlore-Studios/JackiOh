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
    use jackioh_engine::testkit::*; // TEMP-571
    #[allow(dead_code)] const P1: PlayerId = PlayerId::P1; // TEMP-571
    #[allow(dead_code)] const P2: PlayerId = PlayerId::P2; // TEMP-571
    use jackioh_engine::resolve::{HookOptions, apply_effects, make_context};
    use jackioh_engine::rng::Rng;
    use jackioh_engine::script::EngineSink;

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
            [def.base.attack, def.base.health, def.radiant.attack, def.radiant.health],
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
        let events = strike_and_return(&mut s);
        let kinds: Vec<&str> = events.iter().map(|event| event.event_type().as_str()).collect();
        assert!(kinds.contains(&"animated"), "the Cane animated: {kinds:?}");
        assert!(kinds.contains(&"attackDeclared"), "the Cane attacked: {kinds:?}");
        // After that combat's state check it is home: no unit stands, the backrow holds it.
        assert!(s.unit(P1, 1).is_none());
        assert_eq!(s.backrow(P1, 1).map(|card| card.def_id), Some(ID.to_string()));
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
        assert!(!kinds.contains(&"attackDeclared"), "no room, no attack: {kinds:?}");
        assert_eq!(s.backrow(P1, 1).map(|card| card.def_id), Some(ID.to_string()));
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
