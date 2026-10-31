//! SPEC §8.10 Stinkyscape 175 Frost Module, a card of the Stinkyscape set.
//! (1) Unit, Common, 2/1 → 4/2: Starship Piece, Equip.
//! "Activate: Freeze an enemy Unit." / Radiant: "Activate 2: Freeze an enemy Unit." (R1860009).
//!
//! Engine: Equip (R1086): played onto one of your Units it fuses into it, the host keeping its zone
//! and taking its stats, keywords and text (R77, R102), Starship Piece only into a piece
//! (R1852001); played alone its death fuses a copy into its owner's Dormant Starship (R1852000);
//! an equipped host takes the Activate. Activate (R384; Radiant Activate 2, R1751000): `freeze`

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-175";

/// §6.3, R384: once a turn on the base face, twice on the Radiant one (R1751000).
const BASE_USES: i32 = 1;
const RADIANT_USES: i32 = 2;

/// "an enemy Unit": enemy Units only, never a hero (R1860009).
fn targets() -> Vec<TargetDecl> {
    vec![TargetDecl::target(1, 1, json!({ "side": "enemy", "of": ["unit"] }))]
}

fn frost(uses: i32) -> Script {
    Script {
        activations: vec![ActivationDecl {
            id: "freeze".to_string(),
            label: "Freeze an enemy Unit".to_string(),
            uses: ActivationUses::Count(uses),
            cost: None,
            targets: targets(),
            modes: vec![],
            can_activate: None,
            has: None,
            run: hook(|_ctx| {
                vec![freeze(json_as(json!({
                    "target": { "of": "chosen" },
                })))]
            }),
        }],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: frost(BASE_USES),
        radiant: frost(RADIANT_USES),
    }
}

// Stinkyscape #175 Frost Module — SPEC §8.10 row 175, BUILD M12 row S 175 (R1174000).
#[cfg(test)]
mod tests {
    use super::{BASE_USES, ID, RADIANT_USES};
    use jackioh_engine::testkit::*;

    use crate::scenario;
    use crate::filler;

    const FROST: &str = "stinkyscape-175";
    const VANILLA: &str = "core-008"; // (1) Unit 4/4.
    const FILLER: &str = "core-005"; // (1) Spell.

    mod base {
        use super::*;

        #[test]
        fn declares_one_activate_once_a_turn_over_enemy_units() {
            assert_eq!(crate::card_def(ID).id, FROST);
            let scripts = super::super::script();
            for face in [&scripts.base, &scripts.radiant] {
                assert_eq!(face.activations.len(), 1);
            }
            assert_eq!(
                scripts.base.activations.first().map(|decl| decl.uses),
                Some(ActivationUses::Count(BASE_USES))
            );
            assert_eq!(
                scripts.radiant.activations.first().map(|decl| decl.uses),
                Some(ActivationUses::Count(RADIANT_USES))
            );
        }

        #[test]
        fn r1860009_activate_freezes_a_chosen_enemy_unit() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [FILLER], "field": [FROST], "library": filler(4) },
                "p2": { "hand": [FILLER], "field": [VANILLA], "library": filler(4) },
            }));
            let enemy = s.unit(PlayerId::P2, 1).expect("enemy").clone();
            assert!(s.card(&enemy).frozen.is_none());

            s.activate(FROST, json!({ "targets": pick::instance(&enemy) }));

            assert!(s.card(&enemy).frozen.is_some(), "the enemy Unit is Frozen");
        }

        #[test]
        fn r1860009_no_hero_may_be_chosen_once_a_turn() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [FILLER], "field": [FROST], "library": filler(4) },
                "p2": { "hand": [FILLER], "field": [VANILLA], "library": filler(4) },
            }));
            s.expect_refused(|s| {
                s.activate(FROST, json!({ "targets": json!([{ "pick": "hero", "player": "p2" }]) }))
            });
            let enemy = s.unit(PlayerId::P2, 1).expect("enemy").clone();
            s.activate(FROST, json!({ "targets": pick::instance(&enemy) }));
            s.expect_refused(|s| s.activate(FROST, json!({ "targets": pick::instance(&enemy) })));
        }

        #[test]
        fn r1086_equipped_onto_a_unit_the_host_takes_the_activate() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [VANILLA, FROST, FILLER], "library": filler(4) },
                "p2": { "hand": [FILLER], "field": [VANILLA], "library": filler(4) },
            }));
            s.play(VANILLA, json!({ "zone": 1 }));
            s.play(FROST, json!({ "zone": 1, "equip": true }));

            let host = s.unit(PlayerId::P1, 1).expect("the fused host").clone();
            let enemy = s.unit(PlayerId::P2, 1).expect("enemy").clone();
            s.activate(&host, json!({ "targets": pick::instance(&enemy) }));

            assert!(s.card(&enemy).frozen.is_some(), "the host Froze the enemy Unit");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r1860009_radiant_activate_twice_a_turn() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [FILLER], "field": [{ "def": FROST, "radiant": true }], "library": filler(4) },
                "p2": {
                    "hand": [FILLER],
                    "field": [{ "def": VANILLA, "lane": 1 }, { "def": VANILLA, "lane": 2 }],
                    "library": filler(4),
                },
            }));
            let first = s.unit(PlayerId::P2, 1).expect("first").clone();
            let second = s.unit(PlayerId::P2, 2).expect("second").clone();
            s.activate(FROST, json!({ "targets": pick::instance(&first) }));
            s.activate(FROST, json!({ "targets": pick::instance(&second) }));

            assert!(s.card(&first).frozen.is_some());
            assert!(s.card(&second).frozen.is_some());
            s.expect_refused(|s| s.activate(FROST, json!({ "targets": pick::instance(&first) })));
        }
    }
}
