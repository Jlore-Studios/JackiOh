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
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    /// Banisher 2/1s facing tough enemies.
    const BIG: &str = "core-043";
    const MENACE: &str = "core-019";

    fn banisher_game() -> Scenario {
        crate::register_all();
        scenario(json!({
            "p1": { "field": [ID, ID] },
            "p2": { "field": [MENACE, BIG] },
        }))
    }

    /// The ids the last step's `exiled` and `destroyed` events name.
    fn exiled_and_destroyed(s: &Scenario) -> (Vec<String>, Vec<String>) {
        let mut exiled = Vec::new();
        let mut destroyed = Vec::new();
        for event in s.last_events() {
            match event {
                GameEvent::Exiled { instance_id, .. } => exiled.push(instance_id.clone()),
                GameEvent::Destroyed { instance_id, .. } => destroyed.push(instance_id.clone()),
                _ => {}
            }
        }
        (exiled, destroyed)
    }

    #[test]
    fn exiles_both_ways() {
        let mut s = banisher_game();
        // Ours attacks their Taunt, the 9/9 Menace: its 2 marks it, and the check exiles it ahead of
        // deaths, so no death collects it. The Banisher itself dies to the strike back as usual.
        let banisher = s.unit(P1, 1).unwrap().id;
        let menace = s.unit(P2, 1).unwrap().id;
        s.attack(&banisher, &menace);
        assert_eq!(
            exiled_and_destroyed(&s),
            (vec![menace.clone()], vec![banisher.clone()])
        );
        s.expect_in_zone(&menace, "exile");
        s.expect_in_zone(&banisher, "graveyard");
        // Their big Felinor attacks our second Banisher: its strike back marks the attacker, so the
        // exile works facing either way, and our Banisher dies as usual.
        s.end_turn();
        let big = s.unit(P2, 2).unwrap().id;
        let ours = s.unit(P1, 2).unwrap().id;
        s.attack(&big, &ours);
        assert_eq!(exiled_and_destroyed(&s), (vec![big.clone()], vec![ours.clone()]));
        s.expect_in_zone(&big, "exile");
        s.expect_in_zone(&ours, "graveyard");
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
    fn r1124_an_exiled_unit_has_no_death_and_no_destroyed() {
        // Radiant Saintess: "Death: Make your other Units Radiant." Exiled, it never dies.
        crate::register_all();
        let mut s = scenario(json!({
            "p1": { "field": [ID], "hand": ["core-005"] },
            "p2": { "field": ["core-081", "core-008"], "hand": ["core-005"] },
        }));
        let banisher = s.unit(P1, 1).unwrap().id;
        let saintess = s.unit(P2, 1).unwrap().id;
        s.attack(&banisher, &saintess);
        let (exiled, destroyed) = exiled_and_destroyed(&s);
        assert_eq!(exiled, vec![saintess.clone()]);
        assert!(
            !destroyed.contains(&saintess),
            "no `destroyed` for the exiled Unit"
        );
        s.expect_in_zone(&saintess, "exile");
        assert!(!s.unit(P2, 2).unwrap().radiant, "its Death never ran");
    }

    #[test]
    fn r1124_an_exiled_reborn_unit_does_not_come_back() {
        // Right-house defender (1/1 Divine Shield, Reborn): the first Banisher pops the Shield, the
        // second exiles it, and Reborn never brings it back.
        crate::register_all();
        let mut s = scenario(json!({
            "p1": { "field": [ID, ID], "hand": ["core-005"] },
            "p2": { "field": ["core-003"], "hand": ["core-005"] },
        }));
        let defender = s.unit(P2, 1).unwrap().id;
        let first = s.unit(P1, 1).unwrap().id;
        s.attack(&first, &defender);
        s.expect_in_zone(&defender, "field");
        let second = s.unit(P1, 2).unwrap().id;
        s.attack(&second, &defender);
        s.expect_in_zone(&defender, "exile");
        assert!(s.unit(P2, 1).is_none(), "Reborn summoned nothing back");
        assert!(
            s.state().players[P2]
                .units
                .iter()
                .flatten()
                .flatten()
                .all(|card| card.def_id != "core-003"),
            "no Reborn copy anywhere on the field"
        );
    }

    #[test]
    fn r1124_an_exiled_token_ceases_to_exist() {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": { "field": [ID], "hand": ["core-005"] },
            "p2": { "field": ["core-t-rush"], "hand": ["core-005"] },
        }));
        let banisher = s.unit(P1, 1).unwrap().id;
        let token = s.unit(P2, 1).unwrap().id;
        s.attack(&banisher, &token);
        s.expect_in_zone(&token, "gone");
        assert!(!exiled_and_destroyed(&s).1.contains(&token));
    }

    #[test]
    fn zero_damage_exiles_nothing() {
        // The 7/7 with Armor 7 takes 2 − 7 = 0 damage: no mark, no exile.
        crate::register_all();
        let mut s = scenario(json!({
            "p1": { "field": [ID], "hand": ["core-005"] },
            "p2": { "field": ["core-025"], "hand": ["core-005"] },
        }));
        let banisher = s.unit(P1, 1).unwrap().id;
        let armored = s.unit(P2, 1).unwrap().id;
        s.attack(&banisher, &armored);
        s.expect_in_zone(&armored, "field");
        assert_eq!(s.card(&armored).damage, 0);
        assert!(exiled_and_destroyed(&s).0.is_empty());
    }

    #[test]
    fn r1124_a_unit_hit_for_lethal_is_exiled_not_destroyed() {
        // Its 2 on a 1/1 Scarab is lethal: the exile comes ahead of the death.
        crate::register_all();
        let mut s = scenario(json!({
            "p1": { "field": [ID], "hand": ["core-005"] },
            "p2": { "field": ["core-007"], "hand": ["core-005"] },
        }));
        let banisher = s.unit(P1, 1).unwrap().id;
        let scarab = s.unit(P2, 1).unwrap().id;
        s.attack(&banisher, &scarab);
        let (exiled, destroyed) = exiled_and_destroyed(&s);
        assert_eq!(exiled, vec![scarab.clone()]);
        assert!(!destroyed.contains(&scarab));
        s.expect_in_zone(&scarab, "exile");
    }

    #[test]
    fn its_own_death_still_happens() {
        // The 9/9 strikes back for 9: the Banisher dies as usual, `destroyed` and graveyard.
        crate::register_all();
        let mut s = scenario(json!({
            "p1": { "field": [ID], "hand": ["core-005"] },
            "p2": { "field": [MENACE], "hand": ["core-005"] },
        }));
        let banisher = s.unit(P1, 1).unwrap().id;
        let menace = s.unit(P2, 1).unwrap().id;
        s.attack(&banisher, &menace);
        let (exiled, destroyed) = exiled_and_destroyed(&s);
        assert!(destroyed.contains(&banisher));
        assert!(!exiled.contains(&banisher), "it never exiles itself");
        s.expect_in_zone(&banisher, "graveyard");
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
        let exiled: Vec<String> = s
            .events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::Exiled { instance_id, .. } => Some(instance_id.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(exiled.len(), 3, "the hit and both cleaves exiled: {exiled:?}");
    }
}
