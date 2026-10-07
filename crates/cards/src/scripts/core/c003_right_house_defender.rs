//! SPEC §8.1 #3 Right-house defender — 1/1 → 2/2 Unit, Human, cost 1.
//! Base: "Divine Shield, Reborn" — both are printed on both faces in the catalog and the layer
//! system applies them (§10.4), so the base script has nothing to do. Never re-grant a printed
//! keyword from a script.
//!
//! Radiant: "Divine Shield, Reborn; Death: summon a base Right-house defender". The radiant cell
//! keeps the base keywords and adds the Death clause (§8 Conventions).
//!
//! Engine cell: Death fires on both deaths of a Reborn unit (§4.5's ruling, R8) — the Reborn death
//! and the reborn body's death — and the reborn body keeps the radiant flag (R78), so the radiant
//! form summons twice in total. What it summons is a NEW base Right-house defender rather than a
//! copy: `summon` without `radiant: true` creates the base face, whose script is `base` above and
//! has no Death hook, so the chain ends there. Placement is R64's leftmost free zone, and the dying
//! unit's own zone is reserved for its Reborn return, so the summon lands beside it, not in it.

use jackioh_engine::effects::summon;
use jackioh_engine::prelude::*;

pub const ID: &str = "core-003";

pub fn script() -> CardScripts {
    /* Printed keywords only (§10.4); an empty Script is the whole base card. */
    let base = Script::default();
    let radiant = Script {
        death: Some(hook(|_ctx| vec![summon(json_as(json!({ "defId": "core-003", "player": "self" })))])),
        ..Script::default()
    };
    CardScripts { base, radiant }
}

// SPEC §8.1 #3 Right-house defender. BUILD M4-T4 row 3: "Shield eats first hit; dies → returns at
// 1 without Reborn; radiant Death summons a base Right-house defender on both deaths while Reborn
// keeps its zone (R8, R64)".
//
// Every death here is a combat death, so each one needs its own attacker: #8 Mr. Vanilla (3/3,
// Immutable, no hooks, 3 attack) survives the 1- or 2-point retaliation, so four of them on p2's
// board give four clean hits in one turn.
//
// Divine Shield absorbs a whole hit (§4.4 step 1), so each death costs two hits: one to burn the
// shield, one to kill. The reborn body's shield is back — §4.5 step 4 returns it "as a reset
// instance (R78)", and R78's reset is what clears `divineShieldSpent` — while Reborn itself does
// not come back (the state check marks it used). See the report: R78's enumerated list does not
// name Divine Shield, so this reading deserves its own ruling row.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    /// p2's attacker in `lane`, as an instance id: a string reference survives every later step.
    fn foe(s: &Scenario, lane: i32) -> String {
        match s.unit(PlayerId::P2, lane) {
            Some(unit) => unit.id.clone(),
            None => panic!("the test needs a p2 unit in lane {lane}"),
        }
    }

    const FOUR_VANILLAS: [&str; 4] = ["core-008", "core-008", "core-008", "core-008"];

    mod n3_right_house_defender {
        use super::*;

        /// Taunt on BOTH faces, added by the Core Set balance pass (issue #1, "it just makes sense").
        /// A 1-mana Divine Shield + Reborn body that could be walked past was a defender that did not
        /// defend; with Taunt the enemy has to spend the shield before anything behind it is reachable.
        #[test]
        fn s6_1_taunt_is_printed_on_both_faces_so_the_enemy_must_come_through_it() {
            crate::register_all();
            let mut s = scenario(json!({
                "active": "p2",
                "p1": { "field": [{ "def": "core-003", "lane": 1 }, { "def": "core-008", "lane": 2 }] },
                "p2": { "field": ["core-008"] }
            }));

            assert!(keywords_of(s.state(), s.card("core-003")).contains(&Keyword::Taunt));

            // The ally behind it cannot be reached while the Taunt stands (§4.2).
            let ally = s.unit(PlayerId::P1, 2);
            assert!(ally.is_some());
            let attacker = foe(&s, 1);
            match ally {
                Some(ally) => s.expect_refused_with(|s| s.attack(attacker.as_str(), ally), "Taunt"),
                None => s.expect_refused_with(|s| s.attack(attacker.as_str(), "core-008"), "Taunt"),
            };
        }

        #[test]
        fn s6_1_the_radiant_face_keeps_taunt_too() {
            crate::register_all();
            let s = scenario(json!({ "p1": { "field": [{ "def": "core-003", "radiant": true }] } }));

            assert_eq!(s.unit(PlayerId::P1, 1).map(|unit| unit.radiant), Some(true));
            assert!(keywords_of(s.state(), s.card("core-003")).contains(&Keyword::Taunt));
        }

        #[test]
        fn the_divine_shield_eats_the_first_hit() {
            crate::register_all();
            let mut s = scenario(json!({
                "active": "p2",
                "p1": { "field": ["core-003"] },
                "p2": { "field": ["core-008"] }
            }));
            let attacker = foe(&s, 1);
            s.attack(attacker.as_str(), "core-003");
            s.expect_in_zone("core-003", "field")
                .expect_stats("core-003", json!({ "health": 1, "maxHealth": 1 }));
        }

        #[test]
        fn r64_r83_dies_returns_at_1_health_in_its_own_reserved_zone() {
            crate::register_all();
            let mut s = scenario(json!({
                "active": "p2",
                "p1": { "field": [{ "def": "core-003", "lane": 3 }] },
                "p2": { "field": FOUR_VANILLAS }
            }));
            let rhd = s.card("core-003").id.clone();

            let first = foe(&s, 1);
            s.attack(first.as_str(), rhd.as_str()); // burns the shield
            let second = foe(&s, 2);
            s.attack(second.as_str(), rhd.as_str()); // kills it; Reborn brings it straight back

            s.expect_in_zone(rhd.as_str(), "field")
                .expect_stats(rhd.as_str(), json!({ "health": 1, "maxHealth": 1 }));
            assert_eq!(s.unit(PlayerId::P1, 3).map(|unit| unit.id), Some(rhd.clone()));
            assert!(s.unit(PlayerId::P1, 1).is_none());
        }

        #[test]
        fn r78_the_reborn_body_has_no_reborn_so_its_next_death_is_final() {
            crate::register_all();
            let mut s = scenario(json!({
                "active": "p2",
                "p1": { "field": [{ "def": "core-003", "lane": 3 }] },
                "p2": { "field": FOUR_VANILLAS }
            }));
            let rhd = s.card("core-003").id.clone();

            let first = foe(&s, 1);
            s.attack(first.as_str(), rhd.as_str());
            let second = foe(&s, 2);
            s.attack(second.as_str(), rhd.as_str()); // first death: Reborn returns it at 1 health
            let third = foe(&s, 3);
            s.attack(third.as_str(), rhd.as_str()); // the returned body's shield takes this one
            let fourth = foe(&s, 4);
            s.attack(fourth.as_str(), rhd.as_str()); // second death: nothing brings it back

            s.expect_in_zone(rhd.as_str(), "graveyard");
            assert!(s.unit(PlayerId::P1, 3).is_none());
        }

        #[test]
        fn r8_r64_radiant_death_summons_a_base_right_house_defender_beside_the_reserved_zone() {
            crate::register_all();
            let mut s = scenario(json!({
                "active": "p2",
                "p1": { "field": [{ "def": "core-003", "radiant": true, "lane": 1 }] },
                "p2": { "field": FOUR_VANILLAS }
            }));
            let rhd = s.card("core-003").id.clone();

            let first = foe(&s, 1);
            s.attack(first.as_str(), rhd.as_str()); // burns the shield on the 2/2
            let second = foe(&s, 2);
            s.attack(second.as_str(), rhd.as_str()); // first death → Death fires

            // Lane 1 is reserved for the Reborn return (R64), so the summon takes lane 2, and what it
            // summons is a new BASE Right-house defender, not a copy of the radiant one.
            let reborn = s.unit(PlayerId::P1, 1);
            let summoned = match s.unit(PlayerId::P1, 2) {
                Some(unit) => unit,
                None => panic!("the radiant Death trigger summoned nothing in lane 2"),
            };
            assert_eq!(reborn.as_ref().map(|unit| unit.id.clone()), Some(rhd.clone()));
            // R78: the radiant flag survives leaving the field
            assert_eq!(reborn.as_ref().map(|unit| unit.radiant), Some(true));
            s.expect_stats(rhd.as_str(), json!({ "health": 1, "maxHealth": 2 }));
            assert_eq!(summoned.def_id, "core-003");
            assert!(!summoned.radiant);
            s.expect_stats(&summoned, json!({ "attack": 1, "maxHealth": 1 }));
        }

        #[test]
        fn r8_radiant_death_fires_on_the_second_death_too_for_two_base_bodies_in_all() {
            crate::register_all();
            let mut s = scenario(json!({
                "active": "p2",
                "p1": { "field": [{ "def": "core-003", "radiant": true, "lane": 1 }] },
                "p2": { "field": FOUR_VANILLAS }
            }));
            let rhd = s.card("core-003").id.clone();

            let first = foe(&s, 1);
            s.attack(first.as_str(), rhd.as_str());
            let second = foe(&s, 2);
            s.attack(second.as_str(), rhd.as_str()); // first death  → base body #1 in lane 2
            let third = foe(&s, 3);
            s.attack(third.as_str(), rhd.as_str()); // the returned body's shield
            let fourth = foe(&s, 4);
            s.attack(fourth.as_str(), rhd.as_str()); // second death → base body #2, lane 1 being free again

            s.expect_in_zone(rhd.as_str(), "graveyard");
            let bodies: Vec<CardInstance> = (1..=5).filter_map(|lane| s.unit(PlayerId::P1, lane)).collect();
            assert_eq!(
                bodies.iter().map(|unit| unit.def_id.as_str()).collect::<Vec<_>>(),
                vec!["core-003", "core-003"]
            );
            // Both are base bodies, so neither has a Death hook and the chain ends (§8 Engine cell).
            assert_eq!(bodies.iter().map(|unit| unit.radiant).collect::<Vec<_>>(), vec![false, false]);
            assert!(!bodies.iter().any(|unit| unit.id == rhd));
        }
    }
}
