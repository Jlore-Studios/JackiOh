//! Meditative #72, The Banisher (docs/meditative-set.md M6): a 2/1 Unit (Radiant 4/2 with Cleave)
//! whose damage exiles.
//!
//! Whenever this damages a Unit, that Unit is marked, and the next state check exiles it ahead of
//! deaths — no Death, no Reborn, no `destroyed` (MD-D31, R1124). Both faces carry
//! `StaticFlags.exiles_on_damage`; the Radiant face's Cleave is catalog data. A hit the pipeline
//! stopped (Divine Shield, Indestructible, the zero rule) dealt no damage, so it exiles nothing.

use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-072";

fn the_banisher() -> Script {
    Script {
        static_flags: Some(StaticFlags {
            exiles_on_damage: Some(true),
            ..StaticFlags::default()
        }),
        ..Script::default()
    }
}

pub fn base() -> Script {
    the_banisher()
}

pub fn radiant() -> Script {
    the_banisher()
}

pub fn script() -> CardScripts {
    CardScripts {
        base: the_banisher(),
        radiant: the_banisher(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*; // TEMP-571
    #[allow(dead_code)] const P1: PlayerId = PlayerId::P1; // TEMP-571
    #[allow(dead_code)] const P2: PlayerId = PlayerId::P2; // TEMP-571

    /// Banisher 2/1s facing tough enemies.
    const BIG: &str = "core-043";
    const MENACE: &str = "core-019";

    fn banisher_game() -> Scenario {
        crate::register_all();
        scenario(json!({
            "p1": { "field": [ID, ID] },
            "p2": { "field": [BIG, MENACE] },
        }))
    }

    #[test]
    fn exiles_both_ways() {
        let mut s = banisher_game();
        // Ours hits their big Felinor: marked, then exiled ahead of deaths.
        let banisher = s.unit(P1, 1).unwrap().id;
        let foe = s.unit(P2, 1).unwrap().id;
        s.attack(&banisher, &foe);
        s.expect_events(json!(["exiled"]));
        s.expect_in_zone(&foe, "exile");
        let seen: Vec<&str> = s.events().iter().map(|event| event.event_type().as_str()).collect();
        assert!(!seen.contains(&"destroyed"), "no death collected it: {seen:?}");
        // Their Menace hits our second Banisher: the mark works facing either way.
        s.end_turn();
        let menace = s.unit(P2, 2).unwrap().id;
        let ours = s.unit(P1, 2).unwrap().id;
        s.attack(&menace, &ours);
        s.expect_in_zone(&ours, "exile");
    }

    #[test]
    fn shield_or_indestructible_not_exiled() {
        // A hit Divine Shield stops, and one an Indestructible takes, deal no damage — so neither
        // is marked, and both stay on the field.
        crate::register_all();
        let mut s = scenario(json!({
            "p1": { "field": [ID, ID] },
            "p2": { "field": ["classic-089", "classic-041"] },
        }));
        let first = s.unit(P1, 1).unwrap().id;
        let ghost = s.unit(P2, 1).unwrap().id;
        s.attack(&first, &ghost);
        s.expect_in_zone(&ghost, "field");
        let second = s.unit(P1, 2).unwrap().id;
        let state = s.unit(P2, 2).unwrap().id;
        s.attack(&second, &state);
        s.expect_in_zone(&state, "field");
    }

    #[test]
    fn radiant_cleave_exiles() {
        // The Radiant face's Cleave carries the mark to the neighbours it damages.
        crate::register_all();
        let mut s = scenario(json!({
            "p1": { "field": [{ "def": ID, "radiant": true }] },
            "p2": { "field": ["core-007", "core-007", "core-007"] },
        }));
        let banisher = s.unit(P1, 1).unwrap().id;
        let middle = s.unit(P2, 2).unwrap().id;
        s.attack(&banisher, &middle);
        let exiled: Vec<String> = s.events().iter().filter_map(|event| match event {
            GameEvent::Exiled { instance_id, .. } => Some(instance_id.clone()),
            _ => None,
        }).collect();
        assert_eq!(exiled.len(), 3, "the hit and both cleaves exiled: {exiled:?}");
    }
}
